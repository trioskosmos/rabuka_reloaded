pub mod ability_effects;
pub mod draw;
pub mod misc;
pub mod score;
pub mod state;

pub(crate) use draw::draw_cards_for_player;

use super::debug::AbDebug;
use super::enums::{ActionType, Zone};
use super::resolver::AbilityResolver;
use super::types::Choice;
use super::util;
use crate::card::AbilityEffect;
use crate::game_state::GameState;

impl AbilityResolver {
    pub fn execute_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let mut dbg = AbDebug::new();
        dbg.effect(effect);
        log::debug!(
            "DEBUG: execute_effect - action: {}, source: {}, destination: {}",
            effect.action,
            effect.source_or("none"),
            effect.destination.as_deref().unwrap_or("none")
        );
        if !self.can_activate_effect(gs, effect) {
            log::debug!("DEBUG: cannot activate effect");
            return Ok(());
        }

        // non_stackable check: skip if this effect is already active
        if effect.non_stackable.unwrap_or(false) {
            let effect_key = format!("{}:{}", effect.action, effect.text);
            if gs.non_stackable_effects.contains(&effect_key) {
                log::debug!(
                    "DEBUG: non-stackable effect already active, skipping: {}",
                    effect_key
                );
                return Ok(());
            }
            gs.non_stackable_effects.insert(effect_key);
        }

        if effect.action_by.as_deref() == Some("opponent") || effect.action == "opponent_action" {
            if let Some(ref opponent_action) = effect.opponent_action {
                let mut modified = opponent_action.clone();
                if modified.target.is_none() || modified.target.as_deref() == Some("self") {
                    modified.target = Some("opponent".to_string());
                }
                self.execute_effect(gs, &modified)?;
                return Ok(());
            }
        }

        gs.reset_replacement_effect_flags();
        let action_str = effect.action.as_str();

        // Empty action with opponent_action means it was entirely handled by opponent
        if action_str.is_empty() && effect.action_by.is_some() {
            return Ok(());
        }

        let replacement_indices: Vec<usize> = gs.replacement_effects
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
                        description,
                        allow_skip: false,
                        options: None,
                    });
                    return Err("Pending choice required: apply replacement effect".to_string());
                } else {
                    let effects_to_execute = gs.replacement_effects[idx].replacement_effects.clone();
                    let card_id = gs.replacement_effects[idx].card_id;
                    for replacement_effect in &effects_to_execute {
                        self.execute_effect(gs, replacement_effect)?;
                    }
                    gs.mark_replacement_effect_applied(card_id);
                }
            }
            return Ok(());
        }

        if let Some(ref effect_type) = effect.effect_type {
            if effect_type == "replacement" {
                let original_event = effect.replaces_event.clone();
                let is_choice_based = effect.choice_based.unwrap_or(false);
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
                        event.clone(),
                        vec![effect.clone()],
                        is_choice_based,
                    );
                }
                return Ok(());
            }
        }

        // Handle target="both" generically: execute once for self, then opponent.
        // position_change handles "both" internally (opponent choice first, then self).
        if self.handle_both_targets(gs, effect)? {
            return Ok(());
        }

        // Convert string action to typed enum for stronger dispatch
        let action_type = ActionType::from_str(&action_str).unwrap_or(ActionType::Custom);

        match action_type {
            ActionType::Sequential => self.execute_sequential_effect(
                gs,
                effect,
                effect.conditional.unwrap_or(false),
                effect.is_further.unwrap_or(false),
            ),
            ActionType::ConditionalAlternative => self.execute_conditional_alternative(gs, effect),
            ActionType::LookAndSelect => self.execute_look_and_select(gs, effect),
            ActionType::SelectCards => self.execute_select_cards(gs, effect),
            ActionType::Draw | ActionType::DrawCard => self.execute_draw_wrapper(gs, effect),
            ActionType::DrawUntilCount => {
                self.execute_draw_until_count(
                    gs,
                    effect.target_count.unwrap_or(0),
                    effect.target_name(),
                    effect.destination.as_deref().unwrap_or(Zone::Hand.to_str()),
                );
                Ok(())
            }
            ActionType::DiscardCard | ActionType::MoveCards => self.execute_move_cards(gs, effect),
            ActionType::GainResource => self.execute_gain_resource(gs, effect),
            ActionType::ChangeState => {
                let change_cost_limit = if effect.cost_from_revealed.unwrap_or(false) {
                    gs.revealed_cards
                        .first()
                        .and_then(|&cid| gs.card_database.get_card(cid))
                        .and_then(|c| c.cost)
                } else {
                    effect.cost_limit
                };
                let mut change_count = effect.count_or(0);
                let mut change_group = effect.group_name();
                if effect.per_unit.unwrap_or(false) {
                    let player = gs.resolve_target_player(effect.target_name());
                    let location = effect.location.as_deref().unwrap_or(Zone::Stage.to_str());
                    let cards: Vec<i16> = util::zone_cards(player, location).to_vec();
                    let per_unit_filter = util::filter_from_parts(
                        effect.card_type.as_deref(),
                        change_group,
                        change_cost_limit,
                        None,
                        effect.characters.as_ref(),
                        None,
                        None,
                    );
                    let count = cards
                        .iter()
                        .filter(|&&cid| per_unit_filter.matches(&gs.card_database, cid, false))
                        .count() as u32;
                    let per_unit_cnt = effect.per_unit_count.unwrap_or(1);
                    change_count = (count / per_unit_cnt) * change_count.max(1);
                    change_group = None;
                }
                self.execute_change_state(
                    gs,
                    effect,
                    effect.state_change.as_deref().unwrap_or(""),
                    effect.target_name(),
                    change_count,
                    effect.max.unwrap_or(false),
                    effect.card_type.as_deref(),
                    change_cost_limit,
                    effect.optional.unwrap_or(false),
                    change_group,
                    effect.self_cost.unwrap_or(false),
                    effect.source.as_deref(),
                    effect.destination.as_deref(),
                    effect.cost_limit_operator.clone(),
                    effect.characters.as_ref(),
                    effect.blade_limit,
                    effect.blade_limit_operator.as_deref(),
                )
            }
            ActionType::ModifyScore => self.execute_modify_score(
                gs,
                effect,
                effect.operation.as_deref().unwrap_or("add"),
                effect.value.unwrap_or(0),
                effect.target_name(),
                effect.duration.as_deref(),
                effect.card_type.as_deref(),
                effect.group_name(),
                effect.per_unit.unwrap_or(false),
                effect.per_unit_count.unwrap_or(1),
                effect.per_unit_type.as_deref(),
                effect.effect_constraint.as_deref(),
                effect.self_target.unwrap_or(false),
                &effect.heart_colors,
            ),
            ActionType::ModifyRequiredHearts => self.execute_modify_required_hearts(
                gs,
                effect.operation.as_deref().unwrap_or("decrease"),
                effect.value_or_count(0),
                &effect.heart_colors,
                effect.target_name(),
                effect.per_unit.unwrap_or(false),
                effect.per_unit_count.unwrap_or(1),
                effect.group_name(),
                effect.timing_condition.as_deref(),
                effect.location.as_deref(),
            ),
            ActionType::SetCost => {
                self.execute_set_cost(
                    gs,
                    effect.value.unwrap_or(0),
                    effect.target_name(),
                    effect.card_type.as_deref(),
                );
                Ok(())
            }
            ActionType::SetBladeType => {
                self.execute_set_blade_type(
                    gs,
                    effect.blade_type.as_deref(),
                    effect.target_name(),
                    effect.duration.as_deref(),
                );
                Ok(())
            }
            ActionType::SetHeartType => {
                self.execute_set_heart_type(
                    gs,
                    effect
                        .heart_type
                        .as_deref()
                        .or(effect.heart_colors.first().map(|s| s.as_str())),
                    effect.target_name(),
                    effect.count_or(1) as i32,
                );
                Ok(())
            }
            ActionType::ActivateAbility => {
                self.execute_activate_ability(
                    gs,
                    effect.ability_text.as_deref().unwrap_or(""),
                    effect.target_trigger.as_deref(),
                    effect.count,
                    effect.source_card.as_deref(),
                );
                Ok(())
            }
            ActionType::InvalidateAbility => self.execute_invalidate_ability(gs, effect),
            ActionType::GainAbility => self.execute_gain_ability_effect(gs, effect),
            ActionType::GainAbilityFromSource => self.execute_gain_ability_from_source(gs, effect),
            ActionType::PlayBatonTouch => {
                self.execute_play_baton_touch(gs, effect.count_or(1), effect.target_name())
            }
            ActionType::Reveal => self.execute_reveal_effect(gs, effect),
            ActionType::Select => self.execute_select_effect(gs, effect),
            ActionType::LookAt => {
                let count = if let Some(ref dc) = effect.dynamic_count {
                    self.resolve_dynamic_count(gs, dc)
                } else {
                    effect.count_or(1)
                };
                self.execute_look_at(
                    gs,
                    effect,
                    count,
                    effect.target_name(),
                    effect.source_or(Zone::Deck.to_str()),
                )
            }
            ActionType::ModifyRequiredHeartsGlobal => self.execute_modify_required_hearts_standard(
                gs,
                effect.operation.as_deref().unwrap_or("increase"),
                effect.value_or_count(1),
                &effect.heart_colors,
                effect.target_name(),
            ),
            ActionType::ModifyYellCount => {
                self.execute_modify_yell_count(
                    gs,
                    effect.operation.as_deref().unwrap_or("subtract"),
                    effect.count_or(0),
                );
                Ok(())
            }
            ActionType::PlaceEnergyUnderMember => {
                self.execute_place_energy_under_member(
                    gs,
                    effect.energy_count.unwrap_or(1),
                    effect.target_name(),
                    effect.position.as_ref(),
                    effect.optional.unwrap_or(false),
                    effect.source.as_deref(),
                );
                Ok(())
            }
            ActionType::ActivationCost => {
                self.execute_activation_cost(
                    gs,
                    effect.operation.as_deref().unwrap_or("increase"),
                    effect.value.unwrap_or(0),
                    effect.target_name(),
                    effect.duration.as_deref(),
                );
                Ok(())
            }
            ActionType::PositionChange => self.execute_position_change(
                gs,
                effect,
                effect.position.clone(),
                effect.target_name(),
                effect.target_member.as_deref().unwrap_or("this_member"),
            ),

            ActionType::Choice => self.execute_choice(gs, effect),
            ActionType::PayEnergy => {
                self.execute_pay_energy(gs, effect.count_or(0), effect.target_name());
                Ok(())
            }
            ActionType::SetCardIdentity => self.execute_set_card_identity_effect(gs, effect),
            ActionType::RepeatProcedure => {
                self.execute_repeat_procedure(gs, effect, effect.repeat_limit.unwrap_or(1))
            }
            ActionType::DiscardUntilCount => self.execute_discard_until_count(
                gs,
                effect.target_count.unwrap_or(0),
                effect.target_name(),
            ),
            ActionType::Restriction => self.execute_restriction(
                gs,
                effect.restriction_type.as_deref(),
                effect
                    .restricted_destination
                    .as_deref()
                    .or(effect.destination.as_deref()),
                effect.target_name(),
                effect.delayed.unwrap_or(false),
            ),
            ActionType::ReYell => {
                self.execute_re_yell(
                    gs,
                    effect.lose_blade_hearts.unwrap_or(false),
                    effect.target_name(),
                );
                Ok(())
            }
            ActionType::ActivationRestriction => {
                self.execute_activation_restriction(gs, effect.target_name());
                Ok(())
            }
            ActionType::ChooseRequiredHearts => {
                self.execute_choose_required_hearts(gs);
                Ok(())
            }
            ActionType::ModifyLimit => self.execute_modify_limit(
                gs,
                effect.operation.as_deref().unwrap_or("decrease"),
                effect.count_or(0),
            ),
            ActionType::SetBladeCount => {
                self.execute_set_blade_count(
                    gs,
                    effect.value.unwrap_or(effect.count_or(0)),
                    effect.target_name(),
                );
                Ok(())
            }
            ActionType::Custom => self.execute_custom(gs, effect, &action_str),
            ActionType::DoNothing => Ok(()),

            ActionType::SpecifyHeartColor => {
                self.execute_specify_heart_color(
                    gs,
                    effect.choice.unwrap_or(false),
                    effect.target_name(),
                );
                Ok(())
            }
            ActionType::ModifyRequiredHeartsSuccess => {
                self.execute_modify_required_hearts_success(
                    gs,
                    effect.operation.as_deref().unwrap_or("increase"),
                    effect.value.unwrap_or(0),
                    effect.target_name(),
                    effect.card_type.as_deref(),
                    &effect.heart_colors,
                );
                Ok(())
            }
            ActionType::SetCostToUse => self.execute_set_cost_to_use(gs, effect.value.unwrap_or(0)),
            ActionType::AllBladeTiming => {
                self.execute_all_blade_timing(
                    gs,
                    effect.timing.as_deref().unwrap_or("check_required_hearts"),
                    effect.treat_as.as_deref().unwrap_or("any_heart_color"),
                );
                Ok(())
            }
            ActionType::SetCardIdentityAllRegions => {
                self.execute_set_card_identity_all_regions(
                    gs,
                    effect.identities.as_ref(),
                    effect.target_name(),
                );
                Ok(())
            }
            ActionType::Shuffle => {
                self.execute_shuffle(
                    gs,
                    effect.target_name(),
                    effect.source_or(Zone::Deck.to_str()),
                );
                Ok(())
            }
            ActionType::RevealPerGroup => self.execute_reveal_per_group(
                gs,
                effect.source_or(Zone::Hand.to_str()),
                effect.count_or(1),
                effect.target_name(),
            ),
            ActionType::ConditionalOnResult => self.execute_conditional_on_result(gs, effect),
            ActionType::ConditionalOnOptional => self.execute_conditional_on_optional(gs, effect),
            ActionType::ModifyCost => {
                let card_db = &gs.card_database;
                let mut value = effect.value.unwrap_or(0);
                if effect.per_unit.unwrap_or(false) {
                    let per_unit_type_str = effect
                        .per_unit_type
                        .as_deref()
                        .or(effect.location.as_deref())
                        .unwrap_or("枚");
                    let player = gs.resolve_target_player(effect.target_name());
                    // Use resolve_per_unit_count which handles under_member,
                    // discard, waitroom_card and other special zones that
                    // zone_cards() cannot represent as a flat slice.
                    let per_unit_filter = util::CardFilter {
                        card_type: effect.card_type.as_deref(),
                        ..util::CardFilter::default()
                    };
                    let matching_count = util::resolve_per_unit_count(
                        true,
                        Some(per_unit_type_str),
                        player,
                        card_db,
                        &per_unit_filter,
                        &[],
                        effect.state.as_deref(),
                        &gs.mods.orientation_modifiers,
                    );
                    let per_unit_count = effect.per_unit_count.unwrap_or(1);
                    let mut units = matching_count / per_unit_count;
                    // Apply max_repeats cap (aliased as repeat_limit).
                    // The text side-constraint "N枚までしか数えない" is parsed as
                    // max_repeats on the effect.
                    if let Some(cap) = effect.repeat_limit {
                        units = units.min(cap);
                    }
                    value *= units;
                }
                self.execute_modify_cost(
                    gs,
                    effect.operation.as_deref().unwrap_or("add"),
                    value,
                    effect.target_name(),
                    effect.card_type.as_deref(),
                    effect.duration.as_deref(),
                );
                Ok(())
            }
            ActionType::RevealUntilLiveCard => {
                self.execute_reveal_until_live_card(gs, effect.target_name())
            }
            ActionType::RevealUntilChosenCard => self.execute_reveal_until_chosen_card(gs, effect),
            ActionType::PerformYell => {
                let count = if let Some(ref dc) = effect.dynamic_count {
                    self.resolve_dynamic_count(gs, dc)
                } else {
                    effect.count_or(1)
                };
                self.execute_perform_yell(gs, count, effect.target_name());
                Ok(())
            }
            _ => {
                log::debug!("Unknown effect action: '{}'", action_str);
                Ok(())
            }
        }
    }
}
