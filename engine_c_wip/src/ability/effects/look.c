#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

/* Look/select pools — mirrors engine/src/ability/look.rs
   For portable core we keep two pools per player:
   looked_at (cards revealed by look_at) and selected (choice result).
   The host's SELECT_CARD choice picks from looked_at; looked_at_remaining
   goes to destination (usually discard or deck). Full keep_shuffle_under
   2-phase lands with the 20-fixture harness. */

#define MAX_LOOKED 64
typedef struct {
    int cards[MAX_LOOKED];
    int n;
    int from_deck;
    int owner;
    char origin[32];
} LookPool;

static LookPool g_look[2];

static void look_pool_reset(LookPool *pool, int owner, const char *origin)
{
    pool->n = 0;
    pool->from_deck = 0;
    pool->owner = owner;
    snprintf(pool->origin, sizeof(pool->origin), "%s",
             origin && *origin ? origin : "deck_top");
}

/* rb_order_prompt_begin lives in src/ability/choice.c: it is the SHARED body of BOTH
 * deck-order producers (Rust move/mod.rs:2896-2927 for move_cards, and
 * move/mod.rs:3127-3157 for the look_and_select Finalize arm), reached through
 * rb_move_handle_order_selection -> choice.rs:2984-3055. Declared locally rather
 * than via rabuka.h, which is another owner's file. */
extern int rb_order_prompt_begin(GameState *g, int actor, const int *cards, int n,
                                int reverse_pool);

void rb_look_clear(int pl)
{
    if (pl < 0 || pl >= 2) return;
    look_pool_reset(&g_look[pl], pl, "deck_top");
}

void rb_look_reset_all(void)
{
    rb_look_clear(0);
    rb_look_clear(1);
}

int rb_look_from_deck(int pl)
{
    return pl >= 0 && pl < 2 ? g_look[pl].from_deck : 0;
}

int rb_look_remove(int pl, int cid)
{
    if (pl < 0 || pl >= 2) return 0;
    LookPool *pool = &g_look[pl];
    for (int i = 0; i < pool->n; i++) {
        if (pool->cards[i] != cid) continue;
        for (int k = i; k < pool->n - 1; k++) pool->cards[k] = pool->cards[k + 1];
        pool->n--;
        return 1;
    }
    return 0;
}

void rb_look_add(int pl, int cid)
{
    if (pl < 0 || pl >= 2) return;
    LookPool *pool = &g_look[pl];
    if (pool->n < MAX_LOOKED) pool->cards[pool->n++] = cid;
}

int rb_looked_at_pool(int pl, int *out_ids, int max)
{
    if (pl < 0 || pl >= 2 || max < 0 || (!out_ids && max > 0)) return 0;
    LookPool *pool = &g_look[pl];
    int n = pool->n < max ? pool->n : max;
    for (int i = 0; i < n; i++) out_ids[i] = pool->cards[i];
    return n;
}

static const char *look_extra(const AbilityEffect *effect, const char *key)
{
    if (!effect || !key) return NULL;
    for (int i = 0; i < effect->n_extra; i++)
        if (effect->extra_k[i] && !strcmp(effect->extra_k[i], key))
            return effect->extra_v[i];
    return NULL;
}

static int look_bool(const AbilityEffect *effect, const char *key)
{
    const char *value = look_extra(effect, key);
    return value && (!strcmp(value, "true") || !strcmp(value, "1"));
}

/* ── util::card_plural (engine/src/ability/util/labels.rs:8-10) ───────────
   "card" for count==1, "cards" otherwise. Used by every choice description
   look.rs builds. */
static const char *look_card_plural(int count)
{
    return count == 1 ? "card" : "cards";
}

/* ── describe::zone_label / zone_label_ja (engine/src/ability/describe.rs:22)
   Prompt text only. `None` renders "unknown" / 「不明」 exactly as Rust does. */
static const char *look_zone_label(const char *zone)
{
    if (!zone) return "unknown";
    if (!strcmp(zone, "hand")) return "hand";
    if (!strcmp(zone, "discard")) return "the waiting room";
    if (!strcmp(zone, "deck")) return "deck";
    if (!strcmp(zone, "deck_top")) return "top of deck";
    if (!strcmp(zone, "deck_bottom")) return "bottom of deck";
    if (!strcmp(zone, "stage")) return "stage";
    if (!strcmp(zone, "energy")) return "energy";
    if (!strcmp(zone, "energy_deck")) return "energy deck";
    if (!strcmp(zone, "energy_zone")) return "energy zone";
    if (!strcmp(zone, "waitroom")) return "wait room";
    if (!strcmp(zone, "success_zone")) return "success zone";
    if (!strcmp(zone, "live_card_zone")) return "live card zone";
    if (!strcmp(zone, "under_member")) return "under this member";
    if (!strcmp(zone, "revealed_cards")) return "revealed cards";
    if (!strcmp(zone, "those_cards")) return "those cards";
    if (!strcmp(zone, "all_selected")) return "selected cards";
    if (!strcmp(zone, "looked_at")) return "looked_at";
    return zone;
}

static const char *look_zone_label_ja(const char *zone)
{
    if (!zone) return "\xE6\x9C\xAA\xE6\x98\x8E"; /* 不明 */
    if (!strcmp(zone, "hand")) return "\xE6\x89\x8B\xE7\xAE\x8D"; /* 手札 */
    if (!strcmp(zone, "discard") || !strcmp(zone, "waitroom"))
        return "\xE6\x82\x9A\xE3\x81\x88\xE5\xAE\xA4"; /* 控え室 */
    if (!strcmp(zone, "deck")) return "\xE3\x83\x87\xE3\x83\x83\xE3\x82\xAD"; /* デッキ */
    if (!strcmp(zone, "deck_top"))
        return "\xE3\x83\x87\xE3\x83\x83\xE3\x82\xAD\xE3\x81\xAE\xE4\xB8\x8A";
    if (!strcmp(zone, "deck_bottom"))
        return "\xE3\x83\x87\xE3\x83\x83\xE3\x82\xAD\xE3\x81\xAE\xE4\xB8\x8B";
    if (!strcmp(zone, "stage"))
        return "\xE3\x82\xB9\xE3\x83\x86\xE3\x83\xBC\xE3\x82\xB8"; /* ステージ */
    if (!strcmp(zone, "energy")) return "\xE3\x82\xA8\xE3\x83\x8D\xE3\x83\xAB\xE3\x82\xAE\xE3\x83\xBC";
    if (!strcmp(zone, "energy_zone"))
        return "\xE3\x82\xA8\xE3\x83\x8D\xE3\x83\xAB\xE3\x82\xAE\xE3\x83\xBC\xE3\x82\xBE\xE3\x83\xBC\xE3\x83\xB3";
    if (!strcmp(zone, "live_card_zone"))
        return "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE3\x82\xAB\xE3\x83\xBC\xE3\x83\x89\xE3\x82\xBE\xE3\x83\xBC\xE3\x83\xB3";
    if (!strcmp(zone, "revealed_cards"))
        return "\xE5\x85\xAC\xE9\x96\x8B\xE3\x81\x95\xE3\x82\x8C\xE3\x81\x9F\xE3\x82\xAB\xE3\x83\xBC\xE3\x83\x89";
    if (!strcmp(zone, "success_zone"))
        return "\xE6\x88\x90\xE5\x8A\x9F\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE3\x82\xAB\xE3\x83\xBC\xE3\x83\x89\xE7\xBD\xAE\xE3\x81\x8D\xE5\xA0\xB4";
    return zone;
}

/* util::target_player_label (engine/src/ability/util/labels.rs consumers) —
   "P1"/"P2" for the resolving seat, used by the reveal rule log. */
static const char *look_player_label(const GameState *g, int who)
{
    if (!g) return "P1";
    return (who == 1) ? "P2" : "P1";
}

/* ── look.rs:355-398 rule log: "[Turn N] P? [[log_reveal_zone:source=Z]] » names" */
static void look_push_reveal_log(const GameState *g, int who, const char *source,
                                 const char *names)
{
    char line[224];
    snprintf(line, sizeof(line), "[Turn %d] %s [[log_reveal_zone:source=%s]] \xC2\xBB %s",
             g ? g->turn : 0, look_player_label(g, who), source ? source : "", names);
    rb_log_push_verdict(line, "rule_log", 1);
}

/* look.rs:1188-1194 rule log for the per-group reveal. */
static void look_push_reveal_group_log(const GameState *g, int who, const char *source, int n)
{
    char line[224];
    snprintf(line, sizeof(line), "[Turn %d] %s [[log_reveal_group:n=%d,source=zone_%s]]",
             g ? g->turn : 0, look_player_label(g, who), n, source ? source : "");
    rb_log_push_verdict(line, "rule_log", 1);
}

/* look.rs:1303-1309 rule log for the reveal-until walk. */
static void look_push_reveal_until_log(const GameState *g, int who, const char *names, int found)
{
    char line[224];
    snprintf(line, sizeof(line), "[Turn %d] %s [[log_reveal_deck_until:found=%s]]: %s",
             g ? g->turn : 0, look_player_label(g, who), found ? "true" : "false", names);
    rb_log_push_verdict(line, "rule_log", 1);
}

/* util::player_prefix (engine/src/ability/util.rs:490-502) — which seat owns
   the activating card; used by the 「確認 N枚」 / 「カード選択 N枚」 logs. */
static const char *look_player_prefix(const GameState *g, int card_id)
{
    if (g && card_id >= 0) {
        for (int pl = 0; pl < 2; pl++) {
            const RbPlayer *P = &g->p[pl];
            for (int q = 0; q < RB_STAGE_SIZE; q++) if (P->stage[q] == card_id) return pl == 0 ? "P1" : "P2";
            for (int i = 0; i < P->live.n; i++) if (P->live.cards[i] == card_id) return pl == 0 ? "P1" : "P2";
            for (int i = 0; i < P->hand.n; i++) if (P->hand.cards[i] == card_id) return pl == 0 ? "P1" : "P2";
        }
    }
    return (g && g->active == 1) ? "P2" : "P1";
}

static const char *look_activating_card_name(const GameState *g, char *out, size_t out_sz)
{
    out[0] = '\0';
    if (!g || g->activating_card < 0) return out;
    Card c;
    if (rb_decode_card_by_index((uint32_t)g->activating_card, &c)) {
        if (c.name) strncpy(out, c.name, out_sz - 1);
        out[out_sz - 1] = '\0';
        rb_free_card(&c);
    }
    return out;
}

/* look.rs:1087-1094 / :891-898 — the 「確認 N枚」 and 「カード選択 N枚」 logs. */
static void look_push_look_log(const GameState *g, int count)
{
    char act[64];
    char line[160];
    look_activating_card_name(g, act, sizeof(act));
    snprintf(line, sizeof(line), "%s %s: \xE7\xA4\xBA\xE8\xAA\x8D %d\xE6\x9E\x9A",
             look_player_prefix(g, g ? g->activating_card : -1), act, count);
    rb_log_push_verdict(line, "rule_log", 1);
}

static void look_push_select_log(const GameState *g, int count)
{
    char act[64];
    char line[160];
    look_activating_card_name(g, act, sizeof(act));
    snprintf(line, sizeof(line), "%s %s: \xE3\x82\xAB\xE3\x83\xBC\xE3\x83\x89\xE9\x81\xB8\xE6\x8A\x9E %d\xE6\x9E\x9A",
             look_player_prefix(g, g ? g->activating_card : -1), act, count);
    rb_log_push_verdict(line, "rule_log", 1);
}

/* ── or_card_types (look.rs::handle_or_card_types) ──────────────────────────
   Rust parks `ConditionalChoice::Str` on the queue entry so the re-executed
   effect reads the answer back. `RbQueueEntry` has no such field, so the port
   keeps the issued-gate marker and the resolved answer in this file; the answer
   itself arrives as `queue.choice_result`, which choice.c sets from the picked
   index before dispatching (choice.c:3992). */
static int  g_look_or_gate_issued;      /* 1 between the gate and its answer */
static char g_look_or_answer[32];       /* resolved type, e.g. "member_card"  */

/* Split the comma-separated `or_card_types` extra (defined after
   look_split_list). */

/* Defined further down (they need the look_* filter helpers); forward-declared
   so the earlier executors can reuse them. */
static int look_filter_matches(const GameState *g, const AbilityEffect *effect, int cid);
static int look_group_list_matches(int cid, const char *group_list);
static int look_select_matches(GameState *g, const AbilityEffect *select, int cid);
/* The select effect behind the currently-offered looked_at pool, and the
   look_and_select follow-up that has to outlive a multi-pick re-prompt. Both
   are parked at OFFER time because choice.c clears queue.resume_eff and
   consumes queue.resume_after_look on the first resume.

   ── OWNERSHIP: STILL AN ALIAS, AND MEASURED TO BE ────────────────────────
   Both slots park the node they were handed (`e`, `sel`, `select_action`,
   `e->followup_action`) -- a pointer INTO a tree some other owner frees. That
   is the alias-defect shape, and it is why the three landed fixes were
   deferrals rather than ownership: a producer that frees its own tree on a
   prompt-open pass leaves these slots dangling, and the answer path, which runs
   after that free, reads them again (rb_look_resume_indices, the
   per_group_count / placement_order / re-prompt lookups).

   Converting them to OWNING `rb_effect_deep_clone` copies -- Rust's model, where
   `effect.target.clone()` (choice.rs:120) is an owned String and the parked tail
   is COPIED back by requeue_remaining_after_choice (choice.rs:87-105) -- was
   ATTEMPTED on 2026-10-05, MEASURED, and REVERTED. The reason is not the clone;
   it is that these slots are NOT INDEPENDENT OWNERS. See the note on
   look_park_select below for the measurement and the mechanism.

   What did land is that the slots are now MEASURABLE (rb_look_slot_* below) and
   that the property blocking the conversion is pinned by
   tests/test_alias_ownership.c, so the attempt is not repeated from scratch. */
static AbilityEffect *g_look_pending_select;
static AbilityEffect *g_look_reprompt_followup;
/* Set only by the looked-at SELECT offers. rb_effect_select / _select_cards
   also drive rb_look_resume_indices (with a hand pool), and must not be able to
   inherit a looked-at select's re-prompt state. */
static int g_look_pool_select_offered;
static void look_discard_unmatched(GameState *g, int owner, AbilityEffect *parent);
static const char *look_filter_card_type(const AbilityEffect *effect);

/* ── the two parked slots: OWNED, not aliased (see the note at the decls) ── */

/* Park an OWNED DEEP CLONE of `alias`, releasing whatever the slot held.
   The caller keeps ownership of `alias` and may free its tree at any time --
   including on the pass that opens the prompt, which is the case the alias
   form could not survive. A failed clone parks NULL rather than the alias:
   degrading to "no re-prompt state" is correct, degrading back to the alias
   would reintroduce the defect exactly when memory is short. */
/* Park the select effect behind the currently-offered looked_at pool.
 *
 * ⛔ THIS SLOT CANNOT BE CONVERTED TO AN OWNING CLONE ON ITS OWN. The attempt
 * was measured and reverted on 2026-10-05, and the reason is ownership, not the
 * clone: `g_look_pending_select` and `g->queue.resume_eff` hold THE SAME NODE on
 * the multi-pick re-prompt path.
 *
 * rb_look_resume_indices reads the parked value into `sel` and hands it to
 * look_emit_pool_choice, whose body is, in this order:
 *
 *     795:  g->queue.resume_eff = (AbilityEffect *)sel;
 *     797:  look_park_select((AbilityEffect *)sel);
 *
 * So on that path `sel` IS the previously parked value. A park that clones and
 * then RELEASES the old value releases the node `resume_eff` published one line
 * earlier. That is not a subtle degradation:
 *
 *   MEASURED (same-snapshot A/B -- tools/alias_probe.sh --prechange versus the
 *   converted arm, ONE tree state, three suites, no rebuild between):
 *     pre-change  alias        : 0 alias hits; FAIL 37 / 65 / 44
 *     converted   own+release  : 32 alias hits, ALL on queue.resume_eff with
 *                                 slot_value == freed_root; FAIL 41 / 73 / 44
 *
 *   The owning clone did not merely fail to help. It REMOVED the node that
 *   `resume_eff` was aliasing, i.e. it turned a two-slot alias into a live
 *   use-after-free, and it cost 10 previously-passing assertions.
 *
 * The correct conversion is therefore a JOINT one: `resume_eff`,
 * `resume_after_look` and this file-static become owning copies IN THE SAME
 * PASS, with `resume_eff` taking ownership of a clone that the other slots then
 * either share or clone from. Converting any one of them alone produces the
 * numbers above. That is the concrete demonstration of why a half-converted
 * ownership scheme is worse than either end state -- it is a third state with
 * the alias's fragility and none of the target's safety.
 *
 * The slot therefore stays an alias, exactly as before 2026-10-05. What changed
 * is that it is now MEASURABLE and its blocking property is pinned:
 * rb_look_slot_pending_select below, and tests/test_alias_ownership.c asserts
 * the sharing, so the conversion cannot be rediscovered by accident. */
static void look_park_select(AbilityEffect *alias)
{
    g_look_pending_select = alias;
}

/* The consumer's half. Clears the pointer; it must NOT free, because the value
   is still reachable from g->queue.resume_eff (see above) and from the ability
   tree that produced it. */
static void look_clear_select(void)
{
    g_look_pending_select = NULL;
}

/* ── READ-ONLY parked-slot diagnostics ─────────────────────────────────────
   Two accessors and nothing else. They exist so the alias-class probe
   (tools/alias_probe.c, linked with -Wl,--wrap) and the ownership regression
   test (tests/test_alias_ownership.c) can ASSERT on what is parked instead of
   inferring it. Before these, `static` meant no linker could name either slot,
   which is part of why the class went unmeasured for as long as it did.
   Read-only by contract: writing through the returned pointer would make it
   an alias again, which is the defect. */
const AbilityEffect *rb_look_slot_pending_select(void)    { return g_look_pending_select; }
const AbilityEffect *rb_look_slot_reprompt_followup(void) { return g_look_reprompt_followup; }

static int look_target_player(const GameState *g, int actor, const AbilityEffect *effect)
{
    int who = actor;
    if (g && actor >= 0 && actor < 2 && g->queue.actor >= 0 && g->queue.actor < 2)
        who = g->queue.actor;
    if (effect && effect->target && !strcmp(effect->target, "opponent")) {
        if (who >= 0 && who < 2) who ^= 1;
    }
    if (who < 0 || who >= 2) who = 0;
    return who;
}

static int look_take_top(RbBag *bag)
{
    if (!bag || bag->n <= 0) return -1;
    int cid = bag->cards[0];
    for (int i = 1; i < bag->n; i++) bag->cards[i - 1] = bag->cards[i];
    bag->n--;
    return cid;
}

static int look_take_bottom(RbBag *bag)
{
    if (!bag || bag->n <= 0) return -1;
    return bag->cards[--bag->n];
}

static void look_append(LookPool *pool, int cid)
{
    if (pool->n < MAX_LOOKED) pool->cards[pool->n++] = cid;
}

/* Split a comma-separated wire list ("heart04,heart05" / "Aqours,Liella!")
   into tokens. Mirrors how the Rust CardFilter reads its Vec<String> fields. */
static int look_split_list(const char *value, const char **out, int max, char *buffer, size_t cap)
{
    int n = 0;
    if (!value || !*value || !out || max <= 0) return 0;
    snprintf(buffer, cap, "%s", value);
    char *token = strtok(buffer, ",");
    while (token && n < max) {
        out[n++] = token;
        token = strtok(NULL, ",");
    }
    return n;
}

/* Split the comma-separated `or_card_types` extra. */
static int look_or_card_types(const AbilityEffect *effect, const char **out, int max,
                              char *buffer, size_t cap)
{
    return look_split_list(look_extra(effect, "or_card_types"), out, max, buffer, cap);
}

/* look.rs::filter_select_candidates (look.rs:443-465) — the card-type plus
   heart-colour gate shared by the looked_at arms of reveal and select.
   An absent/empty filter accepts everything. */
static int look_reveal_candidate_ok(const AbilityEffect *effect, int cid, const char *card_type)
{
    if (card_type && *card_type && !rb_card_matches_type(cid, card_type)) return 0;
    const char *heart = look_extra(effect, "heart_colors");
    if (!heart) heart = look_extra(effect, "heart_color");
    if (!heart || !*heart) return 1;
    const char *colors[8];
    char buffer[192];
    int n_colors = look_split_list(heart, colors, 8, buffer, sizeof(buffer));
    if (n_colors <= 0) return 1;
    int require_all = look_bool(effect, "require_all_heart_colors");
    return require_all ? rb_card_matches_all_heart_colors(cid, colors, n_colors)
                       : rb_card_matches_heart_colors(cid, colors, n_colors);
}

static void look_record_revealed(GameState *g, const int *cards, int n)
{
    if (!g) return;
    for (int i = 0; i < n; i++)
        if (g->n_revealed < RB_MAX_REVEALED_CARDS)
            g->revealed_cards[g->n_revealed++] = cards[i];
}

static int look_at_fetch(GameState *g, int actor, AbilityEffect *effect)
{
    if (!g || !effect) return 0;
    int who = look_target_player(g, actor, effect);
    RbPlayer *player = &g->p[who];
    const char *source = effect->source && *effect->source ? effect->source : "deck";
    int count = rb_effect_count(g, who, -1, effect, g->last_draw_count);
    if (count < 0) count = 0;
    if (look_bool(effect, "all")) count = RB_MAX_ZONE;
    LookPool *pool = &g_look[who];
    int deck_source = !strcmp(source, "deck") || !strcmp(source, "deck_top") ||
                      !strcmp(source, "deck_bottom");
    look_pool_reset(pool, who, source);
    pool->from_deck = deck_source;
    if (!strcmp(source, "deck") || !strcmp(source, "deck_top")) {
        int initial = player->deck.n;
        int take = initial < count ? initial : count;
        for (int i = 0; i < take; i++) look_append(pool, look_take_top(&player->deck));
        if (pool->n < count && player->deck.n == 0 && player->discard.n > 0) {
            rb_player_refresh(g, who);
            int remaining = count - pool->n;
            take = player->deck.n < remaining ? player->deck.n : remaining;
            for (int i = 0; i < take; i++) look_append(pool, look_take_top(&player->deck));
        }
    } else if (!strcmp(source, "deck_bottom")) {
        int take = player->deck.n < count ? player->deck.n : count;
        int taken[MAX_LOOKED];
        for (int i = 0; i < take; i++) taken[i] = look_take_bottom(&player->deck);
        for (int i = take - 1; i >= 0; i--) look_append(pool, taken[i]);
    } else if (!strcmp(source, "hand")) {
        for (int i = 0; i < player->hand.n && i < count; i++) look_append(pool, player->hand.cards[i]);
    } else if (!strcmp(source, "discard") || !strcmp(source, "waitroom")) {
        for (int i = 0; i < player->discard.n && i < count; i++) look_append(pool, player->discard.cards[i]);
    } else if (!strcmp(source, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE && i < count; i++)
            if (player->stage[i] != RB_EMPTY_SLOT) look_append(pool, player->stage[i]);
    }
    return pool->n;
}

/* look.rs:1055-1096 — execute_look_at FILLS gs.looked_at_cards, records
 * `carried.looked_at_origin` and returns Ok(()). It opens NO Choice: the ONLY
 * prompt it can raise is maybe_offer_look_cost_gate (look.rs:995-1013), an
 * OptionalCost skip gate, and only when the effect carries `optional` —
 * MEASURED 0 of 200 look_at nodes on this corpus. 見る informs (rule 5.7), so
 * the prompt the player answers belongs to whatever CONSUMES the pool; for
 * 「3枚見て好きな順番でデッキの上に置く」 that is the deck-order SelectTarget
 * (move/mod.rs:1915-1932, raised at :2272), never a card pick over the freshly
 * revealed pool.
 *
 * This body used to emit SelectCard{zone:"looked_at", count} and PARK the queue,
 * which is why a `sequential` whose step 0 was a look_at never reached step 1
 * (MEASURED 15 such steps). The pool fill and the origin recording both already
 * happen in look_at_fetch -> look_pool_reset (:359), which is the C's
 * `carried.looked_at_origin`, so only the emit + park block goes. */
void rb_effect_look_at(GameState *g, int actor, AbilityEffect *e)
{
    int who = look_target_player(g, actor, e);
    int count = look_at_fetch(g, actor, e);
    if (count <= 0) return;
    look_push_look_log(g, count);
    g->queue.resume_look_owner = who;
    look_clear_select();
    g_look_pool_select_offered = 0;
}

void rb_effect_select_cards(GameState *g, int actor, AbilityEffect *e){
    /* If we have a looked_at pool, the choice is from it; otherwise from hand */
    int who = actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    LookPool *lp=&g_look[who];
    const char *zone = lp->n>0 ? "looked_at" : (e->source ? e->source : "hand");
    const char *ctype=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"card_type")) ctype=e->extra_v[i];
    int cnt=e->count>=0?e->count:1;
    /* Heart-color selection (C-port prompt shape: "pick a heart colour", not a
       card pick). Rust's execute_select_cards has no such branch, so the
       looked_at matching / clamping below deliberately does not apply here. */
    int has_heart_color = 0;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && (!strcmp(e->extra_k[i], "heart_color") ||
                              !strcmp(e->extra_k[i], "heart_colors")))
            has_heart_color = 1;
    if (!has_heart_color) {
        /* look.rs:753-820 — keep every looked-at card, but restrict the
           selectable indices to those passing the filter, and clamp the
           offered count to the matching count. */
        int matching[MAX_LOOKED];
        int n_matching = 0;
        for (int i = 0; i < lp->n; i++)
            if (look_select_matches(g, e, lp->cards[i])) matching[n_matching++] = i;
        if (n_matching == 0) {
            /* look.rs:804-812 — nothing matched: discard the pool, offer
               nothing, and return. */
            look_discard_unmatched(g, who, NULL);
            return;
        }
        int any_number = look_bool(e, "any_number");
        if (any_number) cnt = n_matching;
        else if (cnt > n_matching) cnt = n_matching;
        if (cnt <= 0) return;
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, zone, ctype, cnt,
                       e->is_optional?1:0, NULL);
        rb_queue_pause_for_choice(g, &g->queue.pending);
        g->queue.pending.n_filtered_indices = n_matching;
        for (int i = 0; i < n_matching; i++)
            g->queue.pending.filtered_indices[i] = matching[i];
        /* Snapshot the SelectionContext so rb_look_resume can re-validate the
           pick after the pending choice is cleared. Without this the resume
           would inherit a stale resume_filter_heart (0 = heart00) and reject
           every card. */
        g->queue.pending.filter_group[0] = 0;
        g->queue.pending.filter_heart = -1;
        const char *grp = look_extra(e, "group_names");
        if (grp) snprintf(g->queue.pending.filter_group,
                          sizeof(g->queue.pending.filter_group), "%s", grp);
        strncpy(g->queue.resume_filter_group, g->queue.pending.filter_group,
                sizeof(g->queue.resume_filter_group)-1);
        g->queue.resume_filter_heart = g->queue.pending.filter_heart;
        g->queue.resume_mode = 2; g->queue.resume_eff = e; g->queue.resume_is_select = 1;
        look_park_select(e);
        g_look_pool_select_offered = 0;
        g->queue.resume_actor = actor;
        g->queue.resume_host = g->queue.resume_host >= 0 ? g->queue.resume_host : actor;
        return;
    }
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, zone, ctype, cnt, e->is_optional?1:0, NULL);
    rb_queue_pause_for_choice(g, &g->queue.pending);
    /* SelectionContext filter (ability/choice.rs): narrow the valid pool to a
        group and/or heart color so a host UI / test picks a legal card. */
    g->queue.pending.filter_group[0] = 0;
    g->queue.pending.filter_heart = -1;
    for(int i=0;i<e->n_extra;i++){
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"group_names") && e->extra_v[i])
            strncpy(g->queue.pending.filter_group, e->extra_v[i], sizeof(g->queue.pending.filter_group)-1);
        else if(e->extra_k[i] && !strcmp(e->extra_k[i],"heart_colors") && e->extra_v[i]){
            const char *hc=e->extra_v[i];
            int col=-1;
            if(!strcmp(hc,"pink")||!strcmp(hc,"heart00")) col=0;
            else if(!strcmp(hc,"red")) col=1; else if(!strcmp(hc,"yellow")) col=2;
            else if(!strcmp(hc,"green")) col=3; else if(!strcmp(hc,"blue")) col=4;
            else if(!strcmp(hc,"purple")) col=5; else if(!strcmp(hc,"orange")) col=6;
            else if(!strcmp(hc,"all")) col=7;
            g->queue.pending.filter_heart = col;
        }
    }
    /* snapshot the filter so rb_look_resume can validate after the pending choice is cleared */
    strncpy(g->queue.resume_filter_group, g->queue.pending.filter_group, sizeof(g->queue.resume_filter_group)-1);
    g->queue.resume_filter_heart = g->queue.pending.filter_heart;
    /* Heart-color selection (mirrors Rust execute_choice → conditional_choice =
        Str(color)). A select with a heart_color extra is a "pick a heart color"
        prompt; stash the chosen color so the following gain_resource applies it.
        Route it through the default resume branch (mode 0) so the parent's later
        siblings (the gain) run after the choice resolves — NOT the card-select
        look/keep path (mode 2). */
    g->queue.selected_heart_color = -1;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && (!strcmp(e->extra_k[i], "heart_color") ||
                              !strcmp(e->extra_k[i], "heart_colors")) && e->extra_v[i]) {
            int col = -1;
            if (!strcmp(e->extra_v[i], "pink") || !strcmp(e->extra_v[i], "heart00")) col = 0;
            else if (!strcmp(e->extra_v[i], "red") || !strcmp(e->extra_v[i], "heart01")) col = 1;
            else if (!strcmp(e->extra_v[i], "yellow") || !strcmp(e->extra_v[i], "heart02")) col = 2;
            else if (!strcmp(e->extra_v[i], "green") || !strcmp(e->extra_v[i], "heart03")) col = 3;
            else if (!strcmp(e->extra_v[i], "blue") || !strcmp(e->extra_v[i], "heart04")) col = 4;
            else if (!strcmp(e->extra_v[i], "purple") || !strcmp(e->extra_v[i], "heart05")) col = 5;
            else if (!strcmp(e->extra_v[i], "orange") || !strcmp(e->extra_v[i], "heart06")) col = 6;
            else if (!strcmp(e->extra_v[i], "all") || !strcmp(e->extra_v[i], "heart07") || !strcmp(e->extra_v[i], "b_all")) col = 7;
            if (col >= 0) g->queue.selected_heart_color = col;
        }
    }
    g->queue.resume_mode = 0; g->queue.resume_is_select = 0;
    g->queue.resume_eff = e; g->queue.resume_actor = actor; g->queue.resume_host = g->queue.resume_host >= 0 ? g->queue.resume_host : actor;
}

/* Called when host resumes SELECT_CARD — move chosen card to destination.
   Mirrors engine/src/ability/look.rs keep_shuffle_under: cards that came
   from the deck and were NOT kept are shuffled back into the owner's deck
   (under), not discarded; hand-sourced cards return to hand; the rest go to
   the destination / discard. */
static int look_card_allowed(const GameState *g, int cid)
{
    if (!g) return 0;
    if (g->queue.resume_is_select && g->queue.resume_eff) {
        /* util.rs:1509-1532 re-checks the whole CardFilter on the answer; the
           old port reduced it to ONE group token plus ONE heart colour, so a
           comma-list heart_colors collapsed to heart00 and rejected every pick
           (nothing reached hand, everything went to the waitroom). The pool was
           already narrowed by the same filter when it was offered, so re-running
           it here is the faithful check. */
        return look_filter_matches(g, g->queue.resume_eff, cid);
    }
    if (g->queue.resume_filter_group[0] &&
        !look_group_list_matches(cid, g->queue.resume_filter_group)) return 0;
    if (g->queue.resume_filter_heart < 0) return 1;
    Card card;
    if (!rb_decode_card_by_index((uint32_t)cid, &card)) return 0;
    int found = 0;
    for (int i = 0; i < card.n_hearts; i++)
        if (card.heart_color[i] == g->queue.resume_filter_heart) found = 1;
    rb_free_card(&card);
    return found;
}

static void look_remove_origin_card(RbPlayer *player, const char *origin, int cid)
{
    if (!player || !origin) return;
    RbBag *bag = NULL;
    if (!strcmp(origin, "hand")) bag = &player->hand;
    else if (!strcmp(origin, "discard") || !strcmp(origin, "waitroom")) bag = &player->discard;
    else if (!strcmp(origin, "live_card_zone") || !strcmp(origin, "live")) bag = &player->live;
    else if (!strcmp(origin, "success_live_zone") || !strcmp(origin, "success")) bag = &player->success;
    if (bag) {
        for (int i = 0; i < bag->n; i++) {
            if (bag->cards[i] != cid) continue;
            for (int k = i; k < bag->n - 1; k++) bag->cards[k] = bag->cards[k + 1];
            bag->n--;
            return;
        }
    } else if (!strcmp(origin, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            if (player->stage[i] != cid) continue;
            player->stage[i] = RB_EMPTY_SLOT;
            player->stage_wait[i] = 0;
            return;
        }
    }
}

static void look_append_cards(RbBag *bag, const int *cards, int n)
{
    if (!bag) return;
    for (int i = 0; i < n && bag->n < RB_MAX_ZONE; i++) bag->cards[bag->n++] = cards[i];
}

static void look_prepend_cards(RbBag *bag, const int *cards, int n)
{
    if (!bag || n <= 0) return;
    int count = n;
    if (count > RB_MAX_ZONE - bag->n) count = RB_MAX_ZONE - bag->n;
    for (int i = bag->n - 1; i >= 0; i--) bag->cards[i + count] = bag->cards[i];
    for (int i = 0; i < count; i++) bag->cards[i] = cards[i];
    bag->n += count;
}

static void look_place_stage(RbPlayer *player, const int *cards, int n)
{
    if (!player) return;
    for (int i = 0; i < n; i++) {
        int slot = -1;
        for (int j = 0; j < RB_STAGE_SIZE; j++)
            if (player->stage[j] == RB_EMPTY_SLOT) { slot = j; break; }
        if (slot < 0) {
            if (player->discard.n < RB_MAX_ZONE)
                player->discard.cards[player->discard.n++] = cards[i];
            continue;
        }
        player->stage[slot] = cards[i];
        player->stage_wait[slot] = 0;
    }
}

/* ── multi-pick re-prompt (move/mod.rs:3184-3230) ──────────────────────────
   「最大N枚」 keeps offering the looked_at pool until the count is satisfied.
   g_look_pending_select / g_look_reprompt_followup are declared with the
   forward declarations above; see there why they cannot be read off the queue
   at resume time. */

/* max_select = any_number ? (taken + remaining) : the JSON count
   (move/mod.rs:3190-3194). */
static int look_select_max_count(const AbilityEffect *sel, int n_selected, int n_remainder)
{
    int json_count = sel && sel->count >= 0 ? sel->count : 1;
    return look_bool(sel, "any_number") ? (n_selected + n_remainder) : json_count;
}

/* can_select_more && max_select > selected_count (move/mod.rs:3195-3207). */
static int look_select_wants_more(GameState *g, const AbilityEffect *sel,
                                  int n_selected, int n_remainder)
{
    if (!sel) return 0;
    if (g) g_look_reprompt_followup = g->queue.resume_after_look;
    int json_count = sel->count >= 0 ? sel->count : 1;
    int any_number = look_bool(sel, "any_number");
    int is_max = look_bool(sel, "max");
    int is_optional = sel->is_optional;
    int can_select_more = is_max || is_optional || any_number || json_count > n_selected;
    if (!can_select_more) return 0;
    return look_select_max_count(sel, n_selected, n_remainder) > n_selected;
}

/* Emit the looked_at SelectCard for a (re-)offered pool. */
static void look_emit_pool_choice(GameState *g, int owner, const AbilityEffect *sel,
                                  const int *filtered, int n_filtered,
                                  int offer, int allow_skip)
{
    const char *card_type = look_filter_card_type(sel);
    rb_emit_choice(g, owner, RB_CHOICE_SELECT_CARD, "looked_at", card_type,
                   offer, allow_skip, NULL);
    g->queue.pending.n_filtered_indices = n_filtered;
    for (int i = 0; i < n_filtered; i++)
        g->queue.pending.filtered_indices[i] = filtered[i];
    const char *group = look_extra(sel, "group_names");
    if (!group) group = look_extra(sel, "group");
    g->queue.pending.filter_group[0] = 0;
    g->queue.resume_filter_group[0] = 0;
    if (group) {
        snprintf(g->queue.pending.filter_group, sizeof(g->queue.pending.filter_group), "%s", group);
        snprintf(g->queue.resume_filter_group, sizeof(g->queue.resume_filter_group), "%s", group);
    }
    const char *heart = look_extra(sel, "heart_colors");
    if (!heart) heart = look_extra(sel, "heart_color");
    int heart_color = -1;
    if (heart && *heart && !strchr(heart, ',')) heart_color = rb_parse_heart_color(heart);
    g->queue.pending.filter_heart = heart_color;
    g->queue.resume_filter_heart = heart_color;
    const char *cost_limit = look_extra(sel, "cost_limit");
    if (cost_limit) {
        g->queue.pending.cost_limit = atoi(cost_limit);
        const char *operator_text = look_extra(sel, "cost_limit_operator");
        if (!operator_text) operator_text = look_extra(sel, "operation");
        if (operator_text)
            snprintf(g->queue.pending.cost_limit_op,
                     sizeof(g->queue.pending.cost_limit_op), "%s", operator_text);
    }
    snprintf(g->queue.pending.target_player_id,
             sizeof(g->queue.pending.target_player_id), "p%d", owner + 1);
    g->queue.resume_look_owner = owner;
    rb_queue_pause_for_choice(g, &g->queue.pending);
    g->queue.resume_mode = 2;
    g->queue.resume_eff = (AbilityEffect *)sel;
    g->queue.resume_is_select = 1;
    look_park_select((AbilityEffect *)sel);
    g_look_pool_select_offered = 1;
}

/* move/mod.rs:3070-3090 — 「1シリーズ1枚」 is validated BEFORE anything is
   removed: if the answer names more cards of one series than the printed
   per_group_count allows, the WHOLE answer is rejected and the pool is
   untouched. The C port had no per_group_count at all, so a same-series pair
   both reached the hand. */
static int look_per_group_ok(const int *selected, int n_selected, int max_per_group)
{
    if (max_per_group <= 0 || n_selected <= 1) return 1;
    const char *series[MAX_LOOKED];
    int n_series = 0;
    for (int i = 0; i < n_selected; i++) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)selected[i], &c)) continue;
        const char *name = c.series_idx ? rb_card_string(c.series_idx) : NULL;
        rb_free_card(&c);
        if (name && *name) series[n_series++] = name;
    }
    for (int i = 0; i < n_series; i++) {
        int count = 0;
        for (int k = 0; k < n_series; k++)
            if (!strcmp(series[k], series[i])) count++;
        if (count > max_per_group) return 0;
    }
    return 1;
}

void rb_look_resume_indices(GameState *g, int owner, const int *indices, int n_indices,
                            const char *destination, const char *remainder_destination,
                            int discard_remaining, int is_select)
{
    if (!g || owner < 0 || owner >= 2) return;
    LookPool *pool = &g_look[owner];
    RbPlayer *player = &g->p[owner];
    int selected_flags[MAX_LOOKED] = {0};
    int selected[MAX_LOOKED];
    int n_selected = 0;
    for (int i = 0; i < n_indices; i++) {
        int index = indices ? indices[i] : -1;
        if (index < 0 || index >= pool->n || selected_flags[index]) continue;
        int cid = pool->cards[index];
        if (is_select && !look_card_allowed(g, cid)) continue;
        selected_flags[index] = 1;
        selected[n_selected++] = cid;
    }
    if (is_select && g_look_pool_select_offered && g_look_pending_select) {
        const char *mpg_text = look_extra(g_look_pending_select, "per_group_count");
        if (mpg_text && *mpg_text && !look_per_group_ok(selected, n_selected, atoi(mpg_text)))
            n_selected = 0;   /* the whole answer is rejected; the pool survives */
    }
    int remainder[MAX_LOOKED];
    int n_remainder = 0;
    for (int i = 0; i < pool->n; i++)
        if (!selected_flags[i]) remainder[n_remainder++] = pool->cards[i];
    if (!pool->from_deck) {
        for (int i = 0; i < n_selected; i++) look_remove_origin_card(player, pool->origin, selected[i]);
        for (int i = 0; i < n_remainder; i++) look_remove_origin_card(player, pool->origin, remainder[i]);
    }
    if (n_selected == 0 && discard_remaining < 0 &&
        (!remainder_destination || !*remainder_destination)) {
        if (!strcmp(pool->origin, "deck_bottom")) {
            look_append_cards(&player->deck, remainder, n_remainder);
        } else if (!strcmp(pool->origin, "deck") || !strcmp(pool->origin, "deck_top")) {
            int ordered[MAX_LOOKED];
            for (int i = 0; i < n_remainder; i++) ordered[i] = remainder[n_remainder - 1 - i];
            look_prepend_cards(&player->deck, ordered, n_remainder);
        } else if (!strcmp(pool->origin, "hand")) {
            look_append_cards(&player->hand, remainder, n_remainder);
        } else if (!strcmp(pool->origin, "discard") || !strcmp(pool->origin, "waitroom")) {
            look_append_cards(&player->discard, remainder, n_remainder);
        } else {
            look_append_cards(&player->discard, remainder, n_remainder);
        }
pool->n = 0;
           look_clear_select();
           g_look_pool_select_offered = 0;
           return;
    }
    /* move/mod.rs:3120-3157 -- the SECOND deck-order producer, and the only
     * consumer of `placement_order: any_order` in the `look_and_select` arm.
     *
     * The FIRST producer is move_missing.c:rb_move_maybe_prompt_deck_order (the
     * `move_cards` action), reachable from 15 of 985 abilities. This one is the
     * `look_and_select` effect's own Finalize step, so it fires when the player
     * picks >= 2 of the looked-at cards AND the select action declares
     * `placement_order: any_order` AND the destination is a deck. Measured over
     * cards/abilities.json: EXACTLY 11 look_and_select nodes in the whole corpus
     * carry placement_order=any_order, and all 11 ALSO carry any_number:true, so
     * the select's `max_select` is `total_available` (look.rs:867-877) rather
     * than the `count.unwrap_or(1)` default, and a player CAN hand back 2 or 3
     * indices. 10 of the 11 declare destination deck_top and 1 declares
     * deck_bottom, so `is_deck_dest` holds for all 11. (The `count: 1` trap that
     * killed one node in the FIRST batch does not bite here, and that was checked
     * rather than assumed.)
     *
     * The predicate is Rust's `needs_order` verbatim: is_deck_dest &&
     * placement_order == AnyOrder && selected_cards.len() > 1. `placement_order`
     * is read off the select action first and the parked `current` effect second,
     * which is mod.rs:3035-3046's `select_action.placement_order_any().or_else(
     * || current.and_then(|c| c.placement_order_any()))`.
     *
     * What the arm does, in Rust's order (mod.rs:3127-3157): the looked-at pool
     * BECOMES the selected cards, the REMAINDER goes to discard (or deck_bottom
     * when discard_remaining is false), and a `SelectTarget{target: "order",
     * allow_skip: false}` prompt opens. `rb_order_prompt_begin` is the shared
     * body of both producers (choice.c, reached through
     * rb_move_handle_order_selection -> choice.rs:2984-3055's
     * clear_choice_state_and_resume), and `reverse_pool` is 0 here because Rust's
     * second producer hands the cards over in the order chosen, not reversed.
     *
     * NOTE, PRE-EXISTING, NOT ADDRESSED HERE: Rust retains `selected_cards`
     * through `card_matches_cost_limit_op` (mod.rs:3107-3116) BEFORE computing
     * remaining_cards and needs_order. This C function has never applied a
     * select-action cost_limit to `selected[]` at all (see :719-726, which gate on
     * `look_card_allowed` only), so `n_selected` here can over-count relative to
     * Rust when a select action declares cost_limit. None of the 11 nodes
     * declares one (MEASURED), so it cannot change a verdict on this corpus, and
     * fixing it would reshape the whole function, which is out of scope here.
     */
    {
        const AbilityEffect *ord_sel =
            (is_select && g_look_pool_select_offered) ? g_look_pending_select : NULL;
        const char *po = ord_sel ? look_extra(ord_sel, "placement_order") : NULL;
        if (!po && g->queue.resume_eff) po = look_extra(g->queue.resume_eff, "placement_order");
        int is_deck_dest = destination &&
                           (!strcmp(destination, "deck_top") ||
                            !strcmp(destination, "deck") ||
                            !strcmp(destination, "deck_bottom"));
        if (is_deck_dest && po && !strcmp(po, "any_order") && n_selected > 1) {
            /* mod.rs:3130-3137: the remainder goes to discard, or deck_bottom
             * when discard_remaining is false. `discard_remaining == -1` means the
             * key is absent, and Rust's `explicit_discard.unwrap_or(true)` makes
             * absent mean discard too, so the plain truthiness test is right. */
            const char *dest_zone = discard_remaining ? "discard" : "deck_bottom";
            for (int i = 0; i < n_remainder; i++) {
                if (!strcmp(dest_zone, "discard"))
                    look_append_cards(&player->discard, &remainder[i], 1);
                else
                    look_append_cards(&player->deck, &remainder[i], 1);
            }
look_clear_select();
           g_look_pool_select_offered = 0;
           rb_order_prompt_begin(g, owner, selected, n_selected, 0);
            return;
        }
    }

    const char *destination_zone = destination && *destination ? destination : "hand";
    if (!strcmp(destination_zone, "deck") || !strcmp(destination_zone, "deck_top")) {
        int ordered[MAX_LOOKED];
        for (int i = 0; i < n_selected; i++) ordered[i] = selected[n_selected - 1 - i];
        look_prepend_cards(&player->deck, ordered, n_selected);
    } else if (!strcmp(destination_zone, "deck_bottom")) {
        look_append_cards(&player->deck, selected, n_selected);
    } else if (!strcmp(destination_zone, "discard") || !strcmp(destination_zone, "waitroom")) {
        look_append_cards(&player->discard, selected, n_selected);
    } else if (!strcmp(destination_zone, "stage")) {
        look_place_stage(player, selected, n_selected);
    } else {
        look_append_cards(&player->hand, selected, n_selected);
    }
const char *remainder_zone = remainder_destination && *remainder_destination
                                     ? remainder_destination
                                     : (discard_remaining != 0 ? "discard" : "deck_top");
    const AbilityEffect *sel = (is_select && g_look_pool_select_offered) ? g_look_pending_select : NULL;
    /* move/mod.rs:3205-3230 — a select whose count is not satisfied yet
       RE-PRESENTS the leftover pool instead of flushing it. The chosen cards are
       already at their destination; only the unchosen looked-at cards survive and
       the next prompt offers min(max - taken, remaining). Without this the
       ability ended after a single pick, so 「最大3枚」 could never take 3. */
    if (is_select && n_selected > 0 && n_remainder > 0 &&
        look_select_wants_more(g, sel, n_selected, n_remainder)) {
        pool->n = 0;
        for (int i = 0; i < n_remainder; i++) look_append(pool, remainder[i]);
        int filtered[MAX_LOOKED];
        int n_filtered = 0;
        for (int i = 0; i < pool->n; i++)
            if (look_select_matches(g, sel, pool->cards[i])) filtered[n_filtered++] = i;
        int available = pool->n;
        int offer = look_select_max_count(sel, n_selected, n_remainder) - n_selected;
        if (offer > available) offer = available;
        if (offer <= 0) { pool->n = 0; return; }
        look_emit_pool_choice(g, owner, sel, filtered, n_filtered, offer, 1);
        g->queue.resume_after_look = g_look_reprompt_followup;
        return;
    }
    if (!strcmp(remainder_zone, "looked_at")) {
        pool->n = 0;
        for (int i = 0; i < n_remainder; i++) look_append(pool, remainder[i]);
        return;
    }
    if (!strcmp(remainder_zone, "deck") || !strcmp(remainder_zone, "deck_top")) {
        int ordered[MAX_LOOKED];
        for (int i = 0; i < n_remainder; i++) ordered[i] = remainder[n_remainder - 1 - i];
        look_prepend_cards(&player->deck, ordered, n_remainder);
    } else if (!strcmp(remainder_zone, "deck_bottom")) {
        look_append_cards(&player->deck, remainder, n_remainder);
    } else if (!strcmp(remainder_zone, "hand")) {
        look_append_cards(&player->hand, remainder, n_remainder);
    } else if (!strcmp(remainder_zone, "stage")) {
        look_place_stage(player, remainder, n_remainder);
    } else {
        look_append_cards(&player->discard, remainder, n_remainder);
    }
    if (is_select) {
        for (int i = 0; i < n_selected; i++) {
            if (g->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                g->selected_cards[g->n_selected_cards++] = selected[i];
            if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED)
                g->recently_moved[g->n_recently_moved++] = selected[i];
        }
    }
    for (int i = 0; i < n_remainder; i++)
        if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED)
            g->recently_moved[g->n_recently_moved++] = remainder[i];
pool->n = 0;
   look_clear_select();
   g_look_pool_select_offered = 0;
}

void rb_look_resume(GameState *g, int actor, int selected_idx, const char *destination, int is_select)
{
    int owner = g && g->queue.resume_look_owner >= 0 && g->queue.resume_look_owner < 2
                    ? g->queue.resume_look_owner
                    : (actor >= 0 && actor < 2 ? actor : 0);
    int discard_remaining = -1;
    const char *remainder = NULL;
    if (g && g->queue.resume_eff) {
        const char *value = look_extra(g->queue.resume_eff, "discard_remaining");
        if (value) discard_remaining = !strcmp(value, "true") || !strcmp(value, "1");
        remainder = look_extra(g->queue.resume_eff, "remainder_destination");
    }
    int index = selected_idx;
    rb_look_resume_indices(g, owner, &index, selected_idx < 0 ? 0 : 1,
                           destination, remainder, discard_remaining, is_select);
}

/* Reveal cards from the target's deck top into the looked_at pool until
   `pred` returns true (or the deck empties). Mirrors engine/src/ability/look.rs
   reveal_until — the matched card and everything revealed above it become the
   looked_at pool (gs.looked_at_cards = all_revealed). */
static int reveal_until(GameState *g, int who, int (*pred)(const GameState*, int cid))
{
    RbPlayer *player = &g->p[who];
    LookPool *pool = &g_look[who];
    look_pool_reset(pool, who, "deck_top");
    pool->from_deck = 1;
    int refreshed = 0;
    while (pool->n < MAX_LOOKED) {
        if (player->deck.n == 0) {
            if (refreshed || player->discard.n == 0) break;
            rb_player_refresh(g, who);
            refreshed = 1;
            if (player->deck.n == 0) break;
        }
        int cid = look_take_top(&player->deck);
        look_append(pool, cid);
        look_record_revealed(g, &cid, 1);
        if (pred(g, cid)) return 1;
    }
    return 0;
}

static int card_is_live_pred(const GameState *g, int cid){
    Card c; if(!rb_decode_card_by_index((uint32_t)cid,&c)) return 0;
    int r = (c.type_flags & 0x3) == 1; rb_free_card(&c); return r;
}

/* Adapter so card_matches_card_type_filter (int,int) can be used as a
   reveal_until predicate (const GameState*,int). */
static const char *g_reveal_ctype;
static int g_reveal_cost_limit = -1;
static const char *g_reveal_cost_op;
static int card_type_pred(const GameState *g, int cid){
    (void)g;
    return card_matches_card_type_filter(cid, g_reveal_ctype);
}

/* Mirror look.rs::execute_reveal_until_target — reveal from the deck until a card
    matching card_type (and, for member_card, cost_limit op) is found. The predicate
    layers the optional cost gate on top of the card_type gate. */
static int card_type_cost_pred(const GameState *g, int cid){
    (void)g;
    if (!g_reveal_ctype) return 0;
    if (!card_matches_card_type_filter(cid, g_reveal_ctype)) return 0;
    /* cost_limit only applies when the selected card_type is member_card. */
    if (g_reveal_cost_limit >= 0 && !strcmp(g_reveal_ctype, "member_card"))
        if (!rb_card_matches_cost_limit(cid, g_reveal_cost_limit, g_reveal_cost_op)) return 0;
    return 1;
}

void rb_effect_reveal_until_live_card(GameState *g, int actor, AbilityEffect *e)
{
    int who = look_target_player(g, actor, e);
    reveal_until(g, who, card_is_live_pred);
}

void rb_effect_reveal_until_chosen_card(GameState *g, int actor, AbilityEffect *e)
{
    int who = look_target_player(g, actor, e);
    /* vm.c:1098-1112 — `card_type` is stored in the STRUCT FIELD and the branch
       `continue`s, so it is NEVER in the extras table (MEASURED 0 of 1221
       effect nodes across 1074 abilities carry it as an extra). A bare
       look_extra(e,"card_type") is therefore permanently NULL. Rust reads the
       typed `effect.card_type_any()`; the field-first-with-fallback idiom is the
       shape draw.c:444 and look.c:976 already use. */
    const char *card_type = look_filter_card_type(e);
    g_reveal_ctype = card_type ? card_type : "live_card";
    reveal_until(g, who, card_type ? card_type_pred : card_is_live_pred);
}

/* Mirror look.rs::execute_reveal_until_target — reveal from the deck until a card
    matching card_type (and the member_card cost_limit gate) is found. On a match
    the matched card is moved to the FRONT of the looked_at pool; on no match the
    pool is cleared, mirroring Rust's matched_idx reordering. */
void rb_effect_reveal_until_target(GameState *g, int actor, AbilityEffect *e)
{
    int who = look_target_player(g, actor, e);
    /* Same defect as rb_effect_reveal_until_chosen_card above: `card_type` lives
       in the struct field, never in the extras table. */
    const char *card_type = look_filter_card_type(e);
    const char *cost_limit = look_extra(e, "cost_limit");
    const char *cost_operator = look_extra(e, "cost_limit_operator");
    g_reveal_ctype = card_type ? card_type : "live_card";
    g_reveal_cost_limit = cost_limit ? atoi(cost_limit) : -1;
    g_reveal_cost_op = cost_operator;
    LookPool *pool = &g_look[who];
    int matched = reveal_until(g, who, card_type_cost_pred);
    if (matched && pool->n > 1) {
        int matched_card = pool->cards[pool->n - 1];
        for (int i = pool->n - 1; i > 0; i--) pool->cards[i] = pool->cards[i - 1];
        pool->cards[0] = matched_card;
    } else if (!matched) {
        pool->n = 0;
    }
}

/* -- execute_reveal_per_group -- */
void rb_effect_reveal_per_group(GameState *g, int actor, AbilityEffect *e) {
    int who = look_target_player(g, actor, e);
    int count = rb_effect_count(g, who, -1, e, g->last_draw_count);
    if (count < 0) count = 0;
    RbPlayer *player = &g->p[who];
    const char *source = e->source && *e->source ? e->source : "hand";
    int card_ids[RB_MAX_ZONE];
    int n_cards = 0;
    if (!strcmp(source, "hand")) {
        for (int i = 0; i < player->hand.n && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = player->hand.cards[i];
    } else if (!strcmp(source, "deck") || !strcmp(source, "deck_top")) {
        int take = count < player->deck.n ? count : player->deck.n;
        for (int i = 0; i < take && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = player->deck.cards[i];
    } else if (!strcmp(source, "discard") || !strcmp(source, "waitroom")) {
        for (int i = 0; i < player->discard.n && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = player->discard.cards[i];
    } else if (!strcmp(source, "looked_at")) {
        LookPool *pool = &g_look[who];
        for (int i = 0; i < pool->n && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = pool->cards[i];
    }
    look_record_revealed(g, card_ids, n_cards);
}

/* -- execute_reveal -- */
void rb_effect_reveal(GameState *g, int actor, AbilityEffect *e) {
    int who = look_target_player(g, actor, e);
    int count = rb_effect_count(g, who, -1, e, g->last_draw_count);
    if (count < 0) count = 0;
    RbPlayer *player = &g->p[who];
    const char *source = e->source && *e->source ? e->source : "hand";
    const char *card_type = e->card_type_field[0] ? e->card_type_field
                                                  : look_extra(e, "card_type");
    int any_number = look_bool(e, "any_number");
    int is_max = look_bool(e, "max");
    int blind = look_bool(e, "blind");
    int is_optional = e->is_optional ? 1 : 0;
    int available = 0;
    if (!strcmp(source, "hand")) available = player->hand.n;
    else if (!strcmp(source, "looked_at")) available = g_look[who].n;
    else if (!strcmp(source, "deck") || !strcmp(source, "deck_top")) available = player->deck.n;
    if ((!strcmp(source, "hand") || !strcmp(source, "looked_at")) && available > 0 &&
        (is_max || is_optional || count == 0 || count < available)) {
        /* look.rs::offer_reveal_choice (look.rs:222-277) */
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, source, card_type,
                       any_number ? available : count,
                       any_number || is_optional || is_max, NULL);
    g->queue.resume_look_owner = who;
    rb_queue_pause_for_choice(g, &g->queue.pending);
        g->queue.pending.is_reveal = 1;
        g->queue.pending.blind = blind;
        snprintf(g->queue.pending.target_player_id,
                 sizeof(g->queue.pending.target_player_id), "p%d", who + 1);
        const char *cost_limit = look_extra(e, "cost_limit");
        if (cost_limit) {
            g->queue.pending.cost_limit = atoi(cost_limit);
            const char *op = look_extra(e, "cost_limit_operator");
            if (!op) op = look_extra(e, "operation");
            if (op) snprintf(g->queue.pending.cost_limit_op,
                             sizeof(g->queue.pending.cost_limit_op), "%s", op);
        }
        const char *group = look_extra(e, "group_names");
        if (group) snprintf(g->queue.pending.filter_group,
                            sizeof(g->queue.pending.filter_group), "%s", group);
        const char *characters = look_extra(e, "characters");
        if (characters) {
            const char *names[8];
            char buffer[256];
            g->queue.pending.n_characters =
                look_split_list(characters, names, 8, buffer, sizeof(buffer));
            for (int i = 0; i < g->queue.pending.n_characters && i < 16; i++)
                snprintf(g->queue.pending.characters[i],
                         sizeof(g->queue.pending.characters[i]), "%s", names[i]);
        }
        g->queue.resume_mode = 0;
        g->queue.resume_eff = e;
        g->queue.resume_is_select = 0;
        g->queue.resume_actor = actor;
        g->queue.resume_host = g->queue.resume_host >= 0 ? g->queue.resume_host : actor;
        return;
    }
    int card_ids[RB_MAX_ZONE];
    int n_cards = 0;
    if (!strcmp(source, "hand")) {
        for (int i = 0; i < player->hand.n && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = player->hand.cards[i];
    } else if (!strcmp(source, "deck") || !strcmp(source, "deck_top")) {
        /* Peek only — the cards stay on the deck for the conditional
           move_cards to consume (look.rs:356-362). */
        int take = count < player->deck.n ? count : player->deck.n;
        for (int i = 0; i < take && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = player->deck.cards[i];
    } else if (!strcmp(source, "looked_at")) {
        /* look.rs:364-370 — the looked_at arm runs the type/heart filter. */
        LookPool *pool = &g_look[who];
        for (int i = 0; i < pool->n && n_cards < RB_MAX_ZONE; i++)
            if (look_reveal_candidate_ok(e, pool->cards[i], card_type))
                card_ids[n_cards++] = pool->cards[i];
    }
    /* look.rs:375-379 — `blind` only decorates the prompt; the revealed pool
       is recorded either way. */
    look_record_revealed(g, card_ids, n_cards);
}

/* -- execute_select -- */
void rb_effect_select(GameState *g, int actor, AbilityEffect *e) {
    int who = actor;
    if (e->target && !strcmp(e->target, "opponent")) who = actor ^ 1;
    RbPlayer *P = &g->p[who];
    const char *source = e->source ? e->source : "hand";
    const char *card_type = e->card_type_field[0] ? e->card_type_field
                                                  : look_extra(e, "card_type");
    int count = e->count >= 0 ? e->count : 1;
    int is_optional = e->is_optional ? 1 : 0;
    int card_ids[RB_MAX_ZONE];
    int n_cards = 0;
    if (!strcmp(source, "hand")) {
        for (int i = 0; i < P->hand.n && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = P->hand.cards[i];
    } else if (!strcmp(source, "deck")) {
        int take = count < P->deck.n ? count : P->deck.n;
        for (int i = 0; i < take && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = P->deck.cards[P->deck.n - 1 - i];
    } else if (!strcmp(source, "discard") || !strcmp(source, "waitroom")) {
        for (int i = 0; i < P->discard.n && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = P->discard.cards[i];
    } else if (!strcmp(source, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] != RB_EMPTY_SLOT && n_cards < RB_MAX_ZONE)
                card_ids[n_cards++] = P->stage[i];
    } else if (!strcmp(source, "looked_at")) {
        LookPool *lp = &g_look[who];
        for (int i = 0; i < lp->n && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = lp->cards[i];
    } else if (!strcmp(source, "selected_cards")) {
        for (int i = 0; i < g->n_selected_cards && n_cards < RB_MAX_ZONE; i++)
            card_ids[n_cards++] = g->selected_cards[i];
    }
    /* look.rs:609-636 — the candidate list is narrowed by the card-type and
       heart-colour filter before it becomes the looked-at pool. */
    int filtered[RB_MAX_ZONE];
    int n_filtered = 0;
    for (int i = 0; i < n_cards; i++)
        if (look_reveal_candidate_ok(e, card_ids[i], card_type))
            filtered[n_filtered++] = card_ids[i];
    LookPool *lp = &g_look[who];
    lp->n = 0; lp->from_deck = 0; lp->owner = who;
    for (int i = 0; i < n_filtered && lp->n < MAX_LOOKED; i++)
        lp->cards[lp->n++] = filtered[i];
    /* look.rs::clamp_select_count (look.rs:471-483) — an empty pool, a zero
       count, or a distinct shortfall offers nothing at all (Q118). */
    if (count == 0 || lp->n == 0) return;
    if (e->distinct_flag && lp->n < count) return;
    if (count > lp->n) count = lp->n;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, source, NULL, count, is_optional, NULL);
    rb_queue_pause_for_choice(g, &g->queue.pending);
    g->queue.pending.n_filtered_indices = lp->n;
    for (int i = 0; i < lp->n; i++) g->queue.pending.filtered_indices[i] = i;
    /* Snapshot the SelectionContext so rb_look_resume re-validates the pick
       against this effect's own filter rather than whatever the previous
       choice left behind (a stale resume_filter_heart of 0 reads as
       "heart00 only" and rejects every card). */
    g->queue.pending.filter_group[0] = 0;
    g->queue.pending.filter_heart = -1;
    const char *grp = look_extra(e, "group_names");
    if (grp) snprintf(g->queue.pending.filter_group,
                      sizeof(g->queue.pending.filter_group), "%s", grp);
    strncpy(g->queue.resume_filter_group, g->queue.pending.filter_group,
            sizeof(g->queue.resume_filter_group)-1);
    g->queue.resume_filter_heart = g->queue.pending.filter_heart;
    g->queue.resume_mode = 2; g->queue.resume_eff = e;
    g->queue.resume_is_select = 1;
    look_park_select(e);
    g_look_pool_select_offered = 0;
    g->queue.resume_actor = actor; g->queue.resume_host = g->queue.resume_host >= 0 ? g->queue.resume_host : actor;
}

/* -- execute_look_and_select -- */
static const char *look_filter_card_type(const AbilityEffect *effect)
{
    if (!effect) return NULL;
    if (effect->card_type_field[0]) return effect->card_type_field;
    return look_extra(effect, "card_type");
}

/* ── CardFilter parity helpers (util.rs::CardFilter) ──────────────────────
   The looked-at pool narrowing used to re-implement only six of the filter
   fields. Everything below ports the remaining ones so a look-and-select
   narrows exactly like util.rs::CardFilter::matches. */

/* util.rs:1484-1499 check_group — `group` is group_names[0] and `groups` is the
   WHOLE list; a card passes when either matches. The wire frequently carries a
   comma list with the same group twice (「μ's,μ's'」), so handing the raw string
   to the single-value matcher rejected every card and the pool was discarded
   without ever offering a pick. */
static int look_group_list_matches(int cid, const char *group_list)
{
    const char *tokens[16];
    char buffer[512];
    int n = look_split_list(group_list, tokens, 16, buffer, sizeof(buffer));
    if (n <= 0) return 1;
    for (int i = 0; i < n; i++)
        if (rb_card_matches_group_str(cid, tokens[i])) return 1;
    return 0;
}

/* util.rs:1310-1321 any_trigger — the resolved trigger text STARTS WITH the
   wanted label (「常時」 is a prefix of 「常時:...」). */
static int look_ability_has_trigger(int cid, const char *wanted)
{
    size_t len = wanted ? strlen(wanted) : 0;
    if (!len) return 0;
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
int hit = ab.triggers && !strncmp(ab.triggers, wanted, len);
            rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

/* util.rs:1290-1318 card_passes_ability_filter. `require_ability` selects
   Unconstrained::RequireAbility (the or_ability_filters branch shape): a filter
   name with no trigger list then resolves to "has an ability at all". */
static int look_ability_filter_passes(int cid, const char *ability_filter,
                                      const char *const *triggers, int n_triggers,
                                      int require_ability)
{
    int has_ability = rb_card_num_abilities((uint32_t)cid) > 0;
    int unconstrained = require_ability ? has_ability : 1;
    const char *af = ability_filter ? ability_filter : "";
    if (!strcmp(af, "no_ability")) return !has_ability;
    if (!strcmp(af, "has_ability")) return has_ability;
    if (!strcmp(af, "no_ability_type")) {
        if (n_triggers > 0) {
            for (int i = 0; i < n_triggers; i++)
                if (look_ability_has_trigger(cid, triggers[i])) return 0;
            return 1;
        }
        return unconstrained;
    }
    if (!strcmp(af, "has_ability_type")) {
        if (n_triggers > 0) {
            for (int i = 0; i < n_triggers; i++)
                if (look_ability_has_trigger(cid, triggers[i])) return 1;
            return 0;
        }
        return unconstrained;
    }
    return 1;   /* an unrecognised name constrains nothing */
}

/* Read `"key":"value"` out of a JSON fragment. */
static const char *look_json_field(const char *json, const char *key)
{
    char pattern[64];
    snprintf(pattern, sizeof(pattern), "\"%s\"", key);
    const char *p = strstr(json, pattern);
    if (!p) return NULL;
    p += strlen(pattern);
    while (*p && *p != ':') {
        if (*p == ',') return NULL;
        p++;
    }
    if (*p != ':') return NULL;
    p++;
    while (*p == ' ' || *p == '"') p++;
    static char value[64];
    size_t n = 0;
    while (*p && *p != '"' && n + 1 < sizeof(value)) value[n++] = *p++;
    value[n] = 0;
    return n ? value : NULL;
}

/* Copy `src` into `dst`, dropping one surrounding pair of JSON quotes. The
   or_ability_filters trigger list is JSON, so its tokens arrive quoted while
   the plain extras (`ability_filter_triggers`, `group_names`) do not. */
static void look_copy_unquoted(char *dst, size_t cap, const char *src)
{
    size_t len = src ? strlen(src) : 0;
    if (len >= 2 && src[0] == '"' && src[len - 1] == '"') { src++; len -= 2; }
    if (len >= cap) len = cap - 1;
    char tmp[64];
    if (len > sizeof(tmp)) len = sizeof(tmp) - 1;
    memcpy(tmp, src, len);          /* src may BE dst — never memcpy onto itself */
    tmp[len] = 0;
    memcpy(dst, tmp, len + 1);
}

typedef struct {
    char ability_filter[32];
    const char *triggers[4];
    char trigger_store[4][48];
    int n_triggers;
} LookAbilityBranch;

/* util.rs:1530-1538 or_ability_filters — a JSON array of branches, each
   {"$variant":N,"$fields":{"ability_filter":"...","ability_filter_triggers":[...]}}.
   A card passes when ANY branch matches. */
static int look_parse_or_ability_filters(const char *json, LookAbilityBranch *out, int max)
{
    int n = 0;
    if (!json || !*json) return 0;
    const char *p = json;
    while (n < max) {
        const char *obj = strchr(p, '{');
        if (!obj) break;
        /* The fields all live inside the nested "$fields" object, so its
           closing brace is the end of every field we care about. */
        const char *end = strchr(obj, '}');
        if (!end) break;
        char body[768];
        size_t len = (size_t)(end - obj);
        if (len >= sizeof(body)) len = sizeof(body) - 1;
        memcpy(body, obj, len);
        body[len] = 0;
        const char *af = look_json_field(body, "ability_filter");
        if (af && *af) {
            LookAbilityBranch *b = &out[n];
            b->n_triggers = 0;
            snprintf(b->ability_filter, sizeof(b->ability_filter), "%s", af);
            const char *ta = strstr(body, "\"ability_filter_triggers\"");
            const char *lb = ta ? strchr(ta, '[') : NULL;
            const char *rb = lb ? strchr(lb, ']') : NULL;
            if (lb && rb && rb > lb + 1) {
                char list[256];
                size_t ll = (size_t)(rb - lb - 1);
                if (ll >= sizeof(list)) ll = sizeof(list) - 1;
                memcpy(list, lb + 1, ll);
                list[ll] = 0;
                const char *toks[4];
                b->n_triggers = look_split_list(list, toks, 4,
                                                b->trigger_store[0], sizeof(b->trigger_store[0]));
                if (b->n_triggers > 1) {
                    /* look_split_list tokenises one buffer; re-point the rest. */
                    char all[256];
                    snprintf(all, sizeof(all), "%s", list);
                    char *cursor = all;
                    for (int i = 0; i < b->n_triggers && i < 4; i++) {
                        char *comma = strchr(cursor, ',');
                        if (comma) *comma = 0;
                        look_copy_unquoted(b->trigger_store[i], sizeof(b->trigger_store[i]), cursor);
                        b->triggers[i] = b->trigger_store[i];
                        if (!comma) break;
                        cursor = comma + 1;
                    }
                } else if (b->n_triggers == 1) {
                    look_copy_unquoted(b->trigger_store[0], sizeof(b->trigger_store[0]), toks[0]);
                    b->triggers[0] = b->trigger_store[0];
                }
            }
            n++;
        }
        p = end + 1;
    }
    return n;
}

/* The looked-at pool narrowing. This used to be a WHOLE second implementation of
   CardFilter::matches walking the extras one key at a time. It now takes ONE
   builder — rb_effect_filter_subset (core/card.c, the twin of util.rs:1819-1884
   CardFilter::from_effect) — and hands it to ONE matcher, rb_matching_indices_
   filter (util.c card_filter_matches, the twin of util.rs:1676-1688
   CardFilter::matches). Everything the central pair still cannot carry is listed
   below with the Rust line that says so; none of it is a second copy of a term
   the builder or the matcher already owns.

   It is NOT deletable, for the reason the residuals spell out: `groups`,
   `exclude_group_names`, `or_ability_filters` and `card_property` have no
   RbCardFilter member and no term in the central matcher, so a central-only
   version of this function drops four of Rust's terms. */
static int look_filter_matches(const GameState *g, const AbilityEffect *effect, int cid)
{
    if (!effect) return 1;
    RbCardFilter f;
    rb_effect_filter_subset(effect, &f);

    /* util.rs:1821-1825 + check_group (util.rs:1484-1499) keep BOTH `group`
       (group_names.first()) and `groups` (the whole Vec); a card passes when it
       matches the first OR any entry of the list. RbCardFilter has no list
       member and util.c:1320-1324 keeps only the first, so a paired list like
       「μ's,μ's'」 — the shape that used to reject every card — would still be
       rejected centrally. The list is taken out of the filter and decided here.
       util.rs:1500-1506 exclude_group_names has no member at all and is counted
       by has_filter (util.rs:1429), so it is decided here too. */
    const char *group = look_extra(effect, "group_names");
    if (!group) group = look_extra(effect, "group");
    f.group[0] = '\0';
    f.has_group = 0;
    if (group && *group && !look_group_list_matches(cid, group)) return 0;
    const char *exclude_group = look_extra(effect, "exclude_group_names");
    if (exclude_group && *exclude_group && look_group_list_matches(cid, exclude_group)) return 0;

    /* util.rs:1836 heart_colors reaches CardFilter ungated; card.c:867-869 puts
       it behind `filter_targets_by_heart_colors`, which is the gate on
       filter_subset (card.rs:2415-2421), NOT on from_effect — so a
       look-and-select whose effect omits that flag would filter on nothing.
       The list is filled in here; `require_all_heart_colors` (util.rs:1837) and
       `heart_color_count` (util.rs:1838) come from the builder and are evaluated
       by the matcher (util.c:1371-1400), which is where the per-colour
       threshold twin of the deleted look_heart_count_for_threshold now lives. */
    const char *heart = look_extra(effect, "heart_colors");
    if (!heart) heart = look_extra(effect, "heart_color");
    f.n_heart_colors = 0;
    if (heart && *heart) {
        const char *colors[8];
        char buffer[192];
        int n_colors = look_split_list(heart, colors, 8, buffer, sizeof(buffer));
        for (int i = 0; i < n_colors && i < 8; i++)
            look_copy_unquoted(f.heart_colors[i], sizeof f.heart_colors[i], colors[i]);
        f.n_heart_colors = n_colors;
    }

    /* util.rs:1833 `characters` is a Vec<String> and matches() tests it with
       card_matches_characters (util.rs:1723-1731). RbCardFilter carries ONE
       comma-joined string and util.c:1356-1358 wraps it as a single name, so a
       multi-name card could never match it centrally. */
    f.has_characters = 0;
    const char *characters = look_extra(effect, "characters");
    if (characters && *characters) {
        const char *names[8];
        char buffer[256];
        int n_names = look_split_list(characters, names, 8, buffer, sizeof(buffer));
        if (n_names > 0 && !rb_card_matches_characters(cid, names, n_names)) return 0;
    }

    /* util.rs:1596-1601 any_trigger matches the trigger label with
       `starts_with`, not `contains`. util.c:1450-1452 uses strstr, so the
       central matcher would let 「常時:...」 satisfy a plain 「常時」 request
       (and vice versa). The prefix matcher stays. */
    f.ability_filter[0] = '\0';
    f.n_ability_filter_triggers = 0;
    /* util.rs:1291-1307 ability_filter, then util.rs:1631-1650 or_ability_filters
       (when branches are present they ARE the whole filter). or_ability_filters
       is a JSON array of branches (util.rs:1881) that RbCardFilter does not
       carry and the central matcher has no term for — util.rs:1443 counts it in
       has_filter — so it is still the only place in the engine that honours it. */
    const char *ability_filter = look_extra(effect, "ability_filter");
    if (ability_filter && *ability_filter) {
        const char *triggers[4];
        char trigger_store[4][48];
        int n_triggers = 0;
        const char *trig_list = look_extra(effect, "ability_filter_triggers");
        if (trig_list && *trig_list) {
            char buffer[256];
            const char *toks[4];
            n_triggers = look_split_list(trig_list, toks, 4, buffer, sizeof(buffer));
            if (n_triggers > 1) {
                char all[256];
                snprintf(all, sizeof(all), "%s", trig_list);
                char *cursor = all;
                for (int i = 0; i < n_triggers && i < 4; i++) {
                    char *comma = strchr(cursor, ',');
                    if (comma) *comma = 0;
                    snprintf(trigger_store[i], sizeof(trigger_store[i]), "%s", cursor);
                    triggers[i] = trigger_store[i];
                    if (!comma) break;
                    cursor = comma + 1;
                }
            }
        }
        if (!look_ability_filter_passes(cid, ability_filter, triggers, n_triggers, 0)) return 0;
    }
    LookAbilityBranch branches[8];
    int n_branches = look_parse_or_ability_filters(look_extra(effect, "or_ability_filters"),
                                                   branches, 8);
    if (n_branches > 0) {
        int passes = 0;
        for (int i = 0; i < n_branches; i++)
            if (look_ability_filter_passes(cid, branches[i].ability_filter,
                                           branches[i].triggers,
                                           branches[i].n_triggers, 1)) { passes = 1; break; }
        if (!passes) return 0;
    }

    /* util.rs:1654-1673 check_card_property — checked LAST, after the ability
       filter, and `negation` inverts it. card.c:908-911 populates the member and
       util.c:42-46 reads it, both behind RB_CARD_FILTER_CARD_PROPERTY, which no
       build defines; the test therefore still lives here. */
    const char *card_property = look_extra(effect, "card_property");
    if (card_property && *card_property) {
        const char *negation = look_extra(effect, "negation");
        int negate = negation && (!strcmp(negation, "true") || strtol(negation, NULL, 10) != 0);
        Card c;
        const Card *cp = rb_decode_card_by_index((uint32_t)cid, &c) ? &c : NULL;
        int passes = rb_check_card_property(card_property, negate, cp);
        if (cp) rb_free_card(&c);
        if (!passes) return 0;
    }

    /* util.rs:1315-1316 cost_total and util.rs:1327-1332 current_blade_limit are
       deliberately NOT matches() terms — Rust applies cost_total after
       selection and the blade bound via filter_current_blade, which needs the
       modifier table matches() has no access to. card.c:922-925 / :937 fill both
       regardless, so they are switched off here rather than silently narrowing
       every looked-at card.
       util.rs:1848-1852 pins exclude_self to Some(-1), so check_exclude_self
       (util.rs:1448-1456) rejects nothing at all; the old
       `cid == g->activating_card` compare here was a divergence, not a fix. */
    f.has_cost_total = 0;
    f.has_blade_limit = 0;
    (void)g;

    int one[1];
    int hit[1];
    one[0] = cid;
    return rb_matching_indices_filter(&f, one, 1, hit, 1) == 1;
}

static int look_select_matches(GameState *g, const AbilityEffect *select, int cid)
{
    if (!select) return 1;
    if (select->n_options > 0) {
        for (int i = 0; i < select->n_options; i++)
            if (look_filter_matches(g, select->options[i], cid)) return 1;
        return 0;
    }
    return look_filter_matches(g, select, cid);
}

static void look_discard_unmatched(GameState *g, int owner, AbilityEffect *parent)
{
    if (!g || owner < 0 || owner >= 2) return;
    LookPool *pool = &g_look[owner];
    RbPlayer *player = &g->p[owner];
    for (int i = 0; i < pool->n; i++)
        if (player->discard.n < RB_MAX_ZONE)
            player->discard.cards[player->discard.n++] = pool->cards[i];
    pool->n = 0;
    if (parent && parent->followup_action)
        rb_execute_effect_ex(g, owner, parent->followup_action,
                             g->queue.resume_host >= 0 ? g->queue.resume_host : -1);
}

void rb_effect_look_and_select(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    AbilityEffect *look_action = e->look_action;
    if (!look_action && e->n_child > 0) look_action = e->child[0];
    AbilityEffect *select_action = e->select_action;
    if (!select_action && e->n_child > 1) select_action = e->child[1];
    int who = look_target_player(g, actor, look_action ? look_action : e);
    if (look_action) {
        if (look_action->action && !strcmp(look_action->action, "look_at")) {
            look_at_fetch(g, actor, look_action);
            who = look_target_player(g, actor, look_action);
        } else {
            rb_execute_effect_ex(g, actor, look_action, g->activating_card);
            if (rb_has_pending_choice(g)) return;
            who = look_action->target && !strcmp(look_action->target, "opponent")
                      ? actor ^ 1
                      : actor;
        }
    }
    if (!select_action) return;
    LookPool *pool = &g_look[who];
    int filtered[MAX_LOOKED];
    int n_filtered = 0;
    for (int i = 0; i < pool->n; i++)
        if (look_select_matches(g, select_action, pool->cards[i]))
            filtered[n_filtered++] = i;
    if (n_filtered == 0) {
        look_discard_unmatched(g, who, e);
        return;
    }
    int count = select_action->count >= 0 ? select_action->count : 1;
    int any_number = look_bool(select_action, "any_number");
    int is_max = look_bool(select_action, "max");
    int is_optional = select_action->is_optional;
    int max_select = any_number ? n_filtered : (count < n_filtered ? count : n_filtered);
    const char *card_type = look_filter_card_type(select_action);
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "looked_at", card_type,
                   max_select, is_optional || is_max || any_number, NULL);
    g->queue.pending.n_filtered_indices = n_filtered;
    for (int i = 0; i < n_filtered; i++)
        g->queue.pending.filtered_indices[i] = filtered[i];
    const char *group = look_extra(select_action, "group_names");
    if (!group) group = look_extra(select_action, "group");
    g->queue.pending.filter_group[0] = 0;
    g->queue.resume_filter_group[0] = 0;
    if (group) {
        snprintf(g->queue.pending.filter_group, sizeof(g->queue.pending.filter_group), "%s", group);
        snprintf(g->queue.resume_filter_group, sizeof(g->queue.resume_filter_group), "%s", group);
    }
    const char *heart = look_extra(select_action, "heart_colors");
    if (!heart) heart = look_extra(select_action, "heart_color");
    /* A flat RbChoice can only carry ONE heart colour. `heart_colors` on the
       wire is a LIST ("heart02,heart04,heart05"), and rb_parse_heart_color on
       the joined string matches nothing and returns heart00 — which then read as
       "pink only" and rejected every pick. Only collapse a genuine single-token
       value; the multi-colour AND/OR gate already ran through
       look_select_matches and is carried by pending.filtered_indices. */
    int heart_color = -1;
    if (heart && *heart && !strchr(heart, ',')) heart_color = rb_parse_heart_color(heart);
    g->queue.pending.filter_heart = heart_color;
    g->queue.resume_filter_heart = heart_color;
    const char *cost_limit = look_extra(select_action, "cost_limit");
    if (cost_limit) {
        g->queue.pending.cost_limit = atoi(cost_limit);
        const char *operator_text = look_extra(select_action, "cost_limit_operator");
        if (!operator_text) operator_text = look_extra(select_action, "operation");
        if (operator_text)
            snprintf(g->queue.pending.cost_limit_op,
                     sizeof(g->queue.pending.cost_limit_op), "%s", operator_text);
    }
    g->queue.pending.is_reveal = look_bool(select_action, "reveal");
    snprintf(g->queue.pending.target_player_id,
             sizeof(g->queue.pending.target_player_id), "p%d", who + 1);
    g->queue.resume_look_owner = who;
    g->queue.resume_after_look = e->followup_action;
    g_look_reprompt_followup = e->followup_action;
    rb_queue_pause_for_choice(g, &g->queue.pending);
    g->queue.resume_mode = 2;
    g->queue.resume_eff = select_action;
    g->queue.resume_is_select = 1;
    look_park_select(select_action);
    g_look_pool_select_offered = 1;
    g->queue.resume_actor = actor;
    g->queue.resume_host = g->queue.resume_host >= 0 ? g->queue.resume_host : -1;
}