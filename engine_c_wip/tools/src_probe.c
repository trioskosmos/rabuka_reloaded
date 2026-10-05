#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "rabuka.h"

static const char *xg(const AbilityEffect *e, const char *k) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}

static void walk(const AbilityEffect *e, int d, const char *card) {
    if (!e) return;
    for (int i = 0; i < d; i++) printf("  ");
    printf("[%s] action=%s e->source=%s extra(\"source\")=%s n_extra=%d\n",
           card, e->action ? e->action : "(none)",
           e->source ? e->source : "(null)",
           xg(e, "source") ? xg(e, "source") : "(NULL)",
           e->n_extra);
    for (int i = 0; i < e->n_extra; i++) {
        for (int j = 0; j <= d; j++) printf("  ");
        printf("extra[%d] %s = %s\n", i, e->extra_k[i],
               e->extra_v[i] ? e->extra_v[i] : "(null)");
    }
    for (int i = 0; i < e->n_child; i++) walk(e->child[i], d + 1, card);
    walk(e->primary_effect, d + 1, card);
    walk(e->alternative_effect, d + 1, card);
    walk(e->followup_action, d + 1, card);
    walk(e->optional_action, d + 1, card);
    walk(e->conditional_action, d + 1, card);
}

int main(int argc, char **argv) {
    if (rb_load("src") != 0 && rb_load("../cards/build") != 0) { fprintf(stderr, "load fail\n"); return 2; }
    if (argc < 2) { fprintf(stderr, "usage: src_probe <card_no>\n"); return 2; }
    int nc = rb_num_cards();
    for (int cid = 0; cid < nc; cid++) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
        const char *no = rb_card_string(c.card_no_idx);
        if (!no || strcmp(no, argv[1]) != 0) { rb_free_card(&c); continue; }
        int n = rb_card_num_abilities((uint32_t)cid);
        for (int ai = 0; ai < n; ai++) {
            Ability ab; memset(&ab, 0, sizeof(ab));
            if (!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
            printf("CARD %s ab#%d\n", no, ai);
            walk(ab.effect, 1, no);
        }
        rb_free_card(&c);
    }
    rb_unload();
    return 0;
}