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
int rb_move_maybe_prompt_success_replacement(GameState *g, int actor, int card_id,
                                               const char *dest, const char *target);
void rb_move_finalize_card_movement(GameState *g, int actor,
                                     const int *moved_cards, int n_moved,
                                     const char *destination, const char *source,
                                     const char *state_change, const char *target);

int rb_move_optional_gate_source(const char *zone_str);
int rb_move_resolve_cards_from_source(GameState *g, int actor, AbilityEffect *e,
                                        int count, int *out_ids, int max);
static const char *cmf_extra(const AbilityEffect *e, const char *k);

static int move_port_remove_any(GameState *g, int pl, int cid) {
    RbPlayer *P = &g->p[pl];
    for (int i = 0; i < P->hand.n; i++) if (P->hand.cards[i] == cid) {
        for (int k = i; k < P->hand.n - 1; k++) P->hand.cards[k] = P->hand.cards[k + 1];
        P->hand.n--; return 1;
    }
    for (int i = 0; i < P->discard.n; i++) if (P->discard.cards[i] == cid) {
        for (int k = i; k < P->discard.n - 1; k++) P->discard.cards[k] = P->discard.cards[k + 1];
        P->discard.n--; return 1;
    }
    for (int i = 0; i < P->deck.n; i++) if (P->deck.cards[i] == cid) {
        for (int k = i; k < P->deck.n - 1; k++) P->deck.cards[k] = P->deck.cards[k + 1];
        P->deck.n--; return 1;
    }
    for (int i = 0; i < P->energy.n; i++) if (P->energy.cards[i] == cid) {
        for (int k = i; k < P->energy.n - 1; k++) P->energy.cards[k] = P->energy.cards[k + 1];
        P->energy.n--; return 1;
    }
    for (int i = 0; i < P->live.n; i++) if (P->live.cards[i] == cid) {
        for (int k = i; k < P->live.n - 1; k++) P->live.cards[k] = P->live.cards[k + 1];
        P->live.n--; return 1;
    }
    for (int i = 0; i < P->success.n; i++) if (P->success.cards[i] == cid) {
        for (int k = i; k < P->success.n - 1; k++) P->success.cards[k] = P->success.cards[k + 1];
        P->success.n--; return 1;
    }
    for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] == cid) {
        P->stage[i] = RB_EMPTY_SLOT;
        P->stage_wait[i] = 0;
        return 1;
    }
    return 0;
}

static int move_port_place_deck(GameState *g, int pl, int cid, const char *destination, int vacated) {
    RbBag *deck = &g->p[pl].deck;
    if (!strcmp(destination, "deck_bottom")) {
        if (deck->n >= RB_MAX_ZONE) return 0;
        deck->cards[deck->n++] = cid;
        return 1;
    }
    int idx = vacated >= 0 && vacated <= deck->n ? vacated : 0;
    if (deck->n >= RB_MAX_ZONE) return 0;
    for (int i = deck->n; i > idx; i--) deck->cards[i] = deck->cards[i - 1];
    deck->cards[idx] = cid;
    deck->n++;
    return 1;
}

static int move_port_same_area(GameState *g, int pl, int cid, int area) {
    RbPlayer *P = &g->p[pl];
    if (area < 0) {
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] == RB_EMPTY_SLOT) { area = i; break; }
    }
    if (area < 0 || area >= RB_STAGE_SIZE) return 0;
    if (P->stage[area] != RB_EMPTY_SLOT) rb_waitroom_add(P, P->stage[area]);
    P->stage[area] = cid;
    P->stage_wait[area] = 0;
    if (area >= 0) g->stage_arrived[pl][area] = 1;
    return 1;
}

void rb_move_execute_move_cards_ported(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    const char *source = e->source ? e->source : "hand";
    const char *destination = e->destination ? e->destination : "discard";
    int count = e->count;
    if (count < 0) count = RB_MAX_ZONE;
    int pl = actor;
    if (e->target && !strcmp(e->target, "opponent")) pl = actor ^ 1;
    if (pl < 0 || pl > 1) pl = actor;
    int ids[RB_MAX_ZONE];
    int n = rb_move_resolve_cards_from_source(g, actor, e, count, ids, RB_MAX_ZONE);
    if (n <= 0 || rb_has_pending_choice(g)) return;

    int source_is_relay = !strcmp(source, "selected_cards") ||
                          !strcmp(source, "those_cards") ||
                          !strcmp(source, "recently_moved") ||
                          !strcmp(source, "preceding_moved") ||
                          !strcmp(source, "revealed_cards");
    const char *state = cmf_extra(e, "state_change");
    int is_max = cmf_extra(e, "max") && (!strcmp(cmf_extra(e, "max"), "true") || !strcmp(cmf_extra(e, "max"), "1"));
    int allow_occupied = cmf_extra(e, "allow_occupied_stage") &&
        (!strcmp(cmf_extra(e, "allow_occupied_stage"), "true") || !strcmp(cmf_extra(e, "allow_occupied_stage"), "1"));
    int under_self = cmf_extra(e, "under_self") &&
        (!strcmp(cmf_extra(e, "under_self"), "true") || !strcmp(cmf_extra(e, "under_self"), "1"));
    int moved[RB_MAX_ZONE];
    int nm = 0;
    for (int i = 0; i < n; i++) {
        int cid = ids[i];
        if (source_is_relay) move_port_remove_any(g, pl, cid);
        if (rb_move_maybe_prompt_success_replacement(g, pl, cid, destination, e->target)) return;
        if (!strcmp(destination, "deck_top_or_bottom")) {
            rb_move_prompt_deck_top_or_bottom(g, actor, cid, e->target, source, e->is_optional);
            g->queue.resume_eff = e;
            g->queue.resume_actor = actor;
            return;
        }
        int vacated = g->baton_last_vacated_area[pl];
        int placed = 0;
        if (!strcmp(destination, "stage") || !strcmp(destination, "empty_area") ||
            !strcmp(destination, "under_member")) {
            placed = rb_move_place_card_with_stage_choice(g, actor, -1, e->target,
                cid, destination, vacated, is_max, count, state, -1, source,
                allow_occupied, under_self) == 0;
        } else if (!strcmp(destination, "same_area")) {
            placed = move_port_same_area(g, pl, cid, vacated);
        } else if (!strcmp(destination, "deck") || !strcmp(destination, "deck_top") ||
                   !strcmp(destination, "deck_bottom")) {
            placed = move_port_place_deck(g, pl, cid, destination, vacated);
        } else {
            placed = rb_place_card_in_zone(g, pl, cid, destination, vacated);
        }
        if (!placed) {
            rb_place_card_in_zone(g, pl, cid, source_is_relay ? "discard" : source, -1);
            continue;
        }
        rb_mods_clear_card(&g->mods, cid);
        moved[nm++] = cid;
    }
    if (nm > 0) {
        rb_move_finalize_card_movement(g, actor, moved, nm, destination, source,
                                       state, e->target);
        if (!strcmp(destination, "stage") || !strcmp(destination, "empty_area") ||
            !strcmp(destination, "same_area")) {
            for (int i = 0; i < nm; i++)
                rb_move_fire_debut_side_effects(g, actor, moved[i], e->target ? e->target : "self", NULL);
        }
    }
}

int rb_move_looked_at_matches(GameState *g, int cid, AbilityEffect *e);
int rb_move_resolve_cost_limit_reference(const GameState *g, const AbilityEffect *e);

int rb_move_resolve_source_looked_at(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                      int count, int *out_ids, int max) {
    if (!g || !e || !out_ids || max <= 0 || actor < 0 || actor > 1) return 0;
    int pl = use_p2 ? 1 : 0;
    int cards[RB_MAX_ZONE], matching[RB_MAX_ZONE], nm = 0;
    int n = rb_looked_at_pool(pl, cards, RB_MAX_ZONE);
    int is_all = e->count < 0;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "all") && e->extra_v[i])
            is_all |= !strcmp(e->extra_v[i], "true") || !strcmp(e->extra_v[i], "1");
    for (int i = 0; i < n; i++)
        if (rb_move_looked_at_matches(g, cards[i], e)) matching[nm++] = cards[i];
    int take = is_all ? nm : (count < nm ? count : nm);
    if (take > max) take = max;
    if (take <= 0) return 0;
    if (e->is_optional) {
        rb_move_prompt_card_selection(g, actor, "looked_at", take, 1, e);
        const char *dest = e->destination ? e->destination : "discard";
        char desc[128];
        snprintf(desc, sizeof(desc), "Move up to %d looked-at card(s) to %s?", take, dest);
        rb_choice_set_description(&g->queue.pending, desc);
        if (e->card_type_field[0])
            snprintf(g->queue.pending.card_type, sizeof(g->queue.pending.card_type), "%s", e->card_type_field);
        g->queue.pending.cost_limit = rb_move_resolve_cost_limit_reference(g, e);
        snprintf(g->queue.pending.cost_limit_op, sizeof(g->queue.pending.cost_limit_op), "<=");
        for (int i = 0; i < e->n_extra; i++)
            if (e->extra_k[i] && e->extra_v[i] &&
                (!strcmp(e->extra_k[i], "cost_limit_operator") || !strcmp(e->extra_k[i], "cost_operator")))
                snprintf(g->queue.pending.cost_limit_op, sizeof(g->queue.pending.cost_limit_op), "%s", e->extra_v[i]);
        snprintf(g->queue.pending.target_player_id, sizeof(g->queue.pending.target_player_id), "%s", pl ? "p2" : "p1");
        g->n_recently_moved = 0;
        g->n_those_cards = 0;
        g->queue.resume_mode = 6;
        g->queue.resume_eff = e;
        g->queue.resume_actor = actor;
        g->queue.resume_draw_target = pl;
        g->queue.resume_draw_count = take;
        g->queue.resume_draw_self_id = 0;
        return 0;
    }
    int taken = 0;
    for (int i = 0; i < take; i++)
        if (rb_look_remove(pl, matching[i])) out_ids[taken++] = matching[i];
    return taken;
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
    rb_queue_pause_for_choice(g, &g->queue.pending);
    rb_choice_set_description(&g->queue.pending, desc_en);
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_OPTIONAL_COST);
    g->queue.resume_mode = 5;
    g->queue.resume_actor = actor;
    return 1;
}


