use super::condition::ConditionContext;
use super::enums::Zone;
use super::types::{Choice, ChoiceResult, ChoiceRoute, ExecutionContext, LookAndSelectStep};
use super::util;
use crate::ability::debug::ABILITY_DEBUG;
use crate::ability::types::Command;
use crate::card::AbilityEffect;
use crate::game_state::GameState;
use std::sync::atomic::Ordering;

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
        eprintln!("[RPC] pending_commands_count={}", pending.len());
        for (idx, command) in pending.iter().enumerate() {
            match command {
                Command::Effect(effect) => {
                    self.spawn_context.target = effect.target.clone();
                    eprintln!(
                        "[RPC] executing effect action={} target={:?}",
                        effect.action, effect.target
                    );
                    self.execute_effect(gs, effect)?;
                    eprintln!(
                        "[RPC] after execute: pending={}",
                        self.pending_choice.is_some()
                    );
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
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.effect_started = true;
                }
                if idx + 1 < pending.len() {
                    let mut existing = gs.ability_queue.take_pending_commands();
                    existing.extend(pending[idx + 1..].to_vec());
                    gs.ability_queue.set_pending_commands(existing);
                }
                return Ok(());
            }
            if self.cancel_remaining_commands {
                // An optional sub-action (e.g. insufficient pay_energy) has flagged
                // that remaining commands should be dropped.
                self.cancel_remaining_commands = false;
                eprintln!("[RPC] cancel_remaining_commands set — aborting pending commands");
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
            log::debug!("Expired {} effects with duration 'live_end'", expired_count);
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
            .map(|choice| matches!(choice, Choice::SelectCard { zone, .. } if Zone::from_str(zone) == Some(Zone::LookedAt)))
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
        log::debug!(
            "[FINALIZE_CHOICE] pending={} selected={:?} context={:?}",
            has_pending_sequential,
            self.selected_cards,
            context
        );
        Ok(())
    }

    fn reveal_selected_looked_at(&mut self, gs: &mut GameState, indices: &[usize]) {
        let mut revealed_ids = Vec::new();
        for &idx in indices.iter() {
            if idx < gs.looked_at_cards.len() {
                let cid = gs.looked_at_cards[idx];
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
                    blind,
                    ..
                }),
                ChoiceResult::CardSelected { indices },
            ) => self.handle_select_card(
                gs,
                choice.as_ref().unwrap(),
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
                *blind,
                choice.as_ref().is_some_and(|c| {
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
        choice: &Choice,
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
        blind: bool,
        is_reveal: bool,
    ) -> Result<(), String> {
        if ABILITY_DEBUG.load(Ordering::Relaxed) {
            eprintln!(
                "[SEL_CARD] zone='{}' indices={:?} count={} allow_skip={} context={:?} is_reveal={}",
                zone, indices, count, allow_skip, context, is_reveal
            );
        }

        // Distinguish cost vs effect: cost handler only fires when effect NOT yet started.
        // effect_started is false during cost payment, true during effect execution.
        let effect_started = gs
            .ability_queue
            .current_entry()
            .is_some_and(|e| e.effect_started);

        // Handle reveal action: push selected cards to revealed_cards, don't discard.
        // Supports sequential multi-pick via re-prompt when indices.len() < count.
        if is_reveal && Zone::from_str(zone) == Some(Zone::Hand) {
            let target = target_player_id
                .clone()
                .unwrap_or_else(|| "self".to_string());

            // Resolve actual hand positions from filtered_indices (if any).
            // The initial reveal choice has no filtered_indices, so this is
            // a no-op on first call.  Any-number re-prompts (count==0) set
            // filtered_indices with REMAINING positions; fixed-count re-prompts
            // (count>0) use a different convention (storing already-selected
            // positions), so we don't map through filtered_indices for those.
            let hand_positions: Vec<usize> = if let Some(ref fi) = filtered_indices {
                if count == 0 {
                    indices.iter().filter_map(|&i| fi.get(i).copied()).collect()
                } else {
                    indices.to_vec()
                }
            } else {
                indices.to_vec()
            };

            // Re-prompt if we need more cards (sequential multi-pick, fixed count)
            if !hand_positions.is_empty() && count > 0 && hand_positions.len() < count {
                for &hp in &hand_positions {
                    if !self.selected_cards.contains(&(hp as i16)) {
                        self.selected_cards.push(hp as i16);
                    }
                }
                let remaining = count - hand_positions.len();
                self.pending_choice = Some(
                    Choice::select_cards(
                        Zone::Hand.to_str(),
                        remaining,
                        format!(
                            "Select {} more card(s) from hand{}",
                            remaining,
                            if blind { " (blind)" } else { "" }
                        ),
                        false,
                    )
                    .target_player_id(Some(target.clone()))
                    .blind(blind)
                    .is_reveal(true)
                    .filtered_indices(Some(
                        self.selected_cards.iter().map(|&i| i as usize).collect(),
                    ))
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }

            // Merge accumulated indices + current selection
            let mut all_indices: Vec<usize> =
                self.selected_cards.iter().map(|&i| i as usize).collect();
            self.selected_cards.clear();
            for &hp in &hand_positions {
                if !all_indices.contains(&hp) {
                    all_indices.push(hp);
                }
            }

            // Fixed-count (count > 0): push ALL accumulated cards on final batch.
            // Any-number (count == 0): push only current selection incrementally
            // (accumulation handled by self.selected_cards between rounds).
            let ids_to_reveal = if count == 0 {
                &hand_positions
            } else {
                &all_indices
            };
            let revealed_card_ids = {
                let player = gs.resolve_target_player_mut(&target);
                util::resolve_indices_to_ids(player, Zone::Hand.to_str(), ids_to_reveal)
            };

            for &cid in &revealed_card_ids {
                gs.revealed_cards.push(cid);
            }
            if !revealed_card_ids.is_empty() {
                let names: Vec<String> = revealed_card_ids
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
            if !effect_started {
                for &cid in &revealed_card_ids {
                    gs.revealed_cost_cards.push(cid);
                }
            }

            // Any-number re-prompt for cost-phase reveal:
            // after each non-empty selection, player can pick more or skip.
            // Store accumulated hand positions so they carry over to the next round.
            if count == 0 && allow_skip && !effect_started && !all_indices.is_empty() {
                self.selected_cards = all_indices.iter().map(|&i| i as i16).collect();
                let hand_len = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.hand.cards.len()
                };
                let remaining_indices: Vec<usize> = (0..hand_len)
                    .filter(|&i| !all_indices.contains(&i))
                    .collect();
                if !remaining_indices.is_empty() {
                    self.pending_choice = Some(
                        Choice::select_cards(
                            Zone::Hand.to_str(),
                            0,
                            "Select more cards to reveal from hand (or skip to finish)",
                            true,
                        )
                        .filtered_indices(Some(remaining_indices))
                        .card_type(card_type.clone())
                        .is_reveal(true)
                        .target_player_id(Some(target.clone()))
                        .build(),
                    );
                    self.store_pending_choice(gs);
                    return Ok(());
                }
            }

            self.clear_choice_state(gs);
            return self.resume_pending_commands(gs);
        }

        if !effect_started && gs.entry_cost().map(|c| c.action.as_str()) == Some("reveal") {
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
                gs.revealed_cost_cards.push(card_id);
            }
            return self.finalize_choice(gs, &context);
        }

        let card_db = gs.card_database.clone();
        let validate_filter = choice.as_filter();
        let mut validate_card =
            |cid: i16| -> bool { validate_filter.matches(&card_db, cid, false) };

        // Cost-phase zone handlers — only for actual cost payments (not effect selections).
        // Effect-phase multi-pick is handled by the effect-phase zone handlers below.
        let mut skip_discard_cleanup = false;
        // Enter cost-phase when: ability has a cost, and we're handling a hand choice
        // (either with picks, or with skip when allow_skip is true).
        log::debug!(
            "[COST_GATE] entry_cost={:?} allow_skip={} indices={:?} effect_started={}",
            gs.entry_cost().is_some(),
            allow_skip,
            indices,
            gs.ability_queue
                .current_entry()
                .is_some_and(|e| e.effect_started)
        );
        if gs.entry_cost().is_some() && (!effect_started || allow_skip || !indices.is_empty()) {
            // Cost-phase zone handler — only for hand choices where effect hasn't started,
            // or stage/other choices that need to be processed regardless.
            // Also allows empty-indices finalization from effect-phase if cost hasn't started.
            if !indices.is_empty() {
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.optional_cost_result = Some(true);
                }
            }
            log::debug!("[ZONE_MATCH] zone={} about to match effect-phase", zone);
            match Zone::from_str(zone) {
                Some(Zone::Hand) if gs.entry_cost().is_some() && !effect_started => {
                    let target = target_player_id
                        .clone()
                        .unwrap_or_else(|| "self".to_string());
                    let hand_cards: Vec<i16> = {
                        let p = gs.resolve_target_player_mut(&target);
                        p.hand.cards.to_vec()
                    };
                    // Cost-phase Hand handler: the frontend sends filtered-relative indices
                    // (positions within filtered_indices). Map to actual hand positions.
                    let cost_hand_indices: Vec<usize> = if let Some(ref fi) = filtered_indices {
                        indices.iter().filter_map(|&i| fi.get(i).copied()).collect()
                    } else {
                        indices.to_vec()
                    };
                    let new_card_ids: Vec<i16> = cost_hand_indices
                        .iter()
                        .filter_map(|&i| {
                            if i < hand_cards.len() {
                                Some(hand_cards[i])
                            } else {
                                None
                            }
                        })
                        .filter(|&cid| validate_card(cid))
                        .collect();
                    if !new_card_ids.is_empty() {
                        let player = gs.resolve_target_player_mut(&target);
                        let _ = util::move_cards(
                            player,
                            &new_card_ids,
                            Zone::Hand.to_str(),
                            Zone::Discard.to_str(),
                            None,
                            &card_db,
                        );
                        gs.mods.last_cost_discard_count += new_card_ids.len() as u32;
                        for &cid in &new_card_ids {
                            self.moved_cards.push(cid);
                            if !self.selected_cards.contains(&cid) {
                                self.selected_cards.push(cid);
                            }
                        }
                    }
                    log::debug!("[COST_HAND] indices={:?} count={} allow_skip={} new_cards={:?} moved_so_far={:?}", indices, count, allow_skip, new_card_ids, self.moved_cards);
                    // Empty indices: user is Done (or skipping entirely).
                    if new_card_ids.is_empty() {
                        log::debug!("[COST_HAND] empty indices branch");
                        if !self.moved_cards.is_empty() {
                            gs.mods.last_cost_discard_count = self.moved_cards.len() as u32;
                            gs.recently_moved_cards = Some(self.moved_cards.clone());
                            gs.recently_moved_from_zone = Some("hand".to_string());
                            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                                entry.cost_paid = true;
                                entry.optional_cost_result = Some(true);
                            }
                        } else if allow_skip {
                            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                                entry.cost_paid = true;
                                entry.optional_cost_result = Some(false);
                            }
                        }
                        self.pending_choice = None;
                        return Ok(());
                    }
                    // Sequential count-based: re-prompt if more picks allowed, else finalize.
                    if count > 0 && new_card_ids.len() < count {
                        let is_any_number = gs
                            .entry_cost()
                            .is_some_and(|c| c.count.is_none() || c.any_number.unwrap_or(false));
                        let remaining = count - new_card_ids.len();
                        log::debug!(
                            "[COST_HAND] re-prompt branch: count={} this_pick={} remaining={}",
                            count,
                            new_card_ids.len(),
                            remaining
                        );
                        let hand_now: Vec<i16> = {
                            let p = gs.resolve_target_player_mut(&target);
                            p.hand.cards.to_vec()
                        };
                        let available_idxs: Vec<usize> = (0..hand_now.len())
                            .filter(|i| validate_card(hand_now[*i]))
                            .collect();
                        self.pending_choice = Some(
                            Choice::select_cards(
                                Zone::Hand.to_str(),
                                remaining,
                                format!("Select {} more card(s) from hand for cost", remaining),
                                is_any_number || allow_skip,
                            )
                            .card_type(card_type.clone())
                            .cost_limit(cost_limit, cost_limit_operator.clone())
                            .cost_total(cost_total, cost_total_operator.clone())
                            .group(group.clone())
                            .characters(characters.clone())
                            .filtered_indices(if available_idxs.is_empty() {
                                None
                            } else {
                                Some(available_idxs)
                            })
                            .target_player_id(Some(target.clone()))
                            .build(),
                        );
                        self.store_pending_choice(gs);
                        return Ok(());
                    }
                    // any_number (count == 0) with at least one pick: re-prompt with skip
                    if count == 0 && allow_skip {
                        let hand_now: Vec<i16> = {
                            let p = gs.resolve_target_player_mut(&target);
                            p.hand.cards.to_vec()
                        };
                        let available_idxs: Vec<usize> = (0..hand_now.len())
                            .filter(|i| validate_card(hand_now[*i]))
                            .collect();
                        if available_idxs.is_empty() && !self.moved_cards.is_empty() {
                            let final_count = self.moved_cards.len() as u32;
                            gs.mods.last_cost_discard_count = final_count;
                            gs.recently_moved_cards = Some(self.moved_cards.clone());
                            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                                entry.cost_paid = true;
                                entry.optional_cost_result = Some(true);
                            }
                            self.pending_choice = None;
                            return Ok(());
                        }
                        self.pending_choice = Some(
                            Choice::select_cards(
                                Zone::Hand.to_str(),
                                0,
                                "Select more card(s) from hand for cost (or skip to finish)",
                                true,
                            )
                            .card_type(card_type.clone())
                            .cost_limit(cost_limit, cost_limit_operator.clone())
                            .cost_total(cost_total, cost_total_operator.clone())
                            .group(group.clone())
                            .characters(characters.clone())
                            .filtered_indices(if available_idxs.is_empty() {
                                None
                            } else {
                                Some(available_idxs)
                            })
                            .target_player_id(Some(target.clone()))
                            .build(),
                        );
                        self.store_pending_choice(gs);
                        return Ok(());
                    }
                    // Cap met or only one card left (any_number): finalize.
                    log::debug!(
                        "[COST_HAND] finalize branch: moved_so_far={:?}",
                        self.moved_cards
                    );
                    let final_count = self.moved_cards.len() as u32;
                    gs.mods.last_cost_discard_count = final_count;
                    gs.recently_moved_cards = Some(self.moved_cards.clone());
                    gs.recently_moved_from_zone = Some("hand".to_string());
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.cost_paid = true;
                    }
                    self.pending_choice = None;
                    return Ok(());
                }
                Some(Zone::Energy) if !effect_started => {
                    // Cost phase: pay energy (any-number selection).
                    let count_paid = indices.len();
                    if count_paid > 0 {
                        let player = gs.resolve_target_player_mut(
                            target_player_id.as_deref().unwrap_or("self"),
                        );
                        player.energy_zone.pay_energy(count_paid)?;
                        gs.mods.last_cost_energy_count += count_paid as u32;
                        if let Some(entry) = gs.ability_queue.current_entry_mut() {
                            entry.cost_paid = true;
                            entry.optional_cost_result = Some(true);
                        }
                    }
                    // Re-prompt for more, or skip to finish (any_number with count=0)
                    let energy_left = {
                        let player =
                            gs.resolve_target_player(target_player_id.as_deref().unwrap_or("self"));
                        player.energy_zone.active_energy_count
                    };
                    if count_paid > 0 && energy_left > 0 {
                        let filtered_indices: Vec<usize> = (0..energy_left).collect();
                        self.pending_choice = Some(
                            Choice::select_cards(
                                Zone::Energy.to_str().to_string(),
                                0,
                                format!(
                                    "Select energy card to pay (active: {}). Skip when done",
                                    energy_left
                                ),
                                true,
                            )
                            .filtered_indices(Some(filtered_indices))
                            .target_player_id(Some(
                                target_player_id
                                    .clone()
                                    .unwrap_or_else(|| "self".to_string()),
                            ))
                            .build(),
                        );
                        self.store_pending_choice(gs);
                        return Ok(());
                    }
                    self.clear_choice_state(gs);
                    return self.resume_pending_commands(gs);
                }
                Some(Zone::Stage) => {
                    // Effect phase: move selected stage card(s) to destination zone.
                    // When is_select_action is true, store card IDs without moving
                    // (the actual move will be handled by a subsequent effect).
                    if is_select_action {
                        let stage_indices: Vec<usize> = if let Some(ref fidx) = filtered_indices {
                            indices
                                .iter()
                                .filter_map(|&i| fidx.get(i).copied())
                                .collect()
                        } else {
                            indices.to_vec()
                        };
                        let player = gs.resolve_target_player_mut(
                            target_player_id.as_deref().unwrap_or("self"),
                        );
                        for &idx in &stage_indices {
                            if idx < 3 && player.stage.stage[idx] != -1 {
                                let cid = player.stage.stage[idx];
                                if validate_card(cid) && !self.selected_cards.contains(&cid) {
                                    self.selected_cards.push(cid);
                                }
                            }
                        }
                        self.clear_choice_state(gs);
                        // If no valid targets were selected, don't execute the
                        // pending command — the effect is effectively cancelled.
                        if stage_indices.is_empty() {
                            return Ok(());
                        }
                        return self.resume_pending_commands(gs);
                    }
                    // Non-is_select_action: actually move the card(s) to destination.
                    // Same logic as the OUTER_MATCH Stage handler.
                    let dst = gs.entry_destination().map(|s| s.to_string());
                    let dst_str = dst.as_deref().unwrap_or(Zone::Discard.to_str()).to_string();
                    let player = gs.active_player_mut();
                    let card_ids =
                        util::resolve_indices_to_ids(player, Zone::Stage.to_str(), indices);
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
                    let moved_count = util::move_cards(
                        player,
                        &valid_ids,
                        Zone::Stage.to_str(),
                        &dst_str,
                        None,
                        &card_db,
                    );
                    if moved_count > 0 {
                        if let Some(pos) = last_vacated {
                            gs.last_vacated_stage_area = Some(pos);
                        }
                        self.selected_cards = valid_ids.clone();
                        self.moved_cards = valid_ids.clone();
                        gs.recently_moved_cards = Some(valid_ids);
                        gs.recently_moved_from_zone = Some("stage".to_string());
                    }
                    self.clear_choice_state(gs);
                    return self.resume_pending_commands(gs);
                }
                Some(Zone::Energy) => {
                    self.mark_energy_as_wait(gs, indices, &mut validate_card);
                }
                Some(Zone::Discard) => {
                    // When effect has started, skip cost handling so the effect
                    // handler's discard arm (with position choice persistence) runs instead.
                    if effect_started {
                        skip_discard_cleanup = true;
                    } else if is_select_action {
                        let mapped_indices: Vec<usize> = if let Some(ref fidx) = filtered_indices {
                            indices
                                .iter()
                                .filter_map(|&i| fidx.get(i).copied())
                                .collect()
                        } else {
                            indices.to_vec()
                        };
                        let player = gs.active_player_mut();
                        let mut cards: Vec<i16> = Vec::new();
                        for &i in mapped_indices.iter() {
                            if i < player.waitroom.cards.len() {
                                let cid = player.waitroom.cards[i];
                                if validate_card(cid) {
                                    cards.push(cid);
                                }
                            }
                        }
                        for &cid in &cards {
                            if !self.selected_cards.contains(&cid) {
                                self.selected_cards.push(cid);
                            }
                        }
                    } else {
                        let mapped_indices: Vec<usize> = if let Some(ref fidx) = filtered_indices {
                            indices
                                .iter()
                                .filter_map(|&i| fidx.get(i).copied())
                                .collect()
                        } else {
                            indices.to_vec()
                        };
                        self.execute_selected_cards_from_zone(
                            gs,
                            Zone::Discard.to_str(),
                            &mapped_indices,
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
                        // Persist sub-choice (e.g. SelectPosition for empty_area placement)
                        if self.pending_choice.is_some() {
                            self.store_pending_choice(gs);
                        }
                        return self.finalize_choice(gs, &context);
                    }
                }
                Some(Zone::Deck) => self.execute_selected_cards_from_zone(
                    gs,
                    Zone::Deck.to_str(),
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
                Some(Zone::LookedAt) => {
                    log::debug!("[HSC1_LOOKED_AT] START: looked_cards.len()={}, indices={:?}, filtered_indices={:?}, is_select_cards={}",
                        gs.looked_at_cards.len(), indices, filtered_indices, gs.ability_queue.current_entry()
                            .and_then(|e| e.ability.effect.as_ref())
                            .and_then(|ef| ef.compound.select_action.as_ref())
                            .map(|sa| {
                                log::debug!("[HSC1_SA] sa.action={:?}, compound.actions.len={:?}", sa.action, sa.compound.actions.as_ref().map(|a| a.len()));
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
                        if matches!(self.pending_choice, Some(Choice::SelectTarget { ref target, .. }) if super::enums::SelectTargetKind::from_str(target) == Some(super::enums::SelectTargetKind::Order))
                        {
                            return Ok(());
                        }
                    } else if let ExecutionContext::LookAndSelect { .. } = self.execution_context {
                        let _ = select_action_entry.is_some();
                        self.handle_select_cards_looked_at(gs, &mapped_indices)?;
                    } else {
                        self.handle_select_cards_looked_at(gs, &mapped_indices)?;
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
                                Zone::LookedAt.to_str(),
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
                Some(Zone::RevealedCards) => {
                    let dst = gs.entry_destination().map(|s| s.to_string());
                    let dst_str = dst.as_deref().unwrap_or(Zone::Hand.to_str());
                    let moved = self.move_from_revealed(gs, indices, &mut validate_card, dst_str);
                    // If discard_remaining is set, move non-selected revealed cards to discard.
                    // Check self.current_effect (the inner sub-action effect) rather than
                    // gs.entry_effect() (the outer ability effect) — discard_remaining is
                    // on the select_cards sub-action, not the outer sequential.
                    if self
                        .current_effect
                        .as_ref()
                        .is_some_and(|e| e.discard_remaining.unwrap_or(false))
                    {
                        let remaining: Vec<i16> = gs
                            .revealed_cards
                            .iter()
                            .filter(|&&cid| !moved.contains(&cid))
                            .copied()
                            .collect();
                        let player = gs.resolve_target_player_mut("self");
                        for &cid in &remaining {
                            player.waitroom.add_card(cid);
                        }
                        gs.revealed_cards.clear();
                    }
                    return self.finalize_choice(gs, &context);
                }
                Some(Zone::UnderMember) => {
                    let dst = gs.entry_destination().map(|s| s.to_string());
                    let dst_str = dst
                        .as_deref()
                        .unwrap_or(Zone::EnergyDeck.to_str())
                        .to_string();
                    let tgt = target_player_id
                        .clone()
                        .unwrap_or_else(|| "self".to_string());
                    self.move_from_under_member(gs, indices, &mut validate_card, &dst_str, &tgt)?;
                    return self.finalize_choice(gs, &context);
                }
                _ => {
                    // When effect has started, don't early-return — fall through
                    // to OUTER_MATCH where effect-phase zone handlers live.
                    if !effect_started {
                        self.clear_choice_state(gs);
                        return self.resume_pending_commands(gs);
                    }
                }
            }
            if !skip_discard_cleanup {
                // Don't early-return for zones with accumulated cards on skip:
                // we need to fall through to execute them.
                let has_accumulated = !self.selected_cards.is_empty();
                let needs_fallthrough = match zone {
                    "hand" => true,
                    "discard" => has_accumulated || effect_started,
                    "looked_at" => effect_started,
                    "stage" => effect_started,
                    "under_member" => effect_started,
                    _ => effect_started,
                };
                if !needs_fallthrough && !effect_started {
                    self.clear_choice_state(gs);
                    if gs.ability_queue.has_pending_commands() {
                        return self.resume_pending_commands(gs);
                    }
                    return Ok(());
                }
            }
        }

        if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!(
                "[OUTER_MATCH] zone={} effect_started={} pending_commands={}",
                zone,
                effect_started,
                gs.ability_queue.has_pending_commands()
            );
        }
        match Zone::from_str(zone) {
            Some(Zone::Hand) => {
                log::debug!("[HAND_START] entering effect-phase Hand handler");
                let mapped_indices: Vec<usize> = if let Some(ref fidx) = filtered_indices {
                    indices
                        .iter()
                        .filter_map(|&i| fidx.get(i).copied())
                        .collect()
                } else {
                    indices.to_vec()
                };
                let hand_idx = if mapped_indices.is_empty() && !allow_skip && count > 0 {
                    return Err("No cards selected from hand for required selection".to_string());
                } else {
                    &mapped_indices
                };
                log::debug!(
                    "[HAND_HANDLER] hand_idx={:?} count={} allow_skip={}",
                    hand_idx,
                    count,
                    allow_skip
                );
                if !hand_idx.is_empty() || allow_skip {
                    if !hand_idx.is_empty() && count > 0 && hand_idx.len() < count {
                        log::debug!("[COUNT_PROMPT] count={} len={}", count, hand_idx.len());
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
                        // and preserving target, blind, and is_reveal for cross-player/opponent hand.
                        let remaining = count - hand_idx.len();
                        // Compute available indices (all valid indices minus already-selected).
                        // filtered_indices maps selection index → actual hand index, so providing
                        // only the not-yet-selected indices prevents re-picking the same card.
                        let available_idxs: Vec<usize> = (0..hand_cards.len())
                            .filter(|i| !all_hand_idxs.contains(i))
                            .collect();
                        self.pending_choice = Some(
                            Choice::select_cards(
                                Zone::Hand.to_str(),
                                remaining,
                                format!(
                                    "Select {} more card(s) from hand{}",
                                    remaining,
                                    if blind { " (blind)" } else { "" }
                                ),
                                false,
                            )
                            .card_type(card_type.clone())
                            .cost_limit(cost_limit, cost_limit_operator.clone())
                            .cost_total(cost_total, cost_total_operator.clone())
                            .group(group.clone())
                            .characters(characters.clone())
                            .filtered_indices(if available_idxs.is_empty() {
                                None
                            } else {
                                Some(available_idxs)
                            })
                            .target_player_id(Some(target.clone()))
                            .blind(blind)
                            .is_reveal(is_reveal)
                            .build(),
                        );
                        self.store_pending_choice(gs);
                        return Ok(());
                    }
                    // any_number re-prompt: after each non-empty selection, user can pick more or skip.
                    // Move current selection to destination immediately, then re-prompt.
                    if !hand_idx.is_empty() && count == 0 && allow_skip {
                        log::debug!("[ANY_ENTERED] count={} hand_idx={:?}", count, hand_idx);
                        let target = target_player_id.as_deref().unwrap_or("self").to_string();
                        // Move current selection to destination immediately
                        self.execute_selected_cards_from_zone(
                            gs,
                            Zone::Hand.to_str(),
                            hand_idx,
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
                        // Re-read hand state after movement
                        let hand_cards: Vec<i16> = {
                            let p = gs.resolve_target_player_mut(&target);
                            p.hand.cards.to_vec()
                        };
                        for &idx in hand_idx {
                            if idx < hand_cards.len()
                                && !self.selected_cards.contains(&hand_cards[idx])
                            {
                                self.selected_cards.push(hand_cards[idx]);
                            }
                        }
                        let exclude_idxs: Vec<usize> = (0..hand_cards.len())
                            .filter(|i| !validate_card(hand_cards[*i]))
                            .collect();
                        self.pending_choice = Some(
                            Choice::select_cards(
                                Zone::Hand.to_str(),
                                0,
                                "Select more card(s) from hand (or skip to finish)",
                                true,
                            )
                            .card_type(card_type.clone())
                            .cost_limit(cost_limit, cost_limit_operator.clone())
                            .cost_total(cost_total, cost_total_operator.clone())
                            .group(group.clone())
                            .characters(characters.clone())
                            .filtered_indices(if exclude_idxs.is_empty() {
                                None
                            } else {
                                Some(exclude_idxs)
                            })
                            .target_player_id(Some(target))
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
                        Zone::Hand.to_str(),
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
                    )?;
                    self.selected_cards.clear();
                }
                // Track whether optional cost was actually paid.
                // `self.moved_cards` is set by finalize_card_movement when cards
                // are actually moved, covering the skip-with-accumulated case.
                if allow_skip {
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.optional_cost_result =
                            Some(!indices.is_empty() || !self.moved_cards.is_empty());
                    }
                    // When optional cost is skipped (count > 0), discard pending "そうした場合" actions.
                    // These were saved by execute_sequential_effect and should NOT run
                    // when the user chose not to pay the optional cost.
                    // Effect any_number selections (count == 0) should NOT drain — sequential continues.
                    // EXCEPTION: opponent actions — when opponent skips, the conditional_on_optional
                    // MUST fire (it has the "if you didn't do so" effect like blade gain).
                    let is_opponent_action = target_player_id.as_deref() == Some("opponent");
                    if indices.is_empty()
                        && count > 0
                        && !is_opponent_action
                        && self.moved_cards.is_empty()
                    {
                        gs.ability_queue.take_pending_commands();
                    }
                }
            }
            Some(Zone::Deck) => self.execute_selected_cards_from_zone(
                gs,
                Zone::Deck.to_str(),
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
            Some(Zone::Discard) => {
                // Map user's filtered-relative indices to actual waitroom indices.
                // The frontend sends indices relative to filtered_indices (0 = first
                // selectable card). Without this mapping, selecting a non-trivial
                // filtered set (e.g. after budget filtering) picks the wrong card.
                let mapped_indices: Vec<usize> = if let Some(ref fidx) = filtered_indices {
                    indices
                        .iter()
                        .filter_map(|&i| fidx.get(i).copied())
                        .collect()
                } else {
                    indices.to_vec()
                };
                if is_select_action {
                    // Just store card IDs without moving
                    let target = target_player_id.as_deref().unwrap_or("self");
                    let player = gs.resolve_target_player_mut(target);
                    let mut cards: Vec<i16> = Vec::new();
                    for &i in mapped_indices.iter() {
                        if i < player.waitroom.cards.len() {
                            cards.push(player.waitroom.cards[i]);
                        }
                    }
                    self.selected_cards = cards;
                    // Finalize to process pending sequential actions
                    return self.finalize_choice(gs, &context);
                } else {
                    // Sequential multi-pick: if fewer indices than count, re-prompt
                    if !mapped_indices.is_empty() && count > 0 && mapped_indices.len() < count {
                        let target = target_player_id.as_deref().unwrap_or("self").to_string();
                        let waitroom_cards: Vec<i16> = {
                            let p = gs.resolve_target_player_mut(&target);
                            p.waitroom.cards.to_vec()
                        };
                        // Build filtered indices for re-prompt display (exclude already-selected).
                        let mut filtered_idxs: Vec<usize> = Vec::new();
                        let prev_ids = self.selected_cards.clone();
                        if !prev_ids.is_empty() {
                            for (idx, cid) in waitroom_cards.iter().enumerate() {
                                if prev_ids.contains(cid) && !filtered_idxs.contains(&idx) {
                                    filtered_idxs.push(idx);
                                }
                            }
                        }
                        // Track current selection as accumulated card IDs
                        let current_card_ids: Vec<i16> = mapped_indices
                            .iter()
                            .filter_map(|&idx| {
                                if idx < waitroom_cards.len() {
                                    Some(waitroom_cards[idx])
                                } else {
                                    None
                                }
                            })
                            .collect();
                        for &cid in &current_card_ids {
                            if !self.selected_cards.contains(&cid) {
                                self.selected_cards.push(cid);
                            }
                        }
                        // Move current selection to destination immediately
                        // (same pattern as hand handler re-prompt for discard zone).
                        self.execute_selected_cards_from_zone(
                            gs,
                            Zone::Discard.to_str(),
                            &mapped_indices,
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
                        // If a sub-choice was created (e.g. SelectPosition for stage),
                        // save the re-prompt as a pending command instead of overwriting
                        // the sub-choice. The sub-choice must be resolved first.
                        if self.sub_choice_created {
                            self.sub_choice_created = false;
                            let remaining = count - mapped_indices.len();
                            if remaining > 0 {
                                let waitroom_cards: Vec<i16> = {
                                    let p = gs.resolve_target_player_mut(&target);
                                    p.waitroom.cards.to_vec()
                                };
                                let spent: u32 = self
                                    .selected_cards
                                    .iter()
                                    .filter_map(|&cid| {
                                        gs.card_database.get_card(cid).and_then(|c| c.cost)
                                    })
                                    .sum();
                                let remaining_budget =
                                    cost_total.map(|tb| tb.saturating_sub(spent));
                                let use_budget_filter = match cost_total_operator.as_deref() {
                                    Some(op) => op == "<=",
                                    None => cost_total.is_some(),
                                };
                                let all_idxs: Vec<usize> = if use_budget_filter {
                                    let rb = remaining_budget.unwrap_or(0);
                                    waitroom_cards
                                        .iter()
                                        .enumerate()
                                        .filter(|(_, &cid)| {
                                            gs.card_database
                                                .get_card(cid)
                                                .and_then(|c| c.cost)
                                                .unwrap_or(99)
                                                <= rb
                                        })
                                        .map(|(idx, _)| idx)
                                        .collect()
                                } else {
                                    (0..waitroom_cards.len()).collect()
                                };
                                if !all_idxs.is_empty() || allow_skip {
                                    let reprompt = Choice::select_cards(
                                        Zone::Discard.to_str(),
                                        remaining,
                                        format!(
                                            "Select {} more card(s) from discard{}",
                                            remaining,
                                            if allow_skip {
                                                " (or skip to finish)"
                                            } else {
                                                ""
                                            }
                                        ),
                                        allow_skip,
                                    )
                                    .card_type(card_type.clone())
                                    .cost_limit(cost_limit, cost_limit_operator.clone())
                                    .cost_total(remaining_budget, Some("<=".to_string()))
                                    .group(group.clone())
                                    .characters(characters.clone())
                                    .filtered_indices(if all_idxs.is_empty() {
                                        Some(vec![])
                                    } else {
                                        Some(all_idxs)
                                    })
                                    .target_player_id(Some(target))
                                    .build();
                                    let mut pending = gs.ability_queue.take_pending_commands();
                                    pending.insert(0, Command::Choice(reprompt));
                                    gs.ability_queue.set_pending_commands(pending);
                                }
                            }
                            return Ok(());
                        }
                        // Re-read waitroom for budget retainer (zone shifted after movement)
                        let waitroom_cards: Vec<i16> = {
                            let p = gs.resolve_target_player_mut(&target);
                            p.waitroom.cards.to_vec()
                        };
                        // Build whitelist from the current waitroom state, filtered by remaining budget.
                        let spent: u32 = self
                            .selected_cards
                            .iter()
                            .filter_map(|&cid| gs.card_database.get_card(cid).and_then(|c| c.cost))
                            .sum();
                        let remaining_budget = cost_total.map(|tb| tb.saturating_sub(spent));
                        let use_budget_filter = match cost_total_operator.as_deref() {
                            Some(op) => op == "<=",
                            None => cost_total.is_some(),
                        };
                        let all_idxs: Vec<usize> = if use_budget_filter {
                            let rb = remaining_budget.unwrap_or(0);
                            waitroom_cards
                                .iter()
                                .enumerate()
                                .filter(|(_, &cid)| {
                                    gs.card_database
                                        .get_card(cid)
                                        .and_then(|c| c.cost)
                                        .unwrap_or(99)
                                        <= rb
                                })
                                .map(|(idx, _)| idx)
                                .collect()
                        } else {
                            (0..waitroom_cards.len()).collect()
                        };
                        // Re-prompt with remaining count, showing only cards within budget.
                        // Pass `allow_skip` through so "up to N" semantics are preserved.
                        let remaining = count - mapped_indices.len();
                        self.pending_choice = Some(
                            Choice::select_cards(
                                Zone::Discard.to_str(),
                                remaining,
                                format!(
                                    "Select {} more card(s) from discard{}",
                                    remaining,
                                    if allow_skip {
                                        " (or skip to finish)"
                                    } else {
                                        ""
                                    }
                                ),
                                allow_skip,
                            )
                            .card_type(card_type.clone())
                            .cost_limit(cost_limit, cost_limit_operator.clone())
                            .cost_total(remaining_budget, Some("<=".to_string()))
                            .group(group.clone())
                            .characters(characters.clone())
                            .filtered_indices(if all_idxs.is_empty() {
                                Some(vec![])
                            } else {
                                Some(all_idxs)
                            })
                            .target_player_id(Some(target))
                            .build(),
                        );
                        self.store_pending_choice(gs);
                        return Ok(());
                    }
                    // Final batch: merge accumulated cards with current indices and execute
                    let target = target_player_id.as_deref().unwrap_or("self").to_string();
                    let mut all_idxs: Vec<usize> = mapped_indices.to_vec();
                    let prev_ids = self.selected_cards.clone();
                    if !prev_ids.is_empty() {
                        let waitroom_cards: Vec<i16> = {
                            let p = gs.resolve_target_player_mut(&target);
                            p.waitroom.cards.to_vec()
                        };
                        for (idx, cid) in waitroom_cards.iter().enumerate() {
                            if prev_ids.contains(cid) && !all_idxs.contains(&idx) {
                                all_idxs.push(idx);
                            }
                        }
                        self.selected_cards.clear();
                    }
                    self.execute_selected_cards_from_zone(
                        gs,
                        Zone::Discard.to_str(),
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
                    )?;
                    // Persist sub-choice (e.g. SelectPosition).
                    // finalize_choice preserves pending_choice when sub_choice_created is true.
                    if self.sub_choice_created {
                        self.store_pending_choice(gs);
                    }
                }
            }
            Some(Zone::LookedAt) => {
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

                if is_select_cards {
                    // Process selection first (moves selected cards to destination,
                    // restores remaining for re-prompt), THEN check for re-prompt.
                    self.handle_select_cards_looked_at(gs, &valid)?;

                    // Preserve order prompt from handle_select_cards_looked_at
                    if matches!(self.pending_choice, Some(Choice::SelectTarget { ref target, .. }) if super::enums::SelectTargetKind::from_str(target) == Some(super::enums::SelectTargetKind::Order))
                    {
                        return Ok(());
                    }

                    // Re-prompt for remaining cards
                    if is_select_cards && !valid.is_empty() {
                        let max_count = count;
                        let selected_count = valid.len();
                        let remaining = gs.looked_at_cards.len();
                        if max_count > selected_count && remaining > 0 {
                            let remaining_max = max_count - selected_count;
                            let ct = card_type.clone();
                            self.pending_choice = Some(
                                Choice::select_cards(
                                    Zone::LookedAt.to_str(),
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
                } else {
                    self.handle_select_cards_looked_at(gs, &valid)?;
                }
                return self.finalize_choice(gs, &context);
            }
            Some(Zone::RevealedCards) => {
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
                let dst_str = dst.as_deref().unwrap_or(Zone::Hand.to_str());
                let player = gs.active_player_mut();
                for &cid in &self.selected_cards.clone() {
                    crate::ability::util::place_card_in_zone(player, cid, dst_str, None, false, 1);
                }
            }
            Some(Zone::Energy) => {
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
            Some(Zone::SelectedCards) => {
                log::debug!(
                    "[SELECTED_CARDS_BEFORE] self.selected_cards={:?} indices={:?}",
                    self.selected_cards,
                    indices
                );
                let mut cards = Vec::new();
                for &i in indices.iter() {
                    if i < self.selected_cards.len() {
                        cards.push(self.selected_cards[i]);
                    }
                }
                self.selected_cards = cards;
                log::debug!(
                    "[SELECTED_CARDS_AFTER] self.selected_cards={:?}",
                    self.selected_cards
                );
            }
            Some(Zone::Stage) => {
                if is_select_action {
                    // Select card(s) for a state change or similar effect
                    // without moving them off stage.
                    // When the user makes no selection (skip or no valid targets),
                    // discard pending re-apply commands to avoid re-prompting
                    // in an infinite loop and to prevent the effect from firing.
                    if indices.is_empty() {
                        gs.ability_queue.take_pending_commands();
                        self.selected_cards = vec![];
                        log::debug!("[SELECT_STAGE] no selection: cleared pending commands");
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
                    log::debug!(
                        "[SELECT_STAGE] is_select_action=true cards={:?} filtered_idx={:?}",
                        cards,
                        filtered_indices
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
                    let dst_str = dst.as_deref().unwrap_or(Zone::Discard.to_str()).to_string();
                    let player = gs.active_player_mut();
                    let card_ids =
                        util::resolve_indices_to_ids(player, Zone::Stage.to_str(), indices);
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
                    let moved_count = util::move_cards(
                        player,
                        &valid_ids,
                        Zone::Stage.to_str(),
                        &dst_str,
                        None,
                        &card_db,
                    );
                    if moved_count > 0 {
                        if let Some(pos) = last_vacated {
                            gs.last_vacated_stage_area = Some(pos);
                        }
                        self.selected_cards = valid_ids.clone();
                        self.moved_cards = valid_ids.clone();
                        gs.recently_moved_cards = Some(valid_ids);
                        gs.recently_moved_from_zone = Some("stage".to_string());
                    }
                }
            }
            Some(Zone::UnderMember) => {
                let dst = gs.entry_destination().map(|s| s.to_string());
                let dst_str = dst
                    .as_deref()
                    .unwrap_or(Zone::EnergyDeck.to_str())
                    .to_string();
                let tgt = target_player_id
                    .clone()
                    .unwrap_or_else(|| "self".to_string());
                self.move_from_under_member(gs, indices, &mut validate_card, &dst_str, &tgt)?;
            }
            Some(Zone::SuccessLiveZone) => {
                let mapped: Vec<usize> = if let Some(ref fidx) = filtered_indices {
                    indices
                        .iter()
                        .filter_map(|&i| fidx.get(i).copied())
                        .collect()
                } else {
                    indices.to_vec()
                };
                let dst = gs.entry_destination().map(|s| s.to_string());
                let dst_str = dst.as_deref().unwrap_or(Zone::Discard.to_str()).to_string();
                let target = target_player_id.as_deref().unwrap_or("self");
                let player = gs.resolve_target_player_mut(target);
                let card_ids =
                    util::resolve_indices_to_ids(player, Zone::SuccessLiveZone.to_str(), &mapped);
                if !card_ids.is_empty() {
                    let valid_ids: Vec<i16> = card_ids
                        .into_iter()
                        .filter(|&cid| validate_card(cid))
                        .collect();
                    if !valid_ids.is_empty() {
                        let mc = util::move_cards(
                            player,
                            &valid_ids,
                            Zone::SuccessLiveZone.to_str(),
                            &dst_str,
                            None,
                            &card_db,
                        );
                        if mc > 0 {
                            self.moved_cards = valid_ids.clone();
                            gs.recently_moved_cards = Some(valid_ids);
                            gs.recently_moved_from_zone =
                                Some(Zone::SuccessLiveZone.to_str().to_string());
                        }
                    }
                }
                self.clear_choice_state(gs);
                let pending = gs.ability_queue.take_pending_commands();
                let filtered: Vec<Command> = pending
                    .into_iter()
                    .filter(|cmd| match cmd {
                        Command::Effect(e) => e.source.as_deref() != Some("success_live_zone"),
                        _ => true,
                    })
                    .collect();
                gs.ability_queue.set_pending_commands(filtered);
                return self.resume_pending_commands(gs);
            }
            _ => log::debug!("Card selection from zone '{}' not yet implemented", zone),
        }
        log::debug!(
            "▶ Select: {} card(s) selected from zone={:?} → [{}]",
            self.selected_cards.len(),
            zone,
            self.fmt_ids(&self.selected_cards)
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

        // choice_card_no-based routing
        match choice_card_no.as_ref() {
            Some(ChoiceRoute::Choice) => {
                if let Some(ref options_json) = conditional_choice {
                    if let Ok(all_options) =
                        serde_json::from_str::<Vec<AbilityEffect>>(options_json)
                    {
                        let idx: usize = selected.parse().unwrap_or(0);
                        if idx < all_options.len() {
                            let selected_effect = all_options[idx].clone();
                            let mut remaining = all_options.clone();
                            remaining.remove(idx);
                            // Schedule the selected effect and optional re-prompt
                            // as pending commands. resume_pending_commands runs them
                            // sequentially — after the selected effect completes (and
                            // any sub-choices), the re-prompt command re-enters
                            // execute_choice which sees the updated conditional_choice
                            // JSON array and creates a new SelectTarget for the
                            // remaining options.
                            let mut commands = vec![Command::Effect(selected_effect)];
                            let wants_re_prompt = !remaining.is_empty()
                                && gs.entry_effect().map_or(false, |eff| {
                                    if eff.any_number.unwrap_or(false) {
                                        return true;
                                    }
                                    if let Some(ref alt_cond) = eff.compound.alternative_condition {
                                        let ctx = ConditionContext::with_moved_cards(
                                            gs,
                                            &self.moved_cards,
                                        );
                                        ctx.evaluate_condition(alt_cond)
                                            && eff.alternative_count_type.as_deref()
                                                == Some("any_number")
                                    } else {
                                        false
                                    }
                                });
                            if wants_re_prompt {
                                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                                    entry.conditional_choice =
                                        serde_json::to_string(&remaining).ok();
                                }
                                let desc: Vec<String> = remaining
                                    .iter()
                                    .map(|o| {
                                        o.answers
                                            .as_ref()
                                            .map(|a| a.join(", "))
                                            .unwrap_or_else(|| o.text.clone())
                                    })
                                    .collect();
                                commands.push(Command::Choice(Choice::SelectTarget {
                                    target: "choice".to_string(),
                                    description: desc.join(" / "),
                                    allow_skip: true,
                                    options: None,
                                }));
                            }
                            let _n_cmds = commands.len();
                            gs.ability_queue.set_pending_commands(commands);
                        }
                    }
                }
                self.pending_choice = None;
                return self.resume_pending_commands(gs);
            }
            Some(ChoiceRoute::ChoiceString) => {
                return self.handle_choice_string_selection(gs, selected, conditional_choice);
            }
            Some(ChoiceRoute::Raw(s)) if s.starts_with("position_change") => {
                return self.handle_position_change_choice(gs, choice_card_no, selected);
            }
            _ => {}
        }

        // target-based routing via typed enum
        // Note: Choice and ChoiceString are handled by choice_card_no above
        match super::enums::SelectTargetKind::from_str(target) {
            Some(super::enums::SelectTargetKind::Choice) => {
                self.clear_choice_state(gs);
                return Ok(());
            }
            Some(super::enums::SelectTargetKind::ChoiceString) => {
                return self.handle_choice_string_store(gs, selected, conditional_choice);
            }
            Some(super::enums::SelectTargetKind::PayOptionalCostSkipOptionalCost) => {
                return self.handle_optional_cost_payment(gs, selected);
            }
            Some(super::enums::SelectTargetKind::DoubleBatonTouch) => {
                return self.handle_double_baton_touch(gs, selected);
            }
            Some(super::enums::SelectTargetKind::PrimaryAlternative) => {
                return self.handle_primary_alternative(gs, selected);
            }
            Some(super::enums::SelectTargetKind::ApplyReplacement) => {
                self.clear_choice_state(gs);
                return Ok(());
            }
            Some(super::enums::SelectTargetKind::ChooseRequiredHearts) => {
                gs.prohibition_effects
                    .push(format!("chosen_required_hearts:{}", selected));
                self.clear_choice_state(gs);
                return Ok(());
            }
            Some(super::enums::SelectTargetKind::PositionDestination) => {
                return self.handle_position_destination(gs, selected);
            }
            Some(super::enums::SelectTargetKind::HeartColor) => {
                return self.handle_heart_color_selection(gs, selected);
            }
            Some(super::enums::SelectTargetKind::ChoiceType) => {
                self.clear_choice_state(gs);
                return Ok(());
            }
            Some(super::enums::SelectTargetKind::ChoiceCondition) => {
                return self.handle_choice_condition(gs, selected);
            }
            Some(super::enums::SelectTargetKind::ConditionalOptional) => {
                return self.handle_conditional_optional(gs, selected);
            }
            Some(super::enums::SelectTargetKind::DrawAnyNumber) => {
                return self.handle_draw_any_number(gs, selected);
            }
            Some(super::enums::SelectTargetKind::Order) => {
                return self.handle_order_selection(gs, selected);
            }
            Some(super::enums::SelectTargetKind::SelfOrOpponent) => {
                let chosen = match selected {
                    "自分" => "self",
                    "相手" => "opponent",
                    _ => return Err("Invalid choice for SelfOrOpponent".to_string()),
                };
                eprintln!("[SELFOR] chosen={}", chosen);
                self.spawn_context.target = Some(chosen.to_string());
                if let Some(ref current) = self.current_effect {
                    eprintln!(
                        "[SELFOR] current.action={} steps={:?}",
                        current.action,
                        current.effect_steps.as_ref().map(|s| s.len())
                    );
                    if let Some(ref steps) = current.effect_steps {
                        if let Some(inner) = steps.first() {
                            let mut modified = inner.clone();
                            eprintln!(
                                "[SELFOR] inner.action={} steps_before={}",
                                modified.action,
                                modified.effect_steps.as_ref().map(|s| s.len()).unwrap_or(0)
                            );
                            Self::set_chosen_target(&mut modified, chosen);
                            eprintln!(
                                "[SELFOR] after_set: target={:?} has_la={} has_sa={}",
                                modified.target,
                                modified.compound.look_action.is_some(),
                                modified.compound.select_action.is_some()
                            );
                            self.pending_choice = None;
                            gs.ability_queue
                                .set_pending_commands(vec![Command::Effect(modified)]);
                            let res = self.resume_pending_commands(gs);
                            eprintln!(
                                "[SELFOR] after resume: pending={:?} res={:?}",
                                self.pending_choice.is_some(),
                                res
                            );
                            match res {
                                Ok(()) => return Ok(()),
                                Err(e) => {
                                    eprintln!("[SELFOR] inner effect failed: {}", e);
                                    self.clear_choice_state(gs);
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
                eprintln!("[SELFOR] no inner effect found");
                self.clear_choice_state(gs);
                return Ok(());
            }
            None => {}
        }

        self.clear_choice_state(gs);
        Ok(())
    }

    fn handle_draw_any_number(&mut self, gs: &mut GameState, selected: &str) -> Result<(), String> {
        let count: usize = selected.parse().unwrap_or(0);
        if let Some(effect) = gs.entry_effect().cloned() {
            let source = effect.source.as_deref().unwrap_or(Zone::Deck.to_str());
            let destination = effect.destination.as_deref().unwrap_or(Zone::Hand.to_str());
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
            if let LookAndSelectStep::Finalize { destination, .. } = step {
                if Zone::from_str(&destination) == Some(Zone::Deck) {
                    if let Ok(idx) = selected.parse::<usize>() {
                        if idx < gs.looked_at_cards.len() {
                            let card = gs.looked_at_cards.remove(idx);
                            gs.looked_at_cards.insert(0, card);
                        }
                    }
                    let card_ids: Vec<i16> = gs.looked_at_cards.iter().rev().copied().collect();
                    let target = self
                        .spawn_context
                        .target
                        .clone()
                        .or_else(|| gs.entry_effect().and_then(|e| e.target.clone()))
                        .unwrap_or_else(|| "self".to_string());
                    let player = gs.resolve_target_player_mut(&target);
                    for card_id in card_ids {
                        player.main_deck.cards.insert(0, card_id);
                    }
                    gs.looked_at_cards.clear();
                }
            }
        }
        self.clear_choice_state(gs);
        Ok(())
    }

    fn handle_position_change_choice(
        &mut self,
        gs: &mut GameState,
        choice_card_no: Option<ChoiceRoute>,
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
            if let Some(ChoiceRoute::Raw(ref raw)) = choice_card_no {
                if let Some(tgt) = raw.strip_prefix("position_change:") {
                    if tgt == "opponent:front" {
                        modified.source_position = Some(selected.to_string());
                        if let Err(e) =
                            self.execute_position_change_with_destination(gs, &modified, "front")
                        {
                            log::debug!("Failed to execute position change: {}", e);
                        }
                        self.clear_choice_state_and_resume(gs)?;
                        return Ok(());
                    } else if tgt.contains(':') {
                        let parts: Vec<&str> = tgt.splitn(2, ':').collect();
                        modified.target = Some(parts[0].to_string());
                        if parts[1] == "select" {
                            modified.source_position = Some(dest.to_string());
                        } else if super::util::stage_position_index(parts[1]).is_some() {
                            modified.source_position = Some(parts[1].to_string());
                        } else {
                            modified.target_member = Some(parts[1].to_string());
                        }
                    } else {
                        modified.target = Some(tgt.to_string());
                    }
                }
            }
            let was_select = choice_card_no.as_ref().is_some_and(|ccn| match ccn {
                ChoiceRoute::Raw(s) => s.contains(":select"),
                _ => false,
            });
            if was_select {
                // The user just chose WHICH member to move (the source position).
                // Now ask WHERE to move it (destination choice).
                // modified.source_position is already set to the chosen source area.
                // Call execute_position_change with target_member="this_member" so it
                // enters the source_position branch and creates a destination choice.
                let target_str = modified
                    .target
                    .clone()
                    .unwrap_or_else(|| "self".to_string());
                // Clear old choice metadata so the destination choice can set fresh routing.
                self.clear_choice_meta(gs);
                modified.target_member = Some("this_member".to_string());
                // Source was already filtered by group_names during source selection;
                // clear it so destinations aren't restricted to the same group.
                // Also clear exclude_self since entry_effect() returns the choice-level
                // effect which may have exclude_self set — that constraint only applies
                // to member targeting, not destination selection.
                modified.group_names = None;
                modified.exclude_self = None;
                if let Err(e) =
                    self.execute_position_change(gs, &modified, None, &target_str, "this_member")
                {
                    log::debug!(
                        "Failed to create destination choice for position change: {}",
                        e
                    );
                }
                // If a pending_choice was created, store it and return — the destination
                // selection will be handled on the next interaction.
                if self.pending_choice.is_some() {
                    self.store_pending_choice(gs);
                    return Ok(());
                }
                // No valid destinations (empty stage, etc.) — resume normally.
                self.clear_choice_state_and_resume(gs)?;
                return Ok(());
            }
            modified.destination = Some(dest.to_string());
            if let Err(e) = self.execute_position_change_with_destination(gs, &modified, dest) {
                log::debug!("Failed to execute position change: {}", e);
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
        // Check if we have a saved MoveCardsPosition context (card was already taken from source zone).
        // If so, place the card directly instead of re-running the entire effect which would fail
        // because the card is no longer in the source zone.
        let ctx = std::mem::replace(&mut self.execution_context, ExecutionContext::None);
        match ctx {
            ExecutionContext::MoveCardsPosition {
                card_id,
                target,
                source_zone,
                ..
            } => {
                // Consume use_limit for optional effects BEFORE the mutable
                // player borrow, to avoid borrow conflicts with gs.
                let mut use_limit_key: Option<String> = None;
                if selected != "skip" {
                    if let Some(entry) = gs.ability_queue.current_entry() {
                        let is_optional = entry
                            .ability
                            .effect
                            .as_ref()
                            .is_some_and(|e| e.optional.unwrap_or(false));
                        if is_optional && entry.ability.use_limit.is_some() {
                            if let Some(cid) = entry.card_id.or(gs.activating_card) {
                                use_limit_key = Some(format!(
                                    "{}_{}_{}",
                                    cid, entry.ability_index, gs.turn_number
                                ));
                            }
                        }
                    }
                }
                let player = gs.resolve_target_player_mut(&target);
                let destination = selected;
                // Player chose to skip — card stays in waitroom, no-op.
                if destination == "skip" {
                    if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        eprintln!("[DECK_DIAG] skip — card stays in waitroom");
                    }
                    self.clear_choice_state(gs);
                    return self.resume_pending_commands(gs);
                }
                if ABILITY_DEBUG.load(Ordering::Relaxed) {
                    eprintln!("[DECK_DIAG] handle_position_destination ctx=MoveCardsPosition card_id={} dest={} src={}", card_id, destination, source_zone);
                }
                // Remove card from source zone first, then place in destination.
                // The card was left in place when the deck_top_or_bottom choice was created.
                if source_zone == Zone::Discard.to_str()
                    || source_zone == Zone::Waitroom.to_str()
                    || source_zone == "those_cards"
                {
                    if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        eprintln!(
                            "[DECK_DIAG] waitroom before retain={:?} removing card_id={}",
                            player.waitroom.cards, card_id
                        );
                    }
                    let before = player.waitroom.cards.len();
                    player.waitroom.cards.retain(|c| *c != card_id);
                    if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        eprintln!(
                            "[DECK_DIAG] waitroom after retain={:?} ({} -> {})",
                            player.waitroom.cards,
                            before,
                            player.waitroom.cards.len()
                        );
                    }
                }
                crate::ability::util::place_card_in_zone(
                    player,
                    card_id,
                    &destination,
                    None,
                    false,
                    1,
                );
                if ABILITY_DEBUG.load(Ordering::Relaxed) {
                    eprintln!(
                        "[DECK_DIAG] deck len={} first={:?} last={:?}",
                        player.main_deck.cards.len(),
                        player.main_deck.cards.first(),
                        player.main_deck.cards.last()
                    );
                }
                // The player chose to place the card — insert use_limit key
                // after the player borrow is done (avoid conflicts with gs).
                if let Some(key) = use_limit_key {
                    gs.turn_limited_abilities_used.insert(key);
                    if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        eprintln!("[DECK_DIAG] recorded use_limit for optional effect");
                    }
                }
                self.clear_choice_state(gs);
                self.resume_pending_commands(gs)
            }
            _ => {
                // Fall back to effect modification for non-card-specific position choices
                // (e.g. stage position selection).
                self.apply_effect_modification(gs, |effect| {
                    effect.destination = Some(selected.to_string());
                })
            }
        }
    }

    fn handle_double_baton_touch(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        if selected == "skip" {
            self.clear_choice_state(gs);
            return Ok(());
        }
        // selected is "left,center" or "left,right" or "center,right"
        let areas: Vec<&str> = selected.split(',').collect();
        if areas.len() != 2 {
            self.clear_choice_state(gs);
            return Err(format!("Invalid double baton selection: {}", selected));
        }
        let area_enums = [
            crate::zones::MemberArea::LeftSide,
            crate::zones::MemberArea::Center,
            crate::zones::MemberArea::RightSide,
        ];
        let area_from_name = |name: &str| -> usize {
            match name {
                "left" => 0,
                "center" => 1,
                "right" => 2,
                _ => 999,
            }
        };
        let idx1 = area_from_name(areas[0]);
        let idx2 = area_from_name(areas[1]);
        if idx1 > 2 || idx2 > 2 {
            self.clear_choice_state(gs);
            return Err(format!(
                "Invalid area in double baton selection: {}",
                selected
            ));
        }
        // Replace both members (move to waitroom)
        let p1 = &mut gs.player1;
        if idx1 < 3 && p1.stage.stage[idx1] != -1 {
            p1.waitroom.add_card(p1.stage.stage[idx1]);
            p1.stage.stage[idx1] = -1;
            p1.areas_locked_this_turn.insert(area_enums[idx1]);
        }
        if idx2 < 3 && p1.stage.stage[idx2] != -1 {
            p1.waitroom.add_card(p1.stage.stage[idx2]);
            p1.stage.stage[idx2] = -1;
            p1.areas_locked_this_turn.insert(area_enums[idx2]);
        }
        gs.record_baton_touch();
        gs.record_baton_touch();
        // Place the activating card (Sumire) in the player's chosen placement area.
        // The play_baton_touch constant ability fires after PlayMemberToStage has already
        // placed the card, so at this point the card is already on stage. If this is triggered
        // as a standalone ability (non-PlayMemberToStage path), we need to move the card.
        // For now, the baton touch replacements and lock are recorded so debut abilities
        // see baton_touch_count > 0 and can trigger correctly.
        self.clear_choice_state(gs);
        self.resume_pending_commands(gs)
    }

    fn handle_conditional_optional(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        // Read before clear_choice_state since it nullifies conditional_choice.
        let entry_eff = gs.entry_effect().cloned();
        let cond_choice = gs.entry_conditional_choice();
        let chose_yes = selected == "1" || selected == "yes";
        self.clear_choice_state(gs);

        // Set choice_card_no and optional_cost_result AFTER clear_choice_state
        // which would otherwise erase them.
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.choice_card_no = Some(crate::ability::types::ChoiceRoute::OptionalCost);
            entry.optional_cost_result = Some(chose_yes);
        }

        // Prefer cond_choice (the sub-effect with optional_action/conditional_action)
        // over entry_eff (which may return a parent sequential effect lacking these fields).
        let effect = cond_choice
            .and_then(|json| serde_json::from_str::<AbilityEffect>(&json).ok())
            .or_else(|| entry_eff);
        if let Some(effect) = effect {
            let is_negation = effect.compound.conditional_negation.unwrap_or(false);
            let chose_yes = selected == "1" || selected == "yes";
            // Record use_limit when the player chose to pay (but NOT when declined)
            if chose_yes {
                if let Some(entry) = gs.ability_queue.current_entry() {
                    if let Some(cid) = entry.card_id {
                        let turn = gs.turn_number;
                        for (idx, ab) in gs
                            .card_database
                            .get_card(cid)
                            .map(|c| &c.abilities)
                            .into_iter()
                            .flatten()
                            .enumerate()
                        {
                            if ab.use_limit.is_some() {
                                let key = format!("{}_{}_{}", cid, idx, turn);
                                if !gs.turn_limited_abilities_used.contains(&key) {
                                    gs.turn_limited_abilities_used.insert(key);
                                }
                                break;
                            }
                        }
                    }
                }
            }
            let cmd = match (chose_yes, is_negation) {
                // yes + negation → optional_action fires, conditional skipped
                (true, true) => effect.compound.optional_action.map(|a| Command::Effect(*a)),
                // yes + no negation → conditional_action fires (the follow-up)
                (true, false) => effect
                    .compound
                    .conditional_action
                    .map(|a| Command::Effect(*a)),
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
            let color = HEART_VALS[idx];
            gs.prohibition_effects
                .push(format!("selected_heart_color:{}", color));
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.conditional_choice = Some(color.to_string());
            }
        }
        self.clear_choice_state(gs);
        self.resume_pending_commands(gs)
    }

    fn handle_choice_condition(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        let idx: usize = selected.parse().unwrap_or(0);
        if let Some(options) = gs.entry_cost().and_then(|c| c.compound.actions.clone()) {
            if idx < options.len() {
                let label = options[idx]
                    .text
                    .split("}}")
                    .last()
                    .unwrap_or(&options[idx].text)
                    .trim()
                    .to_string();
                let pp = gs.player_prefix();
                let act_name = gs
                    .activating_card
                    .and_then(|id| gs.card_database.get_card(id))
                    .map(|c| c.name.clone());
                gs.log_entry(
                    format!(
                        "{pp} {}: [choice] {} ✓",
                        act_name.as_deref().unwrap_or(""),
                        label
                    ),
                    &pp,
                    gs.activating_card,
                    act_name,
                    "choice",
                );
                // Take the pending SelectTarget so we can detect if pay_cost
                // creates a new sub-choice (e.g., SelectCard for discard from hand).
                let old_choice = self.pending_choice.take();
                self.pay_cost(gs, &options[idx])?;
                if self.pending_choice.is_some() {
                    // pay_cost created a sub-choice — signal to preserve it
                    self.sub_choice_created = true;
                } else {
                    // pay_cost resolved immediately — restore original choice
                    // so clear_choice_state can clean it up properly.
                    self.pending_choice = old_choice;
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

    /// Recursively set target on all sub-effects that don't have an explicit target.
    /// Excludes draw/draw_card actions (always target self) and select_cards (handled
    /// via spawn_context.target fallback in handle_select_card_internal).
    fn set_chosen_target(effect: &mut AbilityEffect, target: &str) {
        let skip_actions = ["draw", "draw_card", "select_cards"];
        if skip_actions.contains(&effect.action.as_str()) {
            return;
        }
        if effect.target.is_none() || effect.target.as_deref() == Some("self") {
            effect.target = Some(target.to_string());
        }
        if let Some(ref mut la) = effect.compound.look_action {
            Self::set_chosen_target(la, target);
        }
        if let Some(ref mut sa) = effect.compound.select_action {
            Self::set_chosen_target(sa, target);
        }
        if let Some(ref mut actions) = effect.compound.actions {
            for a in actions.iter_mut() {
                Self::set_chosen_target(a, target);
            }
        }
        if let Some(ref mut steps) = effect.effect_steps {
            for s in steps.iter_mut() {
                Self::set_chosen_target(s, target);
            }
        }
        if let Some(ref mut oa) = effect.opponent_action {
            Self::set_chosen_target(oa, target);
        }
        if let Some(ref mut pri) = effect.compound.primary_effect {
            Self::set_chosen_target(pri, target);
        }
        if let Some(ref mut oa2) = effect.compound.optional_action {
            Self::set_chosen_target(oa2, target);
        }
        if let Some(ref mut ca) = effect.compound.conditional_action {
            Self::set_chosen_target(ca, target);
        }
        if let Some(ref mut fu) = effect.compound.followup_action {
            Self::set_chosen_target(fu, target);
        }
    }
}
