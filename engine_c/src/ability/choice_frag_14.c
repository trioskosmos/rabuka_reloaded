/* engine_c/src/ability/choice_frag_14.c
 *
 * Port fragment of engine/src/ability/choice.rs (~lines 3381-3445):
 *   - clear_choice_meta                     -> rb_resolver_clear_choice_meta
 *   - clear_choice_state                    -> rb_resolver_clear_choice_state
 *   - clear_choice_state_and_resume         -> rb_resolver_clear_choice_state_and_resume
 *   - set_chosen_target                     -> rb_set_chosen_target
 *
 * Rust path notes:
 *   - The Rust methods live on `AbilityResolver` (`self`). `self.gs` becomes a
 *     `GameState*` here. Resolver-local fields that the C engine does not store
 *     on a struct (sub_choice_created, pending_choice, pending_deferred_costs)
 *     are mirrored with module-scope statics (one resolver instance per process).
 *   - `gs.ability_queue.current_entry_mut()` maps to `&g->queue.entries[g->queue.cur]`.
 *     The C `RbQueueEntry` has no `choice_card_no` / `conditional_choice` fields, so
 *     clearing them is a documented no-op on the C side.
 *   - `self.pending_choice = None` maps to `rb_clear_pending_choice(g)`.
 *   - `clear_choice_state_and_resume` ends by resuming; it calls
 *     `rb_resolver_resume_execution` (owned elsewhere) by name — prototype below,
 *     not defined here.
 */

#include "rabuka.h"
#include <string.h>

/* ── Resolver-local state (mirrors AbilityResolver fields with no C struct) ── */
static int g_sub_choice_created = 0;        /* AbilityResolver::sub_choice_created */
static int g_pending_deferred_costs_n = 0;  /* AbilityResolver::pending_deferred_costs (count) */

/* Forward-declared helpers owned by other translation units. */
void rb_resolver_resume_execution(GameState *g);

/* Prototypes for the functions defined in this fragment. */
static void rb_resolver_clear_choice_meta(GameState *g);
void        rb_resolver_clear_choice_state(GameState *g);
void        rb_resolver_clear_choice_state_and_resume(GameState *g);
void        rb_set_chosen_target(AbilityEffect *effect, const char *target);

/* clear_choice_meta — Rust: clear the current queue entry's choice bookkeeping
   and drop any deferred costs so they are not paid on a skip/clear. */
static void rb_resolver_clear_choice_meta(GameState *g) {
    if (!g) return;
    RbAbilityQueue *q = &g->queue;
    /* Rust: if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.choice_card_no = None; entry.conditional_choice = None; }
       RbQueueEntry carries no choice_card_no / conditional_choice field in C,
       so there is no per-entry state to reset here (documented mirror). */
    (void)q;
    /* On skip/clear, drop deferred costs so they aren't paid. */
    g_pending_deferred_costs_n = 0;
}

/* clear_choice_state — Rust: if sub_choice_created { sub_choice_created=false }
   else { pending_choice=None }; then clear_choice_meta. */
void rb_resolver_clear_choice_state(GameState *g) {
    if (!g) return;
    if (g_sub_choice_created) {
        g_sub_choice_created = 0;           /* sub-choice owns its own cleanup */
    } else {
        rb_clear_pending_choice(g);          /* mirrors self.pending_choice = None */
    }
    rb_resolver_clear_choice_meta(g);
}

/* clear_choice_state_and_resume — Rust: clear_choice_state; resume_pending_actions.
   Returns 0 on success. */
void rb_resolver_clear_choice_state_and_resume(GameState *g) {
    if (!g) return;
    rb_resolver_clear_choice_state(g);
    /* mirrors self.resume_pending_actions(gs) — re-enters the resolver FSM. */
    rb_resolver_resume_execution(g);
}

/* set_chosen_target — Rust: recursively pin `target` onto every sub-effect that
   does not already have an explicit target. DrawCard / SelectCards are excluded
   (they always target self) and are NOT recursed into. */
void rb_set_chosen_target(AbilityEffect *effect, const char *target) {
    if (!effect || !target) return;

    /* Excludes draw_card / select_cards (always target self); return before
       setting or recursing, matching the Rust early `return`. */
    if (effect->action &&
        (!strcmp(effect->action, "draw_card") ||
         !strcmp(effect->action, "select_cards"))) {
        return;
    }

    if (effect->target == NULL || strcmp(effect->target, "self") == 0) {
        if (effect->target) rb_free(effect->target);
        effect->target = rb_strdup2(target);
    }

    /* Recurse into compound sub-effects (Rust effect.compound.*). The C decoder
       folds look_action / select_action / actions / effect_steps into child[],
       so we walk child[] for those; the explicit compound pointers cover the
       named branches. alternative_effect is intentionally omitted to match Rust. */
    if (effect->optional_action)   rb_set_chosen_target(effect->optional_action, target);
    if (effect->primary_effect)    rb_set_chosen_target(effect->primary_effect, target);
    if (effect->conditional_action) rb_set_chosen_target(effect->conditional_action, target);
    if (effect->followup_action)   rb_set_chosen_target(effect->followup_action, target);

    for (int i = 0; i < effect->n_child; i++) {
        if (effect->child[i]) rb_set_chosen_target(effect->child[i], target);
    }
}
