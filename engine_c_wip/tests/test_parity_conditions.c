#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static void dump_cond(const Condition *c, int depth) {
    if (!c) { printf("%*s<null>\n", depth * 2, ""); return; }
    printf("%*svariant=%d n_fields=%d\n", depth * 2, "", c->variant, c->n_fields);
    for (int i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        printf("%*s  [%d] key=%s tag=%d", (depth + 1) * 2, "", i, f->key ? f->key : "?", f->v.tag);
        if (f->v.tag == RB_TAG_STR) printf(" str=%s", f->v.s ? f->v.s : "-");
        else if (f->v.tag == RB_TAG_I64) printf(" i=%lld", (long long)f->v.i);
        else if (f->v.tag == RB_TAG_TRUE) printf(" true");
        else if (f->v.tag == RB_TAG_FALSE) printf(" false");
        else if (f->v.tag == RB_TAG_ARRAY) {
            printf(" arr_n=%u", f->v.arr_n);
            for (uint32_t k = 0; k < f->v.arr_n; k++) {
                if (f->v.arr[k].tag == RB_TAG_STR) printf(" [%s]", f->v.arr[k].s ? f->v.arr[k].s : "-");
                else if (f->v.arr[k].tag == RB_TAG_I64) printf(" [%lld]", (long long)f->v.arr[k].i);
                else printf(" [t%u]", f->v.arr[k].tag);
            }
        } else if (f->v.tag == RB_TAG_OBJVAR) { printf(" obj:\n"); dump_cond(f->v.cond, depth + 2); }
        printf("\n");
    }
}

static void dump_effect_extras(const AbilityEffect *e) {
    if (!e) { printf("  effect=<null>\n"); return; }
    printf("  action=%s text=%s n_extra=%d\n", e->action ? e->action : "-", e->text ? e->text : "-", e->n_extra);
    for (int i = 0; i < e->n_extra; i++)
        printf("    extra[%d] %s = %s\n", i, e->extra_k[i], e->extra_v[i] ? e->extra_v[i] : "-");
    printf("  condition:\n");
    dump_cond(e->condition, 2);
}

static void fill_both_decks(TestGame *game, int filler) {
    game->state.p[0].deck.n = 0;
    game->state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) {
        test_add_to_deck(game, filler);
        test_add_to_deck_pl(game, 1, filler);
    }
}

static void drain_auto_choices(TestGame *game) {
    while (test_has_pending_choice(game)) {
        if (strcmp(test_pending_choice_type(game), "SelectAutoAbility") != 0) break;
        rb_resume_with_choice(&game->state, -1);
    }
}

static int set_live_card(TestGame *game, int card_id) {
    RbPlayer *player = &game->state.p[game->state.active];
    for (int i = 0; i < player->hand.n; i++) {
        if (player->hand.cards[i] != card_id) continue;
        int card = rb_hand_remove_card(player, i);
        return card >= 0 && rb_live_add_card(player, card) == 0;
    }
    return 0;
}

static void diag_scenario(void) {
    TestGame game;
    test_game_new(&game);
    int nonfiction = test_id(&game, "PL!SP-bp4-024-L");
    int self_center = test_id(&game, "PL!SP-pb1-001-R");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    { Card c; if (rb_decode_card_by_index((uint32_t)nonfiction, &c)) { printf("nonfiction score=%d cost=%d\n", c.score, c.cost); rb_free_card(&c);} }
    test_add_to_hand(&game, nonfiction);
    game.state.p[0].stage[0] = filler;
    game.state.p[0].stage[1] = self_center;
    game.state.p[0].stage[2] = filler;
    game.state.p[1].stage[0] = filler;
    game.state.p[1].stage[1] = test_id(&game, "PL!-sd1-010-SD");
    game.state.p[1].stage[2] = filler;
    fill_both_decks(&game, filler);
    for (int i = 0; i < 5; i++) test_pass(&game);
    set_live_card(&game, nonfiction);
    test_pass(&game); test_pass(&game);
    drain_auto_choices(&game);
    test_pass(&game); test_pass(&game);
    printf("DIAG: score_mod(nonc)=%d  n_snapshots=%d n_trace=%d\n",
           test_get_score_modifier(&game, nonfiction), game.state.n_snapshots, rb_mods_trace_len(&game.state.mods));
    for (int k = 0; k < rb_mods_trace_len(&game.state.mods); k++) {
        const RbAbilityTraceEntry *e = &game.state.mods.trace[k];
        printf("  trace[%d] type=%d src=%d tgt=%d amt=%d text=%s\n", k, e->effect_type, e->source_card_id, e->target_card_id, e->amount, e->ability_text);
    }
    for (int i = 0; i < game.state.n_snapshots; i++) {
        const RbLiveSnapshot *s = &game.state.snapshots[i];
        printf("  snap[%d] total=%d n_lives=%d:", i, s->total_score, s->n_lives);
        for (int l = 0; l < s->n_lives; l++) printf(" %d", s->lives[l]);
        printf("\n");
    }
}

static void diag_both_players(void) {
    TestGame game;
    test_game_new(&game);
    int p1_card = test_id(&game, "PL!SP-bp4-024-L");
    int p2_card = test_new_id(&game, "PL!SP-bp4-024-L");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    int p_left = test_id(&game, "PL!SP-pb1-001-R");
    int p_center = test_new_id(&game, "PL!SP-pb1-001-R");
    int p2_left = test_new_id(&game, "PL!SP-pb1-001-R");
    int p2_center = test_new_id(&game, "PL!SP-pb1-001-R");
    test_add_to_hand(&game, p1_card);
    game.state.p[0].stage[0] = p_left;
    game.state.p[0].stage[1] = p_center;
    game.state.p[0].stage[2] = filler;
    game.state.p[1].hand.cards[game.state.p[1].hand.n++] = p2_card;
    game.state.p[1].stage[0] = p2_left;
    game.state.p[1].stage[1] = p2_center;
    game.state.p[1].stage[2] = filler;
    fill_both_decks(&game, filler);
    printf("DIAG2 ids: p1card=%d p2card=%d p_left=%d p_center=%d p2_left=%d p2_center=%d filler=%d\n",
           p1_card, p2_card, p_left, p_center, p2_left, p2_center, filler);
    int b1 = test_get_blade_modifier(&game, p_left), b2 = test_get_blade_modifier(&game, p2_left);
    printf("DIAG2 before: p_left=%d p2_left=%d\n", b1, b2);
    for (int i = 0; i < 5; i++) { test_pass(&game); printf("DIAG2 pass%d phase=%d p_left=%d p2_left=%d\n", i, game.state.phase, test_get_blade_modifier(&game, p_left), test_get_blade_modifier(&game, p2_left)); }
    set_live_card(&game, p1_card);
    printf("DIAG2 p1 set: p_left=%d p2_left=%d\n", test_get_blade_modifier(&game, p_left), test_get_blade_modifier(&game, p2_left));
    test_pass(&game);
    game.state.active = 1;
    set_live_card(&game, p2_card);
    printf("DIAG2 p2 set: p_left=%d p2_left=%d\n", test_get_blade_modifier(&game, p_left), test_get_blade_modifier(&game, p2_left));
    test_pass(&game);
    printf("DIAG2 after pass: p_left=%d p2_left=%d\n", test_get_blade_modifier(&game, p_left), test_get_blade_modifier(&game, p2_left));
    drain_auto_choices(&game);
    test_pass(&game);
    printf("DIAG2 after pass2: p_left=%d p2_left=%d\n", test_get_blade_modifier(&game, p_left), test_get_blade_modifier(&game, p2_left));
    drain_auto_choices(&game);
    test_pass(&game);
    printf("DIAG2 after pass3: p_left=%d p2_left=%d\n", test_get_blade_modifier(&game, p_left), test_get_blade_modifier(&game, p2_left));
    drain_auto_choices(&game);
    test_pass(&game);
    drain_auto_choices(&game);
    printf("DIAG2 final: p_left=%d p2_left=%d  (before %d %d)\n", test_get_blade_modifier(&game, p_left), test_get_blade_modifier(&game, p2_left), b1, b2);
}

int main(void) {
    if (rb_load("src") != 0) { fprintf(stderr, "database load failed\n"); return 1; }

    int nc = rb_find_card_by_no("PL!SP-bp4-024-L");
    printf("nonfiction id=%d abilities=%d\n", nc, rb_card_num_abilities((uint32_t)nc));
    for (int a = 0; a < rb_card_num_abilities((uint32_t)nc); a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)nc, a, &ab)) { printf("ab#%d decode fail\n", a); continue; }
        printf("ab#%d trigger=%s\n", a, ab.triggers ? ab.triggers : "-");
        dump_effect_extras(ab.effect);
    }

    int self_c = rb_find_card_by_no("PL!SP-pb1-001-R");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    printf("PL!SP-pb1-001-R id=%d group(Liella!)=%d group(mu)=%d\n", self_c,
           rb_card_matches_group_str(self_c, "Liella!"), rb_card_matches_group_str(self_c, "μ's"));
    printf("PL!-sd1-010-SD id=%d group(Liella!)=%d\n", filler, rb_card_matches_group_str(filler, "Liella!"));
    { Card c; if (rb_decode_card_by_index((uint32_t)self_c, &c)) { printf("  pb1-001-R cost=%d score=%d blade=%d\n", c.cost, c.score, c.blade); rb_free_card(&c);} }
    { Card c; if (rb_decode_card_by_index((uint32_t)filler, &c)) { printf("  sd1-010-SD cost=%d score=%d blade=%d\n", c.cost, c.score, c.blade); rb_free_card(&c);} }

    diag_scenario();
    diag_both_players();
    rb_unload();
    return 0;
}
