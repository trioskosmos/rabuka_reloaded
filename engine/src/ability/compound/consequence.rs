//! Consequence gating and step output recording for sequential execution

use crate::ability::enums::{ActionType, Zone};
use crate::ability::resolver::AbilityResolver;
use crate::card::AbilityEffect;

/// "…したとき" (when you do so) gate: a consequence step that immediately
/// follows a move is only applied when that move actually moved a card.
/// Returns true when the step must be skipped. Always clears the flag so
/// it never gates an unrelated later step.
pub(crate) fn gate_sequential_consequence(
    resolver: &mut AbilityResolver,
    action: &AbilityEffect,
    repeat_idx: u8,
    i: usize,
) -> bool {
    let is_gated_consequence = action.action == ActionType::ModifyScore
        || (action.action == ActionType::MoveCards
            && action.destination == Some(Zone::Hand)
            && action.is_self_target()
            && action.source_any().is_some_and(|s| {
                s == "discard" || s == "waitroom"
            }));
    if is_gated_consequence && resolver.carried.last_move_moved_any == Some(false) {
        log::debug!("[SEQUENCE] source={:?} repeat={} step={} action={} skipped: preceding move moved no cards", resolver.session.activating_card_id, repeat_idx + 1, i + 1, action.action);
        resolver.carried.last_move_moved_any = None;
        return true;
    }
    resolver.carried.last_move_moved_any = None;
    false
}