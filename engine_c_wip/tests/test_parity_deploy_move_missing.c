#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static void dump(TestGame *g, const char *tag) {
    RbPlayer *P = &g->state.p[0];
    int pool[64];
    printf("[%s] stage=[%d,%d,%d] hand=%d discard=%d deck=%d under0=%d recently=%d those=%d looked=%d\n",
           tag, P->stage[0], P->stage[1], P->stage[2], P->hand.n, P->discard.n,
           P->deck.n, P->under_cards[0].n,
           g->state.n_recently_moved, g->state.n_those_cards,
           rb_looked_at_pool(0, pool, 64));
}

static void probe_keke(void) {
    TestGame g; test_game_new(&g);
    int keke = test_new_id(&g, "PL!SP-sd1-002-SD");
    int liella = test_new_id(&g, "PL!SP-sd1-013-SD");
    test_add_to_hand(&g, liella);
    test_add_to_hand(&g, keke);
    for (int i = 0; i < 40; i++) test_add_to_deck(&g, test_id(&g, "PL!-sd1-010-SD"));
    test_give_energy(&g, 20);
    dump(&g, "keke/pre");
    int ok = test_play_to_stage(&g, keke, 1);
    printf("keke play ok=%d pending=%d type=%s\n", ok, test_has_pending_choice(&g),
           test_pending_choice_type(&g));
    dump(&g, "keke/after-play");
    int idx = -1;
    for (int i = 0; i < g.state.p[0].hand.n; i++) if (g.state.p[0].hand.cards[i] == liella) { idx = i; break; }
    printf("liella idx=%d\n", idx);
    if (test_has_pending_choice(&g)) test_resume_choice(&g, idx);
    printf("after select pending=%d type=%s\n", test_has_pending_choice(&g), test_pending_choice_type(&g));
    dump(&g, "keke/after-select");
    if (test_has_pending_choice(&g)) test_resume_choice(&g, 0);
    printf("after pos pending=%d\n", test_has_pending_choice(&g));
    dump(&g, "keke/final");
}

static void probe_full_stage(void) {
    TestGame g; test_game_new(&g);
    int a = test_new_id(&g, "PL!-sd1-010-SD");
    int b = test_new_id(&g, "PL!-sd1-002-SD");
    int c = test_new_id(&g, "PL!SP-sd1-006-SD");
    test_add_to_stage(&g, 0, a);
    test_add_to_stage(&g, 1, b);
    test_add_to_stage(&g, 2, c);
    int m = test_new_id(&g, "PL!-sd1-011-SD");
    test_add_to_hand(&g, m);
    AbilityEffect e = {0};
    e.action = "move_cards"; e.source = "hand"; e.destination = "stage"; e.count = 1;
    rb_execute_effect_ex(&g.state, 0, &e, -1);
    dump(&g, "full-stage");
    printf("m in waitroom=%d in hand=%d\n",
           test_zone_has_id(&g, 0, "waitroom", m), test_hand_has(&g, m));
}

static void probe_empty_area_full(void) {
    TestGame g; test_game_new(&g);
    int a = test_new_id(&g, "PL!-sd1-010-SD");
    int b = test_new_id(&g, "PL!-sd1-002-SD");
    int c = test_new_id(&g, "PL!SP-sd1-006-SD");
    test_add_to_stage(&g, 0, a);
    test_add_to_stage(&g, 1, b);
    test_add_to_stage(&g, 2, c);
    int m = test_new_id(&g, "PL!-sd1-011-SD");
    test_add_to_hand(&g, m);
    AbilityEffect e = {0};
    e.action = "move_cards"; e.source = "hand"; e.destination = "empty_area"; e.count = 1;
    rb_execute_effect_ex(&g.state, 0, &e, -1);
    dump(&g, "empty_area-full");
    printf("m in waitroom=%d in hand=%d pending=%d\n",
           test_zone_has_id(&g, 0, "waitroom", m), test_hand_has(&g, m), test_has_pending_choice(&g));
}

static void probe_deck_pos(void) {
    TestGame g; test_game_new(&g);
    int d1 = test_new_id(&g, "PL!-sd1-010-SD");
    int d2 = test_new_id(&g, "PL!-sd1-002-SD");
    int d3 = test_new_id(&g, "PL!SP-sd1-006-SD");
    test_add_to_deck(&g, d1);
    test_add_to_deck(&g, d2);
    test_add_to_deck(&g, d3);
    int m = test_new_id(&g, "PL!-sd1-011-SD");
    test_add_to_hand(&g, m);
    AbilityEffect e = {0};
    e.action = "move_cards"; e.source = "hand"; e.destination = "deck"; e.count = 1;
    e.extra_k[0] = "position"; e.extra_v[0] = "2"; e.n_extra = 1;
    rb_execute_effect_ex(&g.state, 0, &e, -1);
    printf("deck_pos: n=%d [%d,%d,%d,%d] m=%d\n", g.state.p[0].deck.n,
           g.state.p[0].deck.cards[0], g.state.p[0].deck.cards[1],
           g.state.p[0].deck.cards[2], g.state.p[0].deck.cards[3], m);
}

static void probe_movement(void) {
    TestGame g; test_game_new(&g);
    int host = test_new_id(&g, "PL!SP-sd1-006-SD");
    int en = test_new_id(&g, "LL-E-001-SD");
    test_add_to_stage(&g, 0, host);
    test_place_under(&g, 0, 0, en);
    dump(&g, "mv/pre");
    AbilityEffect e = {0};
    e.action = "move_cards"; e.source = "under_member"; e.destination = "hand"; e.count = 1;
    rb_execute_effect_ex(&g.state, 0, &e, -1);
    dump(&g, "mv/under->hand");
    printf("en in hand=%d\n", test_hand_has(&g, en));
    printf("A\n");
    /* recently_moved -> stage */
    TestGame g2; test_game_new(&g2);
    int a2 = test_new_id(&g2, "PL!-sd1-010-SD");
    int a3 = test_new_id(&g2, "PL!SP-sd1-006-SD");
    test_add_to_discard(&g2, a2);
    g2.state.recently_moved[g2.state.n_recently_moved++] = a2;
    g2.state.recently_moved[g2.state.n_recently_moved++] = a3;
    AbilityEffect e2 = {0};
    e2.action = "move_cards"; e2.source = "recently_moved"; e2.destination = "stage"; e2.count = 1;
    rb_execute_effect_ex(&g2.state, 0, &e2, -1);
    dump(&g2, "mv/recent->stage");
    printf("stage has a2=%d pending=%d\n", test_stage_has(&g2, 0, a2), test_has_pending_choice(&g2));

    /* those_cards -> discard */
    TestGame g3; test_game_new(&g3);
    int t1 = test_new_id(&g3, "PL!-sd1-010-SD");
    g3.state.those_cards[g3.state.n_those_cards++] = t1;
    AbilityEffect e3 = {0};
    e3.action = "move_cards"; e3.source = "those_cards"; e3.destination = "hand"; e3.count = 1;
    rb_execute_effect_ex(&g3.state, 0, &e3, -1);
    printf("those->hand t1 in hand=%d\n", test_hand_has(&g3, t1));

    /* energy_deck -> hand */
    TestGame g4; test_game_new(&g4);
    int e1 = test_new_id(&g4, "LL-E-002-SD");
    test_add_to_energy_deck(&g4, 0, e1);
    AbilityEffect e4 = {0};
    e4.action = "move_cards"; e4.source = "energy_deck"; e4.destination = "hand"; e4.count = 1;
    rb_execute_effect_ex(&g4.state, 0, &e4, -1);
    printf("energy_deck->hand e1 in hand=%d\n", test_hand_has(&g4, e1));

    /* stage -> discard */
    TestGame g5; test_game_new(&g5);
    int s1 = test_new_id(&g5, "PL!SP-sd1-006-SD");
    test_add_to_stage(&g5, 1, s1);
    AbilityEffect e5 = {0};
    e5.action = "move_cards"; e5.source = "stage"; e5.destination = "discard"; e5.count = 1;
    rb_execute_effect_ex(&g5.state, 0, &e5, -1);
    printf("stage->discard s1 in discard=%d stage_empty=%d\n",
           test_zone_has_id(&g5, 0, "discard", s1), g5.state.p[0].stage[1]);
}

int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    if (rb_load("src") != 0) return 1;
    printf("loaded\n");
    probe_keke();
    probe_full_stage();
    probe_empty_area_full();
    probe_deck_pos();
    probe_movement();
    rb_unload();
    return 0;
}
