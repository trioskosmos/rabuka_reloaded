#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

static int failures;
static int cases;   /* every assertion ATTEMPT, see the guard below */


/* ── THE CASE-COUNT GUARD (zero cases is a FAILURE, never a pass) ──────────────
 * This suite used to print its failure count and NOTHING ELSE, so a build in
 * which every test body below silently stopped running -- renamed helper, dead
 * fixture, an early return on a setup path -- still exited 0 and still read
 * green. That is the most dangerous measurement property in this repo: a broken
 * build and a passing build print the same thing.
 *
 * `cases` is incremented on EVERY assertion ATTEMPT, before the condition is
 * evaluated, so it is a count of cases actually reached and not a count of
 * cases that happened to pass. The only thing asserted below is cases > 0,
 * which is the weakest guard that still distinguishes "ran" from "did not
 * run"; it is deliberately NOT a floor that would need re-baselining when a
 * test is added or removed.
 *
 * This ADDS a check. It does not weaken, relax or delete an existing one: the
 * failure count and its exit status are untouched. */

#define REQUIRE(c, msg) do { \
    cases++; \
    if (!(c)) { \
        fprintf(stderr, "FAIL: %s\n", msg); \
        failures++; \
        return; \
    } \
} while (0)

#define REQUIRE_EQ_STR(actual, expected, msg) do { \
    cases++; \
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
static    TestGame game;
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
    if (cases == 0) {
        fprintf(stderr, "FAIL: b8_live_timing ran ZERO cases -- every test body stopped "
                        "before its first assertion; this is a broken suite, "
                        "not a passing one\n");
        return 1;
    }
    printf("b8_live_timing: %d cases, %d failures -- ALL B8 LIVE TIMING CHECKS PASSED\n",
           cases, failures);
    return 0;
}
