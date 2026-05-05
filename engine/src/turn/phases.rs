use crate::constants::MAX_LIVE_CARDS;
use crate::game_state::{GameState, Phase};

impl super::TurnEngine {
    pub fn advance_phase(game_state: &mut GameState) {
        debug_assert!(game_state.phase_invariant(), "Phase invariant violated before advance_phase");

        if matches!(game_state.current_phase, Phase::MulliganP1Turn | Phase::MulliganP2Turn) { return; }

        if game_state.current_turn_phase == crate::game_state::TurnPhase::FirstAttackerNormal || game_state.current_turn_phase == crate::game_state::TurnPhase::SecondAttackerNormal {
            match game_state.current_phase {
                Phase::Active => {
                    game_state.player1.activate_all_energy();
                    game_state.player2.activate_all_energy();
                    game_state.recalculate_constant_blade_modifiers();
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Energy;
                }
                Phase::Energy => {
                    game_state.recalculate_constant_blade_modifiers();
                    Self::check_timing(game_state);
                    let _drawn_card = game_state.active_player_mut().draw_energy();
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Draw;
                }
                Phase::Draw => {
                    Self::check_timing(game_state);
                    let _drawn = game_state.active_player_mut().draw_card();
                    game_state.recalculate_constant_blade_modifiers();
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Main;
                }
                Phase::Main => {
                    game_state.recalculate_constant_blade_modifiers();
                    Self::check_timing(game_state);
                    if game_state.current_turn_phase == crate::game_state::TurnPhase::FirstAttackerNormal {
                        game_state.current_turn_phase = crate::game_state::TurnPhase::SecondAttackerNormal;
                        game_state.current_phase = Phase::Active;
                    } else {
                        game_state.current_turn_phase = crate::game_state::TurnPhase::Live;
                        game_state.current_phase = if game_state.player1.is_first_attacker { Phase::LiveCardSetP1Turn } else { Phase::LiveCardSetP2Turn };
                    }
                }
                _ => {}
            }
        } else if game_state.current_turn_phase == crate::game_state::TurnPhase::Live {
            match game_state.current_phase {
                Phase::LiveCardSetP1Turn => { game_state.current_phase = Phase::LiveCardSetP2Turn; return; }
                Phase::LiveCardSetP2Turn => {
                    game_state.recalculate_constant_blade_modifiers();
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::FirstAttackerPerformance;
                    let first_attacker_id = if game_state.player1.is_first_attacker { game_state.player1.id.clone() } else { game_state.player2.id.clone() };
                    Self::trigger_live_start_abilities(game_state, &first_attacker_id);
                    game_state.process_pending_auto_abilities(&first_attacker_id);
                    return;
                }
                Phase::FirstAttackerPerformance => {
                    let blade_heart_count = {
                        let mut resolution_zone = std::mem::take(&mut game_state.resolution_zone);
                        let player_id = if game_state.player1.is_first_attacker { game_state.player1.id.clone() } else { game_state.player2.id.clone() };
                        let card_db = game_state.card_database.clone();
                        let bm = game_state.blade_modifiers.clone();
                        let ho = game_state.heart_override.clone();
                        let hm = game_state.heart_modifiers.clone();
                        let player = game_state.first_attacker_mut();
                        Self::player_perform_live(player, &mut resolution_zone, &player_id, &card_db, &bm, &ho, &hm)
                    };
                    game_state.player1_cheer_blade_heart_count = blade_heart_count;
                    game_state.current_phase = Phase::SecondAttackerPerformance;
                }
                Phase::SecondAttackerPerformance => {
                    let blade_heart_count = {
                        let mut resolution_zone = std::mem::take(&mut game_state.resolution_zone);
                        let player_id = if game_state.player1.is_first_attacker { game_state.player2.id.clone() } else { game_state.player1.id.clone() };
                        let card_db = game_state.card_database.clone();
                        let bm = game_state.blade_modifiers.clone();
                        let ho = game_state.heart_override.clone();
                        let hm = game_state.heart_modifiers.clone();
                        let player = game_state.second_attacker_mut();
                        Self::player_perform_live(player, &mut resolution_zone, &player_id, &card_db, &bm, &ho, &hm)
                    };
                    game_state.player2_cheer_blade_heart_count = blade_heart_count;
                    game_state.current_phase = Phase::LiveVictoryDetermination;
                }
                Phase::LiveVictoryDetermination => {
                    Self::execute_live_victory_determination(game_state);
                    if game_state.pending_choice.is_some() { return; }
                    game_state.turn_number += 1;
                    game_state.current_turn_phase = crate::game_state::TurnPhase::FirstAttackerNormal;
                    game_state.current_phase = Phase::Active;
                }
                _ => {}
            }
        }
    }

    pub(crate) fn handle_mulligan_selection(game_state: &mut GameState, card_id: Option<i16>, _card_indices: Option<Vec<usize>>) -> Result<(), String> {
        let idx = if let Some(indices) = _card_indices { indices.get(0).copied().unwrap_or(0) }
        else if let Some(cid) = card_id {
            let mulligan_player = match game_state.current_phase { Phase::MulliganP1Turn => &game_state.player1, Phase::MulliganP2Turn => &game_state.player2, _ => &game_state.player1 };
            mulligan_player.get_card_index_by_id(cid).unwrap_or(0)
        } else { 0 };
        if let Some(pos) = game_state.mulligan_selected_indices.iter().position(|&x| x == idx) { game_state.mulligan_selected_indices.remove(pos); }
        else { game_state.mulligan_selected_indices.push(idx); }
        Ok(())
    }

    pub(crate) fn handle_mulligan_confirmation(game_state: &mut GameState) -> Result<(), String> {
        let mulligan_count = game_state.mulligan_selected_indices.len();
        let (player, next_phase) = match game_state.current_phase {
            Phase::MulliganP1Turn => (&mut game_state.player1, Phase::MulliganP2Turn),
            Phase::MulliganP2Turn => (&mut game_state.player2, Phase::Active),
            _ => return Ok(()),
        };
        for &idx in game_state.mulligan_selected_indices.iter().rev() {
            if idx < player.hand.cards.len() { let card = player.hand.cards.remove(idx); player.main_deck.cards.push(card); }
        }
        player.main_deck.shuffle();
        for _ in 0..mulligan_count { if let Some(card) = player.main_deck.draw() { player.hand.add_card(card); } }
        game_state.current_phase = next_phase;
        println!("Mulligan confirmed: {} cards mulliganed", mulligan_count);
        Ok(())
    }

    pub(crate) fn handle_mulligan_skip(game_state: &mut GameState) -> Result<(), String> {
        match game_state.current_phase {
            Phase::MulliganP1Turn => { game_state.current_phase = Phase::MulliganP2Turn; }
            Phase::MulliganP2Turn => { game_state.current_phase = Phase::Active; }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn handle_set_live_card(game_state: &mut GameState, card_id: Option<i16>) -> Result<(), String> {
        let cid = card_id.ok_or("No card selected for live card set")?;
        let player = game_state.active_player_mut();
        let idx = player.get_card_index_by_id(cid).ok_or("Selected card not found in hand")?;
        if !player.hand.cards.is_empty() && idx < player.hand.cards.len() {
            let card = player.hand.cards.remove(idx);
            let live_cards = &mut player.live_card_zone.cards;
            if live_cards.len() >= MAX_LIVE_CARDS { return Err("Live card zone is full".to_string()); }
            live_cards.push(card);
            Ok(())
        } else { Err("Invalid card selection".to_string()) }
    }

    pub fn handle_play_member_to_stage(game_state: &mut GameState, card_id: Option<i16>, stage_area: Option<crate::zones::MemberArea>, use_baton_touch: Option<bool>) -> Result<(), String> {
        let use_baton_touch = use_baton_touch.unwrap_or(false);
        if use_baton_touch && game_state.is_action_prohibited("cannot_baton_touch") { return Err("Baton touch is prohibited by a restriction effect".to_string()); }

        let card_db = game_state.card_database.clone();
        let player = game_state.active_player_mut();
        let idx = if let Some(cid) = card_id { player.get_card_index_by_id(cid).ok_or_else(|| format!("Card with id {} not found in hand", cid))? }
        else { player.hand.cards.iter().position(|c| card_db.get_card(*c).map_or(false, |card| card.is_member())).ok_or_else(|| "No member cards in hand".to_string())? };

        let area = stage_area.unwrap_or_else(|| {
            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            // Prefer empty areas, but if all occupied, auto-select first for baton touch
            if let Some(empty) = areas.iter().find(|&&a| player.stage.get_area(a).is_none()) {
                *empty
            } else if !use_baton_touch {
                // No empty areas and baton touch not explicitly requested — try it
                areas[0]
            } else {
                areas[0]
            }
        });

        let card_id = player.hand.cards[idx];
        let card_no = card_db.get_card(card_id).map(|c| c.card_no.clone()).unwrap_or_default();
        let player_id = player.id.clone();

        let (cost_paid, baton_touch_used, replaced_member_cost) = player.move_card_from_hand_to_stage(idx, area, use_baton_touch, &card_db)?;
        game_state.record_card_movement(card_id);
        game_state.baton_touch_zero_cost = baton_touch_used && cost_paid == 0;
        game_state.baton_touch_replaced_member_cost = replaced_member_cost;

        game_state.active_player_mut().debut_count_this_turn += 1;
        game_state.record_card_appearance(card_id);

        Self::trigger_debut_abilities(game_state, &player_id, &card_no, cost_paid, baton_touch_used);
        Self::trigger_auto_abilities_for_player(game_state, &player_id);
        game_state.process_pending_auto_abilities(&player_id);
        game_state.recalculate_constant_blade_modifiers();

        if baton_touch_used {
            game_state.record_baton_touch();
            for area in [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide] {
                let card_no = if let Some(card_id) = game_state.active_player().stage.get_area(area) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        card.abilities.iter().filter(|ability| ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::BATON_TOUCH)))
                            .map(|ability| (format!("{}_{}", card.card_no, ability.full_text), card.card_no.clone())).collect()
                    } else { Vec::new() }
                } else { Vec::new() };
                for (ability_id, card_no) in card_no {
                    game_state.trigger_auto_ability(ability_id, crate::game_state::AbilityTrigger::Debut, player_id.clone(), Some(card_no), None);
                }
            }
        }

        Ok(())
    }

    pub fn setup_initial_energy(game_state: &mut GameState) {
        for _ in 0..3 {
            if let Some(card_id) = game_state.player1.energy_deck.draw() { let _ = game_state.player1.energy_zone.add_card(card_id, &game_state.card_database); }
            if let Some(card_id) = game_state.player2.energy_deck.draw() { let _ = game_state.player2.energy_zone.add_card(card_id, &game_state.card_database); }
        }
    }

    pub(crate) fn handle_rps_choice_p1(game_state: &mut GameState, choice: i32) -> Result<(), String> { game_state.player1_rps_choice = Some(choice); Self::resolve_rps_if_both_chosen(game_state) }
    pub(crate) fn handle_rps_choice_p2(game_state: &mut GameState, choice: i32) -> Result<(), String> { game_state.player2_rps_choice = Some(choice); Self::resolve_rps_if_both_chosen(game_state) }

    fn resolve_rps_if_both_chosen(game_state: &mut GameState) -> Result<(), String> {
        let p1_choice = match game_state.player1_rps_choice { Some(c) => c, None => return Ok(()) };
        let p2_choice = match game_state.player2_rps_choice { Some(c) => c, None => return Ok(()) };

        let rps_winner = match (p1_choice, p2_choice) {
            (0, 2) | (1, 0) | (2, 1) => 1,
            (2, 0) | (0, 1) | (1, 2) => 2,
            _ => { game_state.player1_rps_choice = None; game_state.player2_rps_choice = None; return Ok(()); }
        };
        game_state.rps_winner = Some(rps_winner);
        game_state.current_phase = Phase::ChooseFirstAttacker;
        Ok(())
    }
}
