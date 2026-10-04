/* cost.c — complete translation of engine/src/ability/cost.rs
   Mirrors pay_deferred_costs, validate_cost, pay_cost, pay_cost_inner,
   handle_optional_cost_payment, handle_pay_cost_all_discard,
   get_change_state_candidates, has_skip_prompt, pay_cost_move_cards,
   pay_cost_change_state. */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* ── Resolver-local state (mirrors AbilityResolver fields not in GameState) ──
   Deferred sub-costs live in GameState::queue (owned deep clones) so they
   survive across choice pauses and cannot dangle when the caller's tree is
   freed — mirrors Rust AbilityResolver.pending_deferred_costs. */
/* Rust's `Option<i16>` has two states at this boundary and the C needs both
   spelled apart, because -1 already means "no activating card" in GameState:
     RB_ACTIVATING_NONE     — Rust's None
     RB_ACTIVATING_DERIVED  — "reconstruct it the way pay time does", i.e.
                              handlers.rs:185-187 passing `gs.activating_card`
                              through cost_self_activating_card. */
#define RB_ACTIVATING_NONE    (-1)
#define RB_ACTIVATING_DERIVED (-1000)

static int s_pending_energy_payment;
static int s_stage_select_intent;
static int s_conditional_choice;

/* ── "the optional cost was declined" latch ──
   Every auto-decline site in this file mirrors Rust's
   `entry.cost_paid = true; entry.optional_cost_result = Some(false);`
   (handlers.rs:326/363/520/536/1082/1125). Rust turns that flag into a hard
   stop: `optional_cost_skip_gate` (engine/src/ability/gates.rs:195-213) is a
   POST_COST_GATE, so resolve_ability:1193-1199 returns before
   run_ability_effect and the ability's EFFECT never runs.

   The C records the flag on the current QUEUE entry, but the inline 登場 loop
   in engine.c resolves the debut cost without an entry of its own — so the
   verdict vanished there and the effect ran anyway. The same latch is exposed
   through rb_cost_optional_skipped() so that loop can honour the gate. */
static int s_cost_optional_skipped;

/* ── host of the in-flight deferred cost batch ──
   A SequentialCost that defers a binary leg (handlers.rs:980-984) parks a
   clone in `pending_deferred_costs` and pays it only from the choice epilogue
   (choice.rs:189 / :880), i.e. AFTER the play action has returned and after
   engine.c restored g->activating_card. Rust keeps `gs.activating_card`
   published for that whole window (resolver.rs:1161 sets it,
   finish_ability_resolution:1091 clears it, and the choice round-trip happens
   in between), so the deferred 「このメンバーをウェイトにし」 leg still resolves
   self_cost against the member that was just placed. Capture the card when the
   batch is opened and republish it around rb_pay_deferred_costs so the C keeps
   the same window. */
static int s_cost_deferred_host = -1;

int rb_cost_optional_skipped(void) { return s_cost_optional_skipped; }

/* cost_auto_decline — the single spelling of "this optional cost resolved as a
   skip": latch it, mark the queue entry when there is one, and report success
   (Rust returns Ok(()) for a skip; it is NOT a payment failure). */
static int cost_auto_decline(GameState *g) {
    s_cost_optional_skipped = 1;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
        g->queue.entries[g->queue.cur].cost_paid = 1;
        g->queue.entries[g->queue.cur].optional_cost_result = 0;
    }
    return 1;
}

/* forward: alloc.c owns the effect-tree clone/free pair */
AbilityEffect *rb_effect_deep_clone(const AbilityEffect *src);
void rb_effect_free(AbilityEffect *e);
/* forward: bag helpers are defined at the bottom of this file */
static void bag_push(RbBag *b, int c);
/* forward: ability_queue.c */
int rb_queue_current_entry(const GameState *g);

/* ── rb_record_cost_paid (game_state/mod.rs:404-414) ──────────────────────────
   The ONLY writer of `cards_paid_cost_this_turn`, and it had none tree-wide:
   core/modifiers.c:780-833 documents the body but explicitly leaves it
   uncompiled ("the one caller is the thing that has to be written first").
   Both of the callers that live in THIS file now exist, so the body is
   compiled here rather than in a file this change does not own.

   A SET, not a tally (mod.rs:397-398): the clause it feeds is 「このメンバーが、
   自分のカードの能力のコストを支払うため」 (PL!-pb2-021-N 南ことり) — "paid at
   least once this turn" — so two payments by the same member must still read as
   one card.

   Rust call sites, of five:
     handlers.rs:1389  accept_optional_move_cost   — LANDED below
     handlers.rs:1477  handle_optional_cost_payment— LANDED below
     choice.rs:734 / :763 / :948                    — still missing (choice.c) */
void rb_record_cost_paid(GameState *g) {
    if (!g) return;
    int cur = rb_queue_current_entry(g);
    if (cur < 0 || cur >= g->queue.n_entries) return;
    int cid = g->queue.entries[cur].card_id;
    if (cid < 0) return;
    for (int i = 0; i < g->n_cards_paid_cost_this_turn; i++)
        if (g->cards_paid_cost_this_turn[i] == cid) return;
    if (g->n_cards_paid_cost_this_turn < (int)(sizeof(g->cards_paid_cost_this_turn) /
                                               sizeof(g->cards_paid_cost_this_turn[0])))
        g->cards_paid_cost_this_turn[g->n_cards_paid_cost_this_turn++] = cid;
}

/* ── effect-field helpers (mirrors AbilityEffect::*_any() getters) ── */
static const char *eff_extra(const AbilityEffect *e, const char *k) {
    if (!e || !k) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}
static int eff_bool(const AbilityEffect *e, const char *k, int dflt) {
    const char *v = eff_extra(e, k);
    if (!v || !*v) return dflt;
    return !strcmp(v, "true") || !strcmp(v, "1");
}
static int eff_int(const AbilityEffect *e, const char *k, int dflt) {
    const char *v = eff_extra(e, k);
    if (!v || !*v) return dflt;
    return atoi(v);
}

/* ── cost_card_type_any -- Rust `cost.card_type_any()` ────────────────────────
   `card_type` is a TYPED field, not a raw string. Rust:
     effect_decoder_gen.rs `"card_type" => { ek.card_type = bc.read_card_type_value(); .. }`
     read_card_type_value is `CardType::from_card_str(get_string(idx)?)`, and
     core/card.rs `from_card_str` has exactly three arms with the miss arm
     `_ => None` (core/card.rs:50-57), so `AbilityEffect::card_type` is
     `Option<CardType>` and the raw spelling CANNOT survive the decode.
     `as_card_str` (core/card.rs:58-64) returns "member_card" / "live_card" /
     "energy_card" -- byte-for-byte what `card_type_field` already holds.

   THE BUG THIS REPLACES: all TEN cost handlers in this file read
   `eff_extra(<cost>, "card_type")`, and vm.c's decode arm for `card_type`
   (vm.c:1098-1112) `continue`s after writing the STRUCT FIELD, so the extras
   table never holds a `card_type` at all. Measured with the engine's own
   decoder over the whole corpus: `extra_present = 0` at all three of the sites
   the census could classify by kind (pay_cost_inner/Reveal 27 cost nodes,
   pay_cost_move_cards 459, pay_cost_change_state 124) and 0 of the 3465 effect
   nodes. Every one of those filters therefore answered "no card_type", i.e. it
   was INERT, while 23 Reveal / 113 move_cards / 124 change_state cost nodes DO
   carry a real one in `card_type_field`.

   The vm.c note records the same thing from the producer's side: the extras
   fallback that core/card.c:959 and dynamic_count.c:655 still read
   (`card_type_field[0] ? field : fx_extra(e, "card_type")`) is already dead and
   stays dead, because Rust drops the raw string too. */
static const char *cost_card_type_any(const AbilityEffect *e) {
    return (e && e->card_type_field[0]) ? e->card_type_field : NULL;
}

/* ── cost_distinct_type -- Rust `cost.distinct_any()` ────────────────────────
   core/card.rs:1977-1979:
       pub fn distinct_any(&self) -> Option<DistinctType> {
           self.kind.as_deref()?.filter()?.distinct.as_deref().copied()
       }
   and core/card.rs:2809-2816:
       pub enum DistinctType {
           #[serde(rename = "card_name")] CardName,
           #[serde(rename = "true")]      True,
           #[serde(rename = "distinct")]  Distinct,
       }
   The serde renames are LOWERCASE and they are what the wire carries, so the
   comparison is against "card_name" / "true" / "distinct". util.rs:2827-2832
   then asks only whether any of the three is set:
       matches!(distinct,
           Some(DistinctType::CardName) | Some(DistinctType::True)
                                    | Some(DistinctType::Distinct)) */
static RbDistinctType cost_distinct_type(const AbilityEffect *e) {
    const char *d = e ? eff_extra(e, "distinct") : NULL;
    if (!d) return RB_DISTINCT_NONE;
    if (!strcmp(d, "card_name")) return RB_DISTINCT_CARDNAME;
    if (!strcmp(d, "true"))     return RB_DISTINCT_TRUE;
    if (!strcmp(d, "distinct")) return RB_DISTINCT_DISTINCT;
    return RB_DISTINCT_NONE;
}

/* ── cost_dedupe_indices_by_name ──
   Rust, the LAST statement of hand_cost_matching_indices
   (engine/src/ability/cost/handlers.rs:476-480):
       let indices = if crate::ability::util::distinct_should_dedupe(cost.distinct_any()) {
           crate::ability::util::dedupe_by_normalized_name(
               &indices, |&i| pl.hand.cards[i], card_db)
       } else { indices };

   Two details matter and both are easy to get wrong:
     - it dedupes the CANDIDATE SET, not the selection, so the duplicate print is
       never OFFERED (the Rust comment says so explicitly: "Deduping the
       CANDIDATE SET (rather than filtering after the selection) also means the
       duplicate is never offered, so the player cannot select it");
     - the key is the card's NORMALIZED NAME at that index, so two PRINTS of one
       character collapse to the first index -- 「名前の異なる」 compares names,
       not card numbers. util.c's rb_dedupe_by_normalized_name takes CARD IDS and
       therefore returns "the first index of every distinct id", which is a
       different answer; the loop below is over INDICES, which is what Rust does.
       The normalizer is rb_card_normalize_name, the already-exported twin of
       `CardDatabase::normalize_name` (core/card.rs:675-685: drop every
       whitespace character), not util.c's file-static rb_norm_ws -- that one only
       drops ASCII whitespace and is a weaker normalizer than Rust's.
   util.c's rb_distinct_should_dedupe is the shared predicate (it is already
   exported) so the three spellings stay in ONE place. */
static int cost_dedupe_indices_by_name(const RbPlayer *P, const int *idx, int n,
                                       int *out, int max) {
    char seen[RB_MAX_HAND][256];
    int  nseen = 0;
    int  m = 0;
    for (int i = 0; i < n && m < max; i++) {
        int k = idx[i];
        int keep = 1;
        if (k >= 0 && k < P->hand.n) {
            Card c;
            if (rb_decode_card_by_index((uint32_t)P->hand.cards[k], &c)) {
                if (c.name) {
                    char nm[256];
                    rb_card_normalize_name(c.name, nm, sizeof nm);
                    int dup = 0;
                    for (int s = 0; s < nseen; s++)
                        if (!strcmp(seen[s], nm)) { dup = 1; break; }
                    if (dup) keep = 0;
                    else if (nseen < RB_MAX_HAND) {
                        size_t l = strlen(nm);
                        if (l >= sizeof seen[0]) l = sizeof seen[0] - 1;
                        memcpy(seen[nseen], nm, l);
                        seen[nseen][l] = '\0';
                        nseen++;
                    }
                }
                rb_free_card(&c);
            }
            /* util.rs:2185-2187 -- `.unwrap_or(true)`: a card missing from the
             * database is KEPT, not dropped. */
        }
        if (keep) out[m++] = idx[i];
    }
    return m;
}
/* ── energy_count_any() ──
   The Rust decoder folds BOTH wire keys "energy_count" and "energy" onto the
   single field `energy_count` (engine/src/ability/effect_decoder_gen.rs:164
   and :226). The C extras table keeps the raw wire key, so both spellings
   have to be consulted: 桜坂しずく PL!N-pb1-003-R and 園田海未 PL!-bp5-004-R＋
   ship `energy`, and reading only "energy_count" silently paid 0 energy. */
int rb_cost_energy_count_any(const AbilityEffect *e) {
    if (!e) return -1;
    const char *v = eff_extra(e, "energy_count");
    if (!v || !*v) v = eff_extra(e, "energy");
    if (!v || !*v) return -1;
    return atoi(v);
}
static int eff_energy_count(const AbilityEffect *e, int dflt) {
    int n = rb_cost_energy_count_any(e);
    return n < 0 ? dflt : n;
}

/* ── action-type predicates ── */
static int cost_is_energy(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "pay_energy")) return 1;
    const char *t = e->target;
    return t && strstr(t, "energy") != NULL;
}
static int cost_is_change_state(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "change_state")) return 1;
    const char *t = e->target;
    return t && strstr(t, "wait") != NULL;
}
static int cost_is_sequential(const AbilityEffect *e) {
    return e && e->action &&
           (!strcmp(e->action, "sequential") || !strcmp(e->action, "sequential_cost"));
}
static int cost_is_move_cards(const AbilityEffect *e) {
    return e && e->action && !strcmp(e->action, "move_cards");
}
static int cost_is_energy_condition(const AbilityEffect *e) {
    return e && e->action && !strcmp(e->action, "energy_condition");
}
static int cost_is_reveal(const AbilityEffect *e) {
    return e && e->action && !strcmp(e->action, "reveal");
}
static int cost_is_place_energy_under_member(const AbilityEffect *e) {
    return e && e->action && !strcmp(e->action, "place_energy_under_member");
}
static int cost_is_custom_under_member(const AbilityEffect *e) {
    return e && e->action && !strcmp(e->action, "custom") &&
           e->destination && strstr(e->destination, "under_member") != NULL;
}

/* ── Mirror cost.rs: has_skip_prompt ── */
static int has_skip_prompt(const AbilityEffect *cost) {
    if (!cost) return 0;
    if (cost_is_energy(cost)) {
        const char *any = eff_extra(cost, "any_number");
        return !(any && !strcmp(any, "true"));
    }
    if (cost_is_change_state(cost)) {
        const char *sc = eff_extra(cost, "self_cost");
        return sc && !strcmp(sc, "true");
    }
    return 0;
}

/* ── effect_uses_selected_cards ──
   Mirrors engine/src/ability/cost/handlers.rs:16-27: true when the effect
   reads the selected-cards zone, directly or through its sequential
   children. Determines whether a cost reveal must populate selected_cards. */
static int effect_uses_selected_cards(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->source && !strcmp(e->source, "selected_cards")) return 1;
    for (int i = 0; i < e->n_child; i++)
        if (effect_uses_selected_cards(e->child[i])) return 1;
    return 0;
}

/* ── CSV splitter ── */
static int split_csv(const char *s, char out[][64], int max) {
    if (!s || !*s) return 0;
    int n = 0;
    const char *p = s;
    while (*p && n < max) {
        const char *comma = strchr(p, ',');
        size_t len = comma ? (size_t)(comma - p) : strlen(p);
        if (len >= 64) len = 63;
        memcpy(out[n], p, len);
        out[n][len] = '\0';
        n++;
        if (!comma) break;
        p = comma + 1;
    }
    return n;
}

/* ── group / character matchers ── */
static int card_matches_any_group(int card_id, const char *group_csv) {
    if (!group_csv || !*group_csv) return 1;
    char groups[16][64];
    int ng = split_csv(group_csv, groups, 16);
    for (int i = 0; i < ng; i++)
        if (rb_card_matches_group_str(card_id, groups[i])) return 1;
    return 0;
}
static int card_matches_any_character(int card_id, const char *chars_csv) {
    if (!chars_csv || !*chars_csv) return 0;
    char chars[16][64];
    int nc = split_csv(chars_csv, chars, 16);
    const char *names[16];
    for (int i = 0; i < nc; i++) names[i] = chars[i];
    return rb_card_matches_characters(card_id, names, nc);
}

/* ── group_names: ONE entry, not the whole CSV ─────────────────────────────
 *
 * `card_matches_any_group` above is the twin of Rust's `card_matches_any_group`
 * (util.rs:799) and is correct only where the Rust source spells that call out.
 * The other shape in this file reads `AbilityEffect::group_name()` (core/card.rs
 * :2462 = `group_names.first()`) — ONE name, never the joined list. The C
 * bytecode decoder flattens the Rust `Vec<String>` into a single comma-joined
 * extra, so handing that whole string to `rb_card_matches_group_str` compares
 * "lilywhite,μ's" as one exact token and admits nothing.
 *
 * util.c already carries `rb_group_name_first` / `rb_effect_group_name` /
 * `rb_card_matches_effect_group` for this, but all three are `static` and are
 * NOT declared in include/rabuka.h, so this file carries a local twin rather
 * than editing a file this change does not own. */
static const char *group_name_first(const char *group_names, char *buf, size_t bufsz) {
    if (!group_names || !*group_names) return NULL;
    const char *comma = strchr(group_names, ',');
    size_t len = comma ? (size_t)(comma - group_names) : strlen(group_names);
    if (!buf || bufsz == 0) return group_names;
    if (len >= bufsz) len = bufsz - 1;
    memcpy(buf, group_names, len);
    buf[len] = '\0';
    return buf;
}

/* Pass when the effect names no group, else match `group_names.first()`. */
static int card_matches_group_name_first(int card_id, const char *group_names) {
    char buf[128];
    const char *g = group_name_first(group_names, buf, sizeof buf);
    if (!g || !*g) return 1;
    return rb_card_matches_group_str(card_id, g);
}

/* ══════════════════════════════════════════════════════════════════════════
   Shared display labels — handlers.rs:108-166 needs them and the Rust twins
   are not reachable from C:

     - util::card_plural            (ability/util/labels.rs:9-11)
     - util::card_type_label        (ability/util/labels.rs:24-32)
     - util::card_type_label_ja     (ability/util/labels.rs:34-42)

   look.c:97 and describe.c:1816-1822 each carry a private twin of the first
   two, so neither can be called from here without editing a file this change
   does not own.

   `card_type_label` must be the labels.rs pair, NOT describe.rs's
   `card_type_label_inner` (describe.rs:71-91, exported as
   rb_desc_card_type_label): the two disagree on exactly one input —
   energy_card renders as "energy" in describe.rs:86 and "energy card" in
   labels.rs:29 — and handlers.rs:112 / :146 call the labels.rs one.
   ══════════════════════════════════════════════════════════════════════════ */
static const char *cost_card_plural(int count) {
    return count == 1 ? "card" : "cards";
}
static const char *cost_card_type_label(const char *ct) {
    if (!ct) return "card";
    if (!strcmp(ct, "member_card")) return "member";
    if (!strcmp(ct, "live_card"))   return "live card";
    if (!strcmp(ct, "energy_card")) return "energy card";
    return ct;
}
static const char *cost_card_type_label_ja(const char *ct) {
    if (!ct) return "";
    if (!strcmp(ct, "member_card")) return "メンバー";
    if (!strcmp(ct, "live_card"))   return "ライブカード";
    if (!strcmp(ct, "energy_card")) return "エネルギー";
    return ct;
}

/* The operator's English/Japanese phrase. `op` is the wire spelling of
   `AbilityCost::cost_limit_operator` (core/card.rs:2811-2817: ">=", "<=",
   ">", "<", "=" with "==" as an alias). handlers.rs:128 / :155 default the
   field to Operator::Lte, and Operator is a closed enum there, so an
   unrecognised spelling takes the Lte branch — exactly what `unwrap_or(Lte)`
   does for the absent field. */
static const char *cost_op_word(const char *op, int ja) {
    int is_lte = !op || !*op || !strcmp(op, "<=");
    if (ja)
        return is_lte ? "以下" : !strcmp(op, "<") ? "未満"
             : !strcmp(op, ">=") ? "以上" : !strcmp(op, ">") ? "超" : "ちょうど";
    return is_lte ? "or less" : !strcmp(op, "<") ? "less than"
         : !strcmp(op, ">=") ? "or more" : !strcmp(op, ">") ? "more than"
         : "equal to";
}

/* `Vec<String>::join(sep)` over a comma-joined extras value. */
static void cost_join_csv(char *out, size_t outsz, const char *csv, const char *sep) {
    out[0] = '\0';
    if (!csv || !*csv) return;
    char buf[256];
    strncpy(buf, csv, sizeof(buf) - 1);
    buf[sizeof(buf) - 1] = '\0';
    size_t used = 0;
    char *tok = strtok(buf, ",");
    while (tok) {
        size_t need = strlen(tok) + strlen(sep) + 1;
        if (used + need > outsz) break;
        if (used) { strcpy(out + used, sep); used += strlen(sep); }
        strcpy(out + used, tok);
        used += strlen(tok);
        tok = strtok(NULL, ",");
    }
    out[used] = '\0';
}

/* ── cost_filter_desc_en / cost_filter_desc_ja (handlers.rs:108-166) ─────────
   The one place a move_cards cost's filters are spelled out for the player.
   BOTH spellings were absent from this file: the hand-cost prompt at :817-826
   hard-coded "Select %d card(s) from hand", so a cost filtered to
   「『Liella!』のメンバーカードを1枚」 was announced as a bare card pick, and
   the Japanese prompt this same function also builds was missing entirely.
   Emits "" when the cost carries no filter at all (handlers.rs:138 / :165). */
static void cost_filter_desc(const AbilityEffect *cost, int ja, char *out, size_t outsz) {
    char parts[8][192];
    int np = 0;
    const char *ct = cost_card_type_any(cost);
    if (ct && *ct && np < 8) {
        snprintf(parts[np], sizeof parts[0], "%s",
                 ja ? cost_card_type_label_ja(ct) : cost_card_type_label(ct));
        np++;
    }
    const char *groups = eff_extra(cost, "group_names");
    if (groups && *groups && np < 8) { cost_join_csv(parts[np], sizeof parts[0], groups, " / "); if (parts[np][0]) np++; }
    const char *chars = eff_extra(cost, "characters");
    if (chars && *chars && np < 8) { cost_join_csv(parts[np], sizeof parts[0], chars, " / "); if (parts[np][0]) np++; }
    const char *limit = eff_extra(cost, "cost_limit");
    if (limit && *limit && np < 8) {
        const char *op = eff_extra(cost, "cost_limit_operator");
        if (ja) snprintf(parts[np], sizeof parts[0], "コスト%s %s", cost_op_word(op, 1), limit);
        else    snprintf(parts[np], sizeof parts[0], "cost %s %s", cost_op_word(op, 0), limit);
        np++;
    }
    if (np == 0) { out[0] = '\0'; return; }
    /* handlers.rs:138 joins the EN parts with ", " inside " (...)" and :165
       joins the JA parts with the FULLWIDTH comma inside FULLWIDTH
       parentheses. Both separators are spelled as escapes so no multi-byte
       literal can be split by an editor. */
    const char *sep = ja ? "\xE3\x80\x81" : ", ";
    size_t used = 0;
    for (int i = 0; i < np; i++) {
        size_t add = strlen(parts[i]) + strlen(sep) + 1;
        if (used + add > outsz) break;
        if (used) { strcpy(out + used, sep); used += strlen(sep); }
        strcpy(out + used, parts[i]);
        used += strlen(parts[i]);
    }
    out[used] = '\0';
    if (used == 0) return;
    if (ja) {
        char body[120];
        strncpy(body, out, sizeof(body) - 1);
        body[sizeof(body) - 1] = '\0';
        snprintf(out, outsz, "\xEF\xBC\x88%s\xEF\xBC\x89", body);
    } else {
        char body[120];
        strncpy(body, out, sizeof(body) - 1);
        body[sizeof(body) - 1] = '\0';
        snprintf(out, outsz, " (%s)", body);
    }
}

/* ── Mirror cost.rs: get_change_state_candidates ── */

/* The card a `self_cost` leg resolves against.
   Rust compares against `gs.activating_card`, which
   `AbilityResolver::resolve_ability` (resolver.rs:1161) publishes for the whole
   resolution — including the 登場 path, because trigger_debut_abilities
   (turn/triggers.rs:108-127) enqueues the debut ability and the drain calls
   resolve_ability with that card.

   The C never published it on either path: `rb_process_current_ability`
   (game_state_abilities.c) leaves g->activating_card alone, and engine.c's
   inline 登場 loop had no queue entry at all. So `self_cost` compared against
   -1, matched no stage slot, and validate_cost rejected the WHOLE bundled
   「ウェイトにし、手札を1枚控え室に置いてもよい」 cost. Fall back to the
   resolving queue entry's card so the self_cost leg finds the member that is
   actually being placed. `exclude_self` deliberately keeps reading
   g->activating_card only, so nothing else in this file changes behaviour. */
static int cost_self_activating_card(const GameState *g) {
    if (g->activating_card >= 0) return g->activating_card;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
        int cid = g->queue.entries[g->queue.cur].card_id;
        if (cid >= 0) return cid;
    }
    return s_cost_deferred_host;
}

static int get_change_state_candidates_for(const GameState *g, int actor,
                                           const char *target,
                                           const char *card_type,
                                           const char *group_names,
                                           int exclude_self,
                                           int self_cost,
                                           int check_name,
                                           const char *state,
                                           int activating_id,
                                           int *out_positions, int max) {
    const RbPlayer *P = &g->p[actor];
    int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE && n < max; i++) {
        int id = P->stage[i];
        if (id == RB_EMPTY_SLOT) continue;
        if (self_cost) {
            if (activating_id != id) continue;
        } else if (exclude_self && activating_id == id) {
            continue;
        }
        if (card_type && *card_type && !rb_card_matches_type(id, card_type)) continue;
        if (group_names && *group_names) {
            if (!card_matches_any_group(id, group_names)) {
                if (!check_name || !card_matches_any_character(id, group_names))
                    continue;
            }
        }
        if (state && *state) {
            const char *ori = rb_mods_get_orientation((RbMods *)&g->mods, id);
            int matches;
            if (!strcmp(state, "active")) {
                matches = !ori || strcmp(ori, "wait") != 0;
            } else if (!strcmp(state, "wait")) {
                matches = ori && !strcmp(ori, "wait");
            } else {
                matches = ori && !strcmp(ori, state);
            }
            if (!matches) continue;
        }
        out_positions[n++] = id;
    }
    return n;
}

/* The DERIVED-activating-card spelling every pay-time call site uses. Rust's
   `get_change_state_candidates` takes the activating card as an argument; at pay
   time that is `gs.activating_card`, which the C has to reconstruct (see
   cost_self_activating_card above). */
static int get_change_state_candidates(const GameState *g, int actor,
                                       const char *target,
                                       const char *card_type,
                                       const char *group_names,
                                       int exclude_self,
                                       int self_cost,
                                       int check_name,
                                       const char *state,
                                       int *out_positions, int max) {
    int activating_id = self_cost ? cost_self_activating_card(g) : g->activating_card;
    return get_change_state_candidates_for(g, actor, target, card_type, group_names,
                                           exclude_self, self_cost, check_name, state,
                                           activating_id, out_positions, max);
}

/* ── player_prefix ── */
static const char *player_prefix(const GameState *g, int card_id) {
    if (!g) return "??";
    for (int pl = 0; pl < 2; pl++)
        for (int s = 0; s < RB_STAGE_SIZE; s++)
            if (g->p[pl].stage[s] == card_id) return pl == 0 ? "P1" : "P2";
    for (int pl = 0; pl < 2; pl++) {
        for (int i = 0; i < g->p[pl].hand.n; i++)
            if (g->p[pl].hand.cards[i] == card_id) return pl == 0 ? "P1" : "P2";
        for (int i = 0; i < g->p[pl].energy.n; i++)
            if (g->p[pl].energy.cards[i] == card_id) return pl == 0 ? "P1" : "P2";
    }
    return "??";
}

/* ── emit_pay_skip_gate ──
   Open the pay-or-skip gate (cost/handlers.rs:1371-1378).

   The answer is routed by RbQueueEntry.cost, the "amount this gate charges
   when it is ACCEPTED" the answer path reads back (choice.c:3442-3443:
   `if (W->energy_active >= e->cost) W->energy_active -= e->cost;`). Nothing in
   the C ever WROTE that field -- `entries[].cost` has zero writers tree-wide,
   so it is always 0 and an accepted pay/skip gate taps nothing:
   `energy_active -= 0`. That is the cost.c half of the "paying an optional
   energy cost changes no energy" symptom, and it is fixed here because this
   function is the only place in the cost path that knows the printed amount
   before the player answers.

   Rust has no equivalent field: the gate writes `pending.card =
   ChoiceCard::Energy`, and `handle_optional_cost_payment` re-derives the amount
   from `gs.entry_cost()` at answer time (handlers.rs:1393/1482-1490). The C's
   `rb_handle_optional_cost_payment` does the same re-derivation, but no live
   path calls it -- only tests/test_parity_cost_compound.c does -- so the C
   needs the number carried on the entry instead.

   Only an energy gate gets a non-zero amount: `e->action` is the gate's own
   cost, and a non-energy gate charges nothing on acceptance, so 0 is right. */
static void emit_pay_skip_gate(GameState *g, int actor, const AbilityEffect *e,
                               const char *description, int optional,
                               const char *route) {
    RbChoiceRoute r = RB_ROUTE_OPTIONAL_COST;
    if (route && !strcmp(route, "ChoiceCost")) r = RB_ROUTE_CHOICE_COST;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, optional,
                   "pay_optional_cost:skip");
    RbChoice *pending = &g->queue.pending;
    if (pending->description[0] == '\0')
        strncpy(pending->description, description, sizeof(pending->description) - 1);
    pending->route = r;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
        int amount = (e && e->action && !strcmp(e->action, "pay_energy"))
                         ? eff_energy_count(e, 0) : 0;
        g->queue.entries[g->queue.cur].cost = amount > 0 ? amount : 0;
        g->queue.entries[g->queue.cur].choice_card_no = r;
    }
}

/* ── cost_publish_pending ──
   Publish a prompt this file built by hand.

   pay_cost_move_cards and pay_cost_inner assemble their `pending` with a raw
   memset (they need filter fields the generic emitter has no parameter for:
   cost_limit / cost_limit_op / characters / target_player_id). Building it by
   hand is fine; PUBLISHING it by hand was not. `memset` + `has_pending = 1`
   leaves three of the five things rb_emit_choice does:

     - g->queue.actor is never set, so the answer is attributed to whatever
       seat the pause below stamps (g->active), not to the player the cost
       asked;
     - rb_queue_pause_for_choice is never called, so the current entry's
       `choice_player_id` is never filled and queue.state stays RESOLVING.
       The answer therefore arrives with the entry still open, and the drain
       after it re-runs the whole ability from the top: the cost is offered
       again and the prompt re-opens in a loop;
     - the look/resume scratch fields are left at whatever the previous
       prompt left in them, so a stale resume_after_look fires against this
       prompt's answer.

   This is the same contract rb_emit_choice honours (choice.c:4724-4770), in
   the same order: pause FIRST (its first line is
   `if (state == RB_QUEUE_AWAITING_CHOICE) return;`, ability_queue.c:412, so
   stamping first makes it a no-op), restore `actor` after the pause because
   pause_for_choice overwrites it with g->active (ability_queue.c:470, and the
   cross-player watchers read that field), then stamp the QueueState. */
static void cost_publish_pending(GameState *g, int actor) {
    if (!g) return;
    g->queue.resume_look_owner = -1;
    g->queue.resume_after_look = NULL;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
        RbChoice ch = g->queue.pending;
        ch.actor = actor;
        rb_queue_pause_for_choice(g, &ch);
        g->queue.actor = actor;
    }
    g->queue.state = RB_QUEUE_AWAITING_CHOICE;
}

/* ── cost_drop_pending_actions ──
    Rust's `entry.pending_actions.clear()` and `entry.pending_actions = vec![alt]`
    (cost/handlers.rs:1417-1420, :1435) both ASSIGN a fresh Vec, dropping whatever
    the entry was holding. rb_queue_take_pending_actions is the same release in
    the C: it detaches the batch AND frees every owned clone in it
    (ability_queue.c:993-998). Unlike writing `pending_actions_n = 0` by hand it
    keeps the ownership contract at ability_queue.c:18-45 — exactly one of the
    three sanctioned release paths runs for a given batch, so no parked clone
    leaks and none is freed twice.

    Only the arms where Rust genuinely REPLACES or CLEARS the batch may call this.
    The arms where Rust EXECUTES it must not; see cost_arm_resume. */
static void cost_drop_pending_actions(GameState *g) {
    if (!g) return;
    rb_queue_take_pending_actions(g);
}

/* ── cost_arm_resume ──
    The epilogue every answer arm of this file ends with. It deliberately does NOT
    consume the parked tail.

    WHY. Rust calls `AbilityResolver::resume_pending_actions` from here
    (cost/handlers.rs:1422 on the skip arm, :1437 on the insufficient-energy arm,
    :1480 and :1656 on the pay arms, :1732 on the all-hand-discard arm), and that
    function is Rust's SINGLE consumer of the parked batch: it takes the batch and
    runs each command through `self.execute_effect`, re-parking the remainder when
    a command opens a new prompt (choice.rs:117-135, and :126-128 ->
    requeue_remaining_after_choice :87-115). Rust therefore has exactly one
    execution point and it is inside this call.

    The C cannot put the execution HERE, because this arm is not the bottom of the
    C's answer path. `rb_handle_optional_cost_payment` and
    `rb_handle_pay_cost_all_discard` are reached from choice.c's pay/skip arm
    (choice.c:3682) and the all-discard arm (choice.c:4820), both of which sit
    inside `rb_resolver_handle_select_target`; that function RETURNS into
    `rb_resume_with_choice_indices_internal`, which then calls
    `rb_resolver_continue_siblings(g, actor, host, cont, cont_from)`
    (choice.c:4927 for the pay/skip arm, :4821 for the all-discard arm — the very
    next line) before the choice_resume tail normalizes the state and drains
    (choice.c:4980-4993).

    `rb_resolver_continue_siblings` (choice.c:4361-4385) is ALREADY a faithful
    port of choice.rs:117-135: it takes the batch by ownership,
    rb_execute_effect_ex's each command in order, rb_queue_repark_pending_actions
    the remainder when a sub-prompt interrupts, and stamps entry.effect_started
    (choice.rs:81-83). It is the C's single execution point, one step further down
    the SAME call chain.

    So the two-sources rule for this file: the C executes the tail at
    rb_resolver_continue_siblings, and a cost arm must LEAVE the batch parked for
    it. The old body here called rb_queue_take_pending_actions — which detaches
    and FREES the batch and returns only the count — and then drained the queue
    that many times having executed nothing. The batch was destroyed at this
    point, so by the time rb_resolver_continue_siblings ran,
    rb_queue_has_pending_actions() was false and it took its `cont` FALLBACK walk
    (choice.c:4386-4397) over the parked parent tree instead of the owned path:
    the same steps ran a second time, from the other of the two carriers. That is
    the defect: the tail was not merely discarded, its destruction is what forced
    the double execution.

    Note the polarity is the OPPOSITE of choice.c's rb_resolver_resume_pending_actions,
    whose two "faithful" execution variants both regressed (choice.c:588-623).
    That site is reached from arms whose C continuation has ALREADY been walked
    before it returns, so executing there double-counts. Here the continuation has
    NOT run yet — it is still parked — so consuming it is precisely what causes the
    double-count. Leaving it parked is the fix; the count-only take that stood here
    was the bug.

    The drain is kept because it is not the batch's business and it preserves this
    arm's behaviour for any caller that reaches it without a current entry (the
    direct test callers, tests/test_parity_cost_compound.c:380/:389, where
    rb_drain_ability_queue is a no-op). The live callers get their drain from the
    choice_resume tail either way. */
static int cost_arm_resume(GameState *g) {
    if (!g) return -1;
    rb_drain_ability_queue(g);
    return 0;
}

/* ── execute_move_cards (simple zone transfer for cost payment) ── */
static int execute_move_cards(GameState *g, int actor, const AbilityEffect *cost) {
    const char *src = cost->source ? cost->source : "";
    const char *dst = cost->destination ? cost->destination : "discard";
    int count = cost->count > 0 ? cost->count : 1;
    int moved = 0;
    RbPlayer *P = &g->p[actor];

    if (!strcmp(src, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE && moved < count; i++) {
            if (P->stage[i] != RB_EMPTY_SLOT) {
                int cid = P->stage[i];
                P->stage[i] = RB_EMPTY_SLOT;
                P->stage_wait[i] = 0;
                if (!strcmp(dst, "waitroom") || !strcmp(dst, "discard")) {
                    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
                } else if (!strcmp(dst, "energy")) {
                    if (P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++] = cid;
                } else if (!strcmp(dst, "hand")) {
                    if (P->hand.n < RB_MAX_HAND) P->hand.cards[P->hand.n++] = cid;
                } else if (!strcmp(dst, "energy_deck")) {
                    if (P->energy_deck.n < RB_MAX_ZONE) P->energy_deck.cards[P->energy_deck.n++] = cid;
                }
                moved++;
            }
        }
        g->mods.last_cost_discard_count = moved;
    } else {
        RbBag *src_bag = NULL;
        if (!strcmp(src, "hand")) src_bag = &P->hand;
        else if (!strcmp(src, "deck") || !strcmp(src, "deck_top")) src_bag = &P->deck;
        else if (!strcmp(src, "waitroom") || !strcmp(src, "discard")) src_bag = &P->discard;
        else if (!strcmp(src, "energy") || !strcmp(src, "energy_zone")) src_bag = &P->energy;

        if (src_bag) {
            while (moved < count && src_bag->n > 0) {
                int cid = src_bag->cards[--src_bag->n];
                if (!strcmp(dst, "waitroom") || !strcmp(dst, "discard")) {
                    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
                } else if (!strcmp(dst, "energy")) {
                    if (P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++] = cid;
                } else if (!strcmp(dst, "hand")) {
                    if (P->hand.n < RB_MAX_HAND) P->hand.cards[P->hand.n++] = cid;
                } else if (!strcmp(dst, "energy_deck")) {
                    if (P->energy_deck.n < RB_MAX_ZONE) P->energy_deck.cards[P->energy_deck.n++] = cid;
                }
                moved++;
            }
            g->mods.last_cost_discard_count = moved;
        }
    }
    return moved;
}

/* ── execute_place_energy_under_member_non_optional ── */
int execute_place_energy_under_member_non_optional(GameState *g, int actor, const AbilityEffect *e) {
    return rb_effect_place_energy_under_member_non_optional(g, actor, e);
}

/* ── cost_zone_cards ──
   Mirror util::count_in_zone's zone coverage. Returns the card array for the
   source zones this file's validators care about, or NULL for every other zone
   (Rust handlers.rs:209-214 returns Ok(()) for anything that is not
   Hand/Stage/Waitroom/Energy — e.g. a deck-sourced cost is never validated).

   "energy_zone" was MISSING here and its absence was a refusal, not a pass:
   ability/enums.rs:98 spells the alias `"energy" | "energy_zone" => Energy`, and
   six corpus costs carry the wire form — PL!SP-bp5-111-P＋, PL!SP-bp7-007-P,
   PL!SP-bp7-010-P, PL!SP-bp7-022-N, PL!SP-bp7-026-L, PL!SP-bp7-027-L — so
   `Zone::from_str` resolved them to Zone::Energy on the Rust side while every
   C zone read here fell through to "not a cost zone" and cost 0
   (`matching_count = 0 < count`). */
static const int *cost_zone_cards(const RbPlayer *P, const char *src, int *n) {
    *n = 0;
    if (!src || !*src) return NULL;
    if (!strcmp(src, "hand"))          { *n = P->hand.n;          return P->hand.cards; }
    if (!strcmp(src, "waitroom") ||
        !strcmp(src, "discard"))       { *n = P->discard.n;       return P->discard.cards; }
    if (!strcmp(src, "energy") ||
        !strcmp(src, "energy_zone"))   { *n = P->energy.n;        return P->energy.cards; }
    if (!strcmp(src, "stage")) {
        static int stage_ids[RB_STAGE_SIZE];
        int k = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] != RB_EMPTY_SLOT) stage_ids[k++] = P->stage[i];
        *n = k;
        return stage_ids;
    }
    return NULL;
}

/* ── validate_cost ──
   Rust handlers.rs:169-181: SequentialCost recurses into every sub-cost, and
   everything else goes to the leaf check. The leaf body is factored out below
   as validate_leaf_cost_for so validate_mandatory_cost (handlers.rs:192-271,
   281-309) can share it — the two differ ONLY in the activating card they pass
   and in the sub-costs they accept. */
static int validate_leaf_cost_for(const GameState *g, int actor,
                                  const AbilityEffect *cost,
                                  int activating_card) {
    if (cost->action && !strcmp(cost->action, "choice_condition")) return 1;
    if (cost_is_move_cards(cost)) {
        const char *src = cost->source ? cost->source : "";
        int count = cost->count > 0 ? cost->count : 1;
        const char *target = cost->target ? cost->target : "self";
        int tp = rb_resolve_target_player(g, target);
        int tpl = (tp >= 0) ? tp : actor;
        int n_cards = 0;
        const int *cards = cost_zone_cards(&g->p[tpl], src, &n_cards);
        /* Unknown / unvalidated source zone: Rust returns Ok(()) immediately. */
        if (!cards) return 1;
        /* Filter-aware: 「『Liella!』のカードを1枚」 is unpayable when the hand
           holds cards but none match the cost filter (handlers.rs:215-222). */
        RbCardFilter f;
        rb_effect_filter_subset(cost, &f);
        int available = rb_count_matching_filter(&f, cards, n_cards);
        /* Rule 9.4.2.3 / Q56: costs must be paid in full. */
        if (available < count) return 0;
        return 1;
    }
    if (cost_is_energy_condition(cost)) {
        int count = cost->count > 0 ? cost->count : 1;
        /* Rust handlers.rs:232-237 resolves the energy condition against
           "self" explicitly, not against the ability's target. */
        int tp = rb_resolve_target_player(g, "self");
        int tpl = (tp >= 0) ? tp : 0;
        const RbPlayer *P = &g->p[tpl];
        if ((int)P->energy.n < count) return 0;
        return 1;
    }
    if (cost_is_change_state(cost)) {
        const char *sc = eff_extra(cost, "state_change");
        if (sc && !strcmp(sc, "wait")) {
            const char *target = cost->target ? cost->target : "self";
            int exclude_self = eff_bool(cost, "exclude_self", 0);
            const char *card_type = cost_card_type_any(cost);
            const char *group_names = eff_extra(cost, "group_names");
            int self_cost = eff_bool(cost, "self_cost", 0);
            int pos[RB_STAGE_SIZE];
            /* handlers.rs:65 — get_change_state_candidates resolves
               `cost.target` itself. The C read the CALLER's seat, so a
               「(::Petite:: Homework を1人ウェイトにし)」-shaped cost filtered
               the wrong player's stage here. */
            int ctp = rb_resolve_target_player(g, target);
            int ctpl = (ctp >= 0) ? ctp : actor;
            /* handlers.rs:185-187 — the pay-time spelling passes
               `gs.activating_card`; the offer/activation spelling passes the
               card under consideration, because `gs.activating_card` is not set
               yet when the offer is generated. */
            int aid = (activating_card == RB_ACTIVATING_DERIVED)
                          ? cost_self_activating_card(g) : activating_card;
            int n = get_change_state_candidates_for(g, ctpl, target, card_type,
                                                    group_names, exclude_self,
                                                    self_cost, 0, "active", aid,
                                                    pos, RB_STAGE_SIZE);
            if (n == 0) return 0;
        }
        return 1;
    }
    return 1;
}

static int validate_cost(const GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;
    if (cost_is_sequential(cost)) {
        for (int i = 0; i < cost->n_child; i++)
            if (!validate_cost(g, actor, cost->child[i])) return 0;
        return 1;
    }
    return validate_leaf_cost_for(g, actor, cost, RB_ACTIVATING_DERIVED);
}

/* ── validate_mandatory_cost (handlers.rs:281-309) ────────────────────────────
   The UPFRONT payability gate for offer generation and activation. It had NO C
   counterpart: game_setup.c:1199 calls rb_validate_cost, which is the PAY-TIME
   twin (handlers.rs:169) and is strictly stronger — it walks optional and
   any-number sub-costs too, so a 「…してもよい」 cost whose optional leg is
   currently unaffordable withheld the whole ability from the offer list.

   Three rules separate the two, and each is a live difference:

     - handlers.rs:287-292 — a whole optional / any-number / max cost resolves
       to pay-zero-or-skip, so it is NEVER a refusal;
     - handlers.rs:294-302 — SequentialCost recurses, ChoiceCondition passes
       (options are chosen at pay time), and PayEnergy is owned by the energy
       gates (check_mandatory_activation_cost + the offer-site energy filter);
     - handlers.rs:192-196 — the activating card is passed EXPLICITLY, because
       `gs.activating_card` is not set when the offer is generated. It is what
       self_cost / exclude_self filter against.

   Live Rust call sites, all of which want this and not rb_validate_cost:
     engine/src/game/game_setup.rs:2188  (stage 起動 offer)
     engine/src/game/game_setup.rs:2295  (discard 起動 offer)
     engine/src/turn/actions/mod.rs:559   (activate_ability, before the card
                                          leaves hand) */
static int validate_mandatory_cost(const GameState *g, int actor,
                                   const AbilityEffect *cost,
                                   int activating_card) {
    if (!cost) return 1;
    if (cost->is_optional || eff_bool(cost, "any_number", 0) ||
        eff_bool(cost, "max", 0))
        return 1;
    if (cost_is_sequential(cost)) {
        for (int i = 0; i < cost->n_child; i++)
            if (!validate_mandatory_cost(g, actor, cost->child[i], activating_card))
                return 0;
        return 1;
    }
    if (cost->action && !strcmp(cost->action, "choice_condition")) return 1;
    if (cost_is_energy(cost)) return 1;
    return validate_leaf_cost_for(g, actor, cost, activating_card);
}

/* ── offer_same_unit_cost (handlers.rs:648-711) ──────────────────────────────
   「手札の同じユニット名を持つカード2枚を控え室に置いてもよい」 — the optional
   same-unit hand cost. It was MISSING from this file, and its absence was not
   inert: pay_cost_move_cards computed `is_from_hand = hand && !same_unit`, so
   a same-unit cost skipped the hand-selection offer entirely, fell through to
   the raw zone count, and `execute_move_cards` then discarded `count`
   ARBITRARY hand cards with no unit check and no prompt. The two corpus cards
   that carry the flag — PL!HS-PR-016-PR (日野下花帆) and PL!HS-PR-017-PR
   (村野さやか), both ライブ開始時, abilities.json:28171-28180 / :28223-28232 —
   therefore paid themselves from the top of hand whatever it held.

   Rust instead offers ONE card, filtered to units that have at least `count`
   matching members in hand, so the second discard (paid by the shared-gate
   re-entry at handlers.rs:1582) is guaranteed to come from the same unit.
   Returns 1 handled, 0 refused. */
static int offer_same_unit_cost(GameState *g, int actor, const AbilityEffect *cost,
                                int count, int optional, int is_activation) {
    const char *target = cost->target ? cost->target : "self";
    int tp = rb_resolve_target_player(g, target);
    int tpl = (tp >= 0) ? tp : actor;
    const RbPlayer *P = &g->p[tpl];
    const char *card_type = cost_card_type_any(cost);
    RbCardFilter filter;
    rb_effect_filter_subset(cost, &filter);
    /* handlers.rs:662 `filter.card_type = card_type_owned.as_deref()` —
       the cost's own card_type REPLACES the one filter_subset derived. */
    filter.card_type[0] = '\0';
    if (card_type && *card_type)
        strncpy(filter.card_type, card_type, sizeof(filter.card_type) - 1);

    /* Group the eligible hand cards by unit name. handlers.rs:664-673 keys the
       map on `card.unit.clone().unwrap_or_default()`, so a card with NO unit
       groups under "" — and the eligible test at :679-681 reads the same ""
       for it, so a hand of unit-less members is eligible exactly as Rust has
       it. rb_card_unit_name is the canonical accessor (core/data.c:364-371):
       `Card.unit_idx` + rb_card_string cannot tell "absent" from index 0xFFFF. */
    static char unit_names[RB_MAX_HAND][64];
    static int  unit_counts[RB_MAX_HAND];
    int n_units = 0;
    for (int i = 0; i < P->hand.n; i++) {
        int cid = P->hand.cards[i];
        if (!rb_count_matching_filter(&filter, &cid, 1)) continue;
        const char *u = rb_card_unit_name(cid);
        int found = -1;
        for (int k = 0; k < n_units; k++)
            if (!strcmp(unit_names[k], u)) { found = k; break; }
        if (found < 0) {
            if (n_units >= RB_MAX_HAND) continue;
            found = n_units++;
            strncpy(unit_names[found], u ? u : "", sizeof(unit_names[0]) - 1);
            unit_names[found][sizeof(unit_names[0]) - 1] = '\0';
            unit_counts[found] = 0;
        }
        unit_counts[found]++;
    }
    int eligible[RB_MAX_ZONE];
    int n_eligible = 0;
    for (int i = 0; i < P->hand.n && n_eligible < RB_MAX_ZONE; i++) {
        int cid = P->hand.cards[i];
        const char *u = rb_card_unit_name(cid);
        for (int k = 0; k < n_units; k++)
            if (!strcmp(unit_names[k], u) && unit_counts[k] >= count) {
                eligible[n_eligible++] = i;
                break;
            }
    }
    if (n_eligible == 0) {
        /* handlers.rs:688-696 — an optional, non-activation cost with no
           eligible unit simply returns (the colon gate then suppresses the
           effect); anything else is a refusal. */
        if (optional && !is_activation) return 1;
        return 0;
    }
    int is_optional = optional && !is_activation;
    RbChoice *pending = &g->queue.pending;
    memset(pending, 0, sizeof(*pending));
    pending->kind = RB_CHOICE_SELECT_CARD;
    strncpy(pending->zone, "hand", sizeof(pending->zone) - 1);
    pending->count = 1;
    pending->allow_skip = is_optional;
    pending->route = RB_ROUTE_SELECT_CARDS;
    snprintf(pending->description, sizeof(pending->description),
             "Select 1 card (need %d with the same unit name)", count);
    snprintf(pending->description_ja, sizeof(pending->description_ja),
             "同名ユニットが%d枚必要なカードを1枚選択", count);
    if (card_type && *card_type)
        strncpy(pending->card_type, card_type, sizeof(pending->card_type) - 1);
    snprintf(pending->target_player_id, sizeof(pending->target_player_id),
             "p%d", tpl + 1);
    for (int i = 0; i < n_eligible && i < RB_MAX_ZONE; i++)
        pending->filtered_indices[i] = eligible[i];
    pending->n_filtered_indices = n_eligible;
    g->queue.has_pending = 1;
    cost_publish_pending(g, actor);
    return 1;
}

/* ── check_zone_cost_availability (handlers.rs:715-776) ───────────────────────
   The generic (non-hand) zone gate. Two divergences from the Rust it replaced:

     - it counted the WHOLE zone (`P->hand.n`, `P->deck.n`, ...) instead of
       `util::count_in_zone(player, zone, &filter, card_db)` (handlers.rs:735).
       A cost filtered to one card type therefore passed this gate on the
       strength of unrelated cards, and `execute_move_cards` then moved cards
       the cost filter rejects;
     - it read `g->p[actor]` (handlers.rs:723 resolves `cost.target`, which
       defaults to "self"), so a cost aimed at the opponent was checked against
       the wrong player's zones.

   The Q104 / Rule 10.2.1 deck_top refresh allowance (handlers.rs:753-774) is
   kept: deck_top counts the DECK, and a shortfall may be made up from the
   waitroom. */
static int check_zone_cost_availability(GameState *g, int actor,
                                        const AbilityEffect *cost,
                                        const char *source, int count) {
    const char *target = cost->target ? cost->target : "self";
    int tp = rb_resolve_target_player(g, target);
    int tpl = (tp >= 0) ? tp : actor;
    const RbPlayer *P = &g->p[tpl];
    const char *card_type = cost_card_type_any(cost);
    RbCardFilter filter;
    rb_effect_filter_subset(cost, &filter);
    /* handlers.rs:727-728 — the cost's own card_type and cost_limit replace
       whatever filter_subset derived. */
    filter.card_type[0] = '\0';
    if (card_type && *card_type)
        strncpy(filter.card_type, card_type, sizeof(filter.card_type) - 1);
    const char *cost_limit = eff_extra(cost, "cost_limit");
    filter.has_cost_limit = 0;
    filter.cost_limit = 0;
    if (cost_limit && *cost_limit) {
        filter.has_cost_limit = 1;
        filter.cost_limit = atoi(cost_limit);
        const char *op = eff_extra(cost, "cost_limit_operator");
        if (op && *op) {
            strncpy(filter.cost_op, op, sizeof(filter.cost_op) - 1);
            filter.cost_op[sizeof(filter.cost_op) - 1] = '\0';
        }
    }
    int is_deck_top = !strcmp(source, "deck_top");
    /* handlers.rs:730-734 — a deck_top cost is COUNTED against the deck. */
    const char *zone_name = is_deck_top ? "deck" : source;
    int n_cards = 0;
    const int *cards = cost_zone_cards(P, zone_name, &n_cards);
    int matching = cards ? rb_count_matching_filter(&filter, cards, n_cards) : 0;
    if (matching >= count) return 1;
    if (!is_deck_top) return 0;
    n_cards = 0;
    cards = cost_zone_cards(P, "waitroom", &n_cards);
    int waitroom_matching = cards ? rb_count_matching_filter(&filter, cards, n_cards) : 0;
    return matching + waitroom_matching >= count;
}

/* ── pay_cost_move_cards ── */
static int pay_cost_move_cards(GameState *g, int actor, const AbilityEffect *cost) {
    const char *source = cost->source ? cost->source : "";
    int is_any_number = eff_bool(cost, "any_number", 0);
    /* handlers.rs:786 `let count = cost.count.unwrap_or(1) as usize;`
       `unwrap_or(1)` supplies the default ONLY when the field is ABSENT, so an
       explicit Some(0) survives as 0. In the C the absent sentinel is -1
       (include/rabuka.h: "int count;  ... -1 = none"; vm.c:440 initialises
       every decoded effect to -1 and vm.c:720 overwrites it only when the wire
       actually carries the field), and rb_effect_count_or is the already-
       written twin of `unwrap_or`: `e->count >= 0 ? e->count : default`.
       The `> 0` test collapsed "absent" together with a legitimate 0.

       That 0 is real, not a hole: resolver.rs:1476-1477 / resolver.c:637-642
       CLAMP a per-unit cost reduction to zero (`saturating_sub`, then
       `cost.count = Some(next)`), and 「成功ライブ1枚につきコスト1減る」
       (PL!SP-pb1-007) with 3 success lives lands on exactly 3-3=0. Rust then
       asks for ZERO cards at handlers.rs:521/594-599
       (`effective_count = if is_any_number { 0 } else { count }`), and the
       prompt the player sees carries count 0. The `> 0` form turned that
       clamped zero back into 1, so a free cost asked for a card. */
    int count = rb_effect_count_or(cost, 1);
    const char *card_type = cost_card_type_any(cost);
    int optional = cost->is_optional;
    const char *target = cost->target ? cost->target : "self";
    int tp = rb_resolve_target_player(g, target);
    int tpl = (tp >= 0) ? tp : actor;
    RbPlayer *P = &g->p[tpl];
    int same_unit = eff_bool(cost, "same_unit_name", 0);
    int is_from_hand = !strcmp(source, "hand") && !same_unit;
    int is_all = eff_bool(cost, "all", 0);
    /* 「このカードを手札から控え室に置く」 — a mandatory, fixed-count
       self-discard. Rust handlers.rs:776-780 computes exactly this and
       handlers.rs:800-820 then SKIPS the hand-selection offer for it, so no
       prompt is opened and the activating card itself is paid
       (engine/src/ability/move_cards.rs:958-982). */
    int hand_self_cost = is_from_hand && eff_bool(cost, "self_cost", 0) &&
                         !optional && !is_any_number;
    int is_activation = 0;

    /* Determine is_activation from current queue entry */
    {
        int cur = g->queue.cur;
        if (cur >= 0 && cur < g->queue.n_entries) {
            int cid = g->queue.entries[cur].card_id;
            int aidx = g->queue.entries[cur].ability_idx;
            Ability ab;
            if (rb_decode_card_ability((uint32_t)cid, aidx, &ab)) {
                is_activation = rb_resolver_current_ability_is_activation(&ab);
                rb_free_ability(&ab);
            }
        }
    }

    /* Optional stage-move costs: emit pay/skip gate first.
       handlers.rs:326-332 (gate_optional_stage_cost) counts stage members with
       `cost.filter_subset()` and `filter.matches(card_db, id, true)` -- i.e.
       card_type, group_names, cost_limit, characters, exclude_characters,
       exclude_self and cost_values ALL apply, and `true` is util.rs's
       `skip_empty` (drop the -1 slots), not a cost-only relaxation.

       The C compared `group_names` ALONE, so any other filter field was
       invisible to the eligibility count. The corpus card PL!SP-pb1-011-P
       (ab#0, printed 「『鬼塚冬毬』以外の『Liella!』のメンバー1人をステージから
       控え室に置いてもよい」) carries exclude_characters:["鬼塚冬毬"], so with
       ONLY herself on stage the C counted 1 >= count and opened the pay/skip
       gate asking the player to sacrifice the one member the printed text
       excludes; Rust counted 0, matched < count, and took the declined branch
       (handlers.rs:333-341) WITHOUT opening a prompt. Both paired positives
       (the same card plus a non-excluded Liella! member, and Wonderful Rush's
       group-filtered gate) are unaffected -- see tests/test_audit_triggers.c A5. */
    if (optional && !is_activation && !is_from_hand && !strcmp(source, "stage")) {
        int stage_ids[RB_STAGE_SIZE];
        int n_stage = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] != RB_EMPTY_SLOT) stage_ids[n_stage++] = P->stage[i];
        RbCardFilter gate_filter;
        rb_effect_filter_subset(cost, &gate_filter);
        /* rb_count_matching_filter returns n when has_filter is 0, so a
           filterless optional stage cost still counts every occupied slot. */
        int matching = rb_count_matching_filter(&gate_filter, stage_ids, n_stage);
        if (matching < count) {
            return cost_auto_decline(g);
        }
        emit_pay_skip_gate(g, actor, cost,
                           "Put members from stage to waitroom (or skip)?",
                           1, "OptionalCost");
        return 1;
    }

    /* All-hand discard */
    if (is_from_hand && is_all) {
        int hand_len = P->hand.n;
        int is_optional = (optional || is_any_number) && !is_activation;
        if (hand_len == 0) {
            /* Rust offer_all_hand_discard (handlers.rs:326) only offers the
               yes/no choice when the hand holds cards; an empty hand auto-skips
               and opens NOTHING. */
            return cost_auto_decline(g);
        }
        RbChoice *pending = &g->queue.pending;
        memset(pending, 0, sizeof(*pending));
        pending->kind = RB_CHOICE_SELECT_TARGET;
        snprintf(pending->description, sizeof(pending->description),
                 "Discard entire hand (%d cards)?", hand_len);
        /* W2b. This prompt IS Rust's `Choice::SelectTarget { target:
           "pay_cost_all:discard_all" }` (cost/handlers.rs:378-388) and the target IS
           the routing key: choice.rs:2816-2818 dispatches on
           `SelectTargetKind::from_str(target)`, and enums.rs:705 spells it with a
           COLON. The C set `kind` and left `target` all-zero, so BOTH dispatchers
           (choice.c:5037 on `pending.target`, choice.c:3837 on `tgt`) had nothing to
           match and `rb_handle_pay_cost_all_discard` was unreachable for the 2
           corpus nodes measured by `make w-census`
           (PL!N-bp5-021-N and PL!N-bp5-028-L, ability #0 each).
           BLOCKED-ON choice.c:5037 / :3837 routes through
           `rb_select_target_kind_from_str`, which is exact -- so this producer MUST
           land with them or the routing fix stays inert.
           `pending` was memset two lines above, so this writes into a known-empty
           64-byte field; `sizeof(pending->target)` is the correct bound. */
        snprintf(pending->target, sizeof(pending->target), "%s",
                 "pay_cost_all:discard_all");
        pending->allow_skip = is_optional;
        pending->route = RB_ROUTE_OPTIONAL_COST;
        g->queue.has_pending = 1;
        g->queue.pending.route = RB_ROUTE_OPTIONAL_COST;
        cost_publish_pending(g, actor);
        return 1;
    }

    /* From hand with filtering. A mandatory self-discard never reaches here
       (handlers.rs:800-812) — it is paid directly below. */
    if (is_from_hand && !hand_self_cost) {
        int is_same_group_name = eff_extra(cost, "group_reference") &&
                                  !strcmp(eff_extra(cost, "group_reference"), "same_group_name");
        int matching_indices[RB_MAX_HAND];
        int n_matching = 0;

        if (is_same_group_name) {
            char group_names_buf[32][64];
            int group_counts[32];
            int n_groups = 0;
            for (int i = 0; i < P->hand.n && n_groups < 32; i++) {
                int cid = P->hand.cards[i];
                Card c;
                if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
                if (!c.group_idx) { rb_free_card(&c); continue; }
                const char *gn = rb_get_string(c.group_idx);
                if (!gn || !*gn) { rb_free_card(&c); continue; }
                int found = -1;
                for (int g = 0; g < n_groups; g++)
                    if (!strcmp(group_names_buf[g], gn)) { found = g; break; }
                if (found < 0) {
                    found = n_groups++;
                    strncpy(group_names_buf[found], gn, sizeof(group_names_buf[0]) - 1);
                    group_names_buf[found][sizeof(group_names_buf[0]) - 1] = '\0';
                    group_counts[found] = 0;
                }
                group_counts[found]++;
                rb_free_card(&c);
            }
            for (int i = 0; i < P->hand.n && n_matching < RB_MAX_HAND; i++) {
                int cid = P->hand.cards[i];
                Card c;
                if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
                if (!c.group_idx) { rb_free_card(&c); continue; }
                const char *gn = rb_get_string(c.group_idx);
                if (!gn || !*gn) { rb_free_card(&c); continue; }
                for (int g = 0; g < n_groups; g++) {
                    if (!strcmp(group_names_buf[g], gn) && group_counts[g] >= count) {
                        matching_indices[n_matching++] = i;
                        break;
                    }
                }
                rb_free_card(&c);
            }
        } else {
            const char *cost_limit = eff_extra(cost, "cost_limit");
            const char *group_names_cost = eff_extra(cost, "group_names");
            const char *chars = eff_extra(cost, "characters");
            /* handlers.rs:423-425 hands the filter the LIMIT and the
               OPERATOR, and util::card_matches_cost_limit defaults to `<=`
               (util.rs). The C compared `c.cost != atoi(cost_limit)`, i.e. an
               implicit `=`, so 「コスト4以下」 only ever accepted a card whose
               printed cost was exactly 4 and rejected every cheaper card. */
            const char *cost_limit_op = eff_extra(cost, "cost_limit_operator");
            if (!cost_limit_op || !*cost_limit_op) cost_limit_op = eff_extra(cost, "operator");
            for (int i = 0; i < P->hand.n && n_matching < RB_MAX_HAND; i++) {
                int cid = P->hand.cards[i];
                int match = 1;
                if (card_type && *card_type && !rb_card_matches_type(cid, card_type)) match = 0;
                if (match && cost_limit && *cost_limit &&
                    !rb_card_matches_cost_limit(cid, atoi(cost_limit), cost_limit_op))
                    match = 0;
                if (match && group_names_cost && *group_names_cost)
                    if (!card_matches_any_group(cid, group_names_cost)) match = 0;
                if (match && chars && *chars)
                    if (!card_matches_any_character(cid, chars)) match = 0;
                if (match) matching_indices[n_matching++] = i;
            }
        }

        /* handlers.rs:476-480, the last statement of hand_cost_matching_indices
         * before it returns `(indices, is_same_group_name)`:
         *     let indices = if distinct_should_dedupe(cost.distinct_any()) {
         *         dedupe_by_normalized_name(&indices, |&i| pl.hand.cards[i], card_db)
         *     } else { indices };
         * It runs on BOTH branches (Rust applies it to the whole `indices`, not
         * to the else-arm only), so it is placed after the if/else here too.
         * REACHABILITY (measured with the engine's own decoder over the corpus):
         * 2 of the 901 decoded cost nodes carry `distinct`, and both are exactly
         * this shape -- move_cards { source: hand, count: 3, optional: true,
         * distinct: "card_name", group_names: <X>, destination: discard }, i.e.
         * 「手札の名前の異なる『<X>』のメンバーカードを3枚控え室に置いてもよい」
         * (PL!-pb2-018-P＋ and its -R＋ sibling). The checklist's older "0 of 138
         * costs carry distinct" counted only TOP-LEVEL costs; the nodes live on
         * the abilities' costs and the count over all decoded cost nodes is 2. */
        if (rb_distinct_should_dedupe(cost_distinct_type(cost)) && n_matching > 1) {
            int deduped[RB_MAX_HAND];
            int n_deduped = cost_dedupe_indices_by_name(P, matching_indices,
                                                        n_matching, deduped,
                                                        RB_MAX_HAND);
            for (int i = 0; i < n_deduped; i++) matching_indices[i] = deduped[i];
            n_matching = n_deduped;
        }

        int is_optional = (optional || is_any_number) && !is_activation;

        if (!is_any_number && n_matching < count) {
            /* handlers.rs:513-523 — an optional hand cost short of `count`
               auto-skips: no prompt, cost_paid=true, result=false. */
            if (is_optional) return cost_auto_decline(g);
            return 0;
        }
        if (is_any_number && n_matching == 0) {
            if (is_optional) return cost_auto_decline(g);
            return 1;
        }

        int effective_count = is_any_number ? 0 : count;
        int max_str = is_any_number ? (eff_bool(cost, "max", 0) ? (count < n_matching ? count : n_matching) : n_matching) : n_matching;

        char desc[256];
        char desc_ja[256];
        char filter_desc[128];
        cost_filter_desc(cost, 0, filter_desc, sizeof filter_desc);
        /* handlers.rs:575-578 — only a hand->waitroom/discard cost announces
           the destination; every other shape prints no suffix. */
        const char *dest_opt = cost->destination;
        int is_hand_to_waitroom = !strcmp(source, "hand") && dest_opt &&
            (!strcmp(dest_opt, "discard") || !strcmp(dest_opt, "waitroom"));
        const char *dest_str = is_hand_to_waitroom ? " to waitroom" : "";
        if (is_any_number) {
            snprintf(desc, sizeof(desc),
                     "Select any number of %s%s from hand (0-%d)%s (or skip)",
                     cost_card_plural(max_str), filter_desc, max_str, dest_str);
        } else {
            snprintf(desc, sizeof(desc),
                     "Select %d %s%s%s%s",
                     effective_count, cost_card_plural(effective_count),
                     filter_desc, dest_str, is_optional ? " (or skip)" : "");
        }
        char filter_desc_ja[128];
        cost_filter_desc(cost, 1, filter_desc_ja, sizeof filter_desc_ja);
        if (is_any_number)
            snprintf(desc_ja, sizeof(desc_ja),
                     "手札から任意枚控え室に置く%s（スキップ可）", filter_desc_ja);
        else
            snprintf(desc_ja, sizeof(desc_ja), "手札から%d枚%s控え室に置く%s",
                     effective_count, filter_desc_ja,
                     is_optional ? "（スキップ可）" : "");

        RbChoice *pending = &g->queue.pending;
        memset(pending, 0, sizeof(*pending));
        pending->kind = RB_CHOICE_SELECT_CARD;
        strncpy(pending->zone, "hand", sizeof(pending->zone) - 1);
        pending->count = effective_count;
        pending->allow_skip = is_optional;
        pending->route = RB_ROUTE_SELECT_CARDS;
        strncpy(pending->description, desc, sizeof(pending->description) - 1);
        /* handlers.rs:614-626 builds the JA prompt from the SAME
           cost_filter_desc_ja whose twin this file previously lacked, and
           attaches it with `.description_ja(Some(..))`. */
        strncpy(pending->description_ja, desc_ja, sizeof(pending->description_ja) - 1);
        if (card_type && *card_type)
            strncpy(pending->card_type, card_type, sizeof(pending->card_type) - 1);
        const char *group_names_cost = eff_extra(cost, "group_names");
        if (group_names_cost && *group_names_cost)
            strncpy(pending->filter_group, group_names_cost, sizeof(pending->filter_group) - 1);
        /* handlers.rs:596-613 hands the cost's own filter to the choice. The
           C prompt carried only card_type + group_names, so the answer side
           (choice.c rb_resolver_handle_select_card →
           rb_resolver_execute_selected_cards_from_zone, which reads
           ch->cost_limit / ch->cost_limit_op / ch->characters) could not see
           「コスト4以下」 or 「〜のカード」 at all: an over-cost card stayed
           selectable and a card the cost filter rejects was accepted. */
        {
            const char *cl = eff_extra(cost, "cost_limit");
            if (cl && *cl) {
                pending->cost_limit = atoi(cl);
                const char *op = eff_extra(cost, "cost_limit_operator");
                if (!op) op = eff_extra(cost, "operation");
                if (op && *op)
                    strncpy(pending->cost_limit_op, op, sizeof(pending->cost_limit_op) - 1);
            }
            const char *chars = eff_extra(cost, "characters");
            if (chars && *chars) {
                char cbuf[256];
                strncpy(cbuf, chars, sizeof(cbuf) - 1);
                cbuf[sizeof(cbuf) - 1] = '\0';
                char *tok = strtok(cbuf, ",");
                while (tok && pending->n_characters < 16) {
                    strncpy(pending->characters[pending->n_characters], tok,
                            sizeof(pending->characters[0]) - 1);
                    pending->n_characters++;
                    tok = strtok(NULL, ",");
                }
            }
            snprintf(pending->target_player_id, sizeof(pending->target_player_id),
                     "p%d", tpl + 1);
        }
        g->queue.has_pending = 1;
        g->queue.pending.route = RB_ROUTE_SELECT_CARDS;
        cost_publish_pending(g, actor);
        return 1;
    }

    /* 「このカードを手札から控え室に置く」 — pay the activating card itself.
       Mirrors engine/src/ability/move_cards.rs:958-982: remove the activating
       card from hand (or, if it already left hand, from the waitroom). Rust
       reports an error when it is in neither, so the C port refuses too. */
    if (hand_self_cost) {
        int card_id = g->activating_card;
        if (card_id < 0) return 0;
        RbBag *from = NULL;
        for (int i = 0; i < P->hand.n; i++)
            if (P->hand.cards[i] == card_id) { from = &P->hand; break; }
        if (!from)
            for (int i = 0; i < P->discard.n; i++)
                if (P->discard.cards[i] == card_id) { from = &P->discard; break; }
        if (!from) return 0;
        int pos = 0;
        while (pos < from->n && from->cards[pos] != card_id) pos++;
        for (int i = pos; i + 1 < from->n; i++) from->cards[i] = from->cards[i + 1];
        from->n--;
        const char *dst = cost->destination ? cost->destination : "discard";
        if (!strcmp(dst, "discard") || !strcmp(dst, "waitroom"))
            bag_push(&P->discard, card_id);
        else if (!strcmp(dst, "energy_deck"))
            bag_push(&P->energy_deck, card_id);
        else if (!strcmp(dst, "energy"))
            bag_push(&P->energy, card_id);
        else if (!strcmp(dst, "hand"))
            bag_push(&P->hand, card_id);
        else if (!strcmp(dst, "deck"))
            bag_push(&P->deck, card_id);
        g->mods.last_cost_discard_count = 1;
        return 1;
    }

    /* handlers.rs:838-847 — the generic zone path. A same-unit cost is routed
       to its own offer FIRST (it needs a unit-grouped candidate set, not the
       generic zone count), then every other non-empty source is gated on
       check_zone_cost_availability. */
    if (*source) {
        if (same_unit)
            return offer_same_unit_cost(g, actor, cost, count, optional, is_activation);
        if (!check_zone_cost_availability(g, actor, cost, source, count)) return 0;
    }

    return execute_move_cards(g, actor, cost) >= 0;
}

/* ── pay_cost_change_state ── */
static int pay_cost_change_state(GameState *g, int actor, const AbilityEffect *cost) {
    g->n_last_cost_waited_members = 0;
    const char *sc = eff_extra(cost, "state_change");
    const char *state_change = sc ? sc : "";
    const char *target = cost->target ? cost->target : "self";
    int optional = cost->is_optional;
    int count = cost->count > 0 ? cost->count : 1;
    int exclude_self = eff_bool(cost, "exclude_self", 0);
    const char *card_type = cost_card_type_any(cost);
    const char *group_names = eff_extra(cost, "group_names");
    int self_cost = eff_bool(cost, "self_cost", 0);
    int tp = rb_resolve_target_player(g, target);
    int tpl = (tp >= 0) ? tp : actor;
    int is_activation = 0;

    {
        int cur = g->queue.cur;
        if (cur >= 0 && cur < g->queue.n_entries) {
            int cid = g->queue.entries[cur].card_id;
            int aidx = g->queue.entries[cur].ability_idx;
            Ability ab;
            if (rb_decode_card_ability((uint32_t)cid, aidx, &ab)) {
                is_activation = rb_resolver_current_ability_is_activation(&ab);
                rb_free_ability(&ab);
            }
        }
    }

    /* Optional gate for wait-state costs — emit pay/skip prompt and return */
    if (optional && !is_activation && !strcmp(state_change, "wait")) {
        int pos[RB_STAGE_SIZE];
        int n = get_change_state_candidates(g, tpl, target, card_type,
                                            group_names, exclude_self,
                                            self_cost, 0, "active",
                                            pos, RB_STAGE_SIZE);
        if (n == 0) return 1;   /* no eligible targets → auto-decline */
        emit_pay_skip_gate(g, actor, cost,
                           "Put this member to wait state", 1, "OptionalCost");
        return 1;
    }

    /* Mandatory cost: resolve all eligible candidates */
    int pos[RB_STAGE_SIZE];
    int n = get_change_state_candidates(g, tpl, target, card_type,
                                        group_names, exclude_self,
                                        self_cost, 1, "active",
                                        pos, RB_STAGE_SIZE);
    if (n == 0) return 0;   /* cannot pay — no matching members */

    if (n <= count) {
        /* Auto-wait all candidates */
        for (int i = 0; i < n; i++) {
            int cid = pos[i];
            if (!strcmp(state_change, "wait")) {
                rb_mods_set_orientation(&g->mods, cid, "wait");
                g->last_cost_waited_members[g->n_last_cost_waited_members++] = (int16_t)cid;
            } else if (!strcmp(state_change, "rest") || !strcmp(state_change, "rested")) {
                rb_mods_set_orientation(&g->mods, cid, "rest");
                g->last_cost_waited_members[g->n_last_cost_waited_members++] = (int16_t)cid;
            }
        }
    } else {
        /* Emit select_cards choice for player to pick exactly `count` members */
        char desc[128];
        char desc_ja[128];
        snprintf(desc, sizeof(desc), "Select %d stage member(s) to %s", count, state_change);
        snprintf(desc_ja, sizeof(desc_ja),
                 "ウェイトにするステージメンバーを%d体選択", count);

        RbChoice *pending = &g->queue.pending;
        memset(pending, 0, sizeof(*pending));
        pending->kind = RB_CHOICE_SELECT_CARD;
        strncpy(pending->zone, "stage", sizeof(pending->zone) - 1);
        pending->count = count;
        pending->allow_skip = 0;
        pending->route = RB_ROUTE_SELECT_CARDS;
        strncpy(pending->description, desc, sizeof(pending->description) - 1);
        if (card_type && *card_type)
            strncpy(pending->card_type, card_type, sizeof(pending->card_type) - 1);
        if (group_names && *group_names)
            strncpy(pending->filter_group, group_names, sizeof(pending->filter_group) - 1);
        g->queue.has_pending = 1;
        s_stage_select_intent = 1;
    }
    return 1;
}

/* ── pay_cost_inner ── */
static int pay_cost_inner(GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;

    /* handlers.rs:974-975 —
     *     let mut dbg = AbDebug::new();
     *     dbg.cost_pay(cost, true);
     * the FIRST two statements of pay_cost_inner, ahead of the `match
     * cost.action` dispatch and therefore ahead of every arm below, including
     * the SequentialCost arm. Rust builds a FRESH AbDebug per invocation (a
     * stack local, `indent: 0`), so these lines carry no indent pad and do not
     * ride resolve_ability's tracer: reproduced here with a by-value
     * `RbAbDebug dbg` — it is a COMPLETE struct in include/rabuka.h, so it must
     * never be declared as a pointer — exactly as condition.c:602 and
     * resolver.c:717 do for their own per-call tracers. The tracer shares the
     * one global 256-line ABILITY_DEBUG buffer (debug.rs:66), flushed later by
     * rb_resolver_resolve_ability (resolver.c:1198-1199), as in Rust.
     *
     * `true` is Rust's literal second argument: pay_cost_inner reports the
     * cost as paid on entry, BEFORE any arm runs, so a cost that later fails
     * still emits "... U+2192 OK". Reproduced verbatim: the FAIL spelling
     * exists in the tracer (debug.rs:158) but nothing in Rust ever passes it.
     * The arrow itself lives in debug.c as an explicit \xE2\x86\x92 escape;
     * it is named here in words so no literal U+2192 enters this file. */
    RbAbDebug dbg;
    rb_abdebug_init(&dbg);
    rb_abdebug_cost_pay(&dbg, cost, 1);

    /* Rule 9.4.2 / Q234: SequentialCost — validate all, then pay in order */
    if (cost_is_sequential(cost)) {
        int start_idx = 0;
        {
            int cur = g->queue.cur;
            if (cur >= 0 && cur < g->queue.n_entries)
                start_idx = g->queue.entries[cur].cost_paid_index;
        }
        for (int i = start_idx; i < cost->n_child; i++)
            if (!validate_cost(g, actor, cost->child[i]))
                return 0;

        int had_binary_auto_pay = 0;
        for (int i = start_idx; i < cost->n_child; i++) {
            const AbilityEffect *sub = cost->child[i];
            int is_binary = sub->is_optional && has_skip_prompt(sub);
            int has_choice_ahead = 0;
            for (int j = i + 1; j < cost->n_child; j++)
                if (!has_skip_prompt(cost->child[j])) { has_choice_ahead = 1; break; }
            if (is_binary && has_choice_ahead) {
                /* Defer: auto-pay without a prompt (clone + strip optionality,
                   mirrors Rust pending_deferred_costs.push(auto)) */
                if (g->queue.n_pending_deferred_costs >= RB_MAX_CHILD) return 0;
                AbilityEffect *deferred = rb_effect_deep_clone(sub);
                if (!deferred) return 0;
                deferred->is_optional = 0;
                g->queue.pending_deferred_costs[g->queue.n_pending_deferred_costs++] = deferred;
                /* Open the batch: remember which card it belongs to so the
                   deferred `self_cost` leg still finds the member being placed
                   when the batch is paid from the choice epilogue. */
                if (g->queue.n_pending_deferred_costs == 1) {
                    int host = g->activating_card;
                    if (host < 0 && g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries &&
                        g->queue.entries[g->queue.cur].card_id >= 0)
                        host = g->queue.entries[g->queue.cur].card_id;
                    s_cost_deferred_host = host;
                }
                had_binary_auto_pay = 1;
            } else {
                if (!pay_cost_inner(g, actor, sub)) return 0;
            }
            /* Update cost_paid_index on the current queue entry */
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
                g->queue.entries[g->queue.cur].cost_paid_index = i + 1;
            /* If a pending choice was emitted, override its description to
               show the combined (auto-paid + pending) cost text */
            if (g->queue.has_pending && had_binary_auto_pay) {
                RbChoice *pending = &g->queue.pending;
                (void)pending;
                had_binary_auto_pay = 0;
            }
            if (g->queue.has_pending) return 1;
        }
        return 1;
    }

    /* ChoiceCondition */
    if (cost->action && !strcmp(cost->action, "choice_condition")) {
        char opts[8][128];
        int n_opts = 0;
        for (int i = 0; i < cost->n_child && n_opts < 8; i++) {
            if (cost->child[i] && cost->child[i]->text) {
                strncpy(opts[n_opts], cost->child[i]->text, sizeof(opts[0]) - 1);
                opts[n_opts][sizeof(opts[0]) - 1] = '\0';
                n_opts++;
            }
        }
        char desc[256] = "Choose cost option: ";
        for (int i = 0; i < n_opts; i++) {
            if (i > 0) strncat(desc, " OR ", sizeof(desc) - strlen(desc) - 1);
            strncat(desc, opts[i], sizeof(desc) - strlen(desc) - 1);
        }

        RbChoice *pending = &g->queue.pending;
        memset(pending, 0, sizeof(*pending));
        pending->kind = RB_CHOICE_SELECT_TARGET;
        strncpy(pending->target, "choice_condition", sizeof(pending->target) - 1);
        strncpy(pending->description, desc, sizeof(pending->description) - 1);
        pending->allow_skip = 0;
        pending->route = RB_ROUTE_CHOICE_COST;
        g->queue.has_pending = 1;
        g->queue.pending.route = RB_ROUTE_CHOICE_COST;
        if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
            g->queue.entries[g->queue.cur].choice_card_no = RB_ROUTE_CHOICE_COST;
        cost_publish_pending(g, actor);
        return 1;
    }

    /* MoveCards */
    if (cost_is_move_cards(cost)) return pay_cost_move_cards(g, actor, cost);

    /* ChangeState */
    if (cost_is_change_state(cost)) return pay_cost_change_state(g, actor, cost);

    /* PayEnergy */
    if (cost_is_energy(cost)) {
        int energy = eff_energy_count(cost, 0);
        const char *target = cost->target ? cost->target : "self";
        int optional = cost->is_optional;
        int any_number = eff_bool(cost, "any_number", 0);
        int tp = rb_resolve_target_player(g, target);
        int tpl = (tp >= 0) ? tp : actor;
        RbPlayer *P = &g->p[tpl];
        int is_activation = 0;

        {
            int cur = g->queue.cur;
            if (cur >= 0 && cur < g->queue.n_entries) {
                int cid = g->queue.entries[cur].card_id;
                int aidx = g->queue.entries[cur].ability_idx;
                Ability ab;
                if (rb_decode_card_ability((uint32_t)cid, aidx, &ab)) {
                    is_activation = rb_resolver_current_ability_is_activation(&ab);
                    rb_free_ability(&ab);
                }
            }
        }

        /* any_number: show active energy for selection */
        if (any_number && (optional || !is_activation)) {
            int active_count = rb_energy_active_count(P);
            if (active_count == 0) {
                /* handlers.rs:1078-1089 — no active energy, treat as skip. */
                return cost_auto_decline(g);
            }
            RbChoice *pending = &g->queue.pending;
            memset(pending, 0, sizeof(*pending));
            pending->kind = RB_CHOICE_SELECT_CARD;
            strncpy(pending->zone, "energy", sizeof(pending->zone) - 1);
            pending->count = 0;
            pending->allow_skip = 1;
            pending->route = RB_ROUTE_OPTIONAL_COST;
            char desc[128];
            snprintf(desc, sizeof(desc),
                     "Select energy card to pay (active: %d). Skip when done",
                     active_count);
        strncpy(pending->description, desc, sizeof(pending->description) - 1);
        /* handlers.rs:1133-1136 attaches this arm's JA prompt. */
        snprintf(pending->description_ja, sizeof(pending->description_ja),
                 "エネルギーカードを選択（アクティブ: %d）完了でスキップ", active_count);
            g->queue.has_pending = 1;
            g->queue.pending.route = RB_ROUTE_OPTIONAL_COST;
            s_pending_energy_payment = 0;
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
                g->queue.entries[g->queue.cur].choice_card_no = RB_ROUTE_OPTIONAL_COST;
            cost_publish_pending(g, actor);
            return 1;
        }

        /* Optional fixed-count energy */
        if (optional && !is_activation) {
            int active = rb_energy_active_count(P);
            if (active < energy) {
                /* handlers.rs:1119-1127 — unaffordable optional energy skips
                   without opening the pay/skip gate. */
                return cost_auto_decline(g);
            }
            emit_pay_skip_gate(g, actor, cost,
                               "Pay energy (or skip)?", 1, "OptionalCost");
            return 1;
        }

        /* Baton touch zero cost */
        if (g->baton_touch_zero_cost && energy > 0) return 1;

        /* Actual payment */
        if (energy > 0) rb_energy_pay(P, energy);
        rb_recalc_constants(g);
        return 1;
    }

    /* EnergyCondition */
    if (cost_is_energy_condition(cost)) {
        int count = cost->count > 0 ? cost->count : 1;
        const char *target = cost->target ? cost->target : "self";
        int tp = rb_resolve_target_player(g, target);
        int tpl = (tp >= 0) ? tp : actor;
        RbPlayer *P = &g->p[tpl];
        if ((int)P->energy.n < count) return 0;
        for (int i = 0; i < count; i++) {
            if (P->energy.n > 0) {
                int cid = P->energy.cards[--P->energy.n];
                if (P->energy_deck.n < RB_MAX_ZONE)
                    P->energy_deck.cards[P->energy_deck.n++] = cid;
            }
        }
        rb_energy_sub_active(P, count);
        return 1;
    }

    /* Reveal */
    if (cost_is_reveal(cost)) {
        const char *source = cost->source ? cost->source : "hand";
        const char *target = cost->target ? cost->target : "self";
        const char *card_type = cost_card_type_any(cost);
        int tp = rb_resolve_target_player(g, target);
        int tpl = (tp >= 0) ? tp : actor;
        RbPlayer *P = &g->p[tpl];
        int card_ids[RB_MAX_HAND];
        int n_ids = 0;
        int optional = eff_bool(cost, "optional", 0);

        if (!strcmp(source, "hand")) {
            const char *cost_values = eff_extra(cost, "cost_values");
            const char *group_names_cost = eff_extra(cost, "group_names");
            int has_cost_values = cost_values && *cost_values;
            int vals[16], nvals = 0;
            if (has_cost_values) {
                char vbuf[256];
                strncpy(vbuf, cost_values, sizeof(vbuf) - 1);
                vbuf[sizeof(vbuf) - 1] = '\0';
                char *tok = strtok(vbuf, ",");
                while (tok && nvals < 16) {
                    vals[nvals++] = atoi(tok);
                    tok = strtok(NULL, ",");
                }
            }
            for (int i = 0; i < P->hand.n && n_ids < RB_MAX_HAND; i++) {
                int cid = P->hand.cards[i];
                int match = 1;
                if (card_type && *card_type && !rb_card_matches_type(cid, card_type)) match = 0;
                if (match && has_cost_values) {
                    Card c;
                    if (rb_decode_card_by_index((uint32_t)cid, &c)) {
                        /* Rust handlers.rs:1214-1218 compares the card's
                           `cost`, falling back to `score` only when the card
                           carries no cost (`.cost.or(.score)`). Matching either
                           field independently would let a cheap member with a
                           matching printed score satisfy 「コストが10か20」. */
                        int face = c.cost ? (int)c.cost : rb_card_get_score(&c);
                        int found = 0;
                        for (int v = 0; v < nvals; v++)
                            if (face == vals[v]) { found = 1; break; }
                        if (!found) match = 0;
                        rb_free_card(&c);
                    } else match = 0;
                }
                if (match && group_names_cost && *group_names_cost)
                    if (!card_matches_any_group(cid, group_names_cost)) match = 0;
                if (match) card_ids[n_ids++] = cid;
            }
        }

        if (n_ids == 0) return 0;

        /* Dedup */
        for (int i = 0; i < n_ids; i++)
            for (int j = i + 1; j < n_ids;)
                if (card_ids[i] == card_ids[j]) {
                    card_ids[j] = card_ids[--n_ids];
                } else j++;

        int has_explicit_count = cost->count > 0;
        int explicit_count = cost->count > 0 ? cost->count : 1;

        /* handlers.rs:1260-1283 — a mandatory reveal cost whose eligible set is
           exactly `explicit_count` wide reveals without opening a choice. */
        if (!optional && has_explicit_count && n_ids == explicit_count) {
            int effect_uses_selected =
                (rb_entry_effect(g) && effect_uses_selected_cards(rb_entry_effect(g)));
            for (int i = 0; i < n_ids; i++) {
                if (g->n_revealed < RB_MAX_REVEALED_CARDS)
                    g->revealed_cards[g->n_revealed++] = card_ids[i];
            }
            if (effect_uses_selected) {
                g->n_selected_cards = 0;
                for (int i = 0; i < n_ids && i < RB_MAX_ZONE; i++)
                    g->selected_cards[i] = card_ids[i];
                g->n_selected_cards = n_ids < RB_MAX_ZONE ? n_ids : RB_MAX_ZONE;
            }
            return 1;
        }

        RbChoice *pending = &g->queue.pending;
        memset(pending, 0, sizeof(*pending));
        pending->kind = RB_CHOICE_SELECT_CARD;
        strncpy(pending->zone, source, sizeof(pending->zone) - 1);
        pending->count = has_explicit_count ? explicit_count : 0;
        pending->allow_skip = 1;
        pending->route = RB_ROUTE_SELECT_CARDS;
        strncpy(pending->description, "Select cards to reveal from hand",
                sizeof(pending->description) - 1);
        if (card_type && *card_type)
            strncpy(pending->card_type, card_type, sizeof(pending->card_type) - 1);
        const char *group_names_cost = eff_extra(cost, "group_names");
        if (group_names_cost && *group_names_cost)
            strncpy(pending->filter_group, group_names_cost, sizeof(pending->filter_group) - 1);
        g->queue.has_pending = 1;
        cost_publish_pending(g, actor);
        return 1;
    }

    /* PlaceEnergyUnderMember */
    if (cost_is_place_energy_under_member(cost)) {
        return execute_place_energy_under_member_non_optional(g, actor, cost);
    }

    /* Custom with UnderMember destination */
    if (cost_is_custom_under_member(cost)) {
        return execute_place_energy_under_member_non_optional(g, actor, cost);
    }

    return 1;
}

/* ── Public: rb_pay_cost ── */
int rb_pay_cost(GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;
    s_cost_optional_skipped = 0;
    int result = pay_cost_inner(g, actor, cost);
    if (result && !g->queue.has_pending) {
        const char *pp = player_prefix(g, g->activating_card);
        const char *act_name = "";
        if (g->activating_card >= 0) act_name = rb_card_string((uint16_t)g->activating_card);
        const char *cost_desc = cost->text ? cost->text : "";
        char logbuf[256];
        snprintf(logbuf, sizeof(logbuf), "%s %s: [cost] %s", pp,
                 act_name[0] ? act_name : "", cost_desc);
        rb_log_push_verdict(logbuf, "cost", 1);
    }
    return result;
}

/* ── Public: rb_validate_cost ── */
int rb_validate_cost(const GameState *g, int actor, const AbilityEffect *cost) {
    return validate_cost(g, actor, cost);
}

/* ── Public: rb_validate_mandatory_cost (handlers.rs:281-309) ─────────────────
   The offer/activation twin of rb_validate_cost. `activating_card` is the card
   under consideration, NOT g->activating_card — see the note above. */
int rb_validate_mandatory_cost(const GameState *g, int actor,
                               const AbilityEffect *cost, int activating_card) {
    if (activating_card < 0) activating_card = RB_ACTIVATING_NONE;
    return validate_mandatory_cost(g, actor, cost, activating_card);
}

/* ── Public: rb_pay_deferred_costs ──
   Mirrors Rust cost.rs:17-25: pay every stored deferred cost in order, then
   clear the list. Returns 0 (and clears the list) if any cost cannot be paid. */
int rb_pay_deferred_costs(GameState *g, int actor, const AbilityEffect *cost) {
    (void)cost;
    if (!g || actor < 0 || actor > 1) return 0;
    AbilityEffect *deferred[RB_MAX_CHILD];
    int n = g->queue.n_pending_deferred_costs;
    for (int i = 0; i < n; i++) {
        deferred[i] = g->queue.pending_deferred_costs[i];
        g->queue.pending_deferred_costs[i] = NULL;
    }
    g->queue.n_pending_deferred_costs = 0;
    /* Republish the batch's host for the duration of the payment — the play
       action that deferred it has already restored g->activating_card, while
       Rust's resolver.rs:1161 window is still open here. */
    int saved_activating = g->activating_card;
    int host = s_cost_deferred_host;
    s_cost_deferred_host = -1;
    if (host >= 0) g->activating_card = host;
    int result = 1;
    for (int i = 0; i < n; i++) {
        if (result && !rb_pay_cost(g, actor, deferred[i])) result = 0;
        rb_effect_free(deferred[i]);
    }
    g->activating_card = saved_activating;
    return result;
}

/* ── DELETED: rb_cost_clear_deferred ─────────────────────────────────────────
   This was the port of Rust `AbilityResolver::discard_deferred_costs`
   (engine/src/ability/cost/handlers.rs:35-37). It is deleted because it has no
   live caller on EITHER side of the port:

     - Rust: `discard_deferred_costs` is `pub`, but nothing in engine/src calls
       it. The only other two touches of `awaiting.deferred_costs` are the push
       at handlers.rs:1011 and an inline
       `resolver.awaiting.deferred_costs.clear()` in
       engine/src/ability/compound/conditional_on.rs:209 — the clear is spelled
       out at its one real use site, which is what makes this method dead.
     - C: nothing outside this file referenced rb_cost_clear_deferred, and this
       file's own skip/decline arms never called it either.

   The only remaining reason it existed was a false symmetry with
   pay_deferred_costs (handlers.rs:20), which IS live — engine/src/ability/
   choice.rs:210 and :916 call it, and the C mirrors that at choice.c:702,
   :1501 and :2153.

   include/rabuka.h:2523 still declares it; that prototype is now orphaned and
   should go with this change. Leaving it does not break the link (nothing
   references it), but it advertises a port that no longer exists. */

/* ── cost_entry_route_is_optional ──
   The `is_effect_optional` of handlers.rs:1619
   (`gs.entry_choice_card_no() == Some(ChoiceRoute::OptionalCost)`), read off
   the current ENTRY's `choice_card_no` — which is what Rust reads
   (core/game_state/abilities.rs:2365-2369: `current_entry().choice_card_no`).

   NOT `rb_entry_choice_card_no` (game_state_abilities.c:3154): that one reads
   `g->queue.pending.route` behind a `has_pending` test, a different carrier,
   and rb_handle_optional_cost_payment clears has_pending on the first line of
   both of its arms — so through that accessor the test is constant-false here
   and the arm it gates could never run. The entry field is the field Rust
   reads, and it is exactly what emit_pay_skip_gate (cost.c:353) and the
   any-number energy prompt (cost.c:1154) stamp. Entries are zero-initialised
   (ability_queue.c:219/251) and RB_ROUTE_NONE is 0, so an entry that never
   opened a pay/skip gate reports NONE and cannot match. */
static int cost_entry_route_is_optional(const GameState *g) {
    if (!g) return 0;
    if (g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return 0;
    return g->queue.entries[g->queue.cur].choice_card_no == RB_ROUTE_OPTIONAL_COST;
}

/* ── Public: rb_handle_optional_cost_payment ── */
int rb_handle_optional_cost_payment(GameState *g, int actor, const AbilityEffect *cost, int pay) {
    g->n_last_cost_waited_members = 0;

    if (!pay) {
        /* ── SKIP path ── */
        /* handlers.rs:1373-1394 — cost_paid=true, optional_cost_result=false,
           effect_started=false: the POST_COST optional_cost_skip_gate stops the
           resolution, so the ability's effect never starts. */
        s_cost_optional_skipped = 1;
        g->queue.has_pending = 0;
        s_pending_energy_payment = 0;
        if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
            g->queue.entries[g->queue.cur].cost_paid = 1;
            g->queue.entries[g->queue.cur].optional_cost_result = 0;
        }
        /* Execute the alternative effect ("unless you pay") immediately.
           In the C port pending_actions is a counter only (no effect-pointer
           array), so we execute the alternative directly rather than enqueuing
           it for the drain loop to find. */
        if (cost && cost->alternative_effect) {
            rb_execute_effect_ex(g, actor, (AbilityEffect *)cost->alternative_effect,
                                 g->activating_card);
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
                g->queue.entries[g->queue.cur].effect_started = 1;
        }
        /* handlers.rs:1416-1420 REPLACES the batch with `vec![alt_effect]`, or
           clears it when the cost carries no alternative; :1422 then resumes. The
           alternative is executed directly above exactly as Rust's resume would run
           it out of the batch, so what remains to do here is the REPLACE — the batch
           becomes empty — which is cost_drop_pending_actions, not the old
           take-and-execute loop. */
        cost_drop_pending_actions(g);
        return cost_arm_resume(g);
    }

    /* ── PAY path ── */
    s_cost_optional_skipped = 0;
    g->queue.has_pending = 0;

    /* Pending energy payment from any_number select */
    if (s_pending_energy_payment > 0) {
        int ep = s_pending_energy_payment;
        s_pending_energy_payment = 0;
        RbPlayer *P = &g->p[actor];
        if (rb_energy_active_count(P) >= ep)
            rb_energy_pay(P, ep);
        else {
            /* handlers.rs:1404-1409 — the pending energy could not be tapped,
               so the cost is effectively declined: no prompt of its own is
               opened and the resolution stops at the skip gate. */
            s_cost_optional_skipped = 1;
            /* handlers.rs:1433-1437: cancel_remaining_commands is raised and the
               tail is CLEARED (not executed), so the resume that follows finds
               nothing to run. The old body zeroed `pending_actions_n` by hand,
               which orphaned the batch's owned deep clones — ability_queue.c's
               ownership contract (ability_queue.c:18-45) admits exactly three
               release paths and a raw field write is none of them. */
            cost_drop_pending_actions(g);
            g->queue.cancel_remaining_commands = 1;
            return cost_arm_resume(g);
        }
    }

    /* `cost` is the entry's OWN cost: choice.c:3831 passes rb_entry_cost(g), so
       it is Rust's `gs.entry_cost().cloned()` at handlers.rs:1482. Every block
       below is guarded on it, exactly as `if let Some(cost) = ...` is. */

    /* handlers.rs:1440-1441 — both flags are read BEFORE the entry is stamped
       at :1462-1471, and the second one decides whether that stamping is
       allowed to stand. */
    int has_pending = rb_queue_has_pending_actions(g);
    int has_cost = (rb_entry_cost(g) != NULL);

    /* handlers.rs:1462-1471 — the pay answer stamps the entry DECIDED. Before
       this the C stamped cost_paid / optional_cost_result only inside the
       move_cards accept arm, so an energy gate and a change_state gate — the
       other two shapes emit_pay_skip_gate is opened for — left the entry
       un-paid after the player said yes: cost_paid stayed 0 (so the
       POST_COST gates still considered the cost outstanding) and
       optional_cost_result stayed at its zero-init 0, which is Rust's
       Some(false) — i.e. the C reported a DECLINE for a payment.

       Rust also resets optional_cost_result to None and re-arms the effect for
       the `!has_cost && is_gated_effect_move` shape (an optional gate opened by
       the EFFECT, not by a cost). That re-arm is NOT reproduced here: the C's
       `decided` test for the shared gate is `g->queue.resume_mode == 5`
       (move_missing.c:379), a C-only carrier that the choice-resume tail
       clears, so there is nowhere in cost.c to leave a durable "already
       decided" mark. Setting effect_started = 0 without one would re-open the
       very gate the player just answered. See the KEEP-REPORT note. */
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
        g->queue.entries[g->queue.cur].cost_paid = 1;
        g->queue.entries[g->queue.cur].optional_cost_result = 1;
    }
    /* handlers.rs:1476-1478 — a payment is recorded only when a cost was
       actually paid. rb_record_cost_paid (core/modifiers.c, game_state/
       mod.rs:404-414) had NO writer tree-wide; this is Rust call site #2 of
       the five (handlers.rs:1389 is accept_optional_move_cost, below). */
    if (has_cost) rb_record_cost_paid(g);

    /* handlers.rs:1479-1481 — the parked tail SHORT-CIRCUITS the whole
       entry-cost block. `has_pending` is read at :1440, before the entry is
       stamped at :1462-1471, and the arm at :1479 returns
       `self.resume_pending_actions(gs)` without ever reaching
       `if let Some(cost) = gs.entry_cost().cloned()` at :1482. So when the
       entry already owns a parked batch, answering "pay" does NOT tap the
       gate's energy, does NOT put the member to wait, does NOT re-enter
       pay_cost_move_cards for the gated stage/energy source, and does NOT walk
       the sequential sub-costs — the batch is the only thing that runs.

       The C had no such test and fell straight through into all four, so a
       pay answer on a queued entry re-charged the cost the gate had already
       been opened for and re-entered the move it was about to accept.

       `cost_arm_resume` is the C's spelling of the returned resume: per
       cost_arm_resume above it does NOT execute the tail, because on this
       call path the C's single execution point, rb_resolver_continue_siblings
       (choice.c:4361), has still not run when this function returns. The batch
       stays parked for it, exactly as it would for the two arms at the bottom
       of this function. Re-adding an execution here is the double-count that
       two previous agents measured as a regression. */
    if (has_pending) return cost_arm_resume(g);

    if (cost) {
        /* handlers.rs:1483-1490 — answering "pay" taps the gated cost's energy.
           The gate is only ever opened when the printed amount is affordable
           (handlers.rs:1150-1160 auto-skips otherwise), so an unaffordable
           re-entry leaves the energy untouched rather than partially paying. */
        int gate_energy = eff_energy_count(cost, 0);
        if (gate_energy > 0) {
            const char *tgt = cost->target ? cost->target : "self";
            int tp = rb_resolve_target_player(g, tgt);
            int tpl = (tp >= 0) ? tp : actor;
            if (rb_energy_active_count(&g->p[tpl]) >= gate_energy)
                rb_energy_pay(&g->p[tpl], gate_energy);
        }

        /* handlers.rs:1491-1576 — the change_state cost's own payment. This
           block was ABSENT, and its absence was not inert: the only gate
           pay_cost_change_state opens is this one (cost.c:968-978), so a
           「○○を1人ウェイトにしてもよい」 cost opened its pay/skip prompt and
           then, on "pay", did nothing at all — the member was never put to
           wait, the wait was never recorded in last_cost_waited_members, and
           the 「これによりウェイト状態になった」 gains that read that list
           (effects/ability_ops.rs:127-129) had nothing to bind to. */
        const char *cs_state = eff_extra(cost, "state_change");
        if (cs_state && !strcmp(cs_state, "wait")) {
            if (eff_bool(cost, "self_cost", 0)) {
                /* handlers.rs:1492-1514 — Q159: the member must be on stage;
                   Q137 / Rule 1.3.2.1: an already-waited member is left alone
                   (the act itself does not happen, so no error either). */
                int self_id = g->activating_card;
                if (self_id >= 0) {
                    int on_stage = 0;
                    for (int s = 0; s < RB_STAGE_SIZE; s++)
                        if (g->p[actor].stage[s] == self_id) { on_stage = 1; break; }
                    if (!on_stage) return 0;   /* Err: "member is not on stage" */
                    const char *ori = rb_mods_get_orientation(&g->mods, self_id);
                    if (!ori || strcmp(ori, "wait") != 0)
                        rb_mods_set_orientation(&g->mods, self_id, "wait");
                }
            } else {
                /* handlers.rs:1515-1537 — Q183: always your OWN stage. */
                const char *tgt = cost->target ? cost->target : "self";
                int tp = rb_resolve_target_player(g, tgt);
                int tpl = (tp >= 0) ? tp : actor;
                int ccount = rb_effect_count_or(cost, 1);
                int exclude_self = eff_bool(cost, "exclude_self", 0);
                const char *c_card_type = cost_card_type_any(cost);
                const char *c_groups = eff_extra(cost, "group_names");
                int pos[RB_STAGE_SIZE];
                int n = get_change_state_candidates(g, tpl, tgt, c_card_type,
                                                    c_groups, exclude_self,
                                                    0, 0, "active", pos,
                                                    RB_STAGE_SIZE);
                /* Q137 / Rule 1.3.2.1: no ACTIVE member to wait — the cost
                   cannot be paid. */
                if (n == 0) return 0;
                if (n <= ccount) {
                    /* handlers.rs:1539-1547 */
                    for (int i = 0; i < n; i++) {
                        rb_mods_set_orientation(&g->mods, pos[i], "wait");
                        if (g->n_last_cost_waited_members < RB_MAX_ZONE)
                            g->last_cost_waited_members[g->n_last_cost_waited_members++] =
                                (int16_t)pos[i];
                    }
                } else {
                    /* handlers.rs:1548-1574 — ask which member(s) to wait. */
                    RbChoice *pending = &g->queue.pending;
                    memset(pending, 0, sizeof(*pending));
                    pending->kind = RB_CHOICE_SELECT_CARD;
                    strncpy(pending->zone, "stage", sizeof(pending->zone) - 1);
                    pending->count = ccount;
                    pending->allow_skip = 0;
                    pending->route = RB_ROUTE_SELECT_CARDS;
                    snprintf(pending->description, sizeof(pending->description),
                             "Select %d stage member(s) to %s", ccount, cs_state);
                    snprintf(pending->description_ja, sizeof(pending->description_ja),
                             "ステージのメンバーを%d人を%sにする", ccount,
                             rb_desc_state_verb_ja(cs_state));
                    if (c_card_type && *c_card_type)
                        strncpy(pending->card_type, c_card_type,
                                sizeof(pending->card_type) - 1);
                    if (c_groups && *c_groups)
                        strncpy(pending->filter_group, c_groups,
                                sizeof(pending->filter_group) - 1);
                    snprintf(pending->target_player_id,
                             sizeof(pending->target_player_id), "p%d", tpl + 1);
                    g->queue.has_pending = 1;
                    s_stage_select_intent = 1;
                    cost_publish_pending(g, actor);
                    return 1;
                }
            }
        }

        /* handlers.rs:1577-1590 / accept_optional_move_cost (:1377-1391) —
           「メンバーを控え室に置いてもよい：…」 and 「エネルギー置き場にある
           エネルギー1枚をエネルギーデッキに置いてもよい：…」. The player
           accepted, so the move runs with optionality stripped, or it loops
           back to the same gate. Optionality is stripped, cost_paid /
           optional_cost_result are stamped, and the payment is RECORDED
           (handlers.rs:1389). */
        if (cost->action && !strcmp(cost->action, "move_cards")) {
            const char *src = cost->source ? cost->source : "";
            /* handlers.rs:1584-1585 compares the resolved ZONE, so the
               energy_zone wire alias (ability/enums.rs:98) counts too. */
            int from_stage = !strcmp(src, "stage");
            int energy_to_deck = (!strcmp(src, "energy") ||
                                  !strcmp(src, "energy_zone")) && cost->destination &&
                                 !strcmp(cost->destination, "energy_deck");
            if (from_stage || energy_to_deck) {
                AbilityEffect committed = *cost;
                committed.is_optional = 0;
                if (!pay_cost_move_cards(g, actor, &committed)) return 0;
                if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
                    g->queue.entries[g->queue.cur].cost_paid = 1;
                    g->queue.entries[g->queue.cur].optional_cost_result = 1;
                }
                rb_record_cost_paid(g);
                return 1;
            }
        }

        /* handlers.rs:1592-1607 — sequential_cost sub-costs, paid after the
           user confirmed. */
        if (cost_is_sequential(cost)) {
            for (int i = 0; i < cost->n_child; i++) {
                const AbilityEffect *sub = cost->child[i];
                const char *sc = sub ? eff_extra(sub, "state_change") : NULL;
                if (sub && sc && !strcmp(sc, "wait") && eff_bool(sub, "self_cost", 0)) {
                    int self_id = g->activating_card;
                    if (self_id >= 0)
                        rb_mods_set_orientation(&g->mods, self_id, "wait");
                } else if (!rb_pay_cost(g, actor, sub)) {
                    /* handlers.rs:1600-1602 — a sub-cost that cannot be paid is
                       logged and stepped over; the loop continues. */
                }
                if (g->queue.has_pending) return 1;
            }
        }

        /* handlers.rs:1614-1616 */
        if (cost->action && !strcmp(cost->action, "place_energy_under_member")) {
            execute_place_energy_under_member_non_optional(g, actor, cost);
        }
    }

    /* ── After all costs paid: run the entry's effect ── */
    g->queue.has_pending = 0;
    int effect_started = 0;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
        effect_started = g->queue.entries[g->queue.cur].effect_started;

    const AbilityEffect *entry_cost = rb_entry_cost(g);
    if (entry_cost && !effect_started) {
        const AbilityEffect *entry_eff = rb_entry_effect(g);
        if (entry_eff) {
            if (entry_eff->action && !strcmp(entry_eff->action, "place_energy_under_member")) {
                execute_place_energy_under_member_non_optional(g, actor, entry_eff);
            } else if (entry_eff->action) {
                rb_execute_effect_ex(g, actor, (AbilityEffect *)entry_eff, g->activating_card);
            }
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
                g->queue.entries[g->queue.cur].effect_started = 1;
        }
    } else if (rb_queue_has_pending_actions(g)) {
        /* handlers.rs:1655-1658 — Rust resumes here. The batch must still be
           parked when this arm returns, because rb_resolver_continue_siblings is
           the C's single consumer of it and runs on the way back out (choice.c:4927).
           See cost_arm_resume. */
        cost_arm_resume(g);
    } else if (cost_entry_route_is_optional(g)) {
        /* handlers.rs:1659-1662 — the THIRD arm of that if/else-if chain, and
           the C had no counterpart for it at all. `is_effect_optional` is
           read at :1619, BEFORE the chain:
           `let is_effect_optional = gs.entry_choice_card_no() ==
                                   Some(ChoiceRoute::OptionalCost);`
           — i.e. the pay/skip gate this answer closes, which emit_pay_skip_gate
           stamps onto the entry as RB_ROUTE_OPTIONAL_COST (cost.c:353), as
           does the any-number energy prompt at cost.c:1154.

           The arm is reachable exactly when the entry has NO cost of its own
           (so the first arm cannot fire), the effect has not started, and no
           batch is parked: the pay/skip gate was opened by the EFFECT, not by
           a cost, and the C's previous shape dropped the answer on the floor.
           Rust runs the effect through
           execute_place_energy_under_member_non_optional — the same
           non-optional call the two arms above use, which is what keeps an
           optional place-energy effect from re-opening this very gate. */
        const AbilityEffect *entry_eff = rb_entry_effect(g);
        if (entry_eff)
            execute_place_energy_under_member_non_optional(g, actor, entry_eff);
    }
    return 1;
}

/* ── Public: rb_handle_pay_cost_all_discard ── */
int rb_handle_pay_cost_all_discard(GameState *g, int actor, const char *selected) {
    if (!g) return 0;
    int accepted = selected && strcmp(selected, "skip_optional_cost") && strcmp(selected, "0");
    s_cost_optional_skipped = accepted ? 0 : 1;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
        g->queue.entries[g->queue.cur].cost_paid = 1;
        g->queue.entries[g->queue.cur].optional_cost_result = accepted ? 1 : 0;
    }
    g->queue.has_pending = 0;

    if (accepted) {
        /* Pass the entry's cost so movement events carry the correct source zone */
        const AbilityEffect *entry_cost = rb_entry_cost(g);
        rb_effect_pay_cost_all_discard(g, actor, entry_cost);
        rb_recalc_constants(g);
    }

    /* After cost settled, run the entry's effect if the cost was paid
       and the effect has not yet started */
    int effect_started = 0;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
        effect_started = g->queue.entries[g->queue.cur].effect_started;

    const AbilityEffect *entry_cost = rb_entry_cost(g);
    if (accepted && entry_cost && !effect_started) {
        const AbilityEffect *entry_eff = rb_entry_effect(g);
        if (entry_eff) {
            if (entry_eff->action && !strcmp(entry_eff->action, "place_energy_under_member")) {
                execute_place_energy_under_member_non_optional(g, actor, entry_eff);
            } else if (entry_eff->action) {
                rb_execute_effect_ex(g, actor, (AbilityEffect *)entry_eff, g->activating_card);
            }
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
                g->queue.entries[g->queue.cur].effect_started = 1;
        }
    } else if (accepted && rb_queue_has_pending_actions(g)) {
        /* handlers.rs:1731-1738 — same single-consumer situation as the pay arm of
           rb_handle_optional_cost_payment. choice.c:4821 calls
           rb_resolver_continue_siblings on the very next line of the answer path,
           so the batch stays parked. See cost_arm_resume. */
        cost_arm_resume(g);
    }
    return 1;
}

/* ── Public: rb_cost_has_skip_prompt ── */
int rb_cost_has_skip_prompt(const AbilityEffect *cost) {
    return has_skip_prompt(cost);
}

/* ── Public: rb_get_change_state_candidates ── */
int rb_get_change_state_candidates(const GameState *g, int actor, int *out_positions, int max) {
    int tmp[RB_STAGE_SIZE];
    int n = get_change_state_candidates(g, actor, "self", NULL, NULL, 0, 0, 0, "active", tmp, max);
    for (int i = 0; i < n && i < max; i++) out_positions[i] = tmp[i];
    return n;
}

/* ── Public: rb_pay_cost_move_cards ── */
int rb_pay_cost_move_cards(GameState *g, int actor, const AbilityEffect *cost,
                           int host_cid, int is_activation) {
    if (!g || !cost) return 0;
    (void)host_cid;
    (void)is_activation;
    return pay_cost_move_cards(g, actor, cost);
}

/* ── Public: rb_pay_cost_change_state ── */
int rb_pay_cost_change_state(GameState *g, int actor, const AbilityEffect *cost,
                             int host_cid, int is_activation) {
    if (!g || !cost) return 0;
    (void)host_cid;
    (void)is_activation;
    return pay_cost_change_state(g, actor, cost);
}

/* ════════════════════════════════════════════════════════════════════
   Play-cost reduction (mirrors engine/src/ability/util.rs
   compute_play_cost / calculate_play_cost_reduction / scan_abilities_for_
   cost_reduction / per_unit_cost_reduction / play_cost_reduction_matches).
   ════════════════════════════════════════════════════════════════════ */

static const char *cr_eff_extra(const AbilityEffect *e, const char *k) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}
static int cr_eff_int(const AbilityEffect *e, const char *k, int dflt) {
    const char *v = cr_eff_extra(e, k);
    if (!v || !*v) return dflt;
    return atoi(v);
}

static const AbilityEffect *cr_find_modify_cost(const AbilityEffect *e,
                                                const char *op, const char *loc) {
    if (!e) return NULL;
    if (e->action && !strcmp(e->action, "modify_cost")) {
        const char *eop = cr_eff_extra(e, "operation");
        const char *eloc = cr_eff_extra(e, "location");
        if ((!op || (eop && !strcmp(eop, op))) &&
            (!loc || (eloc && !strcmp(eloc, loc))))
            return e;
    }
    if (e->action && (!strcmp(e->action, "sequential") ||
                     !strcmp(e->action, "sequential_cost"))) {
        for (int i = 0; i < e->n_child; i++) {
            const AbilityEffect *f = cr_find_modify_cost(e->child[i], op, loc);
            if (f) return f;
        }
    }
    return NULL;
}

static int cr_reduction_matches(const AbilityEffect *e, int card_id, const Card *card) {
    /* util.rs:193-200 play_cost_reduction_matches — `gn.first()` gates the
       single group name. Whole-CSV compare admitted no card at all. */
    const char *gn = cr_eff_extra(e, "group_names");
    if (gn && !card_matches_group_name_first(card_id, gn)) return 0;
    const char *limit = cr_eff_extra(e, "cost_limit");
    if (limit && card->cost != atoi(limit)) return 0;
    if (!rb_cost_threshold_met(card, e)) return 0;
    const char *ct = cost_card_type_any(e);
    if (ct && strcmp(ct, "member_card") != 0) return 0;
    const char *af = cr_eff_extra(e, "ability_filter");
    if (af && !strcmp(af, "no_ability") && rb_card_num_abilities((uint32_t)card_id) > 0)
        return 0;
    return 1;
}

/* cr_per_unit — mirror of per_unit_any(). The wire encodes `per_unit` as a
   BOOL (engine/src/ability/effect_decoder_gen.rs:89), and the C decoder only
   copies I64-tagged values into the struct field, so the decoded truth lives in
   the extras table. Both are consulted. */
static int cr_per_unit(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->per_unit) return 1;
    const char *v = cr_eff_extra(e, "per_unit");
    return v && !strcmp(v, "true");
}

static int cr_per_unit_reduction(const AbilityEffect *e, const GameState *g,
                                 int actor, int hand_count) {
    /* Rust util.rs:156-158: per_unit_location_any().or(location_any())
       — NOT per_unit_type, which names the grouping unit (枚 / group_name /
       unit) and is never a zone. */
    const char *pul  = cr_eff_extra(e, "per_unit_location");
    const char *loc  = cr_eff_extra(e, "location");
    const char *zone = pul ? pul : (loc ? loc : "hand");
    int raw_count;
    char gbuf[128];
    /* util.rs:236-243 per_unit_cost_reduction — the BRANCH test is a presence
       test on the whole list (`group_names_any().is_some()`), but the stage
       scan that follows is filtered by `effect.group_name()` = `.first()`. The
       C passed the joined CSV to the single-token matcher. */
    if (!strcmp(zone, "stage") && cr_eff_extra(e, "group_names")) {
        const char *gn = cr_eff_extra(e, "group_names");
        const RbPlayer *P = &g->p[actor];
        int n = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) {
            int id = P->stage[s];
            if (id != RB_EMPTY_SLOT && rb_card_matches_group_str(id,
                    group_name_first(gn, gbuf, sizeof gbuf))) n++;
        }
        raw_count = n;
    } else {
        raw_count = hand_count;
    }
    int per_unit_count = e->per_unit_count > 0 ? e->per_unit_count : 1;
    const char *ex = cr_eff_extra(e, "exclude_self");
    int exclude_self = ex && !strcmp(ex, "true");
    int effective = exclude_self ? (raw_count > 0 ? raw_count - 1 : 0) : raw_count;
    int value = cr_eff_int(e, "value", 1);
    return (effective / per_unit_count) * value;
}

/* cond_get_state — mirror of Condition::get_state(): the literal string of the
   `state` field, or NULL when the condition carries none. */
static const char *cond_get_state(const Condition *c) {
    if (!c) return NULL;
    for (uint32_t i = 0; i < c->n_fields; i++)
        if (c->fields[i].key && !strcmp(c->fields[i].key, "state"))
            return c->fields[i].v.tag == RB_TAG_STR ? c->fields[i].v.s : NULL;
    return NULL;
}

static int cr_scan_one_effect(const AbilityEffect *eff, int target_id,
                              const Card *target_card, const GameState *g,
                              int actor, int hand_count, int hand_guard) {
    if (!eff || !eff->action || strcmp(eff->action, "modify_cost") != 0) return -1;
    const char *op  = cr_eff_extra(eff, "operation");
    const char *loc = cr_eff_extra(eff, "location");
    if (!(op && !strcmp(op, "subtract"))) return -1;
    if (!(loc && !strcmp(loc, "hand"))) return -1;
    if (hand_guard && eff->condition) {
        for (uint32_t i = 0; i < eff->condition->n_fields; i++) {
            if (eff->condition->fields[i].key &&
                !strcmp(eff->condition->fields[i].key, "location") &&
                eff->condition->fields[i].v.tag == RB_TAG_STR &&
                eff->condition->fields[i].v.s &&
                !strcmp(eff->condition->fields[i].v.s, "hand"))
                return -1;
        }
    }
    /* util.rs:359-375 — a wait-gated aura only applies while at least one of
       the player's own stage members matching group_names is in the wait
       state. Dropping this branch made wait-gated cost auras permanent. */
    {
        const char *st = cond_get_state(eff->condition);
        if (st && !strcmp(st, "wait")) {
            const char *groups = cr_eff_extra(eff, "group_names");
            const RbPlayer *St = &g->p[actor];
            int met = 0;
            for (int s = 0; s < RB_STAGE_SIZE; s++) {
                int id = St->stage[s];
                if (id == RB_EMPTY_SLOT) continue;
                const char *ori = rb_mods_get_orientation((RbMods *)&g->mods, id);
                if (!ori || strcmp(ori, "wait") != 0) continue;
                if (groups && *groups) {
                    char buf[16][64];
                    int n = split_csv(groups, buf, 16);
                    int any = 0;
                    for (int k = 0; k < n; k++)
                        if (rb_card_matches_group_str(id, buf[k])) { any = 1; break; }
                    if (!any) continue;
                }
                met = 1;
                break;
            }
            if (!met) return -1;
        }
    }
    /* util.rs:477-480 scan_abilities_for_cost_reduction — this twin spells out
       `card_matches_any_group(card_db, target_id, gns)` (util.rs:799), NOT
       `group_name()`. A split-and-accept-any compare, so 「『A』と『B』の」 auras
       reach a card in either group. The joined CSV matched neither. */
    const char *gn = cr_eff_extra(eff, "group_names");
    if (gn && !card_matches_any_group(target_id, gn)) return -1;
    const char *limit = cr_eff_extra(eff, "cost_limit");
    if (limit && target_card->cost != atoi(limit)) return -1;
    if (!rb_cost_threshold_met(target_card, eff)) return -1;
    const char *ct = cost_card_type_any(eff);
    if (ct && strcmp(ct, "member_card") != 0) return -1;
    const char *af = cr_eff_extra(eff, "ability_filter");
    if (af && !strcmp(af, "no_ability") &&
        rb_card_num_abilities((uint32_t)target_id) > 0)
        return -1;
    if (cr_per_unit(eff)) return cr_per_unit_reduction(eff, g, actor, hand_count);
    return cr_eff_int(eff, "value", 1);
}

static int cr_scan_one_effect_source(int src_card_id, int target_id,
                                     const Card *target_card, const GameState *g,
                                     int actor, int hand_count, int hand_guard);

static int cr_calc_reduction(const GameState *g, int actor, int card_id,
                             const Card *card) {
    int cost_reduction = 0;
    int hand_count = g->p[actor].hand.n + 1;
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int ai = 0; ai < n; ai++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, ai, &ab)) continue;
        if (ab.effect) {
            const AbilityEffect *mc = cr_find_modify_cost(ab.effect, "subtract", "hand");
            if (mc && cr_reduction_matches(mc, card_id, card)) {
                if (cr_per_unit(mc))
                    cost_reduction = cr_per_unit_reduction(mc, g, actor, hand_count);
                else {
                    int v = cr_eff_int(mc, "value", 1);
                    if (v > cost_reduction) cost_reduction = v;
                }
            }
        }
        rb_free_ability(&ab);
    }
    {
        const RbPlayer *P = &g->p[actor];
        for (int s = 0; s < RB_STAGE_SIZE; s++) {
            int id = P->stage[s];
            if (id == RB_EMPTY_SLOT) continue;
            int r = cr_scan_one_effect_source(id, card_id, card, g, actor,
                                             hand_count, 1);
            if (r >= 0) cost_reduction += r;
        }
    }
    if (cost_reduction == 0) {
        const RbPlayer *P = &g->p[actor];
        for (int i = 0; i < P->success.n; i++) {
            int id = P->success.cards[i];
            int r = cr_scan_one_effect_source(id, card_id, card, g, actor,
                                             hand_count, 0);
            if (r >= 0) { if (r > cost_reduction) cost_reduction = r; break; }
        }
    }
    return cost_reduction;
}

static int cr_scan_one_effect_source(int src_card_id, int target_id,
                                     const Card *target_card, const GameState *g,
                                     int actor, int hand_count, int hand_guard) {
    int n = rb_card_num_abilities((uint32_t)src_card_id);
    for (int ai = 0; ai < n; ai++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)src_card_id, ai, &ab)) continue;
        int r = -1;
        if (ab.effect)
            r = cr_scan_one_effect(ab.effect, target_id, target_card, g,
                                   actor, hand_count, hand_guard);
        rb_free_ability(&ab);
        if (r >= 0) return r;
    }
    return -1;
}

int rb_compute_play_cost(const GameState *g, int actor, int card_id,
                         int set_override) {
    Card card;
    if (!rb_decode_card_by_index((uint32_t)card_id, &card)) return 0;
    int base_cost = card.cost;
    int hand_count = g->p[actor].hand.n + 1;
    int reduction = cr_calc_reduction(g, actor, card_id, &card);
    int increase = 0;
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int ai = 0; ai < n; ai++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, ai, &ab)) continue;
        if (ab.effect && ab.effect->action &&
            !strcmp(ab.effect->action, "modify_cost")) {
            const char *op  = cr_eff_extra(ab.effect, "operation");
            const char *loc = cr_eff_extra(ab.effect, "location");
            if (op && (!strcmp(op, "increase") || !strcmp(op, "add")) &&
                loc && !strcmp(loc, "success_live_zone")) {
                int per_unit_count = ab.effect->per_unit_count > 0
                                     ? ab.effect->per_unit_count : 1;
                int success_count = g->p[actor].success.n;
                int multiplier = ab.effect->count > 0 ? ab.effect->count : 1;
                increase = (success_count / per_unit_count) * multiplier;
            }
        }
        rb_free_ability(&ab);
    }
    int cost = base_cost - reduction + increase;
    if (cost < 0) cost = 0;
    if (cost > 255) cost = 255;
    if (set_override != 0 && set_override != base_cost)
        cost = (int)rb_saturate_u8(set_override);
    rb_free_card(&card);
    return cost;
}

/* ── Bag helpers (kept from existing cost.c) ── */
static void bag_push(RbBag *b, int c) { if (b->n < RB_MAX_ZONE) b->cards[b->n++] = c; }
static int  bag_pop(RbBag *b) { return b->n > 0 ? b->cards[--b->n] : -1; }

static RbBag *cost_source_bag(RbPlayer *P, const char *src) {
    if (!src) return NULL;
    if (!strcmp(src, "hand")) return &P->hand;
    if (!strcmp(src, "deck") || !strcmp(src, "deck_top")) return &P->deck;
    if (!strcmp(src, "waitroom") || !strcmp(src, "discard")) return &P->discard;
    /* ability/enums.rs:98 — "energy_zone" is the other spelling of Zone::Energy. */
    if (!strcmp(src, "energy") || !strcmp(src, "energy_zone")) return &P->energy;
    return NULL;
}
static int cost_count_in_source(const GameState *g, int actor, const char *src) {
    const RbPlayer *P = &g->p[actor];
    if (!src) return 0;
    if (!strcmp(src, "stage")) {
        int n = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) n++;
        return n;
    }
    RbBag *b = cost_source_bag((RbPlayer *)P, src);
    return b ? b->n : 0;
}
static int cost_move_from_source(GameState *g, int actor, const char *src, int count) {
    RbPlayer *P = &g->p[actor];
    int moved = 0;
    if (!src) return 0;
    if (!strcmp(src, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE && moved < count; i++) {
            if (P->stage[i] != RB_EMPTY_SLOT) {
                int cid = P->stage[i];
                P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0;
                bag_push(&P->discard, cid); moved++;
            }
        }
        g->mods.last_cost_discard_count = moved;
        return moved;
    }
    RbBag *b = cost_source_bag(P, src);
    if (!b) return 0;
    while (moved < count && b->n > 0) {
        int cid = bag_pop(b);
        bag_push(&P->discard, cid);
        moved++;
    }
    g->mods.last_cost_discard_count = moved;
    return moved;
}
