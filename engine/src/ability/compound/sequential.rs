//! Sequential routing and step preparation helpers

use crate::ability::condition::ConditionContext;
use crate::ability::enums::ActionType;
use crate::ability::resolver::AbilityResolver;
use crate::ability::types::{ExecutionContext, ZoneSnapshot};
use crate::card::{AbilityEffect, Condition};
use crate::game_state::GameState;
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

/// Split trailing `repeat_procedure` off the step list.
/// Returns `(steps_to_run, total_iterations, has_repeat)`.
pub(crate) fn split_repeat_actions(
    actions: &[Box<AbilityEffect>],
) -> (&[Box<AbilityEffect>], u8, bool) {
    let has_repeat = actions
        .last()
        .is_some_and(|a| a.action == ActionType::RepeatProcedure);
    let repeat_max = if has_repeat {
        actions
            .last()
            .and_then(|a| a.repeat_limit_any())
            .unwrap_or(1)
            + 1
    } else {
        1
    };
    let repeat_actions = if has_repeat {
        &actions[..actions.len() - 1]
    } else {
        actions
    };
    (repeat_actions, repeat_max, has_repeat)
}

/// Conditional routing for one sequential step.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepRoute {
    Execute,
    Skip,
}

/// Conditional routing for one sequential step:
///  • is_otherwise  → skip if condition met, execute if failed
///  • has condition  → evaluate directly (for otherwise routing)
///  • no condition + parent-conditional → skip if preceding failed
pub(crate) fn route_sequential_step(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    repeat_actions: &[Box<AbilityEffect>],
    i: usize,
    repeat_idx: u8,
    condition_failed: &mut Option<bool>,
) -> StepRoute {
    let action = &repeat_actions[i];
    let is_otherwise = action
        .condition
        .as_ref()
        .is_some_and(|c| matches!(c.as_ref(), Condition::AlwaysTrue { .. }));
    if is_otherwise {
        match *condition_failed {
            Some(false) => {
                log::debug!("[SEQUENCE] source={:?} repeat={} step={} action={} skipped: otherwise branch after passed condition", resolver.session.activating_card_id, repeat_idx + 1, i + 1, action.action);
                *condition_failed = None;
                return StepRoute::Skip;
            }
            Some(true) => {
                *condition_failed = None;
                return StepRoute::Execute;
            }
            None => return StepRoute::Execute,
        }
    }
    if *condition_failed == Some(true) && action.condition.is_none() {
        log::debug!("[SEQUENCE] source={:?} repeat={} step={} action={} skipped: preceding condition failed", resolver.session.activating_card_id, repeat_idx + 1, i + 1, action.action);
        return StepRoute::Skip;
    }
    if let Some(cond) = action.condition.as_ref() {
        let same_as_prev = i > 0
            && repeat_actions[i - 1].condition.as_ref()
                == action.condition.as_ref();
        if same_as_prev {
            log::debug!("[SEQUENCE] source={:?} repeat={} step={} action={} decision={} verdict=previous_step condition_failed={:?}", resolver.session.activating_card_id, repeat_idx + 1, i + 1, action.action, if *condition_failed == Some(true) { "skip" } else { "continue" }, *condition_failed);
            if *condition_failed == Some(true) {
                return StepRoute::Skip;
            }
            return StepRoute::Execute;
        }
        let moved_cards = resolver.selection.moved_cards.clone();
        let passed = ConditionContext::with_moved_cards(gs, &moved_cards).evaluate_condition(cond);
        if !action.optional.unwrap_or(false) {
            *condition_failed = Some(!passed);
        }
        log::debug!("[SEQUENCE] source={:?} repeat={} step={} action={} decision={} passed={}", resolver.session.activating_card_id, repeat_idx + 1, i + 1, action.action, if passed { "continue" } else { "skip: condition failed" }, passed);
        if !passed {
            return StepRoute::Skip;
        }
    }
    StepRoute::Execute
}

/// A sequential step prepared for dispatch.
///
/// The point of this type is to avoid the unconditional `action.clone()` that
/// used to happen for EVERY step of EVERY repeat. `AbilityEffect::clone` is a
/// deep clone: an `Option<Arc<EffectKind>>` wrapping a ~1.2 KB `EffectFilter`
/// with 26 `Box`es, i.e. ~4 allocations plus ~1.7 KB of memcpy, and it was paid
/// even though `prepare_sequential_action`'s entire job is to strip one field
/// and inherit at most five others — and on the overwhelming majority of steps
/// none of the inheritance branches fire.
///
/// `Borrowed` is therefore the common case: the step is used as-is. It is only
/// chosen when the step has no condition to strip, because the strip is a
/// mutation and `execute_effect` takes `&AbilityEffect`.
pub(crate) enum PreparedStep<'a> {
    /// Nothing to strip and nothing to inherit — use the step directly.
    Borrowed(&'a AbilityEffect),
    /// A condition had to be stripped, or a parent field was inherited.
    Owned(AbilityEffect),
}

/// Would `prepare_sequential_action` change anything about this step?
///
/// Mirrors the guards inside the function exactly. A step that needs no
/// condition strip and no inheritance is returned borrowed.
fn sequential_step_needs_patch(
    effect: &AbilityEffect,
    action: &AbilityEffect,
    i: usize,
    first: &AbilityEffect,
) -> bool {
    // The condition strip is a mutation, so a step that carries one always
    // needs an owned copy.
    if action.condition.is_some() {
        return true;
    }
    let supports_per_unit = matches!(
        action.action,
        ActionType::DrawCard
            | ActionType::GainResource
            | ActionType::ModifyScore
            | ActionType::ModifyRequiredHearts
            | ActionType::GainAbility
            | ActionType::SetBladeCount
            | ActionType::LookAt
    );
    if supports_per_unit
        && ((action.per_unit_any().is_none() && effect.per_unit_any().is_some())
            || (action.per_unit_count_any().is_none() && effect.per_unit_count_any().is_some())
            || (action.per_unit_type_any().is_none() && effect.per_unit_type_any().is_some())
            || (action.action == ActionType::ModifyRequiredHearts
                && action.distinct_any().is_none()
                && effect.distinct_any().is_some()))
    {
        return true;
    }
    if action.self_target_any().is_none() {
        let inheritable = if i > 0 {
            !(first.card_type_any() != Some(&crate::card::CardType::Member)
                && action.card_type_any() == Some(&crate::card::CardType::Member))
        } else {
            true
        };
        if inheritable {
            let supports_self = matches!(
                action.action,
                ActionType::ModifyScore
                    | ActionType::ModifyRequiredHearts
                    | ActionType::GainResource
                    | ActionType::ChangeState
            );
            if effect.self_target_any().is_some() && supports_self {
                return true;
            }
            if i > 0 && supports_self && first.self_target_any().is_some() {
                return true;
            }
        }
    }
    if action.card_names_any().is_none_or(|v| v.is_empty())
        && !effect.card_names_any().is_none_or(|v| v.is_empty())
    {
        return true;
    }
    false
}

/// Clone a step for execution: strip the already-gated condition and
/// inherit per_unit / self_target / card_names from the parent effect.
/// Only inherit per_unit properties for actions that support them —
/// discard/move_cards actions must not inherit per_unit multipliers.
///
/// Returns [`PreparedStep::Borrowed`] when the step needs no change at all.
pub(crate) fn prepare_sequential_action<'a>(
    effect: &'a AbilityEffect,
    action: &'a AbilityEffect,
    i: usize,
    first: &'a AbilityEffect,
) -> PreparedStep<'a> {
    if !sequential_step_needs_patch(effect, action, i, first) {
        return PreparedStep::Borrowed(action);
    }
    let mut action_to_execute = action.clone();
    action_to_execute.condition = None;
    let supports_per_unit = matches!(
        action.action,
        ActionType::DrawCard
            | ActionType::GainResource
            | ActionType::ModifyScore
            | ActionType::ModifyRequiredHearts
            | ActionType::GainAbility
            | ActionType::SetBladeCount
            | ActionType::LookAt
    );
    if supports_per_unit {
        if action_to_execute.per_unit_any().is_none()
            && effect.per_unit_any().is_some()
        {
            action_to_execute.set_per_unit(effect.per_unit_any());
        }
        if action_to_execute.per_unit_count_any().is_none()
            && effect.per_unit_count_any().is_some()
        {
            action_to_execute.set_per_unit_count(effect.per_unit_count_any());
        }
        if action_to_execute.per_unit_type_any().is_none()
            && effect.per_unit_type_any().is_some()
        {
            action_to_execute
                .set_per_unit_type(effect.per_unit_type_any().map(|s| s.into()));
        }
        if action.action == ActionType::ModifyRequiredHearts
            && action_to_execute.distinct_any().is_none()
        {
            if let Some(d) = effect.distinct_any() {
                if let Some(f) = action_to_execute
                    .kind
                    .as_mut()
                    .map(crate::Arc::make_mut)
                    .and_then(|k| k.filter_mut())
                {
                    f.distinct = Some(Box::new(d));
                }
            }
        }
    }
    if action_to_execute.self_target_any().is_none() {
        let inheritable = if i > 0 {
            let first_ct_binding = first.card_type_any();
            let first_ct = first_ct_binding;
            let cur_ct_binding = action.card_type_any();
            let cur_ct = cur_ct_binding;
            !(first_ct != Some(&crate::card::CardType::Member)
                && cur_ct == Some(&crate::card::CardType::Member))
        } else {
            true
        };
        if inheritable {
            let supports_self = matches!(
                action.action,
                ActionType::ModifyScore
                    | ActionType::ModifyRequiredHearts
                    | ActionType::GainResource
                    | ActionType::ChangeState
            );
            if effect.self_target_any().is_some() && supports_self {
                action_to_execute.set_self_target(effect.self_target_any());
            } else if i > 0 && supports_self {
                action_to_execute
                    .set_self_target(first.self_target_any());
            }
        }
    }
    if action_to_execute
        .card_names_any()
        .is_none_or(|v| v.is_empty())
        && !effect.card_names_any().is_none_or(|v| v.is_empty())
    {
        if let Some(names) = effect.card_names_any() {
            action_to_execute.set_card_names(names.clone());
        }
    }
    PreparedStep::Owned(action_to_execute)
}

/// Execute sequential effect (Rule 9.2.1.1).
///
/// `steps` is the normalized step list, passed separately from `effect` so the
/// caller does not have to build a synthetic parent effect. Previously the
/// routing path deep-cloned the whole effect tree (~4 allocations and ~1.7 KB
/// per element) purely to set `compound.actions` on the copy — and nothing in
/// here or in `setup_sequential` / `prepare_sequential_action` ever read that
/// field back off the parent: they read `conditional`, `is_further`, `condition`
/// and the per-unit/self-target/card-names fields, none of which the clone
/// changed. `effect.action` is used only in a debug log, so `action_for_log`
/// carries it explicitly to keep that output byte-identical.
pub(crate) fn execute_sequential_effect(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    effect: &AbilityEffect,
    steps: &[Box<AbilityEffect>],
    action_for_log: ActionType,
) -> Result<(), String> {
    let conditional = effect.conditional.unwrap_or(false);
    let seq_node = match crate::ability::compound::setup::setup_sequential(resolver, gs, effect) {
        Ok(node) => node,
        Err(()) => return Ok(()),
    };

    if !steps.is_empty() {
        let (repeat_actions, repeat_max, has_repeat) = split_repeat_actions(steps);

        if has_repeat {
            resolver.awaiting.repeat_actions.clear();
        }
        log::debug!(
            "[SEQUENCE] source={:?} action={} steps={} iterations={} conditional={} further={} actions={:?}",
            resolver.session.activating_card_id,
            action_for_log,
            repeat_actions.len(),
            repeat_max,
            conditional,
            effect.is_further.unwrap_or(false),
            repeat_actions.iter().map(|a| a.action).collect::<Vec<_>>()
        );
        let mut condition_failed: Option<bool>;
        for repeat_idx in 0..repeat_max {
            let repeats_remaining = repeat_max.saturating_sub(repeat_idx + 1);
            condition_failed = None;
            'action_loop: for (i, action) in repeat_actions.iter().enumerate() {
                if route_sequential_step(resolver, gs, repeat_actions, i, repeat_idx, &mut condition_failed)
                    == StepRoute::Skip
                {
                    continue 'action_loop;
                }

                let action_to_execute = prepare_sequential_action(
                    effect,
                    action,
                    i,
                    &repeat_actions[0],
                );
                let action_to_execute: &AbilityEffect = match &action_to_execute {
                    PreparedStep::Borrowed(a) => a,
                    PreparedStep::Owned(a) => a,
                };

                if action.action == ActionType::OpponentAction
                    || action.action_by() == Some("opponent")
                {
                    resolver.in_flight.spawn_context.target = Some("opponent".to_string());
                }

                let moved_before = resolver.selection.moved_cards.len();
                let selected_before = resolver.selection.cards.len();
                if crate::ability::compound::consequence::gate_sequential_consequence(resolver, action, repeat_idx, i) {
                    continue 'action_loop;
                }
                match resolver.execute_effect(gs, action_to_execute) {
                    Ok(_) => {
                        log::debug!(
                            "[SEQUENCE] source={:?} repeat={} step={} action={} result=ok pending={}",
                            resolver.session.activating_card_id,
                            repeat_idx + 1,
                            i + 1,
                            action.action,
                            resolver.awaiting.choice.is_some()
                        );
                        crate::ability::compound::post_step::record_step_output(resolver, gs, action, repeat_idx, i);
                        if resolver.awaiting.choice.is_some() {
                            crate::ability::compound::pause::pause_for_sequential_choice(
                                resolver,
                                gs,
                                repeat_actions,
                                steps,
                                i,
                                action,
                                conditional,
                                condition_failed,
                                repeats_remaining,
                                has_repeat,
                                repeat_idx,
                            );
                            return Ok(());
                        } else if crate::ability::compound::post_step::track_sequential_post_step(
                            resolver,
                            gs,
                            action,
                            conditional,
                            &mut condition_failed,
                            moved_before,
                            selected_before,
                            repeat_idx,
                            i,
                        ) {
                            return Ok(());
                        }
                    }
                    Err(e) => {
                        log::debug!("[SEQUENCE] source={:?} repeat={} step={} action={} result=error pending={} error={}", resolver.session.activating_card_id, repeat_idx + 1, i + 1, action.action, resolver.awaiting.choice.is_some(), e);
                        return Err(e);
                    }
                }
            }
            if crate::ability::compound::post_step::maybe_prompt_repeat_continue(resolver, gs, steps, repeat_actions, repeats_remaining) {
                return Ok(());
            }
        }
    }
    resolver.in_flight.execution_context = ExecutionContext::None;

    if let Some(mut node) = seq_node {
        node.after = Some(ZoneSnapshot::from_game_state(gs));
        resolver.in_flight.pipeline.trace.children.push(node);
    }

    Ok(())
}