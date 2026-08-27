#include "rabuka.h"
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    const char *dir = (argc > 1) ? argv[1] : "src";
    if (rb_load(dir) != 0) {
        fprintf(stderr, "rb_load('%s') failed\n", dir);
        return 1;
    }
    printf("loaded: %u cards, %u abilities\n", rb_num_cards(), rb_num_abilities());

    /* ── verify decoder on a real ability ── */
    Ability a;
    if (rb_decode_ability(0, &a)) {
        printf("ability[0].full_text = %s\n", a.full_text ? a.full_text : "(null)");
        if (a.effect) printf("ability[0].effect.action = %s\n", a.effect->action ? a.effect->action : "(null)");
        rb_free_ability(&a);
    }

    /* ── build two decks of cards that actually have abilities ── */
    uint32_t deck0[60], deck1[60]; int n0 = 0, n1 = 0;
    uint32_t nc = rb_num_cards();
    for (uint32_t i = 0; i < nc && (n0 < 40 || n1 < 40); i++) {
        if (rb_card_ability_idx(i) == 0xFFFF) continue;
        if (n0 < 40) deck0[n0++] = i;
        else if (n1 < 40) deck1[n1++] = i;
    }
    printf("decks: P0=%d cards, P1=%d cards\n", n0, n1);

    GameState g;
    rb_seed(0xCAFE);
    rb_game_init(&g, deck0, n0, deck1, n1);

    /* ── run a match — portable host drains pending_choice by auto-skipping optional picks ── */
    int steps = 0;
    while (g.winner < 0 && steps < 200) {
        rb_turn(&g);
        while (rb_has_pending_choice(&g)) {
            const RbChoice *ch = rb_get_pending_choice(&g);
            printf("[host] pending choice kind=%d zone=%s count=%d allow_skip=%d target=%s → auto-skip\n",
                   ch ? (int)ch->kind : -1, ch ? ch->zone : "?", ch ? ch->count : 0, ch ? ch->allow_skip : 0, ch ? ch->target : "?");
            rb_resume_with_choice(&g, -1);
        }
        steps++;
        if (steps <= 10 || steps % 20 == 0) rb_print_state(&g);
    }
    printf("match ended after %d turns, winner=%d\n", steps, g.winner);
    rb_unload();
    return 0;
}
