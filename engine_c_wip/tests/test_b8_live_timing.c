#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

static int failures;

#define REQUIRE(c, msg) do { \
    if (!(c)) { \
        fprintf(stderr, "FAIL: %s\n", msg); \
        failures++; \
        return; \
    } \
} while (0)

#define REQUIRE_EQ_STR(actual, expected, msg) do { \
    const char *actual_ = (actual); \
    const char *expected_ = (expected); \
    if (strcmp(actual_, expected_) != 0) { \
        fprintf(stderr, "FAIL: %s (got %s expected %s)\n", msg, actual_, expected_); \
        failures++; \
        return; \
    } \
} while (0)

static void advance_to_live_success(TestGame *game)
{
    for (int i = 0; i < 5; i++) {
        test_pass(game);
    }
    REQUIRE(strstr(rb_phase_name(game->state.phase), "LiveCardSet") != NULL,
            "advance reaches LiveCardSet");
    for (int i = 0; i < 5; i++) {
        test_pass(game);
    }
}

static void q36_live_success_timing(void)
{
    TestGame game;
    test_game_new(&game);

    int live = test_id(&game, "PL!-sd1-019-SD");
    int member = test_id(&game, "PL!-sd1-001-SD");
    int filler = test_id(&game, "PL!-sd1-010-SD");

    game.state.p[0].stage[0] = member;
    game.state.p[0].stage[1] = -1;
    game.state.p[0].stage[2] = -1;
    test_add_to_hand(&game, live);
    for (int i = 0; i < 10; i++) {
        test_add_to_deck_pl(&game, 0, filler);
        test_add_to_deck_pl(&game, 1, filler);
    }

    advance_to_live_success(&game);

    REQUIRE_EQ_STR(rb_phase_name(game.state.phase), "Active",
                   "After LiveVictoryDetermination, phase should be Active");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    q36_live_success_timing();

    rb_unload();
    if (failures) return 1;
    printf("ALL B8 LIVE TIMING CHECKS PASSED\n");
    return 0;
}
