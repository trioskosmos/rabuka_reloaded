pub mod ability_effects;
pub mod draw;
pub mod misc;
pub mod score;
pub mod state;

pub(crate) use draw::draw_cards_for_player;

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
    // Q55: Effects resolve as much as possible; partial resolution required when full is impossible.
    pub fn execute_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let mut dbg = AbDebug::new();
        log::debug!(
            "[DEBUG_EXEC] execute_effect: action={} effect_ptr={:p}",
            effect.action,
            effect
        );
        dbg.effect(effect);
        log::debug!(
            "DEBUG: execute_effect - action: {}, source: {}, destination: {}",
            effect.action,
            effect.source_or("none"),
            effect.destination.as_deref().unwrap_or("none")
        );
        #[cfg(not(feature = "no_std"))]
        let exec_snapshot = crate::ability::log::buffer_len();
        if !self.can_activate_effect(gs, effect) {
            log::debug!("DEBUG: cannot activate effect");
            // Keep verdicts — condition failure info will be captured by push_ability_result
            return Ok(());
        }
        // Drain condition verdicts from the can_activate_effect pre-check;
        // the effect execution will produce its own items and we don't want duplicates.
        #[cfg(not(feature = "no_std"))]
        {
            let _pre = crate::ability::log::drain_verdicts_since(exec_snapshot);
        }

        // non_stackable check: skip if this effect is already active
        if effect.non_stackable.unwrap_or(false) {
            let effect_key = format!("{}:{}", effect.action, effect.text);
            if gs.non_stackable_effects.iter().any(|x| x == &effect_key) {
                log::debug!(
                    "DEBUG: non-stackable effect already active, skipping: {}",
                    effect_key
                );
                return Ok(());
            }
            gs.non_stackable_effects.push(effect_key);
        }

        // Log effect execution to rule_log
        // Log effect execution to rule_log (skip compound/dispatch-only types
        // whose sub-actions log individually, to avoid duplicates).
        if !effect.text.is_empty()
            && !matches!(
                effect.action,
                ActionType::CompoundAction
                    | ActionType::Sequential
                    | ActionType::Choice
                    | ActionType::ConditionalAlternative
                    | ActionType::ConditionalOnResult
                    | ActionType::ConditionalOnOptional
            )
        {
            // Effect details are captured in the structured ability_resolution entry
            // — no separate [effect] text entry needed.
        }

        // Legacy opponent_action wrapper (pre-parser-flatten). Flat effects
        // carry target="opponent" directly and dispatch via ActionType.
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
                return Ok(());
            }
        }

        gs.reset_replacement_effect_flags();
        let action_str = effect.action.to_str();

        // Empty action (default) with action_by means it was entirely handled by opponent
        if effect.action == ActionType::Custom && effect.action_by().is_some() {
            return Ok(());
        }

        // G3: for non-empty actions with action_by: opponent, tag spawn context
        // so choices created inside are routed to the opponent player.
        if effect.action_by().as_deref() == Some("opponent") {
            self.spawn_context.target = Some("opponent".to_string());
        }

        let replacement_indices: Vec<usize> = gs
            .replacement_effects
            .iter()
            .enumerate()
            .filter(|(_, r)| r.original_event == action_str && !r.applied_this_event)
            .map(|(i, _)| i)
            .collect();

        if !replacement_indices.is_empty() {
            for idx in replacement_indices {
                if gs.replacement_effects[idx].is_choice_based {
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
                    return Err("Pending choice required: apply replacement effect".to_string());
                } else {
                    let effects_to_execute =
                        gs.replacement_effects[idx].replacement_effects.clone();
                    let card_id = gs.replacement_effects[idx].card_id;
                    for replacement_effect in &effects_to_execute {
                        self.execute_effect(gs, replacement_effect)?;
                    }
                    gs.mark_replacement_effect_applied(card_id);
                }
            }
            return Ok(());
        }

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
                return Ok(());
            }
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

        // Rule 9.2.1: Effect dispatch
        //
        // Convert string action to typed enum for stronger dispatch.
        // Each ActionType variant maps to a dedicated handler:
        //
        //   Sequential           → execute_sequential_effect (compound.rs)
        //   ConditionalAlternative → execute_conditional_alternative (compound.rs)
        //   LookAndSelect        → execute_look_and_select (look.rs)
        //   SelectCards          → execute_select_cards (look.rs)
        //   Draw/DrawCard        → execute_draw_wrapper
        //   MoveCards/DiscardCard→ execute_move_cards (move_cards.rs)
        //   PayEnergy            → execute_pay_energy (effects/misc.rs)
        //   ...and many more
        //
        // Compound routing: Sequential and LookAndSelect both route through
        // the generic sequential pipeline when they carry `effect_steps`.
        // LookAndSelect is collapsed by the parser into:
        //   [look_step, select_cards_step, move_selected_step]
        // The sequential pipeline executes them in order, creating a pending
        // choice on the select_cards step and resuming naturally when the
        // player responds. Legacy dedicated handlers remain as fallback
        // for the case where effect_steps is absent.
        let action_type = effect.action;
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!(
                "[EXEC_ACTION] action_type={:?} has_steps={} has_actions={}",
                action_type,
                effect.effect_steps.is_some(),
                effect.compound.actions.is_some()
            );
        }

        // Rule 9.8.1 / Q85 / Q86: Sequential/LookAndSelect routing
        //
        // Q85: LookAndSelect with insufficient deck → refresh mid-look
        //   (handled inside execute_look_at, which implements the 4-step
        //   procedure: look available → refresh → look remaining → resolve)
        //
        // Q86: LookAndSelect with exactly enough cards → no refresh during
        //   look. After resolution, if deck is 0, refresh on next check timing.
        if action_type == ActionType::Sequential || action_type == ActionType::LookAndSelect {
            let steps = effect.normalized_steps();
            if !steps.is_empty() {
                if action_type == ActionType::Sequential {
                    log::debug!(
                        "[DEBUG_STEPS] sequential: n_steps={} actions=[{}] effect.action={}",
                        steps.len(),
                        steps
                            .iter()
                            .map(|s| s.action.to_str())
                            .collect::<Vec<_>>()
                            .join(","),
                        effect.action
                    );
                }
                let mut normalized = effect.clone();
                normalized.effect_steps = None;
                normalized.compound.actions = Some(steps);
                normalized.action = ActionType::Sequential;
                return self.execute_sequential_effect(gs, &normalized);
            }
        }

        let result = match action_type {
            ActionType::Sequential => self.execute_sequential_effect(gs, effect),
            ActionType::ConditionalAlternative => self.execute_conditional_alternative(gs, effect),
            ActionType::LookAndSelect => self.execute_look_and_select(gs, effect),
            ActionType::SelectCards => self.execute_select_cards(gs, effect),
            ActionType::DrawCard => self.execute_draw_wrapper(gs, effect),
            ActionType::DrawUntilCount => {
                self.execute_draw_until_count(gs, effect);
                Ok(())
            }
            ActionType::DiscardCard => {
                let pp = self.player_prefix(gs);
                let cn = gs
                    .activating_card
                    .and_then(|id| gs.card_database.get_card(id))
                    .map(|c| c.name.to_string())
                    .unwrap_or_default();
                gs.push_rule_log(format!("{} {}: [[log_discard]]", pp, cn));
                self.execute_move_cards(gs, effect)
            }
            ActionType::MoveCards => {
                let pp = self.player_prefix(gs);
                let cn = gs
                    .activating_card
                    .and_then(|id| gs.card_database.get_card(id))
                    .map(|c| c.name.to_string())
                    .unwrap_or_default();
                gs.push_rule_log(format!("{} {}: [[log_move]]", pp, cn));
                self.execute_move_cards(gs, effect)
            }
            ActionType::GainResource => self.execute_gain_resource(gs, effect),
            ActionType::ChangeState => self.execute_change_state(gs, effect),
            ActionType::ModifyScore => self.execute_modify_score(gs, effect),
            ActionType::ModifyRequiredHearts => self.execute_modify_required_hearts(gs, effect),
            ActionType::SetCost => {
                self.execute_set_cost(gs, effect);
                Ok(())
            }
            ActionType::SetBladeType => {
                self.execute_set_blade_type(gs, effect);
                Ok(())
            }
            ActionType::SetHeartType => {
                self.execute_set_heart_type(gs, effect);
                Ok(())
            }
            ActionType::ActivateAbility => {
                self.execute_activate_ability(gs, effect);
                Ok(())
            }
            ActionType::InvalidateAbility => self.execute_invalidate_ability(gs, effect),
            ActionType::SuppressAbilityTrigger => self.execute_suppress_ability_trigger(gs, effect),
            ActionType::GainAbility => self.execute_gain_ability_effect(gs, effect),
            ActionType::GainAbilityFromSource => self.execute_gain_ability_from_source(gs, effect),
            ActionType::PlayBatonTouch => self.execute_play_baton_touch(gs, effect),
            ActionType::Reveal => self.execute_reveal_effect(gs, effect),
            ActionType::Select => {
                let pp = self.player_prefix(gs);
                let cn = gs
                    .activating_card
                    .and_then(|id| gs.card_database.get_card(id))
                    .map(|c| c.name.to_string())
                    .unwrap_or_default();
                gs.push_rule_log(format!("{} {}: [[log_select]]", pp, cn));
                self.execute_select_effect(gs, effect)
            }
            ActionType::SelectNumber => self.execute_select_number(gs, effect),
            ActionType::LookAt => self.execute_look_at(gs, effect),
            ActionType::ModifyRequiredHeartsGlobal => self.execute_modify_required_hearts_standard(
                gs,
                effect.operation_any().as_deref().unwrap_or("increase"),
                effect.value_or_count(1) as u8,
                effect.heart_colors_any(),
                effect.target_name(),
            ),
            ActionType::ModifyYellCount => {
                self.execute_modify_yell_count(gs, effect);
                Ok(())
            }
            ActionType::PlaceEnergyUnderMember => {
                self.execute_place_energy_under_member(gs, effect);
                Ok(())
            }
            ActionType::ActivationCost => {
                self.execute_activation_cost(gs, effect);
                Ok(())
            }
            ActionType::PositionChange => self.execute_position_change(
                gs,
                effect,
                effect.position_any().cloned(),
                effect.target_name(),
                effect
                    .target_member_any()
                    .as_deref()
                    .unwrap_or("this_member"),
            ),
            ActionType::Rotation => self.execute_rotation(gs, effect, effect.target_name()),

            ActionType::Choice => self.execute_choice(gs, effect),
            ActionType::PayEnergy => self.execute_pay_energy(gs, effect),
            ActionType::SetCardIdentity => self.execute_set_card_identity_effect(gs, effect),
            ActionType::RepeatProcedure => self.execute_repeat_procedure(gs, effect),
            ActionType::DiscardUntilCount => self.execute_discard_until_count(gs, effect),
            // Q57: A "cannot do X" effect takes priority over an effect that would do X.
            ActionType::Restriction => self.execute_restriction(gs, effect),
            ActionType::ReYell => {
                self.execute_re_yell(gs, effect);
                Ok(())
            }
            ActionType::ActivationRestriction => {
                self.execute_activation_restriction(gs, effect);
                Ok(())
            }
            ActionType::ChooseRequiredHearts => {
                self.execute_choose_required_hearts(gs);
                Ok(())
            }
            ActionType::ModifyLimit => self.execute_modify_limit(gs, effect),
            ActionType::ReduceLiveCardSetLimit => {
                self.execute_reduce_live_card_set_limit(gs, effect);
                Ok(())
            }
            ActionType::SetBladeCount => {
                self.execute_set_blade_count(gs, effect);
                Ok(())
            }
            ActionType::Custom => self.execute_custom(gs, effect, action_str),
            ActionType::DoNothing => Ok(()),
            // G8: yell-source is a 常時 modifier, applied by refresh_yell_sources
            // during recalculate_constants — nothing to execute as a one-shot.
            ActionType::ModifyYellSource => Ok(()),

            ActionType::SpecifyHeartColor => {
                self.execute_specify_heart_color(gs, effect);
                Ok(())
            }
            ActionType::ModifyRequiredHeartsSuccess => {
                self.execute_modify_required_hearts_success(gs, effect);
                Ok(())
            }
            ActionType::SetCostToUse => self.execute_set_cost_to_use(gs, effect),
            ActionType::AllBladeTiming => {
                self.execute_all_blade_timing(gs, effect);
                Ok(())
            }
            ActionType::SetCardIdentityAllRegions => {
                self.execute_set_card_identity_all_regions(gs, effect);
                Ok(())
            }
            ActionType::Shuffle => {
                self.execute_shuffle(gs, effect);
                Ok(())
            }
            ActionType::RevealPerGroup => self.execute_reveal_per_group(gs, effect),
            ActionType::ConditionalOnResult => self.execute_conditional_on_result(gs, effect),
            ActionType::ConditionalOnOptional => self.execute_conditional_on_optional(gs, effect),
            ActionType::ModifyCost => {
                self.execute_modify_cost(gs, effect);
                Ok(())
            }
            ActionType::RevealUntilLiveCard => self.execute_reveal_until_live_card(gs, effect),
            ActionType::RevealUntilChosenCard => self.execute_reveal_until_chosen_card(gs, effect),
            ActionType::ChooseTargetPlayer => self.execute_choose_target_player(gs, effect),
            ActionType::PerformYell => {
                self.execute_perform_yell(gs, effect);
                Ok(())
            }
            // Dispatch-only internal variants — these are routed via separate code paths
            ActionType::CompoundAction
            | ActionType::OpponentAction
            | ActionType::ActionBy
            | ActionType::SequentialCost
            | ActionType::ChoiceCondition
            | ActionType::EnergyCondition => {
                log::warn!(
                    "Unexpected internal action type in execute_effect: {:?}",
                    effect.action
                );
                Ok(())
            }
            ActionType::ConditionalOptional => self.execute_conditional_on_optional(gs, effect),
        };
        // Push effect verdict for non-structural action types
        let is_structural = matches!(
            effect.action,
            ActionType::CompoundAction
                | ActionType::Sequential
                | ActionType::Choice
                | ActionType::ConditionalAlternative
                | ActionType::ConditionalOnResult
                | ActionType::ConditionalOnOptional
        );
        if !is_structural {
            #[cfg(not(feature = "no_std"))]
            let val = effect
                .count
                .or(effect.value_any())
                .map(|v| v.to_string())
                .unwrap_or_default();
            #[cfg(not(feature = "no_std"))]
            let details = if !val.is_empty() {
                format!("{} {}", effect.action, val)
            } else {
                effect.action.to_string()
            };
            #[cfg(not(feature = "no_std"))]
            crate::ability::log::push_verdict(crate::ability::log::AbilityLogItem::Effect {
                text: effect.text.to_string(),
                action: effect.action.to_string(),
                details,
            });
        }
        result
    }
}
