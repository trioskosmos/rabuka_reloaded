/* compound.c — sequential / conditional / alternative ability execution.
   Mirror engine/src/ability/compound.rs (execute_sequential_effect,
   route_conditional_branch, execute_conditional_alternative,
   execute_conditional_on_result, execute_conditional_on_optional,
   handle_choice_string_selection, execute_choice_action).

   STUBS: these mirror the Rust function names but currently call only the
   primitive rb_execute_effect / rb_eval_condition. The richer Rust logic
   (sub-effect grouping, action_kind dispatch) lives in engine.c and will be
   migrated here function-by-function. Bodies are correct shape, minimal. */

#include "rabuka.h"
#include <string.h>

/* Mirror compound.rs:execute_sequential_effect — run effects[0..n] in order. */
int rb_compound_sequential(GameState *g, int actor, const RbPlayer *self,
                           const AbilityEffect *effects, int n, int *resolved) {
    (void)self;
    int ok = 1;
    for (int i = 0; i < n; i++)
        rb_execute_effect(g, actor, (AbilityEffect *)&effects[i]);
    if (resolved) *resolved = ok;
    return ok;
}

/* Mirror compound.rs:route_conditional_branch — pick the matching branch of
   a conditional effect. Returns branch index (0 or 1). */
int rb_compound_route_branch(const GameState *g, int actor, const AbilityEffect *eff) {
    if (!eff->condition) return 1;
    return rb_eval_condition(g, actor, eff->condition) ? 0 : 1;
}

/* Mirror compound.rs:execute_conditional_alternative — run the chosen
   alternative subtree (eff->child[branch]). */
int rb_compound_conditional_alternative(GameState *g, int actor, const RbPlayer *self,
                                        const AbilityEffect *eff, int branch, int *resolved) {
    (void)self;
    if (branch < 0 || branch >= eff->n_child) { if (resolved) *resolved = 0; return 0; }
    rb_execute_effect(g, actor, eff->child[branch]);
    if (resolved) *resolved = 1;
    return 1;
}

/* Mirror compound.rs:execute_conditional_on_result — "if last effect's
   result matches, run consequent". */
int rb_compound_conditional_on_result(GameState *g, int actor, const RbPlayer *self,
                                      const AbilityEffect *eff, int last_result, int *resolved) {
    (void)last_result;
    int branch = rb_compound_route_branch(g, actor, eff);
    return rb_compound_conditional_alternative(g, actor, self, eff, branch, resolved);
}

/* Mirror compound.rs:execute_conditional_on_optional — run consequent only
   if the optional choice was taken. */
int rb_compound_conditional_on_optional(GameState *g, int actor, const RbPlayer *self,
                                        const AbilityEffect *eff, int taken, int *resolved) {
    if (!taken) { if (resolved) *resolved = 1; return 1; }
    int branch = rb_compound_route_branch(g, actor, eff);
    return rb_compound_conditional_alternative(g, actor, self, eff, branch, resolved);
}

/* Mirror compound.rs:handle_choice_string_selection — map a player's chosen
   string to the branch index. Returns -1 if not found. */
int rb_compound_choice_string(const AbilityEffect *eff, const char *choice) {
    if (!eff || !choice) return -1;
    for (int i = 0; i < eff->n_child; i++) {
        if (eff->child[i]->action && !strcmp(eff->child[i]->action, choice))
            return i;
    }
    return -1;
}

/* Mirror compound.rs:execute_choice_action — run the selected choice. */
int rb_compound_choice_action(GameState *g, int actor, const RbPlayer *self,
                              const AbilityEffect *eff, int choice_idx, int *resolved) {
    (void)self;
    if (choice_idx < 0 || choice_idx >= eff->n_child) { if (resolved) *resolved = 0; return 0; }
    rb_execute_effect(g, actor, eff->child[choice_idx]);
    if (resolved) *resolved = 1;
    return 1;
}
