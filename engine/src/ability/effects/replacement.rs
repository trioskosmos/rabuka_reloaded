//! Replacement effects - the mechanism, in one file.
//!
//! Not an `ActionType`: an effect registers itself as a replacement for an
//! event, and when that event fires the registered body runs in place of the
//! original. These two came out of `mod.rs`, where they sat apart from the
//! dispatch that calls them.

use super::super::resolver::AbilityResolver;
use super::super::types::{Choice};


use crate::card::{AbilityEffect};

use crate::game_state::GameState;




#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl AbilityResolver {

    /// Run registered replacement effects for `action_str`.
    /// Returns `Ok(true)` when handled (caller returns `Ok(())`); the
    /// choice-based path returns `Err` to signal the pending choice.
    pub(crate) fn run_replacement_effects(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<bool, String> {
        let action_str = effect.action.to_str();
        if self.awaiting.resolving_replacement {
            return Ok(false);
        }
        if !self.awaiting.resolving_replacement {
            gs.reset_replacement_effect_flags();
            self.awaiting.resolving_replacement = true;
        }
        let replacement_indices: Vec<usize> = gs
            .replacement_effects
            .iter()
            .enumerate()
            .filter(|(_, r)| r.original_event == action_str && !r.applied_this_event)
            .map(|(i, _)| i)
            .collect();

        if replacement_indices.is_empty() {
            self.awaiting.resolving_replacement = false;
            return Ok(false);
        }
        let mut mandatory_effects = Vec::new();
        for idx in replacement_indices {
            if gs.replacement_effects[idx].applied_this_event {
                continue;
            }
            if gs.replacement_effects[idx].is_choice_based {
                for effects in &mandatory_effects {
                    for replacement_effect in effects {
                        self.execute_effect(gs, replacement_effect)?;
                    }
                }
                self.awaiting.replacement = Some((idx, effect.clone()));
                let description =
                    format!("Apply replacement effect for action '{}'?", action_str);
                self.awaiting.choice = Some(Choice::SelectTarget {
                    target: "apply_replacement".to_string(),
                    description: description.clone(),
                    description_en: Some(description.clone()),
                    description_ja: Some(format!(
                        "アクション「{}」の置き換え効果を適用？",
                        action_str
                    )),
                    allow_skip: false,
                    options: None,
                });
                log::debug!(
                    "[REPLACEMENT] event={} registration={} pending=true",
                    action_str,
                    idx
                );
                return Err("Pending choice required: apply replacement effect".to_string());
            }
            let effects_to_execute =
                gs.replacement_effects[idx].replacement_effects.clone();
            gs.mark_replacement_effect_applied(idx);
            log::debug!(
                "[REPLACEMENT] event={} registration={} applied=true choice=false count={:?}",
                action_str,
                idx,
                effects_to_execute
                    .iter()
                    .map(|effect| effect.count_any())
                    .collect::<Vec<_>>()
            );
            mandatory_effects.push(effects_to_execute);
        }
        for effects in &mandatory_effects {
            for replacement_effect in effects {
                self.execute_effect(gs, replacement_effect)?;
            }
        }
        self.awaiting.resolving_replacement = false;
        Ok(true)
    }

    /// Register a "replacement" effect_type for its original event.
    /// Returns true when registered (caller returns `Ok(())`).
    pub(crate) fn register_replacement_effect(&mut self, gs: &mut GameState, effect: &AbilityEffect) -> bool {
        if let Some(effect_type) = effect.effect_type() {
            if effect_type == "replacement" {
                let original_event = effect.replaces_event_any();
                let is_choice_based = effect.choice_based_any().unwrap_or(false);
                let card_id = gs.activating_card.unwrap_or(-1);
                let player_id =
                    if gs.current_turn_phase == crate::game_state::TurnPhase::FirstAttackerNormal {
                        gs.player1.id.clone()
                    } else {
                        gs.player2.id.clone()
                    };
                if let Some(event) = original_event {
                    gs.add_replacement_effect(
                        card_id,
                        player_id.to_string(),
                        event.to_string(),
                        vec![effect.clone()],
                        is_choice_based,
                    );
                }
                return true;
            }
        }
        false
    }
}