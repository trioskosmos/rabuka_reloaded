use crate::ability::enums::{ConditionType, Zone};
use crate::ability::types::ChoiceRoute;
use crate::card::CardDatabase;
use crate::game_state::GameState;
use crate::game_state::Phase;

impl super::TurnEngine {
    pub fn execute_main_phase_action(
        game_state: &mut GameState,
        action: &crate::game_setup::ActionType,
        card_id: Option<i16>,
        card_indices: Option<Vec<usize>>,
        stage_area: Option<crate::zones::MemberArea>,
        use_baton_touch: Option<bool>,
    ) -> Result<(), String> {
        if game_state.has_pending_choice() {
            return Self::resume_with_choice(game_state, card_id, card_indices);
        }

        match action {
            crate::game_setup::ActionType::Pass => match game_state.current_phase {
                Phase::LiveCardSetFirstAttacker => {
                    let player = game_state.active_player_mut();
                    let cards_placed = player.live_card_zone.cards.len();
                    for _ in 0..cards_placed {
                        let _ = player.draw_card();
                    }
                    game_state.current_phase = Phase::LiveCardSetSecondAttacker;
                    Ok(())
                }
                Phase::LiveCardSetSecondAttacker => {
                    let player = game_state.active_player_mut();
                    let cards_placed = player.live_card_zone.cards.len();
                    for _ in 0..cards_placed {
                        let _ = player.draw_card();
                    }
                    Self::advance_phase(game_state);
                    Ok(())
                }
                _ => {
                    Self::advance_phase(game_state);
                    Ok(())
                }
            },
            crate::game_setup::ActionType::MulliganHeader => Ok(()),
            crate::game_setup::ActionType::RockChoice
            | crate::game_setup::ActionType::PaperChoice
            | crate::game_setup::ActionType::ScissorsChoice => {
                let choice_value = match action {
                    crate::game_setup::ActionType::RockChoice => 0,
                    crate::game_setup::ActionType::PaperChoice => 1,
                    crate::game_setup::ActionType::ScissorsChoice => 2,
                    _ => unreachable!(),
                };
                if game_state.player1_rps_choice.is_none() {
                    Self::handle_rps_choice_p1(game_state, choice_value)
                } else {
                    Self::handle_rps_choice_p2(game_state, choice_value)
                }
            }
            crate::game_setup::ActionType::ChooseFirstAttacker => {
                game_state.player1.is_first_attacker = true;
                game_state.player2.is_first_attacker = false;
                for _ in 0..6 {
                    game_state.player1.draw_card();
                    game_state.player2.draw_card();
                }
                game_state.current_phase = Phase::MulliganFirstAttacker;
                game_state.mulligan_selected_indices.clear();
                Ok(())
            }
            crate::game_setup::ActionType::ChooseSecondAttacker => {
                game_state.player1.is_first_attacker = false;
                game_state.player2.is_first_attacker = true;
                for _ in 0..6 {
                    game_state.player1.draw_card();
                    game_state.player2.draw_card();
                }
                game_state.current_phase = Phase::MulliganFirstAttacker;
                game_state.mulligan_selected_indices.clear();
                Ok(())
            }
            crate::game_setup::ActionType::SelectMulligan => {
                Self::handle_mulligan_selection(game_state, card_id, card_indices)
            }
            crate::game_setup::ActionType::ConfirmMulligan => {
                Self::handle_mulligan_confirmation(game_state)
            }
            crate::game_setup::ActionType::SkipMulligan => Self::handle_mulligan_skip(game_state),
            crate::game_setup::ActionType::PlayMemberToStage => Self::handle_play_member_to_stage(
                game_state,
                card_id,
                card_indices.clone(),
                stage_area,
                use_baton_touch,
            ),
            crate::game_setup::ActionType::SetLiveCard => {
                Self::handle_set_live_card(game_state, card_id)
            }
            crate::game_setup::ActionType::FinishLiveCardSet => {
                Err("FinishLiveCardSet action is obsolete - use Pass instead".into())
            }
            crate::game_setup::ActionType::UseAbility => {
                Self::handle_use_ability(game_state, card_id)
            }
            _ => Ok(()),
        }
    }

    fn handle_use_ability(game_state: &mut GameState, card_id: Option<i16>) -> Result<(), String> {
        let card_id = card_id.ok_or("No card specified for ability activation")?;
        if game_state.is_action_prohibited("cannot_activate")
            || game_state.is_action_prohibited("cannot_activate_by_effect")
        {
            return Err("Ability activation is prohibited by a restriction effect".to_string());
        }
        let card_db = game_state.card_database.clone();
        let card = card_db
            .get_card(card_id)
            .ok_or("Card not found in database")?;
        if !card.is_member() {
            return Err("Only member cards can activate abilities".to_string());
        }
        let player = game_state.active_player();
        let player_id = player.id.clone();
        let log_prefix = if player_id == "p1" || player_id == "player1" {
            "P1"
        } else {
            "P2"
        };

        // Find the first ability that can be activated from the current location
        let mut ability_to_activate = None;
        for (idx, ability) in card.abilities.iter().enumerate() {
            if ability
                .triggers
                .as_ref()
                .is_some_and(|t| t == crate::triggers::ACTIVATION)
            {
                let loc = ability
                    .effect
                    .as_ref()
                    .and_then(|e| e.activation_condition_parsed.as_ref())
                    .and_then(|c| {
                        if c.condition_type == Some(ConditionType::LocationCondition)
                            || matches!(c.location.as_deref(), Some("hand") | Some("discard"))
                        {
                            Zone::from_str(c.location.as_deref().unwrap_or(""))
                        } else {
                            None
                        }
                    })
                    .unwrap_or(Zone::Stage);

                let can_activate = match loc {
                    Zone::Hand => player.hand.cards.contains(&card_id),
                    Zone::Discard => player.waitroom.cards.contains(&card_id),
                    Zone::Stage => {
                        let stage_position =
                            player.stage.stage.iter().position(|&id| id == card_id);
                        if let Some(pos) = stage_position {
                            let stage_area = match pos {
                                0 => crate::zones::MemberArea::LeftSide,
                                1 => crate::zones::MemberArea::Center,
                                _ => crate::zones::MemberArea::RightSide,
                            };
                            crate::zones::check_trigger_position(
                                ability.triggers.as_deref(),
                                stage_area,
                            ) && crate::zones::check_effect_position(
                                ability
                                    .effect
                                    .as_ref()
                                    .and_then(|e| e.activation_position.as_deref()),
                                stage_area,
                            )
                        } else {
                            false
                        }
                    }
                    _ => false,
                };

                if can_activate {
                    // Check use limit
                    if let Some(_use_limit) = ability.use_limit {
                        let key = format!("{}_{}_{}", card_id, idx, game_state.turn_number);
                        if game_state.turn_limited_abilities_used.contains(&key) {
                            continue;
                        }
                    }
                    ability_to_activate = Some((idx, ability, loc));
                    break;
                }
            }
        }

        let (_ability_idx, ability, loc) = ability_to_activate
            .ok_or("No activatable ability found for this card at its current location")?;

        if loc == Zone::Hand {
            let player = game_state.active_player_mut();
            player.hand.cards.retain(|id| *id != card_id);
            player.waitroom.add_card(card_id);
        }

        let ability_id = format!("{}_{}", card.card_no, ability.full_text);
        game_state.trigger_auto_ability(
            ability_id,
            crate::game_state::AbilityTrigger::Activation,
            player_id.clone(),
            Some(card.card_no.clone()),
            Some(card_id),
        );
        game_state.process_pending_auto_abilities(&player_id);
        game_state.rule_log.push(format!(
            "{} [Activated] {}: {}",
            log_prefix, card.name, ability.full_text
        ));
        Ok(())
    }

    pub fn resume_with_choice(
        game_state: &mut GameState,
        card_id: Option<i16>,
        card_indices: Option<Vec<usize>>,
    ) -> Result<(), String> {
        let choice = game_state
            .ability_queue
            .is_waiting_for_choice()
            .cloned()
            .ok_or("No pending choice to resume")?;

        let ci = card_indices.clone();
        // Handle non-ability choices early (live success, etc.)
        if matches!(
            choice,
            crate::ability::types::Choice::SelectLiveSuccess { .. }
        ) {
            let result = Self::build_choice_result(&choice, card_id, ci.clone(), None)?;
            if let crate::ability::types::ChoiceResult::LiveSuccessSelected { card_index } = &result
            {
                let player_id = match &choice {
                    crate::ability::types::Choice::SelectLiveSuccess { player_id, .. } => {
                        player_id.clone()
                    }
                    _ => return Err("Wrong choice type".to_string()),
                };
                super::TurnEngine::handle_live_success_choice(game_state, *card_index, &player_id)?;
                game_state.ability_queue.complete_current();
                return Ok(());
            }
        }

        // Handle success zone replacement choices (e.g. 錯覚CROSSROADS)
        if let Some(replaced_card_id) = game_state.pending_success_replacement_card_id.take() {
            let player_id = game_state
                .pending_success_replacement_player_id
                .take()
                .unwrap_or_else(|| "player1".to_string());
            let result = Self::build_choice_result(&choice, card_id, ci, None)?;
            let player = if player_id == game_state.player1.id {
                &mut game_state.player1
            } else {
                &mut game_state.player2
            };
            match result {
                crate::ability::types::ChoiceResult::CardSelected { indices }
                    if !indices.is_empty() =>
                {
                    // Player chose a card from discard — move it to success zone,
                    // and put the original card in waitroom.
                    if let Some(&selected_idx) = indices.first() {
                        if selected_idx < player.waitroom.cards.len() {
                            let selected_card_id = player.waitroom.cards.remove(selected_idx);
                            // Remove the original card from live_card_zone if present
                            player
                                .live_card_zone
                                .cards
                                .retain(|cid| *cid != replaced_card_id);
                            player.waitroom.add_card(replaced_card_id);
                            // Move selected card to success zone
                            player.success_live_card_zone.cards.push(selected_card_id);
                        }
                    }
                    // Move any remaining live cards to waitroom
                    while !player.live_card_zone.cards.is_empty() {
                        player
                            .waitroom
                            .add_card(player.live_card_zone.cards.remove(0));
                    }
                }
                _ => {
                    // Player declined replacement (Skip or empty indices) —
                    // place original card in success zone normally
                    player
                        .live_card_zone
                        .cards
                        .retain(|cid| *cid != replaced_card_id);
                    player.success_live_card_zone.cards.push(replaced_card_id);
                    while !player.live_card_zone.cards.is_empty() {
                        player
                            .waitroom
                            .add_card(player.live_card_zone.cards.remove(0));
                    }
                }
            }
            game_state.ability_queue.complete_current();
            return Ok(());
        }

        let choice_card_no = game_state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_card_no.clone());
        let result =
            Self::build_choice_result(&choice, card_id, card_indices, choice_card_no.as_ref())?;
        Self::resume_queue_with_choice(game_state, choice, result)
    }

    fn build_choice_result(
        choice: &crate::ability::types::Choice,
        card_id: Option<i16>,
        card_indices: Option<Vec<usize>>,
        _choice_card_no: Option<&ChoiceRoute>,
    ) -> Result<crate::ability::types::ChoiceResult, String> {
        match choice {
            crate::ability::types::Choice::SelectCard { .. } => {
                let indices = card_indices.unwrap_or_else(|| {
                    // Allow select_option(0) to produce indices [0] for SelectCard choices
                    // (instead of requiring card_indices to be explicitly passed)
                    card_id.map(|id| vec![id as usize]).unwrap_or_default()
                });
                Ok(crate::ability::types::ChoiceResult::CardSelected { indices })
            }
            crate::ability::types::Choice::SelectTarget {
                target, options, ..
            } => {
                let selected = match target.as_str() {
                    "pay_optional_cost:skip_optional_cost" => {
                        if card_id == Some(1) {
                            "pay_optional_cost".to_string()
                        } else {
                            "skip_optional_cost".to_string()
                        }
                    }
                    "primary|alternative" => {
                        if card_id == Some(1) {
                            "alternative".to_string()
                        } else {
                            "primary".to_string()
                        }
                    }
                    "choice" | "choice_string" | "conditional_optional" => card_id
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "0".into()),
                    _ => {
                        // For position_change:opponent choices, use card_id as option index
                        // to look up the actual option string instead of the raw index.
                        if _choice_card_no
                            == Some(&ChoiceRoute::Raw(
                                "position_change:opponent:front".to_string(),
                            ))
                        {
                            if let Some(ref opts) = options {
                                if let Some(id) = card_id {
                                    if id >= 0 && (id as usize) < opts.len() {
                                        return Ok(
                                            crate::ability::types::ChoiceResult::TargetSelected {
                                                target: opts[id as usize].clone(),
                                            },
                                        );
                                    }
                                }
                            }
                        }
                        match card_id {
                            Some(-1) => "skip".to_string(),
                            Some(id) => id.to_string(),
                            None => "0".into(),
                        }
                    }
                };
                Ok(crate::ability::types::ChoiceResult::TargetSelected { target: selected })
            }
            crate::ability::types::Choice::SelectPosition { .. } => {
                let pos = card_id
                    .map(|id| match id {
                        0 => "left".into(),
                        1 => "center".into(),
                        2 => "right".into(),
                        _ => "center".into(),
                    })
                    .unwrap_or_else(|| "center".into());
                Ok(crate::ability::types::ChoiceResult::PositionSelected { position: pos })
            }
            crate::ability::types::Choice::SelectHeartColor {
                count: _,
                options,
                description: _,
            } => {
                let idx = card_id.unwrap_or(0) as usize;
                let chosen = if idx < options.len() {
                    options[idx].clone()
                } else {
                    "heart00".to_string()
                };
                Ok(crate::ability::types::ChoiceResult::HeartColorSelected {
                    colors: vec![chosen],
                })
            }
            crate::ability::types::Choice::SelectHeartType {
                count: _,
                options,
                description: _,
            } => {
                let idx = card_id.unwrap_or(0) as usize;
                let chosen = if idx < options.len() {
                    options[idx].clone()
                } else {
                    "heart00".to_string()
                };
                Ok(crate::ability::types::ChoiceResult::HeartTypeSelected {
                    types: vec![chosen],
                })
            }
            crate::ability::types::Choice::SelectLiveSuccess { options, .. } => {
                let idx = card_indices
                    .as_ref()
                    .and_then(|v| v.first().copied())
                    .or_else(|| card_id.map(|id| id as usize))
                    .unwrap_or(0);
                let card_index = if idx < options.len() {
                    options[idx].card_index
                } else {
                    0
                };
                Ok(crate::ability::types::ChoiceResult::LiveSuccessSelected { card_index })
            }
            crate::ability::types::Choice::SelectAutoAbility { options, .. } => {
                let idx = card_id.unwrap_or(0) as usize;
                let queue_idx = if idx < options.len() {
                    options[idx].queue_index
                } else {
                    0
                };
                Ok(crate::ability::types::ChoiceResult::AutoAbilitySelected {
                    queue_index: queue_idx,
                })
            }
        }
    }

    fn resume_queue_with_choice(
        game_state: &mut GameState,
        choice: crate::ability::types::Choice,
        result: crate::ability::types::ChoiceResult,
    ) -> Result<(), String> {
        if let crate::ability_queue::QueueState::WaitingForAutoAbilityChoice { .. } = game_state.ability_queue.get_state() {
            if let crate::ability::types::ChoiceResult::AutoAbilitySelected { queue_index } =
                result
            {
                let player_id = if let crate::ability::types::Choice::SelectAutoAbility {
                    ref player_id,
                    ..
                } = choice
                {
                    player_id.clone()
                } else {
                    String::new()
                };
                game_state.ability_queue.resume_with_choice(result.clone());
                game_state.ability_queue.promote_entry_by_abs(queue_index);
                if game_state.ability_queue.start_next() {
                    game_state.recently_moved_cards = None;
                    game_state.recently_moved_from_zone = None;
                    game_state.process_current_ability();
                }
                if !game_state.has_pending_choice() && !player_id.is_empty() {
                    game_state.process_pending_auto_abilities(&player_id);
                }
                return Ok(());
            } else {
                return Err("Expected AutoAbilitySelected result".to_string());
            }
        }

        game_state.ability_queue.resume_with_choice(result.clone());
        let had_pending_sequential = game_state.ability_queue.has_pending_commands();

        // Take the persistent resolver from the queue entry
        let mut resolver = match game_state.ability_queue.take_resolver() {
            Some(r) => {
                log::debug!(
                    "[RWC] took resolver: moved_cards={:?} selected={:?}",
                    r.moved_cards, r.selected_cards
                );
                r
            }
            None => {
                return Err("No resolver found on queue entry".to_string());
            }
        };
        resolver.pending_choice = Some(choice);
        let res = resolver.provide_choice_result(game_state, result);

        if let Err(e) = res {
            log::debug!("[RWC_ERROR] provide_choice_result failed: {}", e);
            game_state.ability_queue.complete_current();
            return Err(e);
        }

        log::debug!(
            "[RWC] after provide: pending_choice={:?} moved_cards={:?} selected={:?}",
            resolver.pending_choice.is_some(),
            resolver.moved_cards,
            resolver.selected_cards
        );

        if resolver.pending_choice.is_some() {
            // Sub-choice created — store resolver back on entry and pause queue
            let sub_choice = resolver.pending_choice.clone().unwrap();
            resolver.store_pending_choice(game_state);
            game_state.ability_queue.set_resolver(resolver);
            game_state.ability_queue.pause_for_choice(sub_choice);
        } else {
            // No more choices — ability execution finished
            let cost_was_paid = game_state
                .ability_queue
                .current_entry()
                .is_some_and(|e| e.cost_paid);
            let effect_started = game_state
                .ability_queue
                .current_entry()
                .is_some_and(|e| e.effect_started);
            log::debug!(
                "[RWC] cost_was_paid={}, effect_started={}, had_pending_sequential={}",
                cost_was_paid, effect_started, had_pending_sequential
            );
            game_state.activating_card = None;

            let optional_skipped = game_state.ability_queue.current_entry().is_some_and(|e| {
                e.cost_paid
                    && !e.optional_cost_was_paid
                    && e.choice_card_no == Some(ChoiceRoute::OptionalCost)
            });
            // Re-check pending commands — they may have been cleared by the choice
            // handler (e.g. optional draw skip), leaving the sequential stranded.
            // Only fire when effect hasn't started yet (the skip is between optional
            // draw choice and the draw action itself). Normal sequential mid-execution
            // has effect_started=true and must NOT be cancelled.
            let pending_cleared = cost_was_paid
                && !effect_started
                && had_pending_sequential
                && !game_state.ability_queue.has_pending_commands();
            let effect_ready = cost_was_paid && !had_pending_sequential && !effect_started;

            if optional_skipped || pending_cleared {
                log::debug!(
                    "[RWC] optional_skipped={} pending_cleared={} completing ability",
                    optional_skipped, pending_cleared
                );
                game_state.ability_queue.complete_current();
                game_state.clear_effect_tracking();
                let player_id = game_state.active_player().id.clone();
                game_state.process_pending_auto_abilities(&player_id);
            } else if effect_ready {
                log::debug!("RWC: calling process_current_ability");
                // Store resolver back on entry so process_current_ability can reuse it
                // (the resolver carries cost-phase state like revealed_cost_cards).
                game_state.ability_queue.set_resolver(resolver);
                game_state.process_current_ability();
                if game_state.has_pending_choice() {
                    let player_id = game_state
                        .ability_queue
                        .current_entry()
                        .map(|e| e.player_id.clone())
                        .unwrap_or_else(|| "p1".to_string());
                    game_state.process_pending_auto_abilities(&player_id);
                }
            } else if cost_was_paid {
                // Record use_limit when ability completes (cost+effect both resolved)
                if let Some(entry) = game_state.ability_queue.current_entry() {
                    if let Some(cid) = entry.card_id {
                        let turn = game_state.turn_number;
                        for (idx, ab) in game_state
                            .card_database
                            .get_card(cid)
                            .map(|c| &c.abilities)
                            .into_iter()
                            .flatten()
                            .enumerate()
                        {
                            if ab.triggers.as_deref() == Some("起動") && ab.use_limit.is_some() {
                                let key = format!("{}_{}_{}", cid, idx, turn);
                                if !game_state.turn_limited_abilities_used.contains(&key) {
                                    game_state.turn_limited_abilities_used.insert(key);
                                }
                                break;
                            }
                        }
                    }
                }
                game_state.ability_queue.complete_current();
                game_state.clear_effect_tracking();
                let player_id = game_state.active_player().id.clone();
                game_state.process_pending_auto_abilities(&player_id);
            } else {
                game_state.ability_queue.complete_current();
                game_state.clear_effect_tracking();
                let player_id = game_state.active_player().id.clone();
                game_state.process_pending_auto_abilities(&player_id);
            }
        }
        Ok(())
    }

    pub fn check_timing(game_state: &mut GameState) {
        game_state.player1.refresh();
        game_state.player2.refresh();
        let p1_needs_refresh = game_state.player1.main_deck.cards.is_empty()
            && !game_state.player1.waitroom.cards.is_empty();
        let p2_needs_refresh = game_state.player2.main_deck.cards.is_empty()
            && !game_state.player2.waitroom.cards.is_empty();
        if p1_needs_refresh {
            let mut waitroom = std::mem::take(&mut game_state.player1.waitroom.cards);
            game_state.player1.main_deck.cards.append(&mut waitroom);
            game_state.player1.main_deck.shuffle();
        }
        if p2_needs_refresh {
            let mut waitroom = std::mem::take(&mut game_state.player2.waitroom.cards);
            game_state.player2.main_deck.cards.append(&mut waitroom);
            game_state.player2.main_deck.shuffle();
        }
        // Rule 10.4: Duplicate member processing
        Self::check_duplicate_members(&mut game_state.player1, &game_state.card_database);
        Self::check_duplicate_members(&mut game_state.player2, &game_state.card_database);
        Self::check_victory_condition(game_state);
        // Rule 10.5: Invalid card processing
        Self::check_invalid_live_cards(&mut game_state.player1, &game_state.card_database);
        Self::check_invalid_live_cards(&mut game_state.player2, &game_state.card_database);
        Self::check_invalid_energy_cards(&mut game_state.player1, &game_state.card_database);
        Self::check_invalid_energy_cards(&mut game_state.player2, &game_state.card_database);
        // Rule 10.5.3-4: Orphaned under-member cards
        Self::check_orphaned_under_cards(&mut game_state.player1, &game_state.card_database);
        Self::check_orphaned_under_cards(&mut game_state.player2, &game_state.card_database);
        Self::check_invalid_resolution_zone(game_state);
        if game_state.check_permanent_loop() {
            game_state.game_result = crate::game_state::GameResult::Draw;
            game_state.game_ended = true;
        }
        Self::check_victory_condition(game_state);
        let active_player_id = game_state.active_player().id.clone();
        game_state.process_pending_auto_abilities(&active_player_id);
    }

    pub fn check_victory_condition(game_state: &mut GameState) {
        let p1_success_count = game_state.player1.success_live_card_zone.cards.len();
        let p2_success_count = game_state.player2.success_live_card_zone.cards.len();

        // Rule 1.2.1.1: Player wins with 3+ cards when opponent has 2- cards
        // Rule 1.2.1.2: Draw if both players have 3+ cards simultaneously
        if p1_success_count >= crate::constants::VICTORY_CARD_COUNT
            && p2_success_count >= crate::constants::VICTORY_CARD_COUNT
        {
            // Both players have 3+ cards - draw
            game_state.game_result = crate::game_state::GameResult::Draw;
            game_state.game_ended = true;
        } else if p1_success_count >= crate::constants::VICTORY_CARD_COUNT && p2_success_count <= 2
        {
            // Player 1 has 3+ cards, player 2 has 2- cards - player 1 wins
            game_state.game_result = if game_state.player1.is_first_attacker {
                crate::game_state::GameResult::FirstAttackerWins
            } else {
                crate::game_state::GameResult::SecondAttackerWins
            };
            game_state.game_ended = true;
        } else if p2_success_count >= crate::constants::VICTORY_CARD_COUNT && p1_success_count <= 2
        {
            // Player 2 has 3+ cards, player 1 has 2- cards - player 2 wins
            game_state.game_result = if game_state.player2.is_first_attacker {
                crate::game_state::GameResult::FirstAttackerWins
            } else {
                crate::game_state::GameResult::SecondAttackerWins
            };
            game_state.game_ended = true;
        }
    }

    /// Rule 10.4: Duplicate member processing.
    /// If the stage area itself somehow has more than one top-level member,
    /// keep only the most recent one. This is a safety check — under-card members
    /// are intentional and not duplicates (they follow rule 4.5.5).
    fn check_duplicate_members(player: &mut crate::player::Player, _card_db: &CardDatabase) {
        // The stage array is already enforced to have 1 card per slot by the engine.
        // Under-cards (4.5.5) are intentional stacking and NOT duplicates.
        // This function is kept as a safety check but currently does nothing
        // beyond what the stage enforces.
        let _ = player;
    }

    /// Rule 10.5.1: Non-live cards in live card zone → moved to discard.
    /// Also Rule 10.5.2: Non-energy cards in energy zone → moved to discard.
    fn check_invalid_live_cards(player: &mut crate::player::Player, card_db: &CardDatabase) {
        let mut invalid_indices = Vec::new();
        for (i, card_id) in player.live_card_zone.cards.iter().enumerate() {
            if !card_db.get_card(*card_id).is_some_and(|c| c.is_live()) {
                invalid_indices.push(i);
            }
        }
        for &i in invalid_indices.iter().rev() {
            if i < player.live_card_zone.cards.len() {
                let card_id = player.live_card_zone.cards.remove(i);
                // Rule 10.5.5: Energy cards go to energy deck, not discard
                if card_db.get_card(card_id).is_some_and(|c| c.is_energy()) {
                    player.energy_deck.cards.push(card_id);
                } else {
                    player.waitroom.add_card(card_id);
                }
            }
        }
    }

    /// Rule 10.5.2: Non-energy cards in energy zone → moved to discard.
    fn check_invalid_energy_cards(player: &mut crate::player::Player, card_db: &CardDatabase) {
        let mut invalid_indices = Vec::new();
        for (i, card_id) in player.energy_zone.cards.iter().enumerate() {
            if !card_db.get_card(*card_id).is_some_and(|c| c.is_energy()) {
                invalid_indices.push(i);
            }
        }
        for &i in invalid_indices.iter().rev() {
            if i < player.energy_zone.cards.len() {
                let card_id = player.energy_zone.cards.remove(i);
                player.waitroom.add_card(card_id);
            }
        }
    }

    /// Rule 10.5.3-4: Orphaned cards under members.
    /// When a member leaves its area, any member cards under it go to discard (10.5.3)
    /// and any energy cards under it go to energy deck (10.5.4).
    fn check_orphaned_under_cards(player: &mut crate::player::Player, card_db: &CardDatabase) {
        for area_idx in 0..3 {
            let top = player.stage.stage[area_idx];
            if top == -1 {
                let under = std::mem::take(&mut player.stage.under_cards[area_idx]);
                for cid in under {
                    if card_db.get_card(cid).is_some_and(|c| c.is_energy()) {
                        player.energy_deck.cards.push(cid);
                    } else {
                        player.waitroom.cards.push(cid);
                    }
                }
            }
        }
    }

    fn check_invalid_resolution_zone(game_state: &mut GameState) {
        let cards = std::mem::take(&mut game_state.resolution_zone.cards);
        if cards.is_empty() {
            return;
        }
        let player = game_state.active_player_mut();
        for card_id in cards {
            player.waitroom.add_card(card_id);
        }
    }

    pub fn player_set_live_cards(
        player: &mut crate::player::Player,
        num_cards_to_set: usize,
        card_database: &crate::card::CardDatabase,
    ) {
        let mut cards_set = Vec::new();
        let mut held_back = Vec::new();
        while let Some(card_id) = player.hand.cards.pop() {
            if cards_set.len() < num_cards_to_set
                && card_database
                    .get_card(card_id)
                    .is_some_and(|c| c.is_live())
            {
                cards_set.push(card_id);
            } else {
                held_back.push(card_id);
            }
        }
        for card_id in cards_set {
            player.live_card_zone.cards.push(card_id);
        }
        player.hand.cards = held_back.into_iter().rev().collect();
    }
}
