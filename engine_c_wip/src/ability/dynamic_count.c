/* dynamic_count.c  Esingle source of truth for resolving a DynamicCount
   reference into a count.
   Mirror engine/src/ability/dynamic_count.rs:GameState::resolve_dynamic_count.

   Both the constant-path (recalculate_constants) and the ability-execution
   path (AbilityResolver) call this one method, so dynamic_count semantics
   live in exactly one place instead of being duplicated per caller.

   The transient resolver context (which cards moved / were selected / how
   many were drawn in the current step) is passed in because the constant
   path has no AbilityResolver. Callers that don't have that context pass
   empty slices / 0. */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* Forward declaration */
static int rb_revealed_count(const struct GameState *g, int owner);

/* ── owner resolution helpers ──────────────────────────────────────────── */

/* Determine which player is "self" for a DynamicCount arm that uses
   resolve_target_player("self"). The C port collapses the concept to
   the player whose turn it is (g->active), equivalent to Rust's
   resolve_target_player("self") for normal play. */
static int rb_dc_self_player(const struct GameState *g)
{
    return (g && g->active >= 0) ? g->active : 0;
}

/* Determine which player is "opponent". */
static int rb_dc_opponent_player(const struct GameState *g, int self_pl)
{
    (void)g;
    return 1 - self_pl;
}

/* Determine the effective owner player from owner_card. Mirrors Rust
   (engine/src/ability/dynamic_count.rs:76-79):
       let own_is_p1 = match owner_card {
           Some(cid) => self.player1.stage.stage.contains(&cid),
           None => true,
       };
   i.e. the owner is whichever player's stage holds the activating card; with
   no owner card the owner is the acting player (player1 in the default C
   model), which is what owner_on_p1 == 0 expresses. */
static int rb_dc_owner_from_card(const struct GameState *g, int owner_card, int owner_on_p1)
{
    if (owner_card >= 0) {
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            if (g->p[0].stage[i] == owner_card) return 0;
            if (g->p[1].stage[i] == owner_card) return 1;
        }
    }
    return owner_on_p1 ? 1 : 0;
}

/* ── stage member count helper ─────────────────────────────────────────── */

static int rb_stage_member_count(const RbPlayer *P)
{
    int c = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (P->stage[i] != RB_EMPTY_SLOT) c++;
    return c;
}

/* ── main resolver ─────────────────────────────────────────────────────── */

int rb_resolve_dynamic_count(const struct GameState *g, int owner, int host_cid,
                             const char *reference,
                             const char *base_reference,
                             const char *count_type,
                             const char *calculation,
                             int calculation_value,
                             int owner_on_p1,
                             const int *moved, int n_moved,
                             const int *selected, int n_selected,
                             int last_draw_count)
{
    (void)moved;
    const char *reference_text = reference ? reference : base_reference;

    int count = 0;

    if (!reference_text) {
        /* fall through to count_type default */
    } else if (!strcmp(reference_text, "selected_card_score")) {
        if (n_selected > 0) {
            int cid = selected[0];
            Card c;
            if (rb_decode_card_by_index((uint32_t)cid, &c)) {
                count = c.score;
                rb_free_card(&c);
            }
        }
    } else if (!strcmp(reference_text, "previous_moved_cards") ||
               !strcmp(reference_text, "previous_move")) {
        if (n_moved > 0)
            count = n_moved;
        else if (g->n_recently_moved > 0)
            count = g->n_recently_moved;
        else
            count = g->mods.last_cost_discard_count;
    } else if (!strcmp(reference_text, "previous_draw")) {
        if (last_draw_count > 0)
            count = last_draw_count;
        else if (g->n_recently_moved > 0)
            count = g->n_recently_moved;
        else
            count = 0;
    } else if (!strcmp(reference_text, "revealed_cards") ||
               !strcmp(reference_text, "previous_reveal")) {
        count = rb_revealed_count(g, owner);
    } else if (!strcmp(reference_text, "unit_count")) {
        int self_pl = rb_dc_self_player(g);
        const RbPlayer *P = &g->p[self_pl];
        count = rb_stage_member_count(P);
    } else if (!strcmp(reference_text, "energy_difference")) {
        int threshold = 0;
        if (base_reference) {
            char *end = NULL;
            long v = strtol(base_reference, &end, 10);
            if (end != base_reference && *end == '\0' && v >= 0 && v <= 255)
                threshold = (int)v;
        }
        int self_pl = rb_dc_self_player(g);
        const RbPlayer *P = &g->p[self_pl];
        int n = P->energy.n - threshold;
        count = n < 0 ? 0 : n;
    } else if (!strcmp(reference_text, "success_pile_count_difference")) {
        int own_pl   = rb_dc_owner_from_card(g, host_cid, owner_on_p1);
        int other_pl = 1 - own_pl;
        const RbPlayer *own   = &g->p[own_pl];
        const RbPlayer *other = &g->p[other_pl];
        int diff = other->success.n - own->success.n;
        count = diff < 0 ? 0 : diff;
    } else if (!strcmp(reference_text, "these_waitroom_placed_count")) {
        if (g->n_recently_moved > 0)
            count = g->n_recently_moved;
        else
            count = n_moved;
    } else if (!strcmp(reference_text, "total_live_score")) {
        int self_pl = rb_dc_self_player(g);
        const RbPlayer *P = &g->p[self_pl];
        for (int i = 0; i < P->live.n; i++) {
            Card c;
            if (rb_decode_card_by_index((uint32_t)P->live.cards[i], &c)) {
                count += c.score;
                rb_free_card(&c);
            }
        }
    } else if (!strcmp(reference_text, "stage_member_count")) {
        int self_pl = rb_dc_self_player(g);
        const RbPlayer *P = &g->p[self_pl];
        count = rb_stage_member_count(P);
    } else if (!strcmp(reference_text, "opponent_stage_member_count")) {
        int self_pl  = rb_dc_self_player(g);
        int opp_pl   = rb_dc_opponent_player(g, self_pl);
        const RbPlayer *P = &g->p[opp_pl];
        count = rb_stage_member_count(P);
    } else if (!strcmp(reference_text, "opponent_waited_member_count")) {
        /* dynamic_count.rs:114-124 keys this arm on
             self.mods.get_orientation_modifier(c) == Some("wait")
           — the ORIENTATION modifier. The C carries that one fact in two
           places: the orientation modifier (set by every ウェイト / 常時
           orientation effect through rb_mods_set_orientation) and the per-slot
           `stage_wait` flag (the ONLY writer for an effect-driven
           state_change, choice.c:3233-3237 — and the field live.c:128,
           zones.c:60, condition.c:1710-1714 and phase.c:51-56 already read).
           Reading only `stage_wait` reported a member made waited by an
           orientation-modifier effect as ACTIVE, so 「相手のステージにいる
           ウェイト状態のメンバーの数まで」 under-counted and the
           abilities.json:3426 虹ヶ咲 select asked for too few cards. Accept
           either — precisely condition.c's orientation_matches_state
           resolution of the same pair. */
        int self_pl  = rb_dc_self_player(g);
        int opp_pl   = rb_dc_opponent_player(g, self_pl);
        const RbPlayer *P = &g->p[opp_pl];
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            if (P->stage[i] == RB_EMPTY_SLOT) continue;
            const char *om = rb_mods_get_orientation((RbMods *)&g->mods, P->stage[i]);
            if (P->stage_wait[i] || (om && !strcmp(om, "wait"))) count++;
        }
    } else if (!strcmp(reference_text, "waitroom_count_below_base")) {
        int threshold = 0;
        if (base_reference) {
            char *end = NULL;
            long v = strtol(base_reference, &end, 10);
            if (end != base_reference && *end == '\0' && v >= 0 && v <= 255)
                threshold = (int)v;
        }
        int self_pl = rb_dc_self_player(g);
        const RbPlayer *P = &g->p[self_pl];
        int diff = threshold - P->discard.n;
        count = diff < 0 ? 0 : diff;
    } else if (!strcmp(reference_text, "energy_cards_under_this_member")) {
        /* Rust (dynamic_count.rs:136-144): the activating card's stage
           *index* is used, and when the card is not on the stage the index
           falls back to 1 (center)  Eit does NOT widen to every member. */
        int self_pl = rb_dc_self_player(g);
        const RbPlayer *P = &g->p[self_pl];
        int area = -1;
        if (host_cid >= 0) {
            for (int a = 0; a < RB_STAGE_SIZE; a++)
                if (P->stage[a] == host_cid) { area = a; break; }
        }
        if (area < 0) area = 1;
        if (area >= 0 && area < RB_STAGE_SIZE)
            count = P->under_cards[area].n;
    } else {
        /* Rust (dynamic_count.rs:145-148): an unknown reference falls through
           to count_type, where "revealed_cards" is the only non-zero arm. */
        if (count_type && !strcmp(count_type, "revealed_cards"))
            count = rb_revealed_count(g, owner);
        else
            count = 0;
    }

    if (calculation && !strcmp(calculation, "add")) {
        count += calculation_value;
    }
    return count;
}

/* ── effect-level count resolution ────────────────────────────────────── */

/* Look up an extra_kv pair by key in an AbilityEffect. */
static const char *dc_extra(const AbilityEffect *e, const char *key)
{
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key))
            return e->extra_v[i];
    return NULL;
}

/* Parse an extra_kv value as a base-10 integer; returns 0 on missing/invalid. */
static int dc_extra_int(const AbilityEffect *e, const char *key)
{
    const char *v = dc_extra(e, key);
    if (!v) return 0;
    char *end = NULL;
    long val = strtol(v, &end, 10);
    if (end == v || *end != '\0') return 0;
    return (int)val;
}

/* util.c's twin of util.rs:2533-2544 `resolve_discard_per_unit_count`. util.c
   defines it non-static but include/rabuka.h does not declare it, so the
   prototype is repeated here exactly as util.c:59 already does for
   rb_card_has_draw_icon. Keeping the CALL rather than a copy keeps
   `discard` per-unit counting single-sourced. */
int rb_resolve_discard_per_unit_count(const GameState *g, int last_discard_count,
                                      const char *card_type, const char *group);

/* `group_names.first()` — core/card.rs:2462 `AbilityEffect::group_name()`.
   The bytecode decoder flattens the Vec into ONE comma-joined extra, and
   rb_count_matching / RbCardFilter compare the whole string, so the list has
   to be cut here or a clause like ['lilywhite', "μ's"] matches nothing. */
static const char *dc_group_first(const char *csv, char *buf, size_t bufsz)
{
    if (!csv || !*csv) return NULL;
    const char *comma = strchr(csv, ',');
    size_t len = comma ? (size_t)(comma - csv) : strlen(csv);
    if (!buf || bufsz == 0) return csv;
    if (len >= bufsz) len = bufsz - 1;
    memcpy(buf, csv, len);
    buf[len] = '\0';
    return buf;
}

/* util.rs:2689-2708 `per_unit_zone` — which zone a per-unit clause counts
   over. NULL means the clause names no zone this engine can count, and the
   caller treats the count as 1 (util.rs:2634-2636 `_ => return 1`). The
   extra spellings ("waitroom", "live", "live_zone", "success") are accepted
   here as aliases; Rust sends them down the same `_ => return None` arm. No
   corpus effect carries one as its per_unit_type (measured over
   cards/abilities.json: 人 26 / 枚 32 / discard 9 / つ 2 / card 1 /
   heart_colors 1 / group_name 1 / energy_deck 1 / absent 2). */
static const char *dc_per_unit_zone(const char *unit, const char *card_type)
{
    if (!unit) return NULL;
    if (!strcmp(unit, "stage") || !strcmp(unit, "member") ||
        !strcmp(unit, "\xE4\xBA\xBA") || !strcmp(unit, "members"))
        return "stage";                                   /* 人 */
    if (!strcmp(unit, "hand") || !strcmp(unit, "card")) return "hand";
    if (!strcmp(unit, "under_member")) return "under_member";
    /* util.rs:2696-2702 — the bare 「1枚につき」 counts the HAND, and only
       becomes the under-member zone when the effect's own filter already pins
       card_type to member_card. */
    if (!strcmp(unit, "\xE6\x9D\xAF"))                    /* 枚 */
        return (card_type && !strcmp(card_type, "member_card")) ? "under_member"
                                                                : "hand";
    if (!strcmp(unit, "discard") || !strcmp(unit, "waitroom")) return "discard";
    if (!strcmp(unit, "live_card_zone") || !strcmp(unit, "live") ||
        !strcmp(unit, "live_zone")) return "live";
    if (!strcmp(unit, "success_live_zone") || !strcmp(unit, "success_live_card_zone") ||
        !strcmp(unit, "success")) return "success";
    return NULL;
}

/* util.rs:2646-2653 — the `heart_colors` per-unit arm asks "does `cid` carry a
   heart of colour `col` in EITHER map?", and there it really is an OR:
       card.base_heart…contains_key(&hc) || card.need_heart…contains_key(&hc)
   The C Card lays the three heart maps out as [0, num_base) = base,
   [num_base, num_base+num_blade) = blade, [num_base+num_blade, n_hearts) = need,
   so the blade span is skipped. The `count > 0` test is load-bearing: the C
   decode keeps every (colour,count) byte pair (core/card.c:110-117) while
   Rust's HeartMap only ever holds colours whose count was non-zero
   (card_binary.rs:335-337 parse_hearts). */
static int dc_card_has_base_or_need(int cid, int col)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    int blade_end = c.num_base + c.num_blade;
    int r = 0;
    for (int i = 0; i < c.n_hearts; i++) {
        if ((int)c.heart_color[i] != col || c.heart_count[i] == 0) continue;
        if (i < c.num_base || i >= blade_end) { r = 1; break; }
    }
    rb_free_card(&c);
    return r;
}

/* util.rs:1128-1138 `card_matches_heart_colors` — and this one is NOT an OR.
   `card.base_heart.as_ref().map_or(<need_heart check>, <base_heart check>)`
   means: a card that PRINTS base hearts is judged on its printed hearts alone
   and its required hearts are never consulted; only a card with no base_heart
   map at all (a live card) falls back to need_heart. The C spelling of
   "base_heart is Some" is `num_base > 0` (card_binary.rs:231-235 sets
   `base_heart: None` exactly when the printed-heart map came out empty), and
   util.c's rb_card_matches_heart_colors (util.c:698-701) already spells the same
   base-first choice for every OTHER caller of this predicate.

   This file previously used dc_card_has_base_or_need here, so every per-unit
   zone count filtered by `heart_colors` also admitted cards that print no heart
   of that colour but REQUIRE one — 54 corpus effects carry both `per_unit` and
   `heart_colors` with a zone per_unit_type (the 55th, per_unit_type
   "heart_colors", is the OR site above and keeps that helper). */
static int dc_card_has_base_first(int cid, int col)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    int start, end;
    if (c.num_base > 0) { start = 0; end = c.num_base; }
    else { start = c.num_base + c.num_blade; end = c.n_hearts; }
    int r = 0;
    for (int i = start; i < end && i < c.n_hearts; i++)
        if ((int)c.heart_color[i] == col && c.heart_count[i] > 0) { r = 1; break; }
    rb_free_card(&c);
    return r;
}

/* "heart01,heart04,…" (the decoder's flattened heart_colors multiset) -> an
   8-bit mask of the named colours. A token is a run of ASCII letters/digits and
   anything else — ',', the ideographic '、', '[', ']', '"', space — separates,
   so no separator spelling has to be enumerated. */
static unsigned dc_color_mask(const char *list)
{
    unsigned mask = 0;
    if (!list) return 0;
    for (const char *p = list; *p; ) {
        const char *start = p;
        while ((*p >= 'a' && *p <= 'z') || (*p >= 'A' && *p <= 'Z') ||
               (*p >= '0' && *p <= '9'))
            p++;
        size_t len = (size_t)(p - start);
        if (p == start) { p++; continue; }
        if (len > 5 && !strncmp(start, "heart", 5)) {
            int v = 0, nd = 0;
            for (size_t k = 5; k < len; k++) {
                if (start[k] < '0' || start[k] > '9') { nd = 0; break; }
                v = v * 10 + (start[k] - '0');
                nd++;
            }
            if (nd > 0 && v >= 0 && v < 8) mask |= 1u << v;
        }
    }
    return mask;
}

/* util.rs:2637-2661 — per_unit_type "heart_colors" counts the DISTINCT heart
   colours carried by the STAGE members the filter admits, not the members. */
static int dc_distinct_heart_colors(const GameState *g, int pl,
                                    const RbCardFilter *f, const char *heart_colors)
{
    unsigned want = dc_color_mask(heart_colors);
    if (want == 0) return 0;
    const RbPlayer *P = &g->p[pl];
    unsigned found = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = P->stage[i];
        if (cid == RB_EMPTY_SLOT) continue;
        if (rb_count_matching_filter(f, &cid, 1) != 1) continue;
        for (int c = 0; c < 8; c++)
            if ((want & (1u << c)) && dc_card_has_base_or_need(cid, c))
                found |= 1u << c;
    }
    int n = 0;
    for (int c = 0; c < 8; c++) n += (found >> c) & 1u;
    return n;
}

/* util.rs:2816-2825 `apply_distinct_filter` — duplicate normalised names
   collapse to one unit. `f->distinct` is the decoder's is_distinct flag, which
   is exactly `util.rs:2809-2814 distinct_should_dedupe`'s input set. */
static int dc_apply_distinct(int *ids, int n)
{
    char seen[RB_MAX_ZONE][96];
    int nseen = 0, kept = 0;
    for (int i = 0; i < n; i++) {
        char key[96];
        Card c;
        if (rb_decode_card_by_index((uint32_t)ids[i], &c)) {
            rb_card_normalize_name(c.name ? c.name : "", key, sizeof key);
            rb_free_card(&c);
        } else {
            key[0] = 0;
        }
        int dup = 0;
        for (int s = 0; s < nseen; s++)
            if (!strcmp(seen[s], key)) { dup = 1; break; }
        if (dup) continue;
        if (nseen < RB_MAX_ZONE) {
            snprintf(seen[nseen], sizeof seen[0], "%s", key);
            nseen++;
        }
        ids[kept++] = ids[i];
    }
    return kept;
}

/* util.rs:2784-2804 `count_filtered` — the filter alone decides when the effect
   named no heart colours; when it did, a card must also carry one of them.
   Both branches keep the MATCHING ids in `ids` because the distinct pass runs
   over exactly that set (util.rs:2802). The heart-colour predicate is
   dc_card_has_base_first (util.rs:1131-1136), NOT the OR form that the
   `heart_colors` per-unit arm uses. */
static int dc_count_filtered(int *ids, int n, const RbCardFilter *f,
                             const char *heart_colors)
{
    unsigned want = dc_color_mask(heart_colors);
    int kept = 0;
    for (int i = 0; i < n; i++) {
        if (rb_count_matching_filter(f, &ids[i], 1) != 1) continue;
        if (want != 0) {
            int hit = 0;
            for (int c = 0; c < 8 && !hit; c++)
                if ((want & (1u << c)) && dc_card_has_base_first(ids[i], c)) hit = 1;
            if (!hit) continue;
        }
        ids[kept++] = ids[i];
    }
    n = kept;
    if (f->distinct) n = dc_apply_distinct(ids, n);
    return n;
}

/* ── per-unit unit count (util.rs::resolve_per_unit_count) ───────────────────
 *
 * The Rust twin is `resolve_per_unit_count` / `calculate_per_unit_multiplier` /
 * `per_unit_zone` in engine/src/ability/util.rs:2577-2708. Six Rust callers raise
 * the same `per_unit` flag with DIFFERENT arguments, so this shared arm is their
 * union rather than any one of them:
 *
 *   look.rs:979      resolve_per_unit_count(true, per_unit_type, ...) x base
 *   deck.rs:257      calculate_per_unit_multiplier(per_unit, per_unit_type, …),
 *                    preceded for per_unit_type == "discard" by the
 *                    moved-batch resolve_discard_per_unit_count
 *   score.rs:221-247 / :393-423  location-or-per_unit_type, and the 「つ」 clause
 *                    is `last_cost_energy_count` (see the つ arm below)
 *   resource.rs:474 / :510-534   same precedence; per_unit_aggregate "score"
 *                    takes resolve_per_unit_score_sum, and it passes an EMPTY
 *                    heart_colors list where score.rs passes the effect's own —
 *                    the one place this union knowingly over-filters
 *   energy.rs:313/551  no per_unit handling at all
 *   reyell.rs:70     per_unit sums the COSTS of the moved cards, then caps
 *
 * Shapes measured over cards/abilities.json (77 per_unit effects):
 *
 *   per_unit_type   count   location            arm taken below
 *   --------------  ------  ------------------  ----------------------------
 *   "人"               26   stage / hand / -    STAGE (state filter applied)
 *   "枚"               32   5 kinds / -         location when present, else the
 *                                             枚 -> Hand / UnderMember rule
 *   "discard"           9   -                   moved batch, NOT the waitroom
 *   "つ"                2   -                   last_cost_energy_count
 *   "card"              1   success_live_zone   location wins -> SUCCESS
 *   "heart_colors"      1   stage               distinct colours; wins over
 *                                             location (score.rs:221-224)
 *   "group_name"        1   -                   unnameable zone -> 1
 *   "energy_deck"       1   -                   unnameable zone -> 1
 *   (none)              2   -                   unnameable zone -> 1
 *
 * `location` wins over `per_unit_type` when both are present (the shape every
 * 枚 clause with an explicit zone uses), which is score.rs:219-223's precedence;
 * look.rs:979 and deck.rs:257 instead read per_unit_type alone. Keeping the
 * location-first order is what the 32 `枚` clauses and the 26 `人` clauses were
 * already measured against, and `location` is absent on exactly the shapes the
 * new 枚 / card / discard / つ / heart_colors arms exist to serve. */
static int dc_per_unit_units(const struct GameState *g, int pl, int host_cid,
                             const AbilityEffect *e, const char *card_type)
{
    const char *put = dc_extra(e, "per_unit_type");
    const char *loc = dc_extra(e, "location");
    const char *heart_colors = dc_extra(e, "heart_colors");
    const char *state_filter = dc_extra(e, "state");

    /* look.rs:978 `util::CardFilter::from_effect(effect)` — the FULL filter,
       not the minimal subset. card.c's builder keeps group_names whole, so the
       `.first()` split has to be applied on top (card.rs:2462). */
    RbCardFilter f;
    rb_effect_filter_subset(e, &f);
    if (f.has_group && f.group[0]) {
        char gbuf[128];
        const char *g1 = dc_group_first(f.group, gbuf, sizeof gbuf);
        strncpy(f.group, g1 ? g1 : "", sizeof f.group - 1);
        f.group[sizeof f.group - 1] = 0;
        if (!f.group[0]) f.has_group = 0;
    }

    /* score.rs:219-224 — "heart_colors" is a PSEUDO-zone: it is matched BEFORE
       the `location` lookup because it triggers its own counting. */
    if (put && !strcmp(put, "heart_colors"))
        return dc_distinct_heart_colors(g, pl, &f, heart_colors);

    /* score.rs:225-227 / :404-408 and resource.rs:520-521 — 「1つにつき」 is
       `GameModifiers::last_cost_energy_count`, i.e. the energy the current cost
       step paid. All three Rust sites read that ONE field, and every caller then
       computes `matching_count / per_unit_count` and multiplies (score.rs:241,
       score.rs:423, resource.rs:624), so a missing field multiplies out to 0 and
       the effect is nullified — which is what returning 0 here reproduces. The
       previous fall-through reported 1 and applied the effect once.
       That field has no home in RbMods yet and rabuka.h:1279 forbids adding it
       unconfirmed, so util.c's RB_LAST_COST_ENERGY accessor — the one
       rb_resolve_per_unit_count uses for the same clause — yields 0 here too. */
    if (put && !strcmp(put, "\xE3\x81\xA4")) return 0;   /* つ */
    if (loc && !strcmp(loc, "\xE3\x81\xA4")) return 0;   /* つ */

    /* util.rs:2560-2575 / deck.rs:258-269 — for per_unit_type == "discard" the
       unit count is the cards MOVED BY THE CURRENT COST/EFFECT BATCH, put
       through the effect's own filter, with last_cost_discard_count as the
       fallback. util.rs:2562-2563 states the rule outright: "they should count
       only cards moved by the current cost/effect batch, not the full
       waitroom". Counting `P->discard` instead scales every 「このコストで控え室
       に置いた…」 clause off every card that ever reached the waitroom. */
    if (put && !strcmp(put, "discard"))
        return rb_resolve_discard_per_unit_count(g, g->mods.last_cost_discard_count,
                                                 card_type, f.has_group ? f.group : NULL);

    const char *zone = dc_per_unit_zone(loc ? loc : put, card_type);
    if (!zone) return 1;                              /* util.rs:2634-2636 */

    const RbPlayer *P = &g->p[pl];
    int ids[RB_MAX_ZONE];
    int n = 0;
    if (!strcmp(zone, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] != RB_EMPTY_SLOT) ids[n++] = P->stage[i];
        /* util.rs:2673-2684 — the state filter is a STAGE-only concept. */
        if (state_filter && *state_filter) {
            int kept = 0;
            for (int i = 0; i < n; i++) {
                const char *ori = rb_mods_get_orientation((RbMods *)&g->mods, ids[i]);
                if (!rb_orientation_matches_state(ori, state_filter)) continue;
                ids[kept++] = ids[i];
            }
            n = kept;
        }
    } else if (!strcmp(zone, "under_member")) {
        /* util.rs:2759-2779 `under_member_cards` — ONE member's under-cards
           (the host's own slot) when the host is on stage; the whole field only
           when the host is unknown. Walking every bag multiplies the effect by
           every member on stage. */
        int slot = -1;
        if (host_cid >= 0)
            for (int s = 0; s < RB_STAGE_SIZE; s++)
                if (P->stage[s] == host_cid) { slot = s; break; }
        int lo = (slot >= 0) ? slot : 0;
        int hi = (slot >= 0) ? slot + 1 : RB_STAGE_SIZE;
        for (int s = lo; s < hi; s++)
            for (int k = 0; k < P->under_cards[s].n && n < RB_MAX_ZONE; k++)
                ids[n++] = P->under_cards[s].cards[k];
    } else if (!strcmp(zone, "discard")) {
        for (int i = 0; i < P->discard.n && n < RB_MAX_ZONE; i++)
            ids[n++] = P->discard.cards[i];
    } else if (!strcmp(zone, "live")) {
        for (int i = 0; i < P->live.n && n < RB_MAX_ZONE; i++)
            ids[n++] = P->live.cards[i];
    } else if (!strcmp(zone, "success")) {
        for (int i = 0; i < P->success.n && n < RB_MAX_ZONE; i++)
            ids[n++] = P->success.cards[i];
    } else if (!strcmp(zone, "hand")) {
        for (int i = 0; i < P->hand.n && n < RB_MAX_ZONE; i++)
            ids[n++] = P->hand.cards[i];
    } else {
        return 1;
    }
    return dc_count_filtered(ids, n, &f, heart_colors);
}

/* Resolve an effect's repeat/draw count: return the static `count` if set,
   otherwise pull the DynamicCount parameters the decoder stored as extra_kv
   and feed them to rb_resolve_dynamic_count. Falls back to 1 when no dynamic
   parameters are present (preserves prior default). */
int rb_effect_count(const struct GameState *g, int actor, int host_cid, const AbilityEffect *e,
                     int last_draw_count)
{
    if (!e) return 0;
    const char *reference = dc_extra(e, "reference");
    const char *base_reference = dc_extra(e, "base_reference");
    const char *count_type = dc_extra(e, "count_type");
    int has_dynamic = reference || base_reference || count_type;
    int dynamic_count = -1;
    if (has_dynamic) {
        const char *calculation = dc_extra(e, "calculation");
        int calc_value = dc_extra_int(e, "calculation_value");
        const char *on_p1 = dc_extra(e, "owner_on_p1");
        int owner_on_p1 = (on_p1 && !strcmp(on_p1, "true")) ? 1 : 0;
        /* deck.rs:94-109 -- the Rust resolver hands GameState::resolve_dynamic_count
           its TRANSIENT step context, not wire data:
               moved_cards    = &self.selection.moved_cards
               selected_cards = &self.selection.cards
           Neither is a wire key: 0 of the 47 dynamic_count objects in
           cards/abilities.json carry `moved` or `selected`, so the dc_extra()
           reads this used to make were always 0 and `selected_card_score`
           (dynamic_count.rs:28-38) could never see a card. The C keeps both
           transients on the GameState — `recently_moved` is what the move choke
           point writes (rabuka.h:1417) and `selected_cards` is what a
           select_cards / select / look_and_select choice fills (rabuka.h:1429,
           read the same way by effects/misc.c:614). The constant path
           (game_state/modifiers.rs:803-809) passes empty slices, which is what
           an all-zero n_* gives here. */
        const int *moved_c = g ? g->recently_moved : NULL;
        int n_moved_c = g ? g->n_recently_moved : 0;
        const int *sel_c = g ? g->selected_cards : NULL;
        int n_sel_c = g ? g->n_selected_cards : 0;
        if (n_moved_c > RB_MAX_RECENTLY_MOVED) n_moved_c = RB_MAX_RECENTLY_MOVED;
        if (n_sel_c > RB_MAX_RECENTLY_MOVED) n_sel_c = RB_MAX_RECENTLY_MOVED;
        dynamic_count = rb_resolve_dynamic_count(
            g, actor, host_cid, reference, base_reference, count_type,
            calculation, calc_value, owner_on_p1,
            moved_c, n_moved_c, sel_c, n_sel_c,
            last_draw_count);
    }
    /* `per_unit` is a first-class decoded field (types.rs per_unit_any); the
       extra is only the fallback for an effect synthesised outside the decoder,
       matching the field-first order score.c:509 and triggers.c:714 use. */
    const char *per_unit = dc_extra(e, "per_unit");
    int is_per_unit = e->per_unit ? 1
                   : (per_unit && (!strcmp(per_unit, "true") || !strcmp(per_unit, "1")));
    if (is_per_unit) {
        const char *card_type = e->card_type_field[0] ? e->card_type_field
                                                       : dc_extra(e, "card_type");
        int units = dc_per_unit_units(g, actor, host_cid, e, card_type);
        if (units < 0) units = 0;
        /* look.rs:990 `base_count * per_mult` — resolve_per_unit_count returns
           the raw card tally and the CALLER owns the per_unit_count divisor
           (deck.rs:245/:265, score.rs:241/:392, hearts.rs:94). Dividing here
           too would halve every 「N枚につき」. */
        int base = dynamic_count >= 0 ? dynamic_count : (e->count >= 0 ? e->count : 1);
        if (base < 0) base = 1;
        return base * units;
    }
    if (dynamic_count >= 0) return dynamic_count;
    return e->count >= 0 ? e->count : 1;
}

/* ── revealed_count: mirror GameState::revealed_count ──
   Number of cards in the revealed (yell) pool belonging to `owner`.

   Rust order (engine/src/ability/dynamic_count.rs):
     1. If cheer_revealed_cards() is non-empty, return its length.
     2. Otherwise filter g.revealed_cards to those belonging to owner's
        zones: hand / waitroom / stage / under_cards / energy_zone /
        main_deck / energy_deck / live_card_zone / success_live_card_zone /
        resolution_zone.

   The C port does not maintain a separate cheer pool, so step 1 always
   falls through to the zone-filtering step. */
int rb_revealed_count(const struct GameState *g, int owner)
{
    const RbPlayer *P = &g->p[owner];
    int count = 0;

    for (int i = 0; i < g->n_revealed; i++) {
        int cid = g->revealed_cards[i];
        int in  = 0;

        /* hand */
        for (int k = 0; k < P->hand.n && !in; k++)
            if (P->hand.cards[k] == cid) { in = 1; break; }

        /* waitroom (discard bag) */
        for (int k = 0; k < P->discard.n && !in; k++)
            if (P->discard.cards[k] == cid) { in = 1; break; }

        /* stage */
        for (int k = 0; k < RB_STAGE_SIZE && !in; k++)
            if (P->stage[k] == cid) { in = 1; break; }

        /* under_cards per stage area */
        for (int a = 0; a < RB_STAGE_SIZE && !in; a++) {
            for (int j = 0; j < P->under_cards[a].n; j++)
                if (P->under_cards[a].cards[j] == cid) { in = 1; break; }
            if (in) break;
        }

        /* energy zone */
        for (int k = 0; k < P->energy.n && !in; k++)
            if (P->energy.cards[k] == cid) { in = 1; break; }

        /* main deck */
        for (int k = 0; k < P->deck.n && !in; k++)
            if (P->deck.cards[k] == cid) { in = 1; break; }

        /* energy deck */
        for (int k = 0; k < P->energy_deck.n && !in; k++)
            if (P->energy_deck.cards[k] == cid) { in = 1; break; }

        /* live card zone */
        for (int k = 0; k < P->live.n && !in; k++)
            if (P->live.cards[k] == cid) { in = 1; break; }

        /* success live card zone */
        for (int k = 0; k < P->success.n && !in; k++)
            if (P->success.cards[k] == cid) { in = 1; break; }

        /* resolution zone (global, not per-player) */
        for (int k = 0; k < g->resolution.n && !in; k++)
            if (g->resolution.cards[k] == cid) { in = 1; break; }

        if (in) count++;
    }

    return count;
}
