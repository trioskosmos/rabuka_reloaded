pub mod ability_effects;
pub mod draw;
pub mod misc;
pub mod score;
pub mod state;

pub(crate) use draw::draw_cards_for_player;

use super::debug::AbDebug;
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
        println!(
            "DEBUG: execute_effect - action: {}, source: {}, destination: {}",
            effect.action,
            effect.source_or("none"),
            effect.destination.as_deref().unwrap_or("none")
        );
        if !self.can_activate_effect(gs, effect) {
            println!("DEBUG: cannot activate effect");
            return Ok(());
        }

        // non_stackable check: skip if this effect is already active
        if effect.non_stackable.unwrap_or(false) {
            let effect_key = format!("{}:{}", effect.action, effect.text);
            if gs.non_stackable_effects.contains(&effect_key) {
                println!(
                    "DEBUG: non-stackable effect already active, skipping: {}",
                    effect_key
                );
                return Ok(());
            }
            gs.non_stackable_effects.insert(effect_key);
        }

        if effect.action_by.as_deref() == Some("opponent") {
            if let Some(ref opponent_action) = effect.opponent_action {
                let mut modified = opponent_action.clone();
                if modified.target.is_none() || modified.target.as_deref() == Some("self") {
                    modified.target = Some("opponent".to_string());
                }
                self.execute_effect(gs, &modified)?;
            }
        }

        gs.reset_replacement_effect_flags();
        let action_str = effect.action.clone();

        // Empty action with opponent_action means it was entirely handled by opponent
        if action_str.is_empty() && effect.action_by.is_some() {
            return Ok(());
        }

        let replacement_effects: Vec<crate::game_state::ReplacementEffect> = gs
            .get_replacement_effects_for_event(&action_str)
            .iter()
            .map(|r| (*r).clone())
            .collect();
        if !replacement_effects.is_empty() {
            for replacement in &replacement_effects {
                if replacement.is_choice_based {
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
                    for replacement_effect in &replacement.replacement_effects {
                        self.execute_effect(gs, replacement_effect)?;
                    }
                    gs.mark_replacement_effect_applied(replacement.card_id);
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

        match action_str.as_str() {
            "sequential" => self.execute_sequential_effect(
                gs,
                effect,
                effect.conditional.unwrap_or(false),
                effect.is_further.unwrap_or(false),
            ),
            "conditional_alternative" => self.execute_conditional_alternative(gs, effect),
            "look_and_select" => self.execute_look_and_select(gs, effect),
            "select_cards" => self.execute_select_cards(gs, effect),
            "draw" | "draw_card" => self.execute_draw_wrapper(gs, effect),
            "draw_until_count" => {
                self.execute_draw_until_count(
                    gs,
                    effect.target_count.unwrap_or(0),
                    effect.target_name(),
                    effect.destination.as_deref().unwrap_or("hand"),
                );
                Ok(())
            }
            "discard_card" | "move_cards" => self.execute_move_cards(gs, effect),
            "gain_resource" => self.execute_gain_resource(gs, effect),
            "change_state" => {
                let mut change_count = effect.count_or(0);
                let mut change_group = effect.group_name();
                if effect.per_unit.unwrap_or(false) {
                    let player = gs.resolve_target_player(&effect.target_name());
                    let location = effect.location.as_deref().unwrap_or("stage");
                    let cards: Vec<i16> = util::zone_cards(player, location).to_vec();
                    let per_unit_filter = util::filter_from_parts(
                        effect.card_type.as_deref(),
                        change_group,
                        effect.cost_limit,
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
                    effect.cost_limit,
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
            "modify_score" => self.execute_modify_score(
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
            "modify_required_hearts" => self.execute_modify_required_hearts(
                gs,
                effect.operation.as_deref().unwrap_or("decrease"),
                effect.value.or(effect.count).unwrap_or(0),
                effect.heart_color_or("heart00"),
                effect.target_name(),
                effect.per_unit.unwrap_or(false),
                effect.per_unit_count.unwrap_or(1),
                effect.group_name(),
                effect.timing_condition.as_deref(),
                effect.location.as_deref(),
            ),
            "set_cost" => {
                self.execute_set_cost(
                    gs,
                    effect.value.unwrap_or(0),
                    effect.target_name(),
                    effect.card_type.as_deref(),
                );
                Ok(())
            }
            "set_blade_type" => {
                self.execute_set_blade_type(
                    gs,
                    effect.blade_type.as_deref(),
                    effect.target_name(),
                    effect.duration.as_deref(),
                );
                Ok(())
            }
            "set_heart_type" => {
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
            "activate_ability" => {
                self.execute_activate_ability(
                    gs,
                    effect.ability_text.as_deref().unwrap_or(""),
                    effect.target_trigger.as_deref(),
                    effect.count,
                );
                Ok(())
            }
            "invalidate_ability" => {
                self.execute_invalidate_ability(gs);
                Ok(())
            }
            "gain_ability" => self.execute_gain_ability_effect(gs, effect),
            "gain_ability_from_source" => self.execute_gain_ability_from_source(gs, effect),
            "play_baton_touch" => {
                self.execute_play_baton_touch(gs, effect.count_or(1), effect.target_name())
            }
            "reveal" => self.execute_reveal_effect(gs, effect),
            "select" => self.execute_select_effect(gs, effect),
            "look_at" => self.execute_look_at(
                gs,
                effect.count_or(1),
                effect.target_name(),
                effect.source_or("deck"),
            ),
            "modify_required_hearts_global" => self.execute_modify_required_hearts_standard(
                gs,
                effect.operation.as_deref().unwrap_or("increase"),
                effect.value.or(effect.count).unwrap_or(1),
                effect.heart_color_or("heart00"),
                effect.target_name(),
            ),
            "modify_yell_count" => {
                self.execute_modify_yell_count(
                    gs,
                    effect.operation.as_deref().unwrap_or("subtract"),
                    effect.count_or(0),
                );
                Ok(())
            }
            "place_energy_under_member" => {
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
            "activation_cost" => {
                self.execute_activation_cost(
                    gs,
                    effect.operation.as_deref().unwrap_or("increase"),
                    effect.value.unwrap_or(0),
                    effect.target_name(),
                    effect.duration.as_deref(),
                );
                Ok(())
            }
            "position_change" => self.execute_position_change(
                gs,
                effect,
                effect.position.clone(),
                effect.target_name(),
                effect.target_member.as_deref().unwrap_or("this_member"),
            ),
            "formation_change" => {
                gs.formation_change_occurred_this_turn = true;
                self.execute_position_change(
                    gs,
                    effect,
                    effect.position.clone(),
                    effect.target_name(),
                    "this_member",
                )
            }
            "appear" => self.execute_appear(gs, effect),
            "choice" => self.execute_choice(
                gs,
                effect.choice_options.as_ref(),
                effect.choice_type.as_deref(),
                effect.options.as_ref(),
                effect.choice_maker.as_deref(),
            ),
            "pay_energy" => {
                self.execute_pay_energy(gs, effect.count_or(0), effect.target_name());
                Ok(())
            }
            "set_card_identity" => self.execute_set_card_identity_effect(gs, effect),
            "repeat_procedure" => {
                self.execute_repeat_procedure(gs, effect, effect.repeat_limit.unwrap_or(1))
            }
            "discard_until_count" => self.execute_discard_until_count(
                gs,
                effect.target_count.unwrap_or(0),
                effect.target_name(),
            ),
            "restriction" => self.execute_restriction(
                gs,
                effect.restriction_type.as_deref(),
                effect.restricted_destination.as_deref(),
                effect.target_name(),
                effect.delayed.unwrap_or(false),
            ),
            "re_yell" => {
                self.execute_re_yell(
                    gs,
                    effect.lose_blade_hearts.unwrap_or(false),
                    effect.target_name(),
                );
                Ok(())
            }
            "activation_restriction" => {
                self.execute_activation_restriction(gs, effect.target_name());
                Ok(())
            }
            "choose_required_hearts" => {
                self.execute_choose_required_hearts(gs);
                Ok(())
            }
            "modify_limit" => self.execute_modify_limit(
                gs,
                effect.operation.as_deref().unwrap_or("decrease"),
                effect.count_or(0),
            ),
            "set_blade_count" => {
                self.execute_set_blade_count(
                    gs,
                    effect.value.unwrap_or(effect.count_or(0)),
                    effect.target_name(),
                );
                Ok(())
            }
            "custom" => self.execute_custom(gs, effect, &action_str),
            "do_nothing" => Ok(()),
            "set_required_hearts" => {
                self.execute_set_required_hearts(gs, &effect.heart_colors, effect.target_name());
                Ok(())
            }
            "set_score" => {
                self.execute_set_score(gs, effect.value.unwrap_or(0), effect.target_name());
                Ok(())
            }
            "specify_heart_color" => {
                self.execute_specify_heart_color(
                    gs,
                    effect.choice.unwrap_or(false),
                    effect.target_name(),
                );
                Ok(())
            }
            "modify_required_hearts_success" => {
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
            "set_cost_to_use" => self.execute_set_cost_to_use(gs, effect.value.unwrap_or(0)),
            "all_blade_timing" => {
                self.execute_all_blade_timing(
                    gs,
                    effect.timing.as_deref().unwrap_or("check_required_hearts"),
                    effect.treat_as.as_deref().unwrap_or("any_heart_color"),
                );
                Ok(())
            }
            "set_card_identity_all_regions" => {
                self.execute_set_card_identity_all_regions(
                    gs,
                    effect.identities.as_ref(),
                    effect.target_name(),
                );
                Ok(())
            }
            "shuffle" => {
                self.execute_shuffle(gs, effect.target_name(), effect.source_or("deck"));
                Ok(())
            }
            "reveal_per_group" => self.execute_reveal_per_group(
                gs,
                effect.source_or("hand"),
                effect.count_or(1),
                effect.target_name(),
            ),
            "conditional_on_result" => self.execute_conditional_on_result(gs, effect),
            "conditional_on_optional" => self.execute_conditional_on_optional(gs, effect),
            "modify_cost" => {
                let mut value = effect.value.unwrap_or(0);
                if effect.per_unit.unwrap_or(false) {
                    let player = gs.resolve_target_player(&effect.target_name());
                    let zone = effect.location.as_deref().unwrap_or("hand");
                    let cards: Vec<i16> = crate::ability::util::zone_cards(player, zone).to_vec();
                    let count = cards.len() as u32;
                    let per_unit_count = effect.per_unit_count.unwrap_or(1);
                    let final_count = if effect.exclude_self.unwrap_or(false) {
                        count.saturating_sub(1)
                    } else {
                        count
                    };
                    value = (final_count / per_unit_count) * value;
                }
                self.execute_modify_cost(
                    gs,
                    effect.operation.as_deref().unwrap_or("add"),
                    value,
                    effect.target_name(),
                    effect.card_type.as_deref(),
                );
                Ok(())
            }
            "reveal_until_live_card" => {
                self.execute_reveal_until_live_card(gs, effect.target_name())
            }
            "reveal_until_chosen_card" => self.execute_reveal_until_chosen_card(gs, effect),
            _ => {
                eprintln!("Unknown effect action: '{}'", action_str);
                Ok(())
            }
        }
    }
}
