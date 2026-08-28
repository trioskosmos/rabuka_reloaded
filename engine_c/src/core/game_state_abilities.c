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

/* Mirror abilities.rs:collect_constant_hand — push the constant-amount
   hand-modifier effects of all active abilities into `out` (cap max).
   Returns the count. Currently returns 0 (no constant modifiers tracked). */
int rb_collect_constant_hand(const GameState *g, int actor, AbilityEffect *out, int max) {
    (void)g; (void)actor; (void)out; (void)max;
    return 0;
}

/* Mirror abilities.rs:collect_live_ability_modifiers — gather temporary
   modifiers applied during a live. Returns 0 (not tracked yet). */
int rb_collect_live_modifiers(const GameState *g, int actor, AbilityEffect *out, int max) {
    (void)g; (void)actor; (void)out; (void)max;
    return 0;
}

/* Mirror abilities.rs:trigger_auto_abilities_for_player — fire all
   auto-trigger abilities of `actor` matching `trigger`. Returns count
   fired. STUB: not yet driven. */
int rb_trigger_auto_abilities(GameState *g, int actor, const char *trigger) {
    (void)g; (void)actor; (void)trigger;
    return 0;
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

/* Mirror abilities.rs:apply_ability_effects — apply the persistent effects
   of an ability (constant modifiers). STUB. */
int rb_apply_ability_effects(GameState *g, int actor, const Ability *ab) {
    (void)g; (void)actor; (void)ab;
    return 0;
}
