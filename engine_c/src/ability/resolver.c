/* resolver.c — ability activation / choice resolution frontend.
   Mirror engine/src/ability/resolver.rs (resolver_get_pending_choice,
   can_activate_effect, resolve_ability, get_trigger_ability_infos,
   card_matches_type).

   STUBS: the authoritative activation/payment logic currently lives in
   engine.c. These wrappers mirror the Rust resolver names and will be
   filled in as the activation pipeline is ported. */

#include "rabuka.h"
#include <string.h>

/* Mirror resolver.rs:resolver_get_pending_choice — return the index of the
   effect currently awaiting an interactive choice, or -1. */
int rb_resolver_pending_choice(const GameState *g) {
    if (!g || !g->queue.has_pending) return -1;
    return (int)g->queue.pending.route;
}

/* Mirror resolver.rs:can_activate_effect — is this effect activatable now?
   Rust evaluates the effect's `condition` (the on-activation gate) and skips
   it when it fails; ConditionalAlternative's condition is a branch selector,
   not a gate, so it is exempt. The activation-position/activation_condition
   merge and cost-paid fast-path are not yet ported (cost is paid by the
   caller via rb_validate_cost); the Main-phase gate mirrors the headless
   auto-activation timing. */
int rb_can_activate_effect(const GameState *g, int actor, const AbilityEffect *eff) {
    if (g->phase != RB_PHASE_MAIN) return 0;
    if (!eff) return 1;
    if (eff->condition) {
        /* ConditionalAlternative's condition picks the branch, it is not a
           gate (mirrors resolver.rs: skip gate for that action type). */
        if (eff->action && !strcmp(eff->action, "conditional_alternative"))
            return 1;
        return rb_eval_condition(g, actor, eff->condition);
    }
    return 1;
}

/* Mirror resolver.rs:get_trigger_ability_infos — collect abilities whose
   trigger matches `trigger` across the actor's controlled zones. Fills out
   (cap max), returns the count. */
int rb_resolver_trigger_infos(const GameState *g, int actor, const char *trigger,
                               AbilityInfo *out, int max) {
    if (!g || !trigger || !out || max <= 0) return 0;
    int n = 0;
    const RbPlayer *P = &g->p[actor];
    /* stage, success, live, hand, energy */
    int zone[RB_STAGE_SIZE + RB_MAX_LIVE_CARDS*2 + RB_MAX_HAND + RB_MAX_ENERGY_CARDS];
    int zn = 0;
    for (int s = 0; s < RB_STAGE_SIZE; s++) if (P->stage[s] >= 0) zone[zn++] = P->stage[s];
    for (int s = 0; s < P->success.n; s++) zone[zn++] = P->success.cards[s];
    for (int s = 0; s < P->live.n; s++)    zone[zn++] = P->live.cards[s];
    for (int s = 0; s < P->hand.n; s++)    zone[zn++] = P->hand.cards[s];
    for (int s = 0; s < P->energy.n; s++)  zone[zn++] = P->energy.cards[s];
    for (int z = 0; z < zn && n < max; z++) {
        int cid = zone[z];
        int nab = rb_card_num_abilities((uint32_t)cid);
        for (int a = 0; a < nab && n < max; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            if (rb_ability_matches_trigger(&ab, trigger)) {
                out[n].cid = cid;
                out[n].ability_idx = a;
                out[n].trigger = trigger;
                n++;
            }
            rb_free_ability(&ab);
        }
    }
    return n;
}

/* Mirror resolver.rs:resolve_ability — run a single ability's effects. */
int rb_resolve_ability(GameState *g, int actor, const AbilityEffect *eff, int *resolved) {
    return rb_compound_sequential(g, actor, eff, -1);
}

/* Mirror resolver.rs:card_matches_type — selector card-type filter.
   Delegates to util.c. */
int rb_resolver_card_matches_type(int cid, const char *filter) {
    return rb_card_matches_type(cid, filter);
}
