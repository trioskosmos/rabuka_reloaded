use crate::card::AbilityEffect;
use super::types::{Choice, ChoiceResult, ExecutionContext, LookAndSelectStep};

#[allow(dead_code)]
impl<'a> super::resolver::AbilityResolver<'a> {
    pub fn resume_execution(&mut self, context: ExecutionContext) -> Result<(), String> {
        match context {
            ExecutionContext::None => Ok(()),
            ExecutionContext::LookAndSelect { step } => {
                match step {
                    LookAndSelectStep::Select { count: _ } => {
                        let select_action_to_execute = if let Some(ref effect) = self.current_effect {
                            effect.select_action.clone()
                        } else {
                            None
                        };

                        if let Some(select_action) = select_action_to_execute {
                            self.execute_effect(&select_action)?;
                        }
                        self.execution_context = ExecutionContext::None;
                        Ok(())
                    }
                    LookAndSelectStep::LookAt { .. } => {
                        self.execution_context = ExecutionContext::None;
                        Ok(())
                    }
                    LookAndSelectStep::Finalize { .. } => {
                        self.execution_context = ExecutionContext::None;
                        Ok(())
                    }
                }
            }
            ExecutionContext::SingleEffect { .. } => {
                self.execution_context = ExecutionContext::None;
                Ok(())
            }
            ExecutionContext::SequentialEffects { current_index, effects } => {
                if current_index + 1 < effects.len() {
                    self.execution_context = ExecutionContext::SequentialEffects {
                        current_index: current_index + 1,
                        effects: effects.clone(),
                    };
                    self.execute_effect(&effects[current_index + 1])
                } else {
                    self.execution_context = ExecutionContext::None;
                    Ok(())
                }
            }
        }
    }

    pub fn expire_live_end_effects(&mut self) {
        let initial_count = self.duration_effects.len();
        self.duration_effects.retain(|(_, duration)| duration != "live_end");
        let expired_count = initial_count - self.duration_effects.len();
        if expired_count > 0 {
            eprintln!("Expired {} effects with duration 'live_end'", expired_count);
        }
    }

    pub fn provide_choice_result(&mut self, result: ChoiceResult) -> Result<(), String> {
        let choice = self.pending_choice.clone();
        let context = self.execution_context.clone();
        match (&choice, result) {
            (Some(Choice::SelectCard { zone, card_type, count, description: _, allow_skip }), ChoiceResult::CardSelected { indices }) => {
                if *allow_skip {
                    eprintln!("Detected optional cost choice (allow_skip=true)");
                    if !indices.is_empty() {
                        eprintln!("User chose to pay optional cost with {} cards", indices.len());
                        match zone.as_str() {
                            "hand" => {
                                let player = self.game_state.active_player_mut();
                                for &idx in indices.iter().rev() {
                                    if idx < player.hand.cards.len() {
                                        let card_id = player.hand.cards[idx];
                                        player.hand.remove_card(idx);
                                        player.waitroom.add_card(card_id);
                                        eprintln!("Moved card {} from hand to waitroom for optional cost", card_id);
                                    }
                                }
                            }
                            "stage" => {
                                let player = self.game_state.active_player_mut();
                                let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
                                for &idx in indices.iter().rev() {
                                    if idx < areas.len() {
                                        let area = areas[idx];
                                        if let Some(card_id) = player.stage.get_area(area) {
                                            player.stage.clear_area(area);
                                            player.waitroom.add_card(card_id);
                                        }
                                    }
                                }
                            }
                            "energy_zone" => {
                                let player = self.game_state.active_player_mut();
                                for &idx in indices.iter().rev() {
                                    if idx < player.energy_zone.cards.len() {
                                        let card_id = player.energy_zone.cards.remove(idx);
                                        player.waitroom.add_card(card_id);
                                    }
                                }
                            }
                            _ => {
                                eprintln!("Optional cost payment from zone '{}' not supported", zone);
                            }
                        }
                    } else {
                        eprintln!("User chose to skip optional cost");
                    }
                    self.pending_choice = None;
                    self.game_state.pending_ability = None;
                    if let Some(ref pending) = self.game_state.pending_ability {
                        let ability = crate::card::Ability {
                            effect: Some(pending.effect.clone()),
                            cost: pending.cost.clone(),
                            ..Default::default()
                        };
                        let effect_clone = pending.effect.clone();
                        self.current_ability = Some(ability);
                        let _ = self.execute_effect(&effect_clone);
                        self.current_ability = None;
                    }
                    return Ok(());
                }

                match zone.as_str() {
                    "hand" => self.execute_selected_cards_from_hand(indices.as_slice(), *count, card_type.as_deref())?,
                    "deck" => self.execute_selected_cards_from_deck(indices.as_slice(), *count, card_type.as_deref())?,
                    "discard" => self.execute_selected_cards_from_discard(indices.as_slice(), *count, card_type.as_deref())?,
                    "stage" => self.execute_selected_cards_from_stage(indices.as_slice(), *count, card_type.as_deref())?,
                    "looked_at" => self.execute_selected_looked_at_cards(indices.as_slice())?,
                    "energy_zone" => self.execute_selected_energy_zone_cards(indices.as_slice(), *count)?,
                    _ => eprintln!("Card selection from zone '{}' not yet implemented", zone),
                }
                self.pending_choice = None;
                self.resume_execution(context)?;
                if let Some(ref pending_actions) = self.game_state.pending_sequential_actions.clone() {
                    for action in pending_actions {
                        self.execute_effect(action)?;
                    }
                    self.game_state.pending_sequential_actions = None;
                }
                Ok(())
            }
            (Some(Choice::SelectCard { .. }), ChoiceResult::Skip) => {
                eprintln!("User skipped optional cost");
                self.pending_choice = None;
                self.resume_execution(context)
            }
            (Some(Choice::SelectTarget { target, .. }), ChoiceResult::TargetSelected { target: selected }) => {
                if let Some(ref pending) = self.game_state.pending_ability.clone() {
                    if pending.card_no == "choice" {
                        if let Some(ref options_json) = pending.conditional_choice {
                            if let Ok(options) = serde_json::from_str::<Vec<AbilityEffect>>(options_json) {
                                let selected_index: usize = selected.parse().unwrap_or(0);
                                if selected_index < options.len() {
                                    let selected_effect = &options[selected_index];
                                    if let Err(e) = self.execute_effect(selected_effect) {
                                        return Err(e);
                                    }
                                }
                            }
                        }
                        self.pending_choice = None;
                        self.game_state.pending_ability = None;
                        return Ok(());
                    }

                    if pending.card_no == "choice_string" {
                        if let Some(ref options_json) = pending.conditional_choice {
                            if let Ok(options) = serde_json::from_str::<Vec<String>>(options_json) {
                                if let Ok(selected_idx) = selected.parse::<usize>() {
                                    if selected_idx > 0 && selected_idx <= options.len() {
                                        let selected_value = &options[selected_idx - 1];
                                        if selected_value.starts_with("heart") ||
                                           selected_value == "赤" || selected_value == "桃" ||
                                           selected_value == "緑" || selected_value == "青" ||
                                           selected_value == "黄" || selected_value == "紫" {
                                            self.game_state.prohibition_effects.push(format!("selected_heart_color:{}", selected_value));
                                        }
                                    }
                                }
                            }
                        }
                        self.pending_choice = None;
                        self.game_state.pending_ability = None;
                        return Ok(());
                    }
                }

                if let Some(ref pending) = self.game_state.pending_ability.clone() {
                    if pending.card_no == "position_change" {
                        let mut modified_effect = pending.effect.clone();
                        modified_effect.destination = Some(selected.clone());
                        if let Err(e) = self.execute_position_change_with_destination(&modified_effect, &selected) {
                            eprintln!("Failed to execute position change: {}", e);
                        }
                        self.pending_choice = None;
                        self.game_state.pending_ability = None;
                        return Ok(());
                    }
                }

                if target == "pay_optional_cost:skip_optional_cost" {
                    if selected == "skip_optional_cost" {
                        self.pending_choice = None;
                        return Ok(());
                    } else if selected == "pay_optional_cost" {
                        if let Some(pending) = self.game_state.pending_ability.clone() {
                            if let Some(ref cost) = pending.cost {
                                if let Some(energy) = cost.energy {
                                    if energy > 0 {
                                        let target = cost.target.as_deref().unwrap_or("self");
                                        let player = match target {
                                            "self" => &mut self.game_state.player1,
                                            "opponent" => &mut self.game_state.player2,
                                            _ => &mut self.game_state.player1,
                                        };
                                        if let Err(e) = player.energy_zone.pay_energy(energy as usize) {
                                            return Err(e);
                                        }
                                    }
                                }
                            }
                            if let Err(e) = self.execute_effect(&pending.effect) {
                                eprintln!("Failed to execute effect after optional cost: {}", e);
                            }
                            self.pending_choice = None;
                            return Ok(());
                        }
                    }
                }

                if target == "primary|alternative" {
                    if let Some(ref pending) = self.game_state.pending_ability.clone() {
                        if let Some(ref ability) = self.current_ability.clone() {
                            if let Some(ref effect) = ability.effect {
                                if effect.action == "conditional_alternative" {
                                    match selected.as_str() {
                                        "primary" => {
                                            if let Some(ref primary) = effect.primary_effect {
                                                if let Err(e) = self.execute_effect(primary) {
                                                    eprintln!("Failed to execute primary effect: {}", e);
                                                }
                                            }
                                        }
                                        "alternative" => {
                                            if let Some(ref alternative) = effect.alternative_effect {
                                                if let Err(e) = self.execute_effect(alternative) {
                                                    eprintln!("Failed to execute alternative effect: {}", e);
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    self.pending_choice = None;
                    self.game_state.pending_ability = None;
                    return Ok(());
                }
                if target == "apply_replacement" {
                    if selected == "1" || selected == "yes" {
                        eprintln!("User accepted replacement effect");
                    } else {
                        eprintln!("User declined replacement effect");
                    }
                    self.pending_choice = None;
                    return Ok(());
                }

                if target == "choose_required_hearts" {
                    eprintln!("User selected required hearts option: {}", selected);
                    self.game_state.prohibition_effects.push(format!("chosen_required_hearts:{}", selected));
                    self.pending_choice = None;
                    self.game_state.pending_ability = None;
                    return Ok(());
                }

                if target == "heart_color" {
                    let heart_values = ["heart00", "heart01", "heart02", "heart03", "heart04", "heart05", "heart06"];
                    let idx: usize = selected.parse().unwrap_or(0);
                    if idx < heart_values.len() {
                        let selected_color = heart_values[idx];
                        eprintln!("User selected heart color: {}", selected_color);
                        self.game_state.prohibition_effects.push(format!("selected_heart_color:{}", selected_color));
                    }
                    self.pending_choice = None;
                    self.game_state.pending_ability = None;
                    return Ok(());
                }

                if target == "choice_type" {
                    eprintln!("User selected choice type option: {}", selected);
                    self.pending_choice = None;
                    self.game_state.pending_ability = None;
                    return Ok(());
                }

                if target == "choice_condition" {
                    let idx: usize = selected.parse().unwrap_or(0);
                    eprintln!("User selected cost option {} for choice_condition", idx);
                    if let Some(ref pending) = self.game_state.pending_ability.clone() {
                        if let Some(ref cost) = pending.cost {
                            if let Some(ref options) = cost.options {
                                if idx < options.len() {
                                    let selected_cost = &options[idx];
                                    if let Err(e) = self.pay_cost(selected_cost) {
                                        eprintln!("Failed to pay selected cost option: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    self.pending_choice = None;
                    self.game_state.pending_ability = None;
                    return Ok(());
                }

                self.pending_choice = None;
                Ok(())
            }
            (Some(Choice::SelectPosition { .. }), ChoiceResult::PositionSelected { position }) => {
                if let ExecutionContext::LookAndSelect { step } = context {
                    if let LookAndSelectStep::Finalize { destination } = step {
                        if destination == "stage" {
                            if let Some(&card_id) = self.looked_at_cards.last() {
                                let player = &mut self.game_state.player1;
                                match position.as_str() {
                                    "center" => {
                                        player.stage.stage[1] = card_id;
                                        player.areas_locked_this_turn.insert(crate::zones::MemberArea::Center);
                                    }
                                    "left_side" => {
                                        player.stage.stage[0] = card_id;
                                        player.areas_locked_this_turn.insert(crate::zones::MemberArea::LeftSide);
                                    }
                                    "right_side" => {
                                        player.stage.stage[2] = card_id;
                                        player.areas_locked_this_turn.insert(crate::zones::MemberArea::RightSide);
                                    }
                                    _ => {
                                        player.hand.add_card(card_id);
                                    }
                                }
                                self.looked_at_cards.clear();
                            }
                        }
                    }
                }
                self.pending_choice = None;
                self.execution_context = ExecutionContext::None;
                Ok(())
            }
            _ => Err("Choice result does not match pending choice".to_string()),
        }
    }

    fn execute_selected_cards_from_zone(&mut self, zone: &str, indices: &[usize], _count: usize, card_type_filter: Option<&str>) -> Result<(), String> {
        let player = &mut self.game_state.player1;
        let card_db = self.game_state.card_database.clone();

        let character_filter = self.game_state.pending_ability.as_ref()
            .and_then(|p| p.cost.as_ref())
            .and_then(|c| c.characters.as_ref());

        let matches_card_type = |card_id: i16, filter: Option<&str>| -> bool {
            match filter {
                Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
                None => true,
                _ => true,
            }
        };

        let matches_character_names = |card_id: i16, names: Option<&Vec<String>>| -> bool {
            if let Some(required_names) = names {
                if let Some(card) = card_db.get_card(card_id) {
                    required_names.iter().any(|name| card.name.contains(name) || card.name == *name)
                } else {
                    false
                }
            } else {
                true
            }
        };

        match zone {
            "hand" => {
                let mut indices_to_remove: Vec<usize> = indices.iter().copied().collect();
                indices_to_remove.sort_by(|a, b| b.cmp(a));
                let mut cards_moved: Vec<i16> = Vec::new();
                for i in indices_to_remove {
                    if i < player.hand.cards.len() {
                        let card_id = player.hand.cards.remove(i);
                        if matches_card_type(card_id, card_type_filter) && matches_character_names(card_id, character_filter) {
                            player.waitroom.add_card(card_id);
                            cards_moved.push(card_id);
                        } else {
                            player.hand.cards.insert(i, card_id);
                        }
                    }
                }
                for card_id in cards_moved {
                    self.game_state.clear_modifiers_for_card(card_id);
                }
            }
            "deck" => {
                let mut indices_to_remove: Vec<usize> = indices.iter().copied().collect();
                indices_to_remove.sort_by(|a, b| b.cmp(a));
                let mut cards_moved: Vec<i16> = Vec::new();
                for i in indices_to_remove {
                    if i < player.main_deck.cards.len() {
                        let card_id = player.main_deck.cards.remove(i);
                        if matches_card_type(card_id, card_type_filter) && matches_character_names(card_id, character_filter) {
                            player.hand.add_card(card_id);
                            cards_moved.push(card_id);
                        } else {
                            player.main_deck.cards.insert(i, card_id);
                        }
                    }
                }
                for card_id in cards_moved {
                    self.game_state.clear_modifiers_for_card(card_id);
                }
            }
            "discard" => {
                let mut indices_to_remove: Vec<usize> = indices.iter().copied().collect();
                indices_to_remove.sort_by(|a, b| b.cmp(a));
                for i in indices_to_remove {
                    if i < player.waitroom.cards.len() {
                        let card_id = player.waitroom.cards.remove(i);
                        if matches_card_type(card_id, card_type_filter) && matches_character_names(card_id, character_filter) {
                            player.hand.add_card(card_id);
                        } else {
                            player.waitroom.cards.insert(i, card_id);
                        }
                    }
                }
            }
            "stage" => {
                let stage_positions = [0, 1, 2];
                for &idx in indices {
                    if idx < stage_positions.len() {
                        let pos = stage_positions[idx];
                        if player.stage.stage[pos] != -1 {
                            let card_id = player.stage.stage[pos];
                            if matches_card_type(card_id, card_type_filter) {
                                player.stage.stage[pos] = -1;
                                player.hand.add_card(card_id);
                            }
                        }
                    }
                }
            }
            _ => return Err(format!("Unknown zone: {}", zone)),
        }
        Ok(())
    }

    fn execute_selected_cards_from_hand(&mut self, indices: &[usize], count: usize, card_type_filter: Option<&str>) -> Result<(), String> {
        self.execute_selected_cards_from_zone("hand", indices, count, card_type_filter)
    }

    fn execute_selected_cards_from_deck(&mut self, indices: &[usize], count: usize, card_type_filter: Option<&str>) -> Result<(), String> {
        self.execute_selected_cards_from_zone("deck", indices, count, card_type_filter)
    }

    fn execute_selected_cards_from_discard(&mut self, indices: &[usize], count: usize, card_type_filter: Option<&str>) -> Result<(), String> {
        self.execute_selected_cards_from_zone("discard", indices, count, card_type_filter)
    }

    fn execute_selected_cards_from_stage(&mut self, indices: &[usize], count: usize, card_type_filter: Option<&str>) -> Result<(), String> {
        self.execute_selected_cards_from_zone("stage", indices, count, card_type_filter)
    }

    fn execute_selected_looked_at_cards(&mut self, indices: &[usize]) -> Result<(), String> {
        let player = &mut self.game_state.player1;
        let mut indices_to_remove: Vec<usize> = indices.iter().copied().collect();
        indices_to_remove.sort_by(|a, b| b.cmp(a));
        for i in indices_to_remove {
            if i < self.looked_at_cards.len() {
                let card_id = self.looked_at_cards.remove(i);
                player.hand.add_card(card_id);
            }
        }
        for card_id in self.looked_at_cards.drain(..) {
            player.waitroom.add_card(card_id);
        }
        Ok(())
    }

    fn execute_selected_energy_zone_cards(&mut self, indices: &[usize], _count: usize) -> Result<(), String> {
        let player = &mut self.game_state.player1;
        let mut indices_to_remove: Vec<usize> = indices.iter().copied().collect();
        indices_to_remove.sort_by(|a, b| b.cmp(a));
        for i in indices_to_remove {
            if i < self.looked_at_cards.len() {
                self.looked_at_cards.remove(i);
            }
        }
        let deactivated_count = indices.len();
        if player.energy_zone.active_energy_count >= deactivated_count {
            player.energy_zone.active_energy_count -= deactivated_count;
        }
        Ok(())
    }
}
