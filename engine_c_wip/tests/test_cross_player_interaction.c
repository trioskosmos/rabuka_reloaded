#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

#define HIMEKO "PL!HS-pb1-014-R"
#define MIRAKURA_RURINO "PL!HS-PR-029-PR"
#define NON_MIRAKURA_MEMBER "PL!-sd1-010-SD"
#define KOKO "PL!SP-sd2-002-P"

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

#define CHECK_NE(actual, unexpected, message) do { \
    int actual_value = (actual); \
    int unexpected_value = (unexpected); \
    if (actual_value == unexpected_value) { \
        fprintf(stderr, "FAIL: %s (got %d)\n", message, actual_value); \
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

static int heart06(TestGame *game, int card_id)
{
    return test_get_heart_modifier(game, card_id, RB_HEART_ORANGE);
}

static void fill_decks(TestGame *game, int filler)
{
    game->state.p[0].deck.n = 0;
    game->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(game, 0, filler);
        test_add_to_deck_pl(game, 1, filler);
    }
}

static int stage_contains(const int stage[3], int card_id)
{
    return stage[0] == card_id || stage[1] == card_id || stage[2] == card_id;
}

static void himeko_debut_repositions_opponent_member_and_koko_responds(void)
{
    TestGame game;
    test_game_new(&game);
    int rurino = test_id(&game, MIRAKURA_RURINO);
    int himeko = test_id(&game, HIMEKO);
    int koko = test_id(&game, KOKO);
    int filler = test_id(&game, "PL!-sd1-010-SD");

    game.state.p[0].stage[0] = rurino;
    game.state.p[0].stage[1] = -1;
    game.state.p[0].stage[2] = -1;
    game.state.p[1].stage[0] = -1;
    game.state.p[1].stage[1] = -1;
    game.state.p[1].stage[2] = koko;
    test_add_to_hand(&game, himeko);
    fill_decks(&game, filler);
    test_give_energy(&game, 15);

    test_play_to_stage(&game, himeko, 1);

    CHECK(test_has_pending_choice(&game), "expected the opponent-member selection prompt");
    CHECK_EQ_STR(test_pending_choice_type(&game), "SelectTarget", "reposition target choice");
    int selected[] = {0};
    test_select_indices(&game, selected, 1);

    while (test_has_pending_choice(&game)) {
        test_select_indices(&game, NULL, 0);
    }

    CHECK_NE(game.state.p[1].stage[2], koko, "可可 must be repositioned away from P2-right");
    CHECK_EQ(game.state.p[1].stage[1], koko, "可可 should land on P2-center (姫芽's front)");
    CHECK(stage_contains(game.state.p[1].stage, koko), "position change keeps the member on the stage");
    CHECK_EQ(heart06(&game, koko), 1,
             "可可's area-move watcher must fire even though P1's effect moved her");
}

static void himeko_gate_blocked_no_reposition_no_koko_response(void)
{
    TestGame game;
    test_game_new(&game);
    int himeko = test_id(&game, HIMEKO);
    int koko = test_id(&game, KOKO);
    int outsider = test_id(&game, NON_MIRAKURA_MEMBER);
    int filler = test_id(&game, "PL!-sd1-010-SD");

    game.state.p[0].stage[0] = outsider;
    game.state.p[0].stage[1] = -1;
    game.state.p[0].stage[2] = -1;
    game.state.p[1].stage[0] = -1;
    game.state.p[1].stage[1] = -1;
    game.state.p[1].stage[2] = koko;
    test_add_to_hand(&game, himeko);
    fill_decks(&game, filler);
    test_give_energy(&game, 15);

    test_play_to_stage(&game, himeko, 1);
    while (test_has_pending_choice(&game)) {
        test_select_indices(&game, NULL, 0);
    }

    CHECK_EQ(game.state.p[1].stage[2], koko, "gate failed: 可可 must stay at P2-right");
    CHECK_EQ(heart06(&game, koko), 0,
             "no movement happened, so 可可's watcher must be silent");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    himeko_debut_repositions_opponent_member_and_koko_responds();
    himeko_gate_blocked_no_reposition_no_koko_response();

    rb_unload();
    if (failures) {
        fprintf(stderr, "%d runtime failures\n", failures);
        return 1;
    }
    printf("ALL CROSS PLAYER INTERACTION CHECKS PASSED\n");
    return 0;
}
