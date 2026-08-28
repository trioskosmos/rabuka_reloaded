/* cost.c — ability cost payment.
   Mirror engine/src/ability/cost.rs (pay_deferred_costs, validate_cost,
   pay_cost, pay_cost_inner, handle_optional_cost_payment, get_change_state_
   candidates, has_skip_prompt).

   The C decoder stores an ability's cost as an AbilityEffect. Its `action`
   is the gate: "sequential"/"sequential_cost" wraps sub-costs in child[],
   while a single cost (pay_energy / change_state / move_cards / ...) carries
   its own `action`. This file recurses through sequential costs and pays the
   leaf costs. Optional costs are auto-skipped in the headless model (the host
   would have offered a pay/skip prompt); this matches cost.rs' skip path. */

#include "rabuka.h"
#include <string.h>

/* local bag helpers (RbBag is a plain int array) */
static void bag_push(RbBag *b, int c) { if (b->n < RB_MAX_ZONE) b->cards[b->n++] = c; }
static int  bag_pop(RbBag *b) { return b->n > 0 ? b->cards[--b->n] : -1; }

/* Is cost-component `e` an energy payment? */
static int cost_is_energy(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "pay_energy")) return 1;
    const char *t = e->target;
    return t && strstr(t, "energy") != NULL;
}
/* Is cost-component `e` a change-state (put a member to wait) payment? */
static int cost_is_change_state(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "change_state")) return 1;
    const char *t = e->target;
    return t && strstr(t, "wait") != NULL;
}

/* Mirror cost.rs: get_change_state_candidates — list stage positions whose
   member can be put into wait state to satisfy a change_state cost.
   Fills out_positions (cap max) and returns the count. Rust filters by
   orientation == "active"; the C model tracks that with stage_wait==0. */
int rb_get_change_state_candidates(const GameState *g, int actor,
                                    int *out_positions, int max) {
    const RbPlayer *P = &g->p[actor];
    int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE && n < max; i++) {
        if (P->stage[i] != RB_EMPTY_SLOT && !P->stage_wait[i])
            out_positions[n++] = i;
    }
    return n;
}

/* Mirror cost.rs: has_skip_prompt */
static int cost_has_skip_prompt(const AbilityEffect *cost) {
    if (!cost) return 0;
    if (cost_is_energy(cost)) return 1;            /* pay_energy w/o any_number */
    if (cost_is_change_state(cost)) return cost->is_optional;
    return 0;
}

/* Resolve a source-zone wire name to the player's bag (stage handled separately). */
static RbBag *cost_source_bag(RbPlayer *P, const char *src) {
    if (!src) return NULL;
    if (!strcmp(src, "hand")) return &P->hand;
    if (!strcmp(src, "deck") || !strcmp(src, "deck_top")) return &P->deck;
    if (!strcmp(src, "waitroom") || !strcmp(src, "discard")) return &P->discard;
    if (!strcmp(src, "energy")) return &P->energy;
    return NULL;
}
static int cost_count_in_source(const GameState *g, int actor, const char *src) {
    const RbPlayer *P = &g->p[actor];
    if (!src) return 0;
    if (!strcmp(src, "stage")) {
        int n = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) n++;
        return n;
    }
    RbBag *b = cost_source_bag((RbPlayer *)P, src);
    return b ? b->n : 0;
}
/* Move up to `count` cards from a source zone to the actor's discard (Rust
   pay_cost_move_cards execution path for a cost). Returns cards moved. */
static int cost_move_from_source(GameState *g, int actor, const char *src, int count) {
    RbPlayer *P = &g->p[actor];
    int moved = 0;
    if (!src) return 0;
    if (!strcmp(src, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE && moved < count; i++) {
            if (P->stage[i] != RB_EMPTY_SLOT) {
                int cid = P->stage[i];
                P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0;
                bag_push(&P->discard, cid); moved++;
            }
        }
        return moved;
    }
    RbBag *b = cost_source_bag(P, src);
    if (!b) return 0;
    while (moved < count && b->n > 0) {
        int cid = bag_pop(b);
        bag_push(&P->discard, cid);
        moved++;
    }
    return moved;
}

static int cost_is_sequential(const AbilityEffect *e) {
    return e->action &&
           (!strcmp(e->action, "sequential") || !strcmp(e->action, "sequential_cost"));
}

/* Mirror cost.rs:validate_cost (single cost). Returns 1 if payable. */
static int validate_one(const GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;
    if (cost_is_sequential(cost)) {
        for (int i = 0; i < cost->n_child; i++)
            if (!validate_one(g, actor, cost->child[i])) return 0;
        return 1;
    }
    if (cost_is_energy(cost)) {
        int need = cost->count > 0 ? cost->count : 1;
        return g->p[actor].energy_active >= need;
    }
    if (cost_is_change_state(cost)) {
        const char *sc = NULL;
        for (int i = 0; i < cost->n_extra; i++)
            if (cost->extra_k[i] && !strcmp(cost->extra_k[i], "state_change")) sc = cost->extra_v[i];
        if (sc && !strcmp(sc, "wait")) {
            int pos[RB_STAGE_SIZE];
            return rb_get_change_state_candidates(g, actor, pos, RB_STAGE_SIZE) > 0;
        }
        return 1;
    }
    if (cost->action && !strcmp(cost->action, "move_cards")) {
        const char *src = cost->source ? cost->source : "";
        int count = cost->count > 0 ? cost->count : 1;
        return cost_count_in_source(g, actor, src) >= count;
    }
    return 1; /* pay/unconditional costs always payable */
}

/* Mirror cost.rs:validate_cost */
int rb_validate_cost(const GameState *g, int actor, const AbilityEffect *cost) {
    return validate_one(g, actor, cost);
}

/* Mirror cost.rs:pay_cost_inner (single cost). Returns 1 on success. */
static int pay_one(GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;
    if (cost_is_sequential(cost)) {
        for (int i = 0; i < cost->n_child; i++)
            if (!pay_one(g, actor, cost->child[i])) return 0;
        return 1;
    }
    /* Optional costs are auto-skipped headless (skip path in cost.rs). */
    if (cost->is_optional) return 1;

    if (cost_is_energy(cost)) {
        int amt = cost->count > 0 ? cost->count : 1;
        RbPlayer *P = &g->p[actor];
        P->energy_active -= amt;
        if (P->energy_active < 0) P->energy_active = 0;
        rb_recalc_constants(g);
        return 1;
    }
    if (cost_is_change_state(cost)) {
        const char *sc = NULL;
        for (int i = 0; i < cost->n_extra; i++)
            if (cost->extra_k[i] && !strcmp(cost->extra_k[i], "state_change")) sc = cost->extra_v[i];
        if (sc && !strcmp(sc, "wait")) {
            int pos[RB_STAGE_SIZE];
            int n = rb_get_change_state_candidates(g, actor, pos, RB_STAGE_SIZE);
            if (n > 0) g->p[actor].stage_wait[pos[0]] = 1;
        }
        return 1;
    }
    if (cost->action && !strcmp(cost->action, "move_cards")) {
        const char *src = cost->source ? cost->source : "";
        int count = cost->count > 0 ? cost->count : 1;
        cost_move_from_source(g, actor, src, count);
        return 1;
    }
    return 1;
}

/* Mirror cost.rs:pay_cost */
int rb_pay_cost(GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;
    if (!rb_validate_cost(g, actor, cost)) return 0;
    return pay_one(g, actor, cost);
}

/* Mirror cost.rs:pay_deferred_costs — settle deferred (post-effect) costs. */
int rb_pay_deferred_costs(GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;
    return rb_pay_cost(g, actor, cost);
}

/* Mirror cost.rs:handle_optional_cost_payment — pay the optional cost if the
   player chose to (pay != 0); skip otherwise. Returns the chosen flag. */
int rb_handle_optional_cost_payment(GameState *g, int actor, const AbilityEffect *cost, int pay) {
    if (pay && cost) rb_pay_cost(g, actor, cost);
    return pay;
}

/* Mirror cost.rs:has_skip_prompt — does this cost carry a "may skip" prompt? */
int rb_cost_has_skip_prompt(const AbilityEffect *cost) {
    if (!cost) return 0;
    return cost_has_skip_prompt(cost);
}
