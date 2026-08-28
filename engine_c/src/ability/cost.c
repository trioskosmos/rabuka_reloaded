/* cost.c — ability cost payment.
   Mirror engine/src/ability/cost.rs (pay_deferred_costs, validate_cost,
   pay_cost, handle_optional_cost_payment, get_change_state_candidates,
   has_skip_prompt).

   The C decoder stores an ability's cost as an AbilityEffect whose child[]
   are the individual cost components. Each component has an `action` /
   `target` describing what is paid (energy, a member put to wait, etc.) and
   a `count`. This file implements the energy + change-state cost path; other
   resource costs fall through as TODO (still return paid). */

#include "rabuka.h"
#include <string.h>

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

/* Mirror cost.rs:validate_cost — does `actor` have the resources? */
int rb_validate_cost(const GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;
    const RbPlayer *P = &g->p[actor];
    int need_energy = 0;
    for (int i = 0; i < cost->n_child; i++) {
        const AbilityEffect *e = cost->child[i];
        if (cost_is_energy(e)) need_energy += e->count > 0 ? e->count : 1;
        else if (cost_is_change_state(e)) {
            int pos[RB_STAGE_SIZE];
            if (rb_get_change_state_candidates(g, actor, pos, RB_STAGE_SIZE) == 0)
                return 0; /* no member available to put to wait */
        }
    }
    return P->energy_active >= need_energy;
}

/* Mirror cost.rs:pay_cost — deduct the cost; returns 1 on success. */
int rb_pay_cost(GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;
    if (!rb_validate_cost(g, actor, cost)) return 0;
    RbPlayer *P = &g->p[actor];
    for (int i = 0; i < cost->n_child; i++) {
        AbilityEffect *e = cost->child[i];
        if (cost_is_energy(e)) {
            int amt = e->count > 0 ? e->count : 1;
            P->energy_active -= amt;
            if (P->energy_active < 0) P->energy_active = 0;
        } else if (cost_is_change_state(e)) {
            int pos[RB_STAGE_SIZE];
            int n = rb_get_change_state_candidates(g, actor, pos, RB_STAGE_SIZE);
            if (n > 0) P->stage_wait[pos[0]] = 1; /* put first candidate to wait */
        }
        /* TODO: other resource / card-discard cost components */
    }
    return 1;
}

/* Mirror cost.rs:pay_deferred_costs — settle any deferred (post-effect)
   costs. Returns 1 on success. */
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

/* Mirror cost.rs:has_skip_prompt — does this cost carry a "may skip"
   prompt? */
int rb_cost_has_skip_prompt(const AbilityEffect *cost) {
    if (!cost) return 0;
    return cost->is_optional;
}

/* Mirror cost.rs:get_change_state_candidates — list stage positions whose
   member can be put into wait state to satisfy a change_state cost.
   Fills out_positions (cap max) and returns the count. */
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
