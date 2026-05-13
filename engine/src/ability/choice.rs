use super::types::{Choice, ChoiceResult, ExecutionContext, LookAndSelectStep};
use super::util;
use crate::card::AbilityEffect;

impl<'a> super::resolver::AbilityResolver<'a> {
    pub fn resume_execution(&mut self, _context: ExecutionContext) -> Result<(), String> {
        self.execution_context = ExecutionContext::None;
        Ok(())
    }

    pub fn expire_live_end_effects(&mut self) {
        let initial_count = self.duration_effects.len();
        self.duration_effects
            .retain(|(_, duration)| duration != "live_end");
        let expired_count = initial_count - self.duration_effects.len();
        if expired_count > 0 {
            eprintln!("Expired {} effects with duration 'live_end'", expired_count);
        }
    }

    /// Shared epilogue: clear pending_choice, resume execution, process pending sequential actions.
    fn finalize_choice(&mut self, context: &ExecutionContext) -> Result<(), String> {
        let is_actual_looked_at_choice = self
            .pending_choice
            .as_ref()
            .map(|choice| matches!(choice, Choice::SelectCard { zone, .. } if zone == "looked_at"))
            .unwrap_or(false);

        let has_pending_sequential = self.game_state.pending_sequential_actions.is_some();

        // Only preserve if this is the initial looked_at selection (not a sub-action choice).
        // Sub-choices created by pending_sequential_actions should NOT be preserved,
        // otherwise the chain never terminates.
        let is_initial_looked_at = is_actual_looked_at_choice
            && matches!(
                context,
                ExecutionContext::LookAndSelect {
                    step: LookAndSelectStep::Select { .. }
                }
            );
        let should_preserve = is_initial_looked_at && has_pending_sequential;

        if !should_preserve {
            self.pending_choice = None;
        }

        self.resume_execution(context.clone())?;
        if let Some(ref pending) = self.game_state.pending_sequential_actions.clone() {
            for (i, action) in pending.iter().enumerate() {
                self.execute_effect(action)?;
                if self.pending_choice.is_some() {
                    if self.game_state.pending_sequential_actions.is_none() {
                        let remaining = pending[i + 1..].to_vec();
                        self.game_state.pending_sequential_actions = if remaining.is_empty() {
                            None
                        } else {
                            Some(remaining)
                        };
                    }
                    return Ok(());
                }
            }
            self.game_state.pending_sequential_actions = None;
            if should_preserve && self.pending_choice.is_some() {
                self.pending_choice = None;
            }
        }
        Ok(())
    }

    fn reveal_selected_looked_at(&mut self, indices: &[usize]) {
        for &idx in indices.iter() {
            if idx < self.looked_at_cards.len() {
                self.game_state
                    .revealed_cards
                    .push(self.looked_at_cards[idx]);
            }
        }
    }

    pub fn provide_choice_result(&mut self, result: ChoiceResult) -> Result<(), String> {
        let choice = self.pending_choice.clone();
        let context = self.execution_context.clone();
        println!(
            "DEBUG: provide_choice_result - choice: {:?}, result: {:?}",
            choice, result
        );
        match (&choice, result) {
            (
                Some(Choice::SelectCard {
                    zone,
                    card_type,
                    count,
                    description: _,
                    allow_skip,
                    cost_limit,
                    cost_limit_operator,
                    group,
                    characters,
                    filtered_indices,
                    is_select_action,
                }),
                ChoiceResult::CardSelected { indices },
            ) => {
                println!("DEBUG: Processing SelectCard choice - zone: '{}', card_type: {:?}, count: {}, indices: {:?}", zone, card_type, count, indices);
                self.handle_select_card(
                    zone,
                    card_type,
                    *count,
                    *allow_skip,
                    &indices,
                    context,
                    *cost_limit,
                    cost_limit_operator.clone(),
                    group.clone(),
                    characters.clone(),
                    filtered_indices.clone(),
                    *is_select_action,
                )
            }
            (Some(Choice::SelectCard { .. }), ChoiceResult::Skip) => {
                self.pending_choice = None;
                self.resume_execution(context)
            }
            (
                Some(Choice::SelectTarget { target, .. }),
                ChoiceResult::TargetSelected { target: selected },
            ) => self.handle_select_target(target, &selected, context),
            (Some(Choice::SelectPosition { .. }), ChoiceResult::PositionSelected { position }) => {
                self.handle_select_position(&position, context)
            }
            (
                Some(Choice::SelectHeartColor { count, .. }),
                ChoiceResult::HeartColorSelected { colors },
            )
            | (
                Some(Choice::SelectHeartType { count, .. }),
                ChoiceResult::HeartTypeSelected { types: colors },
            ) => self.handle_heart_selection(*count as u32, &colors),
            _ => Err("Choice result does not match pending choice".to_string()),
        }
    }

    fn handle_select_card(
        &mut self,
        zone: &str,
        card_type: &Option<String>,
        count: usize,
        allow_skip: bool,
        indices: &[usize],
        context: ExecutionContext,
        cost_limit: Option<u32>,
        cost_limit_operator: Option<String>,
        group: Option<String>,
        characters: Option<Vec<String>>,
        filtered_indices: Option<Vec<usize>>,
        is_select_action: bool,
    ) -> Result<(), String> {
        println!(
            "DEBUG: handle_select_card - zone: '{}', indices: {:?}, context: {:?}",
            zone, indices, context
        );
        if self
            .game_state
            .entry_cost()
            .and_then(|c| c.cost_type.as_deref())
            == Some("reveal")
        {
            let cost = self.game_state.entry_cost().cloned().unwrap();
            let card_db = self.game_state.card_database.clone();
            let player = self.game_state.active_player();
            let card_ids: Vec<i16> = indices
                .iter()
                .filter_map(|&idx| {
                    if idx < player.hand.cards.len() {
                        let cid = player.hand.cards[idx];
                        let passes =
                            util::card_matches_type(&card_db, cid, cost.card_type.as_deref())
                                && util::card_matches_characters(
                                    &card_db,
                                    cid,
                                    cost.characters.as_ref(),
                                )
                                && match cost.group_names.as_ref() {
                                    Some(groups) => groups.iter().any(|g| {
                                        util::card_matches_group_str(
                                            &card_db,
                                            cid,
                                            Some(g.as_str()),
                                        )
                                    }),
                                    None => true,
                                }
                                && util::card_matches_cost_limit(&card_db, cid, cost.cost_limit);
                        if passes {
                            Some(cid)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
            // Validate that we got enough matching cards
            let count = cost.count.unwrap_or(1) as usize;
            if card_ids.len() < count {
                return Err("Not enough valid cards to reveal for cost".to_string());
            }
            for card_id in card_ids {
                self.game_state.revealed_cards.push(card_id);
                self.revealed_cost_cards.push(card_id);
            }
            return self.finalize_choice(&context);
        }

        let card_db = self.game_state.card_database.clone();
        let validate_card = |cid: i16| -> bool {
            util::card_matches_type(&card_db, cid, card_type.as_deref())
                && util::card_matches_cost_limit_op(
                    &card_db,
                    cid,
                    cost_limit,
                    cost_limit_operator.as_deref(),
                )
                && util::card_matches_group_str(&card_db, cid, group.as_deref())
                && match &characters {
                    Some(chars) if !chars.is_empty() => {
                        util::card_matches_characters(&card_db, cid, Some(chars))
                    }
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
                            if idx < 3
                                && player.stage.stage[idx] != -1
                                && validate_card(player.stage.stage[idx])
                            {
                                if let Some(card_id) =
                                    player.remove_member_from_stage_with_recycling(idx, &card_db)
                                {
                                    player.waitroom.add_card(card_id);
                                    last_vacated = Some(idx);
                                }
                            }
                        }
                    }
                    if let Some(pos) = last_vacated {
                        self.game_state.last_vacated_stage_area = Some(pos);
                    }
                }
                "energy_zone" => {
                    let player = self.game_state.active_player_mut();
                    for &idx in indices.iter().rev() {
                        if idx < player.energy_zone.cards.len()
                            && validate_card(player.energy_zone.cards[idx])
                        {
                            player
                                .waitroom
                                .add_card(player.energy_zone.cards.remove(idx));
                        }
                    }
                }
                "discard" => {
                    if is_select_action {
                        let player = self.game_state.active_player_mut();
                        let mut cards: Vec<i16> = Vec::new();
                        for &i in indices.iter() {
                            if i < player.waitroom.cards.len() {
                                let cid = player.waitroom.cards[i];
                                if validate_card(cid) {
                                    cards.push(cid);
                                }
                            }
                        }
                        self.selected_cards = cards;
                    } else {
                        self.execute_selected_cards_from_zone(
                            "discard",
                            indices,
                            count,
                            card_type.as_deref(),
                            cost_limit,
                            cost_limit_operator.as_deref(),
                            group.as_deref(),
                            characters.as_ref(),
                        )?;
                    }
                }
                "deck" => self.execute_selected_cards_from_zone(
                    "deck",
                    indices,
                    count,
                    card_type.as_deref(),
                    None,
                    None,
                    None,
                    None,
                )?,
                "looked_at" => {
                    eprintln!("[HSC1_LOOKED_AT] START: looked_cards.len()={}, indices={:?}, filtered_indices={:?}, is_select_cards={}",
                        self.looked_at_cards.len(), indices, filtered_indices, self.game_state.ability_queue.current_entry()
                            .and_then(|e| e.ability.effect.as_ref())
                            .and_then(|ef| ef.compound.select_action.as_ref())
                            .map(|sa| {
                                eprintln!("[HSC1_SA] sa.action={:?}, compound.actions.len={:?}", sa.action, sa.compound.actions.as_ref().map(|a| a.len()));
                                sa.action == "select_cards"
                            })
                            .unwrap_or(false));
                    let mapped_indices: Vec<usize> = if let Some(ref fidx) = filtered_indices {
                        indices
                            .iter()
                            .filter_map(|&i| fidx.get(i).copied())
                            .collect()
                    } else {
                        indices.to_vec()
                    };
                    self.reveal_selected_looked_at(&mapped_indices);

                    let entry_effect_sa = || -> Option<Box<AbilityEffect>> {
                        self.game_state
                            .ability_queue
                            .current_entry()
                            .and_then(|e| e.ability.effect.as_ref())
                            .and_then(|ef| ef.compound.select_action.clone())
                    };
                    let is_select_cards = entry_effect_sa()
                        .map(|sa| sa.action == "select_cards" || sa.action == "sequential")
                        .unwrap_or(false);

                    if is_select_cards {
                        self.handle_select_cards_looked_at(&mapped_indices)?;
                        // Return early only for order prompts (any_order), not for multi-selection
                        if matches!(self.pending_choice, Some(Choice::SelectTarget { ref target, .. }) if target == "order")
                        {
                            return Ok(());
                        }
                    } else if let ExecutionContext::LookAndSelect { .. } = self.execution_context {
                        if let Some(ref select_action) = entry_effect_sa() {
                            if select_action.action == "select_cards"
                                || select_action.action == "sequential"
                            {
                                self.handle_select_cards_looked_at(&mapped_indices)?;
                                self.looked_at_cards = self.game_state.looked_at_cards.clone();
                            } else if let Some(ref actions) = select_action.compound.actions {
                                self.game_state.pending_sequential_actions = Some(actions.clone());
                                self.handle_select_cards_looked_at(&mapped_indices)?;
                                self.looked_at_cards = self.game_state.looked_at_cards.clone();
                            } else {
                                self.handle_select_cards_looked_at(&mapped_indices)?;
                                self.looked_at_cards = self.game_state.looked_at_cards.clone();
                            }
                        } else {
                            self.handle_select_cards_looked_at(&mapped_indices)?;
                            self.looked_at_cards = self.game_state.looked_at_cards.clone();
                        }
                    } else {
                        self.handle_select_cards_looked_at(&mapped_indices)?;
                        self.looked_at_cards = self.game_state.looked_at_cards.clone();
                    }

                    // Check if we can select more cards (for "up to X" abilities)
                    if is_select_cards && !mapped_indices.is_empty() {
                        let max_count = count;
                        let selected_count = mapped_indices.len();
                        let remaining = self.game_state.looked_at_cards.len();

                        // If user can select more and there are remaining cards, create new choice
                        if max_count > selected_count && remaining > 0 {
                            let remaining_max = max_count - selected_count;
                            let card_type = card_type.clone();
                            self.pending_choice = Some(Choice::SelectCard {
                                zone: "looked_at".to_string(),
                                card_type,
                                count: remaining_max,
                                description: format!("Select up to {} more card(s) from the {} remaining looked-at cards", remaining_max, remaining),
                                allow_skip: true,
                                cost_limit: None,
                                cost_limit_operator: None,
                                group: None,
                                characters: None,
                                filtered_indices: None,
                                is_select_action: false,
                            });
                            self.execution_context = context.clone();
                            return Ok(());
                        }
                    }

                    return self.finalize_choice(&context);
                }
                "revealed_cards" => {
                    let cards: Vec<i16> = indices
                        .iter()
                        .map(|&i| self.game_state.revealed_cards.remove(i))
                        .collect();
                    self.selected_cards = cards;
                    // Move selected cards to destination
                    let dst = self.game_state.entry_destination().map(|s| s.to_string());
                    let dst_str = dst.as_deref().unwrap_or("hand");
                    let player = self.game_state.active_player_mut();
                    for &cid in &self.selected_cards.clone() {
                        crate::ability::util::place_card_in_zone(
                            player, cid, dst_str, None, false, 1,
                        );
                    }
                    return self.finalize_choice(&context);
                }
                "under_member" => {
                    let player = self.game_state.active_player_mut();
                    let area_order = [
                        crate::zones::MemberArea::Center,
                        crate::zones::MemberArea::LeftSide,
                        crate::zones::MemberArea::RightSide,
                    ];
                    let mut cards_to_move = Vec::new();
                    for &idx in indices.iter() {
                        let mut global_idx = 0;
                        let mut found = false;
                        for area in &area_order {
                            let under = player.stage.get_under_cards(*area);
                            if idx < global_idx + under.len() {
                                let card_id = under[idx - global_idx];
                                cards_to_move.push((*area, card_id));
                                found = true;
                                break;
                            }
                            global_idx += under.len();
                        }
                        if !found {
                            return Err(format!("Card at index {} not found in under_member", idx));
                        }
                    }
                    let _ = player;
                    for (area, card_id) in cards_to_move {
                        let under = &mut self.game_state.player1.stage.under_cards[match area {
                            crate::zones::MemberArea::LeftSide => 0,
                            crate::zones::MemberArea::Center => 1,
                            crate::zones::MemberArea::RightSide => 2,
                        }];
                        if let Some(pos) = under.iter().position(|&c| c == card_id) {
                            under.remove(pos);
                            self.game_state.player1.energy_deck.cards.push(card_id);
                        }
                    }
                    return self.finalize_choice(&context);
                }
                _ => {}
            }
            self.pending_choice = None;
            return Ok(());
        }

        match zone {
            "hand" => {
                let hand_idx = if indices.is_empty() && !allow_skip && count > 0 {
                    &[0usize]
                } else {
                    indices
                };
                if !hand_idx.is_empty() || allow_skip {
                    self.execute_selected_cards_from_zone(
                        "hand",
                        hand_idx,
                        count,
                        card_type.as_deref(),
                        cost_limit,
                        cost_limit_operator.as_deref(),
                        group.as_deref(),
                        characters.as_ref(),
                    )?
                }
                // Track whether optional cost was actually paid
                if allow_skip {
                    if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                        entry.optional_cost_was_paid = !indices.is_empty();
                    }
                }
            }
            "deck" => self.execute_selected_cards_from_zone(
                "deck",
                indices,
                count,
                card_type.as_deref(),
                None,
                None,
                None,
                None,
            )?,
            "discard" => {
                if is_select_action {
                    // Just store card IDs without moving
                    let player = self.game_state.active_player_mut();
                    let mut cards: Vec<i16> = Vec::new();
                    for &i in indices.iter() {
                        if i < player.waitroom.cards.len() {
                            cards.push(player.waitroom.cards[i]);
                        }
                    }
                    self.selected_cards = cards;
                } else {
                    self.execute_selected_cards_from_zone(
                        "discard",
                        indices,
                        count,
                        card_type.as_deref(),
                        cost_limit,
                        cost_limit_operator.as_deref(),
                        group.as_deref(),
                        characters.as_ref(),
                    )?;
                }
            }
            "looked_at" => {
                eprintln!("[LOOKED_AT_DEBUG] handle_select_card looked_at indices={:?}, self.looked_at_cards.len()={}", indices, self.looked_at_cards.len());
                self.reveal_selected_looked_at(indices);
                let has_compound = self
                    .game_state
                    .ability_queue
                    .current_entry()
                    .and_then(|e| e.ability.effect.as_ref())
                    .and_then(|ef| ef.compound.select_action.as_ref())
                    .map(|sa| sa.action == "select_cards" || sa.action == "sequential")
                    .unwrap_or(false);

                self.handle_select_cards_looked_at(indices)?;
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
                // Move selected cards to destination
                let dst = self.game_state.entry_destination().map(|s| s.to_string());
                let dst_str = dst.as_deref().unwrap_or("hand");
                let player = self.game_state.active_player_mut();
                for &cid in &self.selected_cards.clone() {
                    crate::ability::util::place_card_in_zone(player, cid, dst_str, None, false, 1);
                }
            }
            "energy_zone" => self.execute_selected_energy_zone_cards(indices, count)?,
            "selected_cards" => {
                let mut cards = Vec::new();
                for &i in indices.iter() {
                    if i < self.selected_cards.len() {
                        cards.push(self.selected_cards[i]);
                    }
                }
                self.selected_cards = cards;
            }
            "under_member" => {
                let player = self.game_state.active_player_mut();
                let area_order = [
                    crate::zones::MemberArea::Center,
                    crate::zones::MemberArea::LeftSide,
                    crate::zones::MemberArea::RightSide,
                ];
                let mut cards_to_move = Vec::new();
                for &idx in indices.iter() {
                    let mut global_idx = 0;
                    let mut found = false;
                    for area in &area_order {
                        let under = player.stage.get_under_cards(*area);
                        if idx < global_idx + under.len() {
                            let card_id = under[idx - global_idx];
                            cards_to_move.push((*area, card_id));
                            found = true;
                            break;
                        }
                        global_idx += under.len();
                    }
                    if !found {
                        return Err(format!("Card at index {} not found in under_member", idx));
                    }
                }
                let _ = player;
                for (area, card_id) in cards_to_move {
                    let under = &mut self.game_state.player1.stage.under_cards[match area {
                        crate::zones::MemberArea::LeftSide => 0,
                        crate::zones::MemberArea::Center => 1,
                        crate::zones::MemberArea::RightSide => 2,
                    }];
                    if let Some(pos) = under.iter().position(|&c| c == card_id) {
                        under.remove(pos);
                        self.game_state.player1.energy_deck.cards.push(card_id);
                    }
                }
            }
            _ => eprintln!("Card selection from zone '{}' not yet implemented", zone),
        }
        if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
            entry.selected_card_ids = self.selected_cards.clone();
        }
        self.finalize_choice(&context)
    }

    fn handle_select_target(
        &mut self,
        target: &str,
        selected: &str,
        _context: ExecutionContext,
    ) -> Result<(), String> {
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

        if choice_card_no
            .as_deref()
            .map(|s| s.starts_with("position_change"))
            .unwrap_or(false)
        {
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
            self.game_state
                .prohibition_effects
                .push(format!("chosen_required_hearts:{}", selected));
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
                    player,
                    count as u32,
                    source,
                    destination,
                    card_type,
                    false,
                    None,
                    &card_db,
                    None,
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
                        if idx < self.game_state.looked_at_cards.len() {
                            let card = self.game_state.looked_at_cards.remove(idx);
                            self.game_state.looked_at_cards.insert(0, card);
                        }
                    }
                    let card_ids: Vec<i16> = self
                        .game_state
                        .looked_at_cards
                        .iter()
                        .rev()
                        .copied()
                        .collect();
                    let player = self.game_state.active_player_mut();
                    for card_id in card_ids {
                        player.main_deck.cards.insert(0, card_id);
                    }
                    self.game_state.looked_at_cards.clear();
                    self.looked_at_cards.clear();
                }
            }
        }
        self.pending_choice = None;
        Ok(())
    }

    fn handle_choice_string_selection(
        &mut self,
        selected: &str,
        conditional_choice: Option<String>,
    ) -> Result<(), String> {
        if let Some(ref options_json) = conditional_choice {
            if let Ok(options) = serde_json::from_str::<Vec<String>>(options_json) {
                if let Ok(idx) = selected.parse::<usize>() {
                    if idx > 0 && idx <= options.len() {
                        let val = &options[idx - 1];
                        if val.starts_with("heart")
                            || ["赤", "桃", "緑", "青", "黄", "紫"].contains(&val.as_str())
                        {
                            self.game_state
                                .prohibition_effects
                                .push(format!("selected_heart_color:{}", val));
                        }
                    }
                }
            }
        }
        self.pending_choice = None;
        self.clear_choice_meta();
        Ok(())
    }

    fn handle_position_change_choice(
        &mut self,
        choice_card_no: Option<String>,
        selected: &str,
    ) -> Result<(), String> {
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
                    if tgt.contains(':') {
                        let parts: Vec<&str> = tgt.splitn(2, ':').collect();
                        modified.target = Some(parts[0].to_string());
                        modified.target_member = Some(parts[1].to_string());
                    } else {
                        modified.target = Some(tgt.to_string());
                    }
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
            for (i, action) in pending.iter().enumerate() {
                if let Err(e) = self.execute_effect(action) {
                    eprintln!(
                        "Failed to execute pending action after position change: {}",
                        e
                    );
                }
                if self.pending_choice.is_some() {
                    let remaining = pending[i + 1..].to_vec();
                    self.game_state.pending_sequential_actions = if remaining.is_empty() {
                        None
                    } else {
                        Some(remaining)
                    };
                    return Ok(());
                }
            }
            self.game_state.pending_sequential_actions = None;
        }
        Ok(())
    }

    fn handle_choice_string_store(
        &mut self,
        selected: &str,
        conditional_choice: Option<String>,
    ) -> Result<(), String> {
        let chosen = conditional_choice.and_then(|json| {
            serde_json::from_str::<Vec<String>>(&json)
                .ok()
                .and_then(|opts| {
                    selected
                        .parse::<usize>()
                        .ok()
                        .and_then(|idx| opts.get(idx).cloned())
                })
        });
        if let Some(s) = chosen {
            self.game_state
                .ability_queue
                .current_entry_mut()
                .map(|e| e.conditional_choice = Some(s));
        }
        self.pending_choice = None;
        if let Some(ref pending) = self.game_state.pending_sequential_actions.clone() {
            for (i, action) in pending.iter().enumerate() {
                self.execute_effect(action)?;
                if self.pending_choice.is_some() {
                    let remaining = pending[i + 1..].to_vec();
                    self.game_state.pending_sequential_actions = if remaining.is_empty() {
                        None
                    } else {
                        Some(remaining)
                    };
                    return Ok(());
                }
            }
            self.game_state.pending_sequential_actions = None;
        }
        Ok(())
    }

    fn handle_optional_cost_payment(&mut self, selected: &str) -> Result<(), String> {
        if selected == "skip_optional_cost" || selected == "0" {
            self.pending_choice = None;
            if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                entry.cost_paid = true;
                entry.optional_cost_was_paid = false;
            }
            return Ok(());
        }
        // "pay_optional_cost" or "1" from select_option(1)
        if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
            entry.optional_cost_was_paid = true;
        }
        let is_pay = true;
        if is_pay {
            if let Some(cost) = self.game_state.entry_cost().cloned() {
                if let Some(energy) = cost.energy {
                    if energy > 0 {
                        let tgt = cost.target.as_deref().unwrap_or("self");
                        self.game_state
                            .resolve_target_player_mut(tgt)
                            .energy_zone
                            .pay_energy(energy as usize)
                            .map_err(|e| e)?;
                    }
                }
                if cost.state_change.as_deref() == Some("wait") && cost.self_cost == Some(true) {
                    if let Some(id) = self.game_state.activating_card {
                        self.game_state.mods.add_orientation_modifier(id, "wait");
                    }
                }
                eprintln!("[OPT_COST] checking cost_type: {:?}, entry_cost: {:?}, entry_effect_action: {:?}", 
                    cost.cost_type, self.game_state.entry_cost().is_some(), 
                    self.game_state.entry_effect().map(|e| e.action.clone()));
                if cost.cost_type.as_deref() == Some("place_energy_under_member") {
                    self.execute_place_energy_under_member(
                        cost.count.unwrap_or(1),
                        cost.target.as_deref().unwrap_or("self"),
                        cost.position.as_ref(),
                        false,
                        cost.source.as_deref(),
                    );
                }
            }
            self.pending_choice = None;
            let is_effect_optional =
                self.game_state.entry_choice_card_no().as_deref() == Some("optional_cost");
            if self.game_state.entry_cost().is_some() {
                if let Some(effect) = self.game_state.entry_effect().cloned() {
                    if let Err(e) = self.execute_effect(&effect) {
                        eprintln!("Failed to execute effect after optional cost: {}", e);
                    }
                    if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                        entry.effect_started = true;
                    }
                }
            } else if let Some(ref pending) = self.game_state.pending_sequential_actions.clone() {
                if !pending.is_empty() {
                    for action in pending {
                        if let Err(e) = self.execute_effect(action) {
                            eprintln!("Failed to execute action after optional: {}", e);
                        }
                    }
                    self.game_state.pending_sequential_actions = None;
                }
            } else if is_effect_optional {
                if let Some(effect) = self.game_state.entry_effect().cloned() {
                    let new_count = effect.energy_count.unwrap_or(effect.count_or(1));
                    self.execute_place_energy_under_member(
                        new_count,
                        effect.target_name(),
                        effect.position.as_ref(),
                        false,
                        effect.source.as_deref(),
                    );
                }
            }
        }
        Ok(())
    }

    fn handle_primary_alternative(&mut self, selected: &str) -> Result<(), String> {
        if let Some(ref ability) = self.current_ability.clone() {
            if let Some(ref effect) = ability.effect {
                if effect.action == "conditional_alternative" {
                    match selected {
                        "primary" => {
                            if let Some(ref p) = effect.compound.primary_effect {
                                if let Err(e) = self.execute_effect(p) {
                                    eprintln!("Failed to execute primary: {}", e);
                                }
                            }
                        }
                        "alternative" => {
                            if let Some(ref a) = effect.compound.alternative_effect {
                                if let Err(e) = self.execute_effect(a) {
                                    eprintln!("Failed to execute alternative: {}", e);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        self.pending_choice = None;
        Ok(())
    }

    fn handle_position_destination(&mut self, selected: &str) -> Result<(), String> {
        let dest = match selected {
            "0" | "left" => "left_side",
            "1" | "center" => "center",
            "2" | "right" => "right_side",
            _ => "center",
        };
        if let Some(ref ability) = self.current_ability.clone() {
            if let Some(ref effect) = ability.effect {
                if effect.action == "position_change" {
                    if let Err(e) = self.execute_position_change_with_destination(effect, dest) {
                        eprintln!("Failed position change: {}", e);
                    }
                }
            }
        }
        self.pending_choice = None;
        if let Some(ref pending) = self.game_state.pending_sequential_actions.clone() {
            for (i, action) in pending.iter().enumerate() {
                if let Err(e) = self.execute_effect(action) {
                    eprintln!(
                        "Failed to execute pending action after position change: {}",
                        e
                    );
                }
                if self.pending_choice.is_some() {
                    let remaining = pending[i + 1..].to_vec();
                    self.game_state.pending_sequential_actions = if remaining.is_empty() {
                        None
                    } else {
                        Some(remaining)
                    };
                    return Ok(());
                }
            }
            self.game_state.pending_sequential_actions = None;
        }
        Ok(())
    }

    fn handle_heart_color_selection(&mut self, selected: &str) -> Result<(), String> {
        const HEART_VALS: [&str; 7] = [
            "heart00", "heart01", "heart02", "heart03", "heart04", "heart05", "heart06",
        ];
        let idx: usize = selected.parse().unwrap_or(0);
        if idx < HEART_VALS.len() {
            self.game_state
                .prohibition_effects
                .push(format!("selected_heart_color:{}", HEART_VALS[idx]));
        }
        self.pending_choice = None;
        Ok(())
    }

    fn handle_choice_condition(&mut self, selected: &str) -> Result<(), String> {
        let idx: usize = selected.parse().unwrap_or(0);
        if let Some(options) = self.game_state.entry_cost().and_then(|c| c.options.clone()) {
            if idx < options.len() {
                if let Err(e) = self.pay_cost(&options[idx]) {
                    eprintln!("Failed to pay cost option: {}", e);
                }
            }
        }
        self.pending_choice = None;
        Ok(())
    }

    fn handle_conditional_optional(&mut self, selected: &str) -> Result<(), String> {
        let effect = self.game_state.entry_effect().cloned();
        self.pending_choice = None;
        let is_negation = effect
            .as_ref()
            .map(|e| e.compound.conditional_negation.unwrap_or(false))
            .unwrap_or(false);
        if selected == "1" || selected == "yes" {
            if let Some(ref e) = effect {
                if let Some(ref o) = e.compound.optional_action {
                    if let Err(err) = self.execute_effect(o) {
                        eprintln!("Failed optional action: {}", err);
                    }
                }
                if !is_negation {
                    if let Some(ref c) = e.compound.conditional_action {
                        if let Err(err) = self.execute_effect(c) {
                            eprintln!("Failed conditional action: {}", err);
                        }
                    }
                }
            }
        } else if is_negation {
            if let Some(ref e) = effect {
                if let Some(ref c) = e.compound.conditional_action {
                    if let Err(err) = self.execute_effect(c) {
                        eprintln!("Failed conditional action: {}", err);
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_select_position(
        &mut self,
        position: &str,
        context: ExecutionContext,
    ) -> Result<(), String> {
        match &context {
            ExecutionContext::LookAndSelect { step } => {
                if let LookAndSelectStep::Finalize { destination } = step {
                    if destination == "stage" {
                        if let Some(&card_id) = self.looked_at_cards.last() {
                            let player = &mut self.game_state.player1;
                            match position {
                                "center" => {
                                    player.stage.stage[1] = card_id;
                                    player
                                        .areas_locked_this_turn
                                        .insert(crate::zones::MemberArea::Center);
                                }
                                "left_side" => {
                                    player.stage.stage[0] = card_id;
                                    player
                                        .areas_locked_this_turn
                                        .insert(crate::zones::MemberArea::LeftSide);
                                }
                                "right_side" => {
                                    player.stage.stage[2] = card_id;
                                    player
                                        .areas_locked_this_turn
                                        .insert(crate::zones::MemberArea::RightSide);
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
            ExecutionContext::MoveCardsPosition {
                card_id,
                state_change,
                target,
            } => {
                let player = match target.as_str() {
                    "opponent" => &mut self.game_state.player2,
                    _ => &mut self.game_state.player1,
                };
                let pos = match position {
                    "center" => 1,
                    "left_side" => 0,
                    "right_side" => 2,
                    _ => {
                        player.hand.add_card(*card_id);
                        return Ok(());
                    }
                };
                if pos < 3 && player.stage.stage[pos] == -1 {
                    player.stage.stage[pos] = *card_id;
                    let area = match pos {
                        0 => crate::zones::MemberArea::LeftSide,
                        1 => crate::zones::MemberArea::Center,
                        2 => crate::zones::MemberArea::RightSide,
                        _ => crate::zones::MemberArea::Center,
                    };
                    player.areas_locked_this_turn.insert(area);
                } else {
                    player.hand.add_card(*card_id);
                }
                // Apply state_change and record movement (same as move_cards executor)
                self.game_state.mods.clear_all_for_card(*card_id);
                self.game_state.record_card_movement(*card_id);
                if let Some(ref sc) = state_change {
                    if sc == "wait" {
                        self.game_state
                            .mods
                            .add_orientation_modifier(*card_id, "wait");
                    }
                }
            }
            _ => {}
        }
        self.pending_choice = None;
        self.execution_context = ExecutionContext::None;

        // Process any pending sequential actions (e.g., target="both" deferred opponent)
        if let Some(ref pending) = self.game_state.pending_sequential_actions.clone() {
            for (i, action) in pending.iter().enumerate() {
                self.execute_effect(action)?;
                if self.pending_choice.is_some() {
                    let remaining = pending[i + 1..].to_vec();
                    self.game_state.pending_sequential_actions = if remaining.is_empty() {
                        None
                    } else {
                        Some(remaining)
                    };
                    return Ok(());
                }
            }
            self.game_state.pending_sequential_actions = None;
        }

        Ok(())
    }

    fn handle_heart_selection(&mut self, count: u32, colors: &[String]) -> Result<(), String> {
        if let Some(chosen) = colors.first() {
            let color = crate::zones::parse_heart_color(chosen);
            if let Some(card_id) = self.game_state.activating_card {
                self.game_state
                    .set_heart_override(card_id, color, count.max(1), "live_end");
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

    fn execute_selected_cards_from_zone(
        &mut self,
        zone: &str,
        indices: &[usize],
        _count: usize,
        card_type_filter: Option<&str>,
        cost_limit: Option<u32>,
        cost_limit_operator: Option<&str>,
        group: Option<&str>,
        characters: Option<&Vec<String>>,
    ) -> Result<(), String> {
        eprintln!("[EXEC_ZONE] enter zone={} indices={:?}", zone, indices);
        let destination = self.game_state.entry_destination().map(|s| s.to_string());
        let target = self
            .game_state
            .entry_effect()
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
                    Some(chars) if !chars.is_empty() => {
                        util::card_matches_characters(&card_db, cid, Some(chars))
                    }
                    _ => true,
                }
        };

        match zone {
            "hand" => {
                let dest = destination.as_deref().unwrap_or("discard");
                let mut idxs: Vec<usize> = indices.iter().copied().collect();
                idxs.sort_by(|a, b| b.cmp(a));
                for i in idxs {
                    if i < player.hand.cards.len() {
                        let card_id = player.hand.cards.remove(i);
                        if passes(card_id) {
                            match dest {
                                "stage" => {
                                    if player.stage.stage[1] == -1 {
                                        player.stage.stage[1] = card_id;
                                        player
                                            .areas_locked_this_turn
                                            .insert(crate::zones::MemberArea::Center);
                                    } else if player.stage.stage[0] == -1 {
                                        player.stage.stage[0] = card_id;
                                        player
                                            .areas_locked_this_turn
                                            .insert(crate::zones::MemberArea::LeftSide);
                                    } else if player.stage.stage[2] == -1 {
                                        player.stage.stage[2] = card_id;
                                        player
                                            .areas_locked_this_turn
                                            .insert(crate::zones::MemberArea::RightSide);
                                    } else {
                                        player.hand.add_card(card_id);
                                    }
                                }
                                "same_area" => {
                                    super::util::place_card_in_zone(
                                        player,
                                        card_id,
                                        "same_area",
                                        vacated_area,
                                        false,
                                        1,
                                    );
                                }
                                _ => player.waitroom.add_card(card_id),
                            }
                            moved.push(card_id);
                        } else {
                            player.hand.cards.insert(i, card_id);
                        }
                    }
                }
            }
            "deck" => {
                let mut idxs: Vec<usize> = indices.iter().copied().collect();
                idxs.sort_by(|a, b| b.cmp(a));
                for i in idxs {
                    if i < player.main_deck.cards.len() {
                        let card_id = player.main_deck.cards.remove(i);
                        if passes(card_id) {
                            player.hand.add_card(card_id);
                            moved.push(card_id);
                        } else {
                            player.main_deck.cards.insert(i, card_id);
                        }
                    }
                }
            }
            "discard" => {
                let dest = destination.as_deref().unwrap_or("hand");
                // Pre-flight: if destination is stage and no room, silently reject
                if dest == "stage" && player.stage.stage.iter().all(|&id| id != -1) {
                    eprintln!("[DISCARD_ZONE] stage is full, cannot place cards from discard");
                    return Ok(());
                }
                let mut idxs: Vec<usize> = indices.iter().copied().collect();
                idxs.sort_by(|a, b| b.cmp(a));
                eprintln!(
                    "[DISCARD_ZONE] indices={:?}, dest={}, discard_before_len={}",
                    indices,
                    dest,
                    player.waitroom.cards.len()
                );
                // Validate total cost limit before moving cards
                if let Some(limit) = cost_limit {
                    let total_cost: u32 = idxs
                        .iter()
                        .filter_map(|&i| {
                            if i < player.waitroom.cards.len() {
                                card_db
                                    .get_card(player.waitroom.cards[i])
                                    .and_then(|c| c.cost)
                            } else {
                                None
                            }
                        })
                        .sum();
                    let op = cost_limit_operator.unwrap_or("<=");
                    let ok = match op {
                        ">=" => total_cost >= limit,
                        ">" => total_cost > limit,
                        "<" => total_cost < limit,
                        "exact" | "=" => total_cost == limit,
                        _ => total_cost <= limit,
                    };
                    eprintln!(
                        "[DISCARD_ZONE] total_cost={}, limit={}, op={}, ok={}",
                        total_cost, limit, op, ok
                    );
                    if !ok {
                        eprintln!(
                            "[TOTAL_COST] Selection rejected: total={} limit={} op={}",
                            total_cost, limit, op
                        );
                        return Ok(());
                    }
                }
                let mut card_ids_moved: Vec<i16> = Vec::new();
                for i in idxs {
                    if i < player.waitroom.cards.len() {
                        let card_id = player.waitroom.cards.remove(i);
                        eprintln!("[DISCARD_ZONE] removing idx={} card_id={}", i, card_id);
                        if passes(card_id) {
                            match dest {
                                "stage" => {
                                    if player.stage.stage[1] == -1 {
                                        player.stage.stage[1] = card_id;
                                    } else if player.stage.stage[0] == -1 {
                                        player.stage.stage[0] = card_id;
                                    } else if player.stage.stage[2] == -1 {
                                        player.stage.stage[2] = card_id;
                                    } else {
                                        // Stage full — return card to discard
                                        player.waitroom.cards.insert(i, card_id);
                                        eprintln!("[DISCARD_ZONE] no stage room, returning card to discard");
                                        continue;
                                    }
                                }
                                "same_area" => {
                                    super::util::place_card_in_zone(
                                        player,
                                        card_id,
                                        "same_area",
                                        vacated_area,
                                        false,
                                        1,
                                    );
                                }
                                _ => player.hand.add_card(card_id),
                            }
                            card_ids_moved.push(card_id);
                            moved.push(card_id);
                        } else {
                            player.waitroom.cards.insert(i, card_id);
                        }
                    }
                }
                // Apply state_change to moved cards (e.g. "wait" state)
                let state_change = self
                    .game_state
                    .ability_queue
                    .current_entry()
                    .and_then(|e| e.ability.effect.as_ref())
                    .and_then(|ef| ef.state_change.clone());
                if let Some(sc) = state_change {
                    if sc == "wait" {
                        for &cid in &card_ids_moved {
                            self.game_state.mods.add_orientation_modifier(cid, "wait");
                        }
                    }
                }
            }
            "stage" => {
                for &idx in indices {
                    if idx < 3 && player.stage.stage[idx] != -1 {
                        let card_id = player.stage.stage[idx];
                        if passes(card_id) {
                            self.selected_cards.push(card_id);
                        }
                    }
                }
            }
            "revealed_cards" => {
                for &idx in indices.iter().rev() {
                    if idx < self.game_state.revealed_cards.len() {
                        let card_id = self.game_state.revealed_cards.remove(idx);
                        if passes(card_id) {
                            self.selected_cards.push(card_id);
                        }
                    }
                }
            }
            _ => return Err(format!("Unknown zone: {}", zone)),
        }
        for cid in moved {
            self.game_state.mods.clear_all_for_card(cid);
        }
        Ok(())
    }

    fn handle_select_cards_looked_at(&mut self, indices: &[usize]) -> Result<(), String> {
        let select_action = self
            .game_state
            .ability_queue
            .current_entry()
            .and_then(|e| e.ability.effect.as_ref())
            .and_then(|ef| ef.compound.select_action.clone());
        let is_sequential = select_action
            .as_ref()
            .map(|sa| sa.action == "sequential")
            .unwrap_or(false);
        let (destination, discard_remaining, placement_order) = if is_sequential {
            let first_move = select_action
                .as_ref()
                .and_then(|sa| sa.compound.actions.as_ref())
                .and_then(|actions| {
                    actions.iter().find(|a| {
                        a.action == "move_cards" && a.source.as_deref() == Some("looked_at")
                    })
                });
            (
                first_move
                    .and_then(|a| a.destination.clone())
                    .unwrap_or_else(|| "hand".to_string()),
                first_move.and_then(|a| a.discard_remaining).unwrap_or(true),
                first_move.and_then(|a| a.placement_order.clone()),
            )
        } else {
            (
                select_action
                    .as_ref()
                    .and_then(|sa| sa.destination.clone())
                    .unwrap_or_else(|| "hand".to_string()),
                select_action
                    .as_ref()
                    .and_then(|sa| sa.discard_remaining)
                    .unwrap_or(true),
                select_action
                    .as_ref()
                    .and_then(|sa| sa.placement_order.clone()),
            )
        };
        println!("DEBUG: handle_select_cards_looked_at - destination: {}, discard_remaining: {}, placement_order: {:?}", destination, discard_remaining, placement_order);

        // If game_state.looked_at_cards was reset (resolver recreation), restore from selected_cards
        if self.game_state.looked_at_cards.is_empty() && !self.selected_cards.is_empty() {
            self.game_state.looked_at_cards = self.selected_cards.clone();
        }

        // Use game_state.looked_at_cards directly
        let looked_at = &mut self.game_state.looked_at_cards;
        let mut indices_sorted: Vec<usize> = indices.iter().copied().collect();
        indices_sorted.sort_by(|a, b| b.cmp(a));

        // Extract selected cards first
        let mut selected_cards: Vec<i16> = Vec::new();
        for i in indices_sorted {
            if i < looked_at.len() {
                selected_cards.insert(0, looked_at.remove(i));
            }
        }
        let selected_count = selected_cards.len();
        println!(
            "DEBUG: Selected {} cards: {:?}",
            selected_count, selected_cards
        );

        // Extract remaining cards
        let remaining_cards: Vec<i16> = looked_at.drain(..).collect();
        println!(
            "DEBUG: Remaining {} cards: {:?}",
            remaining_cards.len(),
            remaining_cards
        );

        // Handle any_order: if placing 2+ cards on deck, prompt for order
        let is_deck_dest = destination == "deck_top" || destination == "deck";
        let needs_order = is_deck_dest
            && placement_order.as_deref() == Some("any_order")
            && selected_cards.len() > 1;

        if needs_order {
            self.looked_at_cards = selected_cards;
            let player = self.game_state.active_player_mut();
            if discard_remaining {
                for card_id in remaining_cards {
                    player.waitroom.add_card(card_id);
                }
            } else {
                for card_id in remaining_cards {
                    player.main_deck.cards.push(card_id);
                }
            }
            let card_count = self.looked_at_cards.len();
            self.pending_choice = Some(Choice::SelectTarget {
                target: "order".to_string(),
                description: format!("Choose order for cards on deck ({} cards)", card_count),
                allow_skip: false,
            });
            self.execution_context = ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Finalize {
                    destination: "deck".to_string(),
                },
            };
            return Ok(());
        }

        // Move selected cards to destination
        let player = self.game_state.active_player_mut();
        for card_id in selected_cards {
            match destination.as_str() {
                "hand" => {
                    println!("DEBUG: Adding card {} to hand", card_id);
                    player.hand.add_card(card_id)
                }
                "deck_top" | "deck" => player.main_deck.cards.insert(0, card_id),
                "discard" => player.waitroom.add_card(card_id),
                _ => player.hand.add_card(card_id),
            }
        }
        let _ = player;
        println!(
            "DEBUG: Hand now has {} cards: {:?}",
            self.game_state.active_player().hand.cards.len(),
            self.game_state.active_player().hand.cards
        );

        // Check for multi-selection: if user selected at least 1 but fewer than max,
        // keep remaining for another pick. If they selected 0, treat as "done".
        let max_select = select_action.as_ref().and_then(|sa| sa.count).unwrap_or(1) as usize;
        let is_max_or_optional = select_action
            .as_ref()
            .map(|sa| sa.max.unwrap_or(false) || sa.optional.unwrap_or(false))
            .unwrap_or(false);

        if selected_count > 0
            && is_max_or_optional
            && max_select > selected_count
            && !remaining_cards.is_empty()
        {
            // Persist remaining cards in both game_state and queue entry for next resolver
            self.game_state.looked_at_cards = remaining_cards.clone();
            if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                entry.selected_card_ids = remaining_cards;
            }
            let remaining_available = self.game_state.looked_at_cards.len();
            let remaining_selections = (max_select - selected_count).min(remaining_available);
            let description = format!(
                "Select up to {} more card(s) from the {} remaining looked-at cards",
                remaining_selections, remaining_available
            );
            self.pending_choice = Some(Choice::SelectCard {
                zone: "looked_at".to_string(),
                card_type: select_action.as_ref().and_then(|sa| sa.card_type.clone()),
                count: remaining_selections,
                description,
                allow_skip: true,
                cost_limit: None,
                cost_limit_operator: None,
                group: None,
                characters: None,
                filtered_indices: None,
                is_select_action: false,
            });
            return Ok(());
        }

        // Handle remaining cards — discard or return to deck
        if discard_remaining {
            for card_id in remaining_cards {
                self.game_state
                    .active_player_mut()
                    .waitroom
                    .add_card(card_id);
            }
        } else {
            for card_id in remaining_cards {
                self.game_state
                    .active_player_mut()
                    .main_deck
                    .cards
                    .push(card_id);
            }
        }

        self.looked_at_cards = self.game_state.looked_at_cards.clone();
        Ok(())
    }
    fn execute_selected_energy_zone_cards(
        &mut self,
        indices: &[usize],
        _count: usize,
    ) -> Result<(), String> {
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
