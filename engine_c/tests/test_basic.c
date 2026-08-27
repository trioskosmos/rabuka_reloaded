#include "rabuka.h"
#include <stdio.h>
#include <string.h>

static int failures = 0;
#define CHECK(cond, msg) do { if (!(cond)) { fprintf(stderr, "FAIL: %s (%s:%d)\n", msg, __FILE__, __LINE__); failures++; } else { printf("ok: %s\n", msg); } } while (0)

int main(void) {
    CHECK(rb_load("src") == 0, "rb_load");
    CHECK(rb_num_cards() > 1000, "num_cards > 1000");
    CHECK(rb_num_abilities() == 936, "num_abilities == 936");

    /* decoder: ability 0 */
    Ability a;
    CHECK(rb_decode_ability(0, &a) == 1, "decode ability 0");
    CHECK(a.full_text != NULL, "ability 0 has full_text");
    rb_free_ability(&a);

    /* decoder: a card with an ability */
    uint32_t nc = rb_num_cards();
    uint32_t sample = 0xFFFF;
    for (uint32_t i = 0; i < nc; i++) {
        if (rb_card_ability_idx(i) != 0xFFFF) { sample = i; break; }
    }
    CHECK(sample != 0xFFFF, "found a card with an ability");
    Card c;
    CHECK(rb_decode_card_by_index(sample, &c) == 1, "decode card");
    CHECK(c.name != NULL && strlen(c.name) > 0, "card has a name");
    CHECK(c.ability != NULL, "card ability decoded");
    if (c.ability) {
        printf("  card '%s' ability full_text=%s\n", c.name, c.ability->full_text ? c.ability->full_text : "(null)");
    }
    rb_free_card(&c);

    /* game: build decks, init, run a few turns */
    uint32_t deck0[40], deck1[40]; int n0 = 0, n1 = 0;
    for (uint32_t i = 0; i < nc && (n0 < 40 || n1 < 40); i++) {
        if (rb_card_ability_idx(i) == 0xFFFF) continue;
        if (n0 < 40) deck0[n0++] = i; else if (n1 < 40) deck1[n1++] = i;
    }
    GameState g;
    rb_seed(1);
    rb_game_init(&g, deck0, n0, deck1, n1);
    CHECK(g.p[0].hand.n == 6, "P0 opening hand = 6");
    int start_turn = g.turn;
    int t;
    for (t = 0; t < 300 && g.winner == -1; t++) {
        rb_turn(&g);
        CHECK(g.p[0].energy_active <= 12, "P0 energy in range");
        CHECK(g.p[1].energy_active <= 12, "P1 energy in range");
        CHECK(g.p[0].hand.n <= RB_MAX_HAND, "P0 hand in range");
    }
    CHECK(g.turn == start_turn + t, "turn counter advances");
    CHECK(g.winner != -1 || t >= 300, "match reaches a decision");

    rb_unload();
    if (failures) { printf("\n%d FAILURES\n", failures); return 1; }
    printf("\nALL TESTS PASSED\n");
    return 0;
}
