use crate::card::{AbilityEffect, PositionInfo};
use crate::effect::Effect;
use super::types::{Choice, ExecutionContext, LookAndSelectStep};
use super::resolver::AbilityResolver;
use super::util;

impl<'a> AbilityResolver<'a> {
    pub fn execute_effect(&mut self, effect: &AbilityEffect) -> Result<(), String> {

        if !self.can_activate_effect(effect) {
            return Ok(());
        }

        if let Some(ref condition) = effect.condition {
            if !self.evaluate_condition(condition) {
                return Ok(());
            }
        }

        if effect.action_by.as_deref() == Some("opponent") {
            if let Some(ref opponent_action) = effect.opponent_action {
                self.execute_effect(opponent_action)?;
            }
        }

        self.game_state.reset_replacement_effect_flags();
        let action_to_use = effect.action.clone();

        // Empty action with opponent_action means it was entirely handled by opponent
        if action_to_use.is_empty() && effect.action_by.is_some() {
            return Ok(());
        }

        let replacement_effects: Vec<crate::game_state::ReplacementEffect> = self.game_state.get_replacement_effects_for_event(&action_to_use)
            .iter().map(|r| (*r).clone()).collect();
        if !replacement_effects.is_empty() {
            for replacement in &replacement_effects {
                if replacement.is_choice_based {
                    let description = format!("Apply replacement effect for action '{}'?", action_to_use);
                    self.pending_choice = Some(Choice::SelectTarget { target: "apply_replacement".to_string(), description });
                    return Err("Pending choice required: apply replacement effect".to_string());
                } else {
                    for replacement_effect in &replacement.replacement_effects {
                        self.execute_effect(replacement_effect)?;
                    }
                    self.game_state.mark_replacement_effect_applied(replacement.card_id);
                }
            }
            return Ok(());
        }

        if let Some(ref effect_type) = effect.effect_type {
            if effect_type == "replacement" {
                let original_event = effect.replaces_event.clone();
                let is_choice_based = effect.choice_based.unwrap_or(false);
                let card_id = self.game_state.activating_card.unwrap_or(-1);
                let player_id = if self.game_state.current_turn_phase == crate::game_state::TurnPhase::FirstAttackerNormal {
                    self.game_state.player1.id.clone()
                } else {
                    self.game_state.player2.id.clone()
                };
                if let Some(event) = original_event {
                    self.game_state.add_replacement_effect(card_id, player_id, event.clone(), vec![effect.clone()], is_choice_based);
                }
                return Ok(());
            }
        }

        let effect_enum = Effect::from_ability_effect(effect);
        match effect_enum {
            Effect::Sequential { conditional, is_further, .. } => self.execute_sequential_effect(effect, conditional, is_further),
            Effect::ConditionalAlternative { .. } => self.execute_conditional_alternative(effect),
            Effect::LookAndSelect { .. } => self.execute_look_and_select(effect),
            Effect::Draw { count, target, source, destination, card_type, per_unit, per_unit_count, per_unit_type } => {
                self.execute_draw(count, &target, &source, &destination, card_type.as_deref(), per_unit, per_unit_count, per_unit_type.as_deref())
            }
            Effect::DrawUntilCount { count, target, destination } => {
                self.execute_draw_until_count(count, &target, &destination)
            }
            Effect::MoveCards { .. } => self.execute_move_cards(effect),
            Effect::GainResource { resource, count, target, duration, card_type, group_name, per_unit, per_unit_count, per_unit_type, heart_color, heart_colors, resource_icon_count } => {
                self.execute_gain_resource(&resource, count, &target, duration.as_deref(), card_type.as_deref(), group_name.as_deref(), per_unit, per_unit_count, per_unit_type.as_deref(), heart_color.as_deref(), heart_colors.as_ref(), resource_icon_count)
            }
            Effect::ChangeState { state_change, target, count, card_type, cost_limit, optional, group_name, self_cost, source, destination } => {
                self.execute_change_state(&state_change, &target, count, card_type.as_deref(), cost_limit, optional, group_name.as_deref(), self_cost, source.as_deref(), destination.as_deref())
            }
            Effect::ModifyScore { operation, value, target, duration, card_type, group_name, per_unit, per_unit_count, per_unit_type, effect_constraint } => {
                self.execute_modify_score(&operation, value, &target, duration.as_deref(), card_type.as_deref(), group_name.as_deref(), per_unit, per_unit_count, per_unit_type.as_deref(), effect_constraint.as_deref())
            }
            Effect::ModifyRequiredHearts { operation, value, heart_color, target } => {
                self.execute_modify_required_hearts(&operation, value, &heart_color, &target)
            }
            Effect::SetCost { value, target, card_type } => self.execute_set_cost(value, &target, card_type.as_deref()),
            Effect::SetBladeType { blade_type, target, duration } => self.execute_set_blade_type(blade_type.as_deref(), &target, duration.as_deref()),
            Effect::SetHeartType { heart_type, target, count } => self.execute_set_heart_type(heart_type.as_deref(), &target, count as i32),
            Effect::ActivateAbility { ability_text } => self.execute_activate_ability(&ability_text),
            Effect::InvalidateAbility => self.execute_invalidate_ability(),
            Effect::GainAbility { ability_text, target, duration } => self.execute_gain_ability(&ability_text, &target, duration.as_deref()),
            Effect::PlayBatonTouch { count, target } => self.execute_play_baton_touch(count, &target),
            Effect::Reveal { source, count, target, card_type, heart_colors } => {
                self.execute_reveal(&source, count, &target, card_type.as_deref(), heart_colors.as_ref())
            }
            Effect::Select { source, count, target, card_type, distinct, heart_colors } => {
                self.execute_select(&source, count, &target, card_type.as_deref(), distinct.as_deref(), heart_colors.as_ref())
            }
            Effect::LookAt { count, target, source } => self.execute_look_at(count, &target, &source),
            Effect::ModifyRequiredHeartsGlobal { operation, value, heart_color, target } => {
                self.execute_modify_required_hearts_global(&operation, value, &heart_color, &target)
            }
            Effect::ModifyYellCount { operation, count } => self.execute_modify_yell_count(&operation, count),
            Effect::PlaceEnergyUnderMember { count, target, position } => {
                self.execute_place_energy_under_member(count, &target, position.as_ref())
            }
            Effect::ActivationCost { operation, value, target, duration } => self.execute_activation_cost(&operation, value, &target, duration.as_deref()),
            Effect::PositionChange { position, target, target_member } => self.execute_position_change(effect, position, &target, &target_member),
            Effect::Appear { source, destination, count, target, card_type } => {
                self.execute_appear(&source, &destination, count, &target, card_type.as_deref())
            }
            Effect::Choice { choice_options, choice_type, options } => self.execute_choice(choice_options.as_ref(), choice_type.as_deref(), options.as_ref()),
            Effect::PayEnergy { count, target } => self.execute_pay_energy(count, &target),
            Effect::SetCardIdentity { identities } => self.execute_set_card_identity(&identities),
            Effect::RepeatProcedure { repeat_limit, .. } => self.execute_repeat_procedure(effect, repeat_limit),
            Effect::DiscardUntilCount { target_count, target } => self.execute_discard_until_count(target_count, &target),
            Effect::Restriction { restriction_type, restricted_destination } => self.execute_restriction(restriction_type.as_deref(), restricted_destination.as_deref()),
            Effect::ReYell { lose_blade_hearts, target } => self.execute_re_yell(lose_blade_hearts, &target),
            Effect::ActivationRestriction { target } => self.execute_activation_restriction(&target),
            Effect::ChooseRequiredHearts => self.execute_choose_required_hearts(),
            Effect::ModifyLimit { operation, count } => self.execute_modify_limit(&operation, count),
            Effect::SetBladeCount { value, target } => self.execute_set_blade_count(value, &target),
            Effect::DoNothing => Ok(()),
            Effect::SetRequiredHearts { count, heart_color, target } => self.execute_set_required_hearts(count, &heart_color, &target),
            Effect::SetScore { value, target } => self.execute_set_score(value, &target),
            Effect::SpecifyHeartColor { choice, target } => self.execute_specify_heart_color(choice, &target),
            Effect::ModifyRequiredHeartsSuccess { operation, value, target, card_type } => {
                self.execute_modify_required_hearts_success(&operation, value, &target, card_type.as_deref())
            }
            Effect::SetCostToUse { value } => self.execute_set_cost_to_use(value),
            Effect::AllBladeTiming { timing, treat_as } => self.execute_all_blade_timing(&timing, &treat_as),
            Effect::SetCardIdentityAllRegions { identities, target } => self.execute_set_card_identity_all_regions(identities.as_ref(), &target),
            Effect::Shuffle { target, source } => self.execute_shuffle(&target, &source),
            Effect::RevealPerGroup { source, count, target } => self.execute_reveal_per_group(&source, count, &target),
            Effect::ConditionalOnResult { .. } => self.execute_conditional_on_result(effect),
            Effect::ConditionalOnOptional { .. } => self.execute_conditional_on_optional(effect),
            Effect::ModifyCost { operation, value, target, card_type } => {
                self.execute_modify_cost(&operation, value, &target, card_type.as_deref())
            }
        }
    }

    // ===== RECURSIVE COMPOUND EFFECTS (need &AbilityEffect for sub-action access) =====

    fn execute_sequential_effect(&mut self, effect: &AbilityEffect, conditional: bool, is_further: bool) -> Result<(), String> {
        let cond_met = if conditional {
            effect.condition.as_ref().map_or(true, |c| self.evaluate_condition(c))
        } else { true };
        if !cond_met { return Ok(()); }

        if is_further { eprintln!("Further conditional effect (さらに) - executing additional actions"); }

        if let Some(ref actions) = effect.actions {
            let has_repeat = actions.last().map_or(false, |a| a.action == "repeat_procedure");
            let repeat_max = if has_repeat {
                actions.last().and_then(|a| a.repeat_limit).unwrap_or(1)
            } else {
                1
            };
            let repeat_actions: &[AbilityEffect] = if has_repeat {
                &actions[..actions.len() - 1]
            } else {
                actions.as_slice()
            };

            for _repeat in 0..repeat_max {
                for (i, action) in repeat_actions.iter().enumerate() {
                    let mut action_to_execute = action.clone();
                    if action_to_execute.per_unit.is_none() && effect.per_unit.is_some() {
                        action_to_execute.per_unit = effect.per_unit;
                    }
                    if action_to_execute.per_unit_count.is_none() && effect.per_unit_count.is_some() {
                        action_to_execute.per_unit_count = effect.per_unit_count;
                    }
                    if action_to_execute.per_unit_type.is_none() && effect.per_unit_type.is_some() {
                        action_to_execute.per_unit_type = effect.per_unit_type.clone();
                    }

                    match self.execute_effect(&action_to_execute) {
                        Ok(_) => {
                            if self.pending_choice.is_some() {
                                let remaining_actions: Vec<AbilityEffect> = repeat_actions[i + 1..].to_vec();
                                if !remaining_actions.is_empty() {
                                    self.game_state.pending_sequential_actions = Some(remaining_actions);
                                }
                                return Ok(());
                            }
                        },
                        Err(e) if e.contains("Pending choice required") => {
                            let remaining_actions: Vec<AbilityEffect> = repeat_actions[i + 1..].to_vec();
                            if !remaining_actions.is_empty() {
                                self.game_state.pending_sequential_actions = Some(remaining_actions);
                            }
                            return Ok(());
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
        }
        Ok(())
    }

    fn execute_conditional_alternative(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let has_primary = effect.primary_effect.is_some();
        let has_alternative = effect.alternative_effect.is_some();

        if has_primary && has_alternative {
            let primary_text = effect.primary_effect.as_ref().map(|e| e.text.as_str()).unwrap_or("Primary effect");
            let alternative_text = effect.alternative_effect.as_ref().map(|e| e.text.as_str()).unwrap_or("Alternative effect");
            let description = format!("Choose effect:\nPrimary: {}\nAlternative: {}", primary_text, alternative_text);
            self.pending_choice = Some(Choice::SelectTarget { target: "primary|alternative".to_string(), description });
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return Ok(());
        }

        if let Some(ref alt_condition) = effect.alternative_condition {
            if self.evaluate_condition(alt_condition) {
                if let Some(ref alt_effect) = effect.alternative_effect {
                    return self.execute_effect(alt_effect);
                }
            }
        }

        if let Some(ref primary_effect) = effect.primary_effect {
            self.execute_effect(primary_effect)
        } else { Ok(()) }
    }

    fn execute_look_and_select(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        self.current_effect = Some(effect.clone());

        if let Some(ref look_action) = effect.look_action {
            self.execute_effect(look_action)?;
        }

        if let Some(ref select_action) = effect.select_action {
            let placement_order = select_action.placement_order.as_deref();
            let count = select_action.count.unwrap_or(1);
            let optional = select_action.optional.unwrap_or(false);
            let any_number = select_action.any_number.unwrap_or(false);

            let card_db = &self.game_state.card_database;
            let card_type_filter = select_action.card_type.as_deref();
            let heart_colors_filter = select_action.heart_colors.as_ref();
            let has_filter = card_type_filter.is_some()
                || heart_colors_filter.map_or(false, |c| !c.is_empty());
            if has_filter {
                self.looked_at_cards = self.looked_at_cards.iter().filter(|&&card_id| {
                    super::util::card_matches_type(card_db, card_id, card_type_filter)
                        && super::util::card_matches_heart_colors(card_db, card_id, heart_colors_filter)
                }).copied().collect();
            }

            let available_count = self.looked_at_cards.len();
            let max_select = if any_number { available_count } else { std::cmp::min(count as usize, available_count) };

            let description = if available_count == 0 {
                "No eligible cards found among looked-at cards".to_string()
            } else if any_number {
                format!("Select any number of cards from the {} looked-at cards (or skip) (placement_order: {})",
                    available_count, placement_order.unwrap_or("default"))
            } else if optional {
                format!("Select up to {} card(s) from the {} looked-at cards (or skip) (placement_order: {})",
                    max_select, available_count, placement_order.unwrap_or("default"))
            } else {
                format!("Select {} card(s) from the {} looked-at cards (placement_order: {})",
                    max_select, available_count, placement_order.unwrap_or("default"))
            };

            let choice = Choice::SelectCard {
                zone: "looked_at".to_string(), card_type: select_action.card_type.clone(), count: max_select,
                description, allow_skip: optional || any_number || available_count == 0,
            };
            self.pending_choice = Some(choice);
            self.execution_context = ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Select { count: max_select },
            };
            return Ok(());
        }

        self.current_effect = None;
        Ok(())
    }

    fn execute_repeat_procedure(&mut self, effect: &AbilityEffect, repeat_limit: u32) -> Result<(), String> {
        let repeat_limit = repeat_limit as usize;
        if let Some(ref actions) = effect.actions {
            for _ in 0..repeat_limit {
                for action in actions {
                    self.execute_effect(action)?;
                }
            }
        }
        Ok(())
    }

    fn execute_conditional_on_result(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let primary_action = effect.primary_effect.as_ref();
        let result_condition = effect.result_condition.as_ref();
        let followup_action = effect.followup_action.as_ref();

        if let Some(ref primary) = primary_action {
            if let Err(e) = self.execute_effect(primary) {
                eprintln!("Primary action failed in conditional_on_result: {}", e);
                return Err(e);
            }
        }

        let condition_met = result_condition.map(|c| self.evaluate_condition(c)).unwrap_or(true);

        if condition_met {
            if let Some(ref followup) = followup_action {
                self.execute_effect(followup)?;
            }
        } else {
            eprintln!("Result condition not met, skipping followup action");
        }
        Ok(())
    }

    fn execute_conditional_on_optional(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let optional_action = effect.optional_action.as_ref();
        let conditional_action = effect.conditional_action.as_ref();

        if optional_action.is_some() && conditional_action.is_some() {
            let desc = optional_action.as_ref().map(|a| a.text.as_str()).unwrap_or("Perform optional action");
            self.pending_choice = Some(Choice::SelectTarget {
                target: "conditional_optional".to_string(),
                description: format!("{}?", desc),
                choice_maker: None,
            });
            return Ok(());
        }

        if let Some(ref optional) = optional_action { self.execute_effect(optional)?; }
        if let Some(ref conditional) = conditional_action { self.execute_effect(conditional)?; }
        Ok(())
    }

    // ===== LEAF EFFECTS (all data from enum params, no &AbilityEffect) =====

    fn execute_draw(&mut self, count: u32, target: &str, source: &str, destination: &str, card_type: Option<&str>, per_unit: bool, per_unit_count: u32, per_unit_type: Option<&str>) -> Result<(), String> {
        let card_db = self.game_state.card_database.clone();

        if target == "both" {
            let card_db1 = card_db.clone();
            let card_db2 = card_db.clone();
            { let p1 = &mut self.game_state.player1; Self::draw_cards_for_player(p1, count, source, destination, card_type, &card_db1)?; }
            { let p2 = &mut self.game_state.player2; Self::draw_cards_for_player(p2, count, source, destination, card_type, &card_db2)?; }
            return Ok(());
        }

        let player = self.game_state.resolve_target_player_mut(target);

        let final_count = if per_unit {
            let multiplier = match per_unit_type {
                Some("member") | Some("人") => player.stage.stage.iter().filter(|&&c| c != -1).count() as u32,
                Some("energy") => player.energy_zone.cards.len() as u32,
                Some("hand") => player.hand.cards.len() as u32,
                _ => 1,
            };
            count * multiplier * per_unit_count
        } else { count };

        match source {
            "deck" | "deck_top" => { Self::draw_cards_for_player(player, final_count, source, destination, card_type, &card_db)?; }
            "discard" => {
                for _ in 0..final_count {
                    if let Some(card) = player.waitroom.cards.pop() {
                        player.hand.add_card(card);
                    } else { break; }
                }
            }
            _ => { eprintln!("Draw from source '{}' not yet implemented", source); }
        }
        Ok(())
    }

    fn draw_cards_for_player(player: &mut crate::player::Player, count: u32, _source: &str, destination: &str, card_type_filter: Option<&str>, card_db: &crate::card::CardDatabase) -> Result<(), String> {
        let mut drawn = 0;
        while drawn < count {
            if let Some(card) = player.main_deck.draw() {
                let matches_type = util::card_matches_type(card_db, card, card_type_filter);
                if matches_type {
                    match destination {
                        "hand" => player.hand.add_card(card),
                        "discard" => player.waitroom.add_card(card),
                        "deck_top" => player.main_deck.cards.insert(0, card),
                        "deck_bottom" | "deck" => player.main_deck.cards.push(card),
                        "stage" => {
                            if player.stage.stage[1] == -1 { player.stage.stage[1] = card; }
                            else if player.stage.stage[0] == -1 { player.stage.stage[0] = card; }
                            else if player.stage.stage[2] == -1 { player.stage.stage[2] = card; }
                            else { player.hand.add_card(card); }
                        },
                        _ => { player.hand.add_card(card); }
                    }
                    drawn += 1;
                } else { player.main_deck.cards.push(card); }
            } else { break; }
        }
        Ok(())
    }

    fn execute_draw_until_count(&mut self, target_count: u32, target: &str, destination: &str) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        let current_count = match destination {
            "hand" => player.hand.len(),
            _ => { return Ok(()); }
        };
        let to_draw = (target_count as usize).saturating_sub(current_count);
        self.execute_draw(to_draw as u32, target, "deck", destination, None, false, 1, None)
    }

    fn execute_gain_resource(
        &mut self, resource: &str, count: u32, target: &str, duration: Option<&str>,
        card_type: Option<&str>, group_name: Option<&str>, per_unit: bool, per_unit_count: u32,
        per_unit_type: Option<&str>, heart_color: Option<&str>, heart_colors: Option<&Vec<String>>,
        _resource_icon_count: Option<u32>,
    ) -> Result<(), String> {
        let resource = resource.to_string();
        let target = target.to_string();
        let duration = duration.map(|s| s.to_string());
        let card_type_filter = card_type.map(|s| s.to_string());
        let group_filter = group_name.map(|s| s.to_string());
        let per_unit_count_val = per_unit_count;
        let per_unit_type_str = per_unit_type.map(|s| s.to_string());
        let is_temporary = duration.is_some() && duration.as_deref() != Some("permanent");
        let activating_card_id = self.game_state.activating_card;
        let card_db = self.game_state.card_database.clone();

        // If heart_colors is present and resource is heart, this is a choose-and-replace operation
        if (resource == "heart" || resource == "ハート") && heart_colors.is_some() {
            if let Some(colors) = heart_colors {
                let mut unique_colors: Vec<String> = Vec::new();
                for c in colors {
                    if !unique_colors.contains(c) {
                        unique_colors.push(c.clone());
                    }
                }
                self.pending_choice = Some(Choice::SelectHeartColor {
                    count: count as usize,
                    options: unique_colors,
                    description: "Choose a heart color to replace this member's original heart".to_string(),
                });
                return Ok(());
            }
        }

        let (blade_targets, heart_targets, heart_color_str, final_count) = {
            let player = self.game_state.resolve_target_player_mut(&target);

            let matches_card_type = |card_id: i16| -> bool {
                util::card_matches_type(&card_db, card_id, card_type_filter.as_deref())
            };

            let matches_group = |card_id: i16| -> bool {
                util::card_matches_group_str(&card_db, card_id, group_filter.as_deref())
            };

            let final_count = if per_unit {
                let matching_count = match per_unit_type_str.as_deref() {
                    Some("stage") => { player.stage.stage.iter().filter(|&&card_id| card_id != -1).filter(|&&card_id| matches_card_type(card_id) && matches_group(card_id)).count() as u32 }
                    Some("hand") => { player.hand.cards.iter().filter(|&&card_id| matches_card_type(card_id) && matches_group(card_id)).count() as u32 }
                    _ => { player.stage.stage.iter().filter(|&&card_id| card_id != -1).filter(|&&card_id| matches_card_type(card_id) && matches_group(card_id)).count() as u32 }
                };
                matching_count * per_unit_count_val
            } else { count };

            let has_blade_filter = card_type_filter.is_some() || group_filter.is_some();
            let blade_targets: Vec<i16> = if has_blade_filter {
                vec![player.stage.stage[0], player.stage.stage[1], player.stage.stage[2]]
                    .into_iter().filter(|&card_id| card_id != -1)
                    .filter(|&card_id| matches_card_type(card_id) && matches_group(card_id))
                    .collect()
            } else {
                vec![]
            };

            let heart_color_inner = heart_color.map(|s| s.to_string());
            let heart_targets: Vec<i16> = if resource == "heart" || resource == "ハート" {
                (0..3).filter_map(|i| {
                    let card_id = player.stage.stage[i];
                    if card_id != -1 && matches_card_type(card_id) && matches_group(card_id) { Some(card_id) } else { None }
                }).collect()
            } else { vec![] };

            (blade_targets, heart_targets, heart_color_inner, final_count)
        };

        let mut effect_data: Option<serde_json::Value> = None;

        if resource == "blade" || resource == "ブレード" {
            let blades_to_add = final_count as i32;
            if blade_targets.is_empty() {
                if let Some(card_id) = activating_card_id {
                    self.game_state.add_blade_modifier(card_id, blades_to_add);
                    if is_temporary {
                        let mut data = serde_json::Map::new();
                        data.insert("card_id".to_string(), serde_json::Value::Number(card_id.into()));
                        data.insert("amount".to_string(), serde_json::Value::Number(blades_to_add.into()));
                        effect_data = Some(serde_json::Value::Object(data));
                    }
                }
            } else {
                for &card_id in &blade_targets {
                    self.game_state.add_blade_modifier(card_id, blades_to_add);
                }
            }
        }

        if resource == "heart" || resource == "ハート" {
            let color = crate::zones::parse_heart_color(heart_color_str.as_deref().unwrap_or("heart00"));
            for card_id in heart_targets {
                self.game_state.add_heart_modifier(card_id, color, final_count as i32);
            }
        }

        if is_temporary {
            self.game_state.temporary_effects.push(crate::game_state::TemporaryEffect {
                effect_type: format!("gain_{}", resource),
                duration: match duration.as_deref() { Some("this_turn") => crate::game_state::Duration::ThisTurn, Some("live_end") => crate::game_state::Duration::LiveEnd, _ => crate::game_state::Duration::ThisLive },
                created_turn: self.game_state.turn_number,
                created_phase: self.game_state.current_phase.clone(),
                target_player_id: target.clone(),
                description: format!("Gain {} {}", final_count, resource),
                creation_order: 0, effect_data,
            });
        }

        Ok(())
    }

    fn execute_change_state(
        &mut self, state_change: &str, target: &str, count: u32, card_type: Option<&str>,
        cost_limit: Option<u32>, optional: bool, group_name: Option<&str>, self_cost: bool,
        source: Option<&str>, destination: Option<&str>,
    ) -> Result<(), String> {
        let state_change = state_change.to_string();
        let target = target.to_string();
        let card_type_filter = card_type.map(|s| s.to_string());
        let group_filter = group_name.map(|s| s.to_string());

        if optional {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "pay_optional_cost:skip_optional_cost".to_string(),
                description: format!("Change state to {} (pay optional cost)?", state_change),
            });
            if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                entry.choice_card_no = Some("change_state".to_string());
            }
            return Ok(());
        }

        // Draw from energy deck and place in energy zone with state (e.g. wait)
        if source == Some("deck") && destination == Some("energy_zone") {
            let player = self.game_state.resolve_target_player_mut(&target);
            for _ in 0..count {
                if let Some(energy_id) = player.energy_deck.draw() {
                    player.energy_zone.cards.push(energy_id);
                    if state_change == "wait" {
                        // Card is placed in wait (not counted as active)
                    } else if state_change == "active" {
                        player.energy_zone.active_energy_count += 1;
                    }
                }
            }
            return Ok(());
        }

        // Member card state change — operate on stage
        let is_member_op = card_type_filter.as_deref() == Some("member_card") || self_cost;

        if is_member_op {
            let card_db = self.game_state.card_database.clone();
            let player = self.game_state.resolve_target_player_mut(&target);

            let mut candidates: Vec<(usize, i16)> = Vec::new();
            for (i, slot_id) in player.stage.stage.iter().enumerate() {
                if *slot_id == -1 { continue; }
                if util::card_matches_type(&card_db, *slot_id, card_type_filter.as_deref())
                    && util::card_matches_group_str(&card_db, *slot_id, group_filter.as_deref())
                    && util::card_matches_cost_limit(&card_db, *slot_id, cost_limit)
                {
                    candidates.push((i, *slot_id));
                }
            }

            if candidates.is_empty() {
                return Err("No matching members on stage to change state".to_string());
            }

            if candidates.len() > count as usize {
                self.pending_choice = Some(Choice::SelectCard {
                    zone: "stage".to_string(),
                    card_type: card_type_filter.clone(),
                    count: count as usize,
                    description: format!("Select {} member(s) to change state", count),
                    allow_skip: false,
                });
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                return Ok(());
            }

            for (_, card_id) in candidates.iter().take(count as usize) {
                self.game_state.add_orientation_modifier(*card_id, &state_change);
            }
            return Ok(());
        }

        // Energy card state change (original behavior)
        let card_db = self.game_state.card_database.clone();
        let (wait_cards, active_cards, deactivate_count) = {
            let player = self.game_state.resolve_target_player_mut(&target);

            let matches_card_type = |card_id: i16| -> bool {
                util::card_matches_type(&card_db, card_id, card_type_filter.as_deref())
            };

            let matches_group = |card_id: i16| -> bool {
                util::card_matches_group_str(&card_db, card_id, group_filter.as_deref())
            };

            let matches_cost_limit = |card_id: i16| -> bool {
                util::card_matches_cost_limit(&card_db, card_id, cost_limit)
            };

            let mut valid_indices: Vec<usize> = Vec::new();
            for i in 0..player.energy_zone.cards.len() {
                if let Some(&card_id) = player.energy_zone.cards.get(i) {
                    if matches_card_type(card_id) && matches_group(card_id) && matches_cost_limit(card_id) {
                        valid_indices.push(i);
                    }
                }
            }

            if valid_indices.len() < count as usize {
                return Err(format!("Not enough energy cards to deactivate: need {}, have {}", count, valid_indices.len()));
            }

            if valid_indices.len() > count as usize {
                self.pending_choice = Some(Choice::SelectCard {
                    zone: "energy_zone".to_string(), card_type: card_type_filter.clone(),
                    count: count as usize,
                    description: format!("Select {} energy card(s) to deactivate (set to wait)", count),
                    allow_skip: false,
                });
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                return Ok(());
            }

            let wait_cards: Vec<i16> = valid_indices.iter().take(count as usize).filter_map(|i| {
                if *i < player.energy_zone.cards.len() { Some(player.energy_zone.cards[*i]) } else { None }
            }).collect();

            let mut active_count = 0u32;
            let mut active_cards: Vec<i16> = Vec::new();
            for i in 0..player.energy_zone.cards.len() {
                if active_count >= count { break; }
                if let Some(&card_id) = player.energy_zone.cards.get(i) {
                    if matches_card_type(card_id) && matches_group(card_id) && matches_cost_limit(card_id) {
                        active_cards.push(card_id);
                        active_count += 1;
                    }
                }
            }

            (wait_cards, active_cards, count)
        };

        match state_change.as_str() {
            "wait" | "ウェイト" => {
                for card_id in &wait_cards {
                    self.game_state.add_orientation_modifier(*card_id, "wait");
                }
                for _ in 0..deactivate_count {
                    let player = self.game_state.resolve_target_player_mut(target.as_str());
                    player.energy_zone.active_energy_count = player.energy_zone.active_energy_count.saturating_sub(1);
                }
            }
            "active" | "アクティブ" => {
                for card_id in &active_cards {
                    self.game_state.add_orientation_modifier(*card_id, "active");
                }
                let player = self.game_state.resolve_target_player_mut(target.as_str());
                player.energy_zone.active_energy_count += active_cards.len();
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_modify_score(
        &mut self, operation: &str, value: u32, target: &str, duration: Option<&str>,
        card_type: Option<&str>, group_name: Option<&str>, per_unit: bool, per_unit_count: u32,
        per_unit_type: Option<&str>, effect_constraint: Option<&str>,
    ) -> Result<(), String> {
        let operation = operation.to_string();
        let target = target.to_string();
        let duration = duration.map(|s| s.to_string());
        let card_type_filter = card_type.map(|s| s.to_string());
        let group_filter = group_name.map(|s| s.to_string());
        let per_unit_count_val = per_unit_count;
        let per_unit_type_str = per_unit_type.map(|s| s.to_string());
        let effect_constraint = effect_constraint.map(|s| s.to_string());
        let card_db = self.game_state.card_database.clone();

        let (live_card_ids, final_value) = {
            let player = self.game_state.resolve_target_player_mut(&target);

            let matches_card_type = |card_id: i16| -> bool {
                util::card_matches_type(&card_db, card_id, card_type_filter.as_deref())
            };

            let matches_group = |card_id: i16| -> bool {
                util::card_matches_group_str(&card_db, card_id, group_filter.as_deref())
            };

            let final_value = if per_unit {
                let matching_count = match per_unit_type_str.as_deref() {
                    Some("hand") => player.hand.cards.iter().filter(|&&card_id| matches_card_type(card_id) && matches_group(card_id)).count() as u32,
                    Some("stage") => player.stage.stage.iter().filter(|&&card_id| card_id != -1).filter(|&&card_id| matches_card_type(card_id) && matches_group(card_id)).count() as u32,
                    _ => 1,
                };
                value * matching_count * per_unit_count_val
            } else { value };

            let live_card_ids: Vec<(i16, i32)> = player.live_card_zone.cards.iter()
                .filter(|&&card_id| matches_card_type(card_id) && matches_group(card_id))
                .map(|&card_id| {
                    let delta = match operation.as_str() {
                        "add" => final_value as i32,
                        "remove" => -(final_value as i32),
                        "set" => final_value as i32,
                        _ => 0i32,
                    };
                    (card_id, delta)
                }).collect();

            (live_card_ids, final_value)
        };

        let mut count_applied = 0u32;
        for (card_id, delta) in &live_card_ids {
            if let Some(constraint) = &effect_constraint {
                let current_mod = self.game_state.get_score_modifier(*card_id);
                match constraint.as_str() {
                    "min:0" => { if current_mod + delta < 0 { continue; } }
                    _ => {}
                }
            }
            if operation == "set" { self.game_state.set_score_modifier(*card_id, *delta); }
            else { self.game_state.add_score_modifier(*card_id, *delta); }
            count_applied += 1;
        }

        if let Some(duration_str) = &duration {
            if duration_str != "permanent" {
                let duration_enum = match duration_str.as_str() {
                    "this_turn" => crate::game_state::Duration::ThisTurn,
                    "this_live" => crate::game_state::Duration::ThisLive,
                    "live_end" => crate::game_state::Duration::LiveEnd,
                    _ => crate::game_state::Duration::ThisLive,
                };
                self.game_state.temporary_effects.push(crate::game_state::TemporaryEffect {
                    effect_type: format!("modify_score_{}", operation),
                    duration: duration_enum, created_turn: self.game_state.turn_number,
                    created_phase: self.game_state.current_phase.clone(),
                    target_player_id: target.clone(),
                    description: format!("Modify score by {} {} (applied to {} cards)", operation, final_value, count_applied),
                    creation_order: 0, effect_data: None,
                });
            }
        }
        Ok(())
    }

    fn execute_modify_required_hearts(&mut self, operation: &str, value: u32, heart_color: &str, target: &str) -> Result<(), String> {
        let color = crate::zones::parse_heart_color(heart_color);
        let card_ids: Vec<i16> = {
            let player = self.game_state.resolve_target_player_mut(target);
            player.live_card_zone.cards.to_vec()
        };
        for card_id in card_ids {
            match operation {
                "decrease" => { self.game_state.add_need_heart_modifier(card_id, color, -(value as i32)); }
                "increase" => { self.game_state.add_need_heart_modifier(card_id, color, value as i32); }
                "set" => { self.game_state.set_need_heart_modifier(card_id, color, value as i32); }
                _ => return Err(format!("Unknown operation: {}", operation)),
            }
        }
        Ok(())
    }

    fn execute_set_cost(&mut self, value: u32, target: &str, card_type: Option<&str>) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        let card_ids: Vec<i16> = if let Some("live_card") = card_type {
            player.live_card_zone.cards.iter().copied().collect()
        } else if let Some("member_card") = card_type {
            player.stage.stage.iter().filter(|&&id| id != -1).copied().collect()
        } else { player.hand.cards.iter().copied().collect() };
        for card_id in card_ids { self.game_state.set_cost_modifier(card_id, value as i32); }
        Ok(())
    }

    fn execute_set_blade_type(&mut self, blade_type: Option<&str>, target: &str, duration: Option<&str>) -> Result<(), String> {
        let current_turn = self.game_state.turn_number;
        let current_phase = self.game_state.current_phase.clone();
        let effect_duration = duration.map(|s| s.to_string());
        let card_db = self.game_state.card_database.clone();
        let stage_card_ids: Vec<(i16, String)> = {
            let player = self.game_state.resolve_target_player(target);
            (0..3).filter_map(|i| {
                let id = player.stage.stage[i];
                if id == -1 { None } else { Some((id, player.id.clone())) }
            }).collect()
        };
        for (card_id, pid) in stage_card_ids {
            let temp_effect = crate::game_state::TemporaryEffect {
                effect_type: format!("set_blade_type:{}", blade_type.unwrap_or("")),
                duration: effect_duration.as_deref().map(|d| match d {
                    "live_end" => crate::game_state::Duration::LiveEnd,
                    "this_turn" => crate::game_state::Duration::ThisTurn,
                    "this_live" => crate::game_state::Duration::ThisLive,
                    "permanent" => crate::game_state::Duration::Permanent,
                    "as_long_as" => crate::game_state::Duration::ThisLive,
                    _ => crate::game_state::Duration::ThisLive,
                }).unwrap_or(crate::game_state::Duration::ThisLive),
                created_turn: current_turn, created_phase: current_phase.clone(),
                target_player_id: pid,
                description: format!("Set blade type to {} for {}", blade_type.unwrap_or(""), card_db.get_card(card_id).map(|c| c.name.as_str()).unwrap_or("unknown")),
                creation_order: 0, effect_data: None,
            };
            self.game_state.temporary_effects.push(temp_effect);
        }
        Ok(())
    }

    fn execute_set_heart_type(&mut self, heart_type: Option<&str>, target: &str, count: i32) -> Result<(), String> {
        let heart_type = heart_type.unwrap_or("heart00");
        let player = self.game_state.resolve_target_player_mut(target);
        let mut card_ids_to_modify: Vec<i16> = Vec::new();
        for index in 0..3 {
            let card_id = player.stage.stage[index];
            if card_id != -1 { card_ids_to_modify.push(card_id); }
        }
        let color = crate::zones::parse_heart_color(heart_type);
        for card_id in card_ids_to_modify { self.game_state.add_heart_modifier(card_id, color, count); }
        Ok(())
    }

    fn execute_activate_ability(&mut self, ability_text: &str) -> Result<(), String> {
        if let Some(card_id) = self.game_state.activating_card {
            self.game_state.gained_abilities.entry(card_id).or_default().push(ability_text.to_string());
        }
        Ok(())
    }

    fn execute_invalidate_ability(&mut self) -> Result<(), String> {
        if let Some(card_id) = self.game_state.activating_card {
            self.game_state.negated_abilities.insert(card_id);
        }
        Ok(())
    }

    fn execute_gain_ability(&mut self, ability_text: &str, target: &str, duration: Option<&str>) -> Result<(), String> {
        if let Some(card_id) = self.game_state.activating_card {
            self.game_state.gained_abilities.entry(card_id).or_default().push(ability_text.to_string());
        }
        let temp_effect = crate::game_state::TemporaryEffect {
            effect_type: format!("gain_ability:{}", ability_text),
            duration: match duration { Some("this_turn") => crate::game_state::Duration::ThisTurn, Some("live_end") => crate::game_state::Duration::LiveEnd, _ => crate::game_state::Duration::ThisLive },
            created_turn: self.game_state.turn_number, created_phase: self.game_state.current_phase.clone(),
            target_player_id: target.to_string(),
            description: format!("Gained ability: {}", ability_text),
            creation_order: 0, effect_data: None,
        };
        self.game_state.temporary_effects.push(temp_effect);
        Ok(())
    }

    fn execute_play_baton_touch(&mut self, count: u32, target: &str) -> Result<(), String> {
        eprintln!("play_baton_touch: count={}, target={}", count, target);
        self.game_state.prohibition_effects.push(format!("baton_touch_allowed:{}", count));
        Ok(())
    }

    pub fn execute_reveal(&mut self, source: &str, count: u32, target: &str, card_type: Option<&str>, heart_colors: Option<&Vec<String>>) -> Result<(), String> {
        let card_db = self.game_state.card_database.clone();
        let card_ids: Vec<i16> = {
            let player = self.game_state.resolve_target_player_mut(target);
            match source {
                "hand" => player.hand.cards.iter().copied().collect(),
                "deck" => player.main_deck.cards.iter().take(count as usize).copied().collect(),
                "looked_at" => self.looked_at_cards.iter().filter(|&&card_id| {
                    super::util::card_matches_type(&card_db, card_id, card_type)
                        && super::util::card_matches_heart_colors(&card_db, card_id, heart_colors)
                }).copied().collect(),
                _ => vec![],
            }
        };

        for card_id in &card_ids { self.game_state.revealed_cards.insert(*card_id); }
        Ok(())
    }

    fn execute_select(&mut self, source: &str, count: u32, target: &str, card_type: Option<&str>, distinct: Option<&str>, heart_colors: Option<&Vec<String>>) -> Result<(), String> {
        let target = target.to_string();
        let card_db = self.game_state.card_database.clone();
        let player = self.game_state.resolve_target_player_mut(&target);

        let card_ids: Vec<i16> = match source {
            "hand" => player.hand.cards.iter().copied().collect(),
            "deck" => player.main_deck.cards.iter().take(count as usize).copied().collect(),
            "discard" => player.waitroom.cards.iter().copied().collect(),
            "looked_at" => self.looked_at_cards.clone(),
            _ => vec![],
        };

        let filtered: Vec<i16> = card_ids.iter().filter(|&&card_id| {
            super::util::card_matches_type(&card_db, card_id, card_type)
                && super::util::card_matches_heart_colors(&card_db, card_id, heart_colors)
        }).copied().collect();

        if distinct == Some("true") || distinct == Some("distinct") {
            let mut names = std::collections::HashSet::new();
            let unique: Vec<i16> = filtered.into_iter().filter(|&card_id| {
                card_db.get_card(card_id)
                    .map(|c| names.insert(c.name.clone()))
                    .unwrap_or(false)
            }).collect();
            self.looked_at_cards = unique;
        } else { self.looked_at_cards = filtered; }

        self.pending_choice = Some(Choice::SelectCard {
            zone: source.to_string(), card_type: card_type.map(|s| s.to_string()),
            count: count as usize,
            description: format!("Select {} card(s) from {}", count, source),
            allow_skip: false,
        });
        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
        Ok(())
    }

    fn execute_look_at(&mut self, count: u32, target: &str, source: &str) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);

        let cards = match source {
            "deck" | "deck_top" => player.main_deck.draw_multiple(count as usize),
            "hand" => player.hand.cards.iter().take(count as usize).copied().collect(),
            "discard" => player.waitroom.cards.iter().take(count as usize).copied().collect(),
            "stage" => player.stage.stage.iter().filter(|&&id| id != -1).take(count as usize).copied().collect(),
            "energy_zone" => player.energy_zone.cards.iter().take(count as usize).copied().collect(),
            _ => vec![],
        };

        self.looked_at_cards = cards;
        Ok(())
    }

    fn execute_modify_required_hearts_global(&mut self, operation: &str, value: u32, heart_color: &str, target: &str) -> Result<(), String> {
        let color = crate::zones::parse_heart_color(heart_color);
        let card_ids: Vec<i16> = {
            let player = self.game_state.resolve_target_player_mut(target);
            player.live_card_zone.cards.to_vec()
        };
        for card_id in card_ids {
            let modifier_value = match operation { "increase" => value as i32, "decrease" => -(value as i32), _ => return Err(format!("Unknown operation: {}", operation)) };
            self.game_state.add_need_heart_modifier(card_id, color, modifier_value);
        }
        Ok(())
    }

    fn execute_modify_yell_count(&mut self, operation: &str, count: u32) -> Result<(), String> {
        match operation {
            "add" => { self.game_state.cheer_checks_required += count; }
            "subtract" => { self.game_state.cheer_checks_required = self.game_state.cheer_checks_required.saturating_sub(count); }
            "set" => { self.game_state.cheer_checks_required = count; }
            _ => return Err(format!("Unknown operation: {}", operation)),
        }
        Ok(())
    }

    pub fn execute_place_energy_under_member(&mut self, count: u32, target: &str, position: Option<&PositionInfo>) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        let mut energy_cards = Vec::new();
        for _ in 0..count {
            if let Some(energy_card) = player.energy_zone.cards.pop() { energy_cards.push(energy_card); }
            else { break; }
        }
        let target_index = match position.and_then(|p| p.get_position()) {
            Some("center") | Some("中央") => 1,
            Some("left") | Some("左側") => 0,
            Some("right") | Some("右側") => 2,
            None => {
                if player.stage.stage[1] != -1 { 1 }
                else if player.stage.stage[0] != -1 { 0 }
                else if player.stage.stage[2] != -1 { 2 }
                else { for card in energy_cards { player.energy_zone.cards.push(card); } return Ok(()); }
            }
            _ => 1,
        };
        if player.stage.stage[target_index] == -1 {
            for card in energy_cards { player.energy_zone.cards.push(card); }
            return Ok(());
        }
        let member_card_id = player.stage.stage[target_index];
        for _ in energy_cards { self.game_state.add_blade_modifier(member_card_id, 1); }
        Ok(())
    }

    fn execute_activation_cost(&mut self, operation: &str, value: u32, target: &str, duration: Option<&str>) -> Result<(), String> {
        let prohibition_text = format!("activation_cost_{}_{}", operation, value);
        match target {
            "self" | "opponent" => { self.game_state.prohibition_effects.push(prohibition_text); }
            _ => {}
        }
        if let Some(duration_str) = duration {
            if duration_str != "permanent" {
                let duration_enum = match duration_str {
                    "live_end" => crate::game_state::Duration::LiveEnd,
                    "this_turn" => crate::game_state::Duration::ThisTurn,
                    "this_live" => crate::game_state::Duration::ThisLive,
                    _ => crate::game_state::Duration::ThisLive,
                };
                self.game_state.temporary_effects.push(crate::game_state::TemporaryEffect {
                    effect_type: format!("activation_cost_{}_{}", operation, value),
                    duration: duration_enum, created_turn: self.game_state.turn_number,
                    created_phase: self.game_state.current_phase.clone(), target_player_id: target.to_string(),
                    description: format!("Modify activation cost by {} {}", operation, value),
                    creation_order: 0, effect_data: None,
                });
            }
        }
        Ok(())
    }

    fn execute_position_change(&mut self, effect: &AbilityEffect, position: Option<PositionInfo>, target: &str, target_member: &str) -> Result<(), String> {
        let position_str = position.as_ref().and_then(|p| p.get_position()).unwrap_or("");

        if target_member == "this_member" {
            if position_str.is_empty() {
                if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some("position_change".to_string());
                }
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "position|destination".to_string(),
                    description: "Choose destination for position change".to_string(),
                });
                return Ok(());
            }
            return self.execute_position_change_with_destination(effect, position_str);
        }

        let card_database = self.game_state.card_database.clone();
        let player = self.game_state.resolve_target_player_mut(target);
        let target_index = match position_str {
            "center" | "センターエリア" => 1,
            "left_side" | "左サイドエリア" => 0,
            "right_side" | "右サイドエリア" => 2,
            _ => return Err(format!("Unknown position: {}", position_str)),
        };

        let current_index = player.stage.stage.iter().position(|&card_id| {
            if card_id == -1 { false }
            else { card_database.get_card(card_id).map(|c| c.card_no == target_member).unwrap_or(false) }
        });

        if let Some(current_idx) = current_index {
            let card_id = player.stage.stage[current_idx];
            if player.stage.stage[target_index] != -1 {
                let occupying_card = player.stage.stage[target_index];
                player.stage.stage[target_index] = card_id;
                player.stage.stage[current_idx] = occupying_card;
            } else {
                player.stage.stage[target_index] = card_id;
                player.stage.stage[current_idx] = -1;
            }
        } else { return Err(format!("Member not found: {}", target_member)); }
        Ok(())
    }

    pub fn execute_position_change_with_destination(&mut self, effect: &AbilityEffect, destination: &str) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        let target_member = effect.target_member.as_deref().unwrap_or("this_member");

        if target_member == "this_member" {
            if let Some(activating_card_id) = self.activating_card_id {
                let player = self.game_state.resolve_target_player_mut(target);

                let target_index = match destination {
                    "center" | "センターエリア" => 1,
                    "left_side" | "左サイドエリア" => 0,
                    "right_side" | "右サイドエリア" => 2,
                    _ => return Err(format!("Unknown destination: {}", destination)),
                };

                let current_index = player.stage.stage.iter().position(|&card_id| card_id == activating_card_id);

                if let Some(current_idx) = current_index {
                    if current_idx == target_index { return Ok(()); }
                    let card_id = player.stage.stage[current_idx];
                    if player.stage.stage[target_index] != -1 {
                        let occupying_card = player.stage.stage[target_index];
                        player.stage.stage[target_index] = card_id;
                        player.stage.stage[current_idx] = occupying_card;
                    } else {
                        player.stage.stage[target_index] = card_id;
                        player.stage.stage[current_idx] = -1;
                    }
                } else { return Err(format!("Activating card {} not found on stage", activating_card_id)); }
            } else { return Err("No activating card for position change".to_string()); }
        }
        Ok(())
    }

    fn execute_appear(&mut self, source: &str, destination: &str, count: u32, target: &str, card_type: Option<&str>) -> Result<(), String> {
        let card_db = self.game_state.card_database.clone();
        let player = self.game_state.resolve_target_player_mut(target);

        match source {
            "deck" => {
                let mut appeared = 0;
                let mut cards_to_record: Vec<i16> = Vec::new();
                while appeared < count {
                    if let Some(card) = player.main_deck.draw() {
                        let matches_type = util::card_matches_type(&card_db, card, card_type);
                        if matches_type {
                            match destination {
                                "stage" => {
                                    if player.stage.stage[1] == -1 { player.stage.stage[1] = card; player.areas_locked_this_turn.insert(crate::zones::MemberArea::Center); }
                                    else if player.stage.stage[0] == -1 { player.stage.stage[0] = card; player.areas_locked_this_turn.insert(crate::zones::MemberArea::LeftSide); }
                                    else if player.stage.stage[2] == -1 { player.stage.stage[2] = card; player.areas_locked_this_turn.insert(crate::zones::MemberArea::RightSide); }
                                    else { player.hand.add_card(card); }
                                    cards_to_record.push(card);
                                }
                                "hand" => player.hand.add_card(card),
                                "discard" => player.waitroom.add_card(card),
                                _ => { eprintln!("Appear destination '{}' not implemented", destination); }
                            }
                            appeared += 1;
                        } else { player.main_deck.cards.push(card); }
                    } else { break; }
                }
                for card_id in cards_to_record { self.game_state.record_card_movement(card_id); }
            }
            "discard" => {
                let mut appeared = 0;
                let mut indices_to_remove = Vec::new();
                for (i, card) in player.waitroom.cards.iter().enumerate() {
                    if appeared >= count { break; }
                    let matches_type = util::card_matches_type(&card_db, *card, card_type);
                    if matches_type { indices_to_remove.push(i); appeared += 1; }
                }
                for i in indices_to_remove.into_iter().rev() {
                    let card = player.waitroom.cards.remove(i);
                    player.hand.add_card(card);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_choice(&mut self, choice_options: Option<&Vec<String>>, choice_type: Option<&str>, options: Option<&Vec<AbilityEffect>>) -> Result<(), String> {
        let options_json = options
            .and_then(|opts| serde_json::to_string(opts).ok())
            .or_else(|| choice_options.and_then(|opts| serde_json::to_string(opts).ok()));
        if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
            entry.choice_card_no = if options.is_some() {
                Some("choice".to_string())
            } else if choice_options.is_some() {
                Some("choice_string".to_string())
            } else {
                Some("choice".to_string())
            };
            entry.conditional_choice = options_json;
        }
        if let Some(effect_options) = options {
            let description = effect_options.iter()
                .map(|o| o.answers.as_ref()
                    .map(|a| a.join(", "))
                    .unwrap_or_else(|| o.text.clone()))
                .collect::<Vec<_>>()
                .join(" / ");
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice".to_string(),
                description,
            });
        } else if let Some(string_options) = choice_options {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice_string".to_string(),
                description: format!("Choose one: {}", string_options.join(", ")),
            });
        } else if let Some(ct) = choice_type {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice".to_string(),
                description: format!("Choose: {}", ct),
            });
        }
        Ok(())
    }

    fn execute_pay_energy(&mut self, count: u32, target: &str) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        if count > 0 { if let Err(e) = player.energy_zone.pay_energy(count as usize) { return Err(e); } }
        Ok(())
    }

    fn execute_set_card_identity(&mut self, identities: &[String]) -> Result<(), String> {
        eprintln!("set_card_identity: identities={:?}", identities);
        if !identities.is_empty() {
            self.game_state.prohibition_effects.push(format!("card_identity:{}", identities.join(",")));
        }
        Ok(())
    }

    fn execute_discard_until_count(&mut self, target_count: u32, target: &str) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        let current_count = player.hand.cards.len();
        if current_count <= target_count as usize { return Ok(()); }
        let cards_to_discard = current_count - target_count as usize;
        self.pending_choice = Some(Choice::SelectCard {
            zone: "hand".to_string(), card_type: None,
            count: cards_to_discard,
            description: format!("Discard {} cards from hand (target: {} cards in hand)", cards_to_discard, target_count),
            allow_skip: false,
        });
        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
        Ok(())
    }

    fn execute_restriction(&mut self, restriction_type: Option<&str>, restricted_destination: Option<&str>) -> Result<(), String> {
        eprintln!("restriction: type={:?}, destination={:?}", restriction_type, restricted_destination);
        self.game_state.prohibition_effects.push(format!("restriction:{}:{}", restriction_type.unwrap_or("unknown"), restricted_destination.unwrap_or("")));
        Ok(())
    }

    fn execute_re_yell(&mut self, lose_blade_hearts: bool, target: &str) -> Result<(), String> {
        eprintln!("re_yell: lose_blade_hearts={}", lose_blade_hearts);
        let player = self.game_state.resolve_target_player_mut(target);
        let mut cards_to_clear_modifiers: Vec<i16> = Vec::new();
        for i in 0..3 {
            if player.stage.stage[i] != -1 {
                let card_id = player.stage.stage[i];
                player.stage.stage[i] = -1;
                player.waitroom.add_card(card_id);
                if lose_blade_hearts { cards_to_clear_modifiers.push(card_id); }
            }
        }
        if lose_blade_hearts {
            for card_id in cards_to_clear_modifiers {
                self.game_state.clear_modifiers_for_card(card_id);
            }
        }
        self.game_state.prohibition_effects.push("re_yell".to_string());
        Ok(())
    }

    fn execute_activation_restriction(&mut self, target: &str) -> Result<(), String> {
        eprintln!("activation_restriction: target={}", target);
        self.game_state.prohibition_effects.push(format!("activation_restriction:{}", target));
        Ok(())
    }

    fn execute_choose_required_hearts(&mut self) -> Result<(), String> {
        self.pending_choice = Some(Choice::SelectTarget {
            target: "choose_required_hearts".to_string(),
            description: "Choose required hearts".to_string(),
        });
        Ok(())
    }

    fn execute_modify_limit(&mut self, operation: &str, count: u32) -> Result<(), String> {
        eprintln!("modify_limit: operation={}, count={}", operation, count);
        match operation {
            "decrease" => { self.game_state.prohibition_effects.push(format!("limit_decrease:{}", count)); }
            "increase" => { self.game_state.prohibition_effects.push(format!("limit_increase:{}", count)); }
            _ => {}
        }
        Ok(())
    }

    fn execute_set_blade_count(&mut self, value: u32, target: &str) -> Result<(), String> {
        eprintln!("set_blade_count: value={}, target={}", value, target);
        let stage_cards: Vec<i16> = {
            let player = self.game_state.resolve_target_player_mut(target);
            player.stage.stage.to_vec()
        };
        for &card_id in stage_cards.iter().filter(|&&id| id != -1) {
            let current = self.game_state.get_blade_modifier(card_id);
            let delta = (value as i32) - current;
            self.game_state.add_blade_modifier(card_id, delta);
        }
        Ok(())
    }

    fn execute_set_required_hearts(&mut self, count: u32, heart_color: &str, target: &str) -> Result<(), String> {
        let color = crate::zones::parse_heart_color(heart_color);
        let card_ids: Vec<i16> = {
            let player = self.game_state.resolve_target_player_mut(target);
            player.live_card_zone.cards.to_vec()
        };
        for card_id in card_ids {
            self.game_state.set_need_heart_modifier(card_id, color, count as i32);
        }
        Ok(())
    }

    fn execute_set_score(&mut self, value: u32, target: &str) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        player.live_score = value as i32;
        Ok(())
    }

    fn execute_specify_heart_color(&mut self, choice: bool, target: &str) -> Result<(), String> {
        eprintln!("specify_heart_color: choice={}, target={}", choice, target);
        if choice {
            self.pending_choice = Some(Choice::SelectTarget { target: "heart_color".to_string(), description: "Choose a heart color".to_string() });
        }
        Ok(())
    }

    fn execute_set_card_identity_all_regions(&mut self, identities: Option<&Vec<String>>, target: &str) -> Result<(), String> {
        let _target = target;
        let card_id = self.activating_card_id.or_else(|| self.game_state.activating_card);
        if let Some(card_id) = card_id {
            if let Some(identities) = identities {
                for identity in identities {
                    self.game_state.prohibition_effects.push(format!("card_identity:{}:{}", card_id, identity));
                }
            }
        }
        Ok(())
    }

    fn execute_modify_required_hearts_success(&mut self, operation: &str, value: u32, target: &str, card_type: Option<&str>) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        let card_ids: Vec<i16> = if let Some("live_card") = card_type { player.success_live_card_zone.cards.iter().copied().collect() } else { vec![] };
        let delta = match operation { "increase" => value as i32, "decrease" => -(value as i32), _ => return Err(format!("Unknown operation: {}", operation)) };
        let heart_colors = ["heart00", "heart01", "heart02", "heart03", "heart04", "heart05", "heart06"];
        for card_id in card_ids {
            for color_str in &heart_colors {
                let color = crate::zones::parse_heart_color(color_str);
                self.game_state.add_need_heart_modifier(card_id, color, delta);
            }
        }
        Ok(())
    }

    fn execute_set_cost_to_use(&mut self, value: u32) -> Result<(), String> {
        let card_id = self.activating_card_id.or_else(|| self.game_state.activating_card);
        if let Some(card_id) = card_id { self.game_state.set_cost_modifier(card_id, value as i32); }
        Ok(())
    }

    fn execute_all_blade_timing(&mut self, timing: &str, treat_as: &str) -> Result<(), String> {
        let card_id = self.activating_card_id.or_else(|| self.game_state.activating_card);
        if let Some(card_id) = card_id {
            self.game_state.prohibition_effects.push(format!("all_blade_timing:{}:{}:{}", card_id, timing, treat_as));
        }
        Ok(())
    }

    fn execute_shuffle(&mut self, target: &str, source: &str) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        match source {
            "deck" => { use rand::seq::SliceRandom; player.main_deck.cards.shuffle(&mut rand::thread_rng()); }
            "energy_deck" => { use rand::seq::SliceRandom; player.energy_deck.cards.shuffle(&mut rand::thread_rng()); }
            _ => { eprintln!("Unknown shuffle zone: {}", source); }
        }
        Ok(())
    }

    fn execute_reveal_per_group(&mut self, source: &str, count: u32, target: &str) -> Result<(), String> {
        let card_db = self.game_state.card_database.clone();
        let card_ids: Vec<i16> = {
            let player = self.game_state.resolve_target_player_mut(target);
            match source {
                "hand" => player.hand.cards.iter().copied().collect(),
                "deck" => player.main_deck.cards.iter().take(count as usize).copied().collect(),
                "discard" => player.waitroom.cards.iter().copied().collect(),
                "looked_at" => self.looked_at_cards.clone(),
                _ => vec![],
            }
        };

        let mut by_group: std::collections::HashMap<String, Vec<i16>> = std::collections::HashMap::new();
        for &card_id in &card_ids {
            let group_name = card_db.get_card(card_id).map(|c| c.group.clone()).unwrap_or_default();
            by_group.entry(group_name).or_default().push(card_id);
        }

        for (_group, members) in &by_group {
            for &card_id in members {
                self.game_state.revealed_cards.insert(card_id);
            }
        }
        Ok(())
    }

    fn execute_modify_cost(&mut self, operation: &str, value: u32, target: &str, card_type: Option<&str>) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        let card_ids: Vec<i16> = if let Some("live_card") = card_type { player.live_card_zone.cards.iter().copied().collect() }
            else if let Some("member_card") = card_type { player.stage.stage.iter().filter(|&&id| id != -1).copied().collect() }
            else if let Some("energy_card") = card_type { player.energy_zone.cards.iter().copied().collect() }
            else { player.hand.cards.iter().copied().collect() };
        let delta = match operation { "add" => value as i32, "subtract" => -(value as i32), "set" => value as i32, _ => return Err(format!("Unknown operation: {}", operation)) };
        for card_id in card_ids {
            if operation == "set" { self.game_state.set_cost_modifier(card_id, delta); }
            else { self.game_state.add_cost_modifier(card_id, delta); }
        }
        Ok(())
    }
}
