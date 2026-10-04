/* dump_cost.c <card_no> [ab_index] -- print one ability's COST tree verbatim,
 * so a test fixture is chosen from the corpus rather than from memory.
 * Usage: ./dump_cost PL!-bp3-003-P 0 */
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
static void dump(const AbilityEffect *e, int d) {
    if (!e) return;
    for (int i = 0; i < d; i++) printf("  ");
    printf("action=%-22s source=%-14s target=%-18s count=%d is_optional=%d"
           " card_type_field='%s' text='%.40s'\n",
           e->action ? e->action : "(none)", e->source ? e->source : "-",
           e->target ? e->target : "-", e->count, e->is_optional,
           e->card_type_field, e->text ? e->text : "-");
    for (int i = 0; i < e->n_extra; i++) {
        for (int j = 0; j <= d; j++) printf("  ");
        printf("extra %s = %s\n", e->extra_k[i], e->extra_v[i] ? e->extra_v[i] : "(null)");
    }
    for (int i = 0; i < e->n_child; i++) dump(e->child[i], d + 1);
    dump(e->primary_effect, d + 1);
    dump(e->alternative_effect, d + 1);
    dump(e->followup_action, d + 1);
    dump(e->optional_action, d + 1);
    dump(e->conditional_action, d + 1);
}

int main(int argc, char **argv) {
    if (rb_load("src") != 0 && rb_load("../cards/build") != 0) { fprintf(stderr, "load fail\n"); return 2; }
    if (argc < 2) { fprintf(stderr, "usage: dump_cost <card_no> [ab]\n"); return 2; }
    int want_ab = argc > 2 ? atoi(argv[2]) : -1;
    int nc = rb_num_cards();
    for (int cid = 0; cid < nc; cid++) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
        const char *no = rb_card_string(c.card_no_idx);
        if (!no || strcmp(no, argv[1]) != 0) { rb_free_card(&c); continue; }
        int n = rb_card_num_abilities((uint32_t)cid);
        printf("CARD %s id=%d abilities=%d member=%d cost=%d\n", no, cid, n,
               rb_card_is_member(cid), c.cost);
        for (int ai = 0; ai < n && (want_ab < 0 || ai == want_ab); ai++) {
            Ability ab; memset(&ab, 0, sizeof(ab));
            if (!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
            printf("  --- ab#%d triggers='%s' ---\n", ai, ab.triggers ? ab.triggers : "-");
            dump(ab.cost, 2);
        }
        rb_free_card(&c);
    }
    rb_unload();
    return 0;
}