use crate::ability::enums::{ConditionType, Zone};
use crate::ability::types::ChoiceRoute;
use crate::card::CardDatabase;
use crate::game_state::GameState;
use crate::game_state::Phase;
#[cfg(feature = "3ds")]
extern "C" {
    fn _3ds_tdbg(msg: *const u8);
}

#[cfg(feature = "3ds")]
macro_rules! tdbg {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let s = format!("{}\0", msg);
        unsafe { _3ds_tdbg(s.as_ptr()); }
    }};
}
#[cfg(not(feature = "3ds"))]
macro_rules! tdbg {
    ($($arg:tt)*) => {{ let _ = format!($($arg)*); }};
}

impl super::TurnEngine {
    pub fn execute_main_phase_action(
        game_state: &mut GameState,
        action: &crate::game_setup::ActionType,
        card_id: Option<i16>,
        card_indices: Option<Vec<usize>>,
        stage_area: Option<crate::zones::MemberArea>,
        use_baton_touch: Option<bool>,
    ) -> Result<(), String> {
        // UseAbility must check activation legality independently — never
        // route through resume_with_choice, even when another ability's choice
        // is pending (e.g. a debut look-and-select from play_to_stage).
        if matches!(action, crate::game_setup::ActionType::UseAbility) {
            if game_state.has_pending_choice() {
                return Err("Cannot activate ability while another choice is pending".to_string());
            }
            return Self::handle_use_ability(game_state, card_id);
        }

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
                // PVP: route by player_id from session
                if let Some(pid) = game_state.pending_rps_player_id {
                    let r = if pid == 0 {
                        Self::handle_rps_choice_p1(game_state, choice_value)
                    } else {
                        Self::handle_rps_choice_p2(game_state, choice_value)
                    };
                    game_state.pending_rps_player_id = None;
                    r
                } else {
                    // Sandbox / no player context: sequential (P1 then P2)
                    if game_state.player1_rps_choice.is_none() {
                        Self::handle_rps_choice_p1(game_state, choice_value)
                    } else {
                        Self::handle_rps_choice_p2(game_state, choice_value)
                    }
                }
            }
            crate::game_setup::ActionType::ChooseFirstAttacker => {
                if game_state.current_phase != Phase::ChooseFirstAttacker {
                    return Err(
                        "ChooseFirstAttacker is only valid during ChooseFirstAttacker phase"
                            .to_string(),
                    );
                }
                let p1_first = game_state.rps_winner != Some(2);
                game_state.player1.is_first_attacker = p1_first;
                game_state.player2.is_first_attacker = !p1_first;
                for _ in 0..6 {
                    game_state.player1.draw_card();
                    game_state.player2.draw_card();
                }
                game_state.current_phase = Phase::MulliganFirstAttacker;
                game_state.mulligan_selected_indices.clear();
                Ok(())
            }
            crate::game_setup::ActionType::ChooseSecondAttacker => {
                if game_state.current_phase != Phase::ChooseFirstAttacker {
                    return Err(
                        "ChooseSecondAttacker is only valid during ChooseFirstAttacker phase"
                            .to_string(),
                    );
                }
                let p1_first = game_state.rps_winner == Some(2);
                game_state.player1.is_first_attacker = p1_first;
                game_state.player2.is_first_attacker = !p1_first;
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
                Self::handle_mulligan_confirmation(game_state, card_indices.clone())
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
            crate::game_setup::ActionType::LiveCardHeader => Ok(()),
            crate::game_setup::ActionType::SelectLiveCard => {
                Self::handle_live_card_selection(game_state, card_id, card_indices)
            }
            crate::game_setup::ActionType::ConfirmLiveCardSet => {
                Self::handle_live_card_confirmation(game_state, card_indices.clone())
            }
            crate::game_setup::ActionType::SkipLiveCardSet => {
                Self::handle_live_card_skip(game_state)
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

        struct AbilityActivation {
            idx: usize,
            ability: crate::card::Ability,
            loc: Zone,
        }

        // Find the first ability that can be activated from the current location
        let mut ability_to_activate: Option<AbilityActivation> = None;
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

                log::debug!(
                    "[ACTIVATE_CHECK] ability idx={} triggers={:?} loc={:?} card_pos={:?}",
                    idx,
                    ability.triggers,
                    loc,
                    player.stage.stage.iter().position(|&id| id == card_id)
                );
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
                    if let Some(use_limit) = ability.use_limit {
                        let key = format!("{}_{}_{}", card_id, idx, game_state.turn_number);
                        let used = game_state
                            .turn_limited_abilities_used
                            .get(&key)
                            .copied()
                            .unwrap_or(0);
                        if u32::from(used) >= use_limit {
                            continue;
                        }
                    }
                    ability_to_activate = Some(AbilityActivation {
                        idx,
                        ability: ability.clone(),
                        loc,
                    });
                    break;
                }
            }
        }

        if ability_to_activate.is_none() {
            if let Some(gained) = game_state
                .gained_card_abilities
                .get(&card_id)
                .and_then(|list| {
                    list.iter()
                        .enumerate()
                        .find(|(_, a)| {
                            a.triggers
                                .as_ref()
                                .is_some_and(|t| t == crate::triggers::ACTIVATION)
                        })
                        .map(|(i, a)| (i, a.clone()))
                })
            {
                if player.stage.stage.iter().any(|&id| id == card_id) {
                    ability_to_activate = Some(AbilityActivation {
                        idx: 10000 + gained.0,
                        ability: gained.1,
                        loc: Zone::Stage,
                    });
                }
            }
        }

        let AbilityActivation { ability, loc, idx } = ability_to_activate
            .ok_or("No activatable ability found for this card at its current location")?;

        if loc == Zone::Hand {
            let player = game_state.active_player_mut();
            player.hand.cards.retain(|id| *id != card_id);
            player.waitroom.add_card(card_id);
        }

        // Gained abilities use the "card_no_gained_{idx}" format so
        // trigger_auto_ability's gained-ability code path (line ~683)
        // can find and enqueue them.
        let ability_id = if idx >= 10000 {
            debug_assert!(
                game_state.gained_card_abilities.contains_key(&card_id),
                "gained ability idx >= 10000 but no gained_card_abilities entry"
            );
            format!("{}_gained_{}", card.card_no, idx - 10000)
        } else {
            format!("{}_{}", card.card_no, ability.full_text)
        };
        game_state.trigger_auto_ability(
            ability_id,
            crate::game_state::AbilityTrigger::Activation,
            player_id.clone(),
            Some(card.card_no.clone()),
            Some(card_id),
            None,
            None,
        );
        game_state.process_pending_auto_abilities(&player_id);
        game_state.rule_log.push(format!(
            "{} [[log_activation]] {}: {}",
            log_prefix, card.name, ability.full_text
        ));
        Ok(())
    }

    pub fn resume_with_choice(
        game_state: &mut GameState,
        card_id: Option<i16>,
        card_indices: Option<Vec<usize>>,
    ) -> Result<(), String> {
        let pending = game_state.ability_queue.is_waiting_for_choice().cloned();
        let choice = pending.ok_or("No pending choice to resume")?;

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
                let indices = card_indices
                    .unwrap_or_else(|| card_id.map(|id| vec![id as usize]).unwrap_or_default());
                Ok(crate::ability::types::ChoiceResult::CardSelected { indices })
            }
            crate::ability::types::Choice::SelectTarget {
                target, options, ..
            } => {
                log::debug!(
                    "[BCR] SelectTarget target={} card_id={:?} card_indices={:?} options={:?}",
                    target,
                    card_id,
                    card_indices,
                    options
                );
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
                    "choice" | "choice_string" | "conditional_optional" => {
                        // card_id=None + card_indices absent/empty means skip
                        if card_id.is_none()
                            && card_indices.as_deref().map_or(true, |v| v.is_empty())
                        {
                            return Ok(crate::ability::types::ChoiceResult::Skip);
                        }
                        card_id
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "0".into())
                    }
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
                        // For position|destination choices, look up the option text in the
                        // options array by index.  The options array contains the actual
                        // position names (e.g. "left", "center", "right") that the handler
                        // expects, not raw numeric indices.
                        if target == "position|destination" || target == "area_select" {
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
                        // For self_or_opponent, look up option text by index from card_indices
                        if target == "self_or_opponent" {
                            if let Some(ref opts) = options {
                                if let Some(ref indices) = card_indices {
                                    if let Some(&idx) = indices.first() {
                                        if idx < opts.len() {
                                            return Ok(
                                                crate::ability::types::ChoiceResult::TargetSelected {
                                                    target: opts[idx].clone(),
                                                },
                                            );
                                        }
                                    }
                                }
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
                ..
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
                ..
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
        if let crate::ability_queue::QueueState::WaitingForAutoAbilityChoice { .. } =
            game_state.ability_queue.get_state()
        {
            if let crate::ability::types::ChoiceResult::AutoAbilitySelected { queue_index } = result
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
                // Set depth-first cutoff BEFORE resolution so entries queued by
                // process_current_ability (each_time watchers) are excluded from
                // the stale-entries pool when process_player_abilities re-enters.
                let cutoff = game_state.ability_queue.len();
                game_state.depth_first_cutoff = Some(cutoff);
                game_state.ability_queue.promote_entry_by_abs(queue_index);
                if game_state.ability_queue.start_next() {
                    let saved_moved = game_state.recently_moved_cards.take();
                    let saved_from_zone = game_state.recently_moved_from_zone.take();
                    game_state.recently_state_changed.clear();
                    game_state.process_current_ability();
                    if game_state.recently_moved_cards.is_none() {
                        game_state.recently_moved_cards = saved_moved;
                        game_state.recently_moved_from_zone = saved_from_zone;
                    }
                }
                // If the ability completed without pausing, drain newly-queued
                // entries (each_time watchers) immediately. If it paused (e.g.,
                // sequential effect sub-choice), the cutoff carries forward to
                // the next process_player_abilities entry.
                if !game_state.has_pending_choice() && game_state.ability_queue.is_idle() {
                    let pid = player_id.clone();
                    let mut drain_iters = 0;
                    while !game_state.has_pending_choice() && game_state.ability_queue.is_idle() {
                        drain_iters += 1;
                        if drain_iters > 50 {
                            break;
                        }
                        let new_idx = (cutoff..game_state.ability_queue.len()).find(|&i| {
                            game_state.ability_queue.is_entry_available(i)
                                && game_state.ability_queue.entry_player_id(i) == Some(&pid)
                        });
                        match new_idx {
                            Some(idx) => {
                                game_state.ability_queue.set_current_entry(idx);
                                if !game_state.ability_queue.start_next() {
                                    break;
                                }
                                game_state.process_current_ability();
                                if game_state.has_pending_choice() {
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
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
        let had_pending_sequential = game_state.ability_queue.has_pending_actions();

        // Take the persistent resolver from the queue entry
        let mut resolver = match game_state.ability_queue.take_resolver() {
            Some(mut r) => {
                let selected = r.selected_cards.clone();
                log::debug!(
                    "[RWC] took resolver: moved_cards={:?} selected={:?}",
                    r.moved_cards,
                    selected
                );
                r.sub_choice_created = false;
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

        // If inner processing (e.g. depth-first trigger scan) created a
        // pending choice on the game-state level (a different queue entry),
        // don't touch it — return immediately.
        if game_state.has_pending_choice() {
            log::debug!("[RWC] inner processing created pending choice — returning early");
            return Ok(());
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
            // G1/G3: route pending choice to opponent if it targets opponent
            let targets_opponent = match &sub_choice {
                crate::ability::types::Choice::SelectCard {
                    target_player_id: Some(tpid),
                    ..
                } if tpid == "opponent"
                    && resolver.spawn_context.target.as_deref() == Some("opponent") =>
                {
                    true
                }
                crate::ability::types::Choice::SelectPosition { .. }
                    if matches!(
                        resolver.execution_context,
                        crate::ability::types::ExecutionContext::MoveCardsPosition { ref target, .. }
                        if target == "opponent"
                    ) =>
                {
                    true
                }
                _ => false,
            };
            log::debug!(
                "[RWC_G1] tpid_opp={} spawn={:?} choice={:?}",
                targets_opponent,
                resolver.spawn_context.target,
                &sub_choice
            );
            if let Some(entry) = game_state.ability_queue.current_entry_mut() {
                if targets_opponent {
                    let current = entry.player_id.clone();
                    let opponent_id = if current == "p1" { "p2" } else { "p1" };
                    entry.choice_player_id = Some(opponent_id.to_string());
                    log::debug!("[RWC_G1] SET choice_player_id={}", opponent_id);
                } else if matches!(&sub_choice, crate::ability::types::Choice::SelectCard { target_player_id: Some(tpid), .. } if tpid == "self")
                {
                    entry.choice_player_id = Some(entry.player_id.clone());
                    log::debug!(
                        "[RWC_G1] RESET choice_player_id to activator={}",
                        entry.player_id
                    );
                }
            }
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
            // Capture key and player_id BEFORE complete_current() removes the entry.
            let just_completed_key = game_state
                .ability_queue
                .current_entry()
                .map(|e| format!("{}_{}", e.card_no, e.ability.full_text));
            let entry_player_id = game_state
                .ability_queue
                .current_entry()
                .map(|e| e.player_id.clone());
            // Capture each_time trigger info before entry is lost
            let cost_entry_trigger = game_state
                .ability_queue
                .current_entry()
                .map(|e| e.trigger_type.clone());
            let cost_entry_card_id = game_state
                .ability_queue
                .current_entry()
                .and_then(|e| e.card_id);
            let cost_entry_opt_result = game_state
                .ability_queue
                .current_entry()
                .and_then(|e| e.optional_cost_result);
            log::debug!(
                "[RWC] cost_was_paid={}, effect_started={}, had_pending_sequential={}",
                cost_was_paid,
                effect_started,
                had_pending_sequential
            );
            game_state.activating_card = None;
            game_state.activating_ability_index = None;

            let optional_skipped = game_state.ability_queue.current_entry().is_some_and(|e| {
                e.cost_paid
                    && e.optional_cost_result == Some(false)
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
                && !game_state.ability_queue.has_pending_actions();
            let effect_ready = cost_was_paid && !had_pending_sequential && !effect_started;
            log::debug!("[RWC_BRANCH] cost_was_paid={} effect_started={} had_pending={} optional_skipped={} pending_cleared={} effect_ready={}",
                cost_was_paid, effect_started, had_pending_sequential,
                game_state.ability_queue.current_entry().is_some_and(|e| {
                    e.cost_paid && e.optional_cost_result == Some(false)
                        && e.choice_card_no == Some(crate::ability::types::ChoiceRoute::OptionalCost)
                }),
                pending_cleared, effect_ready);

            if optional_skipped || pending_cleared {
                log::debug!(
                    "[RWC] optional_skipped={} pending_cleared={} completing ability",
                    optional_skipped,
                    pending_cleared
                );
                game_state.ability_queue.complete_current();
                game_state.clear_effect_tracking();
                let player_id = entry_player_id.clone().unwrap_or_else(|| "p1".to_string());
                game_state.just_completed_ability_key = just_completed_key.clone();
                game_state.process_pending_auto_abilities(&player_id);
                game_state.just_completed_ability_key = None;
                game_state.recently_moved_cards = None;
                game_state.recently_appeared_cards.clear();
                game_state.recently_state_changed.clear();
            } else if effect_ready {
                log::debug!("RWC: calling process_current_ability");
                log::debug!("[RWC_EFFECT_READY] storing resolver and calling PCA");
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
                    game_state.just_completed_ability_key = just_completed_key.clone();
                    game_state.process_pending_auto_abilities(&player_id);
                    game_state.just_completed_ability_key = None;
                } else {
                    // Effect completed without sub-choice — process any newly
                    // enqueued watcher abilities (e.g. each_time triggers).
                    let player_id = entry_player_id.clone().unwrap_or_else(|| "p1".to_string());
                    game_state.just_completed_ability_key = just_completed_key.clone();
                    game_state.process_pending_auto_abilities(&player_id);
                    game_state.just_completed_ability_key = None;
                }
            } else if cost_was_paid {
                // Record use_limit when ability completes (cost+effect both resolved).
                // Insert for any ability with use_limit, unless the player declined
                // an optional action (signaled by optional_cost_result == Some(false)).
                if cost_entry_opt_result != Some(false) {
                    if let Some(entry) = game_state.ability_queue.current_entry() {
                        if let Some(cid) = entry.card_id {
                            let turn = game_state.turn_number;
                            let key = format!("{}_{}_{}", cid, entry.ability_index, turn);
                            *game_state
                                .turn_limited_abilities_used
                                .entry(key)
                                .or_insert(0) += 1;
                        }
                    }
                }
                // Post-resolution each_time for LiveStart/LiveSuccess
                if cost_entry_opt_result != Some(false) {
                    let pid = entry_player_id.clone().unwrap_or_else(|| "p1".to_string());
                    if let Some(crate::game_state::AbilityTrigger::LiveStart) = cost_entry_trigger {
                        if let Some(cid) = cost_entry_card_id {
                            game_state.trigger_each_time_for_member(
                                &pid,
                                crate::triggers::LIVE_START,
                                cid,
                            );
                        }
                    } else if let Some(crate::game_state::AbilityTrigger::LiveSuccess) =
                        cost_entry_trigger
                    {
                        if let Some(cid) = cost_entry_card_id {
                            game_state.trigger_each_time_for_member(
                                &pid,
                                crate::triggers::LIVE_SUCCESS,
                                cid,
                            );
                        }
                    }
                }
                // Don't complete if a pending choice (e.g. SelectPosition) was
                // created by the current effect — it would be orphaned.
                if game_state.has_pending_choice() {
                    log::debug!("[RWC] skipping complete_current — pending choice exists");
                    log::debug!("[RWC] skipping complete_current — pending choice exists");
                    return Ok(());
                }
                // Post-resolution TAS scan for movement-based triggers.
                // Mirrors process_current_ability's post-resolution scan (line ~1072)
                // which is NOT called when a resolver completes via this path.
                // Must run AFTER complete_current() to match process_current_ability ordering.
                game_state.ability_queue.complete_current();
                game_state.clear_effect_tracking();
                let player_id = entry_player_id.clone().unwrap_or_else(|| "p1".to_string());
                if game_state.recently_moved_cards.is_some()
                    || game_state.last_energy_placed_by_effect()
                    || !game_state.recently_appeared_cards.is_empty()
                {
                    let event = crate::ability::types::TriggerEvent {
                        moved_cards: game_state.recently_moved_cards.clone().unwrap_or_default(),
                        moved_from_zone: game_state.recently_moved_from_zone.clone(),
                        position_change_occurred: game_state.position_change_occurred_this_turn,
                        energy_placed_by_effect: game_state.last_energy_placed_by_effect(),
                        energy_placed_by_player: game_state
                            .last_energy_placed_by_player()
                            .map(|s| s.to_string()),
                        ..Default::default()
                    };
                    game_state.just_completed_ability_key = just_completed_key.clone();
                    game_state.trigger_auto_abilities_for_player_with_event(&player_id, &event);
                    game_state.just_completed_ability_key = None;
                }
                game_state.just_completed_ability_key = just_completed_key.clone();
                game_state.process_pending_auto_abilities(&player_id);
                game_state.just_completed_ability_key = None;
                game_state.recently_moved_cards = None;
                game_state.recently_appeared_cards.clear();
                game_state.recently_state_changed.clear();
            } else {
                game_state.ability_queue.complete_current();
                game_state.clear_effect_tracking();
                let player_id = entry_player_id.clone().unwrap_or_else(|| "p1".to_string());
                if game_state.recently_moved_cards.is_some()
                    || game_state.last_energy_placed_by_effect()
                    || !game_state.recently_appeared_cards.is_empty()
                {
                    let event = crate::ability::types::TriggerEvent {
                        moved_cards: game_state.recently_moved_cards.clone().unwrap_or_default(),
                        moved_from_zone: game_state.recently_moved_from_zone.clone(),
                        position_change_occurred: game_state.position_change_occurred_this_turn,
                        energy_placed_by_effect: game_state.last_energy_placed_by_effect(),
                        energy_placed_by_player: game_state
                            .last_energy_placed_by_player()
                            .map(|s| s.to_string()),
                        ..Default::default()
                    };
                    game_state.just_completed_ability_key = just_completed_key.clone();
                    game_state.trigger_auto_abilities_for_player_with_event(&player_id, &event);
                    game_state.just_completed_ability_key = None;
                }
                game_state.just_completed_ability_key = just_completed_key.clone();
                game_state.process_pending_auto_abilities(&player_id);
                game_state.just_completed_ability_key = None;
                game_state.recently_moved_cards = None;
                game_state.recently_appeared_cards.clear();
                game_state.recently_state_changed.clear();
            }
        }
        Ok(())
    }

    pub fn check_timing(game_state: &mut GameState) {
        tdbg!("CHECK_TIMING:0");
        game_state.player1.refresh();
        tdbg!("CHECK_TIMING:1 p1.refresh OK");
        game_state.player2.refresh();
        tdbg!("CHECK_TIMING:2 p2.refresh OK");
        let p1_needs_refresh = game_state.player1.main_deck.cards.is_empty()
            && !game_state.player1.waitroom.cards.is_empty();
        let p2_needs_refresh = game_state.player2.main_deck.cards.is_empty()
            && !game_state.player2.waitroom.cards.is_empty();
        if p1_needs_refresh {
            let waitroom = std::mem::take(&mut game_state.player1.waitroom.cards);
            game_state
                .player1
                .main_deck
                .cards
                .extend(waitroom.iter().copied());
            game_state.player1.main_deck.shuffle();
            tdbg!("CHECK_TIMING:1b p1 refresh shuffled");
        }
        if p2_needs_refresh {
            let waitroom = std::mem::take(&mut game_state.player2.waitroom.cards);
            game_state
                .player2
                .main_deck
                .cards
                .extend(waitroom.iter().copied());
            game_state.player2.main_deck.shuffle();
            tdbg!("CHECK_TIMING:2b p2 refresh shuffled");
        }
        tdbg!("CHECK_TIMING:3 refresh done");
        Self::check_victory_condition(game_state);
        tdbg!("CHECK_TIMING:4 victory OK");
        let p1_id = game_state.player1.id.clone();
        let p2_id = game_state.player2.id.clone();
        Self::check_invalid_live_cards(game_state, &p1_id);
        tdbg!("CHECK_TIMING:5 invalid live p1 OK");
        Self::check_invalid_live_cards(game_state, &p2_id);
        tdbg!("CHECK_TIMING:6 invalid live p2 OK");
        Self::check_invalid_energy_cards(&mut game_state.player1, &game_state.card_database);
        Self::check_invalid_energy_cards(&mut game_state.player2, &game_state.card_database);
        tdbg!("CHECK_TIMING:7 invalid energy OK");
        Self::check_orphaned_under_cards(&mut game_state.player1, &game_state.card_database);
        Self::check_orphaned_under_cards(&mut game_state.player2, &game_state.card_database);
        tdbg!("CHECK_TIMING:8 orphaned under OK");
        game_state.recalculate_constants();
        tdbg!("CHECK_TIMING:9 recalc_constants OK");
        Self::check_invalid_resolution_zone(game_state);
        tdbg!("CHECK_TIMING:10 invalid resolution OK");
        if game_state.check_permanent_loop() {
            game_state.game_result = crate::game_state::GameResult::Draw;
            game_state.game_ended = true;
        }
        tdbg!("CHECK_TIMING:11 perm_loop OK");
        Self::check_victory_condition(game_state);
        tdbg!("CHECK_TIMING:12 victory2 OK");
        let active_player_id = game_state.active_player().id.clone();
        game_state.process_pending_auto_abilities(&active_player_id);
        tdbg!("CHECK_TIMING:13 auto_abilities OK");
    }

    pub fn check_victory_condition(game_state: &mut GameState) {
        let p1_success_count = game_state.player1.success_live_card_zone.cards.len();
        let p2_success_count = game_state.player2.success_live_card_zone.cards.len();

        // Q54: If 3+ cards end up in success zone simultaneously, game is a draw.
        // Q49: Turn order stays same if no player won. Q50: Same if both placed.
        // Q51: Turn order swaps to the player who placed (if only one did).
        // Q52: Turn order stays same if both had 2+ already and neither could place.
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

    // Q88: Players cannot voluntarily discard, retire members, move members,
    // or weigh active cards without an effect or cost.

    /// Rule 10.5.1: Non-live cards in live card zone → moved to discard.
    /// Also records movement events so turn-level tracking (turn_movements)
    /// captures which cards moved where, enabling "from live_card_zone to
    /// discard" conditions.
    fn check_invalid_live_cards(game_state: &mut GameState, player_id: &str) {
        let p1_id = game_state.player1.id.clone();
        let p2_id = game_state.player2.id.clone();
        if player_id != p1_id && player_id != p2_id {
            return;
        }
        // Two-pass: collect invalid IDs via immutable borrow, then mutate.
        // Avoids cloning the entire CardDatabase each call.
        let invalids: Vec<(usize, i16, bool)> = {
            let player = if player_id == p1_id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            player
                .live_card_zone
                .cards
                .iter()
                .enumerate()
                .filter_map(|(i, &card_id)| {
                    let card = game_state.card_database.get_card(card_id)?;
                    if !card.is_live() {
                        Some((i, card_id, card.is_energy()))
                    } else {
                        None
                    }
                })
                .collect()
        };
        if invalids.is_empty() {
            return;
        }
        let mut moved = Vec::new();
        for &(i, card_id, is_energy) in invalids.iter().rev() {
            let player = if player_id == p1_id {
                &mut game_state.player1
            } else {
                &mut game_state.player2
            };
            if i < player.live_card_zone.cards.len() {
                player.live_card_zone.cards.remove(i);
                if is_energy {
                    player.energy_deck.cards.push(card_id);
                    moved.push((card_id, "energy_deck"));
                } else {
                    player.waitroom.add_card(card_id);
                    moved.push((card_id, "waitroom"));
                }
            }
        }
        for (card_id, dest_zone) in moved {
            game_state.push_movement_event(
                card_id,
                "live_card_zone",
                dest_zone,
                None,
                player_id,
                false,
            );
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

    pub fn player_set_live_cards(player: &mut crate::player::Player, num_cards_to_set: usize) {
        let mut cards_set = Vec::new();
        let mut held_back = Vec::new();
        while let Some(card_id) = player.hand.cards.pop() {
            if cards_set.len() < num_cards_to_set {
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
