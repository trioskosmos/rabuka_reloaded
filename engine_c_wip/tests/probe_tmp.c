#include "rabuka.h"
#include <stdio.h>
#include <string.h>

static void dumpfx(const char *label, AbilityEffect *e, int depth)
{
    if (!e) { printf("%*s%s = NULL\n", depth*2, "", label); return; }
    printf("%*s%s action=%s source=%s dest=%s target=%s count=%d opt=%d ct=%s nchild=%d nopt=%d\n",
           depth*2, "", label, e->action?e->action:"-", e->source?e->source:"-",
           e->destination?e->destination:"-", e->target?e->target:"-", e->count, e->is_optional,
           e->card_type_field[0]?e->card_type_field:"-", e->n_child, e->n_options);
    for (int i=0;i<e->n_extra;i++)
        printf("%*s  extra[%d] %s = %s\n", depth*2, "", i, e->extra_k[i]?e->extra_k[i]:"?", e->extra_v[i]?e->extra_v[i]:"(null)");
    dumpfx("look_action", e->look_action, depth+1);
    dumpfx("select_action", e->select_action, depth+1);
    dumpfx("followup", e->followup_action, depth+1);
    for (int i=0;i<e->n_child && i<4;i++){ char b[16]; snprintf(b,sizeof b,"child[%d]",i); dumpfx(b,e->child[i],depth+1); }
    for (int i=0;i<e->n_options && i<4;i++){ char b[16]; snprintf(b,sizeof b,"opt[%d]",i); dumpfx(b,e->options[i],depth+1); }
}

static void dumpcard(const char *no)
{
    int id = rb_find_card_by_no(no);
    if (id < 0) { printf("%s NOT FOUND\n", no); return; }
    Card c; memset(&c,0,sizeof c);
    if (!rb_decode_card_by_index((uint32_t)id,&c)) { printf("%s decode fail\n", no); return; }
    printf("=== %s id=%d abilities=%d\n", no, id, rb_card_num_abilities((uint32_t)id));
    for (int i=0;i<rb_card_num_abilities((uint32_t)id) && i<4;i++){
        Ability ab; memset(&ab,0,sizeof ab);
        if (!rb_decode_card_ability((uint32_t)id,i,&ab)) continue;
        printf(" -- ability %d text=%s\n", i, (const char*)0);
        dumpfx("effect", ab.effect, 1);
        rb_free_ability(&ab);
    }
    rb_free_card(&c);
}

int main(void)
{
    if (rb_load("src") != 0) { fprintf(stderr, "load failed\n"); return 1; }
    dumpcard("PL!S-bp5-007-R");
    dumpcard("PL!N-sd2-009-SD2");
    dumpcard("PL!S-pb1-013-N");
    rb_unload();
    return 0;
}
