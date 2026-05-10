use crate::card::CardDatabase;
use crate::game_state::GameState;
use crate::game_state::Phase;

impl super::TurnEngine {
    pub fn execute_main_phase_action(game_state: &mut GameState, action: &crate::game_setup::ActionType, card_id: Option<i16>, card_indices: Option<Vec<usize>>, stage_area: Option<crate::zones::MemberArea>, use_baton_touch: Option<bool>) -> Result<(), String> {
        if game_state.pending_choice.is_some() { return Self::resume_with_choice(game_state, card_id, card_indices); }

        match action {
            crate::game_setup::ActionType::Pass => {
                match game_state.current_phase {
                    Phase::LiveCardSetP1Turn => {
                        let player = game_state.active_player_mut();
                        let cards_placed = player.live_card_zone.cards.len();
                        for _ in 0..cards_placed { let _ = player.draw_card(); }
                        game_state.current_phase = Phase::LiveCardSetP2Turn;
                        Ok(())
                    }
                    Phase::LiveCardSetP2Turn => {
                        let player = game_state.active_player_mut();
                        let cards_placed = player.live_card_zone.cards.len();
                        for _ in 0..cards_placed { let _ = player.draw_card(); }
                        Self::advance_phase(game_state);
                        Ok(())
                    }
                    _ => { Self::advance_phase(game_state); Ok(()) }
                }
            }
            crate::game_setup::ActionType::MulliganHeader => Ok(()),
            crate::game_setup::ActionType::RockChoice | crate::game_setup::ActionType::PaperChoice | crate::game_setup::ActionType::ScissorsChoice => {
                let choice_value = match action { crate::game_setup::ActionType::RockChoice => 0, crate::game_setup::ActionType::PaperChoice => 1, crate::game_setup::ActionType::ScissorsChoice => 2, _ => unreachable!() };
                if game_state.player1_rps_choice.is_none() { Self::handle_rps_choice_p1(game_state, choice_value) }
                else { Self::handle_rps_choice_p2(game_state, choice_value) }
            }
            crate::game_setup::ActionType::ChooseFirstAttacker => {
                game_state.player1.is_first_attacker = true; game_state.player2.is_first_attacker = false;
                for _ in 0..6 { game_state.player1.draw_card(); game_state.player2.draw_card(); }
                game_state.current_phase = Phase::MulliganP1Turn; game_state.mulligan_selected_indices.clear();
                Ok(())
            }
            crate::game_setup::ActionType::ChooseSecondAttacker => {
                game_state.player1.is_first_attacker = false; game_state.player2.is_first_attacker = true;
                for _ in 0..6 { game_state.player1.draw_card(); game_state.player2.draw_card(); }
                game_state.current_phase = Phase::MulliganP1Turn; game_state.mulligan_selected_indices.clear();
                Ok(())
            }
            crate::game_setup::ActionType::SelectMulligan => Self::handle_mulligan_selection(game_state, card_id, card_indices),
            crate::game_setup::ActionType::ConfirmMulligan => Self::handle_mulligan_confirmation(game_state),
            crate::game_setup::ActionType::SkipMulligan => Self::handle_mulligan_skip(game_state),
            crate::game_setup::ActionType::PlayMemberToStage => Self::handle_play_member_to_stage(game_state, card_id, stage_area, use_baton_touch),
            crate::game_setup::ActionType::SetLiveCard => Self::handle_set_live_card(game_state, card_id),
            crate::game_setup::ActionType::FinishLiveCardSet => Err("FinishLiveCardSet action is obsolete - use Pass instead".into()),
            crate::game_setup::ActionType::UseAbility => Self::handle_use_ability(game_state, card_id),
            _ => Ok(()),
        }
    }

    fn handle_use_ability(game_state: &mut GameState, card_id: Option<i16>) -> Result<(), String> {
        let card_id = card_id.ok_or("No card specified for ability activation")?;
        if game_state.is_action_prohibited("cannot_activate") || game_state.is_action_prohibited("cannot_activate_by_effect") {
            return Err("Ability activation is prohibited by a restriction effect".to_string());
        }
        let card_db = game_state.card_database.clone();
        let card = card_db.get_card(card_id).ok_or("Card not found in database")?;
        if !card.is_member() { return Err("Only member cards can activate abilities".to_string()); }
        let player = game_state.active_player();
        // Check if card activates from hand (activation_condition has location=hand)
        let is_hand_activation = card.abilities.iter().any(|a| {
            a.triggers.as_ref().map_or(false, |t| t == crate::triggers::ACTIVATION)
                && a.effect.as_ref().and_then(|e| e.activation_condition_parsed.as_ref())
                    .map_or(false, |c| c.location.as_deref() == Some("hand"))
        });
        if is_hand_activation {
            // Card activates from hand — verify it's in hand, skip stage position checks
            if !player.hand.cards.contains(&card_id) {
                return Err("Card not found in hand".to_string());
            }
        } else {
            let stage_position = player.stage.stage.iter().position(|&id| id == card_id).ok_or("Card not found on stage")?;
            let stage_area = match stage_position { 0 => crate::zones::MemberArea::LeftSide, 1 => crate::zones::MemberArea::Center, _ => crate::zones::MemberArea::RightSide };
            if !crate::zones::check_trigger_position(card.abilities.iter().find(|a| a.triggers.as_ref().map_or(false, |t| t == crate::triggers::ACTIVATION)).and_then(|a| a.triggers.as_deref()), stage_area) {
                return Err("Ability cannot be activated from this position".to_string());
            }
            if let Some(ability) = card.abilities.iter().find(|a| a.triggers.as_ref().map_or(false, |t| t == crate::triggers::ACTIVATION)) {
                if !crate::zones::check_effect_position(
                    ability.effect.as_ref().and_then(|e| e.activation_position.as_deref()),
                    stage_area,
                ) {
                    return Err("Ability cannot be activated from this position".to_string());
                }
            }
        }
        let player_id = game_state.active_player().id.clone();
        for (_, ability) in card.abilities.iter().enumerate() {
            if ability.triggers.as_ref().map_or(false, |t| t == crate::triggers::ACTIVATION) {
                let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                game_state.trigger_auto_ability(ability_id, crate::game_state::AbilityTrigger::Activation, player_id.clone(), Some(card.card_no.clone()), Some(card_id));
                game_state.process_pending_auto_abilities(&player_id);
            }
        }
        Ok(())
    }

    pub fn resume_with_choice(game_state: &mut GameState, card_id: Option<i16>, card_indices: Option<Vec<usize>>) -> Result<(), String> {
        let choice = game_state.ability_queue.is_waiting_for_choice().cloned().ok_or("No pending choice to resume")?;
        let result = Self::build_choice_result(&choice, card_id, card_indices)?;
        Self::resume_queue_with_choice(game_state, choice, result)
    }

    fn build_choice_result(choice: &crate::ability_resolver::Choice, card_id: Option<i16>, card_indices: Option<Vec<usize>>) -> Result<crate::ability_resolver::ChoiceResult, String> {
        match choice {
            crate::ability_resolver::Choice::SelectCard { .. } => {
                Ok(crate::ability_resolver::ChoiceResult::CardSelected { indices: card_indices.unwrap_or_default() })
            }
            crate::ability_resolver::Choice::SelectTarget { target, .. } => {
                let selected = match target.as_str() {
                    "pay_optional_cost:skip_optional_cost" => {
                        if card_id == Some(1) { "pay_optional_cost".to_string() } else { "skip_optional_cost".to_string() }
                    }
                    "primary|alternative" => {
                        if card_id == Some(1) { "alternative".to_string() } else { "primary".to_string() }
                    }
                    "choice" | "choice_string" | "conditional_optional" => {
                        card_id.map(|id| id.to_string()).unwrap_or_else(|| "0".into())
                    }
                    _ => card_id.map(|id| id.to_string()).unwrap_or_else(|| "0".into())
                };
                Ok(crate::ability_resolver::ChoiceResult::TargetSelected { target: selected })
            }
            crate::ability_resolver::Choice::SelectPosition { .. } => {
                let pos = card_id.map(|id| match id { 0 => "left".into(), 1 => "center".into(), 2 => "right".into(), _ => "center".into() }).unwrap_or_else(|| "center".into());
                Ok(crate::ability_resolver::ChoiceResult::PositionSelected { position: pos })
            }
            crate::ability_resolver::Choice::SelectHeartColor { count: _, options, description: _ } => {
                let idx = card_id.unwrap_or(0) as usize;
                let chosen = if idx < options.len() { options[idx].clone() } else { "heart00".to_string() };
                Ok(crate::ability_resolver::ChoiceResult::HeartColorSelected { colors: vec![chosen] })
            }
            crate::ability_resolver::Choice::SelectHeartType { count: _, options, description: _ } => {
                let idx = card_id.unwrap_or(0) as usize;
                let chosen = if idx < options.len() { options[idx].clone() } else { "heart00".to_string() };
                Ok(crate::ability_resolver::ChoiceResult::HeartTypeSelected { types: vec![chosen] })
            }
        }
    }

    fn resume_queue_with_choice(game_state: &mut GameState, choice: crate::ability_resolver::Choice, result: crate::ability_resolver::ChoiceResult) -> Result<(), String> {
        game_state.ability_queue.resume_with_choice(result.clone());
        let saved_ctx = game_state.ability_queue.current_entry()
            .and_then(|e| e.execution_context.clone())
            .unwrap_or(crate::ability::types::ExecutionContext::None);
        // Capture whether pending sequential actions exist BEFORE choice resolution.
        // True  = effect execution was paused mid-way (pending sequential actions saved).
        // False = cost payment created the choice (effect hasn't started), or no sequential effect.
        let had_pending_sequential = game_state.pending_sequential_actions.as_ref()
            .map(|v| !v.is_empty()).unwrap_or(false);
        let (new_choice, looked_at, ctx, rev, res) = {
            let mut resolver = crate::ability_resolver::AbilityResolver::new(game_state);
            resolver.execution_context = saved_ctx.clone();
            resolver.pending_choice = Some(choice);
            let res = resolver.provide_choice_result(result);
            let new_choice = resolver.get_pending_choice().cloned();
            let looked_at = resolver.take_looked_at();
            let ctx = resolver.execution_context.clone();
            let rev = std::mem::take(&mut resolver.revealed_cost_cards);
            (new_choice, looked_at, ctx, rev, res)
        };
        game_state.looked_at_cards = looked_at;
        game_state.revealed_cost_cards = rev;
        if let Err(e) = res { game_state.ability_queue.complete_current(); game_state.pending_choice = None; return Err(e); }
        if let Some(c) = new_choice {
            if let Some(e) = game_state.ability_queue.current_entry_mut() {
                e.execution_context = Some(ctx);
            }
            game_state.pending_choice = c.to_frontend_json();
            game_state.ability_queue.pause_for_choice(c);
        }
        else {
            let cost_was_paid = game_state.ability_queue.current_entry().map_or(false, |e| e.cost_paid);
            game_state.pending_choice = None;
            game_state.activating_card = None;
            if cost_was_paid && !had_pending_sequential {
                // Cost was paid, no pending sequential actions existed before this choice
                // resolution — effect hasn't started yet → start it.
                game_state.process_current_ability();
                let player_id = game_state.ability_queue.current_entry()
                    .map(|e| e.player_id.clone())
                    .unwrap_or_else(|| "p1".to_string());
                game_state.process_pending_auto_abilities(&player_id);
            } else if cost_was_paid {
                // Cost was paid and pending sequential actions existed before this choice
                // resolution — they were just processed by the choice handler (e.g.
                // handle_choice_string_store). The ability effect is now complete.
                game_state.ability_queue.complete_current();
                let player_id = game_state.ability_queue.current_entry()
                    .map(|e| e.player_id.clone())
                    .unwrap_or_else(|| "p1".to_string());
                game_state.process_pending_auto_abilities(&player_id);
            } else {
                game_state.ability_queue.complete_current();
            }
        }
        Ok(())
    }

    pub fn check_timing(game_state: &mut GameState) {
        game_state.player1.refresh();
        game_state.player2.refresh();
        let p1_needs_refresh = game_state.player1.main_deck.cards.is_empty() && !game_state.player1.waitroom.cards.is_empty();
        let p2_needs_refresh = game_state.player2.main_deck.cards.is_empty() && !game_state.player2.waitroom.cards.is_empty();
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
        Self::check_victory_condition(game_state);
        Self::check_invalid_cards(&mut game_state.player1, &game_state.card_database);
        Self::check_invalid_cards(&mut game_state.player2, &game_state.card_database);
        Self::check_invalid_resolution_zone(game_state);
        if game_state.check_permanent_loop() { game_state.game_result = crate::game_state::GameResult::Draw; game_state.game_ended = true; }
        Self::check_victory_condition(game_state);
        let active_player_id = game_state.active_player().id.clone();
        game_state.process_pending_auto_abilities(&active_player_id);
    }

    pub fn check_victory_condition(game_state: &mut GameState) {
        let p1_success_count = game_state.player1.success_live_card_zone.cards.len();
        let p2_success_count = game_state.player2.success_live_card_zone.cards.len();
        if p1_success_count >= crate::constants::VICTORY_CARD_COUNT { game_state.game_result = crate::game_state::GameResult::FirstAttackerWins; }
        else if p2_success_count >= crate::constants::VICTORY_CARD_COUNT { game_state.game_result = crate::game_state::GameResult::SecondAttackerWins; }
    }

    fn check_invalid_cards(player: &mut crate::player::Player, card_db: &CardDatabase) {
        let mut invalid_indices = Vec::new();
        for (i, card_id) in player.live_card_zone.cards.iter().enumerate() {
            if !card_db.get_card(*card_id).map_or(false, |c| c.is_live()) { invalid_indices.push(i); }
        }
        for &i in invalid_indices.iter().rev() {
            if i < player.live_card_zone.cards.len() { let card_id = player.live_card_zone.cards.remove(i); player.waitroom.add_card(card_id); }
        }
    }

    fn check_invalid_resolution_zone(game_state: &mut GameState) {
        let cards = std::mem::take(&mut game_state.resolution_zone.cards);
        if cards.is_empty() { return; }
        let player = game_state.active_player_mut();
        for card_id in cards { player.waitroom.add_card(card_id); }
    }

    pub fn player_set_live_cards(player: &mut crate::player::Player, num_cards_to_set: usize, card_database: &crate::card::CardDatabase) {
        let mut cards_set = Vec::new();
        let mut held_back = Vec::new();
        while let Some(card_id) = player.hand.cards.pop() {
            if cards_set.len() < num_cards_to_set && card_database.get_card(card_id).map_or(false, |c| c.is_live()) {
                cards_set.push(card_id);
            } else { held_back.push(card_id); }
        }
        for card_id in cards_set { player.live_card_zone.cards.push(card_id); }
        player.hand.cards = held_back.into_iter().rev().collect();
    }
}
