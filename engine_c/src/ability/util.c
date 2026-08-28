/* util.c — pure card/zone/comparison helpers.
   Mirror engine/src/ability/util.rs (compare_counts, card_matches_type,
   card_matches_group_str, card_at_position, pos_to_area, orientation_matches_state,
   zone_cards). These are shared by the condition evaluators and the effect
   executors; keeping them in one place mirrors the Rust single-source layout.

   Group matching is an approximation of the Rust card_matches_group_str: the C
   Card exposes group/unit/name strings (via rb_card_string) but not the series
   or set_card_identity-derived memberships, so only group/unit/name substring
   matches are performed. Fullwidth '！'(U+FF01) and micro 'µ'(U+00B5) are
   normalized to '!'/'μ' exactly like the Rust norm_group_name helper. */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>

/* Normalize a UTF-8 copy: '！'(EF BC 81)→'!', 'µ'(C2 B5)→'μ'(CE BC).
   Caller must rb_free the result. Returns NULL on alloc failure. */
static char *norm_str(const char *s) {
    if (!s) return NULL;
    size_t n = strlen(s);
    char *out = rb_malloc(n + 1);
    if (!out) return NULL;
    size_t j = 0;
    for (size_t i = 0; i < n; ) {
        if ((unsigned char)s[i] == 0xEF && (unsigned char)s[i+1] == 0xBC && (unsigned char)s[i+2] == 0x81) {
            out[j++] = '!'; i += 3; continue; /* fullwidth ! */
        }
        if ((unsigned char)s[i] == 0xC2 && (unsigned char)s[i+1] == 0xB5) {
            out[j++] = (char)0xCE; out[j++] = (char)0xBC; i += 2; continue; /* µ → μ */
        }
        out[j++] = s[i++];
    }
    out[j] = '\0';
    return out;
}

/* Mirror util::compare_counts. operator NULL defaults to ">=". */
int rb_compare_counts(const char *operator, int actual, int expected) {
    const char *op = operator ? operator : ">=";
    if (!strcmp(op, ">=")) return actual >= expected;
    if (!strcmp(op, ">"))  return actual >  expected;
    if (!strcmp(op, "<=")) return actual <= expected;
    if (!strcmp(op, "<"))  return actual <  expected;
    if (!strcmp(op, "==") || !strcmp(op, "="))  return actual == expected;
    if (!strcmp(op, "!=")) return actual != expected;
    return 1;
}

/* Mirror util::card_matches_type. card_id is a card_no index. */
int rb_card_matches_type(int card_id, const char *filter) {
    if (!filter) return 1;
    Card c; if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int is_live  = (c.n_hearts == 0 && c.cost == 0 && c.blade == 0);
    int is_member = !is_live;
    int r;
    if (!strcmp(filter, "live_card"))        r = is_live;
    else if (!strcmp(filter, "member_card")) r = is_member;
    else if (!strcmp(filter, "energy_card")) r = 0; /* no energy cards in C db */
    else r = 1;
    rb_free_card(&c);
    return r;
}

/* Mirror util::orientation_matches_state. */
int rb_orientation_matches_state(const char *orientation, const char *state) {
    if (!orientation) return state && !strcmp(state, "active");
    return !strcmp(orientation, state);
}

/* Mirror util::card_matches_group_str (simplified: group/unit/name substring). */
int rb_card_matches_group_str(int card_id, const char *group_name) {
    if (!group_name) return 1;
    Card c; if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;

    char *gn = norm_str(group_name);
    const char *g  = rb_card_string(c.group_idx);
    const char *u  = rb_card_string(c.unit_idx);
    char *gnorm = g  ? norm_str(g)  : NULL;
    char *unorm = u  ? norm_str(u)  : NULL;
    char *nnorm = c.name ? norm_str(c.name) : NULL;

    int match = 0;
    if (gnorm) {
        if ((g && (!strcmp(g, group_name) || strstr(g, group_name) || strstr(group_name, g))) ||
            (u && (!strcmp(u, group_name) || strstr(u, group_name) || strstr(group_name, u))) ||
            (c.name && (strstr(c.name, group_name) || strstr(group_name, c.name))))
            match = 1;
        /* normalized comparisons catch fullwidth-bang / micro mismatches */
        if (!match && (strstr(gnorm, gn) || strstr(gn, gnorm) ||
                       (unorm && (strstr(unorm, gn) || strstr(gn, unorm))) ||
                       (nnorm && (strstr(nnorm, gn) || strstr(gn, nnorm)))))
            match = 1;
    }
    rb_free_card(&c);
    if (gn) rb_free(gn);
    if (gnorm) rb_free(gnorm);
    if (unorm) rb_free(unorm);
    if (nnorm) rb_free(nnorm);
    return match;
}

/* Mirror util::card_at_position. pos in {"left_side","center","right_side"}.
   Returns card_no index or -1. */
int rb_card_at_position(const struct GameState *g, int pl, const char *pos) {
    int idx = -1;
    if (!pos) return -1;
    if (!strcmp(pos, "left_side")) idx = 0;
    else if (!strcmp(pos, "center")) idx = 1;
    else if (!strcmp(pos, "right_side")) idx = 2;
    else return -1;
    if (idx < 0 || idx >= RB_STAGE_SIZE) return -1;
    int cid = g->p[pl].stage[idx];
    return cid == RB_EMPTY_SLOT ? -1 : cid;
}

/* Mirror util::pos_to_area → stage index (0 left, 1 center, 2 right). */
int rb_pos_to_area(const char *pos) {
    if (!pos) return 1;
    if (!strcmp(pos, "left_side"))  return 0;
    if (!strcmp(pos, "center"))     return 1;
    if (!strcmp(pos, "right_side")) return 2;
    return 1;
}

/* Mirror util::zone_cards. Fills out_ids (capacity max) with card_no indices
   for the named zone of player pl; returns the count written. */
int rb_zone_cards(const struct GameState *g, int pl, const char *zone, int *out_ids, int max) {
    const RbPlayer *P = &g->p[pl];
    int n = 0;
    #define PUSH(cid) do { if (n < max) out_ids[n++] = (cid); } while(0)
    if (!zone) return 0;
    if (!strcmp(zone, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) PUSH(P->stage[i]);
    } else if (!strcmp(zone, "hand")) {
        for (int i = 0; i < P->hand.n; i++) PUSH(P->hand.cards[i]);
    } else if (!strcmp(zone, "deck") || !strcmp(zone, "deck_top") || !strcmp(zone, "deck_bottom")) {
        for (int i = 0; i < P->deck.n; i++) PUSH(P->deck.cards[i]);
    } else if (!strcmp(zone, "discard") || !strcmp(zone, "waitroom")) {
        for (int i = 0; i < P->discard.n; i++) PUSH(P->discard.cards[i]);
    } else if (!strcmp(zone, "energy") || !strcmp(zone, "energy_zone")) {
        for (int i = 0; i < P->energy.n; i++) PUSH(P->energy.cards[i]);
    } else if (!strcmp(zone, "live") || !strcmp(zone, "live_card_zone")) {
        for (int i = 0; i < P->live.n; i++) PUSH(P->live.cards[i]);
    } else if (!strcmp(zone, "success") || !strcmp(zone, "success_live_zone") || !strcmp(zone, "success_live_card_zone")) {
        for (int i = 0; i < P->success.n; i++) PUSH(P->success.cards[i]);
    }
    #undef PUSH
    return n;
}
