use super::types::{Choice, ChoiceResult, ExecutionContext, LookAndSelectStep};
use super::util;
use crate::ability::types::Command;
use crate::card::AbilityEffect;
use crate::game_state::GameState;

impl super::resolver::AbilityResolver {
    pub fn resume_execution(
        &mut self,
        _gs: &mut GameState,
        context: ExecutionContext,
    ) -> Result<(), String> {
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
    pub fn resume_pending_commands(&mut self, gs: &mut GameState) -> Result<(), String> {
        let pending = gs.ability_queue.take_pending_commands();
        for (i, command) in pending.iter().enumerate() {
            match command {
                Command::Effect(effect) => {
                    self.last_effect_target = effect.target.clone();
                    self.execute_effect(gs, effect)?;
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
                    let player = gs.resolve_target_player_mut(&target);
                    super::util::place_card_in_zone(player, card_id, &destination, None, false, 1);
                    gs.mods.clear_all_for_card(card_id);
                    gs.record_card_movement(card_id);
                    if state_change.as_deref() == Some("wait") {
                        gs.mods.add_orientation_modifier(card_id, "wait");
                    }
                }
                Command::Choice(choice) => {
                    self.pending_choice = Some(choice.clone());
                }
            }

            if self.pending_choice.is_some() {
                if i + 1 < pending.len() {
                    let mut existing = gs.ability_queue.take_pending_commands();
                    existing.extend(pending[i + 1..].to_vec());
                    gs.ability_queue.set_pending_commands(existing);
                }
                return Ok(());
            }
        }
        Ok(())
    }

    pub fn expire_live_end_effects(&mut self, _gs: &mut GameState) {
        let initial_count = self.duration_effects.len();
        self.duration_effects
            .retain(|(_, duration)| duration != "live_end");
        let expired_count = initial_count - self.duration_effects.len();
        if expired_count > 0 {
            eprintln!("Expired {} effects with duration 'live_end'", expired_count);
        }
    }

    /// Shared epilogue: clear pending_choice, resume execution, process pending sequential actions.
    fn finalize_choice(
        &mut self,
        gs: &mut GameState,
        context: &ExecutionContext,
    ) -> Result<(), String> {
        let is_actual_looked_at_choice = self
            .pending_choice
            .as_ref()
            .map(|choice| matches!(choice, Choice::SelectCard { zone, .. } if zone == "looked_at"))
            .unwrap_or(false);

        let has_pending_sequential = gs.ability_queue.has_pending_commands();

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

        let sub_choice = self.sub_choice_created;
        self.sub_choice_created = false;
        if !should_preserve && !sub_choice {
            self.pending_choice = None;
        }
        self.resume_execution(gs, context.clone())?;
        if !sub_choice {
            self.resume_pending_commands(gs)?;
        }
        eprintln!(
            "[FINALIZE_CHOICE] pending={} selected={:?} context={:?}",
            has_pending_sequential, self.selected_cards, context
        );
        Ok(())
    }

    fn reveal_selected_looked_at(&mut self, gs: &mut GameState, indices: &[usize]) {
        let mut revealed_ids = Vec::new();
        for &idx in indices.iter() {
            if idx < self.looked_at_cards.len() {
                let cid = self.looked_at_cards[idx];
                gs.revealed_cards.push(cid);
                revealed_ids.push(cid);
            }
        }
        if !revealed_ids.is_empty() {
            let card_db = &gs.card_database;
            let names: Vec<String> = revealed_ids
                .iter()
                .filter_map(|id| card_db.get_card(*id))
                .map(|c| c.name.clone())
                .collect();
            if !names.is_empty() {
                let turn = gs.turn_number;
                gs.rule_log.push(format!(
                    "[Turn {}] P{} reveals {} from looked-at cards",
                    turn,
                    if std::ptr::eq(gs.active_player(), &gs.player1) {
                        1
                    } else {
                        2
                    },
                    names.join(", ")
                ));
            }
        }
    }

    pub fn provide_choice_result(
        &mut self,
        gs: &mut GameState,
        result: ChoiceResult,
    ) -> Result<(), String> {
        let choice = self.pending_choice.clone();
        let context = self.execution_context.clone();
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
                    cost_total,
                    cost_total_operator,
                    group,
                    characters,
                    filtered_indices,
                    is_select_action,
                    ref target_player_id,
                    ..
                }),
                ChoiceResult::CardSelected { indices },
            ) => self.handle_select_card(
                gs,
                zone,
                card_type,
                *count,
                *allow_skip,
                &indices,
                context,
                *cost_limit,
                cost_limit_operator.clone(),
                *cost_total,
                cost_total_operator.clone(),
                group.clone(),
                characters.clone(),
                filtered_indices.clone(),
                *is_select_action,
                target_player_id.clone(),
                choice.as_ref().map_or(false, |c| {
                    matches!(
                        c,
                        Choice::SelectCard {
                            is_reveal: true,
                            ..
                        }
                    )
                }),
            ),
            (Some(Choice::SelectCard { .. }), ChoiceResult::Skip) => {
                self.clear_choice_state(gs);
                self.resume_execution(gs, context)
            }
            (
                Some(Choice::SelectTarget { target, .. }),
                ChoiceResult::TargetSelected { target: selected },
            ) if target == "area_select" => {
                // area_select: look up the actual option string from the choice's options list
                // instead of using the numeric index directly.
                if let Some(Choice::SelectTarget {
                    options: Some(ref opts),
                    ..
                }) = choice
                {
                    let idx: usize = selected.parse().unwrap_or(0);
                    let opt = opts.get(idx).map(|s| s.as_str()).unwrap_or("left");
                    self.selected_area = Some(opt.to_string());
                    self.clear_choice_state(gs);
                    return self.resume_pending_commands(gs);
                }
                self.selected_area = Some(selected.clone());
                self.clear_choice_state(gs);
                self.resume_pending_commands(gs)
            }
            (
                Some(Choice::SelectTarget { target, .. }),
                ChoiceResult::TargetSelected { target: selected },
            ) => self.handle_select_target(gs, target, &selected, context),
            (Some(Choice::SelectPosition { .. }), ChoiceResult::PositionSelected { position }) => {
                self.handle_select_position(gs, &position, context)
            }
            (
                Some(Choice::SelectHeartColor { count, .. }),
                ChoiceResult::HeartColorSelected { colors },
            )
            | (
                Some(Choice::SelectHeartType { count, .. }),
                ChoiceResult::HeartTypeSelected { types: colors },
            ) => self.handle_heart_selection(gs, *count as u32, &colors),
            _ => Err("Choice result does not match pending choice".to_string()),
        }
    }

    fn handle_select_card(
        &mut self,
        gs: &mut GameState,
        zone: &str,
        card_type: &Option<String>,
        count: usize,
        allow_skip: bool,
        indices: &[usize],
        context: ExecutionContext,
        cost_limit: Option<u32>,
        cost_limit_operator: Option<String>,
        cost_total: Option<u32>,
        cost_total_operator: Option<String>,
        group: Option<String>,
        characters: Option<Vec<String>>,
        filtered_indices: Option<Vec<usize>>,
        is_select_action: bool,
        target_player_id: Option<String>,
        is_reveal: bool,
    ) -> Result<(), String> {
        println!(
            "DEBUG: handle_select_card - zone: '{}', indices: {:?}, context: {:?}, is_reveal: {}",
            zone, indices, context, is_reveal
        );

        // Distinguish cost vs effect: cost handler only fires when effect NOT yet started.
        // effect_started is false during cost payment, true during effect execution.
        let effect_started = gs
            .ability_queue
            .current_entry()
            .map_or(false, |e| e.effect_started);

        // Handle reveal action: push selected cards to revealed_cards, don't discard
        if is_reveal && zone == "hand" {
            let target = target_player_id
                .clone()
                .unwrap_or_else(|| "self".to_string());
            let player = gs.resolve_target_player_mut(&target);
            let card_ids = util::resolve_indices_to_ids(player, "hand", indices);
            for &cid in &card_ids {
                gs.revealed_cards.push(cid);
            }
            if !card_ids.is_empty() {
                let names: Vec<String> = card_ids
                    .iter()
                    .filter_map(|id| gs.card_database.get_card(*id))
                    .map(|c| c.name.clone())
                    .collect();
                let player_label =
                    super::util::target_player_label(&target, gs.ability_master_id().as_deref());
                gs.rule_log.push(format!(
                    "[Turn {}] {} reveals {} from hand",
                    gs.turn_number,
                    player_label,
                    names.join(", ")
                ));
            }
            self.clear_choice_state(gs);
            return self.resume_pending_commands(gs);
        }

        if !effect_started && gs.entry_cost().and_then(|c| c.cost_type.as_deref()) == Some("reveal")
        {
            let cost = gs.entry_cost().cloned().unwrap();
            let card_db = gs.card_database.clone();
            let player = gs.active_player();
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
                let card_db = &gs.card_database;
                let names: Vec<String> = card_ids
                    .iter()
                    .filter_map(|id| card_db.get_card(*id))
                    .map(|c| c.name.clone())
                    .collect();
                let turn = gs.turn_number;
                let player_num = if std::ptr::eq(gs.active_player(), &gs.player1) {
                    1
                } else {
                    2
                };
                if !names.is_empty() {
                    gs.rule_log.push(format!(
                        "[Turn {}] P{} reveals {} from hand as cost",
                        turn,
                        player_num,
                        names.join(", ")
                    ));
                }
            }
            for card_id in card_ids {
                gs.revealed_cards.push(card_id);
                self.revealed_cost_cards.push(card_id);
            }
            return self.finalize_choice(gs, &context);
        }

        let card_db = gs.card_database.clone();
        let validate_filter = util::filter_from_parts(
            card_type.as_deref(),
            group.as_deref(),
            cost_limit,
            cost_limit_operator.as_deref(),
            characters.as_ref(),
            None,
            None,
        );
        let mut validate_card =
            |cid: i16| -> bool { validate_filter.matches(&card_db, cid, false) };

        // When effect has started, skip the cost handler for "discard" zone.
        // The effect handler's discard arm has position choice logic that would be
        // skipped by the cost handler's early return. Other zones don't have this
        // conflict, so the cost handler can process them normally.
        if allow_skip && !indices.is_empty() && !(effect_started && zone == "discard") {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.optional_cost_was_paid = true;
            }
            match zone {
                "hand" => {
                    let target = target_player_id
                        .clone()
                        .unwrap_or_else(|| "self".to_string());
                    // If not enough cards yet, accumulate and re-prompt (sequential).
                    // Skip re-prompt for "up-to-N" (まで) or "any_number" costs where
                    // user may legitimately select fewer than the maximum.
                    let cost = gs.entry_cost();
                    let is_up_to = cost.map_or(false, |c| c.text.contains("まで"));
                    let is_any_num = cost.map_or(false, |c| {
                        c.count.is_none() || c.any_number.unwrap_or(false)
                    });
                    if !is_up_to && !is_any_num && indices.len() < count {
                        let hand_cards: Vec<i16> = {
                            let p = gs.resolve_target_player_mut(&target);
                            p.hand.cards.to_vec()
                        };
                        let mut all_hand_idxs = indices.to_vec();
                        let prev_ids = self.selected_cards.clone();
                        if !prev_ids.is_empty() {
                            for (hidx, cid) in hand_cards.iter().enumerate() {
                                if prev_ids.contains(cid) && !all_hand_idxs.contains(&hidx) {
                                    all_hand_idxs.push(hidx);
                                }
                            }
                        }
                        let new_card_ids: Vec<i16> = all_hand_idxs
                            .iter()
                            .filter_map(|&idx| {
                                if idx < hand_cards.len() {
                                    Some(hand_cards[idx])
                                } else {
                                    None
                                }
                            })
                            .collect();
                        for cid in new_card_ids {
                            if !self.selected_cards.contains(&cid) {
                                self.selected_cards.push(cid);
                            }
                        }
                        let remaining = count - indices.len();
                        self.pending_choice = Some(
                            Choice::select_cards(
                                "hand",
                                remaining,
                                format!("Select {} more card(s) from hand", remaining),
                                true,
                            )
                            .card_type(card_type.clone())
                            .cost_limit(cost_limit, cost_limit_operator.clone())
                            .cost_total(cost_total, cost_total_operator.clone())
                            .group(group.clone())
                            .characters(characters.clone())
                            .filtered_indices(Some(all_hand_idxs.to_vec()))
                            .build(),
                        );
                        self.store_pending_choice(gs);
                        return Ok(());
                    }
                    let player = gs.resolve_target_player_mut(&target);
                    let card_ids = util::resolve_indices_to_ids(player, "hand", indices);
                    let valid_ids: Vec<i16> = card_ids
                        .into_iter()
                        .filter(|&cid| validate_card(cid))
                        .collect();
                    let _count =
                        util::move_cards(player, &valid_ids, "hand", "discard", None, &card_db);
                    if !valid_ids.is_empty() {
                        gs.mods.last_cost_discard_count = valid_ids.len() as u32;
                        self.moved_cards = valid_ids.clone();
                        gs.recently_moved_cards = Some(valid_ids);
                    }
                }
                "stage" => {
                    let player = gs.active_player_mut();
                    let card_ids = util::resolve_indices_to_ids(player, "stage", indices);
                    let valid_ids: Vec<i16> = card_ids
                        .into_iter()
                        .filter(|&cid| validate_card(cid))
                        .collect();
                    let mut last_vacated = None;
                    for &cid in &valid_ids {
                        if let Some(pos) = player.stage.stage.iter().position(|&x| x == cid) {
                            last_vacated = Some(pos);
                        }
                    }
                    let moved_count =
                        util::move_cards(player, &valid_ids, "stage", "discard", None, &card_db);
                    if moved_count > 0 {
                        if let Some(pos) = last_vacated {
                            gs.last_vacated_stage_area = Some(pos);
                        }
                        self.moved_cards = valid_ids.clone();
                        gs.recently_moved_cards = Some(valid_ids);
                    }
                    self.clear_choice_state(gs);
                    return self.resume_pending_commands(gs);
                }
                "energy_zone" => {
                    self.mark_energy_as_wait(gs, indices, &mut validate_card);
                }
                "discard" => {
                    if is_select_action {
                        let player = gs.active_player_mut();
                        let mut cards: Vec<i16> = Vec::new();
                        for &i in indices.iter() {
                            if i < player.waitroom.cards.len() {
                                let cid = player.waitroom.cards[i];
                                if validate_card(cid) {
                                    cards.push(cid);
                                }
                            }
                        }
                        // Accumulate: keep previously selected cards across sequential prompts.
                        for &cid in &cards {
                            if !self.selected_cards.contains(&cid) {
                                self.selected_cards.push(cid);
                            }
                        }
                    } else {
                        self.execute_selected_cards_from_zone(
                            gs,
                            "discard",
                            indices,
                            count,
                            card_type.as_deref(),
                            cost_limit,
                            cost_limit_operator.as_deref(),
                            cost_total,
                            cost_total_operator.as_deref(),
                            group.as_deref(),
                            characters.as_ref(),
                            target_player_id.as_deref(),
                        )?;
                    }
                }
                "deck" => self.execute_selected_cards_from_zone(
                    gs,
                    "deck",
                    indices,
                    count,
                    card_type.as_deref(),
                    cost_limit,
                    cost_limit_operator.as_deref(),
                    cost_total,
                    cost_total_operator.as_deref(),
                    group.as_deref(),
                    characters.as_ref(),
                    target_player_id.as_deref(),
                )?,
                "looked_at" => {
                    eprintln!("[HSC1_LOOKED_AT] START: looked_cards.len()={}, indices={:?}, filtered_indices={:?}, is_select_cards={}",
                        self.looked_at_cards.len(), indices, filtered_indices, gs.ability_queue.current_entry()
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
                    let select_action_entry = gs
                        .ability_queue
                        .current_entry()
                        .and_then(|e| e.ability.effect.as_ref())
                        .and_then(|ef| ef.compound.select_action.clone());
                    // Only reveal when select_action has reveal:true
                    // (parser emits this for "公開して手札に加える" / reveal and add to hand)
                    if select_action_entry
                        .as_ref()
                        .and_then(|sa| sa.reveal)
                        .unwrap_or(false)
                    {
                        self.reveal_selected_looked_at(gs, &mapped_indices);
                    }
                    let is_select_cards = select_action_entry
                        .as_ref()
                        .map(|sa| sa.action == "select_cards")
                        .unwrap_or(false);

                    if is_select_cards {
                        self.handle_select_cards_looked_at(gs, &mapped_indices)?;
                        if matches!(self.pending_choice, Some(Choice::SelectTarget { ref target, .. }) if target == "order")
                        {
                            return Ok(());
                        }
                    } else if let ExecutionContext::LookAndSelect { .. } = self.execution_context {
                        if let Some(_) = select_action_entry {}
                        self.handle_select_cards_looked_at(gs, &mapped_indices)?;
                        self.looked_at_cards = gs.looked_at_cards.clone();
                    } else {
                        self.handle_select_cards_looked_at(gs, &mapped_indices)?;
                        self.looked_at_cards = gs.looked_at_cards.clone();
                    }

                    // Check if we can select more cards (for round-based "up to X" abilities).
                    if is_select_cards && !mapped_indices.is_empty() {
                        let _any_number = select_action_entry
                            .as_ref()
                            .and_then(|sa| sa.any_number)
                            .unwrap_or(false);
                        let max_count = count;
                        let selected_count = mapped_indices.len();
                        let remaining = gs.looked_at_cards.len();

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

                    return self.finalize_choice(gs, &context);
                }
                "revealed_cards" => {
                    let dst = gs.entry_destination().map(|s| s.to_string());
                    let dst_str = dst.as_deref().unwrap_or("hand");
                    self.move_from_revealed(gs, indices, &mut validate_card, &dst_str);
                    return self.finalize_choice(gs, &context);
                }
                "under_member" => {
                    let dst = gs.entry_destination().map(|s| s.to_string());
                    let dst_str = dst.as_deref().unwrap_or("energy_deck").to_string();
                    self.move_from_under_member(gs, indices, &mut validate_card, &dst_str)?;
                    return self.finalize_choice(gs, &context);
                }
                _ => {
                    // clear choice state but don't skip pending commands (e.g. empty_area position choice)
                    self.clear_choice_state(gs);
                    return self.resume_pending_commands(gs);
                }
            }
            self.clear_choice_state(gs);
            if gs.ability_queue.has_pending_commands() {
                return self.resume_pending_commands(gs);
            }
            return Ok(());
        }

        match zone {
            "hand" => {
                let hand_idx = if indices.is_empty() && !allow_skip && count > 0 {
                    return Err("No cards selected from hand for required selection".to_string());
                } else {
                    indices
                };
                if !hand_idx.is_empty() || allow_skip {
                    if !hand_idx.is_empty() && count > 0 && hand_idx.len() < count {
                        // Sequential selection: store selected HAND INDICES and re-prompt
                        // Read target player's hand and build index list (no gs borrow after this)
                        let target = target_player_id.as_deref().unwrap_or("self").to_string();
                        let hand_cards: Vec<i16> = {
                            let p = gs.resolve_target_player_mut(&target);
                            p.hand.cards.to_vec()
                        };
                        let mut all_hand_idxs = hand_idx.to_vec();
                        // Restore previously accumulated indices from resolver
                        let prev_ids: Vec<i16> = self.selected_cards.clone();
                        if !prev_ids.is_empty() {
                            for (hidx, cid) in hand_cards.iter().enumerate() {
                                if prev_ids.contains(cid) && !all_hand_idxs.contains(&hidx) {
                                    all_hand_idxs.push(hidx);
                                }
                            }
                        }
                        // Store all accumulated card IDs
                        let new_card_ids: Vec<i16> = all_hand_idxs
                            .iter()
                            .filter_map(|&idx| {
                                if idx < hand_cards.len() {
                                    Some(hand_cards[idx])
                                } else {
                                    None
                                }
                            })
                            .collect();
                        for cid in new_card_ids {
                            if !self.selected_cards.contains(&cid) {
                                self.selected_cards.push(cid);
                            }
                        }
                        // Re-prompt with remaining count, excluding already-selected indices
                        let remaining = count - hand_idx.len();
                        self.pending_choice = Some(
                            Choice::select_cards(
                                "hand",
                                remaining,
                                format!("Select {} more card(s) from hand", remaining),
                                false,
                            )
                            .card_type(card_type.clone())
                            .cost_limit(cost_limit, cost_limit_operator.clone())
                            .cost_total(cost_total, cost_total_operator.clone())
                            .group(group.clone())
                            .characters(characters.clone())
                            .filtered_indices(Some(all_hand_idxs.to_vec()))
                            .build(),
                        );
                        self.store_pending_choice(gs);
                        return Ok(());
                    }
                    // Final batch: accumulate with any previously selected cards
                    let mut all_idxs = hand_idx.to_vec();
                    let selected_ids = self.selected_cards.clone();
                    if !selected_ids.is_empty() {
                        let target = target_player_id.as_deref().unwrap_or("self").to_string();
                        let hand_cards: Vec<i16> = {
                            let p = gs.resolve_target_player_mut(&target);
                            p.hand.cards.to_vec()
                        };
                        for (hidx, cid) in hand_cards.iter().enumerate() {
                            if selected_ids.contains(cid) && !all_idxs.contains(&hidx) {
                                all_idxs.push(hidx);
                            }
                        }
                    }
                    self.execute_selected_cards_from_zone(
                        gs,
                        "hand",
                        &all_idxs,
                        count,
                        card_type.as_deref(),
                        cost_limit,
                        cost_limit_operator.as_deref(),
                        cost_total,
                        cost_total_operator.as_deref(),
                        group.as_deref(),
                        characters.as_ref(),
                        target_player_id.as_deref(),
                    )?
                }
                // Track whether optional cost was actually paid
                if allow_skip {
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.optional_cost_was_paid = !indices.is_empty();
                    }
                    // When optional cost is skipped, discard any pending "そうした場合" actions.
                    // These were saved by execute_sequential_effect and should NOT run
                    // when the user chose not to pay the optional cost.
                    if indices.is_empty() {
                        gs.ability_queue.take_pending_commands();
                    }
                }
            }
            "deck" => self.execute_selected_cards_from_zone(
                gs,
                "deck",
                indices,
                count,
                card_type.as_deref(),
                cost_limit,
                cost_limit_operator.as_deref(),
                cost_total,
                cost_total_operator.as_deref(),
                group.as_deref(),
                characters.as_ref(),
                target_player_id.as_deref(),
            )?,
            "discard" => {
                if is_select_action {
                    // Just store card IDs without moving
                    let target = target_player_id.as_deref().unwrap_or("self");
                    let player = gs.resolve_target_player_mut(target);
                    let mut cards: Vec<i16> = Vec::new();
                    for &i in indices.iter() {
                        if i < player.waitroom.cards.len() {
                            cards.push(player.waitroom.cards[i]);
                        }
                    }
                    self.selected_cards = cards;
                    // Finalize to process pending sequential actions
                    return self.finalize_choice(gs, &context);
                } else {
                    self.execute_selected_cards_from_zone(
                        gs,
                        "discard",
                        indices,
                        count,
                        card_type.as_deref(),
                        cost_limit,
                        cost_limit_operator.as_deref(),
                        cost_total,
                        cost_total_operator.as_deref(),
                        group.as_deref(),
                        characters.as_ref(),
                        target_player_id.as_deref(),
                    )?;
                    // Persist sub-choice (e.g. SelectPosition)
                    if self.pending_choice.is_some() {
                        self.store_pending_choice(gs);
                    }
                }
            }
            "looked_at" => {
                let valid: Vec<usize> = indices
                    .iter()
                    .filter(|&&i| {
                        let cid = gs.looked_at_cards.get(i).copied().unwrap_or(-1);
                        cid != -1 && validate_card(cid)
                    })
                    .copied()
                    .collect();

                let select_action_entry = gs
                    .ability_queue
                    .current_entry()
                    .and_then(|e| e.ability.effect.as_ref())
                    .and_then(|ef| ef.compound.select_action.clone());
                let is_select_cards = select_action_entry
                    .as_ref()
                    .map(|sa| sa.action == "select_cards")
                    .unwrap_or(false);

                if select_action_entry
                    .as_ref()
                    .and_then(|sa| sa.reveal)
                    .unwrap_or(false)
                {
                    self.reveal_selected_looked_at(gs, &valid);
                }
                self.handle_select_cards_looked_at(gs, &valid)?;

                // Preserve order prompt from handle_select_cards_looked_at
                if matches!(self.pending_choice, Some(Choice::SelectTarget { ref target, .. }) if target == "order")
                {
                    return Ok(());
                }

                // Re-prompt for remaining cards (same logic as cost handler arm)
                if is_select_cards && !valid.is_empty() {
                    let _any_number = select_action_entry
                        .as_ref()
                        .and_then(|sa| sa.any_number)
                        .unwrap_or(false);
                    let max_count = count;
                    let selected_count = valid.len();
                    let remaining = gs.looked_at_cards.len();
                    if max_count > selected_count && remaining > 0 {
                        let remaining_max = max_count - selected_count;
                        let ct = card_type.clone();
                        self.pending_choice = Some(
                            Choice::select_cards(
                                "looked_at",
                                remaining_max,
                                format!("Select up to {} more card(s) from the {} remaining looked-at cards", remaining_max, remaining),
                                true,
                            )
                            .card_type(ct)
                            .cost_limit(
                                select_action_entry.as_ref().and_then(|sa| sa.cost_limit),
                                select_action_entry.as_ref().and_then(|sa| sa.cost_limit_operator.clone()),
                            )
                            .group(select_action_entry.as_ref().and_then(|sa| sa.group_names.as_ref()).and_then(|v| v.first().cloned()))
                            .characters(select_action_entry.as_ref().and_then(|sa| sa.characters.clone()))
                            .build(),
                        );
                        self.execution_context = context.clone();
                        return Ok(());
                    }
                }
                return self.finalize_choice(gs, &context);
            }
            "revealed_cards" => {
                let mut cards = Vec::new();
                for &i in indices.iter().rev() {
                    if i < gs.revealed_cards.len() {
                        let cid = gs.revealed_cards.remove(i);
                        if validate_card(cid) {
                            cards.push(cid);
                        }
                    }
                }
                self.selected_cards = cards;
                // Move selected cards to destination
                let dst = gs.entry_destination().map(|s| s.to_string());
                let dst_str = dst.as_deref().unwrap_or("hand");
                let player = gs.active_player_mut();
                for &cid in &self.selected_cards.clone() {
                    crate::ability::util::place_card_in_zone(player, cid, dst_str, None, false, 1);
                }
            }
            "energy_zone" => {
                let filtered: Vec<usize> = {
                    let player = gs.active_player();
                    indices
                        .iter()
                        .filter(|&&i| {
                            i < player.energy_zone.cards.len()
                                && validate_card(player.energy_zone.cards[i])
                        })
                        .copied()
                        .collect()
                };
                self.execute_selected_energy_zone_cards(gs, &filtered, count)?;
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
                    // When the user skips a max/optional selection, discard pending
                    // re-apply commands to avoid re-prompting in an infinite loop.
                    if indices.is_empty() && allow_skip {
                        gs.ability_queue.take_pending_commands();
                        self.selected_cards = vec![];
                        eprintln!("[SELECT_STAGE] skip: cleared pending commands");
                    }
                    let stage_indices: Vec<usize> = if let Some(ref fidx) = filtered_indices {
                        indices
                            .iter()
                            .filter_map(|&i| fidx.get(i).copied())
                            .collect()
                    } else {
                        indices.to_vec()
                    };
                    let player =
                        gs.resolve_target_player_mut(target_player_id.as_deref().unwrap_or("self"));
                    let mut cards: Vec<i16> = Vec::new();
                    for &idx in stage_indices.iter() {
                        if idx < 3 && player.stage.stage[idx] != -1 {
                            let cid = player.stage.stage[idx];
                            if validate_card(cid) {
                                cards.push(cid);
                            }
                        }
                    }
                    eprintln!(
                        "[SELECT_STAGE] is_select_action=true cards={:?} filtered_idx={:?}",
                        cards, filtered_indices
                    );
                    for &cid in &cards {
                        if !self.selected_cards.contains(&cid) {
                            self.selected_cards.push(cid);
                        }
                    }
                } else {
                    // Move selected stage card(s) to the destination zone.
                    // Used by effects like Yoshiko (move chosen Aqours member off stage).
                    let dst = gs.entry_destination().map(|s| s.to_string());
                    let dst_str = dst.as_deref().unwrap_or("discard").to_string();
                    let player = gs.active_player_mut();
                    let card_ids = util::resolve_indices_to_ids(player, "stage", indices);
                    let valid_ids: Vec<i16> = card_ids
                        .into_iter()
                        .filter(|&cid| validate_card(cid))
                        .collect();
                    let mut last_vacated = None;
                    for &cid in &valid_ids {
                        if let Some(pos) = player.stage.stage.iter().position(|&x| x == cid) {
                            last_vacated = Some(pos);
                        }
                    }
                    let moved_count =
                        util::move_cards(player, &valid_ids, "stage", &dst_str, None, &card_db);
                    if moved_count > 0 {
                        if let Some(pos) = last_vacated {
                            gs.last_vacated_stage_area = Some(pos);
                        }
                        self.selected_cards = valid_ids.clone();
                        self.moved_cards = valid_ids.clone();
                        gs.recently_moved_cards = Some(valid_ids);
                    }
                }
            }
            _ => eprintln!("Card selection from zone '{}' not yet implemented", zone),
        }
        eprintln!(
            "▶ Select: {} card(s) selected from zone={:?} → [{}]",
            self.selected_cards.len(),
            zone,
            self.pipeline.fmt_ids(&self.selected_cards)
        );
        self.finalize_choice(gs, &context)
    }

    fn handle_select_target(
        &mut self,
        gs: &mut GameState,
        target: &str,
        selected: &str,
        _context: ExecutionContext,
    ) -> Result<(), String> {
        let choice_card_no = gs.entry_choice_card_no();
        let conditional_choice = gs.entry_conditional_choice();

        if choice_card_no.as_deref() == Some("choice") {
            if let Some(ref options_json) = conditional_choice {
                if let Ok(options) = serde_json::from_str::<Vec<AbilityEffect>>(options_json) {
                    let idx: usize = selected.parse().unwrap_or(0);
                    if idx < options.len() {
                        gs.ability_queue
                            .set_pending_commands(vec![Command::Effect(options[idx].clone())]);
                    }
                }
            }
            return self.clear_choice_state_and_resume(gs);
        }

        if choice_card_no.as_deref() == Some("choice_string") {
            return self.handle_choice_string_selection(gs, selected, conditional_choice);
        }

        if choice_card_no
            .as_deref()
            .map(|s| s.starts_with("position_change"))
            .unwrap_or(false)
        {
            return self.handle_position_change_choice(gs, choice_card_no, selected);
        }

        if target == "choice_string" {
            return self.handle_choice_string_store(gs, selected, conditional_choice);
        }

        if target == "pay_optional_cost:skip_optional_cost" {
            return self.handle_optional_cost_payment(gs, selected);
        }

        if target == "primary|alternative" {
            return self.handle_primary_alternative(gs, selected);
        }

        if target == "apply_replacement" {
            self.clear_choice_state(gs);
            return Ok(());
        }

        if target == "choose_required_hearts" {
            gs.prohibition_effects
                .push(format!("chosen_required_hearts:{}", selected));
            self.clear_choice_state(gs);
            return Ok(());
        }

        if target == "position|destination" {
            return self.handle_position_destination(gs, selected);
        }

        if target == "heart_color" {
            return self.handle_heart_color_selection(gs, selected);
        }

        if target == "choice_type" {
            self.clear_choice_state(gs);
            return Ok(());
        }

        if target == "choice_condition" {
            return self.handle_choice_condition(gs, selected);
        }

        if target == "conditional_optional" {
            return self.handle_conditional_optional(gs, selected);
        }

        if target == "draw_any_number" {
            return self.handle_draw_any_number(gs, selected);
        }

        if target == "order" {
            return self.handle_order_selection(gs, selected);
        }

        self.clear_choice_state(gs);
        Ok(())
    }

    fn handle_draw_any_number(&mut self, gs: &mut GameState, selected: &str) -> Result<(), String> {
        let count: usize = selected.parse().unwrap_or(0);
        if let Some(effect) = gs.entry_effect().cloned() {
            let source = effect.source.as_deref().unwrap_or("deck");
            let destination = effect.destination.as_deref().unwrap_or("hand");
            let card_type = effect.card_type.as_deref();
            let card_db = gs.card_database.clone();
            let target = effect.target.as_deref().unwrap_or("self");
            let player = gs.resolve_target_player_mut(target);
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
        self.clear_choice_state(gs);
        Ok(())
    }

    fn handle_order_selection(&mut self, gs: &mut GameState, selected: &str) -> Result<(), String> {
        let ctx = self.execution_context.clone();
        if let ExecutionContext::LookAndSelect { step } = ctx {
            if let LookAndSelectStep::Finalize { destination } = step {
                if destination == "deck" {
                    if let Ok(idx) = selected.parse::<usize>() {
                        if idx < gs.looked_at_cards.len() {
                            let card = gs.looked_at_cards.remove(idx);
                            gs.looked_at_cards.insert(0, card);
                        }
                    }
                    let card_ids: Vec<i16> = gs.looked_at_cards.iter().rev().copied().collect();
                    let player = gs.active_player_mut();
                    for card_id in card_ids {
                        player.main_deck.cards.insert(0, card_id);
                    }
                    gs.looked_at_cards.clear();
                    self.looked_at_cards.clear();
                }
            }
        }
        self.clear_choice_state(gs);
        Ok(())
    }

    fn handle_position_change_choice(
        &mut self,
        gs: &mut GameState,
        choice_card_no: Option<String>,
        selected: &str,
    ) -> Result<(), String> {
        if selected == "skip" {
            self.clear_choice_state_and_resume(gs)?;
            self.execution_context = ExecutionContext::None;
            return Ok(());
        }
        if let Some(effect) = gs.entry_effect().cloned() {
            let mut modified = effect.clone();
            let dest = match selected {
                "0" | "left" | "left_side" => "left",
                "1" | "center" => "center",
                "2" | "right" | "right_side" => "right",
                _ => selected,
            };
            if let Some(ref ccn) = choice_card_no {
                if let Some(tgt) = ccn.strip_prefix("position_change:") {
                    if tgt == "opponent:front" {
                        // Special case: opponent selects SOURCE member to move to front area.
                        // The destination is "front" (already in effect); selected is the source.
                        modified.source_position = Some(selected.to_string());
                        if let Err(e) =
                            self.execute_position_change_with_destination(gs, &modified, "front")
                        {
                            eprintln!("Failed to execute position change: {}", e);
                        }
                        self.clear_choice_state_and_resume(gs)?;
                        return Ok(());
                    } else if tgt.contains(':') {
                        let parts: Vec<&str> = tgt.splitn(2, ':').collect();
                        modified.target = Some(parts[0].to_string());
                        // parts[1] is either a card_no (for multiple_targets) or
                        // a source position like "left_side" (for formation_change).
                        // Distinguish by checking if it's a valid position index.
                        if super::util::stage_position_index(parts[1]).is_some() {
                            modified.source_position = Some(parts[1].to_string());
                        } else {
                            modified.target_member = Some(parts[1].to_string());
                        }
                    } else {
                        modified.target = Some(tgt.to_string());
                    }
                }
            }
            modified.destination = Some(dest.to_string());
            if let Err(e) = self.execute_position_change_with_destination(gs, &modified, dest) {
                eprintln!("Failed to execute position change: {}", e);
            }
        }
        self.clear_choice_state_and_resume(gs)?;
        Ok(())
    }

    fn apply_effect_modification<F>(
        &mut self,
        gs: &mut GameState,
        modifier: F,
    ) -> Result<(), String>
    where
        F: Fn(&mut AbilityEffect),
    {
        self.clear_choice_state(gs);
        if let Some(mut effect) = gs.entry_effect().cloned() {
            modifier(&mut effect);
            gs.ability_queue
                .set_pending_commands(vec![Command::Effect(effect)]);
        }
        self.resume_pending_commands(gs)?;
        Ok(())
    }

    fn handle_primary_alternative(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        self.apply_effect_modification(gs, |effect| {
            let chosen = if selected == "1" || selected == "alternative" || selected == "secondary"
            {
                effect
                    .compound
                    .alternative_effect
                    .clone()
                    .or(effect.compound.primary_effect.clone())
            } else {
                effect.compound.primary_effect.clone()
            };
            if let Some(sub_effect) = chosen {
                *effect = (*sub_effect).clone();
            }
        })
    }

    fn handle_position_destination(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        self.apply_effect_modification(gs, |effect| {
            effect.destination = Some(selected.to_string());
        })
    }

    fn handle_conditional_optional(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        self.clear_choice_state(gs);
        if let Some(effect) = gs.entry_effect().cloned() {
            let is_negation = effect.compound.conditional_negation.unwrap_or(false);
            let chose_yes = selected == "1" || selected == "yes";
            let cmd = match (chose_yes, is_negation) {
                // yes + negation → optional_action fires, conditional skipped
                (true, true) => effect.compound.optional_action.map(|a| Command::Effect(*a)),
                // yes + no negation → full effect runs (optional then conditional)
                (true, false) => Some(Command::Effect(effect)),
                // no + negation → conditional_action fires (the penalty)
                (false, true) => effect
                    .compound
                    .conditional_action
                    .map(|a| Command::Effect(*a)),
                // no + no negation → nothing fires
                (false, false) => None,
            };
            if let Some(cmd) = cmd {
                gs.ability_queue.set_pending_commands(vec![cmd]);
            }
        }
        self.resume_pending_commands(gs)?;
        Ok(())
    }

    fn handle_heart_color_selection(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        const HEART_VALS: [&str; 7] = [
            "heart00", "heart01", "heart02", "heart03", "heart04", "heart05", "heart06",
        ];
        let idx: usize = selected.parse().unwrap_or(0);
        if idx < HEART_VALS.len() {
            gs.prohibition_effects
                .push(format!("selected_heart_color:{}", HEART_VALS[idx]));
        }
        self.clear_choice_state(gs);
        Ok(())
    }

    fn handle_choice_condition(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        let idx: usize = selected.parse().unwrap_or(0);
        if let Some(options) = gs.entry_cost().and_then(|c| c.options.clone()) {
            if idx < options.len() {
                if let Err(e) = self.pay_cost(gs, &options[idx]) {
                    eprintln!("Failed to pay cost option: {}", e);
                }
            }
        }
        self.clear_choice_state(gs);
        Ok(())
    }

    fn handle_heart_selection(
        &mut self,
        gs: &mut GameState,
        count: u32,
        colors: &[String],
    ) -> Result<(), String> {
        if let Some(chosen) = colors.first() {
            let color = crate::zones::parse_heart_color(chosen);
            if let Some(card_id) = gs.activating_card {
                gs.set_heart_override(card_id, color, count.max(1), "live_end");
            }
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.conditional_choice = Some(chosen.clone());
            }
        }
        self.pending_choice = None;
        self.finalize_choice(gs, &self.execution_context.clone())
    }

    pub fn clear_choice_meta(&mut self, gs: &mut GameState) {
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.choice_card_no = None;
            entry.conditional_choice = None;
        }
    }

    fn clear_choice_state(&mut self, gs: &mut GameState) {
        if self.sub_choice_created {
            self.sub_choice_created = false;
        } else {
            self.pending_choice = None;
        }
        self.clear_choice_meta(gs);
    }

    fn clear_choice_state_and_resume(&mut self, gs: &mut GameState) -> Result<(), String> {
        self.clear_choice_state(gs);
        self.resume_pending_commands(gs)
    }
}
