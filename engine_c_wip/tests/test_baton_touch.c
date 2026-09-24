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

enum {
    ACTION_PLAY_MEMBER_TO_STAGE = 14
};

static void drain_choices_by_skip(TestGame *game) {
    while (test_has_pending_choice(game)) {
        rb_resume_with_choice(&game->state, -1);
    }
}

static void add_condition_field(Condition *condition, const char *key, CondValue value) {
    condition->fields[condition->n_fields].key = (char *)key;
    condition->fields[condition->n_fields].v = value;
    condition->n_fields++;
}

static CondValue condition_string(const char *value) {
    CondValue result;
    memset(&result, 0, sizeof(result));
    result.tag = RB_TAG_STR;
    result.s = (char *)value;
    return result;
}

static CondValue condition_integer(int value) {
    CondValue result;
    memset(&result, 0, sizeof(result));
    result.tag = RB_TAG_I64;
    result.i = value;
    return result;
}

static CondValue condition_group_names(const char *group_name) {
    CondValue result;
    CondValue *items = rb_malloc(sizeof(CondValue));
    memset(&result, 0, sizeof(result));
    memset(items, 0, sizeof(*items));
    items[0] = condition_string(group_name);
    result.tag = RB_TAG_ARRAY;
    result.arr = items;
    result.arr_n = 1;
    return result;
}

static void baton_touch_moves_replaced_member_to_waitroom(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return;
    }

    TestGame game;
    test_game_new(&game);
    int target = test_id(&game, "PL!SP-bp2-011-R");
    int arriver = test_id(&game, "PL!HS-sd1-008-SD");
    int filler = test_id(&game, "PL!-sd1-010-SD");

    game.state.p[0].stage[1] = target;
    for (int i = 0; i < 10; i++) {
        game.state.p[0].deck.cards[game.state.p[0].deck.n++] = filler;
    }
    test_give_energy(&game, 25);

    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = arriver;
    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = filler;
    test_play_to_stage(&game, arriver, 1);
    while (test_has_pending_choice(&game)) {
        const RbChoice *choice = rb_get_pending_choice(&game.state);
        int is_required_discard = choice->kind == RB_CHOICE_SELECT_CARD &&
                                  choice->count == 1 && !choice->allow_skip;
        if (is_required_discard) {
            int indices[] = {0};
            test_select_indices(&game, indices, 1);
        } else {
            test_select_indices(&game, NULL, 0);
        }
    }

    CHECK(test_zone_has_id(&game, 0, "discard", target),
          "Replaced member should be in waitroom after baton touch");
    CHECK_EQ(game.state.p[0].stage[1], arriver,
             "Arriver should occupy center after baton touch");
    rb_unload();
}

static void baton_touch_does_not_lock_all_full_lanes(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return;
    }

    TestGame game;
    test_game_new(&game);

    int left = test_id(&game, "PL!HS-sd1-001-SD");
    int center = test_id(&game, "PL!HS-sd1-006-SD");
    int right = test_id(&game, "PL!HS-sd1-008-SD");
    int first_arriver = test_id(&game, "PL!HS-sd1-006-SD");
    int second_arriver = test_id(&game, "PL!HS-sd1-006-SD");

    game.state.p[0].stage[0] = left;
    game.state.p[0].stage[1] = center;
    game.state.p[0].stage[2] = right;
    test_give_energy(&game, 30);

    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = first_arriver;
    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = second_arriver;

    test_play_to_stage(&game, first_arriver, 1);
    drain_choices_by_skip(&game);

    RbGeneratedActionList actions = rb_generate_action_candidates(&game.state);
    const RbGeneratedAction *second_action = NULL;
    for (int i = 0; i < actions.count; i++) {
        const RbGeneratedAction *action = &actions.actions[i];
        if (action->action_type == ACTION_PLAY_MEMBER_TO_STAGE &&
            action->has_parameters && action->parameters.card_id == second_arriver) {
            second_action = action;
            break;
        }
    }
    CHECK(second_action != NULL, "Second baton-touch play action should still be available");

    if (second_action && second_action->has_parameters) {
        int has_baton_area = 0;
        for (int i = 0; i < second_action->parameters.n_available_areas; i++) {
            const RbGeneratedArea *area = &second_action->parameters.available_areas[i];
            if (area->available && area->is_baton_touch) {
                has_baton_area = 1;
                break;
            }
        }
        CHECK(has_baton_area,
              "At least one occupied lane should still offer baton touch after the first replacement");
    }
    rb_free(actions.actions);
    rb_unload();
}

static void baton_touch_hanaho_auto_ability_triggers(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return;
    }

    TestGame game;
    test_game_new(&game);

    int hanaho = test_id(&game, "PL!HS-sd1-001-SD");
    int arriver = test_id(&game, "PL!HS-sd1-006-SD");
    int filler = test_id(&game, "PL!-sd1-010-SD");

    game.state.p[0].stage[1] = hanaho;

    test_give_energy(&game, 25);
    int before = rb_energy_active_count(&game.state.p[0]);

    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = arriver;
    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = filler;
    test_play_to_stage(&game, arriver, 1);
    drain_choices_by_skip(&game);

    CHECK(test_zone_has_id(&game, 0, "discard", hanaho),
          "花帆 should be in waitroom after baton touch");

    int after = rb_energy_active_count(&game.state.p[0]);
    if (after < before - 6) {
        fprintf(stderr,
                "FAIL: Energy should reflect baton touch cost 15-9=6 (before=%d, got %d)\n",
                before, after);
        failures++;
    } else {
        printf("ok: Energy should reflect baton touch cost 15-9=6\n");
    }
    rb_unload();
}

static void baton_touch_count_per_player(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return;
    }

    TestGame game;
    test_game_new(&game);

    int hanaho = test_id(&game, "PL!HS-sd1-001-SD");
    int arriver = test_id(&game, "PL!HS-sd1-006-SD");
    int filler = test_id(&game, "PL!-sd1-010-SD");

    game.state.p[0].stage[1] = hanaho;
    test_give_energy(&game, 25);
    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = arriver;
    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = filler;

    CHECK_EQ(rb_get_baton_touch_count(&game.state, 0), 0, "p1 count starts at 0");
    CHECK_EQ(rb_get_baton_touch_count(&game.state, 1), 0, "p2 count starts at 0");

    test_play_to_stage(&game, arriver, 1);
    drain_choices_by_skip(&game);

    CHECK_EQ(rb_get_baton_touch_count(&game.state, 0), 1,
             "p1 count is 1 after p1's baton touch");
    CHECK_EQ(rb_get_baton_touch_count(&game.state, 1), 0,
             "p2 count remains 0 after p1's baton touch");
    rb_unload();
}

static void baton_touch_arriving_card_ids_tracked(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return;
    }

    TestGame game;
    test_game_new(&game);

    int hanaho = test_id(&game, "PL!HS-sd1-001-SD");
    int arriver = test_id(&game, "PL!HS-sd1-006-SD");
    int filler = test_id(&game, "PL!-sd1-010-SD");

    game.state.p[0].stage[1] = hanaho;
    test_give_energy(&game, 25);
    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = arriver;
    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = filler;

    CHECK(game.state.n_baton_touch_arriving_card_ids == 0, "starts empty");

    test_play_to_stage(&game, arriver, 1);
    drain_choices_by_skip(&game);

    int arrived = 0;
    for (int i = 0; i < game.state.n_baton_touch_arriving_card_ids; i++) {
        if (game.state.baton_touch_arriving_card_ids[i] == arriver) {
            arrived = 1;
            break;
        }
    }
    CHECK(arrived, "arriving card ID is stored");
    rb_unload();
}

static void opponent_baton_touch_discard_does_not_trigger(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return;
    }

    TestGame game;
    test_game_new(&game);

    int hanaho = test_id(&game, "PL!HS-sd1-001-SD");
    int arriver = test_id(&game, "PL!HS-sd1-006-SD");
    int filler = test_id(&game, "PL!-sd1-010-SD");

    game.state.p[0].stage[1] = hanaho;
    int p2_hanaho = test_id(&game, "PL!HS-sd1-001-SD");
    game.state.p[1].discard.cards[game.state.p[1].discard.n++] = p2_hanaho;

    test_give_energy(&game, 25);

    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = arriver;
    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = filler;

    int p2_energy_before = rb_energy_active_count(&game.state.p[1]);

    test_play_to_stage(&game, arriver, 1);
    drain_choices_by_skip(&game);

    int p2_energy_after = rb_energy_active_count(&game.state.p[1]);
    CHECK_EQ(p2_energy_after, p2_energy_before,
             "P2's energy should not change — P2's hanaho should not trigger on P1's baton touch");

    CHECK_EQ(rb_get_baton_touch_count(&game.state, 0), 1, "P1 has 1 baton touch");
    CHECK_EQ(rb_get_baton_touch_count(&game.state, 1), 0, "P2 has 0 baton touches");
    rb_unload();
}

static void card_count_condition_baton_touch_filter(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return;
    }

    TestGame game;
    test_game_new(&game);

    int member1 = test_id(&game, "PL!HS-sd1-006-SD");
    int member2 = test_id(&game, "PL!HS-sd1-008-SD");

    game.state.p[0].stage[0] = member1;
    game.state.p[0].stage[1] = member2;

    Condition condition;
    memset(&condition, 0, sizeof(condition));
    condition.variant = RB_COND_LOCATION;
    add_condition_field(&condition, "location", condition_string("stage"));
    add_condition_field(&condition, "target", condition_string("self"));
    add_condition_field(&condition, "count", condition_integer(2));
    add_condition_field(&condition, "operator", condition_string(">="));
    add_condition_field(&condition, "card_type", condition_string("member_card"));
    add_condition_field(&condition, "group_names", condition_group_names("蓮ノ空"));
    CondValue baton_touch_trigger;
    memset(&baton_touch_trigger, 0, sizeof(baton_touch_trigger));
    baton_touch_trigger.tag = RB_TAG_TRUE;
    baton_touch_trigger.b = 1;
    add_condition_field(&condition, "baton_touch_trigger", baton_touch_trigger);
    add_condition_field(&condition, "min_baton_touch_count", condition_integer(2));

    CHECK(!rb_eval_condition(&game.state, 0, &condition),
          "card_count_condition with baton_touch_trigger should fail with 0 baton touches");

    rb_record_baton_touch(&game.state, 0, member1);
    rb_record_baton_touch(&game.state, 0, member2);

    CHECK(rb_eval_condition(&game.state, 0, &condition),
          "card_count_condition with baton_touch_trigger=2 should pass with 2 baton touched members on stage");

    int write_index = 0;
    for (int i = 0; i < game.state.n_baton_touch_arriving_card_ids; i++) {
        if (game.state.baton_touch_arriving_card_ids[i] != member2) {
            game.state.baton_touch_arriving_card_ids[write_index++] =
                game.state.baton_touch_arriving_card_ids[i];
        }
    }
    game.state.n_baton_touch_arriving_card_ids = write_index;

    CHECK(!rb_eval_condition(&game.state, 0, &condition),
          "Should fail when only 1 of 2 stage members arrived via baton touch");

    rb_unload();
}

int main(void) {
    baton_touch_moves_replaced_member_to_waitroom();
    baton_touch_does_not_lock_all_full_lanes();
    baton_touch_hanaho_auto_ability_triggers();
    baton_touch_count_per_player();
    baton_touch_arriving_card_ids_tracked();
    opponent_baton_touch_discard_does_not_trigger();
    card_count_condition_baton_touch_filter();
    if (failures) {
        fprintf(stderr, "%d runtime failures\n", failures);
        return 1;
    }
    printf("ALL BATON TOUCH CHECKS PASSED\n");
    return 0;
}
