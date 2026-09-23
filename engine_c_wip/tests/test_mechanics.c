#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>

static int failures;

static void drain_all_choices(TestGame *tg) {
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 100) {
        test_resume_choice(tg, 0);
    }
}



#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

static void test_gained_live_total_bonus(void) {
    TestGame tg;
    test_game_new(&tg);
    int target = test_id(&tg, "PL!SP-bp5-222-R");
    CHECK(target >= 0, "live-total test card exists");

    tg.state.p[0].stage[0] = target;
    tg.state.activating_card = target;

    AbilityEffect inner = {0};
    inner.action = "modify_score";
    inner.target = "live_total";
    inner.count = 1;

    AbilityEffect outer = {0};
    outer.action = "gain_ability";
    outer.text = "live total +1";
    outer.gained_effect = &inner;
    outer.extra_k[0] = "ability_gain";
    outer.extra_v[0] = "live total +1";
    outer.extra_k[1] = "ability_gain_trigger";
    outer.extra_v[1] = "常時";
    outer.extra_k[2] = "duration";
    outer.extra_v[2] = "live_end";
    outer.n_extra = 3;

    rb_gain_ability(&tg.state, 0, &outer);
    CHECK(tg.state.mods.p1_constant_total_score_bonus == 1,
          "gained constant live-total bonus is applied");
    CHECK(rb_card_num_gained_abilities(&tg.state, target) == 1,
          "gained ability is stored");

    rb_check_expired_effects(&tg.state, RB_TEMP_LIVE_END);
    CHECK(tg.state.mods.p1_constant_total_score_bonus == 0,
          "expired gained live-total bonus is removed");
    CHECK(rb_card_num_gained_abilities(&tg.state, target) == 0,
          "expired gained ability is removed");
}

static void test_direct_live_total_bonus(void) {
    TestGame tg;
    test_game_new(&tg);
    AbilityEffect effect = {0};
    effect.action = "modify_score";
    effect.target = "live_total";
    effect.count = 2;

    rb_execute_modify_score(&tg.state, 0, &effect);
    CHECK(tg.state.mods.p1_constant_total_score_bonus == 2,
          "direct live-total effect updates the player bonus");
}

static void test_live_score_consumes_bonus(void) {
    TestGame tg;
    test_game_new(&tg);
    int live = test_id(&tg, "PL!N-sd2-025-P");
    CHECK(live >= 0, "live score test card exists");
    if (live < 0) return;

    test_add_to_live(&tg, live);
    int provided[8] = {7, 7, 7, 7, 7, 7, 7, 7};
    Card card = {0};
    int decoded = rb_decode_card_by_index((uint32_t)live, &card);
    CHECK(decoded != 0, "live score test card decodes");
    if (decoded) {
        int expected = card.score + 3;
        int actual = rb_live_calculate_score(&tg.state, 0, 0, provided, 3);
        CHECK(actual == expected, "live score includes the constant total bonus");
        rb_free_card(&card);
    }
}

static void test_invalid_live_movement(void) {
    TestGame tg;
    test_game_new(&tg);
    int member = test_id(&tg, "PL!-sd1-010-SD");
    test_add_to_live(&tg, member);
    test_add_to_stage(&tg, 0, member);
    for (int i = 0; i < 30; i++) {
        test_add_to_deck(&tg, member);
        test_add_to_deck_pl(&tg, 1, member);
    }
    rb_check_timing(&tg.state);
    CHECK(!test_zone_has_id(&tg, 0, "live", member),
          "invalid live member leaves the live zone");
    CHECK(test_zone_has_id(&tg, 0, "discard", member),
          "invalid live member goes to the waitroom");
    CHECK(tg.state.moved_this_turn[member],
          "invalid live movement records the card movement");
}

static void test_invalid_energy_movement(void) {
    TestGame tg;
    test_game_new(&tg);
    int energy = test_id(&tg, "LL-E-001-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    test_add_to_live(&tg, energy);
    test_add_to_stage(&tg, 0, filler);
    for (int i = 0; i < 30; i++) {
        test_add_to_deck(&tg, filler);
        test_add_to_deck_pl(&tg, 1, filler);
    }
    rb_check_timing(&tg.state);
    CHECK(!test_zone_has_id(&tg, 0, "live", energy),
          "invalid energy leaves the live zone");
    int found = 0;
    for (int i = 0; i < tg.state.p[0].energy_deck.n; i++) {
        if (tg.state.p[0].energy_deck.cards[i] == energy) found = 1;
    }
    CHECK(found, "invalid live energy returns to the energy deck");
}

static void test_movement_tracking_clear(void) {
    TestGame tg;
    test_game_new(&tg);
    int card = test_id(&tg, "PL!-sd1-010-SD");
    tg.state.moved_this_turn[card] = 1;
    CHECK(tg.state.moved_this_turn[card], "movement tracking records a card");
    rb_clear_movement_tracking(&tg.state);
    CHECK(!tg.state.moved_this_turn[card], "movement tracking clears at turn start");
}

static void test_performance_snapshot(void) {
    TestGame tg;
    test_game_new(&tg);
    int live = test_id(&tg, "PL!-bp3-026-L");
    int center = test_id(&tg, "PL!-pb1-014-R");
    int right = test_id(&tg, "PL!-PR-003-PR");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    CHECK(live >= 0 && center >= 0 && right >= 0 && filler >= 0,
          "performance pipeline fixtures exist");
    if (live < 0 || center < 0 || right < 0 || filler < 0) return;
    test_add_to_stage(&tg, 1, center);
    test_add_to_stage(&tg, 2, right);
    for (int i = 0; i < 40; i++) {
        test_add_to_deck(&tg, filler);
        test_add_to_deck_pl(&tg, 1, filler);
    }
    test_add_to_live(&tg, live);
    rb_perform_live(&tg.state, 0);
    CHECK(tg.state.n_snapshots == 1, "performance creates one snapshot");
    if (tg.state.n_snapshots != 1) return;
    RbLiveSnapshot snapshot = tg.state.snapshots[0];
    int member_hearts = 0;
    for (int i = 0; i < 8; i++) member_hearts += snapshot.total_hearts[i];
    int yell_hearts = 0;
    for (int i = 0; i < 8; i++) yell_hearts += snapshot.yell_blade_hearts[i];
    CHECK(member_hearts + yell_hearts > 0, "snapshot records performance hearts");
    CHECK(snapshot.total_score > 0, "snapshot records a positive score");
    CHECK(snapshot.n_lives == 1 && snapshot.lives[0] == live,
          "snapshot records the performed live card");
}
static void test_live_failure_moves_to_waitroom(void) {
    TestGame tg;
    test_game_new(&tg);
    tg.state.live_batch_mode = 1;
    int live = test_id(&tg, "PL!N-sd1-025-SD");
    int member = test_id(&tg, "PL!S-sd1-003-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    CHECK(live >= 0 && member >= 0 && filler >= 0,
          "live failure fixtures exist");
    if (live < 0 || member < 0 || filler < 0) return;
    test_add_to_live(&tg, live);
    rb_perform_live(&tg.state, 0);
    rb_execute_live_victory_determination(&tg.state);
    drain_all_choices(&tg);
    rb_execute_live_victory_determination(&tg.state);
    CHECK(!test_zone_has_id(&tg, 0, "success", live),
          "insufficient hearts do not enter success");
    CHECK(test_zone_has_id(&tg, 0, "discard", live),
          "failed live enters the waitroom");
}

static void test_both_lives_compare_scores(void) {
    TestGame tg;
    test_game_new(&tg);
    tg.state.live_batch_mode = 1;
    int live = test_id(&tg, "PL!N-sd1-025-SD");
    int member = test_id(&tg, "PL!S-sd1-001-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    CHECK(live >= 0 && member >= 0 && filler >= 0,
          "two-live fixtures exist");
    if (live < 0 || member < 0 || filler < 0) return;
    test_add_to_stage(&tg, 0, member);
    test_set_opp_stage(&tg, 0, member);
    for (int i = 0; i < 30; i++) {
        test_add_to_deck(&tg, filler);
        test_add_to_deck_pl(&tg, 1, filler);
    }
    test_add_to_live(&tg, live);
    test_add_to_opp_live(&tg, live);
    rb_perform_live(&tg.state, 0);
    rb_perform_live(&tg.state, 1);
    rb_execute_live_victory_determination(&tg.state);
    drain_all_choices(&tg);
    rb_execute_live_victory_determination(&tg.state);
    CHECK(test_zone_has_id(&tg, 0, "success", live),
          "first attacker places the tied live");
    CHECK(test_zone_has_id(&tg, 1, "success", live),
          "second attacker also places the tied live");
}

static void test_single_live_auto_wins(void) {
    TestGame tg;
    test_game_new(&tg);
    tg.state.live_batch_mode = 1;
    int live = test_id(&tg, "PL!N-sd1-025-SD");
    int member = test_id(&tg, "PL!S-sd1-001-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    CHECK(live >= 0 && member >= 0 && filler >= 0,
          "single-live fixtures exist");
    if (live < 0 || member < 0 || filler < 0) return;
    test_add_to_stage(&tg, 0, member);
    for (int i = 0; i < 30; i++) test_add_to_deck(&tg, filler);
    test_add_to_live(&tg, live);
    rb_perform_live(&tg.state, 0);
    rb_execute_live_victory_determination(&tg.state);
    drain_all_choices(&tg);
    rb_execute_live_victory_determination(&tg.state);
    CHECK(test_zone_has_id(&tg, 0, "success", live),
          "unopposed live is placed");
    CHECK(!test_zone_has_id(&tg, 1, "success", live),
          "unopposed live leaves the opponent empty");
}
int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_gained_live_total_bonus();
    test_direct_live_total_bonus();
    test_live_score_consumes_bonus();
    test_invalid_live_movement();
    test_invalid_energy_movement();
    test_movement_tracking_clear();
    test_performance_snapshot();
    test_live_failure_moves_to_waitroom();
    test_both_lives_compare_scores();
    test_single_live_auto_wins();
    rb_unload();
    if (failures) return 1;
    printf("ALL MECHANIC CHECKS PASSED\n");
    return 0;
}
