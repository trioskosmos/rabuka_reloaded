/* Parity suite for engine/tests/test_modules/effects/score/**
 * (card_score/, per_card/, live_total/) against
 * engine_c_wip/src/ability/effects/score.c.
 *
 * Every test names the Rust file it mirrors. Score effects only became visible
 * to the engine after the vm.c operator-precedence fix restored
 * AbilityEffect.extra_k/extra_v, so this suite drives the *real* decoded wire
 * data: effects are pulled out of the shipped ability blobs with
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

/* Decode ability `idx` of `card` and hand back its root effect. The caller
   owns `out` and must rb_free_ability() it. */
static AbilityEffect *ability_effect(int card, int idx, Ability *out)
{
    memset(out, 0, sizeof(*out));
    if (card < 0) return NULL;
    if (!rb_decode_card_ability((uint32_t)card, idx, out)) return NULL;
    return out->effect;
}

/* An empty board so every card-counting assertion is unambiguous. */
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

static int live_total_bonus(TestGame *tg, int pl)
{
    return pl == 0 ? (int)tg->state.mods.p1_constant_total_score_bonus
                   : (int)tg->state.mods.p2_constant_total_score_bonus;
}

static int score_of(TestGame *tg, int cid)
{
    return test_get_score_modifier(tg, cid);
}

/* A synthetic modify_score effect mirroring a wire shape the corpus does not
   contain, so the operation split and the filters can be probed in isolation. */
typedef struct {
    AbilityEffect e;
    char k[8][24];
    char v[8][24];
} SynthEffect;

static AbilityEffect *synth(SynthEffect *s, const char *target, int n)
{
    memset(s, 0, sizeof(*s));
    s->e.action = (char *)"modify_score";
    s->e.count = -1;
    s->e.target = (char *)target;
    s->e.text = (char *)"synth-probe";
    s->e.n_extra = n;
    for (int i = 0; i < n; i++) {
        s->e.extra_k[i] = s->k[i];
        s->e.extra_v[i] = s->v[i];
    }
    return &s->e;
}

static void synth_set(SynthEffect *s, int i, const char *k, const char *v)
{
    snprintf(s->k[i], sizeof s->k[i], "%s", k);
    snprintf(s->v[i], sizeof s->v[i], "%s", v);
    s->e.n_extra = i + 1;
}

/* ===================================================================== */
/* A. Decoded wire shape.  Baseline for every test below.                 */
/*    card_score/live_start_group_member_four_heart_score_test.rs         */
/*    per_card/solitude_rain_unique_heart_color_score_q67_test.rs        */
/* ===================================================================== */

static void test_decoded_score_shape(void)
{
    /* 絶対的LOVER PL!SP-pb2-045-L ab#0 */
    int lover = rb_find_card_by_no("PL!SP-pb2-045-L");
    Ability ab;
    AbilityEffect *e = ability_effect(lover, 0, &ab);
    CHECK(e != NULL, "PL!SP-pb2-045-L ab#0 decodes");
    if (!e) return;
    CHECK(e->action && !strcmp(e->action, "modify_score"), "LOVER effect action is modify_score");
    CHECK_EQ(e->count, -1, "LOVER effect carries no count (Rust reads value only)");
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
    int link = rb_find_card_by_no("PL!HS-bp2-020-L");
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

    /* Solitude Rain PL!N-bp1-027-L ab#0 — per_unit_type heart_colors. */
    int rain = rb_find_card_by_no("PL!N-bp1-027-L");
    e = ability_effect(rain, 0, &ab);
    CHECK(e != NULL, "PL!N-bp1-027-L ab#0 decodes");
    if (e) {
        CHECK(eff_extra(e, "per_unit_type") && !strcmp(eff_extra(e, "per_unit_type"), "heart_colors"),
              "Solitude Rain per_unit_type decodes as heart_colors");
        CHECK(eff_extra(e, "heart_colors") != NULL, "Solitude Rain heart_colors list decodes");
        CHECK(eff_extra(e, "self_target") && !strcmp(eff_extra(e, "self_target"), "true"),
              "Solitude Rain is a self_target effect");
        rb_free_ability(&ab);
    }

    /* MIRACLE WAVE — the only operation=set score effect in the corpus. */
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
    if (wave >= 0) {
        memset(&ab, 0, sizeof ab);
        if (rb_decode_card_ability((uint32_t)wave, 0, &ab) && ab.effect) {
            CHECK(eff_extra(ab.effect, "value") && !strcmp(eff_extra(ab.effect, "value"), "4"),
                  "the operation=set score effect sets the score to 4");
            CHECK(eff_extra(ab.effect, "self_target") &&
                  !strcmp(eff_extra(ab.effect, "self_target"), "true"),
                  "the operation=set score effect is self-targeting");
        }
        rb_free_ability(&ab);
    }
}

/* ===================================================================== */
/* B. operation split: add / remove / set / unknown                       */
/*    score.rs:301-306, :102-107, :337-357                               */
/* ===================================================================== */

static void test_operation_split(void)
{
    int live = rb_find_card_by_no("PL!SP-bp5-026-L");
    Ability ab;
    AbilityEffect *e = ability_effect(live, 0, &ab);
    if (!e) { CHECK(0, "PL!SP-bp5-026-L ab#0 decodes"); return; }

    {   /* add accumulates */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, live);
        tg.state.activating_card = live;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, live), 1, "operation=add applies +1 to the live card");
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, live), 2, "operation=add accumulates across two resolutions");
    }
    {   /* set overwrites */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, live);
        tg.state.activating_card = live;
        rb_execute_modify_score(&tg.state, 0, e);
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, live), 2, "set-up leaves a +2 modifier before the set op");
        SynthEffect s; synth(&s, NULL, 0);
        synth_set(&s, 0, "operation", "set");
        synth_set(&s, 1, "value", "4");
        rb_execute_modify_score(&tg.state, 0, &s.e);
        CHECK_EQ(score_of(&tg, live), 4,
                 "operation=set replaces the accumulated modifier (2 -> 4)");
    }
    {   /* set must SET even for a self-target (the old C port special-cased
           self_target to add, producing 2+4=6) */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, live);
        tg.state.activating_card = live;
        rb_execute_modify_score(&tg.state, 0, e);
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, live), 2, "self_target add accumulates to +2");
        SynthEffect s; synth(&s, NULL, 0);
        synth_set(&s, 0, "operation", "set");
        synth_set(&s, 1, "value", "4");
        synth_set(&s, 2, "self_target", "true");
        rb_execute_modify_score(&tg.state, 0, &s.e);
        CHECK_EQ(score_of(&tg, live), 4,
                 "self_target with operation=set SETS to 4 instead of adding (was 6)");
    }
    {   /* remove subtracts */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, live);
        tg.state.activating_card = live;
        rb_execute_modify_score(&tg.state, 0, e);
        rb_execute_modify_score(&tg.state, 0, e);
        SynthEffect s; synth(&s, NULL, 0);
        synth_set(&s, 0, "operation", "remove");
        synth_set(&s, 1, "value", "1");
        rb_execute_modify_score(&tg.state, 0, &s.e);
        CHECK_EQ(score_of(&tg, live), 1, "operation=remove subtracts from the modifier");
    }
    {   /* unknown operation yields delta 0 (score.rs:305 `_ => 0i16`) */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, live);
        tg.state.activating_card = live;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, live), 1, "baseline modifier before the unknown-op probe");
        SynthEffect s; synth(&s, NULL, 0);
        synth_set(&s, 0, "operation", "boost");
        synth_set(&s, 1, "value", "9");
        rb_execute_modify_score(&tg.state, 0, &s.e);
        CHECK_EQ(score_of(&tg, live), 1, "unknown operation contributes delta 0, not +9");
    }
    rb_free_ability(&ab);
}

/* ===================================================================== */
/* C. per-unit counting over the stage — card_score cluster                */
/*    card_score/live_start_group_member_four_heart_score_test.rs         */
/* ===================================================================== */

static void test_per_unit_stage_group_and_heart_threshold(void)
{
    /* PL!SP-pb2-045-L 絶対的LOVER: +1 per Liella! member with >= 4 hearts. */
    int lover = rb_find_card_by_no("PL!SP-pb2-045-L");
    int kanon  = rb_find_card_by_no("PL!SP-sd1-001-SD");  /* Liella!, 4 printed hearts */
    int keke   = rb_find_card_by_no("PL!SP-sd1-002-SD");  /* Liella!, 3 printed hearts */
    int honoka = rb_find_card_by_no("PL!-sd1-010-SD");    /*  Printemps, not Liella! */
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
    {   /* the value scales with the unit count, not the recipient count */
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
    {   /* the recipients are still the self-target only */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, lover);
        tg.state.activating_card = lover;
        tg.state.p[0].stage[0] = kanon;
        int other = test_id(&tg, "PL!S-pb1-021-L");
        test_add_to_live(&tg, other);
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, lover), 1, "LOVER scores only its own self-target card");
        CHECK_EQ(score_of(&tg, other), 0, "LOVER does not touch the other live card");
    }
    rb_free_ability(&ab);
}

static void test_per_unit_distinct_group_member(void)
{
    /* PL!HS-bp2-020-L Link to the FUTURE: +2 per DISTINCT-NAMED 蓮ノ空 member.
       card_score/live_start_distinct_group_member_score_test.rs */
    int link = rb_find_card_by_no("PL!HS-bp2-020-L");
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
    {   /* a non-蓮ノ空 member is not counted */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, link);
        tg.state.activating_card = link;
        tg.state.p[0].stage[0] = test_id(&tg, "PL!HS-sd1-001-SD");
        tg.state.p[0].stage[1] = test_id(&tg, "PL!-sd1-010-SD");
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, link), 2,
                 "Link to the FUTURE: a non-蓮ノ空 stage member adds no unit");
    }
    rb_free_ability(&ab);
}

static void test_per_unit_heart_colors(void)
{
    /* Solitude Rain: +1 per distinct heart colour across 虹ヶ咲 members.
       per_card/solitude_rain_unique_heart_color_score_q67_test.rs */
    int rain = rb_find_card_by_no("PL!N-bp1-027-L");
    int ayumu = rb_find_card_by_no("PL!N-sd1-001-SD"); /* base hearts 01,02,04 */
    int kasumi = rb_find_card_by_no("PL!N-sd1-002-SD"); /* base hearts 03,04 */
    int shizuku = rb_find_card_by_no("PL!N-sd1-003-SD"); /* base hearts 01,05 */
    Ability ab;
    AbilityEffect *e = ability_effect(rain, 0, &ab);
    if (!e) { CHECK(0, "Solitude Rain ab#0 decodes"); return; }

    {   /* one member carrying 3 distinct colours -> +3 */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, rain);
        tg.state.activating_card = rain;
        tg.state.p[0].stage[0] = ayumu;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, rain), 3,
                 "Solitude Rain: one 虹ヶ咲 member with heart01/02/04 -> +3");
    }
    {   /* a second member adds only its NEW colour (03) */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, rain);
        tg.state.activating_card = rain;
        tg.state.p[0].stage[0] = ayumu;
        tg.state.p[0].stage[1] = kasumi;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, rain), 4,
                 "Solitude Rain: a second member adds only its unseen colour -> +4");
    }
    {   /* heart04 is shared, heart05 is new */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, rain);
        tg.state.activating_card = rain;
        tg.state.p[0].stage[0] = ayumu;
        tg.state.p[0].stage[1] = shizuku;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, rain), 4,
                 "Solitude Rain: a member sharing heart01/04 and adding heart05 -> +4");
    }
    {   /* a non-虹ヶ咲 member contributes nothing */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, rain);
        tg.state.activating_card = rain;
        tg.state.p[0].stage[0] = test_id(&tg, "PL!-sd1-010-SD");
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, rain), 0,
                 "Solitude Rain: a non-虹ヶ咲 stage member contributes no colour unit");
    }
    {   /* an empty stage contributes nothing */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, rain);
        tg.state.activating_card = rain;
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, rain), 0, "Solitude Rain: empty stage -> +0");
    }
    rb_free_ability(&ab);
}

/* ===================================================================== */
/* D. self_target and the candidate pool — score.rs:254-309               */
/*    card_score/live_success_group_cheer_count_live_score_q36_q107_test.rs */
/* ===================================================================== */

static void test_self_target_and_candidate_pool(void)
{
    /* AWOKE PL!HS-bp1-022-L: self-targeting LiveSuccess +1. */
    int awake = rb_find_card_by_no("PL!HS-bp1-022-L");
    Ability ab;
    AbilityEffect *e = ability_effect(awake, 0, &ab);
    if (!e) { CHECK(0, "AWOKE ab#0 decodes"); return; }
    CHECK(eff_extra(e, "self_target") && !strcmp(eff_extra(e, "self_target"), "true"),
          "AWOKE is a self_target effect");
    CHECK(e->target == NULL, "AWOKE carries no target field, so target_name() defaults to self");

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
    {   /* heart_colors on the effect is a CONDITION predicate, not a recipient
           filter: AWOKE lists heart01..06 yet must still receive its own +1. */
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

    /* A non-self-target add reaches every live + success + stage card. */
    {
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        int l1 = test_id(&tg, "PL!-sd1-020-SD");
        int l2 = test_new_id(&tg, "PL!-sd1-020-SD");
        int m1 = test_id(&tg, "PL!SP-sd1-001-SD");
        test_add_to_live(&tg, l1);
        test_add_to_success(&tg, l2);
        tg.state.p[0].stage[0] = m1;
        tg.state.activating_card = m1;

        SynthEffect s; synth(&s, "self", 0);
        synth_set(&s, 0, "value", "1");
        synth_set(&s, 1, "operation", "add");
        rb_execute_modify_score(&tg.state, 0, &s.e);
        CHECK_EQ(score_of(&tg, l1), 1, "global add reaches a live-zone card");
        CHECK_EQ(score_of(&tg, l2), 1, "global add reaches a success-zone card");
        CHECK_EQ(score_of(&tg, m1), 1, "global add reaches a stage member");
    }
    {   /* a global add with no activating card still scores the board */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        int l1 = test_id(&tg, "PL!-sd1-020-SD");
        test_add_to_live(&tg, l1);
        SynthEffect s; synth(&s, "self", 0);
        synth_set(&s, 0, "value", "1");
        synth_set(&s, 1, "operation", "add");
        rb_execute_modify_score(&tg.state, 0, &s.e);
        CHECK_EQ(score_of(&tg, l1), 1,
                 "global add with no activating card still reaches the live zone");
    }
}

/* ===================================================================== */
/* E. card_type narrowing the candidate pool — score.rs:254-261           */
/* ===================================================================== */

static void test_card_type_member_filter(void)
{
    TestGame tg; test_game_new(&tg); bare_board(&tg);
    int member = test_id(&tg, "PL!SP-sd1-001-SD");
    int live = test_id(&tg, "PL!-sd1-020-SD");
    test_add_to_live(&tg, live);
    tg.state.p[0].stage[0] = member;
    tg.state.activating_card = member;

    SynthEffect s; synth(&s, "self", 0);
    synth_set(&s, 0, "value", "2");
    synth_set(&s, 1, "operation", "add");
    synth_set(&s, 2, "card_type", "member_card");
    rb_execute_modify_score(&tg.state, 0, &s.e);
    CHECK_EQ(score_of(&tg, member), 2, "card_type=member_card scores the stage member");
    CHECK_EQ(score_of(&tg, live), 0,
             "card_type=member_card keeps the live-zone card out of the candidate pool");
}

/* ===================================================================== */
/* F. live_total — the per-player total bonus, score.rs:60-184            */
/*    live_total/twelve_energy_pl_sp_pb1_002_r_test.rs                    */
/*    per_card/excess_heart_score_shift_never_below_zero_pl_n_bp5_010_r_test.rs */
/* ===================================================================== */

static void test_live_total_bonus(void)
{
    int kakko = rb_find_card_by_no("PL!SP-pb1-002-R＋");   /* center: live total +1 */
    Ability ab;
    memset(&ab, 0, sizeof ab);
    AbilityEffect *e = NULL;
    if (rb_decode_card_ability((uint32_t)kakko, 0, &ab) && ab.effect &&
        ab.effect->action && !strcmp(ab.effect->action, "modify_score"))
        e = ab.effect;
    if (!e) { CHECK(0, "PL!SP-pb1-002-R＋ ab#0 is a modify_score effect"); return; }
    CHECK(e->target && !strcmp(e->target, "live_total"), "可可's constant targets live_total");

    {
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, test_id(&tg, "PL!-sd1-020-SD"));
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(live_total_bonus(&tg, 0), 1, "live_total add credits p1_constant_total_score_bonus");
        CHECK_EQ(live_total_bonus(&tg, 1), 0, "live_total add leaves p2 untouched");
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(live_total_bonus(&tg, 0), 2, "live_total add accumulates");
    }
    {
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, test_id(&tg, "PL!-sd1-020-SD"));
        rb_execute_modify_score(&tg.state, 1, e);
        CHECK_EQ(live_total_bonus(&tg, 1), 1,
                 "live_total resolves against the acting player (P2), not always P1");
        CHECK_EQ(live_total_bonus(&tg, 0), 0, "P2's live_total bonus does not leak into P1");
    }
    {   /* set on live_total also ACCUMULATES (score.rs:142-146) */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, test_id(&tg, "PL!-sd1-020-SD"));
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(live_total_bonus(&tg, 0), 1, "live_total baseline before the set-op probe");
        SynthEffect s; synth(&s, "live_total", 0);
        synth_set(&s, 0, "operation", "set");
        synth_set(&s, 1, "value", "4");
        rb_execute_modify_score(&tg.state, 0, &s.e);
        CHECK_EQ(live_total_bonus(&tg, 0), 5,
                 "live_total operation=set adds 4 onto the existing +1 (score.rs:142-146)");
    }
    {   /* a live_total add never writes a per-card score modifier */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        int l = test_id(&tg, "PL!-sd1-020-SD");
        test_add_to_live(&tg, l);
        rb_execute_modify_score(&tg.state, 0, e);
        CHECK_EQ(score_of(&tg, l), 0, "live_total does not write a per-card score modifier");
    }
    rb_free_ability(&ab);
}

static void test_live_total_floor_min_zero(void)
{
    /* PL!N-bp5-010-R＋ 三船栞子: sequential [ +1 if no surplus | -1 if 2+ surplus ]
       with effect_constraint=min:0 on both children. */
    int shiori = rb_find_card_by_no("PL!N-bp5-010-R＋");
    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)shiori, 0, &ab) || !ab.effect) {
        CHECK(0, "PL!N-bp5-010-R＋ ab#0 decodes");
        return;
    }
    CHECK(ab.effect->action && !strcmp(ab.effect->action, "sequential"),
          "三船栞子's score effect is a sequential wrapper");
    CHECK_EQ(ab.effect->n_child, 2, "三船栞子's sequential wrapper has two modify_score children");
    CHECK(eff_extra(ab.effect, "score_floor") != NULL, "三船栞子 carries a score_floor extra");
    AbilityEffect *add_e = ab.effect->n_child > 0 ? ab.effect->child[0] : NULL;
    AbilityEffect *rem_e = ab.effect->n_child > 1 ? ab.effect->child[1] : NULL;
    CHECK(add_e && eff_extra(add_e, "operation") && !strcmp(eff_extra(add_e, "operation"), "add"),
          "first branch of 三船栞子 is operation=add");
    CHECK(rem_e && eff_extra(rem_e, "operation") && !strcmp(eff_extra(rem_e, "operation"), "remove"),
          "second branch of 三船栞子 is operation=remove");
    if (!add_e || !rem_e) { rb_free_ability(&ab); return; }

    {   /* the +1 / -1 pair nets back to 0 */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, test_id(&tg, "PL!S-pb1-021-L"));
        rb_execute_modify_score(&tg.state, 0, add_e);
        CHECK_EQ(live_total_bonus(&tg, 0), 1, "三船栞子 no-surplus branch: +1");
        rb_execute_modify_score(&tg.state, 0, rem_e);
        CHECK_EQ(live_total_bonus(&tg, 0), 0, "三船栞子 2+-surplus branch: -1 lands on 0");
    }
    {   /* with no live cards the floor clamps the -1 exactly to 0 */
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
    {   /* without the floor the same removes go negative */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        SynthEffect s; synth(&s, "live_total", 0);
        synth_set(&s, 0, "operation", "remove");
        synth_set(&s, 1, "value", "1");
        rb_execute_modify_score(&tg.state, 0, &s.e);
        rb_execute_modify_score(&tg.state, 0, &s.e);
        CHECK_EQ(live_total_bonus(&tg, 0), -2,
                 "without score_floor a live_total remove is free to go negative");
    }
    {   /* a live base total lets the -1 through unclamped */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        int l = test_id(&tg, "PL!S-pb1-021-L");   /* printed score 1 */
        test_add_to_live(&tg, l);
        SynthEffect s; synth(&s, "live_total", 0);
        synth_set(&s, 0, "operation", "remove");
        synth_set(&s, 1, "value", "1");
        synth_set(&s, 2, "score_floor", "0");
        rb_execute_modify_score(&tg.state, 0, &s.e);
        CHECK_EQ(live_total_bonus(&tg, 0), -1,
                 "score_floor clamps against base_total + bonus, and a score-1 live keeps -1 legal");
    }
    rb_free_ability(&ab);
}

/* ===================================================================== */
/* G. per-card floor — score.rs:327-336                                   */
/* ===================================================================== */

static void test_per_card_floor(void)
{
    TestGame tg; test_game_new(&tg); bare_board(&tg);
    int l1 = test_id(&tg, "PL!-sd1-020-SD");
    test_add_to_live(&tg, l1);
    tg.state.activating_card = l1;

    SynthEffect f; synth(&f, "self", 0);
    synth_set(&f, 0, "operation", "remove");
    synth_set(&f, 1, "value", "1");
    synth_set(&f, 2, "effect_constraint", "min:0");
    rb_execute_modify_score(&tg.state, 0, &f.e);
    CHECK_EQ(score_of(&tg, l1), 0, "min:0 blocks a remove that would drive the modifier below 0");

    SynthEffect a; synth(&a, "self", 0);
    synth_set(&a, 0, "operation", "add");
    synth_set(&a, 1, "value", "3");
    rb_execute_modify_score(&tg.state, 0, &a.e);
    CHECK_EQ(score_of(&tg, l1), 3, "add 3 raises the modifier above the floor");

    rb_execute_modify_score(&tg.state, 0, &f.e);
    CHECK_EQ(score_of(&tg, l1), 2, "min:0 allows a remove that still lands at 2");

    {   /* score_floor is an equally valid spelling of the same floor */
        TestGame tg2; test_game_new(&tg2); bare_board(&tg2);
        int x = test_id(&tg2, "PL!-sd1-020-SD");
        test_add_to_live(&tg2, x);
        tg2.state.activating_card = x;
        SynthEffect g; synth(&g, "self", 0);
        synth_set(&g, 0, "operation", "remove");
        synth_set(&g, 1, "value", "1");
        synth_set(&g, 2, "score_floor", "0");
        rb_execute_modify_score(&tg2.state, 0, &g.e);
        CHECK_EQ(score_of(&tg2, x), 0, "score_floor=0 blocks the same remove as effect_constraint=min:0");
    }
    {   /* without a floor the remove is allowed through */
        TestGame tg2; test_game_new(&tg2); bare_board(&tg2);
        int x = test_id(&tg2, "PL!-sd1-020-SD");
        test_add_to_live(&tg2, x);
        tg2.state.activating_card = x;
        SynthEffect g; synth(&g, "self", 0);
        synth_set(&g, 0, "operation", "remove");
        synth_set(&g, 1, "value", "1");
        rb_execute_modify_score(&tg2.state, 0, &g.e);
        CHECK_EQ(score_of(&tg2, x), -1, "without any floor a remove may push the modifier negative");
    }
}

/* ===================================================================== */
/* H. target resolution — score.rs:186, abilities.rs:2527-2545             */
/* ===================================================================== */

static void test_target_resolution(void)
{
    SynthEffect base; synth(&base, "self", 0);
    synth_set(&base, 0, "value", "2");
    synth_set(&base, 1, "operation", "add");

    {   /* P1 acts, P2's live card is the recipient */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        int p1l = test_id(&tg, "PL!-sd1-020-SD");
        int p2l = test_new_id(&tg, "PL!-sd1-020-SD");
        test_add_to_live(&tg, p1l);
        test_add_to_opp_live(&tg, p2l);
        tg.state.activating_card = p1l;
        SynthEffect s = base; s.e.target = (char *)"opponent";
        rb_execute_modify_score(&tg.state, 0, &s.e);
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
        SynthEffect s = base; s.e.target = (char *)"opponent";
        rb_execute_modify_score(&tg.state, 1, &s.e);
        CHECK_EQ(score_of(&tg, p1l), 2, "target=opponent scores P1's live card when P2 acts");
        CHECK_EQ(score_of(&tg, p2l), 0, "target=opponent leaves P2's own live card alone");
    }
    {   /* self follows the acting player */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        int p1l = test_id(&tg, "PL!-sd1-020-SD");
        int p2l = test_new_id(&tg, "PL!-sd1-020-SD");
        test_add_to_live(&tg, p1l);
        test_add_to_opp_live(&tg, p2l);
        tg.state.activating_card = p2l;
        rb_execute_modify_score(&tg.state, 1, &base.e);
        CHECK_EQ(score_of(&tg, p2l), 2, "target=self scores the acting player's own live card");
        CHECK_EQ(score_of(&tg, p1l), 0, "target=self does not reach the opponent's board");
    }
    {   /* a NULL target defaults to self (card.rs:2336 target_name) */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        int p1l = test_id(&tg, "PL!-sd1-020-SD");
        int p2l = test_new_id(&tg, "PL!-sd1-020-SD");
        test_add_to_live(&tg, p1l);
        test_add_to_opp_live(&tg, p2l);
        SynthEffect s = base; s.e.target = NULL;
        rb_execute_modify_score(&tg.state, 0, &s.e);
        CHECK_EQ(score_of(&tg, p1l), 2, "a NULL target falls back to the acting player");
        CHECK_EQ(score_of(&tg, p2l), 0, "a NULL target does not fall through to the opponent");
    }
}

/* ===================================================================== */
/* I. per_unit_count divisor, repeat_limit cap, exclude_self              */
/*    score.rs:92-97, :243-248, :46-50                                   */
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
    SynthEffect e; synth(&e, "self", 0);
    synth_set(&e, 0, "value", "1");
    synth_set(&e, 1, "operation", "add");
    synth_set(&e, 2, "per_unit", "true");
    synth_set(&e, 3, "per_unit_type", "人");
    synth_set(&e, 4, "per_unit_count", "2");

    rb_execute_modify_score(&tg.state, 0, &e.e);
    CHECK_EQ(score_of(&tg, live), 1, "3 members / per_unit_count 2 -> 1 unit -> +1");

    e.e.repeat_limit = 1;
    rb_execute_modify_score(&tg.state, 0, &e.e);
    CHECK_EQ(score_of(&tg, live), 2, "repeat_limit=1 caps the second resolution at +1");

    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    e.e.repeat_limit = 0;
    rb_execute_modify_score(&tg.state, 0, &e.e);
    CHECK_EQ(score_of(&tg, live), 3, "dropping to 2 members still yields exactly 1 unit");

    /* per_unit_count=1 scales one-for-one. */
    {
        TestGame tg2; test_game_new(&tg2); bare_board(&tg2);
        int lv = test_id(&tg2, "PL!-sd1-020-SD");
        test_add_to_live(&tg2, lv);
        tg2.state.activating_card = lv;
        tg2.state.p[0].stage[0] = m1;
        tg2.state.p[0].stage[1] = m2;
        SynthEffect s; synth(&s, "self", 0);
        synth_set(&s, 0, "value", "1");
        synth_set(&s, 1, "operation", "add");
        synth_set(&s, 2, "per_unit", "true");
        synth_set(&s, 3, "per_unit_type", "人");
        synth_set(&s, 4, "per_unit_count", "1");
        rb_execute_modify_score(&tg2.state, 0, &s.e);
        CHECK_EQ(score_of(&tg2, lv), 2, "per_unit_count=1 gives one unit per member -> +2");
    }
    /* exclude_self drops the activating member from the per-unit count. */
    {
        TestGame tg2; test_game_new(&tg2); bare_board(&tg2);
        int member = test_id(&tg2, "PL!SP-sd1-001-SD");
        int lv = test_id(&tg2, "PL!-sd1-020-SD");
        test_add_to_live(&tg2, lv);
        tg2.state.p[0].stage[0] = member;
        tg2.state.p[0].stage[1] = test_new_id(&tg2, "PL!SP-sd1-001-SD");
        tg2.state.activating_card = member;
        SynthEffect s; synth(&s, "self", 0);
        synth_set(&s, 0, "value", "1");
        synth_set(&s, 1, "operation", "add");
        synth_set(&s, 2, "per_unit", "true");
        synth_set(&s, 3, "per_unit_type", "人");
        synth_set(&s, 4, "exclude_self", "true");
        rb_execute_modify_score(&tg2.state, 0, &s.e);
        CHECK_EQ(score_of(&tg2, lv), 1,
                 "exclude_self drops the activating member, leaving 1 of 2 members -> +1");
    }
}

/* ===================================================================== */
/* J. conditional activation — a score effect gated by a decoded condition */
/*    card_score/live_start_center_cost_comparison_live_score_test.rs      */
/* ===================================================================== */

static void test_conditional_activation(void)
{
    /* ノンフィクション!! PL!SP-bp4-024-L ab#0: self_target +1, condition =
       own center Liella! member's cost is higher than the opponent's center
       member's cost. score.c does NOT evaluate the condition (score.rs does
       not either — the resolver gates it), so these assertions prove the
       condition is decoded AND that the engine's evaluator agrees with the
       Rust expectations. */
    int nonfic = rb_find_card_by_no("PL!SP-bp4-024-L");
    int kanon = rb_find_card_by_no("PL!SP-pb1-001-R");   /* Liella! cost 11 */
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");  /* cost 4, not Liella! */
    Ability ab;
    AbilityEffect *e = ability_effect(nonfic, 0, &ab);
    if (!e) { CHECK(0, "PL!SP-bp4-024-L ab#0 decodes"); return; }
    CHECK(e->has_condition && e->condition != NULL,
          "ノンフィクション's score effect carries a decoded condition");
    CHECK(eff_extra(e, "position") && !strcmp(eff_extra(e, "position"), "center"),
          "ノンフィクション's condition is position=center");

    {   /* P1 cost 11 > P2 cost 4 -> the condition holds */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, nonfic);
        test_add_to_hand(&tg, filler);
        tg.state.activating_card = nonfic;
        tg.state.p[0].stage[1] = kanon;
        tg.state.p[1].stage[1] = filler;
        CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, nonfic, e->condition), 1,
                 "center cost 11 > opponent center cost 4 -> condition holds");
    }
    {   /* P1 cost 4 < P2 cost 11 -> the condition fails */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, nonfic);
        test_add_to_hand(&tg, filler);
        tg.state.activating_card = nonfic;
        tg.state.p[0].stage[1] = filler;
        tg.state.p[1].stage[1] = kanon;
        CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, nonfic, e->condition), 0,
                 "center cost 4 < opponent center cost 11 -> condition fails");
    }
    {   /* P1 center is not Liella! -> fails even with a higher cost */
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, nonfic);
        test_add_to_hand(&tg, filler);
        tg.state.activating_card = nonfic;
        tg.state.p[0].stage[1] = test_new_id(&tg, "PL!N-sd1-002-SD"); /* cost 9, 虹ヶ咲 */
        tg.state.p[1].stage[1] = filler;
        CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, nonfic, e->condition), 0,
                 "a non-Liella! center member fails the group half of the condition");
    }
    rb_free_ability(&ab);

    /* Let's be ONE PL!SP-bp5-026-L: +1 when the stage's Liella! members hold
       11+ hearts in total (card_score/liella_heart_total_score_test.rs). */
    int one = rb_find_card_by_no("PL!SP-bp5-026-L");
    Ability ab2;
    AbilityEffect *e2 = ability_effect(one, 0, &ab2);
    if (!e2) { CHECK(0, "PL!SP-bp5-026-L ab#0 decodes"); return; }
    {
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, one);
        test_add_to_hand(&tg, test_id(&tg, "PL!-sd1-010-SD"));
        tg.state.activating_card = one;
        tg.state.p[0].stage[0] = test_id(&tg, "PL!SP-pb2-005-R");  /* 9 hearts */
        tg.state.p[0].stage[1] = test_id(&tg, "PL!SP-bp4-004-P");  /* 8 hearts */
        CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, one, e2->condition), 1,
                 "two Liella! members holding 17 hearts in total >= 11 -> condition holds");
        rb_execute_modify_score(&tg.state, 0, e2);
        CHECK_EQ(score_of(&tg, one), 1, "Let's be ONE then scores +1 on its own card");
    }
    {
        TestGame tg; test_game_new(&tg); bare_board(&tg);
        test_add_to_live(&tg, one);
        test_add_to_hand(&tg, test_id(&tg, "PL!-sd1-010-SD"));
        tg.state.activating_card = one;
        tg.state.p[0].stage[0] = test_id(&tg, "PL!SP-pb2-036-N");
        tg.state.p[0].stage[1] = test_id(&tg, "PL!SP-pb2-037-N");
        CHECK_EQ(rb_eval_condition_for_host(&tg.state, 0, one, e2->condition), 0,
                 "two low-heart Liella! members total 4 < 11 -> condition fails");
    }
    rb_free_ability(&ab2);
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
