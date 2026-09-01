/* Missing functions ported from move_cards.rs */
#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* Forward declarations (defined in move.c) */
void rb_move_prompt_card_selection(GameState *g, int actor, const char *zone,
                                    int count, int can_skip, AbilityEffect *e);
int rb_move_take_cards_from_standard_zone(GameState *g, int actor,
                                           const char *zone_name,
                                           AbilityEffect *e,
                                           int count, int is_all,
                                           int can_skip, int *out_ids, int max);
int rb_move_resolve_from_zone(GameState *g, int actor, const char *effective_source,
                               AbilityEffect *e, int use_p2, int count,
                               int *out_ids, int max);
int rb_move_resolve_from_recently_moved(GameState *g, int use_p2,
                                        const char *card_type_filter,
                                        const char *group_name,
                                        int *out_ids, int max);
int rb_move_resolve_source_looked_at(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                      int count, int *out_ids, int max);
int rb_move_place_card_with_stage_choice(GameState *g, int actor, int host_cid,
                                           const char *player_target, int card_id,
                                           const char *destination, int vacated_area,
                                           int is_max, int count, const char *state_change,
                                           int deck_position, const char *source_zone,
                                           int allow_occupied_stage, int under_self);
void rb_move_fire_debut_side_effects(GameState *g, int actor, int card_id,
                                     const char *target, const char *source);
void rb_move_prompt_deck_top_or_bottom(GameState *g, int actor, int card_id,
                                        const char *target, const char *source_zone,
                                        int allow_skip);
int rb_move_optional_gate_source(const char *zone_str);



int rb_move_resolve_source_looked_at(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                      int count, int *out_ids, int max) {
    (void)g; (void)actor; (void)e; (void)use_p2; (void)count; (void)out_ids; (void)max;
    return 0;
}

static const char *cmf_extra(const AbilityEffect *e, const char *k) {
    if (!e || !k) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}

static int extra_true(const AbilityEffect *e, const char *k) {
    const char *v = cmf_extra(e, k);
    return v && (!strcmp(v, "true") || !strcmp(v, "1"));
}

static void remove_card_from_any_zone(RbPlayer *P, int *last_vacated, int cid) {
    int i;
    for (i = 0; i < P->hand.n; i++) if (P->hand.cards[i] == cid) {
        for (int k = i; k < P->hand.n - 1; k++) P->hand.cards[k] = P->hand.cards[k + 1];
        P->hand.n--; return;
    }
    for (i = 0; i < P->discard.n; i++) if (P->discard.cards[i] == cid) {
        for (int k = i; k < P->discard.n - 1; k++) P->discard.cards[k] = P->discard.cards[k + 1];
        P->discard.n--; return;
    }
    for (i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] == cid) {
        P->stage[i] = -1; P->stage_wait[i] = 0;
        if (last_vacated) *last_vacated = i;
        return;
    }
    for (i = 0; i < P->energy.n; i++) if (P->energy.cards[i] == cid) {
        for (int k = i; k < P->energy.n - 1; k++) P->energy.cards[k] = P->energy.cards[k + 1];
        P->energy.n--; return;
    }
    for (i = 0; i < P->live.n; i++) if (P->live.cards[i] == cid) {
        for (int k = i; k < P->live.n - 1; k++) P->live.cards[k] = P->live.cards[k + 1];
        P->live.n--; return;
    }
}




int rb_move_ask_optional_move_gate(GameState *g, int actor, AbilityEffect *e,
                                    const char *source_zone_str,
                                    const char *desc_en, const char *desc_ja) {
    if (!g || !e || !source_zone_str) return 0;
    if (!e->is_optional) return 0;
    if (!rb_move_optional_gate_source(source_zone_str)) return 0;
    if (g->queue.cur >= 0 && g->queue.cur < RB_QUEUE_DEPTH) {
        if (g->queue.resume_mode == 5) return 0;
    }
    int pl = actor;
    if (e->target && !strcmp(e->target, "opponent")) pl = actor ^ 1;
    RbPlayer *P = &g->p[pl];
    int available = 0;
    if (!strcmp(source_zone_str, "energy_deck")) available = P->energy_deck.n;
    else if (!strcmp(source_zone_str, "deck") || !strcmp(source_zone_str, "deck_top") || !strcmp(source_zone_str, "deck_bottom")) available = P->deck.n;
    else if (!strcmp(source_zone_str, "energy")) available = P->energy.n;
    else available = 1;
    if (available == 0) return 0;
    (void)desc_ja;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, 1, "pay_optional_cost");
    rb_choice_set_description(&g->queue.pending, desc_en);
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_OPTIONAL_COST);
    g->queue.resume_mode = 5;
    g->queue.resume_actor = actor;
    return 1;
}


