/* game_state_abilities.c — auto-trigger engine, ability-use tracking, and
   temporary-effect expiry.
   Mirror engine/src/core/game_state/abilities.rs (ability_matches_trigger,
   record_ability_use, collect_constant_hand, collect_live_ability_modifiers,
   trigger_auto_abilities_for_player, process_pending_auto_abilities,
   check_expired_effects, apply_ability_effects).

   STUBS: the Rust engine drives auto-triggers (debut/on_live/on_resolve)
   and temporary effect expiry from here. The C port does neither yet —
   these signatures exist so the wiring can be added function-by-function.
   Defaults are permissive/no-op so the build stays green. */

#include "rabuka.h"
#include <string.h>

/* Mirror abilities.rs:ability_matches_trigger — does ability `ab` fire on
   `trigger`? Uses the decoded ability text trigger set. */
int rb_ability_matches_trigger(const Ability *ab, const char *trigger) {
    if (!ab || !trigger) return 0;
    return rb_trigger_is(ab->triggers, trigger);
}

/* Mirror abilities.rs:record_ability_use — mark `cid`'s `idx`-th ability as
   already used this turn (for once-per-turn gating). STUB: no per-turn
   use-tracking table yet; recorded into a module-local log only. */
static struct { int cid; int idx; } s_used[RB_MAX_USED];
static int s_n_used;
void rb_record_ability_use(GameState *g, int cid, int idx) {
    (void)g;
    if (s_n_used >= RB_MAX_USED) return;
    s_used[s_n_used].cid = cid;
    s_used[s_n_used].idx = idx;
    s_n_used++;
}

/* Apply one constant-modifier effect node to the per-card constant tables.
   The wire maps the resource kind via source/destination/action and the
   magnitude via count. Best-effort mapping (see PROGRESS §12). */
static void apply_constant_node(RbMods *m, int cid, const AbilityEffect *e) {
    if (!e) return;
    int delta = e->count != 0 ? e->count : 1;
    const char *kind = e->source ? e->source : (e->destination ? e->destination : e->action);
    if (!kind) return;
    if (strstr(kind, "score"))            rb_mods_add_score(m, cid, delta);
    else if (strstr(kind, "blade"))       rb_mods_add_blade(m, cid, delta);
    else if (strstr(kind, "heart"))       rb_mods_add_heart(m, cid, 0, delta);
    else if (strstr(kind, "need_heart"))  rb_mods_add_need_heart(m, cid, 0, delta);
    else if (strstr(kind, "cost"))        rb_mods_add_cost(m, cid, delta);
    /* recurse into sub-nodes (conditional constant abilities) */
    for (int i = 0; i < e->n_child; i++) apply_constant_node(m, cid, e->child[i]);
}

/* Mirror abilities.rs:collect_constant_hand — scan the actor's stage members,
   decode their triggerless/constant abilities, and apply their constant
   modifiers into g->mods. Returns the number of constant abilities found. */
int rb_collect_constant_hand(const GameState *g, int actor, AbilityEffect *out, int max) {
    (void)out; (void)max;
    if (!g) return 0;
    int found = 0;
    const RbPlayer *P = &g->p[actor];
    for (int s = 0; s < RB_STAGE_SIZE; s++) {
        int cid = P->stage[s];
        if (cid < 0) continue;
        int nab = rb_card_num_abilities((uint32_t)cid);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            int is_constant = (ab.triggers == NULL || ab.triggers[0] == '\0' ||
                               strstr(ab.triggers, "constant") != NULL ||
                               strstr(ab.triggers, "continuous") != NULL);
            if (is_constant && ab.effect) {
                apply_constant_node((RbMods *)&g->mods, cid, ab.effect);
                found++;
            }
            rb_free_ability(&ab);
        }
    }
    return found;
}

/* Mirror abilities.rs:collect_live_ability_modifiers — gather temporary
   modifiers applied during a live. Returns 0 (not tracked yet). */
int rb_collect_live_modifiers(const GameState *g, int actor, AbilityEffect *out, int max) {
    (void)g; (void)actor; (void)out; (void)max;
    return 0;
}

/* Fire every ability on the cards `ids[0..n]` whose trigger matches. */
static int fire_zone_abilities(GameState *g, int actor, const int *ids, int n,
                                const char *trigger) {
    int fired = 0;
    for (int i = 0; i < n; i++) {
        int cid = ids[i];
        if (cid < 0) continue;
        int nab = rb_card_num_abilities((uint32_t)cid);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            if (rb_ability_matches_trigger(&ab, trigger)) {
                if (ab.effect)
                    rb_compound_sequential(g, actor, &g->p[actor], ab.effect, 1, NULL, cid);
                fired++;
            }
            rb_free_ability(&ab);
        }
    }
    return fired;
}

/* Mirror abilities.rs:trigger_auto_abilities_for_player — fire all
   auto-trigger abilities of `actor` matching `trigger` across the actor's
   stage, success-live, live, hand and energy zones. Returns count fired. */
int rb_trigger_auto_abilities(GameState *g, int actor, const char *trigger) {
    if (!g || !trigger) return 0;
    int total = 0;
    const RbPlayer *P = &g->p[actor];
    total += fire_zone_abilities(g, actor, P->stage, RB_STAGE_SIZE, trigger);
    total += fire_zone_abilities(g, actor, P->success.cards, P->success.n, trigger);
    total += fire_zone_abilities(g, actor, P->live.cards, P->live.n, trigger);
    total += fire_zone_abilities(g, actor, P->hand.cards, P->hand.n, trigger);
    total += fire_zone_abilities(g, actor, P->energy.cards, P->energy.n, trigger);
    return total;
}

/* Mirror abilities.rs:process_pending_auto_abilities — drain the queue of
   deferred auto-triggers. Returns count processed. */
int rb_process_pending_auto_abilities(GameState *g) {
    (void)g;
    return 0;
}

/* Mirror abilities.rs:check_expired_effects — expire temporary effects whose
   duration has elapsed (end of turn / end of live). Definition lives in
   turn/triggers.c (shared with the live/turn pipeline); declared in rabuka.h. */

/* Mirror abilities.rs:apply_ability_effects — run the persistent / on-trigger
   effects of an ability. The engine's rb_execute_effect already handles the
   action dispatch; here we drive the decoded effect tree via the compound
   sequential runner. */
int rb_apply_ability_effects(GameState *g, int actor, const Ability *ab, int host_cid) {
    if (!g || !ab || !ab->effect) return 0;
    rb_compound_sequential(g, actor, &g->p[actor], ab->effect, 1, NULL, host_cid);
    return 1;
}
