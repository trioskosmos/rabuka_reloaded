use crate::card::AbilityEffect;
use super::types::{Choice, ChoiceResult, ExecutionContext, LookAndSelectStep};
use super::util;

#[allow(dead_code)]
impl<'a> super::resolver::AbilityResolver<'a> {
    pub fn resume_execution(&mut self, context: ExecutionContext) -> Result<(), String> {
        match context {
            ExecutionContext::None => Ok(()),
            ExecutionContext::LookAndSelect { step } => {
                match step {
                    LookAndSelectStep::Select { count: _ } => {
                        let select_action = self.current_effect.as_ref().and_then(|e| e.select_action.clone());
                        if let Some(action) = select_action {
                            self.execute_effect(&action)?;
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
                    if !indices.is_empty() {
                        match zone.as_str() {
                            "hand" => {
                                let player = self.game_state.active_player_mut();
                                for &idx in indices.iter().rev() {
                                    if idx < player.hand.cards.len() {
                                        let card_id = player.hand.cards[idx];
                                        player.hand.remove_card(idx);
                                        player.waitroom.add_card(card_id);
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
                            "discard" => {
                                self.execute_selected_cards_from_discard(indices.as_slice(), *count, card_type.as_deref())?;
                            }
                            "deck" => {
                                self.execute_selected_cards_from_deck(indices.as_slice(), *count, card_type.as_deref())?;
                            }
                            "looked_at" => {
                                // Reveal selected cards before moving them
                                for &idx in indices.iter() {
                                    if idx < self.looked_at_cards.len() {
                                        let card_id = self.looked_at_cards[idx];
                                        self.game_state.revealed_cards.insert(card_id);
                                    }
                                }
                                self.execute_selected_looked_at_cards(indices.as_slice())?;
                                self.pending_choice = None;
                                self.resume_execution(context)?;
                                if let Some(ref pending_actions) = self.game_state.pending_sequential_actions.clone() {
                                    for action in pending_actions {
                                        self.execute_effect(action)?;
                                    }
                                    self.game_state.pending_sequential_actions = None;
                                }
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                    self.pending_choice = None;
                    if let Some(effect) = self.game_state.entry_effect().cloned() {
                        self.current_ability = Some(crate::card::Ability {
                            effect: Some(effect.clone()),
                            cost: self.game_state.entry_cost().cloned(),
                            ..Default::default()
                        });
                        let _ = self.execute_effect(&effect);
                        self.current_ability = None;
                    }
                    return Ok(());
                }

                match zone.as_str() {
                    "hand" => self.execute_selected_cards_from_hand(indices.as_slice(), *count, card_type.as_deref())?,
                    "deck" => self.execute_selected_cards_from_deck(indices.as_slice(), *count, card_type.as_deref())?,
                    "discard" => self.execute_selected_cards_from_discard(indices.as_slice(), *count, card_type.as_deref())?,
                    "stage" => self.execute_selected_cards_from_stage(indices.as_slice(), *count, card_type.as_deref())?,
                    "looked_at" => {
                        // Reveal selected cards before moving them
                        for &idx in indices.iter() {
                            if idx < self.looked_at_cards.len() {
                                let card_id = self.looked_at_cards[idx];
                                self.game_state.revealed_cards.insert(card_id);
                            }
                        }
                        self.execute_selected_looked_at_cards(indices.as_slice())?
                    }
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
                self.pending_choice = None;
                self.resume_execution(context)
            }
            (Some(Choice::SelectTarget { target, .. }), ChoiceResult::TargetSelected { target: selected }) => {
                let choice_card_no = self.game_state.entry_choice_card_no();
                let conditional_choice = self.game_state.entry_conditional_choice();

                if choice_card_no.as_deref() == Some("choice") {
                    if let Some(ref options_json) = conditional_choice {
                        if let Ok(options) = serde_json::from_str::<Vec<AbilityEffect>>(options_json) {
                            let selected_index: usize = selected.parse().unwrap_or(0);
                            if selected_index < options.len() {
                                if let Err(e) = self.execute_effect(&options[selected_index]) {
                                    return Err(e);
                                }
                            }
                        }
                    }
                    self.pending_choice = None;
                    self.clear_choice_meta();
                    return Ok(());
                }

                if choice_card_no.as_deref() == Some("choice_string") {
                    if let Some(ref options_json) = conditional_choice {
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
                    self.clear_choice_meta();
                    return Ok(());
                }

                if choice_card_no.as_deref() == Some("position_change") {
                    if let Some(effect) = self.game_state.entry_effect().cloned() {
                        let mut modified_effect = effect.clone();
                        modified_effect.destination = Some(selected.clone());
                        if let Err(e) = self.execute_position_change_with_destination(&modified_effect, &selected) {
                            eprintln!("Failed to execute position change: {}", e);
                        }
                    }
                    self.pending_choice = None;
                    self.clear_choice_meta();
                    return Ok(());
                }

                if target == "pay_optional_cost:skip_optional_cost" {
                    if selected == "skip_optional_cost" {
                        self.pending_choice = None;
                        return Ok(());
                    } else if selected == "pay_optional_cost" {
                        if let Some(cost) = self.game_state.entry_cost().cloned() {
                            if let Some(energy) = cost.energy {
                                if energy > 0 {
                                    let target = cost.target.as_deref().unwrap_or("self");
                                    let player = self.game_state.resolve_target_player_mut(target);
                                    if let Err(e) = player.energy_zone.pay_energy(energy as usize) {
                                        return Err(e);
                                    }
                                }
                            }
                            // Handle change_state optional cost (e.g. put self to wait)
                            if let Some(ref state_change) = cost.state_change {
                                if state_change == "wait" && cost.self_cost == Some(true) {
                                    if let Some(activating_id) = self.game_state.activating_card {
                                        self.game_state.add_orientation_modifier(activating_id, "wait");
                                    }
                                }
                            }
                        }
                        let old_choice = self.pending_choice.clone();
                        if let Some(effect) = self.game_state.entry_effect().cloned() {
                            if let Err(e) = self.execute_effect(&effect) {
                                eprintln!("Failed to execute effect after optional cost: {}", e);
                            }
                        }
                        if self.pending_choice == old_choice {
                            self.pending_choice = None;
                        }
                        return Ok(());
                    }
                }

                if target == "primary|alternative" {
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
                    self.pending_choice = None;
                    return Ok(());
                }

                if target == "apply_replacement" {
                    self.pending_choice = None;
                    return Ok(());
                }

                if target == "choose_required_hearts" {
                    self.game_state.prohibition_effects.push(format!("chosen_required_hearts:{}", selected));
                    self.pending_choice = None;
                    return Ok(());
                }

                if target == "heart_color" {
                    let heart_values = ["heart00", "heart01", "heart02", "heart03", "heart04", "heart05", "heart06"];
                    let idx: usize = selected.parse().unwrap_or(0);
                    if idx < heart_values.len() {
                        let selected_color = heart_values[idx];
                        self.game_state.prohibition_effects.push(format!("selected_heart_color:{}", selected_color));
                    }
                    self.pending_choice = None;
                    return Ok(());
                }

                if target == "choice_type" {
                    self.pending_choice = None;
                    return Ok(());
                }

                if target == "choice_condition" {
                    let idx: usize = selected.parse().unwrap_or(0);
                    let opt = self.game_state.entry_cost().and_then(|c| c.options.clone());
                    if let Some(options) = opt {
                        if idx < options.len() {
                            if let Err(e) = self.pay_cost(&options[idx]) {
                                eprintln!("Failed to pay selected cost option: {}", e);
                            }
                        }
                    }
                    self.pending_choice = None;
                    return Ok(());
                }

                if target == "conditional_optional" {
                    let effect = self.game_state.entry_effect().cloned();
                    if selected == "1" || selected == "yes" {
                        if let Some(ref effect) = effect {
                            if let Some(ref optional) = effect.optional_action {
                                if let Err(e) = self.execute_effect(optional) {
                                    eprintln!("Failed to execute optional action: {}", e);
                                }
                            }
                            if let Some(ref conditional) = effect.conditional_action {
                                if let Err(e) = self.execute_effect(conditional) {
                                    eprintln!("Failed to execute conditional action: {}", e);
                                }
                            }
                        }
                    }
                    self.pending_choice = None;
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
                                    "center" => { player.stage.stage[1] = card_id; player.areas_locked_this_turn.insert(crate::zones::MemberArea::Center); }
                                    "left_side" => { player.stage.stage[0] = card_id; player.areas_locked_this_turn.insert(crate::zones::MemberArea::LeftSide); }
                                    "right_side" => { player.stage.stage[2] = card_id; player.areas_locked_this_turn.insert(crate::zones::MemberArea::RightSide); }
                                    _ => { player.hand.add_card(card_id); }
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

    fn clear_choice_meta(&mut self) {
        if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
            entry.choice_card_no = None;
            entry.conditional_choice = None;
        }
    }

    fn execute_selected_cards_from_zone(&mut self, zone: &str, indices: &[usize], _count: usize, card_type_filter: Option<&str>) -> Result<(), String> {
        let destination = if zone == "discard" { self.game_state.entry_destination().map(|s| s.to_string()) } else { None };
        let character_filter = self.game_state.entry_characters().cloned();
        let player = &mut self.game_state.player1;
        let card_db = self.game_state.card_database.clone();

        let matches_card_type = |card_id: i16, filter: Option<&str>| -> bool {
            util::card_matches_type(&card_db, card_id, filter)
        };

        match zone {
            "hand" => {
                let mut indices_to_remove: Vec<usize> = indices.iter().copied().collect();
                indices_to_remove.sort_by(|a, b| b.cmp(a));
                let mut cards_moved: Vec<i16> = Vec::new();
                for i in indices_to_remove {
                    if i < player.hand.cards.len() {
                        let card_id = player.hand.cards.remove(i);
                        if matches_card_type(card_id, card_type_filter) && util::card_matches_characters(&card_db, card_id, character_filter.as_ref()) {
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
                        if matches_card_type(card_id, card_type_filter) && util::card_matches_characters(&card_db, card_id, character_filter.as_ref()) {
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
                let destination = destination.as_deref().unwrap_or("hand");
                let mut indices_to_remove: Vec<usize> = indices.iter().copied().collect();
                indices_to_remove.sort_by(|a, b| b.cmp(a));
                let mut cards_moved: Vec<i16> = Vec::new();
                for i in indices_to_remove {
                    if i < player.waitroom.cards.len() {
                        let card_id = player.waitroom.cards.remove(i);
                        if matches_card_type(card_id, card_type_filter) && util::card_matches_characters(&card_db, card_id, character_filter.as_ref()) {
                            match destination {
                                "stage" => {
                                    if player.stage.stage[1] == -1 { player.stage.stage[1] = card_id; }
                                    else if player.stage.stage[0] == -1 { player.stage.stage[0] = card_id; }
                                    else if player.stage.stage[2] == -1 { player.stage.stage[2] = card_id; }
                                    else { player.hand.add_card(card_id); }
                                }
                                _ => player.hand.add_card(card_id),
                            }
                            cards_moved.push(card_id);
                        } else {
                            player.waitroom.cards.insert(i, card_id);
                        }
                    }
                }
                for card_id in cards_moved {
                    self.game_state.clear_modifiers_for_card(card_id);
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
            if i < player.energy_zone.cards.len() {
                player.energy_zone.cards.remove(i);
            }
        }
        let deactivated_count = indices.len();
        if player.energy_zone.active_energy_count >= deactivated_count {
            player.energy_zone.active_energy_count -= deactivated_count;
        }
        Ok(())
    }
}
