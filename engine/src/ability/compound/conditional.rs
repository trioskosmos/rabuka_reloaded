use crate::ability::resolver::AbilityResolver;
use crate::card::AbilityEffect;
use crate::game_state::GameState;
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

/// Routes a settled conditional_on_optional answer to the branch that fires.
/// ONE definition of the (chose_yes × negation) matrix, used by
/// `execute_conditional_on_optional` and
/// `handle_conditional_optional`:
///
///   yes + negation    → optional_action     ("unless you pay → do A" honored)
///   yes + no negation → conditional_action  (the follow-up)
///   no  + negation    → conditional_action  (the penalty)
///   no  + no negation → nothing fires
pub(crate) fn route_conditional_branch(
    effect: &AbilityEffect,
    chose_yes: bool,
    is_negation: bool,
) -> Option<Box<AbilityEffect>> {
    match (chose_yes, is_negation) {
        (true, true) => effect.compound.optional_action.clone(),
        (true, false) => effect.compound.conditional_action.clone(),
        (false, true) => effect.compound.conditional_action.clone(),
        (false, false) => None,
    }
}

/// Execute conditional alternative effect (if-then-else pattern).
pub(crate) fn execute_conditional_alternative(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    effect: &AbilityEffect,
) -> Result<(), String> {
    let has_primary = effect.compound.primary_effect.is_some();
    let has_alternative = effect.alternative_effect_any().is_some();

    if has_primary && has_alternative {
        let ctx = crate::ability::condition::ConditionContext::with_moved_cards(gs, &resolver.moved_cards);

        if effect.compound.alternative_condition.is_some() && effect.condition.is_some() {
            if let Some(ref alt_cond) = effect.compound.alternative_condition {
                if ctx.evaluate_condition(alt_cond) {
                    if let Some(ref alt_effect) = effect.alternative_effect_any() {
                        return resolver.execute_effect(gs, alt_effect);
                    }
                }
            }
            if let Some(ref cond) = effect.condition {
                if ctx.evaluate_condition(cond) {
                    if let Some(ref primary_effect) = effect.compound.primary_effect {
                        return resolver.execute_effect(gs, primary_effect);
                    }
                }
            }
            return Ok(());
        }

        let ctx = crate::ability::condition::ConditionContext::with_moved_cards(gs, &resolver.moved_cards);
        if let Some(ref cond) = effect.condition {
            let cond_passed = ctx.evaluate_condition(cond);
            let is_negation = cond.get_negation().unwrap_or(false);
            let branch = if is_negation {
                cond_passed
            } else {
                !cond_passed
            };
            if branch {
                if let Some(ref alt_effect) = effect.alternative_effect_any() {
                    return resolver.execute_effect(gs, alt_effect);
                }
            } else {
                if let Some(ref primary_effect) = effect.compound.primary_effect {
                    return resolver.execute_effect(gs, primary_effect);
                }
            }
        }
    } else if has_alternative {
        if let Some(ref alt_effect) = effect.alternative_effect_any() {
            return resolver.execute_effect(gs, alt_effect);
        }
    } else if has_primary {
        if let Some(ref primary_effect) = effect.compound.primary_effect {
            return resolver.execute_effect(gs, primary_effect);
        }
    }
    Ok(())
}