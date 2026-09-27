#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { fprintf(stderr, "FAIL: %s\n", message); failures++; } \
    else printf("ok: %s\n", message); \
} while (0)
#define CHECK_EQ(actual, expected, message) do { \
    int actual_value = (actual); int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else printf("ok: %s\n", message); \
} while (0)

static void test_mandatory_selection_takes_count(void) {
    TestGame game;
    test_game_new(&game);
    int member = test_new_id(&game, "PL!-sd1-010-SD");
    int other = test_new_id(&game, "PL!-sd1-002-SD");
    test_add_to_hand(&game, member);
    test_add_to_hand(&game, other);
    AbilityEffect effect = {0};
    effect.action = "move_cards";
    effect.source = "hand";
    effect.destination = "discard";
    effect.count = 1;
    rb_execute_effect_ex(&game.state, 0, &effect, -1);
    CHECK(!test_has_pending_choice(&game), "mandatory move does not prompt for a partial zone");
    CHECK(!test_hand_has(&game, member) && test_zone_has_id(&game, 0, "discard", member),
          "mandatory move takes exactly one hand card");
    CHECK(test_hand_has(&game, other), "mandatory move leaves the remainder in hand");
}

static void test_deck_bottom_and_top_placement(void) {
    TestGame game;
    test_game_new(&game);
    int member = test_new_id(&game, "PL!-sd1-010-SD");
    int second = test_new_id(&game, "PL!-sd1-002-SD");
    test_add_to_hand(&game, member);
    AbilityEffect effect = {0};
    effect.action = "move_cards";
    effect.source = "hand";
    effect.destination = "deck_bottom";
    effect.count = 1;
    rb_execute_effect_ex(&game.state, 0, &effect, -1);
    CHECK(test_zone_has_id(&game, 0, "deck", member), "deck bottom receives the moved card");
    CHECK_EQ(game.state.p[0].deck.n, 1, "deck bottom placement conserves card count");
    test_add_to_hand(&game, second);
    effect.destination = "deck_top";
    rb_execute_effect_ex(&game.state, 0, &effect, -1);
    CHECK(game.state.p[0].deck.n == 2 && game.state.p[0].deck.cards[0] == second,
          "deck top placement inserts at index zero");
}

static void test_under_member_single_host(void) {
    TestGame game;
    test_game_new(&game);
    int host = test_new_id(&game, "PL!-sd1-010-SD");
    int energy = test_new_id(&game, "LL-E-001-SD");
    test_add_to_stage(&game, 0, host);
    test_add_to_hand(&game, energy);
    AbilityEffect effect = {0};
    effect.action = "move_cards";
    effect.source = "hand";
    effect.destination = "under_member";
    effect.count = 1;
    rb_execute_effect_ex(&game.state, 0, &effect, -1);
    CHECK(!test_has_pending_choice(&game), "single host under-member placement is automatic");
    CHECK(game.state.p[0].under_cards[0].n == 1 &&
              game.state.p[0].under_cards[0].cards[0] == energy,
          "card is placed under the member");
}

static void test_success_replacement_move(void) {
    TestGame game;
    test_game_new(&game);
    int original = test_new_id(&game, "PL!-bp6-024-L");
    /* Rust fixture: engine/tests/test_modules/effects/conditional/zone_source/
       live_phase_success_zone_replacement_q256_test.rs:19 — the substitute is
       「僕らのLIVE 君とのLIFE」, the μ's live. PL!HS-bp1-019-L is 蓮ノ空
       (Dream Believers) and must NOT satisfy a 「『μ's』の…」 group filter
       (engine/src/ability/util.rs:602-637), so it is not a legal replacement. */
    int replacement = test_new_id(&game, "PL!-bp3-019-L");
    test_add_to_hand(&game, original);
    test_add_to_discard(&game, replacement);
    AbilityEffect effect = {0};
    effect.action = "move_cards";
    effect.source = "hand";
    effect.destination = "success_live_zone";
    effect.count = 1;
    rb_execute_effect_ex(&game.state, 0, &effect, -1);
    CHECK(test_has_pending_choice(&game), "success-zone move prompts for replacement");
    CHECK(rb_get_pending_choice(&game.state) &&
              rb_get_pending_choice(&game.state)->n_filtered_indices == 1,
          "success replacement filters legal waitroom lives");
    test_resume_choice(&game, 0);
    CHECK(test_zone_has_id(&game, 0, "success", replacement),
          "selected replacement reaches success zone");
    CHECK(test_zone_has_id(&game, 0, "discard", original),
          "replaced original returns to waitroom");
}

static void test_looked_at_remainder(void) {
    TestGame game;
    test_game_new(&game);
    int member = test_new_id(&game, "PL!-sd1-010-SD");
    int live = test_new_id(&game, "PL!N-bp1-027-L");
    rb_look_add(0, member);
    rb_look_add(0, live);
    AbilityEffect effect = {0};
    effect.action = "move_cards";
    effect.source = "looked_at";
    effect.destination = "hand";
    effect.count = 1;
    strcpy(effect.card_type_field, "member_card");
    rb_execute_effect_ex(&game.state, 0, &effect, -1);
    CHECK(test_zone_has_id(&game, 0, "hand", member), "looked-at selection moves the matching card");
    int pool[8];
    CHECK_EQ(rb_looked_at_pool(0, pool, 8), 1, "unselected looked-at card remains in the pool");
    effect.source = "looked_at_remaining";
    effect.destination = "discard";
    effect.count = -1;
    effect.card_type_field[0] = '\0';
    rb_execute_effect_ex(&game.state, 0, &effect, -1);
    CHECK(test_zone_has_id(&game, 0, "discard", live), "looked-at remainder uses requested destination");
    CHECK_EQ(rb_looked_at_pool(0, pool, 8), 0, "looked-at remainder is consumed exactly once");
}

int main(void) {
    if (rb_load("src") != 0) return 1;
    test_mandatory_selection_takes_count();
    test_deck_bottom_and_top_placement();
    test_under_member_single_host();
    test_success_replacement_move();
    test_looked_at_remainder();
    rb_unload();
    if (failures) return 1;
    printf("ALL MOVE CARD CHECKS PASSED\n");
    return 0;
}
