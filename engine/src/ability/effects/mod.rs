pub mod ability_effects;
pub mod custom;
pub mod draw;
pub mod executor;
pub mod misc;
mod reveal;
pub mod score;
pub mod state;

pub(crate) use draw::draw_cards_for_player;
pub(crate) use executor::execute_effect as execute_effect_dispatch;

use super::debug::AbDebug;
use super::enums::ActionType;
use super::resolver::AbilityResolver;
use super::types::Choice;
use super::util;
use crate::card::AbilityEffect;
use crate::game_state::GameState;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl AbilityResolver {
    /// Activation gate: skipped effects resolve to Ok without verdicts —
    /// condition failure info is captured by push_ability_result upstream.
    /// Returns true when the caller must `return Ok(())`.
    fn gate_effect_activation(&mut self, gs: &mut GameState, effect: &AbilityEffect) -> bool {
        if self.can_activate_effect(gs, effect) {
            return false;
        }
        log::trace!("[EFFECT] source={:?} action={} skipped: activation gate failed", self.activating_card_id, effect.action);
        // Keep verdicts — condition failure info will be captured by push_ability_result
        true
    }

    /// Q118 all-or-nothing "そうしたとき": after an accepted
    /// conditional_on_optional placement that could not place every required
    /// card, the trailing draw consequence must not fire — whether it runs
    /// in the sequential loop or via resume_pending_actions.
    fn gate_incomplete_placement(&mut self, gs: &mut GameState, effect: &AbilityEffect) -> bool {
        let stopped = super::gates::incomplete_placement_gate(self, gs, effect).is_stop();
        if stopped {
            log::debug!(
                "[EFFECT] source={:?} action={} skipped: placement incomplete (Q118)",
                self.activating_card_id,
                effect.action
            );
        }
        stopped
    }

    /// Non-stackable check: skip if this effect is already active.
    /// Returns true when the caller must `return Ok(())`.
    fn check_non_stackable(&mut self, gs: &mut GameState, effect: &AbilityEffect) -> bool {
        let stopped = super::gates::non_stackable_gate(self, gs, effect).is_stop();
        if stopped {
            log::debug!(
                "[EFFECT] source={:?} action={} skipped: non-stackable effect already active",
                self.activating_card_id,
                effect.action
            );
        }
        stopped
    }

    /// Legacy opponent_action wrapper (pre-parser-flatten) + flat
    /// `action_by: opponent` tagging. Returns `Ok(true)` when the caller
    /// must `return Ok(())` (unwrapped recursion finished, or a fully
    /// opponent-handled Custom needs nothing).
    fn prepare_opponent_routing(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<bool, String> {
        // Flat effects carry target="opponent" directly and dispatch via
        // ActionType; only the legacy wrapper needs unwrapping.
        if effect.action == ActionType::OpponentAction {
            if let Some(ref opponent_action) = effect.opponent_action() {
                // G3: tag spawn context so choices created for this
                // opponent action are routed to the opponent player.
                self.spawn_context.target = Some("opponent".to_string());
                let mut modified = (*opponent_action).clone();
                if modified.target.is_none() || modified.target.as_deref() == Some("self") {
                    modified.target = Some("opponent".into());
                }
                self.execute_effect(gs, &modified)?;
                return Ok(true);
            }
        }

        // Empty action (default) with action_by means it was entirely handled by opponent
        if effect.action == ActionType::Custom && effect.action_by().is_some() {
            return Ok(true);
        }

        // G3: for non-empty actions with action_by: opponent, tag spawn context
        // so choices created inside are routed to the opponent player.
        if effect.action_by().as_deref() == Some("opponent") {
            self.spawn_context.target = Some("opponent".to_string());
        }
        Ok(false)
    }

    /// Run registered replacement effects for `action_str`.
    /// Returns `Ok(true)` when handled (caller returns `Ok(())`); the
    /// choice-based path returns `Err` to signal the pending choice.
    fn run_replacement_effects(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<bool, String> {
        let action_str = effect.action.to_str();
        if self.resolving_replacement {
            return Ok(false);
        }
        if !self.resolving_replacement {
            gs.reset_replacement_effect_flags();
            self.resolving_replacement = true;
        }
        let replacement_indices: Vec<usize> = gs
            .replacement_effects
            .iter()
            .enumerate()
            .filter(|(_, r)| r.original_event == action_str && !r.applied_this_event)
            .map(|(i, _)| i)
            .collect();

        if replacement_indices.is_empty() {
            self.resolving_replacement = false;
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
                self.pending_replacement = Some((idx, effect.clone()));
                let description =
                    format!("Apply replacement effect for action '{}'?", action_str);
                self.pending_choice = Some(Choice::SelectTarget {
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
        self.resolving_replacement = false;
        Ok(true)
    }

    /// Register a "replacement" effect_type for its original event.
    /// Returns true when registered (caller returns `Ok(())`).
    fn register_replacement_effect(&mut self, gs: &mut GameState, effect: &AbilityEffect) -> bool {
        if let Some(ref effect_type) = effect.effect_type() {
            if *effect_type == "replacement" {
                let original_event = effect.replaces_event_any().clone();
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
                        player_id,
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

    /// Rule 9.2.1 compound routing: Sequential and LookAndSelect carrying
    /// `effect_steps` collapse into the generic sequential pipeline
    /// ([look, select, move] in order). Returns the normalized Sequential
    /// effect when this route applies.
    fn normalized_sequential_route(
        effect: &AbilityEffect,
        action_type: ActionType,
    ) -> Option<AbilityEffect> {
        // Rule 9.8.1 / Q85 / Q86: Sequential/LookAndSelect routing
        //
        // Q85: LookAndSelect with insufficient deck → refresh mid-look
        //   (handled inside execute_look_at, which implements the 4-step
        //   procedure: look available → refresh → look remaining → resolve)
        //
        // Q86: LookAndSelect with exactly enough cards → no refresh during
        //   look. After resolution, if deck is 0, refresh on next check timing.
        if action_type != ActionType::Sequential && action_type != ActionType::LookAndSelect {
            return None;
        }
        let steps = effect.normalized_steps();
        if steps.is_empty() {
            return None;
        }
        let mut normalized = effect.clone();
        normalized.effect_steps = None;
        normalized.compound.actions = Some(steps);
        normalized.action = ActionType::Sequential;
        Some(normalized)
    }

    /// Shared MoveCards/DiscardCard tail (rule log + current-effect pinning
    /// + move execution) — the registry's logged-move arms delegate here so
    /// the 3-line sequence lives in exactly one place.
    pub(crate) fn execute_logged_move(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        log_tag: &str,
    ) -> Result<(), String> {
        self.rule_log_activated(gs, log_tag);
        self.current_effect = Some(effect.clone());
        self.execute_move_cards(gs, effect)
    }

    // Q55: Effects resolve as much as possible; partial resolution required when full is impossible.
    pub fn execute_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let mut dbg = AbDebug::new();
        dbg.effect(effect);
        log::trace!(
            "[EFFECT] source={:?} action={} from={} to={} has_steps={} has_actions={}",
            self.activating_card_id,
            effect.action,
            effect.source_or("none"),
            effect.destination.map(|z| z.as_str()).unwrap_or("none"),
            effect.effect_steps.is_some(),
            effect.compound.actions.is_some()
        );
        #[cfg(not(feature = "no_std"))]
        let exec_snapshot = crate::ability::log::buffer_len();
        if self.gate_effect_activation(gs, effect) {
            return Ok(());
        }
        if self.gate_incomplete_placement(gs, effect) {
            return Ok(());
        }
        // Drain condition verdicts from the can_activate_effect pre-check;
        // the effect execution will produce its own items and we don't want duplicates.
        #[cfg(not(feature = "no_std"))]
        {
            let _pre = crate::ability::log::drain_verdicts_since(exec_snapshot);
        }

        // non_stackable check: skip if this effect is already active
        if self.check_non_stackable(gs, effect) {
            return Ok(());
        }

        // Effect details are captured in the structured ability_resolution entry

        if self.prepare_opponent_routing(gs, effect)? {
            return Ok(());
        }

        if self.run_replacement_effects(gs, effect)? {
            return Ok(());
        }
        if self.replacement_original_suppressed {
            self.replacement_original_suppressed = false;
            log::debug!(
                "[REPLACEMENT] action={} original_suppressed=true",
                effect.action
            );
            return Ok(());
        }

        if self.register_replacement_effect(gs, effect) {
            return Ok(());
        }

        // Rule 9.8 / Q158: Handle target="both" generically
        //   execute once for self, then opponent.
        //   position_change handles "both" internally (opponent first, then self).
        //
        // Q158: "Does blade+2 from the effect apply to all members on stage?"
        //   → Yes. "自分のステージにいるメンバー" means all members.
        if self.handle_both_targets(gs, effect)? {
            return Ok(());
        }

        // Rule 9.2.1: Effect dispatch — table-driven via executor registry
        let action_type = effect.action;

        // Compound routing: Sequential and LookAndSelect both route through
        // the generic sequential pipeline when they carry `effect_steps`.
        if let Some(normalized) = Self::normalized_sequential_route(effect, action_type) {
            return self.execute_sequential_effect(gs, &normalized);
        }

        execute_effect_dispatch(self, gs, effect)
    }
}
