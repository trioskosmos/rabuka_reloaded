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

/* Forward declarations for functions defined in this file */
int rb_move_resolve_from_revealed_cards(GameState *g, int actor, AbilityEffect *e,
                                         int count, int is_all, int is_max,
                                         int *out_ids, int max);
int rb_move_resolve_from_those_cards(GameState *g, int actor, AbilityEffect *e,
                                      int use_p2, int count, int *out_ids, int max,
                                      int *out_fell_through);
int rb_move_ask_optional_move_gate(GameState *g, int actor, AbilityEffect *e,
                                    const char *source_zone_str,
                                    const char *desc_en, const char *desc_ja);
int rb_move_resolve_from_standard_zone(GameState *g, int actor, AbilityEffect *e,
                                        int use_p2, int count,
                                        int *out_ids, int max);
int rb_move_resolve_from_selected_cards(GameState *g, int actor, AbilityEffect *e,
                                         int use_p2, int count,
                                         int *out_ids, int max);
int rb_move_resolve_source_revealed_cards(GameState *g, int actor, AbilityEffect *e,
                                           int use_p2, int count,
                                           int *out_ids, int max);
int rb_move_maybe_prompt_success_replacement(GameState *g, int actor, int card_id,
                                              const char *dest, const char *target);
void rb_move_execute_move_cards(GameState *g, int actor, AbilityEffect *e);

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

static void mc_record_movement(GameState *g, int cid) {
    if (cid < 0) return;
    if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++] = cid;
    else {
        for (int i = 1; i < RB_MAX_RECENTLY_MOVED; i++) g->recently_moved[i-1] = g->recently_moved[i];
        g->recently_moved[RB_MAX_RECENTLY_MOVED-1] = cid;
    }
}

int rb_move_resolve_cards_from_source(GameState *g, int actor, AbilityEffect *e,
                                       int count, int *out_ids, int max) {
    if (!g || !out_ids || !e) return 0;
    if (count <= 0) count = 1;
    const char *source = e->source ? e->source : "";
    const char *target = e->target ? e->target : "self";
    int use_p2 = (target && !strcmp(target, "opponent")) ? 1 : 0;
    int pl = use_p2 ? 1 : actor;
    RbPlayer *P = &g->p[pl];
    const char *source_str = (!source || !*source) ? "discard" : source;
    if (g->n_selected_cards > 0 && !strcmp(source_str, "selected_cards")) {
        int n = 0;
        for (int i = 0; i < g->n_selected_cards && n < max; i++) {
            int cid = g->selected_cards[i];
            int lv = -1;
            remove_card_from_any_zone(P, &lv, cid);
            if (lv >= 0) g->baton_last_vacated_area[pl] = lv;
            out_ids[n++] = cid;
        }
        g->n_selected_cards = 0;
        return n;
    }
    if (!strcmp(source_str, "recently_moved")) {
        return rb_move_resolve_from_recently_moved(g, use_p2,
            cmf_extra(e, "card_type"), cmf_extra(e, "group_names"), out_ids, max);
    }
    if (!strcmp(source_str, "preceding_moved")) {
        int n = 0;
        const char *chars = cmf_extra(e, "characters");
        for (int i = 0; i < g->n_recently_moved && n < max; i++) {
            int cid = g->recently_moved[i];
            if (cid == -1) continue;
            if (chars && *chars) {
                const char *names[1] = { chars };
                if (!rb_card_matches_characters(cid, names, 1)) continue;
            }
            int lv = -1;
            remove_card_from_any_zone(P, &lv, cid);
            if (lv >= 0) g->baton_last_vacated_area[pl] = lv;
            out_ids[n++] = cid;
        }
        return n;
    }
    if (!strcmp(source_str, "looked_at_remaining")) {
        return rb_move_resolve_source_looked_at(g, actor, e, use_p2, count, out_ids, max);
    }
    if (!strcmp(source_str, "revealed_cards")) {
        return rb_move_resolve_from_revealed_cards(g, actor, e, count,
            extra_true(e, "all"), extra_true(e, "max"), out_ids, max);
    }
    if (!strcmp(source_str, "those_cards")) {
        int tc_out[RB_MAX_RECENTLY_MOVED];
        int fell_through = 0;
        int r = rb_move_resolve_from_those_cards(g, actor, e, use_p2, count,
            tc_out, RB_MAX_RECENTLY_MOVED, &fell_through);
        if (!fell_through) {
            for (int i = 0; i < r && i < max; i++) out_ids[i] = tc_out[i];
            return r;
        }
        source_str = "discard";
    }
    return rb_move_resolve_from_zone(g, actor, source_str, e, use_p2, count, out_ids, max);
}

int rb_move_resolve_from_revealed_cards(GameState *g, int actor, AbilityEffect *e,
                                         int count, int is_all, int is_max,
                                         int *out_ids, int max) {
    if (!g || !out_ids || !e) return 0;
    int take_count = is_all ? g->n_revealed : (count < g->n_revealed ? count : g->n_revealed);
    int can_skip = is_max || e->is_optional;
    const char *ctype = cmf_extra(e, "card_type");
    const char *gn = cmf_extra(e, "group_names");
    const char *cl = cmf_extra(e, "cost_limit");
    int cost_limit = cl ? atoi(cl) : -1;
    const char *clop = cmf_extra(e, "cost_operator");
    const char *cp = cmf_extra(e, "card_property");
    const char *neg = cmf_extra(e, "negation");
    int is_neg = neg && (!strcmp(neg, "true") || !strcmp(neg, "1"));
    int matching[RB_MAX_RECENTLY_MOVED];
    int nm = 0;
    for (int i = 0; i < g->n_revealed && nm < RB_MAX_RECENTLY_MOVED; i++) {
        int cid = g->revealed_cards[i];
        if (ctype && !rb_card_matches_type(cid, ctype)) continue;
        if (gn && !rb_card_matches_group_str(cid, gn)) continue;
        if (cost_limit >= 0 && !rb_card_matches_cost_limit(cid, cost_limit, clop)) continue;
        if (cp && *cp) {
            Card card;
            int has_prop = 0;
            if (rb_decode_card_by_index((uint32_t)cid, &card)) {
                if (!strcmp(cp, "has_blade_heart")) has_prop = rb_card_has_blade_heart(&card);
                else if (!strcmp(cp, "has_score_icon")) has_prop = rb_card_has_score_icon(&card);
                rb_free_card(&card);
            }
            if (is_neg) { if (has_prop) continue; } else { if (!has_prop) continue; }
        }
        matching[nm++] = i;
    }
    if (nm == 0) return 0;
    if (take_count < nm || can_skip) {
        rb_move_prompt_card_selection(g, actor, "revealed_cards", take_count, can_skip, e);
        return -1;
    }
    int actual = take_count < nm ? take_count : nm;
    int sorted_idx[RB_MAX_RECENTLY_MOVED];
    for (int i = 0; i < actual; i++) sorted_idx[i] = matching[i];
    for (int i = 0; i < actual - 1; i++)
        for (int j = i + 1; j < actual; j++)
            if (sorted_idx[j] > sorted_idx[i]) { int t = sorted_idx[i]; sorted_idx[i] = sorted_idx[j]; sorted_idx[j] = t; }
    int nout = 0;
    for (int k = 0; k < actual; k++) {
        int idx = sorted_idx[k];
        int cid = g->revealed_cards[idx];
        for (int j = idx; j < g->n_revealed - 1; j++) g->revealed_cards[j] = g->revealed_cards[j + 1];
        g->n_revealed--;
        for (int p = 0; p < 2; p++) {
            RbPlayer *PP = &g->p[p];
            for (int i = 0; i < PP->discard.n; i++) if (PP->discard.cards[i] == cid) {
                for (int j = i; j < PP->discard.n - 1; j++) PP->discard.cards[j] = PP->discard.cards[j + 1];
                PP->discard.n--; break;
            }
            for (int i = 0; i < PP->deck.n; i++) if (PP->deck.cards[i] == cid) {
                for (int j = i; j < PP->deck.n - 1; j++) PP->deck.cards[j] = PP->deck.cards[j + 1];
                PP->deck.n--; break;
            }
        }
        if (nout < max) out_ids[nout++] = cid;
    }
    return nout;
}

int rb_move_resolve_from_those_cards(GameState *g, int actor, AbilityEffect *e,
                                      int use_p2, int count, int *out_ids, int max,
                                      int *out_fell_through) {
    if (!g || !out_ids || !e) { if (out_fell_through) *out_fell_through = 1; return 0; }
    if (count <= 0) count = 1;
    int pl = use_p2 ? 1 : actor;
    RbPlayer *P = &g->p[pl];
    const char *destination = e->destination ? e->destination : "";
    const char *ctype = cmf_extra(e, "card_type");
    const char *gn = cmf_extra(e, "group_names");
    int trigger_n = 0;
    const int *trigger_cards = rb_entry_trigger_moved_cards(g, &trigger_n);
    int pool[RB_MAX_RECENTLY_MOVED];
    int pool_n = 0;
    if (trigger_cards && trigger_n > 0) {
        for (int i = 0; i < trigger_n && pool_n < RB_MAX_RECENTLY_MOVED; i++) pool[pool_n++] = trigger_cards[i];
    } else {
        for (int i = 0; i < g->n_recently_moved && pool_n < RB_MAX_RECENTLY_MOVED; i++) pool[pool_n++] = g->recently_moved[i];
    }
    if (pool_n == 0) { if (out_fell_through) *out_fell_through = 1; return 0; }
    int matching[RB_MAX_RECENTLY_MOVED];
    int nm = 0;
    for (int i = 0; i < pool_n; i++) {
        int cid = pool[i];
        if (ctype && !rb_card_matches_type(cid, ctype)) continue;
        if (gn && !rb_card_matches_group_str(cid, gn)) continue;
        matching[nm++] = cid;
    }
    if (nm == 0) { if (out_fell_through) *out_fell_through = 0; return 0; }
    if (nm <= count && (strcmp(destination, "deck_top_or_bottom") == 0 || !e->is_optional)) {
        int take = nm < count ? nm : count;
        if (!e->is_optional) {
            for (int i = 0; i < take; i++) {
                int cid = matching[i];
                for (int j = 0; j < P->discard.n; j++) if (P->discard.cards[j] == cid) {
                    for (int k = j; k < P->discard.n - 1; k++) P->discard.cards[k] = P->discard.cards[k + 1];
                    P->discard.n--; break;
                }
            }
        }
        for (int i = 0; i < take && i < max; i++) out_ids[i] = matching[i];
        if (out_fell_through) *out_fell_through = 0;
        return take;
    }
    if (!strcmp(destination, "deck_top_or_bottom")) {
        int filtered[RB_MAX_RECENTLY_MOVED];
        int nf = 0;
        for (int i = 0; i < nm; i++) {
            int cid = matching[i];
            for (int j = 0; j < P->discard.n; j++) if (P->discard.cards[j] == cid) {
                int dup = 0;
                for (int k = 0; k < nf; k++) if (filtered[k] == j) { dup = 1; break; }
                if (!dup) filtered[nf++] = j;
            }
        }
        char desc[128];
        if (gn && *gn) snprintf(desc, sizeof(desc), "Select 1 %s card to place on deck", gn);
        else snprintf(desc, sizeof(desc), "Select 1 card to place on deck");
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "discard", ctype, 1, 0, NULL);
        rb_choice_set_description(&g->queue.pending, desc);
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
        g->queue.resume_mode = 2;
        g->queue.resume_actor = actor;
        if (out_fell_through) *out_fell_through = 0;
        return -1;
    }
    int filtered[RB_MAX_RECENTLY_MOVED];
    int nf = 0;
    for (int i = 0; i < nm; i++) {
        int cid = matching[i];
        for (int j = 0; j < P->discard.n; j++) if (P->discard.cards[j] == cid) {
            int dup = 0;
            for (int k = 0; k < nf; k++) if (filtered[k] == j) { dup = 1; break; }
            if (!dup) filtered[nf++] = j;
        }
    }
    char desc[128];
    if (gn && *gn) snprintf(desc, sizeof(desc), "Select %d %s card(s)", count, gn);
    else snprintf(desc, sizeof(desc), "Select card(s)");
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "discard", ctype, count, e->is_optional, NULL);
    rb_choice_set_description(&g->queue.pending, desc);
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
    g->queue.resume_mode = 2;
    g->queue.resume_actor = actor;
    if (out_fell_through) *out_fell_through = 0;
    return -1;
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

int rb_move_resolve_from_standard_zone(GameState *g, int actor, AbilityEffect *e,
                                        int use_p2, int count, int *out_ids, int max) {
    if (!g || !out_ids || !e) return 0;
    const char *actual_zone = e->source ? e->source : "hand";
    int is_all = extra_true(e, "all");
    int is_max = extra_true(e, "max");
    int can_skip = e->is_optional || is_max;
    if (!strcmp(actual_zone, "discard")) can_skip = is_max || e->is_optional;
    else if (!strcmp(actual_zone, "hand")) can_skip = e->is_optional || cmf_extra(e, "any_number") != NULL;
    else if (!strcmp(actual_zone, "success_live_zone")) can_skip = e->is_optional;
    return rb_move_take_cards_from_standard_zone(g, actor, actual_zone, e, count, is_all, can_skip, out_ids, max);
}

int rb_move_resolve_from_selected_cards(GameState *g, int actor, AbilityEffect *e,
                                         int use_p2, int count, int *out_ids, int max) {
    if (!g || !out_ids || !e) return 0;
    int pl = use_p2 ? 1 : actor;
    RbPlayer *P = &g->p[pl];
    int is_all = extra_true(e, "all");
    if (g->n_selected_cards == 0) return 0;
    int n_sel = g->n_selected_cards;
    if (count >= n_sel || is_all) {
        int n = 0;
        for (int i = 0; i < n_sel && n < max; i++) {
            int cid = g->selected_cards[i];
            int lv = -1;
            remove_card_from_any_zone(P, &lv, cid);
            if (lv >= 0) g->baton_last_vacated_area[pl] = lv;
            out_ids[n++] = cid;
        }
        g->n_selected_cards = 0;
        return n;
    }
    rb_move_prompt_card_selection(g, actor, "selected_cards", count, 0, e);
    return -1;
}

int rb_move_resolve_source_revealed_cards(GameState *g, int actor, AbilityEffect *e,
                                           int use_p2, int count, int *out_ids, int max) {
    if (!g || !out_ids || !e) return 0;
    int pl = use_p2 ? 1 : actor;
    RbPlayer *P = &g->p[pl];
    int owned[RB_MAX_RECENTLY_MOVED];
    int n_owned = 0;
    for (int i = 0; i < g->n_revealed && n_owned < RB_MAX_RECENTLY_MOVED; i++) {
        int cid = g->revealed_cards[i];
        int is_owned = 0;
        for (int j = 0; j < P->hand.n; j++) if (P->hand.cards[j] == cid) { is_owned = 1; break; }
        if (!is_owned) for (int j = 0; j < P->discard.n; j++) if (P->discard.cards[j] == cid) { is_owned = 1; break; }
        if (!is_owned) for (int j = 0; j < RB_STAGE_SIZE; j++) if (P->stage[j] == cid) { is_owned = 1; break; }
        if (!is_owned) for (int j = 0; j < P->deck.n; j++) if (P->deck.cards[j] == cid) { is_owned = 1; break; }
        if (!is_owned) for (int j = 0; j < P->energy.n; j++) if (P->energy.cards[j] == cid) { is_owned = 1; break; }
        if (!is_owned) for (int j = 0; j < P->energy_deck.n; j++) if (P->energy_deck.cards[j] == cid) { is_owned = 1; break; }
        if (!is_owned) for (int j = 0; j < P->live.n; j++) if (P->live.cards[j] == cid) { is_owned = 1; break; }
        if (!is_owned) for (int j = 0; j < P->success.n; j++) if (P->success.cards[j] == cid) { is_owned = 1; break; }
        if (is_owned) {
            for (int j = i; j < g->n_revealed - 1; j++) g->revealed_cards[j] = g->revealed_cards[j + 1];
            g->n_revealed--;
            i--;
            owned[n_owned++] = cid;
        }
    }
    if (n_owned == 0) return 0;
    if (n_owned > count) {
        for (int i = 0; i < n_owned; i++) if (g->n_revealed < RB_MAX_ZONE) g->revealed_cards[g->n_revealed++] = owned[i];
        rb_move_prompt_card_selection(g, actor, "revealed_cards", count, e->is_optional, e);
        return -1;
    }
    int nout = 0;
    for (int i = 0; i < n_owned && nout < max; i++) {
        int cid = owned[i];
        RbPlayer *A = &g->p[g->active];
        for (int j = 0; j < A->hand.n; j++) if (A->hand.cards[j] == cid) {
            for (int k = j; k < A->hand.n - 1; k++) A->hand.cards[k] = A->hand.cards[k + 1];
            A->hand.n--; break;
        }
        out_ids[nout++] = cid;
    }
    return nout;
}

int rb_move_maybe_prompt_success_replacement(GameState *g, int actor, int card_id,
                                              const char *dest, const char *target) {
    if (!g || !dest || !target) return 0;
    if (strcmp(dest, "success_live_zone") != 0) return 0;
    int pl = rb_resolve_target_player(g, target);
    if (pl < 0) pl = actor;
    RbPlayer *P = &g->p[pl];
    int has_live = 0;
    for (int i = 0; i < P->discard.n; i++) {
        int cid = P->discard.cards[i];
        if (rb_card_is_live(cid)) { has_live = 1; break; }
    }
    if (!has_live) return 0;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "discard", "live_card", 1, 1, NULL);
    rb_choice_set_description(&g->queue.pending,
        "Choose a live card from discard to place in your success zone (or skip to place the original card)");
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
    g->queue.resume_mode = 2;
    g->queue.resume_actor = actor;
    g->queue.resume_host = card_id;
    return 1;
}

void rb_move_execute_move_cards(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    int count = 1;
    if (e->count >= 0) count = e->count;
    else {
        int dc = rb_effect_count(g, actor, -1, e, g->last_draw_count);
        count = dc > 0 ? dc : 1;
    }
    if (count <= 0) count = 1;
    const char *source = e->source ? e->source : "";
    const char *destination = e->destination ? e->destination : "discard";
    const char *target = e->target ? e->target : "self";
    int is_max = extra_true(e, "max");
    int allow_occupied = extra_true(e, "allow_occupied_stage");
    int use_p2 = (target && !strcmp(target, "opponent")) ? 1 : 0;
    int pl = use_p2 ? 1 : actor;
    const char *or_types = cmf_extra(e, "or_card_types");
    if (or_types && *or_types) {
        char desc[128];
        snprintf(desc, sizeof(desc), "Pick card type: %s", or_types);
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, 0, "choice_string");
        rb_choice_set_description(&g->queue.pending, desc);
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_CONDITIONAL_CHOICE);
        g->queue.resume_mode = 6;
        g->queue.resume_actor = actor;
        return;
    }
    if (!strcmp(destination, "empty_area") || !strcmp(destination, "stage")) {
        RbPlayer *P = &g->p[pl];
        int has_empty = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] < 0) { has_empty = 1; break; }
        if (!has_empty) return;
    }
    int taken[RB_MAX_ZONE];
    int n_taken = rb_move_resolve_cards_from_source(g, actor, e, count, taken, RB_MAX_ZONE);
    if (n_taken < 0) return;
    if (n_taken == 0) return;
    RbPlayer *P = &g->p[pl];
    int stage_full = (!strcmp(destination, "stage") && !allow_occupied);
    if (stage_full) {
        int all_occupied = 1;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] < 0) { all_occupied = 0; break; }
        if (all_occupied) {
            for (int i = 0; i < n_taken; i++) rb_waitroom_add(P, taken[i]);
            for (int i = 0; i < n_taken; i++) mc_record_movement(g, taken[i]);
            rb_recalc_constants(g);
            return;
        }
    }
    int moved[RB_MAX_ZONE];
    int n_moved = 0;
    for (int i = 0; i < n_taken; i++) {
        int cid = taken[i];
        if (rb_move_maybe_prompt_success_replacement(g, actor, cid, destination, target)) return;
        if (!strcmp(destination, "deck_top_or_bottom")) {
            rb_move_prompt_deck_top_or_bottom(g, actor, cid, target, source, e->is_optional);
            return;
        }
        int placed = rb_move_place_card_with_stage_choice(g, actor, -1, target, cid,
            destination, -1, is_max, count, NULL, -1, source, allow_occupied, 0);
        if (placed == 1) return;
        if (placed == 0) {
            moved[n_moved++] = cid;
            rb_move_fire_debut_side_effects(g, actor, cid, target, NULL);
        }
    }
    for (int i = 0; i < n_moved; i++) mc_record_movement(g, moved[i]);
    rb_recalc_constants(g);
}
