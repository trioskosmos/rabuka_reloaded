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
                log::debug!("[SEQUENCE] source={:?} repeat={} step={} action={} skipped: otherwise branch after passed condition", resolver.activating_card_id, repeat_idx + 1, i + 1, action.action);
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
        log::debug!("[SEQUENCE] source={:?} repeat={} step={} action={} skipped: preceding condition failed", resolver.activating_card_id, repeat_idx + 1, i + 1, action.action);
        return StepRoute::Skip;
    }
    if action.condition.is_some() {
        let same_as_prev = i > 0
            && repeat_actions[i - 1].condition.as_ref()
                == action.condition.as_ref();
        if same_as_prev {
            log::debug!("[SEQUENCE] source={:?} repeat={} step={} action={} decision={} verdict=previous_step condition_failed={:?}", resolver.activating_card_id, repeat_idx + 1, i + 1, action.action, if *condition_failed == Some(true) { "skip" } else { "continue" }, *condition_failed);
            if *condition_failed == Some(true) {
                return StepRoute::Skip;
            }
            return StepRoute::Execute;
        }
        let cond = action.condition.as_ref().unwrap();
        let moved_cards = resolver.moved_cards.clone();
        let passed = ConditionContext::with_moved_cards(gs, &moved_cards).evaluate_condition(cond);
        if !action.optional.unwrap_or(false) {
            *condition_failed = Some(!passed);
        }
        log::debug!("[SEQUENCE] source={:?} repeat={} step={} action={} decision={} passed={}", resolver.activating_card_id, repeat_idx + 1, i + 1, action.action, if passed { "continue" } else { "skip: condition failed" }, passed);
        if !passed {
            return StepRoute::Skip;
        }
    }
    StepRoute::Execute
}

/// Clone a step for execution: strip the already-gated condition and
/// inherit per_unit / self_target / card_names from the parent effect.
/// Only inherit per_unit properties for actions that support them —
/// discard/move_cards actions must not inherit per_unit multipliers.
pub(crate) fn prepare_sequential_action(
    effect: &AbilityEffect,
    action: &AbilityEffect,
    i: usize,
    first: &AbilityEffect,
) -> AbilityEffect {
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
                    .as_deref_mut()
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
        .map_or(true, |v| v.is_empty())
        && !effect.card_names_any().map_or(true, |v| v.is_empty())
    {
        if let Some(names) = effect.card_names_any() {
            action_to_execute.set_card_names(names.clone());
        }
    }
    action_to_execute
}

/// Execute sequential effect (Rule 9.2.1.1).
pub(crate) fn execute_sequential_effect(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    effect: &AbilityEffect,
) -> Result<(), String> {
    let conditional = effect.conditional.unwrap_or(false);
    let seq_node = match crate::ability::compound::setup::setup_sequential(resolver, gs, effect) {
        Ok(node) => node,
        Err(()) => return Ok(()),
    };

    if let Some(ref actions) = effect.compound.actions {
        let (repeat_actions, repeat_max, has_repeat) = split_repeat_actions(actions);

        if has_repeat {
            resolver.pending_repeat_actions.clear();
        }
        log::debug!(
            "[SEQUENCE] source={:?} action={} steps={} iterations={} conditional={} further={} actions={:?}",
            resolver.activating_card_id,
            effect.action,
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

                if action.action == ActionType::OpponentAction
                    || action.action_by().as_deref() == Some("opponent")
                {
                    resolver.spawn_context.target = Some("opponent".to_string());
                }

                let moved_before = resolver.moved_cards.len();
                let selected_before = resolver.selected_cards.len();
                if crate::ability::compound::consequence::gate_sequential_consequence(resolver, action, repeat_idx, i) {
                    continue 'action_loop;
                }
                log::debug!("[SEQUENCE] source={:?} repeat={} step={}/{} action={} execute: condition_failed={:?} pending_before={}", resolver.activating_card_id, repeat_idx + 1, i + 1, repeat_actions.len(), action.action, condition_failed, resolver.pending_choice.is_some());
                match resolver.execute_effect(gs, &action_to_execute) {
                    Ok(_) => {
                        log::debug!(
                            "[SEQUENCE] source={:?} repeat={} step={} action={} result=ok pending={}",
                            resolver.activating_card_id,
                            repeat_idx + 1,
                            i + 1,
                            action.action,
                            resolver.pending_choice.is_some()
                        );
                        crate::ability::compound::post_step::record_step_output(resolver, gs, action, repeat_idx, i);
                        if resolver.pending_choice.is_some() {
                            crate::ability::compound::pause::pause_for_sequential_choice(
                                resolver,
                                gs,
                                repeat_actions,
                                actions,
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
                        log::debug!("[SEQUENCE] source={:?} repeat={} step={} action={} result=error pending={} error={}", resolver.activating_card_id, repeat_idx + 1, i + 1, action.action, resolver.pending_choice.is_some(), e);
                        return Err(e);
                    }
                }
            }
            if crate::ability::compound::post_step::maybe_prompt_repeat_continue(resolver, gs, actions, repeat_actions, repeats_remaining) {
                return Ok(());
            }
        }
    }
    resolver.execution_context = ExecutionContext::None;

    if let Some(mut node) = seq_node {
        node.after = Some(ZoneSnapshot::from_game_state(gs));
        resolver.pipeline.trace.children.push(node);
    }

    Ok(())
}