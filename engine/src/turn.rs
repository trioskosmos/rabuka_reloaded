use crate::game_state::{GameState, Phase, TurnPhase};
use crate::card::CardDatabase;
use std::vec::Vec;
use std::string::String;

pub struct TurnEngine;

impl TurnEngine {
    pub fn advance_phase(game_state: &mut GameState) {
        // Advance phase according to rules 7.1.2, 7.3.3, and 8.1.2
        let current_phase = game_state.current_phase.clone();
        let current_turn_phase = game_state.current_turn_phase.clone();
        eprintln!("advance_phase called: current_phase={:?}, current_turn_phase={:?}", current_phase, current_turn_phase);
        
        // Handle normal phase sub-phases (Rule 7.3.3)
        if current_turn_phase == TurnPhase::FirstAttackerNormal || current_turn_phase == TurnPhase::SecondAttackerNormal {
            match current_phase {
                Phase::Active => {
                    // Rule 7.4: Activate all energy and stage cards (automatic)
                    // Activate BOTH players' energy, not just active player
                    game_state.player1.activate_all_energy();
                    game_state.player2.activate_all_energy();
                    eprintln!("After Active phase: p1 active={}, p2 active={}", game_state.player1.energy_zone.active_count(), game_state.player2.energy_zone.active_count());
                    // Orientation tracking moved to GameState modifiers
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Energy;
                }
                Phase::Energy => {
                    // Rule 7.5: Draw energy card (automatic)
                    Self::check_timing(game_state);
                    let _ = game_state.active_player_mut().draw_energy();
                    eprintln!("After Energy phase: p1 active={}, p2 active={}", game_state.player1.energy_zone.active_count(), game_state.player2.energy_zone.active_count());
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Draw;
                }
                Phase::Draw => {
                    // Rule 7.6: Draw card (automatic)
                    Self::check_timing(game_state);
                    let _ = game_state.active_player_mut().draw_card();
                    eprintln!("After Draw phase: p1 active={}, p2 active={}", game_state.player1.energy_zone.active_count(), game_state.player2.energy_zone.active_count());
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Main;
                }
                Phase::Main => {
                    // Rule 7.7: Main phase complete, advance to next turn phase
                    Self::check_timing(game_state);
                    if current_turn_phase == TurnPhase::FirstAttackerNormal {
                        game_state.current_turn_phase = TurnPhase::SecondAttackerNormal;
                        game_state.current_phase = Phase::Active;
                    } else {
                        game_state.current_turn_phase = TurnPhase::Live;
                        game_state.current_phase = Phase::LiveCardSet;
                        // Reset live card set flags for new live phase
                        game_state.live_card_set_player1_done = false;
                        game_state.live_card_set_player2_done = false;
                    }
                }
                _ => {}
            }
        }
        // Handle live phase sub-phases (Rule 8.1.2)
        else if current_turn_phase == TurnPhase::Live {
            match current_phase {
                Phase::LiveCardSet => {
                    // Rule 8.2: Both players set live cards - manual phase, not auto-advanced
                    // Players must manually choose live cards via actions
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
                }
                _ => {}
            }
        }
    }

    pub fn execute_main_phase_action(game_state: &mut GameState, action: &crate::game_setup::ActionType, card_id: Option<i16>, _card_indices: Option<Vec<usize>>, stage_area: Option<crate::zones::MemberArea>, use_baton_touch: Option<bool>) -> Result<(), String> {
        // Execute player choice action during various phases
        match action {
            crate::game_setup::ActionType::MulliganHeader => {
                // MulliganHeader is a display-only action, no execution needed
                Ok(())
            }
            crate::game_setup::ActionType::RpsChoice => {
                // Rule 6.2.2: Rock Paper Scissors to determine who chooses to go first
                // For simplicity, player 1 always chooses, and we randomly determine player 2's choice
                use rand::Rng;
                let mut rng = rand::thread_rng();
                
                let p1_choice = match stage_area {
                    Some(crate::zones::MemberArea::LeftSide) => 0,
                    Some(crate::zones::MemberArea::Center) => 1,
                    Some(crate::zones::MemberArea::RightSide) => 2,
                    _ => 0,
                };
                
                let p2_choice = rng.gen_range(0..3);
                
                // Determine winner
                let rps_winner = match (p1_choice, p2_choice) {
                    (0, 2) | (1, 0) | (2, 1) => 1, // Player 1 wins
                    (2, 0) | (0, 1) | (1, 2) => 2, // Player 2 wins
                    _ => {
                        // Tie - play again (simplified: player 1 wins on tie)
                        1
                    }
                };
                
                // Set first attacker
                if rps_winner == 1 {
                    game_state.player1.is_first_attacker = true;
                    game_state.player2.is_first_attacker = false;
                } else {
                    game_state.player1.is_first_attacker = false;
                    game_state.player2.is_first_attacker = true;
                }
                
                // Rule 6.2.5: Initial draw - Each player draws 6 cards from main deck to hand
                for _ in 0..6 {
                    game_state.player1.draw_card();
                    game_state.player2.draw_card();
                }
                
                // Advance to Mulligan phase
                game_state.current_phase = crate::game_state::Phase::Mulligan;
                // Initialize mulligan state - first attacker goes first
                game_state.current_mulligan_player = if game_state.player1.is_first_attacker {
                    "player1".to_string()
                } else {
                    "player2".to_string()
                };
                game_state.mulligan_selected_indices.clear();
                Ok(())
            }
            crate::game_setup::ActionType::SelectMulligan => {
                // Toggle card selection for mulligan
                let idx = if let Some(cid) = card_id {
                    let active_player = game_state.active_player();
                    active_player.get_card_index_by_id(cid).unwrap_or(0)
                } else {
                    0 // Fallback to index 0 if no card_id provided
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
            crate::game_setup::ActionType::ConfirmMulligan => {
                // Rule 6.2.1.6: Mulligan - player has selected cards to mulligan
                // Use the tracked indices from game state
                let indices = game_state.mulligan_selected_indices.clone();
                
                // Determine which player is mulliganing based on current_mulligan_player
                let current_player = if game_state.current_mulligan_player == "player1" {
                    &mut game_state.player1
                } else {
                    &mut game_state.player2
                };
                
                // Mark this player as done
                if game_state.current_mulligan_player == "player1" {
                    game_state.mulligan_player1_done = true;
                } else {
                    game_state.mulligan_player2_done = true;
                }
                
                // Perform mulligan for selected cards
                if !indices.is_empty() {
                    // Sort indices in descending order to remove without shifting
                    let mut sorted_indices = indices.clone();
                    sorted_indices.sort_by(|a, b| b.cmp(a));
                    
                    let num_to_mulligan = sorted_indices.len();
                    let mut cards_to_set_aside = Vec::new();
                    
                    for idx in sorted_indices {
                        if idx < current_player.hand.cards.len() {
                            cards_to_set_aside.push(current_player.hand.cards.remove(idx));
                        }
                    }
                    
                    // Draw the same number from main deck
                    for _ in 0..num_to_mulligan {
                        let _ = current_player.draw_card();
                    }
                    
                    // Move set-aside cards to main deck
                    for card in cards_to_set_aside {
                        current_player.main_deck.cards.push(card);
                    }
                    
                    // Shuffle main deck
                    use rand::seq::SliceRandom;
                    let mut deck_vec: Vec<_> = current_player.main_deck.cards.drain(..).collect();
                    deck_vec.shuffle(&mut rand::thread_rng());
                    for card in deck_vec {
                        current_player.main_deck.cards.push(card);
                    }
                }
                
                // Clear selected indices for next player
                game_state.mulligan_selected_indices.clear();
                
                // Check if both players have mulliganed
                if game_state.mulligan_player1_done && game_state.mulligan_player2_done {
                    // Both done, advance to energy setup
                    Self::setup_initial_energy(game_state);
                    game_state.current_phase = crate::game_state::Phase::Active;
                } else {
                    // Switch to other player
                    game_state.current_mulligan_player = if game_state.current_mulligan_player == "player1" {
                        "player2".to_string()
                    } else {
                        "player1".to_string()
                    };
                }
                Ok(())
            }
            crate::game_setup::ActionType::SkipMulligan => {
                // Player chooses not to mulligan
                // Mark this player as done
                if game_state.current_mulligan_player == "player1" {
                    game_state.mulligan_player1_done = true;
                } else {
                    game_state.mulligan_player2_done = true;
                }
                
                // Clear selected indices
                game_state.mulligan_selected_indices.clear();
                
                // Check if both players have mulliganed (or skipped)
                if game_state.mulligan_player1_done && game_state.mulligan_player2_done {
                    // Both done, advance to energy setup
                    Self::setup_initial_energy(game_state);
                    game_state.current_phase = crate::game_state::Phase::Active;
                } else {
                    // Switch to other player
                    game_state.current_mulligan_player = if game_state.current_mulligan_player == "player1" {
                        "player2".to_string()
                    } else {
                        "player1".to_string()
                    };
                }
                Ok(())
            }
            crate::game_setup::ActionType::PlayMemberToStage => {
                // Get turn number and card_database before any mutable borrows
                // Clone necessary due to borrow checker - card_db used after active_player_mut()
                let _current_turn = game_state.turn_number;
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
                    // Find first available stage area
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

                // Get card info before moving it
                let card_id = player.hand.cards[idx];
                let card_no = card_db.get_card(card_id).map(|c| c.card_no.clone()).unwrap_or_default();
                let player_id = player.id.clone();

                // Check if baton touch is requested (from parameters)
                let use_baton_touch = use_baton_touch.unwrap_or(false);

                let (cost_paid, baton_touch_used) = player.move_card_from_hand_to_stage(idx, area, use_baton_touch, &card_db)?;

                // turn_played tracking moved to GameState modifiers
                // For now, this is a no-op

                // Trigger 登場 abilities for the played card
                // Q197/Q198: Auto abilities don't trigger when played via baton touch with cost 10+
                Self::trigger_debut_abilities(game_state, &player_id, &card_no, cost_paid);
                
                // Trigger baton touch event if baton touch was used
                if baton_touch_used {
                    // Rule 9.6.2.3.2.1: Baton touch event triggers when baton touch is performed
                    // This can trigger abilities with "baton touch" trigger
                    let player_id = game_state.active_player().id.clone();
                    // Find the card that was played and trigger baton touch abilities
                    // For now, trigger abilities on all stage members with baton touch trigger
                    let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
                    for area in areas {
                        let card_no = if let Some(card_id) = game_state.active_player().stage.get_area(area) {
                            if let Some(card) = game_state.card_database.get_card(card_id) {
                                let abilities_to_trigger: Vec<(String, String)> = card.abilities.iter()
                                    .filter(|ability| ability.triggers.as_ref().map_or(false, |t| t == "バトンタッチ" || t == "baton_touch"))
                                    .map(|ability| (format!("{}_{}", card.card_no, ability.full_text), card.card_no.clone()))
                                    .collect();
                                abilities_to_trigger
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        };

                        for (ability_id, card_no) in card_no {
                            game_state.trigger_auto_ability(
                                ability_id,
                                crate::game_state::AbilityTrigger::Debut, // Reuse Debut for now, or add BatonTouch trigger
                                player_id.clone(),
                                Some(card_no),
                            );
                        }
                    }
                }
                
                Ok(())
            }
            crate::game_setup::ActionType::SetLiveCard => {
                // Rule 8.2: Live Card Set Phase - Place individual card face-down, max 3 cards
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
                    if idx < player.hand.cards.len() && player.live_card_zone.cards.len() < 3 {
                        let card = player.hand.cards.remove(idx);
                        let _ = player.live_card_zone.add_card(card, true, &card_db);
                        player.rebuild_hand_index_map();
                        // Don't draw here - drawing happens when finishing live set phase
                    }
                    // Don't advance phase yet - allow placing more cards up to 3
                } else {
                    // No card selected, finish this player's live card set
                    // Use GameState flags to track completion
                    if game_state.current_turn_phase == crate::game_state::TurnPhase::Live && 
                       game_state.current_phase == crate::game_state::Phase::LiveCardSet {
                        
                        let p1_is_first = game_state.player1.is_first_attacker;
                        let p1_done = game_state.live_card_set_player1_done;
                        let p2_done = game_state.live_card_set_player2_done;
                        
                        // Mark current player as finished
                        if !p1_done && p2_done {
                            game_state.live_card_set_player1_done = true;
                        } else if !p2_done && p1_done {
                            game_state.live_card_set_player2_done = true;
                        } else if !p1_done && !p2_done {
                            if p1_is_first {
                                game_state.live_card_set_player1_done = true;
                            } else {
                                game_state.live_card_set_player2_done = true;
                            }
                        }
                        
                        // Check if both players have finished - if so, advance phase
                        if game_state.live_card_set_player1_done && game_state.live_card_set_player2_done {
                            // Both done, run check timing then advance
                            Self::check_timing(game_state);
                            Self::advance_phase(game_state);
                        }
                    } else {
                        Self::advance_phase(game_state);
                    }
                }
                Ok(())
            }
            crate::game_setup::ActionType::FinishLiveCardSet => {
                // Rule 8.2: Live Card Set Phase - Finish live card set and advance phase
                // Rule 8.2.5: After all processing in check timing is complete, live card set phase ends
                if game_state.current_turn_phase == crate::game_state::TurnPhase::Live && 
                   game_state.current_phase == crate::game_state::Phase::LiveCardSet {
                    
                    // Determine which player is currently taking their turn based on completion flags
                    // First attacker goes first, then second attacker
                    let p1_is_first = game_state.player1.is_first_attacker;
                    let p1_done = game_state.live_card_set_player1_done;
                    let p2_done = game_state.live_card_set_player2_done;
                    
                    // Draw cards equal to number of cards placed in live zone
                    let current_player = if !p1_done && p2_done {
                        // P1 is currently taking their turn (P2 already done)
                        &mut game_state.player1
                    } else if !p2_done && p1_done {
                        // P2 is currently taking their turn (P1 already done)
                        &mut game_state.player2
                    } else if !p1_done && !p2_done {
                        // Neither has finished yet - first attacker goes first
                        if p1_is_first {
                            &mut game_state.player1
                        } else {
                            &mut game_state.player2
                        }
                    } else {
                        // Both done - shouldn't happen
                        return Ok(());
                    };
                    
                    // Draw cards equal to number of cards placed in live zone
                    let cards_placed = current_player.live_card_zone.cards.len();
                    for _ in 0..cards_placed {
                        let _ = current_player.draw_card();
                    }
                    current_player.rebuild_hand_index_map();
                    
                    // Mark current player as having finished their live card set
                    if !p1_done && p2_done {
                        // P1 is currently taking their turn (P2 already done)
                        game_state.live_card_set_player1_done = true;
                    } else if !p2_done && p1_done {
                        // P2 is currently taking their turn (P1 already done)
                        game_state.live_card_set_player2_done = true;
                    } else if !p1_done && !p2_done {
                        // Neither has finished yet - first attacker goes first
                        if p1_is_first {
                            game_state.live_card_set_player1_done = true;
                        } else {
                            game_state.live_card_set_player2_done = true;
                        }
                    }
                    
                    // Check if both players have finished
                    let p1_finished = game_state.live_card_set_player1_done;
                    let p2_finished = game_state.live_card_set_player2_done;
                    
                    if p1_finished && p2_finished {
                        // Both players have finished, run check timing then advance to performance phase
                        Self::check_timing(game_state);
                        // Directly advance to FirstAttackerPerformance since advance_phase returns early for LiveCardSet
                        game_state.current_phase = crate::game_state::Phase::FirstAttackerPerformance;
                    }
                    // If only one player finished, the other will get their turn on next action generation
                } else {
                    // Not in Live Card Set phase, just advance
                    Self::advance_phase(game_state);
                }
                Ok(())
            }
            crate::game_setup::ActionType::Pass => {
                // Player passes, advance phase
                Self::advance_phase(game_state);
                Ok(())
            }
            _ => Err(format!("Unknown action: {}", action))
        }
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

    pub fn execute_live_victory_determination(game_state: &mut GameState) {
        // Rule 8.4: Determine live victory
        // Rule 8.4.2.1: Add cheer blade heart count to score
        let player1_score = game_state.player1.live_card_zone.calculate_live_score(&game_state.card_database, game_state.player1_cheer_blade_heart_count);
        let player2_score = game_state.player2.live_card_zone.calculate_live_score(&game_state.card_database, game_state.player2_cheer_blade_heart_count);
        let player1_has_cards = !game_state.player1.live_card_zone.cards.is_empty();
        let player2_has_cards = !game_state.player2.live_card_zone.cards.is_empty();
        
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
            } else {
                // Tie, both win
                player1_won = true;
                player2_won = true;
            }
        }
        
        // Rule 8.4.4: Live success event for players with cards
        let p1_id = game_state.player1.id.clone();
        let p2_id = game_state.player2.id.clone();
        
        if player1_has_cards {
            // Rule 11.6: Trigger LiveSuccess abilities for player1
            Self::trigger_live_success_abilities(game_state, &p1_id);
        }
        if player2_has_cards {
            // Rule 11.6: Trigger LiveSuccess abilities for player2
            Self::trigger_live_success_abilities(game_state, &p2_id);
        }
        
        // Rule 8.4.7: Move winning live card to success zone
        if player1_won && player2_won {
            // Both won - check if either has 2 cards
            if game_state.player1.live_card_zone.cards.len() == 2 {
                // Player1 has 2 cards, doesn't move
            } else {
                let card_index = crate::bot::ai::AIPlayer::choose_live_card_for_success(&game_state.player1);
                Self::move_live_to_success(&mut game_state.player1, card_index);
            }
            if game_state.player2.live_card_zone.cards.len() == 2 {
                // Player2 has 2 cards, doesn't move
            } else {
                let card_index = crate::bot::ai::AIPlayer::choose_live_card_for_success(&game_state.player2);
                Self::move_live_to_success(&mut game_state.player2, card_index);
            }
        } else if player1_won {
            let card_index = crate::bot::ai::AIPlayer::choose_live_card_for_success(&game_state.player1);
            Self::move_live_to_success(&mut game_state.player1, card_index);
        } else if player2_won {
            let card_index = crate::bot::ai::AIPlayer::choose_live_card_for_success(&game_state.player2);
            Self::move_live_to_success(&mut game_state.player2, card_index);
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
        // If both won or no one won, keep current first attacker
        
        // End turn
        game_state.turn_number += 1;
        game_state.reset_keyword_tracking();
        // Clear locked areas at start of new turn
        game_state.player1.areas_locked_this_turn.clear();
        game_state.player2.areas_locked_this_turn.clear();
        game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
        game_state.current_phase = Phase::Active;
    }

    fn move_live_to_success(player: &mut crate::player::Player, card_index: usize) {
        // Move specified card from live card zone to success live card zone
        if card_index < player.live_card_zone.cards.len() {
            let card = player.live_card_zone.cards.remove(card_index);
            player.success_live_card_zone.cards.push(card);
        }
    }

    pub fn check_timing(game_state: &mut GameState) {
        // Rule 9.5: Check timing - process rule processing per rules 10.2-10.6
        
        // Rule 10.2: Refresh (already handled in player.refresh())
        game_state.player1.refresh();
        game_state.player2.refresh();
        
        // Rule 10.3: Victory processing - check for 3+ successful live cards
        Self::check_victory_condition(game_state);
        
        // Rule 10.4: Check for duplicate members
        Self::check_duplicate_members(&mut game_state.player1);
        Self::check_duplicate_members(&mut game_state.player2);
        
        // Rule 10.5: Check for invalid cards
        Self::check_invalid_cards(&mut game_state.player1, &game_state.card_database);
        Self::check_invalid_cards(&mut game_state.player2, &game_state.card_database);
        
        // Rule 10.6: Check for invalid resolution zone
        Self::check_invalid_resolution_zone(game_state);
        
        // Rule 9.5.1: After rule processing, play and resolve automatic abilities
        let active_player_id = game_state.active_player().id.clone();
        game_state.process_pending_auto_abilities(&active_player_id);
    }

    pub fn check_victory_condition(game_state: &mut GameState) {
        // Rule 10.3.1: If a player has 3+ cards in success live card zone, they win
        let p1_success_count = game_state.player1.success_live_card_zone.cards.len();
        let p2_success_count = game_state.player2.success_live_card_zone.cards.len();
        
        if p1_success_count >= 3 {
            game_state.game_result = crate::game_state::GameResult::FirstAttackerWins;
        } else if p2_success_count >= 3 {
            game_state.game_result = crate::game_state::GameResult::SecondAttackerWins;
        }
        
        // Rule 1.2.1.2: If both players have 3+ cards simultaneously, it's a draw
        if p1_success_count >= 3 && p2_success_count >= 3 {
            game_state.game_result = crate::game_state::GameResult::Draw;
        }
    }

    fn check_duplicate_members(_player: &mut crate::player::Player) {
        // Rule 10.4: Check for duplicate members in same area
        // Rule 10.4.1: If multiple members in one area, keep the last one, send others to discard
        // energy_underneath tracking moved to GameState modifiers
        // For now, this is a no-op
    }

    fn check_invalid_cards(player: &mut crate::player::Player, card_db: &CardDatabase) {
        // Rule 10.5: Check for invalid cards in zones
        // Rule 10.5.1: Non-live cards in live card zone
        player.live_card_zone.cards.retain(|card_id| {
            card_db.get_card(*card_id).map(|c| c.is_live()).unwrap_or(false)
        });

        // Rule 10.5.2: Non-energy cards in energy zone
        player.energy_zone.cards.retain(|card_id| {
            card_db.get_card(*card_id).map(|c| c.is_energy()).unwrap_or(false)
        });

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
                    // Move to energy deck
                    player.energy_deck.cards.push(*card_id);
                }
            }
        }
    }

    fn check_invalid_resolution_zone(_game_state: &mut GameState) {
        // Rule 10.6: Invalid resolution zone processing
        // Rule 10.6.1: Cards in resolution zone that are not being played/resolved/cheered go to discard
        // Simplified: For now, assume all cards in resolution zone should be moved to discard
        // after they're done being processed
        // This would need to track which cards are currently being played/resolved
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
        
        // Rule 8.3.11: エール (cheer) - move cards from main deck to resolution zone
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
    fn trigger_debut_abilities(game_state: &mut GameState, player_id: &str, card_no: &str, cost_paid: u32) {
        // Rule 11.4: Trigger Debut (登場) automatic abilities
        // Rule 11.4.2: "【自動】 このメンバーが登場したとき、（効果）"
        
        // Q197/Q198: Auto abilities don't trigger when played via baton touch with cost 10+
        if cost_paid >= 10 {
            return;
        }
        
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
            
            // Find the card on stage
            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            for area in areas {
                if let Some(card_id) = player.stage.get_area(area) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        if card.card_no == card_no_clone {
                            // Check if card has Debut abilities
                            for ability in &card.abilities {
                                // Check if ability has Debut trigger
                                if ability.triggers.as_ref().map_or(false, |t| t == "登場") {
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
        // Rule 8.3.3: "手番プレイヤーの自動能力の'パフォーマンスフェイズの始めに'の誘発条件が発生し、チェックタイミングが発生します。"
        
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
                            if ability.triggers.as_ref().map_or(false, |t| t == "パフォーマンスフェイズの始めに" || t == "performance_phase_start") {
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
        // Rule 11.5.2: "【自動】 あなたが手番プレイヤーであるパフォーマンスフェイズのライブ開始時、（効果）"
        
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
                            if ability.triggers.as_ref().map_or(false, |t| t == "ライブ開始時") {
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

    fn trigger_live_success_abilities(game_state: &mut GameState, player_id: &str) {
        // Rule 11.6: Trigger LiveSuccess automatic abilities
        // Rule 11.6.2: "【自動】 あなたのライブが成功したとき、（効果）"
        
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
                            if ability.triggers.as_ref().map_or(false, |t| t == "ライブ成功時") {
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
                        if ability.full_text.contains("ライブ成功時") || ability.full_text.contains("LiveSuccess") {
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
