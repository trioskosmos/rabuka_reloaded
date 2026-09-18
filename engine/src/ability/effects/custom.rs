//! Custom action handler extracted from misc.rs

use crate::ability::enums::Zone;
use crate::ability::resolver::AbilityResolver;
use crate::card::{AbilityEffect, PlacementOrder};
use crate::game_state::GameState;
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

impl AbilityResolver {
    /// Handles "custom" actions that could not be parsed into a standard action type.
    pub(crate) fn execute_custom(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        action_str: &str,
    ) -> Result<(), String> {
        // 1) Deck reordering: placement_order=any_order → route as move_cards looked_at→deck_top
        if effect.placement_order_any() == Some(PlacementOrder::AnyOrder) {
            let mut routed = effect.clone();
            routed.action = crate::ability::enums::ActionType::MoveCards;
            if routed.source.is_none() {
                routed.source = Some(Zone::LookedAt.to_str().into());
            }
            if routed.destination.is_none() {
                routed.destination = Some(Zone::DeckTop.to_str().into());
            }
            self.current_effect = Some(routed.clone());
            return self.execute_move_cards(gs, &routed);
        }

        // 2) Complex conditional scoring / gain_ability: has duration
        if effect.duration_any().is_some() {
            let text = if effect.text.is_empty() {
                action_str
            } else {
                &effect.text
            };
            return self.execute_gain_ability(
                gs,
                text,
                effect.target_any().unwrap_or("self"),
                effect.duration_any().as_deref(),
                effect.gained_effect_any().cloned(),
                effect.ability_gain_trigger_any().as_deref(),
                gs.activating_card,
            );
        }

        log::debug!("Unhandled custom action: {}", action_str);
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: [[log_custom_effect]]", pp, act_name));
        Ok(())
    }
}