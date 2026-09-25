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

static void test_gained_ability_target_and_duration(void)
{
    TestGame game;
    test_game_new(&game);
    int source = test_id(&game, "PL!-sd1-010-SD");
    int target = test_new_id(&game, "PL!-sd1-010-SD");
    test_add_to_stage(&game, 0, source);
    game.state.p[1].stage[0] = target;

    AbilityEffect effect;
    AbilityEffect inner;
    memset(&effect, 0, sizeof(effect));
    memset(&inner, 0, sizeof(inner));
    effect.action = (char *)"gain_ability";
    effect.target = (char *)"opponent";
    effect.text = (char *)"gained score";
    effect.count = 2;
    effect.gained_effect = &inner;
    effect.n_extra = 3;
    effect.extra_k[0] = (char *)"ability_gain";
    effect.extra_v[0] = (char *)"gained score";
    effect.extra_k[1] = (char *)"ability_gain_trigger";
    effect.extra_v[1] = (char *)"常時";
    effect.extra_k[2] = (char *)"duration";
    effect.extra_v[2] = (char *)"until_end_of_turn";
    inner.action = (char *)"modify_score";
    inner.target = (char *)"self";
    inner.count = 2;

    game.state.activating_card = source;
    game.state.n_selected_cards = 1;
    game.state.selected_cards[0] = target;
    CHECK(rb_translated_execute_gain_ability_effect(&game.state, 0, &effect, source),
          "gain ability resolves a selected target");
    CHECK(rb_card_num_gained_abilities(&game.state, target) == 1,
          "gained ability is stored on the selected target");
    CHECK(rb_card_num_gained_abilities(&game.state, source) == 0,
          "gained ability is not attributed to the source owner");
    CHECK(test_get_score_modifier(&game, target) == 4,
          "gained constant score is applied through the constant landscape");
    rb_check_expired_effects(&game.state, RB_TEMP_TURN_END);
    CHECK(rb_card_num_gained_abilities(&game.state, target) == 0,
          "until_end_of_turn removes the gained ability");
    CHECK(test_get_score_modifier(&game, target) == 0,
          "duration expiry reverts the gained score");
}

static void test_activate_ability_stores_trigger_context(void)
{
    TestGame game;
    test_game_new(&game);
    int card = test_id(&game, "PL!-bp3-005-R");
    AbilityEffect effect;
    memset(&effect, 0, sizeof(effect));
    effect.action = (char *)"activate_ability";
    effect.n_extra = 2;
    effect.extra_k[0] = (char *)"source_card";
    effect.extra_v[0] = (char *)"previous_selected";
    effect.extra_k[1] = (char *)"target_trigger";
    effect.extra_v[1] = (char *)"登場";
    game.state.n_selected_cards = 1;
    game.state.selected_cards[0] = card;
    game.state.activating_card = card;

    CHECK(rb_translated_execute_activate_ability(&game.state, 0, &effect, card),
          "activation queues the matching ability");
    CHECK(game.state.queue.n_entries == 1, "activation queues one ability");
    CHECK(strcmp(game.state.queue.entries[0].trigger, "登場") == 0,
          "queue entry stores the activated trigger");
}

static void test_gain_trigger_is_scanned_and_suppressible(void)
{
    TestGame game;
    test_game_new(&game);
    int source = test_id(&game, "PL!-sd1-010-SD");
    int target = test_new_id(&game, "PL!-sd1-010-SD");
    int live = test_id(&game, "PL!-sd1-020-SD");
    test_add_to_stage(&game, 0, target);
    game.state.p[0].live.cards[0] = live;
    game.state.p[0].live.n = 1;
    game.state.live_success[0] = 1;

    Ability gained;
    AbilityEffect inner;
    memset(&gained, 0, sizeof(gained));
    memset(&inner, 0, sizeof(inner));
    gained.triggers = (char *)"ライブ成功時";
    gained.full_text = (char *)"gained live success";
    gained.triggerless_text = gained.full_text;
    gained.use_limit = -1;
    inner.action = (char *)"draw_card";
    inner.count = 1;
    gained.effect = &inner;
    rb_register_gained_ability(&game.state, target, &gained);

    AbilityEffect suppression;
    memset(&suppression, 0, sizeof(suppression));
    suppression.n_extra = 1;
    suppression.extra_k[0] = (char *)"suppressed_trigger";
    suppression.extra_v[0] = (char *)"live_success";
    rb_suppress_ability_trigger(&game.state, 0, &suppression, source);
    CHECK(rb_trigger_live_success(&game.state, 0) == 0,
          "suppression blocks gained live-success triggers");
    CHECK(rb_is_trigger_suppressed(&game.state, 0, "live_success"),
          "suppression wiring records the trigger");
    CHECK(!rb_is_trigger_suppressed(&game.state, 1, "live_success"),
          "suppression remains player-specific");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    test_gained_ability_target_and_duration();
    test_activate_ability_stores_trigger_context();
    test_gain_trigger_is_scanned_and_suppressible();
    rb_unload();
    if (failures) {
        fprintf(stderr, "%d ability effect failures\n", failures);
        return 1;
    }
    printf("ALL ABILITY EFFECT CHECKS PASSED\n");
    return 0;
}
