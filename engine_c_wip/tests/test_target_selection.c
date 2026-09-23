#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ_STR(actual, expected, message) do { \
    const char *actual_value = (actual); \
    const char *expected_value = (expected); \
    if (strcmp(actual_value, expected_value) != 0) { \
        fprintf(stderr, "FAIL: %s (got %s expected %s)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

static void process_abilities(TestGame *tg)
{
    rb_trigger_live_start(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
    while (test_has_pending_choice(tg)) {
        int indices[] = {0};
        test_select_indices(tg, indices, 1);
        rb_process_pending_auto_abilities(&tg->state);
    }
}

static void setup_live_phase_with_hearts(TestGame *tg)
{
    tg->state.phase = RB_PHASE_LIVE_SET;
    tg->state.stage_hearts[0][0] = 7;
    tg->state.stage_hearts[0][1] = 2;
    tg->state.stage_hearts[0][5] = 6;
    tg->state.stage_hearts[0][7] = 10;
}

static void target_count_1_gain_resource_chooses_one_of_many(void)
{
    TestGame tg;
    test_game_new(&tg);
    int member_a = test_id(&tg, "PL!N-PR-003-PR");
    int member_b = test_id(&tg, "PL!N-PR-005-PR");
    int phoenix = test_id(&tg, "PL!N-pb1-038-L");
    int stellar = test_id(&tg, "PL!N-pb1-039-L");
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    tg.state.p[0].stage[1] = member_a;
    tg.state.p[0].stage[0] = member_b;
    test_add_to_live(&tg, phoenix);
    test_add_to_live(&tg, stellar);
    setup_live_phase_with_hearts(&tg);
    process_abilities(&tg);
    int a_heart06 = test_get_heart_modifier(&tg, member_a, 5);
    int b_heart06 = test_get_heart_modifier(&tg, member_b, 5);
    CHECK_EQ(a_heart06 + b_heart06, 4,
             "Total +4 heart06 across both members");
    CHECK(a_heart06 == 4 || b_heart06 == 4,
          "Exactly one member got the full +4 heart06 buff");
}

static void distinct_card_name_prevents_same_card_twice(void)
{
    TestGame tg;
    test_game_new(&tg);
    int live_card = test_id(&tg, "PL!SP-bp1-026-L");
    test_add_to_live(&tg, live_card);
    tg.state.stage_hearts[0][0] = 5;
    tg.state.stage_hearts[0][7] = 5;
    tg.state.phase = RB_PHASE_LIVE_SET;
    process_abilities(&tg);
    int h02 = rb_mods_get_need_heart(&tg.state.mods, live_card, 1);
    if (h02 != 0) {
        fprintf(stderr,
                "FAIL: condition unmet: required hearts must be unchanged, got modifier %d\n",
                h02);
        failures++;
    } else {
        printf("ok: condition unmet: required hearts must be unchanged, got modifier 0\n");
    }
}

static void target_count_on_draw_until_count(void)
{
    TestGame tg;
    test_game_new(&tg);
    int card = test_id(&tg, "PL!N-PR-028-PR");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    tg.state.p[0].hand.n = 0;
    test_add_to_hand(&tg, card);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 30);
    for (int i = 0; i < 10; i++) {
        test_add_to_deck(&tg, filler);
    }
    test_play_to_stage(&tg, card, 1);
    CHECK(test_has_pending_choice(&tg), "optional discard-2 cost prompt expected");
    CHECK_EQ_STR(test_pending_choice_type(&tg), "SelectCard",
                 "expected SelectCard (hand, count=2, allow_skip)");
    rb_resume_with_choice(&tg.state, 0);
    rb_drain_ability_queue(&tg.state);
    if (test_has_pending_choice(&tg)) {
        rb_resume_with_choice(&tg.state, 1);
        rb_drain_ability_queue(&tg.state);
    }
    while (test_has_pending_choice(&tg)) {
        rb_resume_with_choice(&tg.state, -1);
    }
    CHECK_EQ(tg.state.p[0].hand.n, 5,
             "draw_until_count must fill the hand to target_count=5");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    target_count_1_gain_resource_chooses_one_of_many();
    distinct_card_name_prevents_same_card_twice();
    target_count_on_draw_until_count();
    rb_unload();
    if (failures) return 1;
    printf("ALL TARGET SELECTION CHECKS PASSED\n");
    return 0;
}
