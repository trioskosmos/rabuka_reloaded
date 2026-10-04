#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>

static int failures;
static int checks;

/* The common corpus shape (tests/test_parity_draw_flat.c:119-129 is the twin):
 * a `FAIL: ` prefix a `^FAIL` scan can see, a running failure counter, and an
 * `ok:` echo on success. The previous macro printed a bare
 * "<message>: got N, expected M" line with no prefix, bumped no counter, and
 * `return`ed from the TEST FUNCTION - so this suite emitted 0 bytes on both
 * streams and a failure was indistinguishable from a pass. */
#define ASSERT_EQ(actual, expected, message) do { \
    checks++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

static void test_draw_phase_no_unwanted_discards(void)
{
static    TestGame game;
    test_game_new(&game);

    int filler = test_id(&game, "PL!-sd1-010-SD");

    game.state.p[0].deck.n = 0;
    game.state.p[1].deck.n = 0;
    for (int i = 0; i < 10; i++) {
        game.state.p[0].deck.cards[game.state.p[0].deck.n++] = filler;
        game.state.p[1].deck.cards[game.state.p[1].deck.n++] = filler;
    }

    int p2_deck_before = game.state.p[1].deck.n;
    int p2_discard_before = game.state.p[1].discard.n;

    for (int i = 0; i < 4; i++) {
        test_pass(&game);
    }

    int p2_deck_after = game.state.p[1].deck.n;
    int p2_discard_after = game.state.p[1].discard.n;

    ASSERT_EQ(p2_deck_after, p2_deck_before - 1,
              "1 card drawn from deck during draw phase");
    ASSERT_EQ(p2_discard_after, p2_discard_before,
              "No discard during draw phase");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load failed\n");
        failures++;
        return 1;
    }

    test_draw_phase_no_unwanted_discards();
    rb_unload();
    printf("%d checks, %d failures\n", checks, failures);
    return failures ? 1 : 0;
}
