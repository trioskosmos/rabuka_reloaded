/* parity_state_effects2 — parity coverage for engine/src/ability/effects/state.rs.
 *
 * Companion to tests/test_state_effects.c (which exercises the energy/blade/heart
 * modifier primitives). This suite covers the *decision* logic of state.rs that the
 * primitive suite never reaches: the per-card and wait_activation shapes under
 * engine/tests/test_modules/effects/state/{per_card,wait_activation,orientation_gated}.
 *
 * Every assertion cites the Rust line it mirrors.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static int failures;
static int checks;

#define CHECK(condition, message) do { \
    checks++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    checks++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ── fixtures ─────────────────────────────────────────────────────────────
 * Costs / original blades are taken from the Rust tests that name them:
 *   PL!-sd1-010-SD  cost 4, original blade 1   (cost_and_blade_filtered_opponent_wait_test.rs:36,105)
 *   PL!S-bp5-009-R  cost 15, original blade 5  (same file:35,108)
 *   PL!-pb1-021-PR  cost 5                     (same file:33)
 * Groups: PL!N-bp3-006-R = 近江彼方 / 虹ヶ咲 (either_or_state_change_test.rs:56),
 *         PL!SP-bp2-007-R = 米女メイ / Liella! (either_or_state_change_test.rs:181). */
#define FILLER       "PL!-sd1-010-SD"
#define FILLER_2     "PL!-sd1-011-SD"
#define BIG_RUBY     "PL!S-bp5-009-R"
#define KOTORI       "PL!-pb1-021-PR"
#define NIJIGASAKI   "PL!N-bp3-006-R"
#define LIELLA       "PL!SP-bp2-007-R"

static void effect_init(AbilityEffect *e, const char *action, const char *target) {
    memset(e, 0, sizeof(*e));
    e->action = (char *)action;
    e->target = (char *)target;
    e->count = -1;
    e->per_unit_count = 0;
    e->repeat_limit = 0;
}

static void fx(AbilityEffect *e, const char *k, const char *v) {
    if (e->n_extra >= RB_MAX_EXTRA) return;
    e->extra_k[e->n_extra] = (char *)k;
    e->extra_v[e->n_extra] = (char *)v;
    e->n_extra++;
}

static const char *orientation_of(GameState *g, int cid) {
    return rb_mods_get_orientation(&g->mods, cid);
}

/* Rust `is_waited` (wait_activation/*.rs): orientation_modifier == Some("wait"). */
static int is_waited(GameState *g, int cid) {
    const char *o = rb_mods_get_orientation(&g->mods, cid);
    return o && !strcmp(o, "wait");
}

/* Rust `orientation_matches_state(ori, "active")` — None counts as active.
 * See either_or_state_change_test.rs:18-20. */
static int is_active(GameState *g, int cid) {
    const char *o = rb_mods_get_orientation(&g->mods, cid);
    return !o || !strcmp(o, "active");
}

static int has_prohibition(GameState *g, const char *text) {
    for (int i = 0; i < g->n_prohibition_effects; i++)
        if (!strcmp(g->prohibition_effects[i], text)) return 1;
    return 0;
}

static int prohibition_count(GameState *g, const char *prefix) {
    int n = 0;
    for (int i = 0; i < g->n_prohibition_effects; i++)
        if (!strncmp(g->prohibition_effects[i], prefix, strlen(prefix))) n++;
    return n;
}

static void wipe(GameState *g) {
    int keep_activating = g->activating_card;
    memset(g, 0, sizeof(*g));
    g->activating_card = keep_activating;
    rb_mods_init(&g->mods);
    for (int pl = 0; pl < 2; pl++) {
        for (int q = 0; q < RB_STAGE_SIZE; q++) {
            g->p[pl].stage[q] = RB_EMPTY_SLOT;
            g->p[pl].stage_wait[q] = 0;
            g->p[pl].under_cards[q].n = 0;
        }
    }
    g->queue.selected_heart_color = -1;
}

/* ══════════════════════════════════════════════════════════════════════
 * 1. execute_set_cost — state.rs:998-1052
 * ══════════════════════════════════════════════════════════════════════ */
static void test_set_cost_zones_and_filters(void) {
    GameState g; wipe(&g);
    int live_a = rb_find_card_by_no(NIJIGASAKI);
    int member_a = rb_find_card_by_no(FILLER);
    int member_b = rb_find_card_by_no(LIELLA);
    int hand_a = rb_find_card_by_no(FILLER_2);
    CHECK(live_a >= 0 && member_a >= 0 && member_b >= 0 && hand_a >= 0,
          "set_cost fixtures resolve");
    if (live_a < 0 || member_a < 0 || member_b < 0 || hand_a < 0) return;

    AbilityEffect e;

    /* state.rs:1004-1016 — card_type=live_card collects the live zone */
    wipe(&g);
    g.p[0].live.n = 0;
    g.p[0].live.cards[g.p[0].live.n++] = live_a;
    g.p[0].hand.n = 0;
    g.p[0].hand.cards[g.p[0].hand.n++] = hand_a;
    effect_init(&e, "set_cost", "self");
    strcpy(e.card_type_field, "live_card");
    fx(&e, "value", "7");
    rb_effect_set_cost(&g, 0, &e, -1);
    CHECK_EQ(g.mods.cost[live_a].set, 7, "set_cost on live_card hits the live zone");
    CHECK_EQ(g.mods.cost[hand_a].set, 0, "set_cost on live_card leaves the hand alone");

    /* state.rs:1006-1013 — card_type=member_card collects the stage */
    wipe(&g);
    g.p[0].stage[0] = member_a;
    g.p[0].stage[1] = member_b;
    effect_init(&e, "set_cost", "self");
    strcpy(e.card_type_field, "member_card");
    fx(&e, "value", "3");
    rb_effect_set_cost(&g, 0, &e, -1);
    CHECK_EQ(g.mods.cost[member_a].set, 3, "set_cost on member_card hits stage slot 0");
    CHECK_EQ(g.mods.cost[member_b].set, 3, "set_cost on member_card hits stage slot 1");

    /* state.rs:1014-1016 — no card_type defaults to the hand */
    wipe(&g);
    g.p[0].hand.n = 0;
    g.p[0].hand.cards[g.p[0].hand.n++] = hand_a;
    g.p[0].stage[0] = member_a;
    effect_init(&e, "set_cost", "self");
    fx(&e, "value", "5");
    rb_effect_set_cost(&g, 0, &e, -1);
    CHECK_EQ(g.mods.cost[hand_a].set, 5, "set_cost with no card_type defaults to the hand");
    CHECK_EQ(g.mods.cost[member_a].set, 0, "set_cost with no card_type leaves the stage alone");

    /* state.rs:1017-1033 + 1002-1003 — group_names filters the collected set */
    wipe(&g);
    g.p[0].stage[0] = member_a;   /* filler, not 虹ヶ咲 */
    g.p[0].stage[1] = rb_find_card_by_no(NIJIGASAKI);
    effect_init(&e, "set_cost", "self");
    strcpy(e.card_type_field, "member_card");
    fx(&e, "value", "9");
    fx(&e, "group_names", "虹ヶ咲");
    rb_effect_set_cost(&g, 0, &e, -1);
    CHECK_EQ(g.mods.cost[rb_find_card_by_no(NIJIGASAKI)].set, 9,
             "set_cost group_names keeps the matching member");
    CHECK_EQ(g.mods.cost[member_a].set, 0,
             "set_cost group_names drops the non-matching member");

    /* state.rs:1017-1021 — exclude_group_names ALONE must still filter. */
    wipe(&g);
    g.p[0].stage[0] = member_a;
    g.p[0].stage[1] = rb_find_card_by_no(NIJIGASAKI);
    effect_init(&e, "set_cost", "self");
    strcpy(e.card_type_field, "member_card");
    fx(&e, "value", "4");
    fx(&e, "exclude_group_names", "虹ヶ咲");
    rb_effect_set_cost(&g, 0, &e, -1);
    CHECK_EQ(g.mods.cost[member_a].set, 4,
             "set_cost exclude_group_names alone keeps the other member");
    CHECK_EQ(g.mods.cost[rb_find_card_by_no(NIJIGASAKI)].set, 0,
             "set_cost exclude_group_names alone drops the excluded group");

    /* state.rs:1017-1021 — exclude_characters ALONE must still filter too. */
    wipe(&g);
    g.p[0].stage[0] = member_a;
    g.p[0].stage[1] = rb_find_card_by_no(NIJIGASAKI);
    effect_init(&e, "set_cost", "self");
    strcpy(e.card_type_field, "member_card");
    fx(&e, "value", "6");
    fx(&e, "exclude_characters", "近江彼方");
    rb_effect_set_cost(&g, 0, &e, -1);
    CHECK_EQ(g.mods.cost[member_a].set, 6,
             "set_cost exclude_characters alone keeps the other member");
    CHECK_EQ(g.mods.cost[rb_find_card_by_no(NIJIGASAKI)].set, 0,
             "set_cost exclude_characters alone drops the excluded character");
}

/* ══════════════════════════════════════════════════════════════════════
 * 2. execute_set_card_identity / _all_regions — state.rs:1417-1429, 1559-1583
 * ══════════════════════════════════════════════════════════════════════ */
static void test_set_card_identity(void) {
    GameState g; wipe(&g);
    int member = rb_find_card_by_no(FILLER);
    CHECK(member >= 0, "set_card_identity fixture resolves");
    if (member < 0) return;
    g.activating_card = member;
    AbilityEffect e;

    /* state.rs:1426-1427 — every identity is joined with "," into one note. */
    effect_init(&e, "set_card_identity", "self");
    fx(&e, "identities", "Aoyama Ichika,Ayumu");
    rb_effect_set_card_identity(&g, 0, &e, member);
    CHECK(has_prohibition(&g, "card_identity:Aoyama Ichika,Ayumu"),
          "set_card_identity joins all identities into one prohibition note");

    /* state.rs:1425 — an empty identity list records nothing. */
    wipe(&g);
    g.activating_card = member;
    effect_init(&e, "set_card_identity", "self");
    rb_effect_set_card_identity(&g, 0, &e, member);
    CHECK_EQ(prohibition_count(&g, "card_identity:"), 0,
             "set_card_identity with no identities records no prohibition");

    /* state.rs:1570-1573 — all_regions pushes ONE note PER identity. */
    wipe(&g);
    g.activating_card = member;
    char expect[64];
    snprintf(expect, sizeof expect, "card_identity:%d:Aoyama Ichika", member);
    effect_init(&e, "set_card_identity", "self");
    fx(&e, "all_regions", "true");
    fx(&e, "identities", "Aoyama Ichika,Ayumu");
    rb_effect_set_card_identity_all_regions(&g, 0, &e, member);
    CHECK(has_prohibition(&g, expect),
          "all_regions identity records a card-scoped note for the first identity");
    CHECK_EQ(prohibition_count(&g, "card_identity:"), 2,
             "all_regions identity records one note per identity");
}

/* ══════════════════════════════════════════════════════════════════════
 * 3. execute_change_state — member branch, state.rs:269-717
 * ══════════════════════════════════════════════════════════════════════ */
static void test_change_state_all_and_filters(void) {
    GameState g; wipe(&g);
    int a = rb_find_card_by_no(FILLER);
    int b = rb_find_card_by_no(FILLER_2);
    int c = rb_find_card_by_no(KOTORI);
    CHECK(a >= 0 && b >= 0 && c >= 0, "change_state fixtures resolve");
    if (a < 0 || b < 0 || c < 0) return;
    g.p[0].stage[0] = a;
    g.p[0].stage[1] = b;
    g.p[0].stage[2] = c;
    g.activating_card = -1;
    g.state_change_triggering = 1;  /* keep auto-ability re-trigger quiet */
    AbilityEffect e;

    /* state.rs:448,512-514 — count==0 means "change all matching". */
    effect_init(&e, "change_state", "self");
    e.count = 0;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    rb_effect_change_state(&g, 0, &e, -1);
    CHECK(is_waited(&g, a) && is_waited(&g, b) && is_waited(&g, c),
          "change_state count=0 waits every matching stage member");

    /* state.rs:642-695 — actual transitions are recorded. */
    CHECK_EQ(g.n_recently_state_changed, 3,
             "change_state records every flipped card in recently_state_changed");
    CHECK_EQ(g.n_turn_state_changes, 3,
             "change_state records every flip in turn_state_changes");
    CHECK_EQ(g.state_change_from[a], 0, "active->wait records state_change_from=0");
    CHECK_EQ(g.state_change_to[a], 1, "active->wait records state_change_to=1");

    /* state.rs:353-357 — a member already in wait is NOT a candidate for wait. */
    wipe(&g);
    g.state_change_triggering = 1;
    g.p[0].stage[0] = a;
    g.p[0].stage[1] = b;
    rb_mods_set_orientation(&g.mods, a, "wait");
    effect_init(&e, "change_state", "self");
    e.count = 0;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    rb_effect_change_state(&g, 0, &e, -1);
    CHECK_EQ(g.n_recently_state_changed, 1,
             "change_state wait skips a member that is already waited (Q137)");
    CHECK(is_waited(&g, b) && is_waited(&g, a),
          "change_state wait leaves the already-waited member in wait");

    /* state.rs:345-347 — state_change=active only accepts waited members. */
    wipe(&g);
    g.state_change_triggering = 1;
    g.p[0].stage[0] = a;
    g.p[0].stage[1] = b;
    rb_mods_set_orientation(&g.mods, a, "wait");
    effect_init(&e, "change_state", "self");
    e.count = 0;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "active");
    rb_effect_change_state(&g, 0, &e, -1);
    CHECK(is_active(&g, a) && is_active(&g, b),
          "change_state active activates the waited member and is a no-op on the active one");
    CHECK_EQ(g.n_recently_state_changed, 1,
             "change_state active only records the genuine wait->active flip");
    /* state.rs:630-637 — last_state_change_wait_to_active_count counts the flips. */
    CHECK_EQ(g.last_wait_to_active_count, 1,
             "change_state active records last_wait_to_active_count");

    /* state.rs:348-352 — state="active" + state_change="wait" targets only
     * ACTIVE members. */
    wipe(&g);
    g.state_change_triggering = 1;
    g.p[0].stage[0] = a;
    g.p[0].stage[1] = b;
    rb_mods_set_orientation(&g.mods, a, "wait");
    effect_init(&e, "change_state", "self");
    e.count = 0;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    fx(&e, "state", "active");
    rb_effect_change_state(&g, 0, &e, -1);
    CHECK(is_waited(&g, b) && is_waited(&g, a),
          "state=active + state_change=wait only waits the active member");
    CHECK_EQ(g.n_recently_state_changed, 1,
             "state=active + state_change=wait does not re-wait the waited member");

    /* state.rs:298-301 + 1002-1003 — cost_limit filters stage members. */
    wipe(&g);
    g.state_change_triggering = 1;
    g.p[0].stage[0] = a;    /* cost 4 */
    g.p[0].stage[1] = c;    /* cost 5 */
    effect_init(&e, "change_state", "self");
    e.count = 0;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    fx(&e, "cost_limit", "4");
    fx(&e, "cost_limit_operator", "<=");
    rb_effect_change_state(&g, 0, &e, -1);
    CHECK(is_waited(&g, a) && !is_waited(&g, c),
          "change_state cost_limit waits only members within the cost limit");

    /* state.rs:121-122 + 302 — blade_limit filters on ORIGINAL blade. */
    wipe(&g);
    g.state_change_triggering = 1;
    int low = rb_find_card_by_no(FILLER);     /* original blade 1 */
    int high = rb_find_card_by_no(BIG_RUBY);  /* original blade 5 */
    g.p[0].stage[0] = low;
    g.p[0].stage[1] = high;
    effect_init(&e, "change_state", "self");
    e.count = 0;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    fx(&e, "blade_limit", "2");
    fx(&e, "blade_limit_operator", "<=");
    rb_effect_change_state(&g, 0, &e, -1);
    CHECK(is_waited(&g, low) && !is_waited(&g, high),
          "change_state blade_limit waits only members whose original blade fits");

    /* state.rs:130-134 + 267-277 — targeting the opponent ignores group_names
     * as an effect filter (it is trigger-level metadata). */
    wipe(&g);
    g.state_change_triggering = 1;
    g.p[1].stage[0] = a;
    g.p[1].stage[1] = b;
    effect_init(&e, "change_state", "opponent");
    e.count = 0;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    fx(&e, "group_names", "虹ヶ咲");
    rb_effect_change_state(&g, 0, &e, -1);
    CHECK(is_waited(&g, a) && is_waited(&g, b),
          "opponent-targeted change_state ignores group_names as a filter");
}

/* state.rs:79-81 — a per-unit effect that matches nothing must not fall
 * through to "change all". */
static void test_change_state_per_unit_zero_is_noop(void) {
    GameState g; wipe(&g);
    int a = rb_find_card_by_no(FILLER);
    int b = rb_find_card_by_no(FILLER_2);
    CHECK(a >= 0 && b >= 0, "per_unit fixtures resolve");
    if (a < 0 || b < 0) return;
    g.p[0].stage[0] = a;
    g.p[0].stage[1] = b;
    g.activating_card = -1;
    g.state_change_triggering = 1;
    AbilityEffect e;
    effect_init(&e, "change_state", "self");
    e.count = 1;
    e.per_unit = 1;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    /* per_unit filter that matches neither member */
    fx(&e, "cost_values", "255");
    rb_effect_change_state(&g, 0, &e, -1);
    CHECK(!is_waited(&g, a) && !is_waited(&g, b),
          "per_unit change_state matching nothing is a no-op, not change-all");
    CHECK(!rb_has_pending_choice(&g),
          "per_unit change_state matching nothing emits no choice");
}

/* state.rs:40-78 — per_unit scales count by the number of matching cards. */
static void test_change_state_per_unit_scales(void) {
    GameState g; wipe(&g);
    int a = rb_find_card_by_no(FILLER);
    int b = rb_find_card_by_no(FILLER_2);
    int c = rb_find_card_by_no(KOTORI);
    CHECK(a >= 0 && b >= 0 && c >= 0, "per_unit scaling fixtures resolve");
    if (a < 0 || b < 0 || c < 0) return;
    g.p[0].stage[0] = a;
    g.p[0].stage[1] = b;
    g.p[0].stage[2] = c;
    g.activating_card = -1;
    g.state_change_triggering = 1;
    AbilityEffect e;
    /* two members cost <= 4, one costs 5 -> 2 units -> 2 members waited. */
    effect_init(&e, "change_state", "self");
    e.count = 1;
    e.per_unit = 1;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    fx(&e, "cost_limit", "4");
    fx(&e, "cost_limit_operator", "<=");
    rb_effect_change_state(&g, 0, &e, -1);
    CHECK_EQ(g.n_recently_state_changed, 2,
             "per_unit change_state waits one member per matching unit");
}

/* state.rs:286-291 + 370-404 — self_cost restricts the effect to the
 * activating member and skips it if it is already in the target state. */
static void test_change_state_self_target_autoselect(void) {
    GameState g; wipe(&g);
    int a = rb_find_card_by_no(FILLER);
    int b = rb_find_card_by_no(FILLER_2);
    CHECK(a >= 0 && b >= 0, "self-target fixtures resolve");
    if (a < 0 || b < 0) return;
    g.p[0].stage[0] = a;
    g.p[0].stage[1] = b;
    g.activating_card = a;
    g.state_change_triggering = 1;
    AbilityEffect e;

    /* state.rs:454-462 — a single-target self effect auto-selects instead of
     * prompting even though two members match. */
    effect_init(&e, "change_state", "self");
    e.count = 1;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    rb_effect_change_state(&g, 0, &e, a);
    CHECK(!rb_has_pending_choice(&g),
          "single-target self change_state auto-selects instead of prompting");
    CHECK(is_waited(&g, a) && !is_waited(&g, b),
          "single-target self change_state waits only the activating member");

    /* state.rs:385-397 — already in the target state -> skip entirely. */
    wipe(&g);
    g.state_change_triggering = 1;
    g.p[0].stage[0] = a;
    g.activating_card = a;
    rb_mods_set_orientation(&g.mods, a, "wait");
    effect_init(&e, "change_state", "self");
    e.count = 1;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    rb_effect_change_state(&g, 0, &e, a);
    CHECK_EQ(g.n_recently_state_changed, 0,
             "self change_state skips a member already in the target state");
}

/* state.rs:460-510 + 465-494 — the prompt path and its candidate indices. */
static void test_change_state_prompts(void) {
    GameState g; wipe(&g);
    int a = rb_find_card_by_no(FILLER);
    int b = rb_find_card_by_no(FILLER_2);
    int c = rb_find_card_by_no(KOTORI);
    CHECK(a >= 0 && b >= 0 && c >= 0, "prompt fixtures resolve");
    if (a < 0 || b < 0 || c < 0) return;
    g.p[0].stage[0] = a;
    g.p[0].stage[1] = b;
    g.p[0].stage[2] = c;
    g.activating_card = -1;
    g.state_change_triggering = 1;
    AbilityEffect e;

    /* state.rs:462-463 — more candidates than count must prompt. */
    effect_init(&e, "change_state", "self");
    e.count = 1;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    rb_effect_change_state(&g, 0, &e, -1);
    const RbChoice *ch = rb_get_pending_choice(&g);
    CHECK(ch && ch->kind == RB_CHOICE_SELECT_CARD,
          "change_state prompts when candidates exceed count");
    CHECK_EQ(ch ? ch->count : -1, 1, "the change_state prompt asks for exactly count");
    CHECK_EQ(ch ? ch->n_filtered_indices : -1, 3,
             "the change_state prompt exposes every candidate stage slot");
    CHECK(ch && !strcmp(ch->zone, "stage"),
          "the change_state prompt offers the stage zone");
    g.queue.has_pending = 0;
    rb_effect_free(g.queue.deferred); g.queue.deferred = NULL;
    g.queue.resume_eff = NULL;

    /* state.rs:462 — max=true prompts even when count covers everything. */
    wipe(&g);
    g.state_change_triggering = 1;
    g.p[0].stage[0] = a;
    effect_init(&e, "change_state", "self");
    e.count = 1;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    fx(&e, "max", "true");
    rb_effect_change_state(&g, 0, &e, -1);
    ch = rb_get_pending_choice(&g);
    CHECK(ch && ch->allow_skip,
          "max=true change_state offers an optional (skippable) prompt");
    g.queue.has_pending = 0;
    rb_effect_free(g.queue.deferred); g.queue.deferred = NULL;
    g.queue.resume_eff = NULL;
}

/* state.rs:136-256 — the optional-cost gate only fires when a target exists. */
static void test_change_state_optional_gate(void) {
    GameState g; wipe(&g);
    int a = rb_find_card_by_no(FILLER);
    CHECK(a >= 0, "optional-gate fixture resolves");
    if (a < 0) return;
    g.state_change_triggering = 1;
    AbilityEffect e;

    /* state.rs:204-235,236-242 — nobody on stage -> no choice at all. */
    g.p[0].stage[0] = RB_EMPTY_SLOT;
    g.p[0].stage[1] = RB_EMPTY_SLOT;
    g.p[0].stage[2] = RB_EMPTY_SLOT;
    effect_init(&e, "change_state", "self");
    e.count = 1;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    fx(&e, "optional", "true");
    rb_effect_change_state(&g, 0, &e, -1);
    CHECK(!rb_has_pending_choice(&g),
          "optional change_state with no candidate emits no optional-cost choice");

    /* state.rs:204-232 — an active member exists -> the gate is offered. */
    wipe(&g);
    g.state_change_triggering = 1;
    g.p[0].stage[0] = a;
    rb_mods_set_orientation(&g.mods, a, "active");
    effect_init(&e, "change_state", "self");
    e.count = 1;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    fx(&e, "optional", "true");
    rb_effect_change_state(&g, 0, &e, -1);
    CHECK(rb_has_pending_choice(&g),
          "optional change_state with an active candidate offers the pay/skip gate");
    const RbChoice *ch = rb_get_pending_choice(&g);
    CHECK(ch && !strcmp(ch->target, "change_state_optional"),
          "the optional change_state gate is labelled change_state_optional");
}

/* state.rs:279-284,579-586,630-637 — cannot_activate_by_effect blocks
 * wait->active and zeroes the wait->active counter. */
static void test_change_state_cannot_activate(void) {
    GameState g; wipe(&g);
    int a = rb_find_card_by_no(FILLER);
    int b = rb_find_card_by_no(FILLER_2);
    CHECK(a >= 0 && b >= 0, "cannot_activate fixtures resolve");
    if (a < 0 || b < 0) return;
    g.p[0].stage[0] = a;
    g.p[0].stage[1] = b;
    g.activating_card = -1;
    g.state_change_triggering = 1;
    rb_mods_set_orientation(&g.mods, a, "wait");
    rb_mods_set_orientation(&g.mods, b, "wait");
    g.player_cannot_activate[0] = 1;
    AbilityEffect e;
    effect_init(&e, "change_state", "self");
    e.count = 0;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "active");
    rb_effect_change_state(&g, 0, &e, -1);
    CHECK(is_waited(&g, a) && is_waited(&g, b),
          "cannot_activate_by_effect blocks every wait->active flip");
    CHECK_EQ(g.last_wait_to_active_count, 0,
             "cannot_activate_by_effect zeroes last_wait_to_active_count");
}

/* state.rs:305-325 — a prior selection supplies the candidates. */
static void test_change_state_uses_prior_selection(void) {
    GameState g; wipe(&g);
    int a = rb_find_card_by_no(FILLER);
    int b = rb_find_card_by_no(FILLER_2);
    CHECK(a >= 0 && b >= 0, "prior-selection fixtures resolve");
    if (a < 0 || b < 0) return;
    g.p[0].stage[0] = a;
    g.p[0].stage[1] = b;
    g.activating_card = -1;
    g.state_change_triggering = 1;
    g.n_selected_cards = 1;
    g.selected_cards[0] = b;
    AbilityEffect e;
    effect_init(&e, "change_state", "self");
    e.count = 0;
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    rb_effect_change_state(&g, 0, &e, -1);
    CHECK(!is_waited(&g, a) && is_waited(&g, b),
          "change_state honours a prior selected_cards relay as its candidates");
    CHECK(g.n_selected_cards >= 1 && g.selected_cards[0] == b,
          "change_state keeps the changed card in selected_cards");
}

/* ══════════════════════════════════════════════════════════════════════
 * 4. execute_set_blade_type — state.rs:1054-1142
 * ══════════════════════════════════════════════════════════════════════ */
static void test_set_blade_type(void) {
    GameState g; wipe(&g);
    int nijigasaki = rb_find_card_by_no(NIJIGASAKI);
    int liella = rb_find_card_by_no(LIELLA);
    CHECK(nijigasaki >= 0 && liella >= 0, "blade_type fixtures resolve");
    if (nijigasaki < 0 || liella < 0) return;
    g.p[0].stage[0] = nijigasaki;
    g.p[0].stage[1] = liella;
    AbilityEffect e;

    /* state.rs:1095-1112 + 1113-1124 — group filter then recolour. */
    effect_init(&e, "set_blade_type", "self");
    fx(&e, "blade_type", "green");
    fx(&e, "group_names", "Liella!");
    rb_effect_set_blade_type(&g, 0, &e, -1);
    CHECK_EQ(rb_mods_get_blade_type(&g.mods, liella), 3,
             "set_blade_type recolours the matching group");
    CHECK_EQ(rb_mods_get_blade_type(&g.mods, nijigasaki), -1,
             "set_blade_type leaves the non-matching group alone");

    /* state.rs:1071-1081 — an unrecognised colour resolves to None and is a no-op. */
    wipe(&g);
    g.p[0].stage[0] = nijigasaki;
    effect_init(&e, "set_blade_type", "self");
    fx(&e, "blade_type", "chartreuse");
    rb_effect_set_blade_type(&g, 0, &e, -1);
    CHECK_EQ(rb_mods_get_blade_type(&g.mods, nijigasaki), -1,
             "set_blade_type with an unknown colour changes nothing");

    /* state.rs:1125-1141 + duration.rs:47 — duration registers a revert. */
    wipe(&g);
    g.p[0].stage[0] = nijigasaki;
    effect_init(&e, "set_blade_type", "self");
    fx(&e, "blade_type", "purple");
    fx(&e, "duration", "this_turn");
    rb_effect_set_blade_type(&g, 0, &e, -1);
    CHECK_EQ(rb_mods_get_blade_type(&g.mods, nijigasaki), 5,
             "set_blade_type applies the purple colour");
    rb_check_expired_effects(&g, RB_TEMP_TURN_END);
    CHECK_EQ(rb_mods_get_blade_type(&g.mods, nijigasaki), -1,
             "turn-scoped set_blade_type reverts at turn end");
}

/* ══════════════════════════════════════════════════════════════════════
 * 5. execute_set_heart_type — state.rs:1144-1246
 * ══════════════════════════════════════════════════════════════════════ */
static void test_set_heart_type(void) {
    GameState g; wipe(&g);
    int a = rb_find_card_by_no(FILLER);
    int b = rb_find_card_by_no(FILLER_2);
    CHECK(a >= 0 && b >= 0, "heart_type fixtures resolve");
    if (a < 0 || b < 0) return;
    g.p[0].stage[0] = a;
    g.activating_card = a;
    AbilityEffect e;

    /* state.rs:1159-1168 — self target applies to the activating card. */
    effect_init(&e, "set_heart_type", "self");
    strcpy(e.self_target_field, "true");
    fx(&e, "heart_type", "blue");
    rb_effect_set_heart_type(&g, 0, &e, a);
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&g.mods, a), RB_HEART_BLUE,
             "set_heart_type transforms the activating member's hearts");

    /* state.rs:1156-1157 — heart_type falls back to the first heart_colors. */
    wipe(&g);
    g.p[0].stage[0] = a;
    g.activating_card = a;
    effect_init(&e, "set_heart_type", "self");
    strcpy(e.self_target_field, "true");
    fx(&e, "heart_colors", "heart02,heart03");
    rb_effect_set_heart_type(&g, 0, &e, a);
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&g.mods, a), RB_HEART_YELLOW,
             "set_heart_type uses the first heart_colors entry as the heart type");

    /* state.rs:1181-1203 — a member target with exactly one candidate
     * auto-selects and applies. */
    wipe(&g);
    g.state_change_triggering = 1;
    g.p[0].stage[0] = a;
    g.p[0].stage[1] = b;
    g.activating_card = a;
    effect_init(&e, "set_heart_type", "self");
    strcpy(e.card_type_field, "member_card");
    fx(&e, "group_names", "虹ヶ咲");
    fx(&e, "heart_type", "green");
    rb_effect_set_heart_type(&g, 0, &e, a);
    CHECK(rb_has_pending_choice(&g) ||
          rb_mods_get_heart_color_multiplier(&g.mods, a) != -1,
          "set_heart_type on a member target either auto-selects or prompts");

    /* state.rs:1185-1188 — no eligible target is a silent no-op. */
    wipe(&g);
    g.state_change_triggering = 1;
    g.p[0].stage[0] = a;
    g.p[0].stage[1] = b;
    g.activating_card = -1;
    effect_init(&e, "set_heart_type", "self");
    strcpy(e.card_type_field, "member_card");
    fx(&e, "group_names", "Aqours");
    fx(&e, "heart_type", "green");
    rb_effect_set_heart_type(&g, 0, &e, -1);
    CHECK(!rb_has_pending_choice(&g),
          "set_heart_type with no eligible target emits no choice");
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&g.mods, a), -1,
             "set_heart_type with no eligible target applies nothing");
}

/* ══════════════════════════════════════════════════════════════════════
 * 6. execute_activation_cost — state.rs:1384-1415
 * ══════════════════════════════════════════════════════════════════════ */
static void test_activation_cost(void) {
    GameState g; wipe(&g);
    int a = rb_find_card_by_no(FILLER);
    CHECK(a >= 0, "activation_cost fixture resolves");
    if (a < 0) return;
    g.activating_card = a;
    AbilityEffect e;

    /* state.rs:1396-1406 — self records "activation_cost_{op}_{value}". */
    effect_init(&e, "activation_cost", "self");
    fx(&e, "operation", "increase");
    fx(&e, "value", "2");
    rb_effect_activation_cost(&g, 0, &e, a);
    CHECK(has_prohibition(&g, "activation_cost_increase_2"),
          "activation_cost records the prohibition note for self");

    /* state.rs:1401-1406 — a non self/opponent target records nothing. */
    wipe(&g);
    g.activating_card = a;
    effect_init(&e, "activation_cost", "live_card");
    fx(&e, "operation", "decrease");
    fx(&e, "value", "1");
    rb_effect_activation_cost(&g, 0, &e, a);
    CHECK_EQ(prohibition_count(&g, "activation_cost_"), 0,
             "activation_cost on a non self/opponent target records no prohibition");

    /* duration.rs:47-70 — a temporary effect removes the note at expiry. */
    wipe(&g);
    g.activating_card = a;
    effect_init(&e, "activation_cost", "opponent");
    fx(&e, "operation", "increase");
    fx(&e, "value", "3");
    fx(&e, "duration", "this_turn");
    rb_effect_activation_cost(&g, 0, &e, a);
    CHECK(has_prohibition(&g, "activation_cost_increase_3"),
          "activation_cost records the note before expiry");
    rb_check_expired_effects(&g, RB_TEMP_TURN_END);
    CHECK(!has_prohibition(&g, "activation_cost_increase_3"),
          "turn-scoped activation_cost prohibition expires at turn end");
}

/* ══════════════════════════════════════════════════════════════════════
 * 7. execute_modify_cost — state.rs:1626-1870
 * ══════════════════════════════════════════════════════════════════════ */
static void test_modify_cost(void) {
    GameState g; wipe(&g);
    int a = rb_find_card_by_no(FILLER);
    int b = rb_find_card_by_no(FILLER_2);
    CHECK(a >= 0 && b >= 0, "modify_cost fixtures resolve");
    if (a < 0 || b < 0) return;
    AbilityEffect e;

    /* state.rs:1817-1848 — add / subtract. */
    g.p[0].stage[0] = a;
    effect_init(&e, "modify_cost", "self");
    strcpy(e.card_type_field, "member_card");
    fx(&e, "operation", "add");
    fx(&e, "value", "2");
    rb_effect_modify_cost(&g, 0, &e, -1);
    CHECK_EQ(g.mods.cost[a].add, 2, "modify_cost add applies a positive delta");
    effect_init(&e, "modify_cost", "self");
    strcpy(e.card_type_field, "member_card");
    fx(&e, "operation", "subtract");
    fx(&e, "value", "1");
    rb_effect_modify_cost(&g, 0, &e, -1);
    CHECK_EQ(g.mods.cost[a].add, 1, "modify_cost subtract applies a negative delta");

    /* state.rs:1676-1682 — a hand-scoped cost modifier beats card_type. */
    wipe(&g);
    g.p[0].stage[0] = a;
    g.p[0].hand.n = 0;
    g.p[0].hand.cards[g.p[0].hand.n++] = b;
    effect_init(&e, "modify_cost", "self");
    strcpy(e.card_type_field, "member_card");
    fx(&e, "source", "hand");
    fx(&e, "operation", "add");
    fx(&e, "value", "3");
    rb_effect_modify_cost(&g, 0, &e, -1);
    CHECK_EQ(g.mods.cost[b].add, 3,
             "modify_cost with source=hand targets the hand even with card_type=member_card");
    CHECK_EQ(g.mods.cost[a].add, 0,
             "modify_cost with source=hand leaves the stage member alone");

    /* state.rs:1718-1722 — self_target narrows to the activating card. */
    wipe(&g);
    g.p[0].stage[0] = a;
    g.p[0].stage[1] = b;
    g.activating_card = a;
    effect_init(&e, "modify_cost", "self");
    strcpy(e.card_type_field, "member_card");
    strcpy(e.self_target_field, "true");
    fx(&e, "operation", "add");
    fx(&e, "value", "2");
    rb_effect_modify_cost(&g, 0, &e, a);
    CHECK_EQ(g.mods.cost[a].add, 2, "self_target modify_cost applies to the activating card");
    CHECK_EQ(g.mods.cost[b].add, 0, "self_target modify_cost skips the other members");

    /* state.rs:1821-1824 — an unknown operation is a no-op. */
    wipe(&g);
    g.p[0].stage[0] = a;
    effect_init(&e, "modify_cost", "self");
    strcpy(e.card_type_field, "member_card");
    fx(&e, "operation", "teleport");
    fx(&e, "value", "2");
    rb_effect_modify_cost(&g, 0, &e, -1);
    CHECK_EQ(g.mods.cost[a].add, 0, "modify_cost with an unknown operation is a no-op");
}

/* ══════════════════════════════════════════════════════════════════════
 * 8. execute_reduce_live_card_set_limit / set_cost_to_use / all_blade_timing
 *    — state.rs:1431-1448, 1585-1603, 1605-1624
 * ══════════════════════════════════════════════════════════════════════ */
static void test_small_state_effects(void) {
    GameState g; wipe(&g);
    int a = rb_find_card_by_no(FILLER);
    CHECK(a >= 0, "small-effect fixture resolves");
    if (a < 0) return;
    g.activating_card = a;
    AbilityEffect e;

    /* state.rs:1436,1446-1447 — count defaults to 1. */
    effect_init(&e, "reduce_live_card_set_limit", "self");
    e.count = 2;
    rb_effect_reduce_live_card_set_limit(&g, 0, &e, a);
    CHECK_EQ(g.live_set_limit_reduction[0], 2, "reduce_live_card_set_limit adds the count");
    e.count = -1;
    rb_effect_reduce_live_card_set_limit(&g, 0, &e, a);
    CHECK_EQ(g.live_set_limit_reduction[0], 3,
             "reduce_live_card_set_limit defaults to 1 when count is absent");

    /* state.rs:1590-1593 — set_cost_to_use writes the cost modifier. */
    wipe(&g);
    g.activating_card = a;
    effect_init(&e, "set_cost_to_use", "self");
    fx(&e, "value", "4");
    rb_effect_set_cost_to_use(&g, 0, &e, a);
    CHECK_EQ(g.mods.cost[a].set, 4, "set_cost_to_use sets the activating card's cost");

    /* state.rs:1610-1616 — all_blade_timing records timing + treat_as. */
    wipe(&g);
    g.activating_card = a;
    effect_init(&e, "all_blade_timing", "self");
    rb_effect_all_blade_timing(&g, 0, &e, a);
    char expect[80];
    snprintf(expect, sizeof expect, "all_blade_timing:%d:check_required_hearts:any_heart_color", a);
    CHECK(has_prohibition(&g, expect),
          "all_blade_timing records the default timing and treat_as");
}

/* ══════════════════════════════════════════════════════════════════════
 * 9. execute_energy_placement / energy_state_change — state.rs:733-996
 * ══════════════════════════════════════════════════════════════════════ */
static void test_energy_shapes(void) {
    GameState g; wipe(&g);
    int energy_ids[3];
    int ne = 0;
    for (uint32_t i = 0; i < rb_num_cards() && ne < 3; i++)
        if (rb_card_is_energy((int)i)) energy_ids[ne++] = (int)i;
    CHECK_EQ(ne, 3, "energy fixtures resolve");
    if (ne != 3) return;
    AbilityEffect e;

    /* state.rs:740-754 — deck->energy placement draws count cards. */
    g.p[0].energy_deck.n = 0;
    g.p[0].energy_deck.cards[g.p[0].energy_deck.n++] = energy_ids[0];
    g.p[0].energy_deck.cards[g.p[0].energy_deck.n++] = energy_ids[1];
    g.p[0].energy_deck.cards[g.p[0].energy_deck.n++] = energy_ids[2];
    g.p[0].energy.n = 0;
    g.p[0].energy_active = 0;
    effect_init(&e, "change_state", "self");
    e.count = 2;
    e.source = (char *)"deck";
    e.destination = (char *)"energy";
    fx(&e, "state_change", "active");
    rb_effect_change_state(&g, 0, &e, -1);
    CHECK_EQ(g.p[0].energy.n, 2, "deck->energy placement moves count cards into the energy zone");
    CHECK_EQ(g.p[0].energy_deck.n, 1, "deck->energy placement drains the energy deck");
    CHECK_EQ(g.p[0].energy_active, 2,
             "deck->energy placement with state_change=active activates the placed cards");

    /* state.rs:259-264 — the placement short-circuit only fires for
     * source=destination=deck->energy. */
    wipe(&g);
    g.state_change_triggering = 1;
    g.p[0].stage[0] = rb_find_card_by_no(FILLER);
    effect_init(&e, "change_state", "self");
    e.count = 1;
    e.source = (char *)"hand";
    e.destination = (char *)"stage";
    strcpy(e.card_type_field, "member_card");
    fx(&e, "state_change", "wait");
    rb_effect_change_state(&g, 0, &e, -1);
    CHECK(is_waited(&g, g.p[0].stage[0]),
          "a non deck->energy change_state still runs the member branch");
}

/* ══════════════════════════════════════════════════════════════════════
 * 10. execute_set_heart_copy_from_under — state.rs:1314-1382
 * ══════════════════════════════════════════════════════════════════════ */
static void test_heart_copy_from_under(void) {
    GameState g; wipe(&g);
    int a = rb_find_card_by_no(FILLER);
    int b = rb_find_card_by_no(FILLER_2);
    CHECK(a >= 0 && b >= 0, "heart-copy fixtures resolve");
    if (a < 0 || b < 0) return;
    g.p[0].stage[0] = a;
    g.p[0].under_cards[0].n = 0;
    g.p[0].under_cards[0].cards[g.p[0].under_cards[0].n++] = b;
    g.activating_card = a;
    AbilityEffect e;
    effect_init(&e, "set_heart_type", "self");
    fx(&e, "ref_value", "placed_under");
    fx(&e, "duration", "this_turn");
    rb_effect_set_heart_type(&g, 0, &e, a);
    CHECK_EQ(rb_mods_get_heart_copy(&g.mods, a), b,
             "set_heart_type ref_value=placed_under copies the under-card's hearts");
    rb_check_expired_effects(&g, RB_TEMP_TURN_END);
    CHECK_EQ(rb_mods_get_heart_copy(&g.mods, a), -1,
             "the copied hearts expire with the turn-scoped duration");
}

/* ══════════════════════════════════════════════════════════════════════
 * 11. execute_specify_heart_color — state.rs:1521-1557
 * ══════════════════════════════════════════════════════════════════════ */
static void test_specify_heart_color(void) {
    GameState g; wipe(&g);
    int a = rb_find_card_by_no(FILLER);
    CHECK(a >= 0, "specify_heart_color fixture resolves");
    if (a < 0) return;
    g.activating_card = a;
    AbilityEffect e;

    /* state.rs:1534-1548 — the 6 selectable colours, ALL excluded. */
    effect_init(&e, "specify_heart_color", "self");
    fx(&e, "choice", "true");
    rb_effect_specify_heart_color(&g, 0, &e, a);
    const RbChoice *ch = rb_get_pending_choice(&g);
    CHECK(ch && ch->kind == RB_CHOICE_SELECT_HEART_COLOR,
          "specify_heart_color offers a heart-colour choice");
    CHECK_EQ(ch ? ch->n_heart_options : -1, 6,
             "specify_heart_color offers exactly the 6 non-ALL colours");
    CHECK(ch && !strcmp(ch->heart_options[0], "heart01") &&
              !strcmp(ch->heart_options[5], "heart06"),
          "specify_heart_color lists heart01..heart06");
    g.queue.has_pending = 0;

    /* state.rs:1526,1532 — choice=false emits nothing. */
    wipe(&g);
    g.activating_card = a;
    effect_init(&e, "specify_heart_color", "self");
    rb_effect_specify_heart_color(&g, 0, &e, a);
    CHECK(!rb_has_pending_choice(&g),
          "specify_heart_color without choice=true emits no choice");
}

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_set_cost_zones_and_filters();
    test_set_card_identity();
    test_change_state_all_and_filters();
    test_change_state_per_unit_zero_is_noop();
    test_change_state_per_unit_scales();
    test_change_state_self_target_autoselect();
    test_change_state_prompts();
    test_change_state_optional_gate();
    test_change_state_cannot_activate();
    test_change_state_uses_prior_selection();
    test_set_blade_type();
    test_set_heart_type();
    test_activation_cost();
    test_modify_cost();
    test_small_state_effects();
    test_energy_shapes();
    test_heart_copy_from_under();
    test_specify_heart_color();
    rb_unload();
    printf("checks=%d failures=%d\n", checks, failures);
    if (failures) return 1;
    printf("ALL PARITY STATE EFFECTS 2 CHECKS PASSED\n");
    return 0;
}
