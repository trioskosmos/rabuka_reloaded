//! Post-step tracking and repeat prompting for sequential execution

use crate::ability::enums::ActionType;
use crate::ability::resolver::AbilityResolver;
use crate::ability::types::StepOutput;
use crate::card::AbilityEffect;
use crate::game_state::GameState;

/// Post-step tracking for non-choice steps. Returns true when the caller
/// must `return Ok(())` (cancelled remainder / optional no-target stop).
pub(crate) fn track_sequential_post_step(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    action: &AbilityEffect,
    conditional: bool,
    condition_failed: &mut Option<bool>,
    moved_before: usize,
    selected_before: usize,
    repeat_idx: u8,
    i: usize,
) -> bool {
    if resolver.cancel_remaining_commands {
        resolver.cancel_remaining_commands = false;
        log::debug!("[SEQUENCE] source={:?} repeat={} step={} action={} stopped: remaining actions cancelled", resolver.activating_card_id, repeat_idx + 1, i + 1, action.action);
        return true;
    }
    if action.optional.unwrap_or(false) {
        if action.action == ActionType::ChangeState {
            log::debug!("[SEQUENCE] source={:?} repeat={} step={} action={} stopped: optional state change had no choice", resolver.activating_card_id, repeat_idx + 1, i + 1, action.action);
            return true;
        }
        let was_moved = resolver.moved_cards.len() - moved_before;
        if was_moved == 0 {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.optional_cost_result = Some(false);
            }
        }
        if conditional
            && action.condition.is_none()
            && condition_failed.is_none()
        {
            let was_moved = resolver.moved_cards.len() - moved_before;
            let was_selected = resolver.selected_cards.len() - selected_before;
            *condition_failed = Some(was_moved == 0 && was_selected == 0);
        }
        return false;
    }
    if condition_failed.is_none()
        && !resolver.pending_choice.is_some()
        && conditional
        && action.condition.is_none()
    {
        let was_moved = resolver.moved_cards.len() - moved_before;
        let was_selected = resolver.selected_cards.len() - selected_before;
        *condition_failed = Some(was_moved == 0 && was_selected == 0);
    }
    false
}

/// Record a finished step's output under its id (if any) so downstream
/// steps in the same sequential can reference it via `ref: "<id>"`.
pub(crate) fn record_step_output(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    action: &AbilityEffect,
    repeat_idx: u8,
    i: usize,
) {
    if let Some(ref step_id) = action.id_any() {
        let mut out = StepOutput::default();
        if !resolver.selected_cards.is_empty() {
            out.cards.extend_from_slice(&resolver.selected_cards);
        } else if !resolver.moved_cards.is_empty() {
            out.cards.extend_from_slice(&resolver.moved_cards);
        } else if !gs.looked_at_cards.is_empty() {
            out.cards.extend_from_slice(&gs.looked_at_cards);
        } else if !gs.revealed_cards.is_empty() {
            out.cards.extend_from_slice(&gs.revealed_cards);
        }
        if resolver.step_state.last_draw_count > 0 {
            out.value = Some(resolver.step_state.last_draw_count as i32);
        }
        resolver.step_state
            .step_results
            .entry(step_id.to_string())
            .or_insert_with(StepOutput::default)
            .merge(&out);
        log::debug!(
            "[SEQUENCE] source={:?} repeat={} step={} action={} output_id={} cards={:?} value={:?}",
            resolver.activating_card_id,
            repeat_idx + 1,
            i + 1,
            action.action,
            step_id,
            out.cards,
            out.value
        );
    }
}

/// After an iteration, an optional `repeat_procedure` tail asks the
/// player whether to run more iterations. Returns true when the prompt
/// was issued (caller must `return Ok(())`).
pub(crate) fn maybe_prompt_repeat_continue(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    actions: &[Box<AbilityEffect>],
    repeat_actions: &[Box<AbilityEffect>],
    repeats_remaining: u8,
) -> bool {
    if repeats_remaining == 0 {
        return false;
    }
    if let Some(ref repeat_action) = actions.last() {
        if repeat_action.action == ActionType::RepeatProcedure
            && repeat_action.optional.unwrap_or(false)
        {
            for _ in 0..repeats_remaining {
                resolver.pending_repeat_actions
                    .extend(repeat_actions.iter().cloned());
            }
            resolver.pending_choice =
                Some(crate::ability::types::repeat_prompt_choice());
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.choice_card_no =
                    Some(crate::ability::types::ChoiceRoute::Raw(
                        "pay_optional_cost".to_string(),
                    ));
            }
            return true;
        }
    }
    false
}