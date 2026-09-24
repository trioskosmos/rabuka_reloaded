#include "rabuka.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void print_hand(const GameState *g, int pl) {
    const RbPlayer *p = &g->p[pl];
    printf("P%d hand (%d):", pl + 1, p->hand.n);
    for (int i = 0; i < p->hand.n; i++) {
        Card card;
        const char *name = "?";
        if (rb_decode_card_by_index((uint32_t)p->hand.cards[i], &card)) {
            name = card.name ? card.name : "?";
            rb_free_card(&card);
        }
        printf(" %d:%s", i, name);
    }
    printf("\n");
}

static int read_choice(GameState *g) {
    char line[32];
    printf("choice [index, -1 to skip]: ");
    fflush(stdout);
    if (!fgets(line, sizeof(line), stdin)) return -1;
    return atoi(line);
}

static int interactive_match(GameState *g) {
    char line[32];
    while (g->winner < 0) {
        if (rb_has_pending_choice(g)) {
            const RbChoice *choice = rb_get_pending_choice(g);
            printf("pending choice kind=%d zone=%s target=%s count=%d\n",
                   choice ? (int)choice->kind : -1,
                   choice ? choice->zone : "?", choice ? choice->target : "?", choice ? choice->count : 0);
            rb_resume_with_choice(g, read_choice(g));
            continue;
        }
        if (g->phase == RB_PHASE_MAIN) {
            RbGeneratedActionList actions = rb_generate_action_candidates(g);
            print_hand(g, g->active);
            for (int i = 0; i < actions.count; i++) {
                RbGeneratedAction *action = &actions.actions[i];
                if (action->action_type == 1) {
                    printf("%d: pass\n", i);
                } else {
                    int area = 0;
                    while (area < action->parameters.n_available_areas && !action->parameters.available_areas[area].available) area++;
                    Card card;
                    const char *name = "?";
                    if (rb_decode_card_by_index((uint32_t)action->parameters.card_id, &card)) {
                        name = card.name ? card.name : "?";
                        rb_free_card(&card);
                    }
                    printf("%d: play %s to area %d%s\n", i, name, area,
                           action->parameters.available_areas[area].is_baton_touch ? " (baton)" : "");
                }
            }
            printf("action: ");
            fflush(stdout);
            if (!fgets(line, sizeof(line), stdin)) return 1;
            int selected = atoi(line);
            if (selected < 0 || selected >= actions.count) {
                rb_free(actions.actions);
                continue;
            }
            RbGeneratedAction action = actions.actions[selected];
            if (action.action_type == 1) {
                rb_execute_main_phase_action(g, 1, -1, 0, 0, 0);
            } else {
                int hand_index = -1;
                for (int i = 0; i < g->p[g->active].hand.n; i++) {
                    if (g->p[g->active].hand.cards[i] == action.parameters.card_id) {
                        hand_index = i;
                        break;
                    }
                }
                int area = 0;
                while (area < action.parameters.n_available_areas && !action.parameters.available_areas[area].available) area++;
                if (hand_index >= 0) rb_play_member(g, g->active, hand_index, area);
            }
            rb_free(actions.actions);
            continue;
        }
        if (g->phase == RB_PHASE_LIVE_SET) {
            print_hand(g, g->active);
            printf("live hand index, -1 for no live: ");
            fflush(stdout);
            if (!fgets(line, sizeof(line), stdin)) return 1;
            int index = atoi(line);
            RbPlayer *p = &g->p[g->active];
            if (index >= 0 && index < p->hand.n && rb_card_is_live(p->hand.cards[index])) {
                int card = rb_hand_remove_card(p, index);
                rb_live_add_card(p, card);
            }
            rb_advance_phase(g);
            continue;
        }
        printf("phase=%d active=P%d\n", g->phase, g->active + 1);
        rb_print_state(g);
        rb_advance_phase(g);
    }
    return 0;
}

int main(int argc, char **argv) {
    const char *dir = (argc > 1 && strcmp(argv[1], "--interactive") != 0) ? argv[1] : "src";
    if (rb_load(dir) != 0) {
        fprintf(stderr, "rb_load('%s') failed\n", dir);
        return 1;
    }
    printf("loaded: %u cards, %u abilities\n", rb_num_cards(), rb_num_abilities());

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
    int result;
    if (argc > 1 && !strcmp(argv[1], "--interactive")) {
        printf("C interactive prototype: incomplete engine, choices and unsupported actions may stop progress.\n");
        result = interactive_match(&g);
    } else {
        int steps = 0;
        while (g.winner < 0 && steps < 200) {
            rb_turn(&g);
            while (rb_has_pending_choice(&g)) rb_resume_with_choice(&g, -1);
            steps++;
            if (steps <= 10 || steps % 20 == 0) rb_print_state(&g);
        }
        printf("match ended after %d turns, winner=%d\n", steps, g.winner);
        result = 0;
    }
    rb_unload();
    return result;
}
