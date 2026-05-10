use crate::card::AbilityEffect;
use super::types::{Choice, ChoiceResult, ExecutionContext, LookAndSelectStep};
use super::util;

impl<'a> super::resolver::AbilityResolver<'a> {
    pub fn resume_execution(&mut self, context: ExecutionContext) -> Result<(), String> {
        match context {
            ExecutionContext::None => Ok(()),
            ExecutionContext::LookAndSelect { step } => {
                match step {
                    LookAndSelectStep::Select { count: _ } => {
                        if let Some(ref select_action) = self.current_effect.as_ref().and_then(|e| e.compound.select_action.clone()) {
                            if select_action.action == "sequential" {
                                self.pending_choice = None;
                                self.execution_context = ExecutionContext::None;
                                if let Some(ref actions) = select_action.compound.actions {
                                    if actions.len() == 3 {
                                        if let Some(last_action) = actions.last() {
                                            if last_action.action == "move_cards" && last_action.source.as_deref() == Some("looked_at_remaining") {
                                                let mut discard_only = last_action.clone();
                                                discard_only.source = Some("looked_at".to_string());
                                                return self.execute_effect(&discard_only);
                                            }
                                        }
                                    }
                                }
                                return self.execute_effect(&select_action);
                            }
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
            ExecutionContext::SingleEffect { effect_index: _ } => {
                self.execution_context = ExecutionContext::None;
                Ok(())
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

    /// Shared epilogue: clear pending_choice, resume execution, process pending sequential actions.
    fn finalize_choice(&mut self, context: &ExecutionContext) -> Result<(), String> {
        self.pending_choice = None;
        self.resume_execution(context.clone())?;
        if let Some(ref pending) = self.game_state.pending_sequential_actions.clone() {
            for action in pending {
                self.execute_effect(action)?;
            }
            self.game_state.pending_sequential_actions = None;
        }
        Ok(())
    }

    fn reveal_selected_looked_at(&mut self, indices: &[usize]) {
        for &idx in indices.iter() {
            if idx < self.looked_at_cards.len() {
                self.game_state.revealed_cards.push(self.looked_at_cards[idx]);
            }
        }
    }

    pub fn provide_choice_result(&mut self, result: ChoiceResult) -> Result<(), String> {
        let choice = self.pending_choice.clone();
        let context = self.execution_context.clone();
        match (&choice, result) {
            (Some(Choice::SelectCard { zone, card_type, count, description: _, allow_skip, cost_limit, cost_limit_operator, group, characters, filtered_indices }), ChoiceResult::CardSelected { indices }) => {
                self.handle_select_card(zone, card_type, *count, *allow_skip, &indices, context, *cost_limit, cost_limit_operator.clone(), group.clone(), characters.clone(), filtered_indices.clone())
            }
            (Some(Choice::SelectCard { .. }), ChoiceResult::Skip) => {
                self.pending_choice = None;
                self.resume_execution(context)
            }
            (Some(Choice::SelectTarget { target, .. }), ChoiceResult::TargetSelected { target: selected }) => {
                self.handle_select_target(target, &selected, context)
            }
            (Some(Choice::SelectPosition { .. }), ChoiceResult::PositionSelected { position }) => {
                self.handle_select_position(&position, context)
            }
            (Some(Choice::SelectHeartColor { count, .. }), ChoiceResult::HeartColorSelected { colors })
            | (Some(Choice::SelectHeartType { count, .. }), ChoiceResult::HeartTypeSelected { types: colors }) => {
                self.handle_heart_selection(*count as u32, &colors)
            }
            _ => Err("Choice result does not match pending choice".to_string()),
        }
    }

    fn handle_select_card(&mut self, zone: &str, card_type: &Option<String>, count: usize, allow_skip: bool, indices: &[usize], context: ExecutionContext,
        cost_limit: Option<u32>, cost_limit_operator: Option<String>, group: Option<String>, characters: Option<Vec<String>>,
        filtered_indices: Option<Vec<usize>>) -> Result<(), String> {
        if self.game_state.entry_cost().and_then(|c| c.cost_type.as_deref()) == Some("reveal") {
            let card_ids: Vec<i16> = {
                let player = self.game_state.active_player();
                indices.iter().filter_map(|&idx| {
                    if idx < player.hand.cards.len() { Some(player.hand.cards[idx]) } else { None }
                }).collect()
            };
            for card_id in card_ids {
                self.game_state.revealed_cards.push(card_id);
                self.revealed_cost_cards.push(card_id);
            }
            return self.finalize_choice(&context);
        }

        let card_db = self.game_state.card_database.clone();
        let validate_card = |cid: i16| -> bool {
            util::card_matches_type(&card_db, cid, card_type.as_deref())
                && util::card_matches_cost_limit_op(&card_db, cid, cost_limit, cost_limit_operator.as_deref())
                && util::card_matches_group_str(&card_db, cid, group.as_deref())
                && match &characters {
                    Some(chars) if !chars.is_empty() => util::card_matches_characters(&card_db, cid, Some(chars)),
                    _ => true,
                }
        };

        if allow_skip && !indices.is_empty() {
            match zone {
                "hand" => {
                    let player = self.game_state.active_player_mut();
                    for &idx in indices.iter().rev() {
                        if idx < player.hand.cards.len() {
                            if validate_card(player.hand.cards[idx]) {
                                player.waitroom.add_card(player.hand.cards[idx]);
                                player.hand.remove_card(idx);
                            }
                        }
                    }
                }
                "stage" => {
                    let mut last_vacated = None;
                    {
                        let player = self.game_state.active_player_mut();
                        for &idx in indices.iter().rev() {
                            if idx < 3 && player.stage.stage[idx] != -1 && validate_card(player.stage.stage[idx]) {
                                if let Some(card_id) = player.remove_member_from_stage_with_recycling(idx, &card_db) {
                                    player.waitroom.add_card(card_id);
                                    last_vacated = Some(idx);
                                }
                            }
                        }
                    }
                    if let Some(pos) = last_vacated { self.game_state.last_vacated_stage_area = Some(pos); }
                }
                "energy_zone" => {
                    let player = self.game_state.active_player_mut();
                    for &idx in indices.iter().rev() {
                        if idx < player.energy_zone.cards.len() && validate_card(player.energy_zone.cards[idx]) {
                            player.waitroom.add_card(player.energy_zone.cards.remove(idx));
                        }
                    }
                }
                "discard" => self.execute_selected_cards_from_discard(indices, count, card_type.as_deref(), cost_limit, cost_limit_operator.as_deref(), group.as_deref(), characters.as_ref())?,
                "deck" => self.execute_selected_cards_from_deck(indices, count, card_type.as_deref())?,
                "looked_at" => {
                    let mapped_indices: Vec<usize> = if let Some(ref fidx) = filtered_indices {
                        indices.iter().filter_map(|&i| fidx.get(i).copied()).collect()
                    } else {
                        indices.to_vec()
                    };
                    self.reveal_selected_looked_at(&mapped_indices);
                    self.execute_selected_looked_at_cards(&mapped_indices)?;
                    return self.finalize_choice(&context);
                }
                "revealed_cards" => {
                    let cards: Vec<i16> = indices.iter().map(|&i| self.game_state.revealed_cards.remove(i)).collect();
                    self.selected_cards = cards;
                    return self.finalize_choice(&context);
                }
                _ => {}
            }
            self.pending_choice = None;
            return Ok(());
        }

        match zone {
            "hand" => self.execute_selected_cards_from_hand(indices, count, card_type.as_deref(), cost_limit, cost_limit_operator.as_deref(), group.as_deref(), characters.as_ref())?,
            "deck" => self.execute_selected_cards_from_deck(indices, count, card_type.as_deref())?,
            "discard" => self.execute_selected_cards_from_discard(indices, count, card_type.as_deref(), cost_limit, cost_limit_operator.as_deref(), group.as_deref(), characters.as_ref())?,
            "stage" => self.execute_selected_cards_from_stage(indices, count, card_type.as_deref(), cost_limit, cost_limit_operator.as_deref(), group.as_deref(), characters.as_ref())?,
            "looked_at" => {
                self.reveal_selected_looked_at(indices);
                self.execute_selected_looked_at_cards(indices)?
            }
            "revealed_cards" => {
                let mut cards = Vec::new();
                for &i in indices.iter().rev() {
                    if i < self.game_state.revealed_cards.len() {
                        let cid = self.game_state.revealed_cards.remove(i);
                        if validate_card(cid) {
                            cards.push(cid);
                        }
                    }
                }
                self.selected_cards = cards;
            }
            "energy_zone" => self.execute_selected_energy_zone_cards(indices, count)?,
            _ => eprintln!("Card selection from zone '{}' not yet implemented", zone),
        }
        if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
            entry.selected_card_ids = self.selected_cards.clone();
        }
        self.finalize_choice(&context)
    }

    fn handle_select_target(&mut self, target: &str, selected: &str, _context: ExecutionContext) -> Result<(), String> {
        let choice_card_no = self.game_state.entry_choice_card_no();
        let conditional_choice = self.game_state.entry_conditional_choice();

        if choice_card_no.as_deref() == Some("choice") {
            if let Some(ref options_json) = conditional_choice {
                if let Ok(options) = serde_json::from_str::<Vec<AbilityEffect>>(options_json) {
                    let idx: usize = selected.parse().unwrap_or(0);
                    if idx < options.len() {
                        self.execute_effect(&options[idx]).map_err(|e| e)?;
                    }
                }
            }
            self.pending_choice = None;
            self.clear_choice_meta();
            return Ok(());
        }

        if choice_card_no.as_deref() == Some("choice_string") {
            return self.handle_choice_string_selection(selected, conditional_choice);
        }

        if choice_card_no.as_deref().map(|s| s.starts_with("position_change")).unwrap_or(false) {
            return self.handle_position_change_choice(choice_card_no, selected);
        }

        if target == "choice_string" {
            return self.handle_choice_string_store(selected, conditional_choice);
        }

        if target == "pay_optional_cost:skip_optional_cost" {
            return self.handle_optional_cost_payment(selected);
        }

        if target == "primary|alternative" {
            return self.handle_primary_alternative(selected);
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

        if target == "position|destination" {
            return self.handle_position_destination(selected);
        }

        if target == "heart_color" {
            return self.handle_heart_color_selection(selected);
        }

        if target == "choice_type" {
            self.pending_choice = None;
            return Ok(());
        }

        if target == "choice_condition" {
            return self.handle_choice_condition(selected);
        }

        if target == "conditional_optional" {
            return self.handle_conditional_optional(selected);
        }

        if target == "draw_any_number" {
            return self.handle_draw_any_number(selected);
        }

        if target == "order" {
            return self.handle_order_selection(selected);
        }

        self.pending_choice = None;
        Ok(())
    }

    fn handle_draw_any_number(&mut self, selected: &str) -> Result<(), String> {
        let count: usize = selected.parse().unwrap_or(0);
        if let Some(effect) = self.game_state.entry_effect().cloned() {
            let source = effect.source.as_deref().unwrap_or("deck");
            let destination = effect.destination.as_deref().unwrap_or("hand");
            let card_type = effect.card_type.as_deref();
            let card_db = self.game_state.card_database.clone();
            let target = effect.target.as_deref().unwrap_or("self");
            let player = self.game_state.resolve_target_player_mut(target);
            if count > 0 {
                crate::ability::effects::draw_cards_for_player(
                    player, count as u32, source, destination, card_type, false, None, &card_db, None
                )?;
            }
        }
        self.pending_choice = None;
        Ok(())
    }

    fn handle_order_selection(&mut self, selected: &str) -> Result<(), String> {
        let ctx = self.execution_context.clone();
        if let ExecutionContext::LookAndSelect { step } = ctx {
            if let LookAndSelectStep::Finalize { destination } = step {
                if destination == "deck" {
                    if let Ok(idx) = selected.parse::<usize>() {
                        if idx < self.looked_at_cards.len() {
                            let card = self.looked_at_cards.remove(idx);
                            self.looked_at_cards.insert(0, card);
                        }
                    }
                }
            }
        }
        self.pending_choice = None;
        Ok(())
    }

    fn handle_choice_string_selection(&mut self, selected: &str, conditional_choice: Option<String>) -> Result<(), String> {
        if let Some(ref options_json) = conditional_choice {
            if let Ok(options) = serde_json::from_str::<Vec<String>>(options_json) {
                if let Ok(idx) = selected.parse::<usize>() {
                    if idx > 0 && idx <= options.len() {
                        let val = &options[idx - 1];
                        if val.starts_with("heart") || ["赤", "桃", "緑", "青", "黄", "紫"].contains(&val.as_str()) {
                            self.game_state.prohibition_effects.push(format!("selected_heart_color:{}", val));
                        }
                    }
                }
            }
        }
        self.pending_choice = None;
        self.clear_choice_meta();
        Ok(())
    }

    fn handle_position_change_choice(&mut self, choice_card_no: Option<String>, selected: &str) -> Result<(), String> {
        if let Some(effect) = self.game_state.entry_effect().cloned() {
            let mut modified = effect.clone();
            let dest = match selected {
                "0" | "left" => "left_side",
                "1" | "center" => "center",
                "2" | "right" => "right_side",
                _ => selected,
            };
            if let Some(ref ccn) = choice_card_no {
                if let Some(tgt) = ccn.strip_prefix("position_change:") {
                    modified.target = Some(tgt.to_string());
                }
            }
            modified.destination = Some(dest.to_string());
            if let Err(e) = self.execute_position_change_with_destination(&modified, dest) {
                eprintln!("Failed to execute position change: {}", e);
            }
        }
        self.pending_choice = None;
        self.clear_choice_meta();
        if let Some(ref pending) = self.game_state.pending_sequential_actions.clone() {
            for action in pending {
                if let Err(e) = self.execute_effect(action) {
                    eprintln!("Failed to execute pending action after position change: {}", e);
                }
            }
            self.game_state.pending_sequential_actions = None;
        }
        Ok(())
    }

    fn handle_choice_string_store(&mut self, selected: &str, conditional_choice: Option<String>) -> Result<(), String> {
        let chosen = conditional_choice.and_then(|json| {
            serde_json::from_str::<Vec<String>>(&json).ok().and_then(|opts| {
                selected.parse::<usize>().ok().and_then(|idx| opts.get(idx).cloned())
            })
        });
        if let Some(s) = chosen {
            self.game_state.ability_queue.current_entry_mut().map(|e| e.conditional_choice = Some(s));
        }
        self.pending_choice = None;
        if let Some(ref pending) = self.game_state.pending_sequential_actions.clone() {
            for action in pending {
                self.execute_effect(action)?;
            }
            self.game_state.pending_sequential_actions = None;
        }
        Ok(())
    }

    fn handle_optional_cost_payment(&mut self, selected: &str) -> Result<(), String> {
        if selected == "skip_optional_cost" {
            self.pending_choice = None;
            return Ok(());
        }
        if selected == "pay_optional_cost" {
            if let Some(cost) = self.game_state.entry_cost().cloned() {
                if let Some(energy) = cost.energy { if energy > 0 {
                    let tgt = cost.target.as_deref().unwrap_or("self");
                    self.game_state.resolve_target_player_mut(tgt).energy_zone.pay_energy(energy as usize).map_err(|e| e)?;
                }}
                if cost.state_change.as_deref() == Some("wait") && cost.self_cost == Some(true) {
                    if let Some(id) = self.game_state.activating_card {
                        self.game_state.add_orientation_modifier(id, "wait");
                    }
                }
            }
            self.pending_choice = None;
            if self.game_state.entry_cost().is_some() {
                if let Some(effect) = self.game_state.entry_effect().cloned() {
                    if let Err(e) = self.execute_effect(&effect) { eprintln!("Failed to execute effect after optional cost: {}", e); }
                }
            } else if let Some(ref pending) = self.game_state.pending_sequential_actions.clone() {
                for action in pending {
                    if let Err(e) = self.execute_effect(action) { eprintln!("Failed to execute action after optional: {}", e); }
                }
                self.game_state.pending_sequential_actions = None;
            }
        }
        Ok(())
    }

    fn handle_primary_alternative(&mut self, selected: &str) -> Result<(), String> {
        if let Some(ref ability) = self.current_ability.clone() {
            if let Some(ref effect) = ability.effect {
                if effect.action == "conditional_alternative" {
                    match selected {
                    "primary" => if let Some(ref p) = effect.compound.primary_effect { if let Err(e) = self.execute_effect(p) { eprintln!("Failed to execute primary: {}", e); } }
                    "alternative" => if let Some(ref a) = effect.compound.alternative_effect { if let Err(e) = self.execute_effect(a) { eprintln!("Failed to execute alternative: {}", e); } }
                        _ => {}
                    }
                }
            }
        }
        self.pending_choice = None;
        Ok(())
    }

    fn handle_position_destination(&mut self, selected: &str) -> Result<(), String> {
        let dest = match selected { "0" | "left" => "left_side", "1" | "center" => "center", "2" | "right" => "right_side", _ => "center" };
        if let Some(ref ability) = self.current_ability.clone() {
            if let Some(ref effect) = ability.effect {
                if effect.action == "position_change" {
                    if let Err(e) = self.execute_position_change_with_destination(effect, dest) { eprintln!("Failed position change: {}", e); }
                }
            }
        }
        self.pending_choice = None;
        Ok(())
    }

    fn handle_heart_color_selection(&mut self, selected: &str) -> Result<(), String> {
        const HEART_VALS: [&str; 7] = ["heart00", "heart01", "heart02", "heart03", "heart04", "heart05", "heart06"];
        let idx: usize = selected.parse().unwrap_or(0);
        if idx < HEART_VALS.len() {
            self.game_state.prohibition_effects.push(format!("selected_heart_color:{}", HEART_VALS[idx]));
        }
        self.pending_choice = None;
        Ok(())
    }

    fn handle_choice_condition(&mut self, selected: &str) -> Result<(), String> {
        let idx: usize = selected.parse().unwrap_or(0);
        if let Some(options) = self.game_state.entry_cost().and_then(|c| c.options.clone()) {
            if idx < options.len() {
                if let Err(e) = self.pay_cost(&options[idx]) { eprintln!("Failed to pay cost option: {}", e); }
            }
        }
        self.pending_choice = None;
        Ok(())
    }

    fn handle_conditional_optional(&mut self, selected: &str) -> Result<(), String> {
        let effect = self.game_state.entry_effect().cloned();
        self.pending_choice = None;
        let is_negation = effect.as_ref().map(|e| e.compound.conditional_negation.unwrap_or(false)).unwrap_or(false);
        if selected == "1" || selected == "yes" {
            if let Some(ref e) = effect {
                if let Some(ref o) = e.compound.optional_action { if let Err(err) = self.execute_effect(o) { eprintln!("Failed optional action: {}", err); } }
                if !is_negation { if let Some(ref c) = e.compound.conditional_action { if let Err(err) = self.execute_effect(c) { eprintln!("Failed conditional action: {}", err); } } }
            }
        } else if is_negation {
            if let Some(ref e) = effect { if let Some(ref c) = e.compound.conditional_action { if let Err(err) = self.execute_effect(c) { eprintln!("Failed conditional action: {}", err); } } }
        }
        Ok(())
    }

    fn handle_select_position(&mut self, position: &str, context: ExecutionContext) -> Result<(), String> {
        if let ExecutionContext::LookAndSelect { step } = context {
            if let LookAndSelectStep::Finalize { destination } = step {
                if destination == "stage" {
                    if let Some(&card_id) = self.looked_at_cards.last() {
                        let player = &mut self.game_state.player1;
                        match position {
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

    fn handle_heart_selection(&mut self, count: u32, colors: &[String]) -> Result<(), String> {
        if let Some(chosen) = colors.first() {
            let color = crate::zones::parse_heart_color(chosen);
            if let Some(card_id) = self.game_state.activating_card {
                self.game_state.set_heart_override(card_id, color, count.max(1), "live_end");
            }
            if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                entry.conditional_choice = Some(chosen.clone());
            }
        }
        self.pending_choice = None;
        self.finalize_choice(&self.execution_context.clone())
    }

    fn clear_choice_meta(&mut self) {
        if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
            entry.choice_card_no = None;
            entry.conditional_choice = None;
        }
    }

    fn execute_selected_cards_from_zone(&mut self, zone: &str, indices: &[usize], _count: usize, card_type_filter: Option<&str>,
        cost_limit: Option<u32>, cost_limit_operator: Option<&str>, group: Option<&str>, characters: Option<&Vec<String>>) -> Result<(), String> {
        let destination = if zone == "discard" { self.game_state.entry_destination().map(|s| s.to_string()) } else { None };
        let target = self.game_state.entry_effect()
            .and_then(|e| e.target.clone())
            .unwrap_or_else(|| "self".to_string());
        let card_db = self.game_state.card_database.clone();
        let vacated_area = self.game_state.last_vacated_stage_area;
        let player = self.game_state.resolve_target_player_mut(&target);

        let mut moved: Vec<i16> = Vec::new();

        let passes = |cid: i16| -> bool {
            util::card_matches_type(&card_db, cid, card_type_filter)
                && util::card_matches_cost_limit_op(&card_db, cid, cost_limit, cost_limit_operator)
                && util::card_matches_group_str(&card_db, cid, group)
                && match characters {
                    Some(chars) if !chars.is_empty() => util::card_matches_characters(&card_db, cid, Some(chars)),
                    _ => true,
                }
        };

        match zone {
            "hand" => {
                let mut idxs: Vec<usize> = indices.iter().copied().collect();
                idxs.sort_by(|a, b| b.cmp(a));
                for i in idxs {
                    if i < player.hand.cards.len() {
                        let card_id = player.hand.cards.remove(i);
                        if passes(card_id) { player.waitroom.add_card(card_id); moved.push(card_id); }
                        else { player.hand.cards.insert(i, card_id); }
                    }
                }
            }
            "deck" => {
                let mut idxs: Vec<usize> = indices.iter().copied().collect();
                idxs.sort_by(|a, b| b.cmp(a));
                for i in idxs {
                    if i < player.main_deck.cards.len() {
                        let card_id = player.main_deck.cards.remove(i);
                        if passes(card_id) { player.hand.add_card(card_id); moved.push(card_id); }
                        else { player.main_deck.cards.insert(i, card_id); }
                    }
                }
            }
            "discard" => {
                let dest = destination.as_deref().unwrap_or("hand");
                let mut idxs: Vec<usize> = indices.iter().copied().collect();
                idxs.sort_by(|a, b| b.cmp(a));
                for i in idxs {
                    if i < player.waitroom.cards.len() {
                        let card_id = player.waitroom.cards.remove(i);
                        if passes(card_id) {
                            match dest {
                                "stage" => {
                                    if player.stage.stage[1] == -1 { player.stage.stage[1] = card_id; }
                                    else if player.stage.stage[0] == -1 { player.stage.stage[0] = card_id; }
                                    else if player.stage.stage[2] == -1 { player.stage.stage[2] = card_id; }
                                    else { player.hand.add_card(card_id); }
                                }
                                "same_area" => { super::util::place_card_in_zone(player, card_id, "same_area", vacated_area, false, 1); }
                                _ => player.hand.add_card(card_id),
                            }
                            moved.push(card_id);
                        } else { player.waitroom.cards.insert(i, card_id); }
                    }
                }
            }
            "stage" => {
                for &idx in indices {
                    if idx < 3 && player.stage.stage[idx] != -1 {
                        let card_id = player.stage.stage[idx];
                        if passes(card_id) { self.selected_cards.push(card_id); }
                    }
                }
            }
            "revealed_cards" => {
                for &idx in indices.iter().rev() {
                    if idx < self.game_state.revealed_cards.len() {
                        let card_id = self.game_state.revealed_cards.remove(idx);
                        if passes(card_id) { self.selected_cards.push(card_id); }
                    }
                }
            }
            _ => return Err(format!("Unknown zone: {}", zone)),
        }
        for cid in moved { self.game_state.clear_modifiers_for_card(cid); }
        Ok(())
    }

    fn execute_selected_cards_from_hand(&mut self, indices: &[usize], count: usize, card_type_filter: Option<&str>,
        cost_limit: Option<u32>, cost_limit_operator: Option<&str>, group: Option<&str>, characters: Option<&Vec<String>>) -> Result<(), String> {
        self.execute_selected_cards_from_zone("hand", indices, count, card_type_filter, cost_limit, cost_limit_operator, group, characters)
    }
    fn execute_selected_cards_from_deck(&mut self, indices: &[usize], count: usize, card_type_filter: Option<&str>) -> Result<(), String> {
        self.execute_selected_cards_from_zone("deck", indices, count, card_type_filter, None, None, None, None)
    }
    fn execute_selected_cards_from_discard(&mut self, indices: &[usize], count: usize, card_type_filter: Option<&str>,
        cost_limit: Option<u32>, cost_limit_operator: Option<&str>, group: Option<&str>, characters: Option<&Vec<String>>) -> Result<(), String> {
        self.execute_selected_cards_from_zone("discard", indices, count, card_type_filter, cost_limit, cost_limit_operator, group, characters)
    }
    fn execute_selected_cards_from_stage(&mut self, indices: &[usize], count: usize, card_type_filter: Option<&str>,
        cost_limit: Option<u32>, cost_limit_operator: Option<&str>, group: Option<&str>, characters: Option<&Vec<String>>) -> Result<(), String> {
        self.execute_selected_cards_from_zone("stage", indices, count, card_type_filter, cost_limit, cost_limit_operator, group, characters)
    }
    fn execute_selected_looked_at_cards(&mut self, indices: &[usize]) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut("self");
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
        let player = self.game_state.resolve_target_player_mut("self");
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
