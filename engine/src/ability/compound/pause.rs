//! Pause and repeat logic for sequential execution

use crate::ability::enums::ActionType;
use crate::ability::resolver::AbilityResolver;
use crate::card::AbilityEffect;
use crate::game_state::GameState;
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

/// Save remaining actions back to the queue.
fn save_remaining_actions(
    gs: &mut GameState,
    remaining: Vec<Box<AbilityEffect>>,
) {
    if !remaining.is_empty() {
        let mut existing = gs.ability_queue.take_pending_actions();
        existing.extend(remaining.into_iter().map(|b| *b));
        gs.ability_queue.set_pending_actions(existing);
    }
}

/// Pause the sequential on a pending choice: arm the deferred
/// parent-conditional gate, strip already-settled otherwise conditions
/// from the saved remainder, stash repeat state, and park the rest.
/// The caller must `return Ok(())` afterwards.
pub(crate) fn pause_for_sequential_choice(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    repeat_actions: &[Box<AbilityEffect>],
    actions: &[Box<AbilityEffect>],
    i: usize,
    action: &AbilityEffect,
    conditional: bool,
    condition_failed: Option<bool>,
    repeats_remaining: u8,
    has_repeat: bool,
    repeat_idx: u8,
) {
    let current_was_optional = action.optional.unwrap_or(false);
    let is_opponent_action = action.action
        == ActionType::OpponentAction
        || action.action_by() == Some("opponent");
    if conditional
        && action.condition.is_none()
        && condition_failed.is_none()
        && !is_opponent_action
    {
        resolver.deferred_conditional_gate = true;
    }
    let completes_in_handler =
        resolver.pending_choice.as_ref().is_some_and(|c| {
            matches!(
                c,
                crate::ability::types::Choice::SelectCard { .. }
            )
        }) || matches!(
            resolver.pending_choice.as_ref(),
            Some(crate::ability::types::Choice::SelectTarget {
                target,
                ..
            }) if target == "position|destination"
        );
    let mut remaining = if current_was_optional
        && i + 1 < repeat_actions.len()
        && !is_opponent_action
        && !completes_in_handler
    {
        let mut acts: Vec<Box<AbilityEffect>> =
            repeat_actions[i..].to_vec();
        if !acts.is_empty() {
            acts[0].set_optional(None);
        }
        acts
    } else {
        repeat_actions[i + 1..].to_vec()
    };
    if condition_failed == Some(false) {
        remaining.retain(|a| {
            !a.condition.as_ref().is_some_and(|c| {
                matches!(c.as_ref(), crate::card::Condition::AlwaysTrue { .. })
            })
        });
    }
    if condition_failed == Some(false) {
        for a in &mut remaining {
            if a.condition.as_ref().is_some_and(|c| {
                matches!(
                    c.as_ref(),
                    crate::card::Condition::AllRevealedMatchHeartColor { .. }
                )
            }) {
                a.condition = None;
            }
        }
    }
    if repeats_remaining > 0 && has_repeat {
        if let Some(repeat_action) = actions.last() {
            if repeat_action.action == ActionType::RepeatProcedure
                && repeat_action.optional.unwrap_or(false)
            {
                for _ in 0..repeats_remaining {
                    resolver.pending_repeat_actions
                        .extend(repeat_actions.iter().cloned());
                }
            }
        }
    }
    log::debug!("[SEQUENCE] source={:?} repeat={} step={} action={} paused: deferred_gate={} remaining={:?} repeat_actions={}", resolver.activating_card_id, repeat_idx + 1, i + 1, action.action, resolver.deferred_conditional_gate, remaining.iter().map(|a| a.action).collect::<Vec<_>>(), resolver.pending_repeat_actions.len());
    save_remaining_actions(gs, remaining);
}