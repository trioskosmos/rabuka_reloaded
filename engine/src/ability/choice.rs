use super::types::{Choice, ChoiceResult, ExecutionContext, LookAndSelectStep};
use super::util;
use crate::ability::types::Command;
use crate::card::AbilityEffect;

impl<'a> super::resolver::AbilityResolver<'a> {
    pub fn resume_execution(&mut self, context: ExecutionContext) -> Result<(), String> {
        // Clear the execution context if finalizing a look_and_select — otherwise the
        // caller's saved_ctx check (actions.rs:369) prevents process_current_ability.
        if matches!(context, ExecutionContext::LookAndSelect { .. })
            && self.pending_choice.is_none()
        {
            self.execution_context = ExecutionContext::None;
        }
        Ok(())
    }

    /// Resumes executing sequential commands parked on the current queue entry.
    /// If another choice interrupts execution, the remaining actions are safely parked back.
    pub fn resume_pending_commands(&mut self) -> Result<(), String> {
        let pending = self.game_state.ability_queue.take_pending_commands();
        for (i, command) in pending.iter().enumerate() {
            match command {
                Command::Effect(effect) => {
                    self.execute_effect(effect)?;
                }
                Command::MoveCard {
                    card_id,
                    destination,
                    target,
                    state_change,
                } => {
                    let card_id = *card_id;
                    let target = target.clone();
                    let destination = destination.clone();
                    let state_change = state_change.clone();

                    let player = self.game_state.resolve_target_player_mut(&target);
                    super::util::place_card_in_zone(player, card_id, &destination, None, false, 1);

                    self.game_state.mods.clear_all_for_card(card_id);
                    self.game_state.record_card_movement(card_id);
                    if state_change.as_deref() == Some("wait") {
                        self.game_state
                            .mods
                            .add_orientation_modifier(card_id, "wait");
                    }
                }
                Command::Choice(choice) => {
                    self.pending_choice = Some(choice.clone());
                }
            }

            if self.pending_choice.is_some() {
                if i + 1 < pending.len() {
                    let mut existing = self.game_state.ability_queue.take_pending_commands();
                    existing.extend(pending[i + 1..].to_vec());
                    self.game_state.ability_queue.set_pending_commands(existing);
                }
                return Ok(());
            }
        }
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

        let has_pending_sequential = self.game_state.ability_queue.has_pending_commands();

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
        eprintln!(
            "[FINALIZE_CHOICE] pending={} selected={:?} context={:?}",
            has_pending_sequential, self.selected_cards, context
        );
        self.resume_pending_commands()?;
        Ok(())
    }

    fn reveal_selected_looked_at(&mut self, indices: &[usize]) {
        let mut revealed_ids = Vec::new();
        for &idx in indices.iter() {
            if idx < self.looked_at_cards.len() {
                let cid = self.looked_at_cards[idx];
                self.game_state.revealed_cards.push(cid);
                revealed_ids.push(cid);
            }
        }
        if !revealed_ids.is_empty() {
            let card_db = &self.game_state.card_database;
            let names: Vec<String> = revealed_ids
                .iter()
                .filter_map(|id| card_db.get_card(*id))
                .map(|c| c.name.clone())
                .collect();
            if !names.is_empty() {
                let turn = self.game_state.turn_number;
                self.game_state.rule_log.push(format!(
                    "[Turn {}] P{} reveals {} from looked-at cards",
                    turn,
                    if std::ptr::eq(self.game_state.active_player(), &self.game_state.player1) {
                        1
                    } else {
                        2
                    },
                    names.join(", ")
                ));
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
                    ref target_player_id,
                    ..
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
                    target_player_id.clone(),
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
        target_player_id: Option<String>,
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
            if !card_ids.is_empty() {
                let card_db = &self.game_state.card_database;
                let names: Vec<String> = card_ids
                    .iter()
                    .filter_map(|id| card_db.get_card(*id))
                    .map(|c| c.name.clone())
                    .collect();
                let turn = self.game_state.turn_number;
                let player_num =
                    if std::ptr::eq(self.game_state.active_player(), &self.game_state.player1) {
                        1
                    } else {
                        2
                    };
                if !names.is_empty() {
                    self.game_state.rule_log.push(format!(
                        "[Turn {}] P{} reveals {} from hand as cost",
                        turn,
                        player_num,
                        names.join(", ")
                    ));
                }
            }
            for card_id in card_ids {
                self.game_state.revealed_cards.push(card_id);
                self.revealed_cost_cards.push(card_id);
            }
            return self.finalize_choice(&context);
        }

        let card_db = self.game_state.card_database.clone();
        let validate_filter = util::CardFilter {
            card_type: card_type.as_deref(),
            group: group.as_deref(),
            groups: None,
            cost_limit,
            cost_operator: cost_limit_operator.as_deref(),
            characters: characters.as_ref(),
            exclude_characters: None,
            heart_colors: &[],
            name_fragments: None,
            distinct: None,
            exclude_self: None,
            original_blade_limit: None,
            original_blade_operator: None,
        };
        let mut validate_card =
            |cid: i16| -> bool { validate_filter.matches(&card_db, cid, false) };

        if allow_skip && !indices.is_empty() {
            match zone {
                "hand" => {
                    self.discard_from_hand(indices, &mut validate_card);
                }
                "stage" => {
                    self.remove_from_stage(indices, &mut validate_card, &card_db);
                }
                "energy_zone" => {
                    self.mark_energy_as_wait(indices, &mut validate_card);
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
                    cost_limit,
                    cost_limit_operator.as_deref(),
                    group.as_deref(),
                    characters.as_ref(),
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

                    let select_action_entry = self
                        .game_state
                        .ability_queue
                        .current_entry()
                        .and_then(|e| e.ability.effect.as_ref())
                        .and_then(|ef| ef.compound.select_action.clone());
                    let is_select_cards = select_action_entry
                        .as_ref()
                        .map(|sa| sa.action == "select_cards")
                        .unwrap_or(false);

                    if is_select_cards {
                        self.handle_select_cards_looked_at(&mapped_indices)?;
                        if matches!(self.pending_choice, Some(Choice::SelectTarget { ref target, .. }) if target == "order")
                        {
                            return Ok(());
                        }
                    } else if let ExecutionContext::LookAndSelect { .. } = self.execution_context {
                        if let Some(_) = select_action_entry {}
                        self.handle_select_cards_looked_at(&mapped_indices)?;
                        self.looked_at_cards = self.game_state.looked_at_cards.clone();
                    } else {
                        self.handle_select_cards_looked_at(&mapped_indices)?;
                        self.looked_at_cards = self.game_state.looked_at_cards.clone();
                    }

                    // Check if we can select more cards (for round-based "up to X" abilities).
                    if is_select_cards && !mapped_indices.is_empty() {
                        let _any_number = select_action_entry
                            .as_ref()
                            .and_then(|sa| sa.any_number)
                            .unwrap_or(false);
                        let max_count = count;
                        let selected_count = mapped_indices.len();
                        let remaining = self.game_state.looked_at_cards.len();

                        // If user can select more and there are remaining cards, create new choice
                        if max_count > selected_count && remaining > 0 {
                            let remaining_max = max_count - selected_count;
                            let card_type = card_type.clone();
                            self.pending_choice = Some(Choice::select_cards(
                                "looked_at",
                                remaining_max,
                                format!("Select up to {} more card(s) from the {} remaining looked-at cards", remaining_max, remaining),
                                true,
                            )
                            .card_type(card_type)
                            .cost_limit(
                                select_action_entry.as_ref().and_then(|sa| sa.cost_limit),
                                select_action_entry.as_ref().and_then(|sa| sa.cost_limit_operator.clone()),
                            )
                            .group(select_action_entry.as_ref().and_then(|sa| sa.group_names.as_ref()).and_then(|v| v.first().cloned()))
                            .characters(select_action_entry.as_ref().and_then(|sa| sa.characters.clone()))
                            .build());
                            self.execution_context = context.clone();
                            return Ok(());
                        }
                    }

                    return self.finalize_choice(&context);
                }
                "revealed_cards" => {
                    let dst = self.game_state.entry_destination().map(|s| s.to_string());
                    let dst_str = dst.as_deref().unwrap_or("hand");
                    self.move_from_revealed(indices, &mut validate_card, &dst_str);
                    return self.finalize_choice(&context);
                }
                "under_member" => {
                    let dst = self.game_state.entry_destination().map(|s| s.to_string());
                    let dst_str = dst.as_deref().unwrap_or("energy_deck").to_string();
                    self.move_from_under_member(indices, &mut validate_card, &dst_str)?;
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
                    // Validate count for non-skip selections: must meet the required count
                    if !hand_idx.is_empty() && count > 0 && hand_idx.len() < count {
                        return Err(format!(
                            "Not enough cards selected: need {}, got {}",
                            count,
                            hand_idx.len()
                        ));
                    }
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
                cost_limit,
                cost_limit_operator.as_deref(),
                group.as_deref(),
                characters.as_ref(),
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
                    // Sync to queue entry BEFORE finalize_choice, because the resolver
                    // will be recreated for sub-choices and must restore selected_cards.
                    if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                        entry.selected_card_ids = self.selected_cards.clone();
                    }
                    // Finalize to process pending sequential actions
                    return self.finalize_choice(&context);
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
                self.reveal_selected_looked_at(indices);
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
            "energy_zone" => {
                let filtered: Vec<usize> = {
                    let player = self.game_state.active_player();
                    indices
                        .iter()
                        .filter(|&&i| {
                            i < player.energy_zone.cards.len()
                                && validate_card(player.energy_zone.cards[i])
                        })
                        .copied()
                        .collect()
                };
                self.execute_selected_energy_zone_cards(&filtered, count)?;
            }
            "selected_cards" => {
                eprintln!(
                    "[SELECTED_CARDS_BEFORE] self.selected_cards={:?} indices={:?}",
                    self.selected_cards, indices
                );
                let mut cards = Vec::new();
                for &i in indices.iter() {
                    if i < self.selected_cards.len() {
                        cards.push(self.selected_cards[i]);
                    }
                }
                self.selected_cards = cards;
                eprintln!(
                    "[SELECTED_CARDS_AFTER] self.selected_cards={:?}",
                    self.selected_cards
                );
            }
            "stage" => {
                if is_select_action {
                    // Select card(s) for a state change or similar effect
                    // without moving them off stage.
                    let player = self
                        .game_state
                        .resolve_target_player_mut(target_player_id.as_deref().unwrap_or("self"));
                    let mut cards: Vec<i16> = Vec::new();
                    for &idx in indices.iter() {
                        if idx < 3 && player.stage.stage[idx] != -1 {
                            let cid = player.stage.stage[idx];
                            if validate_card(cid) {
                                cards.push(cid);
                            }
                        }
                    }
                    self.selected_cards = cards;
                } else {
                    // Move selected stage card(s) to the destination zone.
                    // Used by effects like Yoshiko (move chosen Aqours member off stage).
                    let dst = self.game_state.entry_destination().map(|s| s.to_string());
                    let dst_str = dst.as_deref().unwrap_or("discard").to_string();
                    let mut moved_ids: Vec<i16> = Vec::new();
                    let mut last_vacated: Option<usize> = None;
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
                                    crate::ability::util::place_card_in_zone(
                                        player, card_id, &dst_str, None, false, 1,
                                    );
                                    moved_ids.push(card_id);
                                    last_vacated = Some(idx);
                                }
                            }
                        }
                    }
                    if let Some(pos) = last_vacated {
                        self.game_state.last_vacated_stage_area = Some(pos);
                    }
                    self.selected_cards = moved_ids.clone();
                    if !moved_ids.is_empty() {
                        self.moved_cards = moved_ids.clone();
                        self.game_state.recently_moved_cards = Some(moved_ids);
                    }
                }
            }
            _ => eprintln!("Card selection from zone '{}' not yet implemented", zone),
        }
        eprintln!(
            "[ARN] handle_select_card: selected_cards.len()={}, zone={:?} allow_skip={}",
            self.selected_cards.len(),
            zone,
            allow_skip
        );
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
                        self.game_state
                            .ability_queue
                            .set_pending_commands(vec![Command::Effect(options[idx].clone())]);
                    }
                }
            }
            self.pending_choice = None;
            self.clear_choice_meta();
            return self.resume_pending_commands();
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

    fn handle_position_change_choice(
        &mut self,
        choice_card_no: Option<String>,
        selected: &str,
    ) -> Result<(), String> {
        if selected == "skip" {
            self.pending_choice = None;
            self.clear_choice_meta();
            self.resume_pending_commands()?;
            self.execution_context = ExecutionContext::None;
            if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                entry.execution_context = None;
            }
            return Ok(());
        }
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
        self.resume_pending_commands()?;
        Ok(())
    }

    fn handle_primary_alternative(&mut self, selected: &str) -> Result<(), String> {
        self.pending_choice = None;
        if let Some(effect) = self.game_state.entry_effect().cloned() {
            let chosen = if selected == "1" || selected == "alternative" || selected == "secondary"
            {
                effect
                    .compound
                    .alternative_effect
                    .or(effect.compound.primary_effect)
            } else {
                effect.compound.primary_effect
            };
            if let Some(sub_effect) = chosen {
                self.game_state
                    .ability_queue
                    .set_pending_commands(vec![Command::Effect(*sub_effect)]);
            }
        }
        self.resume_pending_commands()?;
        Ok(())
    }

    fn handle_position_destination(&mut self, selected: &str) -> Result<(), String> {
        self.pending_choice = None;
        if let Some(effect) = self.game_state.entry_effect().cloned() {
            let mut modified = effect.clone();
            modified.destination = Some(selected.to_string());
            self.game_state
                .ability_queue
                .set_pending_commands(vec![Command::Effect(modified)]);
        }
        self.resume_pending_commands()?;
        Ok(())
    }

    fn handle_conditional_optional(&mut self, selected: &str) -> Result<(), String> {
        self.pending_choice = None;
        if selected == "1" || selected == "yes" {
            if let Some(effect) = self.game_state.entry_effect().cloned() {
                self.game_state
                    .ability_queue
                    .set_pending_commands(vec![Command::Effect(effect)]);
            }
        }
        self.resume_pending_commands()?;
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

    pub fn clear_choice_meta(&mut self) {
        if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
            entry.choice_card_no = None;
            entry.conditional_choice = None;
        }
    }
}
