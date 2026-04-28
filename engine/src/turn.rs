use crate::game_state::{GameState, Phase, TurnPhase};
use crate::card::CardDatabase;
use crate::constants::{VICTORY_CARD_COUNT, MAX_LIVE_CARDS};
use std::vec::Vec;
use std::string::String;

pub struct TurnEngine;

impl TurnEngine {
    // ... (rest of the code remains the same)
    pub fn advance_phase(game_state: &mut GameState) {
        // Advance phase according to rules 7.1.2, 7.3.3, and 8.1.2
        debug_assert!(game_state.phase_invariant(), "Phase invariant violated before advance_phase");

        // Mulligan phases are manual - don't auto-advance them
        // Players must explicitly choose SkipMulligan or ConfirmMulligan actions
        if matches!(game_state.current_phase, Phase::MulliganP1Turn | Phase::MulliganP2Turn | Phase::Mulligan) {
            return;
        }

        // Handle normal phase sub-phases (Rule 7.3.3)
        if game_state.current_turn_phase == TurnPhase::FirstAttackerNormal || game_state.current_turn_phase == TurnPhase::SecondAttackerNormal {
            match game_state.current_phase {
                Phase::Active => {
                    // Rule 7.4: Activate all energy and stage cards (automatic)
                    // Activate BOTH players' energy, not just active player
                    game_state.player1.activate_all_energy();
                    game_state.player2.activate_all_energy();
                    // eprintln!("After Active phase: p1 active={}, p2 active={}", game_state.player1.energy_zone.active_count(), game_state.player2.energy_zone.active_count());
                    // Orientation tracking moved to GameState modifiers
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Energy;
                }
                Phase::Energy => {
                    // Rule 7.5: Draw energy card (automatic)
                    Self::check_timing(game_state);
                    let _ = game_state.active_player_mut().draw_energy();
                    // eprintln!("After Energy phase: p1 active={}, p2 active={}", game_state.player1.energy_zone.active_count(), game_state.player2.energy_zone.active_count());
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Draw;
                }
                Phase::Draw => {
                    // Rule 7.6: Draw card (automatic)
                    Self::check_timing(game_state);
                    let _ = game_state.active_player_mut().draw_card();
                    // eprintln!("After Draw phase: p1 active={}, p2 active={}", game_state.player1.energy_zone.active_count(), game_state.player2.energy_zone.active_count());
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Main;
                }
                Phase::Main => {
                    // Rule 7.7: Main phase complete, advance to next turn phase
                    Self::check_timing(game_state);
                    if game_state.current_turn_phase == TurnPhase::FirstAttackerNormal {
                        game_state.current_turn_phase = TurnPhase::SecondAttackerNormal;
                        game_state.current_phase = Phase::Active;
                    } else {
                        // Set current_turn_phase to Live BEFORE setting current_phase to LiveCardSet
                        // This ensures active_player() works correctly during LiveCardSet
                        game_state.current_turn_phase = TurnPhase::Live;
                        game_state.current_phase = if game_state.player1.is_first_attacker {
                            Phase::LiveCardSetP1Turn
                        } else {
                            Phase::LiveCardSetP2Turn
                        };
                    }
                }
                _ => {}
            }
        }
        // Handle live phase sub-phases (Rule 8.1.2)
        else if game_state.current_turn_phase == TurnPhase::Live {
            match game_state.current_phase {
                Phase::LiveCardSetP1Turn => {
                    // Transition from P1 turn to P2 turn
                    game_state.current_phase = Phase::LiveCardSetP2Turn;
                    return;
                }
                Phase::LiveCardSetP2Turn => {
                    // Both done, advance to FirstAttackerPerformance
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::FirstAttackerPerformance;
                    let first_attacker_id = if game_state.player1.is_first_attacker {
                        game_state.player1.id.clone()
                    } else {
                        game_state.player2.id.clone()
                    };
                    Self::trigger_live_start_abilities(game_state, &first_attacker_id);
                    Self::trigger_performance_phase_start_abilities(game_state, &first_attacker_id);
                    return;
                }
                Phase::LiveCardSet => {
                    // Legacy phase - should not be used anymore, but handle for compatibility
                    // Transition to FirstAttackerPerformance
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::FirstAttackerPerformance;
                    let first_attacker_id = if game_state.player1.is_first_attacker {
                        game_state.player1.id.clone()
                    } else {
                        game_state.player2.id.clone()
                    };
                    Self::trigger_live_start_abilities(game_state, &first_attacker_id);
                    Self::trigger_performance_phase_start_abilities(game_state, &first_attacker_id);
                    return;
                }
                Phase::FirstAttackerPerformance => {
                    // Rule 8.3: First attacker performs (automatic)
                    let blade_heart_count = {
                        // Take resolution_zone first to avoid borrow conflicts
                        let mut resolution_zone = std::mem::take(&mut game_state.resolution_zone);
                        let player_id = if game_state.player1.is_first_attacker {
                            game_state.player1.id.clone()
                        } else {
                            game_state.player2.id.clone()
                        };
                        let card_db = game_state.card_database.clone();
                        let player = game_state.first_attacker_mut();
                        Self::player_perform_live(player, &mut resolution_zone, &player_id, &card_db)
                    };
                    game_state.player1_cheer_blade_heart_count = blade_heart_count;

                    game_state.current_phase = Phase::SecondAttackerPerformance;
                }
                Phase::SecondAttackerPerformance => {
                    // Rule 8.3: Second attacker performs (automatic)
                    let blade_heart_count = {
                        // Take resolution_zone first to avoid borrow conflicts
                        let mut resolution_zone = std::mem::take(&mut game_state.resolution_zone);
                        let player_id = if game_state.player1.is_first_attacker {
                            game_state.player2.id.clone()
                        } else {
                            game_state.player1.id.clone()
                        };
                        let card_db = game_state.card_database.clone();
                        let player = game_state.second_attacker_mut();
                        Self::player_perform_live(player, &mut resolution_zone, &player_id, &card_db)
                    };
                    game_state.player2_cheer_blade_heart_count = blade_heart_count; // This is actually total blades for cheer bonus

                    game_state.current_phase = Phase::LiveVictoryDetermination;
                }
                Phase::LiveVictoryDetermination => {
                    // Rule 8.4: Determine live victory (automatic)
                    Self::execute_live_victory_determination(game_state);
                    
                    // After Live phase completes, start a new turn
                    // Rule 7.1.2: Each turn consists of FirstAttackerNormal, SecondAttackerNormal, Live phases
                    // After Live completes, increment turn number and start new FirstAttackerNormal phase
                    game_state.turn_number += 1;
                    game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
                    game_state.current_phase = Phase::Active;
                }
                _ => {}
            }
        }
    }

    fn handle_mulligan_selection(game_state: &mut GameState, card_id: Option<i16>, _card_indices: Option<Vec<usize>>) -> Result<(), String> {
        // Toggle card selection for mulligan
        let idx = if let Some(indices) = _card_indices {
            indices.get(0).copied().unwrap_or(0)
        } else if let Some(cid) = card_id {
            // Determine current player by phase
            let mulligan_player = match game_state.current_phase {
                Phase::MulliganP1Turn => &game_state.player1,
                Phase::MulliganP2Turn => &game_state.player2,
                _ => &game_state.player1, // fallback
            };
            mulligan_player.get_card_index_by_id(cid).unwrap_or(0)
        } else {
            0
        };
        if let Some(pos) = game_state.mulligan_selected_indices.iter().position(|&x| x == idx) {
            // Already selected, deselect
            game_state.mulligan_selected_indices.remove(pos);
        } else {
            // Not selected, select
            game_state.mulligan_selected_indices.push(idx);
        }
        Ok(())
    }

    fn handle_mulligan_confirmation(game_state: &mut GameState) -> Result<(), String> {
        // Simple mulligan: return selected cards, draw new ones, shuffle
        let indices = game_state.mulligan_selected_indices.clone();
        
        let current_player = match game_state.current_phase {
            Phase::MulliganP1Turn => &mut game_state.player1,
            Phase::MulliganP2Turn => &mut game_state.player2,
            _ => return Err("Not in mulligan phase".to_string()),
        };

        // Return selected cards and draw new ones
        if !indices.is_empty() {
            let sorted_indices = {
                let mut sorted = indices.clone();
                sorted.sort_by(|a, b| b.cmp(a));
                sorted
            };

            for idx in sorted_indices {
                if idx < current_player.hand.cards.len() {
                    let card = current_player.hand.cards.remove(idx);
                    current_player.main_deck.cards.push(card);
                }
            }

            for _ in 0..indices.len() {
                let _ = current_player.draw_card();
            }

            // Shuffle deck
            use rand::seq::SliceRandom;
            let mut deck_vec: Vec<_> = current_player.main_deck.cards.drain(..).collect();
            deck_vec.shuffle(&mut rand::thread_rng());
            for card in deck_vec {
                current_player.main_deck.cards.push(card);
            }
        }

        // Clear selection and advance to next phase
        game_state.mulligan_selected_indices.clear();
        
        let p1_is_first = game_state.player1.is_first_attacker;
        match game_state.current_phase {
            Phase::MulliganP1Turn => {
                if p1_is_first {
                    // P1 first, go to P2 mulligan
                    game_state.current_phase = Phase::MulliganP2Turn;
                } else {
                    // P1 second, go to Active
                    Self::setup_initial_energy(game_state);
                    game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
                    game_state.current_phase = Phase::Active;
                }
            }
            Phase::MulliganP2Turn => {
                if p1_is_first {
                    // P2 second, go to Active
                    Self::setup_initial_energy(game_state);
                    game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
                    game_state.current_phase = Phase::Active;
                } else {
                    // P2 first, go to P1 mulligan
                    game_state.current_phase = Phase::MulliganP1Turn;
                }
            }
            _ => {}
        }
        
        Ok(())
    }

    fn handle_mulligan_skip(game_state: &mut GameState) -> Result<(), String> {
        // Player chooses not to mulligan - advance to next phase
        game_state.mulligan_selected_indices.clear();
        let p1_is_first = game_state.player1.is_first_attacker;

        match game_state.current_phase {
            Phase::MulliganP1Turn => {
                if p1_is_first {
                    // P1 first, go to P2 mulligan
                    game_state.current_phase = Phase::MulliganP2Turn;
                } else {
                    // P1 second, go to Active
                    Self::setup_initial_energy(game_state);
                    game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
                    game_state.current_phase = Phase::Active;
                }
            }
            Phase::MulliganP2Turn => {
                if p1_is_first {
                    // P2 second, go to Active
                    Self::setup_initial_energy(game_state);
                    game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
                    game_state.current_phase = Phase::Active;
                } else {
                    // P2 first, go to P1 mulligan
                    game_state.current_phase = Phase::MulliganP1Turn;
                }
            }
            _ => {}
        }
        
        Ok(())
    }

    fn handle_set_live_card(game_state: &mut GameState, card_id: Option<i16>) -> Result<(), String> {
        // Rule 8.2: Live Card Set Phase - Place individual card face-down, max MAX_LIVE_CARDS cards
        let card_idx = if let Some(cid) = card_id {
            let active_player = game_state.active_player();
            active_player.hand.cards.iter()
                .position(|c| *c == cid)
        } else {
            None
        };
        let card_db = game_state.card_database.clone();

        if let Some(idx) = card_idx {
            // Place a single card
            let player = game_state.active_player_mut();
            if idx < player.hand.cards.len() && player.live_card_zone.cards.len() < MAX_LIVE_CARDS {
                let card = player.hand.cards.remove(idx);
                let card_no = card_db.get_card(card).map(|c| c.card_no.clone()).unwrap_or_default();
                let _ = player.live_card_zone.add_card(card, true, &card_db);

                // Trigger live start abilities for the set live card
                let player_id = player.id.clone();
                Self::trigger_live_start_abilities_for_card(game_state, &player_id, &card_no);

                // Process the triggered live start abilities
                game_state.process_pending_auto_abilities(&player_id);
            }
        } else {
            // No card selected, finish this player's live card set
            // Transition to next player's phase
            if game_state.current_turn_phase == crate::game_state::TurnPhase::Live {
                match game_state.current_phase {
                    Phase::LiveCardSetP1Turn => {
                        game_state.current_phase = Phase::LiveCardSetP2Turn;
                    }
                    Phase::LiveCardSetP2Turn => {
                        // Both done - advance via advance_phase
                        Self::advance_phase(game_state);
                    }
                    Phase::LiveCardSet => {
                        // Legacy phase - use flag-based transition for compatibility
                        if game_state.current_live_card_set_player == 0 {
                            game_state.current_live_card_set_player = 1;
                        } else {
                            game_state.current_live_card_set_player = 2;
                        }
                        if game_state.current_live_card_set_player == 2 {
                            Self::advance_phase(game_state);
                        }
                    }
                    _ => {
                        Self::advance_phase(game_state);
                    }
                }
            } else {
                Self::advance_phase(game_state);
            }
        }
        Ok(())
    }

    fn handle_finish_live_card_set(game_state: &mut GameState) -> Result<(), String> {
        // Rule 8.2: Live Card Set Phase - Finish live card set and advance phase
        // This should only be called during the Live phase
        if game_state.current_turn_phase == crate::game_state::TurnPhase::Live &&
           game_state.current_phase == crate::game_state::Phase::LiveCardSet {

            // Use the consolidated active_player() method to determine current player
            let current_player = game_state.active_player_mut();
            let active_player_id = current_player.id.clone();

            // Draw cards equal to number of cards placed in live zone
            let cards_placed = current_player.live_card_zone.cards.len();
            for _ in 0..cards_placed {
                let _ = current_player.draw_card();
            }

            // Mark current player as finished based on who is currently active
            if active_player_id == "player1" {
                game_state.current_live_card_set_player = 1; // P1 done, advance to P2
            } else {
                game_state.current_live_card_set_player = 2; // P2 done, mark both done
            }

            // Check if both players finished
            if game_state.current_live_card_set_player == 2 {
                Self::check_timing(game_state);
                game_state.current_phase = crate::game_state::Phase::FirstAttackerPerformance;
                let first_attacker_id = if game_state.player1.is_first_attacker {
                    game_state.player1.id.clone()
                } else {
                    game_state.player2.id.clone()
                };
                Self::trigger_live_start_abilities(game_state, &first_attacker_id);
                Self::trigger_performance_phase_start_abilities(game_state, &first_attacker_id);
            }
        } else {
            // Not in Live phase - this is an error condition, but don't regress phases
            // Just return an error instead of calling advance_phase which could regress
            return Err("Cannot finish live card set outside of Live phase".to_string());
        }
        Ok(())
    }

    
    fn handle_play_member_to_stage(game_state: &mut GameState, card_id: Option<i16>, stage_area: Option<crate::zones::MemberArea>, use_baton_touch: Option<bool>) -> Result<(), String> {
        let card_db = game_state.card_database.clone();
        let player = game_state.active_player_mut();

        // Find card by card_id in hand using HashMap (O(1) lookup)
        let idx = if let Some(cid) = card_id {
            player.get_card_index_by_id(cid)
                .ok_or_else(|| format!("Card with id {} not found in hand", cid))?
        } else {
            // Fallback: play first member card
            player.hand.cards.iter()
                .position(|c| card_db.get_card(*c).map_or(false, |card| card.is_member()))
                .ok_or_else(|| "No member cards in hand".to_string())?
        };

        // Use provided stage_area if available, otherwise find first available
        let area = if let Some(sa) = stage_area {
            sa
        } else {
            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            let mut area_enum = crate::zones::MemberArea::LeftSide;
            for area in areas {
                if player.stage.get_area(area).is_none() {
                    area_enum = area;
                    break;
                }
            }
            area_enum
        };

        let card_id = player.hand.cards[idx];
        let card_no = card_db.get_card(card_id).map(|c| c.card_no.clone()).unwrap_or_default();
        let player_id = player.id.clone();
        let use_baton_touch = use_baton_touch.unwrap_or(false);

        let (cost_paid, baton_touch_used) = player.move_card_from_hand_to_stage(idx, area, use_baton_touch, &card_db)?;

        game_state.record_card_movement(card_id);
        game_state.baton_touch_zero_cost = baton_touch_used && cost_paid == 0;

        Self::trigger_debut_abilities(game_state, &player_id, &card_no, cost_paid, baton_touch_used);
        game_state.process_pending_auto_abilities(&player_id);

        if baton_touch_used {
            game_state.record_baton_touch();
            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            for area in areas {
                let card_no = if let Some(card_id) = game_state.active_player().stage.get_area(area) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        card.abilities.iter()
                            .filter(|ability| ability.triggers.as_ref().map_or(false, |t| t == "baton touch"))
                            .map(|ability| (format!("{}_{}", card.card_no, ability.full_text), card.card_no.clone()))
                            .collect::<Vec<(String, String)>>()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                for (ability_id, card_no) in card_no {
                    game_state.trigger_auto_ability(
                        ability_id,
                        crate::game_state::AbilityTrigger::Debut,
                        player_id.clone(),
                        Some(card_no),
                    );
                }
            }
        }

        Ok(())
    }

    pub fn execute_main_phase_action(game_state: &mut GameState, action: &crate::game_setup::ActionType, card_id: Option<i16>, _card_indices: Option<Vec<usize>>, stage_area: Option<crate::zones::MemberArea>, use_baton_touch: Option<bool>) -> Result<(), String> {
        // Execute player choice action during various phases
        match action {
            crate::game_setup::ActionType::MulliganHeader => {
                // MulliganHeader is a display-only action, no execution needed
                Ok(())
            }
            crate::game_setup::ActionType::RockChoice |
            crate::game_setup::ActionType::PaperChoice |
            crate::game_setup::ActionType::ScissorsChoice => {
                // Determine which player is choosing based on who has already chosen
                let choice_value = match action {
                    crate::game_setup::ActionType::RockChoice => 0,
                    crate::game_setup::ActionType::PaperChoice => 1,
                    crate::game_setup::ActionType::ScissorsChoice => 2,
                    _ => unreachable!(),
                };
                
                // If P1 hasn't chosen yet, this is P1's choice
                // Otherwise, it's P2's choice
                if game_state.player1_rps_choice.is_none() {
                    Self::handle_rps_choice_p1(game_state, choice_value)
                } else {
                    Self::handle_rps_choice_p2(game_state, choice_value)
                }
            }
            crate::game_setup::ActionType::ChooseFirstAttacker => {
                // RPS winner chooses who goes first
                // Check who won RPS and validate they can make this choice
                let winner = game_state.rps_winner.ok_or("No RPS winner determined")?;
                
                println!("DEBUG: ChooseFirstAttacker action executed, RPS winner: {}", winner);
                
                game_state.player1.is_first_attacker = true;
                game_state.player2.is_first_attacker = false;
                
                // Draw 6 cards to hand
                for _ in 0..6 {
                    game_state.player1.draw_card();
                    game_state.player2.draw_card();
                }
                
                // Go to mulligan phase (P1 goes first since they're first attacker)
                game_state.current_phase = crate::game_state::Phase::MulliganP1Turn;
                game_state.mulligan_selected_indices.clear();
                
                println!("DEBUG: Phase transitioned to MulliganP1Turn");
                Ok(())
            }
            crate::game_setup::ActionType::ChooseSecondAttacker => {
                // RPS winner chooses who goes first
                // Check who won RPS and validate they can make this choice
                let winner = game_state.rps_winner.ok_or("No RPS winner determined")?;
                
                println!("DEBUG: ChooseSecondAttacker action executed, RPS winner: {}", winner);
                
                game_state.player1.is_first_attacker = false;
                game_state.player2.is_first_attacker = true;
                
                // Draw 6 cards to hand
                for _ in 0..6 {
                    game_state.player1.draw_card();
                    game_state.player2.draw_card();
                }
                
                // Go to mulligan phase (P2 goes first since they're first attacker)
                game_state.current_phase = crate::game_state::Phase::MulliganP2Turn;
                game_state.mulligan_selected_indices.clear();
                
                println!("DEBUG: Phase transitioned to MulliganP2Turn");
                Ok(())
            }
            crate::game_setup::ActionType::SelectMulligan => {
                Self::handle_mulligan_selection(game_state, card_id, _card_indices)
            }
            crate::game_setup::ActionType::ConfirmMulligan => {
                Self::handle_mulligan_confirmation(game_state)
            }
            crate::game_setup::ActionType::SkipMulligan => {
                Self::handle_mulligan_skip(game_state)
            }
            crate::game_setup::ActionType::PlayMemberToStage => {
                Self::handle_play_member_to_stage(game_state, card_id, stage_area, use_baton_touch)
            }
            crate::game_setup::ActionType::SetLiveCard => {
                Self::handle_set_live_card(game_state, card_id)
            }
            crate::game_setup::ActionType::FinishLiveCardSet => {
                Self::handle_finish_live_card_set(game_state)
            }
            crate::game_setup::ActionType::Pass => {
                // Handle Pass differently based on current phase
                match game_state.current_phase {
                    Phase::LiveCardSetP1Turn => {
                        // P1 passes, transition to P2
                        game_state.current_phase = Phase::LiveCardSetP2Turn;
                        Ok(())
                    }
                    Phase::LiveCardSetP2Turn => {
                        // P2 passes, advance to Performance
                        Self::advance_phase(game_state);
                        Ok(())
                    }
                    Phase::LiveCardSet => {
                        // Legacy phase - use flag-based transition for compatibility
                        if game_state.current_live_card_set_player == 0 {
                            game_state.current_live_card_set_player = 1;
                        } else {
                            game_state.current_live_card_set_player = 2;
                        }
                        if game_state.current_live_card_set_player == 2 {
                            Self::check_timing(game_state);
                            Self::advance_phase(game_state);
                        }
                        Ok(())
                    }
                    _ => {
                        // In other phases, Pass advances the phase
                        Self::advance_phase(game_state);
                        Ok(())
                    }
                }
            }
            crate::game_setup::ActionType::UseAbility => {
                Self::handle_use_ability(game_state, card_id)
            }
        }
    }

    fn handle_use_ability(game_state: &mut GameState, card_id: Option<i16>) -> Result<(), String> {
        if let Some(card_id) = card_id {
            let card_db = game_state.card_database.clone();
            let turn_number = game_state.turn_number;
            let player = game_state.active_player_mut();

            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            let mut found_card = None;
            for area in areas {
                if let Some(stage_card_id) = player.stage.get_area(area) {
                    if stage_card_id == card_id {
                        found_card = Some((area, stage_card_id));
                        break;
                    }
                }
            }

            if let Some((_area, stage_card_id)) = found_card {
                if let Some(card) = card_db.get_card(stage_card_id) {
                    let player_id = player.id.clone();
                    for (ability_index, ability) in card.abilities.iter().enumerate() {
                        if ability.triggers.as_ref().map_or(false, |t| t == "Debut") {
                            if ability.use_limit.is_some() {
                                let ability_key = format!("{}_{}_{}", stage_card_id, ability_index, turn_number);
                                game_state.turn_limited_abilities_used.insert(ability_key);
                            }

                            let should_trigger_effect = true;
                            if let Some(ref cost) = ability.cost {
                                if cost.cost_type.as_deref() == Some("choice_condition") {
                                    let cost_text = &cost.text;

                                    if let Some(ref cost_options) = cost.options {
                                        let option_texts: Vec<&str> = cost_options.iter()
                                            .map(|opt| opt.text.as_str())
                                            .collect();

                                        let option_indices: Vec<String> = cost_options.iter()
                                            .enumerate()
                                            .map(|(i, _)| i.to_string())
                                            .collect();

                                        let options_display = option_texts.iter()
                                            .enumerate()
                                            .map(|(i, text)| format!("{}. {}", i + 1, text))
                                            .collect::<Vec<_>>()
                                            .join("\n");

                                        let mut resolver = crate::ability_resolver::AbilityResolver::new(game_state);
                                        resolver.pending_choice = Some(crate::ability_resolver::Choice::SelectTarget {
                                            target: option_indices.join("|"),
                                            description: format!("Pay cost to activate ability:\n{}\n{}\nSelect option:", cost_text, options_display),
                                        });
                                    }
                                }

                                if should_trigger_effect {
                                    let mut resolver = crate::ability_resolver::AbilityResolver::new(game_state);
                                    if let Err(e) = resolver.pay_cost(cost) {
                                        return Err(format!("Failed to pay cost: {}", e));
                                    }
                                }
                            }

                            if should_trigger_effect {
                                let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                                game_state.trigger_auto_ability(
                                    ability_id,
                                    crate::game_state::AbilityTrigger::Activation,
                                    player_id.clone(),
                                    Some(card.card_no.clone()),
                                );
                            }
                        }
                    }
                }
            } else {
                return Err("Card not found on stage".to_string());
            }
        } else {
            return Err("No card specified for ability activation".to_string());
        }
        Ok(())
    }

    pub fn setup_initial_energy(game_state: &mut GameState) {
        // Rule 6.2.7: Initial energy - Each player draws 3 cards from energy deck to Energy Zone
        for _ in 0..3 {
            if let Some(card_id) = game_state.player1.energy_deck.draw() {
                let _ = game_state.player1.energy_zone.add_card(card_id, &game_state.card_database);
            }
            if let Some(card_id) = game_state.player2.energy_deck.draw() {
                let _ = game_state.player2.energy_zone.add_card(card_id, &game_state.card_database);
            }
        }
    }

    fn handle_rps_choice_p1(game_state: &mut GameState, choice: i32) -> Result<(), String> {
        // Store P1's choice
        game_state.player1_rps_choice = Some(choice);
        
        // Check if both players have chosen
        Self::resolve_rps_if_both_chosen(game_state)
    }
    
    fn handle_rps_choice_p2(game_state: &mut GameState, choice: i32) -> Result<(), String> {
        // Store P2's choice
        game_state.player2_rps_choice = Some(choice);
        
        // Check if both players have chosen
        Self::resolve_rps_if_both_chosen(game_state)
    }
    
    fn resolve_rps_if_both_chosen(game_state: &mut GameState) -> Result<(), String> {
        // If either player hasn't chosen yet, stay in RPS phase
        let p1_choice = match game_state.player1_rps_choice {
            Some(c) => c,
            None => return Ok(()),
        };
        let p2_choice = match game_state.player2_rps_choice {
            Some(c) => c,
            None => return Ok(()),
        };
        
        // Both have chosen - determine winner
        let rps_winner = match (p1_choice, p2_choice) {
            (0, 2) | (1, 0) | (2, 1) => 1, // Rock beats Scissors, Paper beats Rock, Scissors beats Paper
            (2, 0) | (0, 1) | (1, 2) => 2, // Player 2 wins
            _ => {
                // Tie - reset and wait for both to choose again
                game_state.player1_rps_choice = None;
                game_state.player2_rps_choice = None;
                return Ok(());
            }
        };
        
        // Store winner and let them choose turn order
        game_state.rps_winner = Some(rps_winner);
        game_state.current_phase = crate::game_state::Phase::ChooseFirstAttacker;
        
        Ok(())
    }

    pub fn execute_live_victory_determination(game_state: &mut GameState) {
        // Rule 8.4: Determine live victory
        // Rule 8.4.2.1: Add cheer blade heart count to score
        // Calculate stage hearts for heart satisfaction bonus
        let player1_stage_hearts = game_state.player1.calculate_stage_hearts(&game_state.card_database);
        let player2_stage_hearts = game_state.player2.calculate_stage_hearts(&game_state.card_database);
        
        // Store hearts temporarily to extend their lifetime
        game_state.player1.stage_hearts = Some(player1_stage_hearts);
        game_state.player2.stage_hearts = Some(player2_stage_hearts);
        
        let player1_score = game_state.player1.live_card_zone.calculate_live_score(&game_state.card_database, game_state.player1_cheer_blade_heart_count, game_state.player1.stage_hearts.as_ref());
        let player2_score = game_state.player2.live_card_zone.calculate_live_score(&game_state.card_database, game_state.player2_cheer_blade_heart_count, game_state.player2.stage_hearts.as_ref());
        let player1_has_cards = !game_state.player1.live_card_zone.cards.is_empty();
        let player2_has_cards = !game_state.player2.live_card_zone.cards.is_empty();
        
        println!("DEBUG LiveVictoryDetermination: P1 has_cards={}, score={}, P2 has_cards={}, score={}", 
            player1_has_cards, player1_score, player2_has_cards, player2_score);
        
        let mut player1_won = false;
        let mut player2_won = false;
        
        // Rule 8.4.6.1: If both players have no cards, no one wins
        if !player1_has_cards && !player2_has_cards {
            // No winner, keep current first attacker
        }
        // Rule 8.4.6.2: Compare scores
        else if player1_has_cards && !player2_has_cards {
            player1_won = true;
        } else if !player1_has_cards && player2_has_cards {
            player2_won = true;
        } else {
            // Both have cards, compare scores
            if player1_score > player2_score {
                player1_won = true;
            } else if player2_score > player1_score {
                player2_won = true;
            }
        }
        
        // Rule 8.4.7: Move winning live card to success zone
        // First, send cards with "cannot_place" restriction straight to discard
        let card_db = game_state.card_database.clone();
        Self::move_restricted_cards_to_discard(&mut game_state.player1, &card_db);
        Self::move_restricted_cards_to_discard(&mut game_state.player2, &card_db);
        
        Self::move_live_to_success_and_handle_wins(game_state, player1_won, player2_won);
    }

    fn move_restricted_cards_to_discard(player: &mut crate::player::Player, card_db: &CardDatabase) {
        // Check each card in live_card_zone for "cannot_place" restriction
        let mut cards_to_remove = Vec::new();
        
        for (index, card_id) in player.live_card_zone.cards.iter().enumerate() {
            // Check if this specific card has the restriction by looking at its abilities
            if let Some(card) = card_db.get_card(*card_id) {
                let has_restriction = card.abilities.iter().any(|ability| {
                    if let Some(ref effect) = ability.effect {
                        effect.action == "restriction" 
                            && effect.restriction_type.as_deref() == Some("cannot_place")
                            && (effect.restricted_destination.as_deref() == Some("success_live_zone")
                                || effect.restricted_destination.as_deref() == Some("live_card_zone"))
                    } else {
                        false
                    }
                });
                
                if has_restriction {
                    cards_to_remove.push(index);
                    eprintln!("Card {} ({}) has cannot_place restriction for success_live_zone - will send to discard", 
                        card_id, card.card_no);
                }
            }
        }
        
        // Remove restricted cards in reverse order to maintain indices
        for index in cards_to_remove.into_iter().rev() {
            let card = player.live_card_zone.cards.remove(index);
            player.waitroom.cards.push(card);
        }
    }

    fn move_live_to_success(player: &mut crate::player::Player, card_index: usize, card_db: &CardDatabase) {
        // Move specified card from live card zone to success live card zone
        if card_index < player.live_card_zone.cards.len() {
            let card = player.live_card_zone.cards.remove(card_index);
            
            // Check if card has constant ability restriction preventing placement in success live zone
            let can_place = if let Some(card_data) = card_db.get_card(card) {
                !card_data.abilities.iter().any(|ability| {
                    if let Some(ref effect) = ability.effect {
                        effect.action == "restriction" 
                            && ability.triggers.as_ref().map_or(false, |t| t == "constant")
                            && effect.restriction_type.as_deref() == Some("cannot_place")
                            && (effect.restricted_destination.as_deref() == Some("success_live_zone")
                                || effect.restricted_destination.as_deref() == Some("live_card_zone"))
                    } else {
                        false
                    }
                })
            } else {
                true
            };
            
            if can_place {
                player.success_live_card_zone.cards.push(card);
            } else {
                // Send to discard instead due to restriction
                player.waitroom.cards.push(card);
                if let Some(card_data) = card_db.get_card(card) {
                    eprintln!("Card {} cannot be placed in success live zone due to constant restriction, sent to discard", card_data.card_no);
                }
            }
        }
    }

    fn move_live_to_success_and_handle_wins(game_state: &mut GameState, player1_won: bool, player2_won: bool) {
        let card_db = game_state.card_database.clone();
        
        if player1_won && player2_won {
            // Both won - check if either has 2 cards
            if game_state.player1.live_card_zone.cards.len() == 2 {
                // Player1 has 2 cards, doesn't move
            } else {
                let card_index = crate::bot::ai::AIPlayer::choose_live_card_for_success(&game_state.player1);
                Self::move_live_to_success(&mut game_state.player1, card_index, &card_db);
            }
            if game_state.player2.live_card_zone.cards.len() == 2 {
                // Player2 has 2 cards, doesn't move
            } else {
                let card_index = crate::bot::ai::AIPlayer::choose_live_card_for_success(&game_state.player2);
                Self::move_live_to_success(&mut game_state.player2, card_index, &card_db);
            }
        } else if player1_won {
            let card_index = crate::bot::ai::AIPlayer::choose_live_card_for_success(&game_state.player1);
            Self::move_live_to_success(&mut game_state.player1, card_index, &card_db);
        } else if player2_won {
            let card_index = crate::bot::ai::AIPlayer::choose_live_card_for_success(&game_state.player2);
            Self::move_live_to_success(&mut game_state.player2, card_index, &card_db);
        }
        
        // Rule 8.4.8: Move remaining live cards and cheer cards to discard
        for card in game_state.player1.live_card_zone.clear() {
            game_state.player1.waitroom.cards.push(card);
        }
        for card in game_state.player2.live_card_zone.clear() {
            game_state.player2.waitroom.cards.push(card);
        }
        
        // Rule 8.4.9-8.4.12: Turn end trigger loop
        loop {
            // Rule 8.4.9: Check timing
            Self::check_timing(game_state);
            
            // Rule 8.4.10: Trigger 'turn end' abilities that haven't triggered yet
            // Track abilities triggered this turn to prevent re-triggering
            let abilities_before = game_state.pending_auto_abilities.len();
            
            // Rule 8.4.11: Check timing again
            Self::check_timing(game_state);
            
            // Rule 8.4.11: End 'until end of turn' and 'during this turn' effects
            game_state.check_expired_effects();
            
            // Rule 8.4.12: Loop back to 8.4.9 if new abilities triggered
            let abilities_after = game_state.pending_auto_abilities.len();
            if abilities_after > abilities_before {
                // New abilities triggered, loop back
                continue;
            }
            
            break;
        }
        
        // Rule 8.4.13: Winner becomes first attacker next turn
        if player1_won && !player2_won {
            game_state.player1.is_first_attacker = true;
            game_state.player2.is_first_attacker = false;
        } else if player2_won && !player1_won {
            game_state.player1.is_first_attacker = false;
            game_state.player2.is_first_attacker = true;
        }
        
        game_state.player1.areas_locked_this_turn.clear();
        game_state.player2.areas_locked_this_turn.clear();
        game_state.turn_limited_abilities_used.clear();
        game_state.clear_card_movement_tracking();
        game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
        game_state.current_phase = Phase::Active;
    }

    pub fn check_timing(game_state: &mut GameState) {
        // Rule 9.5: Check timing - process rule processing per rules 10.2-10.6
        
        // Rule 10.2: Refresh (already handled in player.refresh())
        game_state.player1.refresh();
        game_state.player2.refresh();
        
        // Rule 10.3: Victory processing - check for 3+ successful live cards
        Self::check_victory_condition(game_state);
        
        
        // Rule 10.5: Check for invalid cards
        Self::check_invalid_cards(&mut game_state.player1, &game_state.card_database);
        Self::check_invalid_cards(&mut game_state.player2, &game_state.card_database);
        
        // Rule 10.6: Check for invalid resolution zone
        Self::check_invalid_resolution_zone(game_state);

        // Rule 12.1: Check for permanent loop
        if game_state.check_permanent_loop() {
            // Rule 12.1.1.3: If permanent loop detected and neither player can stop it, game ends in draw
            game_state.game_result = crate::game_state::GameResult::Draw;
            game_state.game_ended = true;
        }

        // Check for victory condition
        Self::check_victory_condition(game_state);

        // Rule 9.5.1: After rule processing, play and resolve automatic abilities
        let active_player_id = game_state.active_player().id.clone();
        game_state.process_pending_auto_abilities(&active_player_id);
    }

    pub fn check_victory_condition(game_state: &mut GameState) {
        // Rule 10.3.1: If a player has VICTORY_CARD_COUNT+ cards in success live card zone, they win
        let p1_success_count = game_state.player1.success_live_card_zone.cards.len();
        let p2_success_count = game_state.player2.success_live_card_zone.cards.len();

        if p1_success_count >= VICTORY_CARD_COUNT {
            game_state.game_result = crate::game_state::GameResult::FirstAttackerWins;
        } else if p2_success_count >= VICTORY_CARD_COUNT {
            game_state.game_result = crate::game_state::GameResult::SecondAttackerWins;
        }

        // Rule 1.2.1.2: If both players have VICTORY_CARD_COUNT+ cards simultaneously, it's a draw
        if p1_success_count >= VICTORY_CARD_COUNT && p2_success_count >= VICTORY_CARD_COUNT {
            game_state.game_result = crate::game_state::GameResult::Draw;
        }
    }

    fn check_invalid_cards(player: &mut crate::player::Player, card_db: &CardDatabase) {
        // Rule 10.5: Check for invalid cards in zones
        // Rule 10.5.1: Non-live cards in live card zone
        let invalid_live_cards: Vec<i16> = player.live_card_zone.cards.iter()
            .filter(|card_id| !card_db.get_card(**card_id).map(|c| c.is_live()).unwrap_or(false))
            .copied()
            .collect();

        // Rule 10.5.4: Energy cards go to energy deck instead of discard
        for card_id in invalid_live_cards {
            player.live_card_zone.cards.retain(|c| *c != card_id);
            if card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false) {
                player.energy_deck.cards.push(card_id);
            } else {
                player.waitroom.cards.push(card_id);
            }
        }

        // Rule 10.5.2: Non-energy cards in energy zone
        let invalid_energy_cards: Vec<i16> = player.energy_zone.cards.iter()
            .filter(|card_id| !card_db.get_card(**card_id).map(|c| c.is_energy()).unwrap_or(false))
            .copied()
            .collect();

        for card_id in invalid_energy_cards {
            player.energy_zone.cards.retain(|c| *c != card_id);
            if card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false) {
                player.energy_deck.cards.push(card_id);
            } else {
                player.waitroom.cards.push(card_id);
            }
        }

        // Rule 10.5.3: Energy cards without member above in member area
        // Check each member area - cache stage card lookups
        let stage_card_ids = [
            player.stage.stage[0],
            player.stage.stage[1],
            player.stage.stage[2],
        ];

        for (index, card_id) in stage_card_ids.iter().enumerate() {
            if *card_id != -1 {
                // If no member above, move energy cards to energy deck
                if card_db.get_card(*card_id).map(|c| c.is_energy()).unwrap_or(false) {
                    player.stage.stage[index] = -1;
                    // Rule 10.5.4: Move to energy deck instead of discard
                    player.energy_deck.cards.push(*card_id);
                }
            }
        }
    }

    fn check_invalid_resolution_zone(game_state: &mut GameState) {
        // Rule 10.6: Invalid resolution zone processing
        // Rule 10.6.1: Cards in resolution zone that are not being played/resolved/cheered go to discard
        // The resolution zone is used for:
        // - Cards being played (temporary during play)
        // - Cards being resolved (during ability resolution)
        // - Cards revealed during cheer (8.3.11)

        // After cheer processing is complete (cheer_check_completed), move all cards to discard
        if game_state.cheer_check_completed && !game_state.resolution_zone.cards.is_empty() {
            // Move all resolution zone cards to player1's waitroom (cheer is done by turn player)
            for card_id in game_state.resolution_zone.cards.drain(..) {
                game_state.player1.waitroom.cards.push(card_id);
            }
        }
    }

    pub fn player_set_live_cards(player: &mut crate::player::Player, num_cards_to_set: usize, card_database: &crate::card::CardDatabase) {
        // Rule 8.2: Player sets live cards face-down and draws equal amount
        let live_cards: Vec<i16> = player.hand.cards.iter()
            .filter(|&c| {
                if let Some(card) = card_database.get_card(*c) {
                    card.is_live()
                } else {
                    false
                }
            })
            .copied()
            .collect();

        if live_cards.is_empty() || num_cards_to_set == 0 {
            return;
        }

        // Set specified number of cards
        let cards_to_set = std::cmp::min(num_cards_to_set, live_cards.len());
        for i in 0..cards_to_set {
            let card_id = live_cards[i];
            // Remove from hand
            if let Some(pos) = player.hand.cards.iter().position(|&c| c == card_id) {
                player.hand.cards.remove(pos);
            }
            // Add to live card zone face-down
            let _ = player.live_card_zone.add_card(card_id, true, card_database);
        }
        
        // Draw equal amount
        for _ in 0..cards_to_set {
            let _ = player.draw_card();
        }
    }

    pub fn player_perform_live(player: &mut crate::player::Player, resolution_zone: &mut crate::zones::ResolutionZone, _player_id: &str, card_database: &crate::card::CardDatabase) -> u32 {
        // Rule 8.3: Player performs live - check heart requirements
        // Note: This function no longer takes game_state to avoid borrow conflicts
        // Ability triggering should be handled by the caller

        // Rule 8.3.4: Reveal cards, discard non-live cards
        player.live_card_zone.cards.retain(|c| {
            if let Some(card) = card_database.get_card(*c) {
                card.is_live()
            } else {
                false
            }
        });
        
        // Rule 8.3.4.1: If player is 'cannot live' state, discard all revealed cards
        // Note: This check should be done by the caller before calling this function
        // For now, we'll skip this check here
        
        // Rule 8.3.6: If no live cards, end performance
        if player.live_card_zone.cards.is_empty() {
            return 0;
        }
        
        // Rule 8.3.7: Live cards exist, perform live
        
        // Rule 8.3.10: Calculate total blades from active members
        let total_blades = player.stage.total_blades(card_database);
        
        // Rule 8.3.11: Cheer - move cards from main deck to resolution zone
        for _ in 0..total_blades {
            if !player.main_deck.cards.is_empty() {
                let card_id = player.main_deck.cards.remove(0);
                resolution_zone.cards.push(card_id);
            }
        }
        
        // Rule 8.3.12: Check blade hearts on cards in resolution zone
        let mut blade_heart_count = 0;
        let mut special_heart_draw_count = 0;
        let mut special_heart_score_count = 0;
        let mut b_all_count = 0;
        
        for card_id in &resolution_zone.cards {
            if let Some(card) = card_database.get_card(*card_id) {
                if let Some(ref blade_heart) = card.blade_heart {
                    // Count b_all separately for wildcard treatment
                    b_all_count += blade_heart.hearts.get(&crate::card::HeartColor::BAll).copied().unwrap_or(0);
                    // Count regular blade hearts (b_heart01, etc.) for drawing
                    for (color, count) in &blade_heart.hearts {
                        if *color != crate::card::HeartColor::BAll {
                            blade_heart_count += count;
                        }
                    }
                }
                // Rule: Handle special_heart types (draw and score)
                if let Some(ref special_heart) = card.special_heart {
                    special_heart_draw_count += special_heart.hearts.get(&crate::card::HeartColor::Draw).copied().unwrap_or(0);
                    special_heart_score_count += special_heart.hearts.get(&crate::card::HeartColor::Score).copied().unwrap_or(0);
                }
            }
        }

        // Rule 8.3.12.1: Draw cards based on blade heart count (excluding b_all)
        for _ in 0..blade_heart_count {
            let _ = player.draw_card();
        }
        
        // Rule: Draw additional cards based on special_heart draw count
        for _ in 0..special_heart_draw_count {
            let _ = player.draw_card();
        }
        
        // Rule 8.3.13: Check timing (caller responsibility)
        
        // Rule 8.3.14: Calculate live-owned hearts from stage and blade hearts
        let stage_hearts = player.stage.get_available_hearts(&card_database);
        let mut live_owned_hearts = stage_hearts.clone();
        
        // Add blade hearts from resolution zone (excluding b_all which is handled as wildcard)
        for card_id in &resolution_zone.cards {
            if let Some(card) = card_database.get_card(*card_id) {
                if let Some(ref blade_heart) = card.blade_heart {
                    for (color, count) in &blade_heart.hearts {
                        if *color != crate::card::HeartColor::BAll {
                            *live_owned_hearts.hearts.entry(color.clone()).or_insert(0) += count;
                        }
                    }
                }
            }
        }
        
        // Add b_all as wildcard hearts (can be any color, stored as BAll key)
        if b_all_count > 0 {
            *live_owned_hearts.hearts.entry(crate::card::HeartColor::BAll).or_insert(0) += b_all_count;
        }
        
        // Rule 8.3.15: Check if each live card can satisfy required hearts
        let mut remaining_hearts = live_owned_hearts.clone();
        let mut live_cards_to_remove = Vec::new();
        
        for card_id in &player.live_card_zone.cards {
            if let Some(card) = card_database.get_card(*card_id) {
                if let Some(ref need_heart) = card.need_heart {
                    let mut can_satisfy = true;
                    let mut temp_hearts = remaining_hearts.hearts.clone();

                    for (color, needed) in &need_heart.hearts {
                        if *color == crate::card::HeartColor::Heart00 {
                            // Wildcard heart (rule 8.3.15.1.1) - can be any color
                            // Count total hearts available (including b_all wildcards)
                            let total_available: u32 = temp_hearts.values().sum();
                            if total_available < *needed {
                                can_satisfy = false;
                                break;
                            }
                            // Consume from any colors (prefer non-wildcards first)
                            let mut _consumed = 0;
                            for (c, count) in temp_hearts.iter_mut() {
                                if *c != crate::card::HeartColor::Heart00 {
                                    let to_consume = std::cmp::min(*count, *needed - _consumed);
                                    *count -= to_consume;
                                    _consumed += to_consume;
                                    if _consumed >= *needed {
                                        break;
                                    }
                                }
                            }
                            // If still need more, consume from wildcards (heart00)
                            if _consumed < *needed {
                                if let Some(wildcard_count) = temp_hearts.get_mut(&crate::card::HeartColor::Heart00) {
                                    let to_consume = std::cmp::min(*wildcard_count, *needed - _consumed);
                                    *wildcard_count -= to_consume;
                                    _consumed += to_consume;
                                }
                            }
                        } else {
                            // Specific color heart - can use heart00 as wildcard
                            let specific_available = temp_hearts.get(color).unwrap_or(&0);
                            let wildcard_available = temp_hearts.get(&crate::card::HeartColor::Heart00).unwrap_or(&0);
                            let total_available = specific_available + wildcard_available;

                            if total_available < *needed {
                                can_satisfy = false;
                                break;
                            }

                            // Use specific color first, then b_all
                            let specific_to_consume = std::cmp::min(*specific_available, *needed);
                            if let Some(heart_count) = temp_hearts.get_mut(color) {
                                *heart_count -= specific_to_consume;
                            }
                            let remaining_needed = *needed - specific_to_consume;

                            if remaining_needed > 0 {
                                if let Some(wildcard_count) = temp_hearts.get_mut(&crate::card::HeartColor::Heart00) {
                                    let to_consume = std::cmp::min(*wildcard_count, remaining_needed);
                                    *wildcard_count -= to_consume;
                                }
                            }
                        }
                    }

                    if can_satisfy {
                        // Update remaining hearts with the temp consumption
                        remaining_hearts.hearts = temp_hearts;
                    } else {
                        live_cards_to_remove.push(*card_id);
                    }
                }
            }
        }
        
        // Rule 8.3.16: If any fails, all live cards go to discard
        if !live_cards_to_remove.is_empty() {
            // Move all live cards to discard
            for card_id in player.live_card_zone.clear() {
                player.waitroom.cards.push(card_id);
            }
        }
        
        // Move resolution zone cards to discard
        for card_id in resolution_zone.cards.drain(..) {
            player.waitroom.cards.push(card_id);
        }
        
        // Rule 8.3.17: Check timing (caller responsibility)
        
        // Return total heart count for victory determination (blade hearts + b_all + special_heart score)
        blade_heart_count + b_all_count + special_heart_score_count
    }

    /// Trigger debut abilities for a player when a card is placed on stage
    fn trigger_debut_abilities(game_state: &mut GameState, player_id: &str, card_no: &str, cost_paid: u32, baton_touch_used: bool) {
        // Rule 11.4: Trigger Debut automatic abilities
        // Rule 11.4.2: Trigger when member is placed on stage
        
        // Q197/Q198: Auto abilities don't trigger when played via baton touch with cost 10+
        if baton_touch_used && cost_paid >= 10 {
            return;
        }
        
        let player_id_clone = player_id.to_string();
        let card_no_clone = card_no.to_string();
        
        // Collect abilities to trigger first to avoid borrow conflicts
        let mut abilities_to_trigger = Vec::new();
        
        // Get the played card's cost for Q229 condition check
        let _played_card_cost = {
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            
            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            let mut found_cost = None;
            for area in areas {
                if let Some(card_id) = player.stage.get_area(area) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        if card.card_no == card_no_clone {
                            found_cost = Some(card.cost);
                            break;
                        }
                    }
                }
            }
            found_cost
        };
        
        {
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            
            // Find the card on stage
            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            for area in areas {
                if let Some(card_id) = player.stage.get_area(area) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        if card.card_no == card_no_clone {
                            // Check if card has Debut abilities
                            for ability in &card.abilities {
                                // Check if ability has Debut trigger (check both English and Japanese)
                                if ability.triggers.as_ref().map_or(false, |t| t == "Debut") {
                                    // Q229: Check if ability requires baton touch from lower-cost member
                                    // The ability text contains "debut via baton touch from lower-cost member"
                                    let requires_baton_touch = ability.full_text.contains("baton touch") && ability.full_text.contains("debut");
                                    
                                    if requires_baton_touch {
                                        // Only trigger if played via baton touch
                                        if !baton_touch_used {
                                            continue;
                                        }
                                        // Check if replaced member had lower cost (simplified - assumes baton touch was valid)
                                        // In a full implementation, we'd track the replaced member's cost
                                    }
                                    
                                    let ability_id = format!("{}_{}", card_no_clone, ability.full_text);
                                    abilities_to_trigger.push((ability_id, card_no_clone.clone()));
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
        
        // Trigger collected abilities
        for (ability_id, card_no) in abilities_to_trigger {
            game_state.trigger_auto_ability(
                ability_id,
                crate::game_state::AbilityTrigger::Debut,
                player_id_clone.clone(),
                Some(card_no),
            );
        }
    }

    #[allow(dead_code)]
    fn trigger_performance_phase_start_abilities(game_state: &mut GameState, player_id: &str) {
        // Rule 8.3.3: Trigger 'performance phase start' automatic abilities
        
        let player_id_clone = player_id.to_string();
        
        // Collect abilities to trigger first to avoid borrow conflicts
        let mut abilities_to_trigger = Vec::new();
        
        {
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            
            // Check all members on stage for performance phase start abilities
            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            for area in areas {
                if let Some(card_id) = player.stage.get_area(area) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        // Check if card has performance phase start abilities
                        for ability in &card.abilities {
                            // Check if ability has performance phase start trigger
                            if ability.triggers.as_ref().map_or(false, |t| t == "performance_phase_start") {
                                // Collect ability to trigger
                                let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                                abilities_to_trigger.push((ability_id, card.card_no.clone()));
                            }
                        }
                    }
                }
            }
        }
        
        // Trigger collected abilities
        for (ability_id, card_no) in abilities_to_trigger {
            game_state.trigger_auto_ability(
                ability_id,
                crate::game_state::AbilityTrigger::PerformancePhaseStart,
                player_id_clone.clone(),
                Some(card_no),
            );
        }
    }

    #[allow(dead_code)]
    fn trigger_live_start_abilities(game_state: &mut GameState, player_id: &str) {
        // Rule 11.5: Trigger LiveStart automatic abilities
        // Rule 11.5.2: Trigger when live card is set
        
        let player_id_clone = player_id.to_string();
        
        // Collect abilities to trigger first to avoid borrow conflicts
        let mut abilities_to_trigger = Vec::new();
        
        {
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            
            // Check all members on stage for LiveStart abilities
            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            for area in areas {
                if let Some(card_id) = player.stage.get_area(area) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        // Check if card has LiveStart abilities
                        for ability in &card.abilities {
                            // Check if ability has LiveStart trigger
                            if ability.triggers.as_ref().map_or(false, |t| t == "LiveStart") {
                                // Collect ability to trigger
                                let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                                abilities_to_trigger.push((ability_id, card.card_no.clone()));
                            }
                        }
                    }
                }
            }
        }
        
        // Trigger collected abilities
        for (ability_id, card_no) in abilities_to_trigger {
            game_state.trigger_auto_ability(
                ability_id,
                crate::game_state::AbilityTrigger::LiveStart,
                player_id_clone.clone(),
                Some(card_no),
            );
        }
    }

    /// Trigger live start abilities for a specific live card when it is set
    fn trigger_live_start_abilities_for_card(game_state: &mut GameState, player_id: &str, card_no: &str) {
        // Rule 11.5: Trigger LiveStart automatic abilities for live cards
        // Live cards can have live start abilities that trigger when the live card is set
        
        let player_id_clone = player_id.to_string();
        let card_no_clone = card_no.to_string();
        
        // Collect abilities to trigger first to avoid borrow conflicts
        let mut abilities_to_trigger = Vec::new();
        
        {
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            
            // Check the live card zone for the set live card
            for card_id in &player.live_card_zone.cards {
                if let Some(card) = game_state.card_database.get_card(*card_id) {
                    if card.card_no == card_no_clone {
                        // Check if card has LiveStart abilities
                        for ability in &card.abilities {
                            // Check if ability has LiveStart trigger
                            if ability.triggers.as_ref().map_or(false, |t| t == "LiveStart") {
                                // Collect ability to trigger
                                let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                                abilities_to_trigger.push((ability_id, card.card_no.clone()));
                            }
                        }
                    }
                }
            }
        }
        
        // Trigger collected abilities
        for (ability_id, card_no) in abilities_to_trigger {
            game_state.trigger_auto_ability(
                ability_id,
                crate::game_state::AbilityTrigger::LiveStart,
                player_id_clone.clone(),
                Some(card_no),
            );
        }
    }

    #[allow(dead_code)]
    fn trigger_live_success_abilities(game_state: &mut GameState, player_id: &str) {
        // Rule 11.6: Trigger LiveSuccess automatic abilities
        // Rule 11.6.2: Trigger when live succeeds
        
        let player_id_clone = player_id.to_string();
        
        // Collect abilities to trigger first to avoid borrow conflicts
        let mut abilities_to_trigger = Vec::new();
        
        {
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            
            // Check all members on stage for LiveSuccess abilities
            let area_indices = [(crate::zones::MemberArea::LeftSide, 0), (crate::zones::MemberArea::Center, 1), (crate::zones::MemberArea::RightSide, 2)];
            for (_area, index) in area_indices {
                let card_id = player.stage.stage[index];
                if card_id != -1 {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        // Check if card has LiveSuccess abilities
                        for ability in &card.abilities {
                            // Check if ability has LiveSuccess trigger
                            if ability.triggers.as_ref().map_or(false, |t| t == "LiveSuccess") {
                                // Collect ability to trigger
                                let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                                abilities_to_trigger.push((ability_id, card.card_no.clone()));
                            }
                        }
                    }
                }
            }

            // Also check live cards in live card zone
            for card_id in &player.live_card_zone.cards {
                if let Some(card) = game_state.card_database.get_card(*card_id) {
                    for ability in &card.abilities {
                        if ability.full_text.contains("LiveSuccess") {
                            let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                            abilities_to_trigger.push((ability_id, card.card_no.clone()));
                        }
                    }
                }
            }
        }
        
        // Trigger collected abilities
        for (ability_id, card_no) in abilities_to_trigger {
            game_state.trigger_auto_ability(
                ability_id,
                crate::game_state::AbilityTrigger::LiveSuccess,
                player_id_clone.clone(),
                Some(card_no),
            );
        }
    }
}
