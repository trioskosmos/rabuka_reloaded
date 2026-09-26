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
static int extra_true(const AbilityEffect *e, const char *k);
void rb_move_execute_move_cards_both(GameState *g, int actor, AbilityEffect *e);

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

static int move_port_same_area(GameState *g, int pl, int cid, int area) {
    RbPlayer *P = &g->p[pl];
    int slot = -1;
    if (area >= 0 && area < RB_STAGE_SIZE && P->stage[area] == RB_EMPTY_SLOT) slot = area;
    if (slot < 0)
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] == RB_EMPTY_SLOT) { slot = i; break; }
    if (slot < 0) {
        /* Rust util.rs:2183-2190 — with no empty slot the card falls back to hand. */
        if (P->hand.n < RB_MAX_ZONE) { P->hand.cards[P->hand.n++] = cid; return 1; }
        rb_waitroom_add(P, cid);
        return 1;
    }
    P->stage[slot] = cid;
    P->stage_wait[slot] = 0;
    g->stage_arrived[pl][slot] = 1;
    return 1;
}

/* Rust move_cards.rs:1567-1578 parse_effect_deck_pos — the wire `position` is a
   1-based deck slot; it maps to a 0-based index, and an unparsable/absent value
   means "no position". */
static int cmf_effect_deck_pos(const AbilityEffect *e) {
    const char *v = cmf_extra(e, "position");
    if (!v || !*v) return -1;
    for (const char *p = v; *p; p++)
        if (*p < '0' || *p > '9') return -1;
    long n = strtol(v, NULL, 10);
    if (n <= 0) return 0;
    if (n - 1 > RB_MAX_ZONE) n = (long)RB_MAX_ZONE + 1;
    return (int)(n - 1);
}

/* Rust move_cards.rs:2030-2031 — effect.distinct_any() is a dedupe variant. */
static int cmf_distinct_dedupe(const AbilityEffect *e) {
    const char *v = cmf_extra(e, "distinct");
    if (!v) return 0;
    return !strcmp(v, "card_name") || !strcmp(v, "true") || !strcmp(v, "distinct");
}

int rb_dedupe_by_normalized_name(const int *items, int n, int *out, int max);

void rb_move_execute_move_cards_ported(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    const char *source = e->source ? e->source : "hand";
    const char *destination = e->destination ? e->destination : "discard";
    /* Rust move_cards.rs:1866-1872 — `count` is the effect's own count (0 when
       absent); `is_all` is a separate flag. move.c instead encodes "all" as a
       negative count, so the raw value has to reach the resolver unchanged —
       saturating it to RB_MAX_ZONE here silently turned every `all: true` take
       into an unsatisfiable "take everything" request. */
    int raw_count = e->count;
    int place_count = raw_count < 0 ? RB_MAX_ZONE : raw_count;

    /* Rust move_cards.rs:1861-1865 — a multi-target move to the deck is owned by
       the dedicated both-targets path. */
    if (e->target && !strcmp(e->target, "deck") && extra_true(e, "multiple_targets")) {
        rb_move_execute_move_cards_both(g, actor, e);
        return;
    }

    int pl = actor;
    if (e->target && !strcmp(e->target, "opponent")) pl = actor ^ 1;
    if (pl < 0 || pl > 1) pl = actor;

    /* Rust move_cards.rs:1976-1982 — 「メンバーのいないエリアに」 is a no-op when the
       target already has no empty slot; the move must not even take a card. */
    if (!strcmp(destination, "empty_area")) {
        int tp = rb_resolve_target_player(g, e->target ? e->target : "self");
        if (tp < 0 || tp > 1) tp = actor;
        int any_empty = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (g->p[tp].stage[i] == RB_EMPTY_SLOT) { any_empty = 1; break; }
        if (!any_empty) return;
    }

    int ids[RB_MAX_ZONE];
    int n = rb_move_resolve_cards_from_source(g, actor, e, raw_count, ids, RB_MAX_ZONE);
    if (rb_has_pending_choice(g)) return;

    /* Rust move_cards.rs:2005-2009 and 2015-2024 — an optional move that found
       nothing reports "the cost was not paid"; inside an all-or-nothing optional
       placement it also marks the move incomplete so the trailing
       「そうしたとき」 consequence is skipped. */
    if (n <= 0) {
        if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
            if (e->is_optional) g->queue.entries[g->queue.cur].optional_cost_result = 0;
            if (g->queue.entries[g->queue.cur].optional_moves_all_moved >= 0)
                g->queue.entries[g->queue.cur].optional_moves_all_moved = 0;
        }
        return;
    }

    /* Rust move_cards.rs:2030-2037 — distinct card-name filter; if the deduped
       take is short of `count` the whole move is dropped. */
    if (cmf_distinct_dedupe(e)) {
        int deduped[RB_MAX_ZONE];
        int dn = rb_dedupe_by_normalized_name(ids, n, deduped, RB_MAX_ZONE);
        if (dn < place_count && raw_count > 0) return;
        for (int i = 0; i < dn; i++) ids[i] = deduped[i];
        n = dn;
    }

    int source_is_relay = !strcmp(source, "selected_cards") ||
                          !strcmp(source, "those_cards") ||
                          !strcmp(source, "recently_moved") ||
                          !strcmp(source, "preceding_moved") ||
                          !strcmp(source, "revealed_cards");
    const char *state = cmf_extra(e, "state_change");
    int is_max = extra_true(e, "max");
    int allow_occupied = extra_true(e, "allow_occupied_stage");
    int under_self = extra_true(e, "under_self");
    int deck_pos = cmf_effect_deck_pos(e);
    /* Rust move_cards.rs:1892/1960 — the vacated area is read once, before the
       per-card placement loop, and is the same value for every card. */
    int vacated = g->baton_last_vacated_area[pl];

    /* Rust move_cards.rs:1762-1784 — a `stage` destination with no free slot and
       no allow_occupied_stage sends the whole take to the waitroom (still counted
       as moved, so finalize runs and the movement event is recorded). */
    if (!strcmp(destination, "stage") && !allow_occupied) {
        int any_empty = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (g->p[pl].stage[i] == RB_EMPTY_SLOT) { any_empty = 1; break; }
        if (!any_empty) {
            int staged[RB_MAX_ZONE];
            for (int i = 0; i < n; i++) { rb_waitroom_add(&g->p[pl], ids[i]); staged[i] = ids[i]; }
            rb_move_finalize_card_movement(g, actor, staged, n, destination, source,
                                           state, e->target);
            return;
        }
    }

    int moved[RB_MAX_ZONE];
    int nm = 0;
    for (int i = 0; i < n; i++) {
        int cid = ids[i];
        if (source_is_relay) move_port_remove_any(g, pl, cid);
        if (rb_move_maybe_prompt_success_replacement(g, pl, cid, destination, e->target)) return;
        int placed = 0;
        if (!strcmp(destination, "same_area")) {
            placed = move_port_same_area(g, pl, cid, vacated);
        } else if (!strcmp(destination, "deck") && deck_pos >= 0 && !is_max) {
            /* Rust move_cards.rs:1795-1805 — an explicit `position` inserts the
               card at that deck index instead of going through the stage-choice
               placement path. */
            RbBag *deck = &g->p[pl].deck;
            if (deck->n < RB_MAX_ZONE) {
                int idx = deck_pos < deck->n ? deck_pos : deck->n;
                for (int k = deck->n; k > idx; k--) deck->cards[k] = deck->cards[k - 1];
                deck->cards[idx] = cid;
                deck->n++;
                placed = 1;
            }
        } else if (!strcmp(destination, "deck_top_or_bottom")) {
            rb_move_prompt_deck_top_or_bottom(g, actor, cid, e->target, source, e->is_optional);
            g->queue.resume_eff = e;
            g->queue.resume_actor = actor;
            return;
        } else {
            int r = rb_move_place_card_with_stage_choice(g, actor, -1, e->target,
                cid, destination, vacated, is_max, place_count, state, deck_pos, source,
                allow_occupied, under_self);
            if (r == 1) return;  /* a sub-choice was issued; the queue resumes it */
            placed = (r == 0);
        }
        if (!placed) {
            rb_place_card_in_zone(g, pl, cid,
                                  !strcmp(source, "those_cards") ? "discard" : source, -1);
            continue;
        }
        rb_mods_clear_card(&g->mods, cid);
        moved[nm++] = cid;
    }
    if (nm > 0)
        rb_move_finalize_card_movement(g, actor, moved, nm, destination, source,
                                       state, e->target);
    /* Rust move_cards.rs:2316-2321 — the debut side effects are fired by
       finalize_card_movement itself, and only for Zone::Stage. Firing them here
       as well double-counted every 「登場」 for a `stage` destination. */
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


