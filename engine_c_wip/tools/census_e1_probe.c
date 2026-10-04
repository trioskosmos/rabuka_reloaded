/* Targeted probe for the 12 nodes where census_e.c (214) and census_e1.c (226)
 * disagree: cards 575-578 and 1611-1614. Prints the FULL link structure of every
 * constant ability on those cards, plus rb_card_is_member, so the disagreement
 * is settled by structure rather than by argument. */
#include <stdio.h>
#include <string.h>
#include "rabuka.h"

static const char *CONST_MARK = "\xe5\xb8\xb8\xe6\x99\x82"; /* 常時 */

static void dump(const AbilityEffect *e, const char *link, int d) {
    if (!e || d > 6) return;
    for (int i = 0; i < d; i++) printf("  ");
    printf("[%s] action=%s text=%.36s n_child=%d\n", link,
           e->action ? e->action : "-", e->text ? e->text : "", e->n_child);
    for (int i = 0; i < e->n_child; i++) { char b[32]; snprintf(b,sizeof b,"child[%d]",i); dump(e->child[i], b, d+1); }
    for (int i = 0; i < e->n_options; i++) { char b[32]; snprintf(b,sizeof b,"options[%d]",i); dump(e->options[i], b, d+1); }
    dump(e->primary_effect,     "primary_effect",     d+1);
    dump(e->alternative_effect, "alternative_effect", d+1);
    dump(e->look_action,        "look_action",        d+1);
    dump(e->select_action,      "select_action",      d+1);
    dump(e->followup_action,    "followup_action",    d+1);
    dump(e->optional_action,    "optional_action",    d+1);
    dump(e->conditional_action, "conditional_action", d+1);
    dump(e->gained_effect,      "gained_effect",      d+1);
    dump(e->resource_on_select, "resource_on_select", d+1);
    dump(e->opponent_action,    "opponent_action",    d+1);
}

int main(int argc, char **argv) {
    const char *dir = argc > 1 ? argv[1] : "src";
    if (rb_load(dir) != 0) { fprintf(stderr, "rb_load failed\n"); return 2; }
    int ids[] = {575,576,577,578,1611,1612,1613,1614, 0};
    for (int k = 0; ids[k]; k++) {
        int cid = ids[k];
        Card c; int hc = rb_decode_card_by_index((uint32_t)cid, &c);
        printf("=== cid=%d is_member=%d type_flags=0x%02x type_bits=%d n_ab=%d\n",
               cid, rb_card_is_member(cid), hc ? c.type_flags : 0,
               hc ? (c.type_flags & 3) : -1, rb_card_num_abilities((uint32_t)cid));
        if (hc) rb_free_card(&c);
        int n = rb_card_num_abilities((uint32_t)cid);
        for (int ai = 0; ai < n; ai++) {
            Ability ab; memset(&ab,0,sizeof ab);
            if (!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
            int isc = ab.triggers && strstr(ab.triggers, CONST_MARK) != NULL;
            printf(" ability#%d constant=%d triggers=%s effect=%s\n", ai, isc,
                   ab.triggers ? ab.triggers : "(null)",
                   ab.effect ? (ab.effect->action ? ab.effect->action : "-") : "(NULL)");
            if (isc && ab.effect) dump(ab.effect, "ROOT", 1);
            rb_free_ability(&ab);
        }
    }
    rb_unload();
    return 0;
}