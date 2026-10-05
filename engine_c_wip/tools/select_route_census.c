#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "rabuka.h"

static const char *xg(const AbilityEffect *e, const char *k) {
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}

static int n_select = 0, n_src_field = 0, n_src_extra = 0, n_neither = 0;
static int n_hc = 0, n_orct = 0, n_chars = 0, n_group = 0, n_ctype = 0;
static int routed_area = 0, routed_hc = 0, routed_generic = 0;

static void walk(const AbilityEffect *e) {
    if (!e) return;
    if (e->action && !strcmp(e->action, "select")) {
        n_select++;
        const char *src_f = e->source;
        const char *src_e = xg(e, "source");
        if (src_f && *src_f) n_src_field++;
        if (src_e) n_src_extra++;
        if (!(src_f && *src_f) && !src_e) n_neither++;
        int has_hc = 0;
        for (int i = 0; i < e->n_extra; i++)
            if (e->extra_k[i] && !strcmp(e->extra_k[i], "heart_colors")) has_hc = 1;
        if (has_hc) n_hc++;
        if (xg(e, "or_card_types")) n_orct++;
        if (xg(e, "characters")) n_chars++;
        if (xg(e, "group_names")) n_group++;
        if (e->card_type_field && e->card_type_field[0]) n_ctype++;
        /* draw.c:653 routing, verbatim */
        if (!(src_e) && !has_hc && !xg(e,"or_card_types") &&
            !xg(e,"characters") && !xg(e,"group_names")) routed_area++;
        else if (!(src_e) && !(e->card_type_field && e->card_type_field[0]) && has_hc)
            routed_hc++;
        else routed_generic++;
    }
    for (int i = 0; i < e->n_child; i++) walk(e->child[i]);
    walk(e->primary_effect); walk(e->alternative_effect);
    walk(e->followup_action); walk(e->optional_action);
    walk(e->conditional_action);
}

int main(void) {
    if (rb_load("src") != 0 && rb_load("../cards/build") != 0) { fprintf(stderr, "load fail\n"); return 2; }
    int nc = rb_num_cards();
    for (int cid = 0; cid < nc; cid++) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
        int n = rb_card_num_abilities((uint32_t)cid);
        for (int ai = 0; ai < n; ai++) {
            Ability ab; memset(&ab, 0, sizeof(ab));
            if (!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
            walk(ab.effect); walk(ab.cost);
        }
        rb_free_card(&c);
    }
    printf("SELECT_NODES=%d\n", n_select);
    printf("  e->source set (STRUCT FIELD)      : %d\n", n_src_field);
    printf("  extra(\"source\") set              : %d\n", n_src_extra);
    printf("  neither                          : %d\n", n_neither);
    printf("  heart_colors=%d or_card_types=%d characters=%d group_names=%d card_type=%d\n",
           n_hc, n_orct, n_chars, n_group, n_ctype);
    printf("DRAW_C_ROUTING: area_select=%d heart_color=%d generic_select_card=%d\n",
           routed_area, routed_hc, routed_generic);
    /* What the FAITHFUL routing (Rust selection.rs:45-87, e->source) would do: */
    int fa = 0, fg = 0, fh = 0;
    nc = rb_num_cards();
    for (int cid = 0; cid < nc; cid++) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
        int n = rb_card_num_abilities((uint32_t)cid);
        for (int ai = 0; ai < n; ai++) {
            Ability ab; memset(&ab, 0, sizeof(ab));
            if (!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
            AbilityEffect *stack[64]; int sp = 0;
            stack[sp++] = ab.effect; stack[sp++] = ab.cost;
            while (sp) {
                AbilityEffect *e = stack[--sp];
                if (!e) continue;
                if (e->action && !strcmp(e->action, "select")) {
                    const char *sf = e->source;
                    int has_hc = 0;
                    for (int i = 0; i < e->n_extra; i++)
                        if (e->extra_k[i] && !strcmp(e->extra_k[i], "heart_colors")) has_hc = 1;
                    if (!(sf && *sf) && !has_hc && !xg(e,"or_card_types") &&
                        !xg(e,"characters") && !xg(e,"group_names")) fa++;
                    else if (!(sf && *sf) && !(e->card_type_field && e->card_type_field[0]) && has_hc) fh++;
                    else fg++;
                }
                for (int i = 0; i < e->n_child && sp < 60; i++) stack[sp++] = e->child[i];
                if (e->primary_effect && sp<60) stack[sp++]=e->primary_effect;
                if (e->alternative_effect && sp<60) stack[sp++]=e->alternative_effect;
                if (e->followup_action && sp<60) stack[sp++]=e->followup_action;
                if (e->optional_action && sp<60) stack[sp++]=e->optional_action;
                if (e->conditional_action && sp<60) stack[sp++]=e->conditional_action;
            }
        }
        rb_free_card(&c);
    }
    printf("RUST_ROUTING:  area_select=%d heart_color=%d generic_select_card=%d\n",
           fa, fh, fg);
    rb_unload();
    return 0;
}