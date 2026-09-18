//! Sequential setup and core routing for compound effects

use crate::ability::condition::ConditionContext;
use crate::ability::resolver::AbilityResolver;
use crate::ability::types::{AbilityTraceNode, ZoneSnapshot};
use crate::card::AbilityEffect;
use crate::game_state::GameState;

/// Setup for a sequential run: trace label/node, step-state reset, and the
/// top-level sequence condition gate. `Err(())` means "skipped — caller
/// must `return Ok(())`".
pub(crate) fn setup_sequential(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    effect: &AbilityEffect,
) -> Result<Option<AbilityTraceNode>, ()> {
    let conditional = effect.conditional.unwrap_or(false);
    let is_further = effect.is_further.unwrap_or(false);
    let seq_label = if conditional {
        "sequential_conditional".to_string()
    } else if is_further {
        "sequential_further".to_string()
    } else {
        "sequential".to_string()
    };

    let card_name = resolver
        .activating_card_id
        .and_then(|cid| gs.card_database.get_card(cid))
        .map(|c| c.name.to_string());

    let seq_node = resolver.debug_trace.then(|| {
        AbilityTraceNode::new(seq_label)
            .with_card(card_name.clone())
            .with_before(ZoneSnapshot::from_game_state(gs))
    });

    resolver.step_state.clear();

    let cond_met = {
        let ctx = ConditionContext::new(gs);
        effect
            .condition
            .as_ref()
            .is_none_or(|c| ctx.evaluate_condition(c))
    };
    if !cond_met {
        log::debug!("[SEQUENCE] source={:?} action={} skipped: sequence condition failed", resolver.activating_card_id, effect.action);
        return Err(());
    }
    Ok(seq_node)
}