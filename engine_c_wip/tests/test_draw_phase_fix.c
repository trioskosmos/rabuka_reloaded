#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>

#define ASSERT_EQ(actual, expected, message) do { \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "%s: got %d, expected %d\n", message, actual_value, expected_value); \
        return; \
    } \
} while (0)

static void test_draw_phase_no_unwanted_discards(void)
{
    TestGame game;
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
        fprintf(stderr, "database load failed\n");
        return 1;
    }

    test_draw_phase_no_unwanted_discards();
    rb_unload();
    return 0;
}
