#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>

static int failures;

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

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_gained_live_total_bonus();
    test_direct_live_total_bonus();
    test_live_score_consumes_bonus();
    rb_unload();
    if (failures) return 1;
    printf("ALL MECHANIC CHECKS PASSED\n");
    return 0;
}
