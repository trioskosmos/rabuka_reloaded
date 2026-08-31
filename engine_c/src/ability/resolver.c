/* resolver.c — ability activation / choice resolution frontend.
   Mirror engine/src/ability/resolver.rs.

   The Rust `AbilityResolver` orchestrator (resolve_ability / can_activate_effect
   / cost payment / use_limit / pending-choice routing) is reproduced here against
   the C data model: the persistent resolver state lives in `GameState` (the
   ability queue FSM in choice.c/engine.c), and the activating card is the
   `host_cid` threaded through effect execution. */

#include "rabuka.h"
#include <string.h>

/* ── resolver.rs::get_pending_choice ──
   Return the pending choice, or NULL when none is awaiting input. */
const RbChoice *rb_resolver_get_pending_choice(const GameState *g) {
    if (!g || !g->queue.has_pending) return NULL;
    return &g->queue.pending;
}

/* ── resolver.rs::resolver_get_pending_choice (index form, kept for API parity) ──
   Return the route/index of the effect awaiting an interactive choice, or -1. */
int rb_resolver_pending_choice(const GameState *g) {
    if (!g || !g->queue.has_pending) return -1;
    return (int)g->queue.pending.route;
}

/* ── resolver.rs::current_ability_is_activation ──
   Whether the ability's trigger set contains 起動 (Activation). Single source of
   truth for the optional-cost activation check. */
int rb_resolver_current_ability_is_activation(const Ability *ab) {
    return ab && ab->triggers && strstr(ab->triggers, "起動") != NULL;
}

/* ── resolver.rs::zone_for_card ──
   Which zone a card currently occupies (stage / live / success / hand / waitroom
   / energy), or "?" if it is nowhere. Mirrors the per-player scan in resolver.rs. */
const char *rb_resolver_zone_for_card(const GameState *g, int card_id) {
    if (!g) return "?";
    for (int pl = 0; pl < 2; pl++) {
        const RbPlayer *P = &g->p[pl];
        for (int s = 0; s < RB_STAGE_SIZE; s++)
            if (P->stage[s] == card_id) return "stage";
        for (int i = 0; i < P->live.n; i++)
            if (P->live.cards[i] == card_id) return "live_card_zone";
        for (int i = 0; i < P->success.n; i++)
            if (P->success.cards[i] == card_id) return "success_live_card_zone";
        for (int i = 0; i < P->hand.n; i++)
            if (P->hand.cards[i] == card_id) return "hand";
        for (int i = 0; i < P->discard.n; i++)
            if (P->discard.cards[i] == card_id) return "waitroom";
        for (int i = 0; i < P->energy.n; i++)
            if (P->energy.cards[i] == card_id) return "energy_zone";
    }
    return "?";
}

/* ── resolver.rs::check_use_limit_reached ──
   True when the ability has already been used `use_limit`+ times this turn. */
int rb_resolver_use_limit_reached(const GameState *g, int card_id,
                                   int ability_index, int use_limit) {
    if (!g || use_limit <= 0) return 0;
    return rb_ability_uses_used(g, card_id, ability_index) >= use_limit;
}

/* Read the activation_position wire value off an effect (extra_kv), if present.
   Mirrors Rust effect.activation_position_any(). */
static const char *rb_effect_activation_position(const AbilityEffect *e) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "activation_position"))
            return e->extra_v[i];
    }
    return NULL;
}

/* Stage index (0..2) of a card for `actor` or the opponent, or -1. */
static int rb_stage_index_of_card(const GameState *g, int actor, int cid) {
    for (int pl = 0; pl < 2; pl++) {
        if (actor >= 0 && pl != actor) continue;
        for (int s = 0; s < RB_STAGE_SIZE; s++)
            if (g->p[pl].stage[s] == cid) return s;
    }
    return -1;
}

/* ── resolver.rs::can_activate_effect (faithful gate) ──
   Evaluates the effect's activation gate: the standalone activation_position
   restriction (Q240) when there is no condition, and the effect `condition`
   itself (ConditionalAlternative's condition is a branch selector, not a gate).
   `host_cid` is the activating card (Rust `gs.activating_card`). */
int rb_can_activate_effect(const GameState *g, int actor,
                            const AbilityEffect *eff, int host_cid) {
    if (!g || !eff) return 1;

    /* Standalone activation_position check (Q240): when the effect has an
       activation_position but no condition, the merge paths never fire, so
       check the position directly. */
    const char *actpos = rb_effect_activation_position(eff);
    if (!eff->condition && actpos && host_cid >= 0) {
        int pos = rb_stage_index_of_card(g, actor, host_cid);
        if (pos < 0) return 0;
        int passes = 0;
        char buf[64];
        strncpy(buf, actpos, sizeof(buf) - 1);
        buf[sizeof(buf) - 1] = '\0';
        char *tok = strtok(buf, ",");
        while (tok) {
            int idx = rb_activation_position_index(tok);
            if (idx >= 0 && idx < RB_STAGE_SIZE && idx == pos) { passes = 1; break; }
            tok = strtok(NULL, ",");
        }
        if (!passes) return 0;
    }

    if (eff->condition) {
        /* ConditionalAlternative's condition picks the branch, it is not a gate. */
        if (eff->action && !strcmp(eff->action, "conditional_alternative"))
            return 1;
        return rb_eval_condition_for_host(g, actor, host_cid, eff->condition);
    }
    return 1;
}

/* ── resolver.rs::get_trigger_ability_infos ──
   Collect abilities whose trigger matches `trigger` across the actor's controlled
   zones. Fills out (cap max), returns the count. */
int rb_resolver_trigger_infos(const GameState *g, int actor, const char *trigger,
                               AbilityInfo *out, int max) {
    if (!g || !trigger || !out || max <= 0) return 0;
    int n = 0;
    const RbPlayer *P = &g->p[actor];
    int zn = 0;
    int zone[RB_STAGE_SIZE + RB_MAX_LIVE_CARDS * 2 + RB_MAX_HAND + RB_MAX_ENERGY_CARDS];
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

/* ── resolver.rs::resolve_ability (orchestrator) ──
   Mirrors the Rust control flow: use_limit guard → pay cost → condition gate →
   execute effect → record use. `host_cid` is the activating card (Rust
   `activating_card`); `resolved` is set to 1 when the effect ran. Cost/position
   keyword gating is handled by the caller's queue FSM (engine.c::rb_activate_card)
   for the interactive path; this is the headless resolution entry. */
int rb_resolve_ability(GameState *g, int actor, const Ability *ab,
                        int ability_idx, int host_cid, int *resolved) {
    if (resolved) *resolved = 0;
    if (!g || !ab) return 0;

    if (ab->use_limit > 0) {
        if (rb_resolver_use_limit_reached(g, host_cid, ability_idx, ab->use_limit))
            return 0;
    }

    if (ab->cost) {
        if (!rb_pay_cost(g, actor, ab->cost))
            return 0;
    }

    if (ab->effect) {
        if (!rb_can_activate_effect(g, actor, ab->effect, host_cid))
            return 0;
        rb_execute_effect_ex(g, actor, ab->effect, host_cid);
    }

    if (ab->use_limit > 0)
        rb_record_ability_use(g, host_cid, ability_idx);

    if (resolved) *resolved = 1;
    return 1;
}

/* ── resolver.rs::card_matches_type ──
   Selector card-type filter. Delegates to util.c. */
int rb_resolver_card_matches_type(int cid, const char *filter) {
    return rb_card_matches_type(cid, filter);
}
