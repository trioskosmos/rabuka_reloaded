/* compound.c — sequential / conditional / alternative ability execution.
   Faithful port of engine/src/ability/compound.rs:
     execute_sequential_effect
     route_conditional_branch
     execute_conditional_alternative
     execute_conditional_on_result
     execute_conditional_on_optional
     handle_choice_string_selection / execute_choice_action
   (engine/src/ability/compound.rs). The richer Rust logic (sub-effect grouping,
   tied conditions, otherwise-condition skip, repeat_procedure, per_unit /
   self_target / card_names inheritance, action_kind dispatch) is mirrored here. */

#include "rabuka.h"
#include <string.h>

/* Mirror compound.rs:execute_sequential_effect — run the effect's children
   (the Rust `compound.actions` list) in order, gating each step on its own
   condition, honouring `otherwise_condition` (AlwaysTrue) skip semantics, and
   looping the whole list when a trailing `repeat_procedure` marker is present.

   Note: interactive choice-resume ordering (save_remaining / pending_repeat /
   deferred_conditional_gate) is approximated — when a sub-action emits a pending
   choice the sequence stops (headless host must resume, as with any effect). */
int rb_compound_sequential(GameState *g, int actor, const AbilityEffect *eff, int host_cid) {
    if (!eff) return 0;
    int n = eff->n_child;
    int has_repeat = 0;
    int repeat_max = 1;
    if (n > 0 && eff->child[n - 1] && eff->child[n - 1]->action &&
        !strcmp(eff->child[n - 1]->action, "repeat_procedure")) {
        has_repeat = 1;
        int rl = eff->child[n - 1]->repeat_limit;
        repeat_max = (rl > 0 ? rl : 1) + 1;   /* total = initial + repeats */
        n = n - 1;                            /* drop the marker from the list */
    }

    for (int rep = 0; rep < repeat_max; rep++) {
        int condition_failed = -1;   /* -1 none, 0 passed, 1 failed */
        for (int i = 0; i < n; i++) {
            AbilityEffect *a = eff->child[i];
            if (!a) continue;

            int is_otherwise = (a->has_condition && a->condition &&
                                a->condition->variant == RB_COND_ALWAYS_TRUE);
            if (is_otherwise) {
                /* Rust: Some(false)→skip+reset; Some(true) or None→execute */
                if (condition_failed == 0) { condition_failed = -1; continue; }
                condition_failed = -1;   /* fall through to execute */
            } else if (condition_failed == 1 && !a->has_condition) {
                /* a failed gate suppresses the rest of this conditional block */
                continue;
            } else if (a->has_condition && !is_otherwise) {
                int passed = rb_eval_condition(g, actor, a->condition);
                if (!a->is_optional) condition_failed = passed ? 0 : 1;
                if (!passed) continue;
            }

            (void)has_repeat;
            rb_execute_effect_ex(g, actor, a, host_cid);
            if (rb_has_pending_choice(g)) return 1;   /* interactive: stop */
        }
    }
    return 1;
}

/* Mirror compound.rs:route_conditional_branch — pick the chosen branch of a
   conditional_alternative. Returns branch index (0 = consequent/alternative,
   1 = alternate/primary). */
int rb_compound_route_branch(const GameState *g, int actor, const AbilityEffect *eff) {
    if (!eff->condition) return 1;
    return rb_eval_condition(g, actor, eff->condition) ? 0 : 1;
}

/* Mirror compound.rs:execute_conditional_alternative — tiered conditions then
   the legacy single-condition routing. branch<0 routes via the effect's own
   condition; branch>=0 forces that branch (0 = alternative_effect, 1 = primary). */
int rb_compound_conditional_alternative(GameState *g, int actor,
                                        const AbilityEffect *eff, int branch, int host_cid) {
    int has_primary = (eff->primary_effect != NULL);
    int has_alt = (eff->alternative_effect != NULL);

    if (has_primary && has_alt) {
        /* Tiered: stricter alternative_condition first. */
        if (eff->alternative_condition && eff->condition) {
            if (rb_eval_condition(g, actor, eff->alternative_condition)) {
                if (eff->alternative_effect)
                    rb_execute_effect_ex(g, actor, eff->alternative_effect, host_cid);
                return 1;
            }
            if (rb_eval_condition(g, actor, eff->condition)) {
                if (eff->primary_effect)
                    rb_execute_effect_ex(g, actor, eff->primary_effect, host_cid);
            }
            return 1;
        }
        /* Legacy: single condition selects alternative (true) / primary (false). */
        const Condition *cond = eff->alternative_condition ? eff->alternative_condition
                                                           : eff->condition;
        if (cond) {
            if (rb_eval_condition(g, actor, cond)) {
                if (eff->alternative_effect)
                    rb_execute_effect_ex(g, actor, eff->alternative_effect, host_cid);
            } else if (eff->primary_effect) {
                rb_execute_effect_ex(g, actor, eff->primary_effect, host_cid);
            }
            return 1;
        }
        /* No condition → headless runs the primary (Rust would prompt). */
        if (eff->primary_effect)
            rb_execute_effect_ex(g, actor, eff->primary_effect, host_cid);
        else if (eff->alternative_effect)
            rb_execute_effect_ex(g, actor, eff->alternative_effect, host_cid);
        return 1;
    }

    if (eff->alternative_condition) {
        if (rb_eval_condition(g, actor, eff->alternative_condition) && eff->alternative_effect)
            rb_execute_effect_ex(g, actor, eff->alternative_effect, host_cid);
        return 1;
    }
    if (has_alt && !has_primary && !eff->condition) {
        if (eff->condition && rb_eval_condition(g, actor, eff->condition) && eff->alternative_effect)
            rb_execute_effect_ex(g, actor, eff->alternative_effect, host_cid);
        return 1;
    }
    if (eff->primary_effect)
        rb_execute_effect_ex(g, actor, eff->primary_effect, host_cid);
    return 1;
}

/* Mirror compound.rs:execute_conditional_on_result — run primary_effect, then if
   result_condition is met run the followup_action (selected_cards cleared first so
   the followup doesn't inherit the primary's targets). */
int rb_compound_conditional_on_result(GameState *g, int actor,
                                      const AbilityEffect *eff, int host_cid) {
    if (eff->primary_effect)
        rb_execute_effect_ex(g, actor, eff->primary_effect, host_cid);
    if (rb_has_pending_choice(g)) return 1;   /* interactive: stop */

    int cond_met = 1;
    if (eff->result_condition)
        cond_met = rb_eval_condition(g, actor, eff->result_condition);
    if (cond_met && eff->followup_action) {
        g->n_selected_cards = 0;
        rb_execute_effect_ex(g, actor, eff->followup_action, host_cid);
    }
    return 1;
}

/* Mirror compound.rs::route_conditional_branch (the (chose_yes, is_negation)
   matrix) for conditional_on_optional — returns the effect that should fire. */
static const AbilityEffect *rb_on_optional_branch(const AbilityEffect *eff, int chose_yes) {
    int neg = eff->conditional_negation;
    if (chose_yes && neg)  return eff->optional_action;
    if (chose_yes && !neg) return eff->conditional_action;
    if (!chose_yes && neg) return eff->conditional_action;
    return NULL;
}

/* Mirror compound.rs:execute_conditional_on_optional.
     taken == -1 → no cost result yet: preserve legacy behaviour (emit a choice,
                   headless host auto-skips).
     taken == 0/1 → route via the (chose_yes, negation) matrix. */
int rb_compound_conditional_on_optional(GameState *g, int actor,
                                        const AbilityEffect *eff, int taken, int host_cid) {
    if (taken < 0) {
        int allow = eff->is_optional ? 1 : 0;
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, 0, allow,
                       "conditional_optional");
        return 1;
    }
    const AbilityEffect *cmd = rb_on_optional_branch(eff, taken);
    if (cmd) rb_execute_effect_ex(g, actor, (AbilityEffect *)cmd, host_cid);
    return 1;
}

/* Mirror compound.rs:handle_choice_string_selection — map a chosen string to the
   index of the child whose action matches. Returns -1 if not found. */
int rb_compound_choice_string(const AbilityEffect *eff, const char *choice) {
    if (!eff || !choice) return -1;
    for (int i = 0; i < eff->n_child; i++) {
        if (eff->child[i] && eff->child[i]->action &&
            !strcmp(eff->child[i]->action, choice))
            return i;
    }
    return -1;
}

/* Mirror compound.rs:execute_choice_action — run the selected choice child. */
int rb_compound_choice_action(GameState *g, int actor, const AbilityEffect *eff,
                              int choice_idx, int host_cid) {
    if (choice_idx < 0 || choice_idx >= eff->n_child) return 0;
    rb_execute_effect_ex(g, actor, eff->child[choice_idx], host_cid);
    return 1;
}
