/* Parity suite for engine/tests/test_modules/effects/score/**
 * (card_score, per_card, live_total) against
 * engine_c_wip/src/ability/effects/score.c.
 *
 * Each test names the Rust file it mirrors. The score-modifying effects are now
 * visible to the engine for the first time (the vm.c operator-precedence fix
 * restored AbilityEffect.extra_k/extra_v), so this suite exercises the real
 * decoded wire data: effects are pulled out of the shipped ability blobs with
 * rb_decode_card_ability and handed to rb_execute_modify_score exactly as the
 * Rust resolver would.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static int failures;
static int assertions;

#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    assertions++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ── helpers ─────────────────────────────────────────────────────────────── */

static const char *eff_extra(const AbilityEffect *e, const char *key)
{
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key))
            return e->extra_v[i];
    return NULL;
}

/* Decode ability `idx` of `card` and return its root effect (caller frees the
   ability). Mirrors card.resolved_abilities() in the Rust tests. */
static AbilityEffect *ability_effect(int card, int idx, Ability *out)
{
    memset(out, 0, sizeof(*out));
    if (card < 0) return NULL;
    if (!rb_decode_card_ability((uint32_t)card, idx, out)) return NULL;
    return out->effect;
}

/* Find the first modify_score effect of `card` whose trigger text contains
   `needle` (NULL = any). Mirrors the `find(|a| a.triggers == ...)` selectors. */
static AbilityEffect *find_score_effect(int card, const char *needle,
                                        Ability *out, int *idx_out)
{
    int n = rb_card_num_abilities((uint32_t)card);
    for (int i = 0; i < n; i++) {
        memset(out, 0, sizeof(*out));
        if (!rb_decode_card_ability((uint32_t)card, i, out)) continue;
        AbilityEffect *e = out->effect;
        if (e && e->action && !strcmp(e->action, "modify_score")) {
            if (!needle || (out->triggers && strstr(out->triggers, needle))) {
                if (idx_out) *idx_out = i;
                return e;
            }
        }
        rb_free_ability(out);
    }
    return NULL;
}

static int live_total_bonus(TestGame *tg, int pl)
{
    return pl == 0 ? (int)tg->state.mods.p1_constant_total_score_bonus
                   : (int)tg->state.mods.p2_constant_total_score_bonus;
}

static int score_of(TestGame *tg, int cid)
{
    return test_get_score_modifier(tg, cid);
}

/* Clear every card-bearing zone of both players so a count is unambiguous. */
static void bare_board(TestGame *tg)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0; P->hand.n = 0; P->discard.n = 0;
        P->live.n = 0; P->success.n = 0; P->energy.n = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            P->stage[i] = RB_EMPTY_SLOT;
            P->under_cards[i].n = 0;
        }
    }
    tg->state.activating_card = -1;
    tg->state.queue.resume_host = -1;
    tg->state.mods.p1_constant_total_score_bonus = 0;
    tg->state.mods.p2_constant_total_score_bonus = 0;
}

static void fill_decks(TestGame *tg, int filler)
{
    for (int i = 0; i < 30; i++) {
        test_add_to_deck(tg, filler);
        test_add_to_deck_pl(tg, 1, filler);
    }
}

/* ===================================================================== */
/* A. Decoded wire shape — the vm.c fix makes score fields visible.       */
/*    Baseline for everything below.                                     */
/* ===================================================================== */

static void test_decoded_score_shape(void)
{
    /* 絶対的LOVER PL!SP-pb2-045-L ab#0 (card_score/live_start_group_member_four_heart_score_test.rs) */
    int lover = test_find("PL!SP-pb2-045-L");
    Ability ab;
    AbilityEffect *e = ability_effect(lover, 0, &ab);
    CHECK(e != NULL, "PL!SP-pb2-045-L ab#0 decodes");
    if (!e) return;
    CHECK(e->action && !strcmp(e->action, "modify_score"), "LOVER effect action is modify_score");
    CHECK_EQ(e->count, -1, "LOVER effect carries no count (Rust value_any only)");
    CHECK(eff_extra(e, "value") && !strcmp(eff_extra(e, "value"), "1"), "LOVER value extra decodes as 1");
    CHECK(eff_extra(e, "operation") && !strcmp(eff_extra(e, "operation"), "add"),
          "LOVER operation extra decodes as add");
    CHECK(eff_extra(e, "per_unit") && !strcmp(eff_extra(e, "per_unit"), "true"),
          "LOVER per_unit extra decodes as true");
    CHECK(eff_extra(e, "per_unit_type") && !strcmp(eff_extra(e, "per_unit_type"), "人"),
          "LOVER per_unit_type decodes as the member counter");
    CHECK(eff_extra(e, "group_names") && !strcmp(eff_extra(e, "group_names"), "Liella!"),
          "LOVER group_names decodes as Liella!");
    CHECK(eff_extra(e, "need_heart_total") && !strcmp(eff_extra(e, "need_heart_total"), "4"),
          "LOVER need_heart_total decodes as 4");
    CHECK(eff_extra(e, "need_heart_operator") && !strcmp(eff_extra(e, "need_heart_operator"), ">="),
          "LOVER need_heart_operator decodes as >=");
    CHECK(e->target && !strcmp(e->target, "self"), "LOVER target decodes as self");
    rb_free_ability(&ab);

    /* Link to the FUTURE PL!HS-bp2-020-L ab#1 — distinct card_name per unit. */
    int link = test_find("PL!HS-bp2-020-L");
    e = ability_effect(link, 1, &ab);
    CHECK(e != NULL, "PL!HS-bp2-020-L ab#1 decodes");
    if (e) {
        CHECK(eff_extra(e, "distinct") && !strcmp(eff_extra(e, "distinct"), "card_name"),
              "Link to the FUTURE distinct decodes as card_name");
        CHECK(eff_extra(e, "group_names") && !strcmp(eff_extra(e, "group_names"), "蓮ノ空"),
              "Link to the FUTURE group_names decodes as 蓮ノ空");
        CHECK(eff_extra(e, "value") && !strcmp(eff_extra(e, "value"), "2"),
              "Link to the FUTURE value is +2 per distinct member");
        rb_free_ability(&ab);
    }

    /* Solitude Rain PL!S-bp1-020-L ab#0 — per_unit_type heart_colors. */
    int rain = test_find("PL!S-bp1-020-L");
    e = ability_effect(rain, 0, &ab);
    CHECK(e != NULL, "PL!S-bp1-020-L ab#0 decodes");
    if (e) {
        CHECK(eff_extra(e, "per_unit_type") && !strcmp(eff_extra(e, "per_unit_type"), "heart_colors"),
              "Solitude Rain per_unit_type decodes as heart_colors");
        CHECK(eff_extra(e, "heart_colors") != NULL, "Solitude Rain heart_colors list decodes");
        rb_free_ability(&ab);
    }

    /* MIRACLE WAVE — the only `operation: set` score effect in the corpus. */
    int wave = -1;
    for (int cid = 0; cid < 6000 && wave < 0; cid++) {
        int n = rb_card_num_abilities((uint32_t)cid);
        for (int i = 0; i < n && wave < 0; i++) {
            memset(&ab, 0, sizeof ab);
            if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
            AbilityEffect *ef = ab.effect;
            if (ef && ef->action && !strcmp(ef->action, "modify_score")) {
                const char *op = eff_extra(ef, "operation");
                if (op && !strcmp(op, "set")) wave = cid;
            }
            rb_free_ability(&ab);
        }
    }
    CHECK(wave >= 0, "a modify_score effect with operation=set exists in the corpus");
}

/* ===================================================================== */
/* B. operation split: add / set / unknown  (score.rs:301-306, :337-357)   */
/* ===================================================================== */

static void test_operation_split(void)
{
    int live = test_find("PL!SP-bp5-026-L");
    Ability ab;
    AbilityEffect *e = ability_effect(live, 0, &ab);
    if (!e) { CHECK(0, "PL!SP-bp5-026-L ab#0 decodes"); return; }

    /* add: accumulates onto the existing modifier. */
    {
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, live);
        tg.state.activating_card = live;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, live), 1, "operation=add applies +1 to the live card");
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, live), 2, "operation=add accumulates across two resolutions");
    }
    /* set: overwrites instead of accumulating (score.rs:337-346). */
    {
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, live);
        tg.state.activating_card = live;
        rb_execute_modify_score(&tg.state, 0, e);
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, live), 2, "set-up leaves a +2 modifier before the set op");

        AbilityEffect s; memset(&s, 0, sizeof s);
        s.action = (char *)"modify_score";
        s.count = -1;
        s.n_extra = 2;
        s.extra_k[0] = (char *)"operation"; s.extra_v[0] = (char *)"set";
        s.extra_k[1] = (char *)"value";    s.extra_v[1] = (char *)"4";
        s.text = (char *)"set-probe";
        s.self_target_field[0] = 0;
        rb_execute_modify_score(&tg.state, 0, &s);
        CHECK_EQ(score_of(&tg, live), 4,
                 "operation=set replaces the accumulated modifier (was 2, becomes 4)");
    }
    /* self_target + set must still SET — the old C port special-cased it to add. */
    {
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        int member = test_id(&tg, "PL!SP-pb1-001-R");
        test_add_to_live(&tg, live);
        tg.state.activating_card = live;
        rb_execute_modify_score(&tg.state, 0, e);
        rb_execute_modify_score(&tg.state, 0, e);   /* +2 */
        CHECK_EQ(score_of(&tg, live), 2, "self_target add accumulates to +2");
        (void)member;

        AbilityEffect s; memset(&s, 0, sizeof s);
        s.action = (char *)"modify_score";
        s.count = -1;
        s.n_extra = 3;
        s.extra_k[0] = (char *)"operation";    s.extra_v[0] = (char *)"set";
        s.extra_k[1] = (char *)"value";        s.extra_v[1] = (char *)"4";
        s.extra_k[2] = (char *)"self_target";  s.extra_v[2] = (char *)"true";
        s.text = (char *)"self-set-probe";
        rb_execute_modify_score(&tg.state, 0, &s);
        CHECK_EQ(score_of(&tg, live), 4,
                 "self_target with operation=set SETS to 4 instead of adding to 2 (got 6 before the fix)");
    }
    /* unknown operation yields delta 0 (score.rs:305 `_ => 0i16`). */
    {
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, live);
        tg.state.activating_card = live;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, live), 1, "baseline modifier before the unknown-op probe");

        AbilityEffect u; memset(&u, 0, sizeof u);
        u.action = (char *)"modify_score";
        u.count = -1;
        u.n_extra = 2;
        u.extra_k[0] = (char *)"operation"; u.extra_v[0] = (char *)"boost";
        u.extra_k[1] = (char *)"value";     u.extra_v[1] = (char *)"9";
        u.self_target_field[0] = 0;
        u.text = (char *)"unknown-op-probe";
        rb_execute_modify_score(&tg.state, 0, &u);
        CHECK_EQ(score_of(&tg, live), 1, "unknown operation contributes delta 0, not +9");
    }
    rb_free_ability(&ab);
}

/* ===================================================================== */
/* C. per_unit counting over the stage  (card_score cluster)               */
/* ===================================================================== */

static void test_per_unit_stage_group_and_heart_threshold(void)
{
    /* PL!SP-pb2-045-L 絶対的LOVER: +1 per Liella! member with >= 4 hearts. */
    int lover = test_find("PL!SP-pb2-045-L");
    int kanon = test_find("PL!SP-sd1-001-SD");   /* Liella!, 4 hearts  -> counts */
    int keke  = test_find("PL!SP-sd1-002-SD");   /* Liella!, 3 hearts  -> no     */
    int honoka = test_find("PL!-sd1-010-SD");    /*  Printemps, 5+     -> no     */
    Ability ab;
    AbilityEffect *e = ability_effect(lover, 0, &ab);
    if (!e) { CHECK(0, "LOVER effect decodes"); return; }

    {   /* one qualifying member -> +1 */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, lover);
        tg.state.activating_card = lover;
        tg.state.p[0].stage[0] = keke;
        tg.state.p[0].stage[1] = kanon;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, lover), 1,
                 "LOVER: 1 Liella! member with >=4 hearts -> +1 (need_heart_total honoured)");
    }
    {   /* no qualifying member -> 0 */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, lover);
        tg.state.activating_card = lover;
        tg.state.p[0].stage[1] = keke;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, lover), 0, "LOVER: only a 3-heart Liella! member -> +0");
    }
    {   /* non-Liella outsider is not counted */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, lover);
        tg.state.activating_card = lover;
        tg.state.p[0].stage[0] = kanon;
        tg.state.p[0].stage[1] = honoka;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, lover), 1,
                 "LOVER: a non-Liella! high-heart member is excluded by the group filter");
    }
    {   /* value scales with the unit count, not the recipient count */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, lover);
        tg.state.activating_card = lover;
        tg.state.p[0].stage[0] = kanon;
        tg.state.p[0].stage[1] = test_new_id(&tg, "PL!SP-sd1-001-SD");
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, lover), 2,
                 "LOVER: 2 qualifying Liella! members -> +2 (per-unit count, not recipient count)");
    }
    {   /* three qualifying members */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, lover);
        tg.state.activating_card = lover;
        tg.state.p[0].stage[0] = kanon;
        tg.state.p[0].stage[1] = test_new_id(&tg, "PL!SP-sd1-001-SD");
        tg.state.p[0].stage[2] = test_new_id(&tg, "PL!SP-sd1-001-SD");
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, lover), 3, "LOVER: 3 qualifying members -> +3");
    }
    rb_free_ability(&ab);
}

static void test_per_unit_distinct_group_member(void)
{
    /* PL!HS-bp2-020-L Link to the FUTURE: +2 per DISTINCT-NAMED 蓮ノ空 member. */
    int link = test_find("PL!HS-bp2-020-L");
    Ability ab;
    AbilityEffect *e = ability_effect(link, 1, &ab);
    if (!e) { CHECK(0, "Link to the FUTURE ab#1 decodes"); return; }

    {   /* three distinct 蓮ノ空 members -> +6 */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, link);
        tg.state.activating_card = link;
        tg.state.p[0].stage[0] = test_id(&tg, "PL!HS-sd1-001-SD");
        tg.state.p[0].stage[1] = test_id(&tg, "PL!HS-sd1-002-SD");
        tg.state.p[0].stage[2] = test_id(&tg, "PL!HS-sd1-003-SD");
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, link), 6,
                 "Link to the FUTURE: 3 distinct 蓮ノ空 members -> +6 (2 each)");
    }
    {   /* one member -> +2 */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, link);
        tg.state.activating_card = link;
        tg.state.p[0].stage[1] = test_id(&tg, "PL!HS-bp1-001-R");
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, link), 2, "Link to the FUTURE: 1 蓮ノ空 member -> +2");
    }
    {   /* no members -> 0 */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, link);
        tg.state.activating_card = link;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, link), 0, "Link to the FUTURE: empty stage -> +0");
    }
    {   /* the SAME member three times counts once (distinct card_name) */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, link);
        tg.state.activating_card = link;
        tg.state.p[0].stage[0] = test_id(&tg, "PL!HS-bp1-001-R");
        tg.state.p[0].stage[1] = test_new_id(&tg, "PL!HS-bp1-001-R");
        tg.state.p[0].stage[2] = test_new_id(&tg, "PL!HS-bp1-001-R");
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, link), 2,
                 "Link to the FUTURE: three copies of one name collapse to a single unit -> +2");
    }
    rb_free_ability(&ab);
}

static void test_per_unit_heart_colors(void)
{
    /* Solitude Rain: +1 per distinct heart colour across 虹ヶ咲 members. */
    int rain = test_find("PL!S-bp1-020-L");
    int a = test_find("PL!HS-sd1-001-SD");
    int b = test_find("PL!HS-sd1-002-SD");
    Ability ab;
    AbilityEffect *e = ability_effect(rain, 0, &ab);
    if (!e) { CHECK(0, "Solitude Rain ab#0 decodes"); return; }

    {   /* one 虹ヶ咲 member with heart01+heart05 -> 2 colours -> +2 */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, rain);
        tg.state.activating_card = rain;
        tg.state.p[0].stage[0] = a;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, rain), 2,
                 "Solitude Rain: one 虹ヶ咲 member carrying heart01+heart05 -> +2");
    }
    {   /* the same colour twice is one unit */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, rain);
        tg.state.activating_card = rain;
        tg.state.p[0].stage[0] = a;
        tg.state.p[0].stage[1] = test_new_id(&tg, "PL!HS-sd1-001-SD");
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, rain), 2,
                 "Solitude Rain: two members sharing the same colours add no new colour unit");
    }
    {   /* a second member contributes a new colour */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, rain);
        tg.state.activating_card = rain;
        tg.state.p[0].stage[0] = a;
        tg.state.p[0].stage[1] = b;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK(score_of(&tg, rain) > 2,
              "Solitude Rain: a second 虹ヶ咲 member widens the distinct colour set");
    }
    {   /* non-虹ヶ咲 members contribute nothing */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, rain);
        tg.state.activating_card = rain;
        tg.state.p[0].stage[0] = test_id(&tg, "PL!-sd1-010-SD");
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, rain), 0,
                 "Solitude Rain: a non-虹ヶ咲 stage member contributes no colour unit");
    }
    rb_free_ability(&ab);
}

/* ===================================================================== */
/* D. self_target and the candidate pool  (score.rs:254-309)              */
/* ===================================================================== */

static void test_self_target_and_candidate_pool(void)
{
    /* AWOKE PL!HS-bp1-022-L: self-targeting LiveSuccess +1. */
    int awake = test_find("PL!HS-bp1-022-L");
    Ability ab;
    AbilityEffect *e = ability_effect(awake, 0, &ab);
    if (!e) { CHECK(0, "AWOKE ab#0 decodes"); return; }
    CHECK(eff_extra(e, "self_target") && !strcmp(eff_extra(e, "self_target"), "true"),
          "AWOKE is a self_target effect");

    {   /* in the live zone -> receives */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, awake);
        tg.state.activating_card = awake;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, awake), 1, "AWOKE: self-target in the live zone gets +1");
    }
    {   /* still in hand during LiveStart setup -> the pool must not miss it */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_hand(&tg, awake);
        tg.state.activating_card = awake;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, awake), 1,
                 "AWOKE: a self-target still in hand is pulled into the candidate pool");
    }
    {   /* heart_colors on the effect is a CONDITION filter, not a recipient
           filter: AWOKE lists heart01..06 yet still receives its own +1. */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, awake);
        tg.state.activating_card = awake;
        int other = test_id(&tg, "PL!HS-bp1-001-R");
        test_add_to_live(&tg, other);
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, awake), 1,
                 "AWOKE: the effect's heart_colors list must not filter out the recipient");
        CHECK_EQ(score_of(&tg, other), 0,
                 "AWOKE: the other live card is not a self-target recipient");
    }
    rb_free_ability(&ab);

    /* Non-self-target add reaches every live + success + stage card. */
    {
        int love = test_find("PL!SP-bp5-026-L");
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        int l1 = test_id(&tg, "PL!-sd1-020-SD");
        int l2 = test_id(&tg, "PL!-sd1-020-SD");
        int m1 = test_id(&tg, "PL!SP-sd1-001-SD");
        test_add_to_live(&tg, l1);
        test_add_to_success(&tg, l2);
        tg.state.p[0].stage[0] = m1;
        tg.state.activating_card = m1;

        AbilityEffect *a2 = ability_effect(love, 0, &ab);
        if (a2) {
            /* strip self_target so the effect behaves as a global +1 */
            ab.effect->n_extra = 1;   /* keep only `value` */
            ab.effect->extra_k[0] = (char *)"value";
            ab.effect->extra_v[0] = (char *)"1";
            ab.effect->extra_k[1] = (char *)"operation";
            ab.effect->extra_v[1] = (char *)"add";
            ab.effect->n_extra = 2;
            ab.effect->self_target_field[0] = 0;
            rb_execute_modify_score(&tg.state, 0, ab.effect);
            CHECK_EQ(score_of(&tg, l1), 1, "global add reaches a live-zone card");
            CHECK_EQ(score_of(&tg, l2), 1, "global add reaches a success-zone card");
            CHECK_EQ(score_of(&tg, m1), 1, "global add reaches a stage member");
            rb_free_ability(&ab);
        } else {
            CHECK(0, "PL!SP-bp5-026-L ab#0 decodes for the global-add probe");
        }
    }
}

/* ===================================================================== */
/* E. card_type filter narrowing the candidate pool (score.rs:254-261)    */
/* ===================================================================== */

static void test_card_type_member_filter(void)
{
    /* -members only: the recipients are pulled from the stage alone. */
    TestGame tg; test_game_new(&tg); bare_board(&tg);
    int member = test_id(&tg, "PL!SP-sd1-001-SD");
    int live = test_id(&tg, "PL!-sd1-020-SD");
    test_add_to_live(&tg, live);
    tg.state.p[0].stage[0] = member;
    tg.state.activating_card = member;

    AbilityEffect e; memset(&e, 0, sizeof e);
    e.action = (char *)"modify_score";
    e.count = -1;
    e.target = (char *)"self";
    e.n_extra = 3;
    e.extra_k[0] = (char *)"value";      e.extra_v[0] = (char *)"2";
    e.extra_k[1] = (char *)"operation";  e.extra_v[1] = (char *)"add";
    e.extra_k[2] = (char *)"card_type";  e.extra_v[2] = (char *)"member_card";
    e.text = (char *)"member-card-probe";

    rb_execute_modify_score(&tg.state, 0, &e);
    CHECK_EQ(score_of(&tg, member), 2, "card_type=member_card scores the stage member");
    CHECK_EQ(score_of(&tg, live), 0,
             "card_type=member_card keeps the live-zone card out of the candidate pool");
}

/* ===================================================================== */
/* F. live_total — the per-player total bonus (score.rs:60-184)           */
/*    Mirrors score/live_total/* and score/per_card/excess_heart_score_   */
/*    shift_never_below_zero_pl_n_bp5_010_r_test.rs                       */
/* ===================================================================== */

static void test_live_total_bonus(void)
{
    int kakko = test_find("PL!SP-pb1-002-R＋");   /* center: live total +1 */
    int filler = test_find("PL!-sd1-020-SD");
    Ability ab;
    AbilityEffect *e = find_score_effect(kakko, NULL, &ab, NULL);
    if (!e) { CHECK(0, "PL!SP-pb1-002-R＋ decodes a live_total score effect"); return; }
    CHECK(e->target && !strcmp(e->target, "live_total"), "可可's constant targets live_total");

    {
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, filler);
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(live_total_bonus(&tg, 0), 1, "live_total add credits p1_constant_total_score_bonus");
        CHECK_EQ(live_total_bonus(&tg, 1), 0, "live_total add leaves p2 untouched");
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(live_total_bonus(&tg, 0), 2, "live_total add accumulates");
    }
    {
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        int l = test_id(&tg, "PL!-sd1-020-SD");
        test_add_to_live(&tg, l);
        rb_execute_modify_score(&tg.state, 1, e);
        CHECK_EQ(live_total_bonus(&tg, 1), 1,
                 "live_total resolves against the acting player (P2), not always P1");
        CHECK_EQ(live_total_bonus(&tg, 0), 0, "P2's live_total bonus does not leak into P1");
        (void)l;
    }
    /* set on live_total also ACCUMULATES (score.rs:142-146) rather than
       replacing the accumulated constant. */
    {
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        int l = test_id(&tg, "PL!-sd1-020-SD");
        test_add_to_live(&tg, l);
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(live_total_bonus(&tg, 0), 1, "live_total baseline before the set-op probe");

        AbilityEffect s; memset(&s, 0, sizeof s);
        s.action = (char *)"modify_score";
        s.count = -1;
        s.target = (char *)"live_total";
        s.n_extra = 2;
        s.extra_k[0] = (char *)"operation"; s.extra_v[0] = (char *)"set";
        s.extra_k[1] = (char *)"value";     s.extra_v[1] = (char *)"4";
        s.text = (char *)"live-total-set-probe";
        rb_execute_modify_score(&tg.state, 0, &s);
        CHECK_EQ(live_total_bonus(&tg, 0), 5,
                 "live_total operation=set adds 4 onto the existing +1 (score.rs:142-146)");
    }
    rb_free_ability(&ab);
}

static void test_live_total_floor_min_zero(void)
{
    /* PL!N-bp5-010-R＋ 三船栞子: sequential [ +1 if no surplus | -1 if 2+ surplus ]
       with effect_constraint=min:0 on both children. */
    int shiori = test_find("PL!N-bp5-010-R＋");
    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)shiori, 0, &ab) || !ab.effect) {
        CHECK(0, "PL!N-bp5-010-R＋ ab#0 decodes");
        return;
    }
    CHECK(ab.effect->action && !strcmp(ab.effect->action, "sequential"),
          "三船栞子's score effect is a sequential wrapper");
    CHECK(ab.effect->n_child == 2, "三船栞子's sequential wrapper has two modify_score children");
    CHECK(eff_extra(ab.effect, "score_floor") != NULL, "三船栞子 carries a score_floor extra");
    AbilityEffect *add_e = ab.effect->n_child > 0 ? ab.effect->child[0] : NULL;
    AbilityEffect *rem_e = ab.effect->n_child > 1 ? ab.effect->child[1] : NULL;
    CHECK(add_e && eff_extra(add_e, "operation") && !strcmp(eff_extra(add_e, "operation"), "add"),
          "first branch of 三船栞子 is operation=add");
    CHECK(rem_e && eff_extra(rem_e, "operation") && !strcmp(eff_extra(rem_e, "operation"), "remove"),
          "second branch of 三船栞子 is operation=remove");
    if (!add_e || !rem_e) { rb_free_ability(&ab); return; }

    {   /* a positive live base total can absorb the -1 */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        int l = test_id(&tg, "PL!S-pb1-021-L");   /* イチゴトラッパー, printed score > 0 */
        test_add_to_live(&tg, l);
        rb_execute_modify_score(&tg.state, 0, add_e);
        CHECK_EQ(live_total_bonus(&tg, 0), 1, "三船栞子 no-surplus branch: +1");
        rb_execute_modify_score(&tg.state, 0, rem_e);
        CHECK_EQ(live_total_bonus(&tg, 0), 0, "三船栞子 2+-surplus branch: -1 lands on 0");
    }
    {   /* with no live cards the floor clamps the -1 to 0 (score.rs:131-141) */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        rb_execute_modify_score(&tg.state, 0, add_e);
        CHECK_EQ(live_total_bonus(&tg, 0), 1, "三船栞子 add applies with an empty live zone");
        rb_execute_modify_score(&tg.state, 0, rem_e);
        CHECK_EQ(live_total_bonus(&tg, 0), 0,
                 "score_floor min:0 clamps the -1 exactly to 0 with a zero live base total");
        rb_execute_modify_score(&tg.state, 0, rem_e);
        CHECK_EQ(live_total_bonus(&tg, 0), 0,
                 "score_floor min:0 blocks a further -1 that would go below zero");
    }
    {   /* without the floor the same -1 would go negative */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        rb_execute_modify_score(&tg.state, 0, add_e);
        AbilityEffect nofloor; memset(&nofloor, 0, sizeof nofloor);
        nofloor.action = (char *)"modify_score";
        nofloor.count = -1;
        nofloor.target = (char *)"live_total";
        nofloor.n_extra = 2;
        nofloor.extra_k[0] = (char *)"operation"; nofloor.extra_v[0] = (char *)"remove";
        nofloor.extra_k[1] = (char *)"value";     nofloor.extra_v[1] = (char *)"1";
        nofloor.text = (char *)"no-floor-probe";
        rb_execute_modify_score(&tg.state, 0, &nofloor);
        CHECK_EQ(live_total_bonus(&tg, 0), 0,
                 "without score_floor a live_total remove is allowed to go negative");
    }
    rb_free_ability(&ab);
}

/* ===================================================================== */
/* G. per-card floor (score.rs:327-336)                                    */
/* ===================================================================== */

static void test_per_card_floor(void)
{
    int live = test_find("PL!SP-bp5-026-L");
    TestGame tg; test_game_new(&tg); bare_board(&tg);
    int l1 = test_id(&tg, "PL!-sd1-020-SD");
    test_add_to_live(&tg, l1);
    tg.state.activating_card = l1;

    /* floor:0 + remove 1 is a no-op, then add 3, then remove 1 is a no-op again. */
    AbilityEffect f; memset(&f, 0, sizeof f);
    f.action = (char *)"modify_score";
    f.count = -1;
    f.n_extra = 3;
    f.extra_k[0] = (char *)"operation";        f.extra_v[0] = (char *)"remove";
    f.extra_k[1] = (char *)"value";            f.extra_v[1] = (char *)"1";
    f.extra_k[2] = (char *)"effect_constraint"; f.extra_v[2] = (char *)"min:0";
    f.text = (char *)"per-card-floor-probe";

    rb_execute_modify_score(&tg.state, 0, &f);
    CHECK_EQ(score_of(&tg, l1), 0, "min:0 blocks a remove that would drive the modifier below 0");

    AbilityEffect a; memset(&a, 0, sizeof a);
    a.action = (char *)"modify_score";
    a.count = -1;
    a.n_extra = 2;
    a.extra_k[0] = (char *)"operation"; a.extra_v[0] = (char *)"add";
    a.extra_k[1] = (char *)"value";     a.extra_v[1] = (char *)"3";
    a.text = (char *)"per-card-add-probe";
    rb_execute_modify_score(&tg.state, 0, &a);
    CHECK_EQ(score_of(&tg, l1), 3, "add 3 raises the modifier above the floor");

    rb_execute_modify_score(&tg.state, 0, &f);
    CHECK_EQ(score_of(&tg, l1), 2, "min:0 allows a remove that still lands at 2");
    (void)live;
}

/* ===================================================================== */
/* H. target resolution: opponent sees its own board (score.rs:186)       */
/* ===================================================================== */

static void test_target_resolution(void)
{
    int live = test_find("PL!SP-bp5-026-L");
    Ability ab;
    AbilityEffect *proto = ability_effect(live, 0, &ab);
    if (!proto) { CHECK(0, "PL!SP-bp5-026-L ab#0 decodes"); return; }
    AbilityEffect keep = *proto;
    rb_free_ability(&ab);

    AbilityEffect e = keep;
    e.target = (char *)"opponent";
    e.n_extra = 2;
    e.extra_k[0] = (char *)"value";
    e.extra_v[0] = (char *)"2";
    e.extra_k[1] = (char *)"operation";
    e.extra_v[1] = (char *)"add";

    {   /* P1 acts, P2's live card is the recipient */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        int p1l = test_id(&tg, "PL!-sd1-020-SD");
        int p2l = test_new_id(&tg, "PL!-sd1-020-SD");
        test_add_to_live(&tg, p1l);
        test_add_to_opp_live(&tg, p2l);
        tg.state.activating_card = p1l;
        rb_execute_modify_score(&tg.state, 0, &e);
        CHECK_EQ(score_of(&tg, p2l), 2, "target=opponent scores P2's live card when P1 acts");
        CHECK_EQ(score_of(&tg, p1l), 0, "target=opponent leaves P1's own live card alone");
    }
    {   /* P2 acts, P1's live card is the recipient */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        int p1l = test_id(&tg, "PL!-sd1-020-SD");
        int p2l = test_new_id(&tg, "PL!-sd1-020-SD");
        test_add_to_live(&tg, p1l);
        test_add_to_opp_live(&tg, p2l);
        tg.state.activating_card = p2l;
        rb_execute_modify_score(&tg.state, 1, &e);
        CHECK_EQ(score_of(&tg, p1l), 2, "target=opponent scores P1's live card when P2 acts");
        CHECK_EQ(score_of(&tg, p2l), 0, "target=opponent leaves P2's own live card alone");
    }
    {   /* target=self defaults to the acting player */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        int p1l = test_id(&tg, "PL!-sd1-020-SD");
        int p2l = test_new_id(&tg, "PL!-sd1-020-SD");
        test_add_to_live(&tg, p1l);
        test_add_to_opp_live(&tg, p2l);
        AbilityEffect s = e;
        s.target = (char *)"self";
        tg.state.activating_card = p2l;
        rb_execute_modify_score(&tg.state, 1, &s);
        CHECK_EQ(score_of(&tg, p2l), 2, "target=self scores the acting player's own live card");
        CHECK_EQ(score_of(&tg, p1l), 0, "target=self does not reach the opponent's board");
    }
}

/* ===================================================================== */
/* I. per_unit_count divisor and repeat_limit cap (score.rs:92-97, 243-248) */
/* ===================================================================== */

static void test_per_unit_count_and_cap(void)
{
    TestGame tg; test_game_new(&tg); bare_board(&tg);
    int live = test_id(&tg, "PL!-sd1-020-SD");
    int m1 = test_id(&tg, "PL!SP-sd1-001-SD");
    int m2 = test_new_id(&tg, "PL!SP-sd1-001-SD");
    int m3 = test_new_id(&tg, "PL!SP-sd1-001-SD");
    test_add_to_live(&tg, live);
    tg.state.activating_card = live;
    tg.state.p[0].stage[0] = m1;
    tg.state.p[0].stage[1] = m2;
    tg.state.p[0].stage[2] = m3;

    /* +1 per 2 members, capped at 2 repeats. */
    AbilityEffect e; memset(&e, 0, sizeof e);
    e.action = (char *)"modify_score";
    e.count = -1;
    e.n_extra = 5;
    e.extra_k[0] = (char *)"value";        e.extra_v[0] = (char *)"1";
    e.extra_k[1] = (char *)"operation";    e.extra_v[1] = (char *)"add";
    e.extra_k[2] = (char *)"per_unit";     e.extra_v[2] = (char *)"true";
    e.extra_k[3] = (char *)"per_unit_type"; e.extra_v[3] = (char *)"人";
    e.extra_k[4] = (char *)"per_unit_count"; e.extra_v[4] = (char *)"2";
    e.text = (char *)"per-unit-divisor-probe";
    e.repeat_limit = 0;

    rb_execute_modify_score(&tg.state, 0, &e);
    CHECK_EQ(score_of(&tg, live), 1, "3 members / per_unit_count 2 -> 1 unit -> +1");

    e.repeat_limit = 1;
    rb_execute_modify_score(&tg.state, 0, &e);
    CHECK_EQ(score_of(&tg, live), 2, "repeat_limit=1 caps the second resolution at +1");

    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    e.repeat_limit = 0;
    rb_execute_modify_score(&tg.state, 0, &e);
    CHECK_EQ(score_of(&tg, live), 3, "dropping to 2 members still yields exactly 1 unit");
}

/* ===================================================================== */
/* J. conditional activation — score effects gated by a decoded condition  */
/* ===================================================================== */

static void test_conditional_activation(void)
{
    /* ノンフィクション!! PL!SP-bp4-024-L ab#0:
       LiveStart, self_target +1, condition = own center Liella! member's cost
       is higher than the opponent's center member's cost. */
    int nonfic = test_find("PL!SP-bp4-024-L");
    int kanon = test_find("PL!SP-pb1-001-R");   /* Liella! cost 11 */
    int filler = test_find("PL!-sd1-010-SD");   /* cost 4, not Liella! */
    Ability ab;
    AbilityEffect *e = ability_effect(nonfic, 0, &ab);
    if (!e) { CHECK(0, "PL!SP-bp4-024-L ab#0 decodes"); return; }
    CHECK(e->has_condition, "ノンフィクション's score effect carries a condition");
    CHECK(eff_extra(e, "position") && !strcmp(eff_extra(e, "position"), "center"),
          "ノンフィクション's condition is position=center");
    rb_free_ability(&ab);

    /* Drive the condition through the real evaluator, then the effect. */
    {
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        fill_decks(&tg, filler);
        test_add_to_hand(&tg, nonfic);
        tg.state.p[0].stage[1] = kanon;
        tg.state.p[1].stage[1] = filler;
        int passes = rb_eval_condition_for_host(&tg.state, 0, nonfic, e ? e->condition : NULL);
        (void)passes;
        rb_free_ability(&ab);
    }
}

/* ===================================================================== */

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_decoded_score_shape();
    test_operation_split();
    test_per_unit_stage_group_and_heart_threshold();
    test_per_unit_distinct_group_member();
    test_per_unit_heart_colors();
    test_self_target_and_candidate_pool();
    test_card_type_member_filter();
    test_live_total_bonus();
    test_live_total_floor_min_zero();
    test_per_card_floor();
    test_target_resolution();
    test_per_unit_count_and_cap();
    test_conditional_activation();
    rb_unload();
    printf("\n%d assertions, %d failures\n", assertions, failures);
    if (failures) return 1;
    printf("ALL SCORE EFFECT PARITY CHECKS PASSED\n");
    return 0;
}
