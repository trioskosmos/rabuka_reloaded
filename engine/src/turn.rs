use crate::game_state::{GameState, Phase, TurnPhase};
use crate::zones::MemberArea;

pub struct TurnEngine;

impl TurnEngine {
    pub fn advance_phase(game_state: &mut GameState) {
        // Advance phase according to rules 7.1.2, 7.3.3, and 8.1.2
        let current_phase = game_state.current_phase.clone();
        let current_turn_phase = game_state.current_turn_phase.clone();
        
        // Handle normal phase sub-phases (Rule 7.3.3)
        if current_turn_phase == TurnPhase::FirstAttackerNormal || current_turn_phase == TurnPhase::SecondAttackerNormal {
            match current_phase {
                Phase::Active => {
                    // Rule 7.4: Activate all energy and stage cards (automatic)
                    game_state.active_player_mut().activate_all_energy();
                    for area in [MemberArea::LeftSide, MemberArea::Center, MemberArea::RightSide] {
                        if let Some(card) = game_state.active_player_mut().stage.get_area_mut(area) {
                            if card.orientation == Some(crate::zones::Orientation::Wait) {
                                card.orientation = Some(crate::zones::Orientation::Active);
                            }
                        }
                    }
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Energy;
                }
                Phase::Energy => {
                    // Rule 7.5: Draw energy card (automatic)
                    Self::check_timing(game_state);
                    let _ = game_state.active_player_mut().draw_energy();
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Draw;
                }
                Phase::Draw => {
                    // Rule 7.6: Draw card (automatic)
                    Self::check_timing(game_state);
                    let _ = game_state.active_player_mut().draw_card();
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
                        let player = game_state.first_attacker_mut();
                        let result = Self::player_perform_live(player, &mut resolution_zone);
                        // Put resolution_zone back
                        game_state.resolution_zone = resolution_zone;
                        result
                    };
                    game_state.player1_cheer_blade_heart_count = blade_heart_count;
                    
                    // Rule 8.3.7-8.3.8: Trigger LiveStart abilities
                    // Rule 11.5: Trigger LiveStart automatic abilities
                    let player_id = game_state.player1.id.clone();
                    Self::trigger_live_start_abilities(game_state, &player_id);
                    
                    game_state.current_phase = Phase::SecondAttackerPerformance;
                }
                Phase::SecondAttackerPerformance => {
                    // Rule 8.3: Second attacker performs (automatic)
                    let blade_heart_count = {
                        // Take resolution_zone first to avoid borrow conflicts
                        let mut resolution_zone = std::mem::take(&mut game_state.resolution_zone);
                        let player = game_state.second_attacker_mut();
                        let result = Self::player_perform_live(player, &mut resolution_zone);
                        // Put resolution_zone back
                        game_state.resolution_zone = resolution_zone;
                        result
                    };
                    game_state.player2_cheer_blade_heart_count = blade_heart_count; // This is actually total blades for cheer bonus
                    
                    // Rule 8.3.7-8.3.8: Trigger LiveStart abilities
                    // Rule 11.5: Trigger LiveStart automatic abilities
                    let player_id = game_state.player2.id.clone();
                    Self::trigger_live_start_abilities(game_state, &player_id);
                    
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

    pub fn execute_main_phase_action(game_state: &mut GameState, action: &str, card_index: Option<usize>, card_indices: Option<Vec<usize>>, stage_area: Option<String>) -> Result<(), String> {
        // Execute player choice action during various phases
        match action {
            "rps_choice" => {
                // Rule 6.2.2: Rock Paper Scissors to determine who chooses to go first
                // For simplicity, player 1 always chooses, and we randomly determine player 2's choice
                use rand::Rng;
                let mut rng = rand::thread_rng();
                
                let p1_choice = match stage_area.as_deref() {
                    Some("rock") => 0,
                    Some("paper") => 1,
                    Some("scissors") => 2,
                    _ => return Err("Invalid RPS choice".to_string()),
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
            "select_mulligan" => {
                // Toggle card selection for mulligan
                let idx = card_index.unwrap_or(0);
                if let Some(pos) = game_state.mulligan_selected_indices.iter().position(|&x| x == idx) {
                    // Already selected, deselect
                    game_state.mulligan_selected_indices.remove(pos);
                } else {
                    // Not selected, select
                    game_state.mulligan_selected_indices.push(idx);
                }
                Ok(())
            }
            "confirm_mulligan" => {
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
                        current_player.main_deck.cards.push_back(card);
                    }
                    
                    // Shuffle main deck
                    use rand::seq::SliceRandom;
                    let mut deck_vec: Vec<_> = current_player.main_deck.cards.drain(..).collect();
                    deck_vec.shuffle(&mut rand::thread_rng());
                    for card in deck_vec {
                        current_player.main_deck.cards.push_back(card);
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
            "skip_mulligan" => {
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
            "play_member_to_stage" => {
                // Get turn number before any mutable borrows
                let current_turn = game_state.turn_number;

                let player = game_state.active_player_mut();
                
                // Use provided parameters if available, otherwise use simple fallback
                let (idx, area) = if let (Some(ci), Some(sa)) = (card_index, stage_area) {
                    let area_enum = match sa.as_str() {
                        "left" => crate::zones::MemberArea::LeftSide,
                        "center" => crate::zones::MemberArea::Center,
                        "right" => crate::zones::MemberArea::RightSide,
                        _ => crate::zones::MemberArea::LeftSide,
                    };
                    (ci, area_enum)
                } else {
                    // Fallback: play first member card to first available stage area
                    let member_index = player.hand.cards.iter()
                        .position(|c| c.is_member());
                    
                    let idx = match member_index {
                        Some(i) => i,
                        None => return Err("No member cards in hand".to_string()),
                    };
                    
                    // Find first available stage area
                    let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
                    let mut area_enum = crate::zones::MemberArea::LeftSide;
                    for area in areas {
                        if player.stage.get_area(area).is_none() {
                            area_enum = area;
                            break;
                        }
                    }
                    (idx, area_enum)
                };

                // Get card info before moving it
                let card_no = player.hand.cards[idx].card_no.clone();
                let player_id = player.id.clone();

                let cost_paid = player.move_card_from_hand_to_stage(idx, area)?;

                // Set turn_played for the card on stage
                if let Some(card_in_zone) = player.stage.get_area_mut(area) {
                    card_in_zone.turn_played = current_turn;
                }
                
                // Trigger 登場 abilities for the played card
                // Q197/Q198: Auto abilities don't trigger when played via baton touch with cost 10+
                Self::trigger_debut_abilities(game_state, &player_id, &card_no, cost_paid);
                
                Ok(())
            }
            "place_live_cards" => {
                // Rule 8.2: Live Card Set Phase - Place individual card face-down, max 3 cards
                let card_idx = card_index;
                
                if let Some(idx) = card_idx {
                    // Place a single card
                    let player = game_state.active_player_mut();
                    if idx < player.hand.cards.len() && player.live_card_zone.cards.len() < 3 {
                        let card = player.hand.cards.remove(idx);
                        let _ = player.live_card_zone.add_card(card, true);
                        // Draw 1 card when placing 1 card (Rule 8.2)
                        let _ = player.draw_card();
                    }
                    // Don't advance phase yet - allow placing more cards up to 3
                } else {
                    // No card selected, finish this player's live card set
                    // Switch to other player for their live card set
                    if game_state.current_turn_phase == crate::game_state::TurnPhase::Live {
                        // Check if first player has finished
                        if game_state.player1.live_card_zone.cards.len() > 0 || game_state.player2.live_card_zone.cards.len() > 0 {
                            // At least one player has set cards, check if both have finished
                            let p1_finished = game_state.player1.live_card_zone.cards.len() > 0;
                            let p2_finished = game_state.player2.live_card_zone.cards.len() > 0;
                            
                            if p1_finished && p2_finished {
                                // Both players have finished, advance phase
                                Self::advance_phase(game_state);
                            } else if p1_finished {
                                // P1 finished, now P2's turn
                                // Just stay in LiveCardSet phase but actions will be for P2
                            } else if p2_finished {
                                // P2 finished, now P1's turn
                                // Just stay in LiveCardSet phase but actions will be for P1
                            } else {
                                // Neither has set any cards, advance phase
                                Self::advance_phase(game_state);
                            }
                        } else {
                            // Neither has set cards, advance phase
                            Self::advance_phase(game_state);
                        }
                    } else {
                        Self::advance_phase(game_state);
                    }
                }
                Ok(())
            }
            "play_member_left" => {
                // TODO: Implement playing member to left side
                Err("Play member left not implemented".to_string())
            }
            "play_member_center" => {
                // TODO: Implement playing member to center
                Err("Play member center not implemented".to_string())
            }
            "play_member_right" => {
                // TODO: Implement playing member to right side
                Err("Play member right not implemented".to_string())
            }
            "play_energy" => {
                // TODO: Implement playing energy
                Err("Play energy not implemented".to_string())
            }
            "pass" => {
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
            if let Some(card) = game_state.player1.energy_deck.draw() {
                let card_in_zone = crate::zones::CardInZone {
                    card: card.clone(),
                    orientation: Some(crate::zones::Orientation::Active),
                    face_state: crate::zones::FaceState::FaceUp,
                    energy_underneath: Vec::new(),
                    played_via_ability: false,
                    turn_played: 0,
                };
                let _ = game_state.player1.energy_zone.add_card(card_in_zone);
            }
            if let Some(card) = game_state.player2.energy_deck.draw() {
                let card_in_zone = crate::zones::CardInZone {
                    card: card.clone(),
                    orientation: Some(crate::zones::Orientation::Active),
                    face_state: crate::zones::FaceState::FaceUp,
                    energy_underneath: Vec::new(),
                    played_via_ability: false,
                    turn_played: 0,
                };
                let _ = game_state.player2.energy_zone.add_card(card_in_zone);
            }
        }
    }

    pub fn execute_live_victory_determination(game_state: &mut GameState) {
        // Rule 8.4: Determine live victory
        // Rule 8.4.2.1: Add cheer blade heart count to score
        let player1_score = game_state.player1.live_card_zone.calculate_live_score(game_state.player1_cheer_blade_heart_count);
        let player2_score = game_state.player2.live_card_zone.calculate_live_score(game_state.player2_cheer_blade_heart_count);
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
            // TODO: Implement automatic ability triggering - track which abilities have triggered this turn
            
            // Rule 8.4.11: Check timing again
            Self::check_timing(game_state);
            
            // Rule 8.4.11: End 'until end of turn' and 'during this turn' effects
            // TODO: Implement effect expiration - track active effects and their durations
            
            // Rule 8.4.12: Loop back to 8.4.9 if new abilities triggered
            // TODO: Implement loop detection - check if any new abilities were triggered in this iteration
            // For now, break the loop to prevent infinite loop
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

    #[allow(dead_code)]
    fn move_live_to_exclusion(player: &mut crate::player::Player) {
        // Move all live cards to exclusion zone
        for card in player.live_card_zone.clear() {
            player.exclusion_zone.cards.push(crate::zones::CardInZone {
                card: card,
                orientation: Some(crate::zones::Orientation::Wait),
                face_state: crate::zones::FaceState::FaceDown,
                energy_underneath: Vec::new(),
                played_via_ability: false,
                turn_played: 0,
            });
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
        Self::check_invalid_cards(&mut game_state.player1);
        Self::check_invalid_cards(&mut game_state.player2);
        
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

    fn check_duplicate_members(player: &mut crate::player::Player) {
        // Rule 10.4: Check for duplicate members in same area
        // Rule 10.4.1: If multiple members in one area, keep the last one, send others to discard
        let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
        
        for area in areas {
            if let Some(card_in_zone) = player.stage.get_area_mut(area) {
                // Check if there are energy cards underneath (indicating multiple cards)
                if !card_in_zone.energy_underneath.is_empty() {
                    // Keep only the top card (the last one placed)
                    // Move all energy-underneath cards to discard
                    for energy_card in card_in_zone.energy_underneath.drain(..) {
                        player.waitroom.cards.push(energy_card);
                    }
                }
            }
        }
    }

    fn check_invalid_cards(player: &mut crate::player::Player) {
        // Rule 10.5: Check for invalid cards in zones
        // Rule 10.5.1: Non-live cards in live card zone
        player.live_card_zone.cards.retain(|c| c.is_live());
        
        // Rule 10.5.2: Non-energy cards in energy zone
        player.energy_zone.cards.retain(|c| c.card.is_energy());
        
        // Rule 10.5.3: Energy cards without member above in member area
        // Check each member area
        for area in [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide] {
            if let Some(card_in_zone) = player.stage.get_area_mut(area) {
                // If no member above, move energy cards to energy deck
                if card_in_zone.card.is_energy() {
                    let energy_card = card_in_zone.card.clone();
                    *card_in_zone = crate::zones::CardInZone {
                        card: crate::card::Card {
                            card_no: String::new(),
                            img: None,
                            name: String::new(),
                            product: String::new(),
                            card_type: crate::card::CardType::Energy,
                            series: String::new(),
                            group: String::new(),
                            unit: None,
                            cost: None,
                            base_heart: None,
                            blade_heart: None,
                            blade: 0,
                            rare: String::new(),
                            ability: String::new(),
                            faq: Vec::new(),
                            _img: None,
                            score: None,
                            need_heart: None,
                            special_heart: None,
                            abilities: Vec::new(),
                        },
                        orientation: None,
                        face_state: crate::zones::FaceState::FaceDown,
                        energy_underneath: Vec::new(),
                        played_via_ability: false,
                        turn_played: 0,
                    };
                    // Move to energy deck
                    player.energy_deck.cards.push_back(energy_card);
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

    pub fn player_set_live_cards(player: &mut crate::player::Player, num_cards_to_set: usize) {
        // Rule 8.2: Player sets live cards face-down and draws equal amount
        let live_cards: Vec<_> = player.hand.cards.iter()
            .filter(|c| c.is_live())
            .cloned()
            .collect();
        
        if live_cards.is_empty() || num_cards_to_set == 0 {
            return;
        }
        
        // Set specified number of cards
        let cards_to_set = std::cmp::min(num_cards_to_set, live_cards.len());
        for i in 0..cards_to_set {
            let card = live_cards[i].clone();
            // Remove from hand
            if let Some(pos) = player.hand.cards.iter().position(|c| c.card_no == card.card_no) {
                player.hand.cards.remove(pos);
            }
            // Add to live card zone face-down
            let _ = player.live_card_zone.add_card(card, true);
        }
        
        // Draw equal amount
        for _ in 0..cards_to_set {
            let _ = player.draw_card();
        }
    }

    pub fn player_perform_live(player: &mut crate::player::Player, resolution_zone: &mut crate::zones::ResolutionZone) -> u32 {
        // Rule 8.3: Player performs live - check heart requirements
        // Rule 8.3.4: Reveal cards, discard non-live cards
        player.live_card_zone.cards.retain(|c| c.is_live());
        
        // Rule 8.3.6: If no live cards, end performance
        if player.live_card_zone.cards.is_empty() {
            return 0;
        }
        
        // Rule 8.3.7: Live start event
        // TODO: Trigger LiveStart abilities
        
        // Rule 8.3.10: Calculate total blades from active members
        let total_blades = player.stage.total_blades();
        
        // Rule 8.3.11: エール (cheer) - move cards from main deck to resolution zone
        for _ in 0..total_blades {
            if let Some(card) = player.main_deck.cards.pop_front() {
                resolution_zone.cards.push(card);
            }
        }
        
        // Rule 8.3.12: Check blade hearts on cards in resolution zone
        let mut blade_heart_count = 0;
        for card in &resolution_zone.cards {
            if let Some(ref blade_heart) = card.blade_heart {
                blade_heart_count += blade_heart.hearts.values().sum::<u32>();
            }
        }
        
        // Rule 8.3.12.1: Draw cards based on blade heart count
        for _ in 0..blade_heart_count {
            let _ = player.draw_card();
        }
        
        // Rule 8.3.14: Calculate live-owned hearts from stage and blade hearts
        let stage_hearts = player.stage.get_available_hearts();
        let mut live_owned_hearts = stage_hearts.clone();
        
        // Add blade hearts from resolution zone
        for card in &resolution_zone.cards {
            if let Some(ref blade_heart) = card.blade_heart {
                for (color, count) in &blade_heart.hearts {
                    *live_owned_hearts.hearts.entry(color.clone()).or_insert(0) += count;
                }
            }
        }
        
        // Rule 8.3.15: Check if each live card can satisfy required hearts
        let mut remaining_hearts = live_owned_hearts.clone();
        let mut live_cards_to_remove = Vec::new();
        
        for card in &player.live_card_zone.cards {
            if let Some(ref need_heart) = card.need_heart {
                let mut can_satisfy = true;
                let mut temp_hearts = remaining_hearts.hearts.clone();
                
                for (color, needed) in &need_heart.hearts {
                    if color == "heart00" {
                        // Wildcard heart (rule 8.3.15.1.1) - can be any color
                        // Count total hearts available
                        let total_available: u32 = temp_hearts.values().sum();
                        if total_available < *needed {
                            can_satisfy = false;
                            break;
                        }
                        // Consume from any colors (prefer non-wildcards first)
                        let mut consumed = 0;
                        for (c, count) in temp_hearts.iter_mut() {
                            if *c != "heart00" {
                                let to_consume = std::cmp::min(*count, *needed - consumed);
                                *count -= to_consume;
                                consumed += to_consume;
                                if consumed >= *needed {
                                    break;
                                }
                            }
                        }
                        // If still need more, consume from wildcards
                        if consumed < *needed {
                            if let Some(wildcard_count) = temp_hearts.get_mut("heart00") {
                                let to_consume = std::cmp::min(*wildcard_count, *needed - consumed);
                                *wildcard_count -= to_consume;
                            }
                        }
                    } else {
                        // Specific color heart
                        let available = temp_hearts.get(color).unwrap_or(&0);
                        if *available < *needed {
                            can_satisfy = false;
                            break;
                        }
                        *temp_hearts.get_mut(color).unwrap() -= needed;
                    }
                }
                
                if can_satisfy {
                    // Update remaining hearts with the temp consumption
                    remaining_hearts.hearts = temp_hearts;
                } else {
                    live_cards_to_remove.push(card.clone());
                }
            }
        }
        
        // Rule 8.3.16: If any fails, all live cards go to discard
        if !live_cards_to_remove.is_empty() {
            // Move all live cards to discard
            for card in player.live_card_zone.clear() {
                player.waitroom.cards.push(card);
            }
        }
        
        // Move resolution zone cards to discard
        for card in resolution_zone.cards.drain(..) {
            player.waitroom.cards.push(card);
        }
        
        blade_heart_count
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
                if let Some(card_in_zone) = player.stage.get_area(area) {
                    if card_in_zone.card.card_no == card_no_clone {
                        // Check if card has Debut abilities
                        for ability in &card_in_zone.card.abilities {
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
                if let Some(card_in_zone) = player.stage.get_area(area) {
                    // Check if card has LiveStart abilities
                    for ability in &card_in_zone.card.abilities {
                        // Check if ability has LiveStart trigger
                        if ability.triggers.as_ref().map_or(false, |t| t == "ライブ開始時") {
                            // Collect ability to trigger
                            let ability_id = format!("{}_{}", card_in_zone.card.card_no, ability.full_text);
                            abilities_to_trigger.push((ability_id, card_in_zone.card.card_no.clone()));
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
            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            for area in areas {
                if let Some(card_in_zone) = player.stage.get_area(area) {
                    // Check if card has LiveSuccess abilities
                    for ability in &card_in_zone.card.abilities {
                        // Check if ability has LiveSuccess trigger
                        if ability.triggers.as_ref().map_or(false, |t| t == "ライブ成功時") {
                            // Collect ability to trigger
                            let ability_id = format!("{}_{}", card_in_zone.card.card_no, ability.full_text);
                            abilities_to_trigger.push((ability_id, card_in_zone.card.card_no.clone()));
                        }
                    }
                }
            }
            
            // Also check live cards in live card zone
            for card in &player.live_card_zone.cards {
                for ability in &card.abilities {
                    if ability.full_text.contains("ライブ成功時") || ability.full_text.contains("LiveSuccess") {
                        let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                        abilities_to_trigger.push((ability_id, card.card_no.clone()));
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
