// Game setup and initialization functions
// This module contains shared game setup logic used by both the web server and bot modules

use crate::game_state::GameState;
use crate::player::Player;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Action {
    pub description: String,
    pub action_type: String,
}

pub fn setup_game(game_state: &mut GameState) {
    // Rule 6.2: Pre-Game Procedure
    
    // Rule 6.2.2: Rock Paper Scissors to determine who chooses to go first
    let rps_winner = play_rock_paper_scissors();
    // Winner chooses to be first or second attacker
    // For now, default: winner of RPS becomes first attacker
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
    
    // Rule 6.2.6: Mulligan - Players may return cards and draw new cards
    // Rule 6.2.1.6: Starting from first attacker, each player chooses cards to mulligan
    perform_mulligan(game_state);
    
    // Rule 6.2.7: Initial energy - Each player draws 3 cards from energy deck to Energy Zone
    // Rule 6.2.1.7: These initial energy cards start in Active state
    for _ in 0..3 {
        if let Some(card) = game_state.player1.energy_deck.draw() {
            let card_in_zone = crate::zones::CardInZone {
                card: card.clone(),
                orientation: Some(crate::zones::Orientation::Active),
                face_state: crate::zones::FaceState::FaceUp,
                energy_underneath: Vec::new(),
            };
            let _ = game_state.player1.energy_zone.add_card(card_in_zone);
        }
        if let Some(card) = game_state.player2.energy_deck.draw() {
            let card_in_zone = crate::zones::CardInZone {
                card: card.clone(),
                orientation: Some(crate::zones::Orientation::Active),
                face_state: crate::zones::FaceState::FaceUp,
                energy_underneath: Vec::new(),
            };
            let _ = game_state.player2.energy_zone.add_card(card_in_zone);
        }
    }
    
    // Set phase to Active to start the game
    game_state.current_phase = crate::game_state::Phase::Active;
}

/// Rock-paper-scissors for determining first attacker
/// Returns 1 if player 1 wins, 2 if player 2 wins
pub fn play_rock_paper_scissors() -> u8 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    let choices = [RockPaperScissorsChoice::Rock, RockPaperScissorsChoice::Paper, RockPaperScissorsChoice::Scissors];
    
    let p1_choice = choices[rng.gen_range(0..3)];
    let p2_choice = choices[rng.gen_range(0..3)];
    
    match (p1_choice, p2_choice) {
        (RockPaperScissorsChoice::Rock, RockPaperScissorsChoice::Scissors) => 1,
        (RockPaperScissorsChoice::Paper, RockPaperScissorsChoice::Rock) => 1,
        (RockPaperScissorsChoice::Scissors, RockPaperScissorsChoice::Paper) => 1,
        (RockPaperScissorsChoice::Scissors, RockPaperScissorsChoice::Rock) => 2,
        (RockPaperScissorsChoice::Rock, RockPaperScissorsChoice::Paper) => 2,
        (RockPaperScissorsChoice::Paper, RockPaperScissorsChoice::Scissors) => 2,
        _ => {
            // Tie - play again
            play_rock_paper_scissors()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RockPaperScissorsChoice {
    Rock,
    Paper,
    Scissors,
}

fn perform_mulligan(game_state: &mut GameState) {
    // Rule 6.2.1.6: Mulligan phase
    // Starting from first attacker, each player may return cards to deck and draw new ones
    // NOTE: This is a simplified implementation. In a real game, players would choose which cards to mulligan.
    // For automated play, this uses a simple rule-based strategy.
    
    // Determine order: first attacker goes first
    let (first_player, second_player) = if game_state.player1.is_first_attacker {
        (&mut game_state.player1, &mut game_state.player2)
    } else {
        (&mut game_state.player2, &mut game_state.player1)
    };
    
    // Simple rule-based strategy: mulligan if hand has no member cards
    perform_player_mulligan(first_player);
    perform_player_mulligan(second_player);
}

fn perform_player_mulligan(player: &mut Player) {
    use rand::seq::SliceRandom;
    
    // Simple rule-based strategy: mulligan if hand has no member cards
    let has_member = player.hand.cards.iter().any(|c| !c.is_energy() && !c.is_live());
    
    if !has_member && !player.hand.cards.is_empty() {
        // Mulligan all cards
        let cards_to_return: Vec<_> = player.hand.cards.drain(..).collect();
        let num_to_draw = cards_to_return.len();
        
        // Move set-aside cards to main deck
        for card in cards_to_return {
            player.main_deck.cards.push_back(card);
        }
        
        // Shuffle main deck
        let mut deck_vec: Vec<_> = player.main_deck.cards.drain(..).collect();
        deck_vec.shuffle(&mut rand::thread_rng());
        for card in deck_vec {
            player.main_deck.cards.push_back(card);
        }
        
        // Draw new cards
        for _ in 0..num_to_draw {
            let _ = player.draw_card();
        }
        
        println!("Player mulliganed {} cards", num_to_draw);
    } else {
        println!("Player kept their hand");
    }
}

pub fn generate_possible_actions(game_state: &GameState) -> Vec<Action> {
    let mut actions = Vec::new();
    let active_player = game_state.active_player();
    
    match game_state.current_phase {
        crate::game_state::Phase::Active => {
            // Rule 7.4: Active Phase - AUTOMATIC, no player actions
            // Energy activation happens automatically in advance_phase
        }
        crate::game_state::Phase::Energy => {
            // Rule 7.5: Energy Phase - AUTOMATIC, no player actions
            // Card draw happens automatically in advance_phase
        }
        crate::game_state::Phase::Draw => {
            // Rule 7.6: Draw Phase - AUTOMATIC, no player actions
            // Card draw happens automatically in advance_phase
        }
        crate::game_state::Phase::Main => {
            // Add pass action to end Main phase
            actions.push(Action {
                description: "Pass - End Main Phase".to_string(),
                action_type: "pass".to_string(),
            });
            
            // Rule 8.2: Main Phase - Can play member cards to stage
            if active_player.can_play_member_to_stage() {
                actions.push(Action {
                    description: "Play member card from hand to stage".to_string(),
                    action_type: "play_member_to_stage".to_string(),
                });
            }
            
            // Rule 5.5: Can shuffle deck if not empty
            if active_player.can_shuffle_zone("deck") {
                actions.push(Action {
                    description: "Shuffle main deck".to_string(),
                    action_type: "shuffle_deck".to_string(),
                });
            }
            
            // Rule 5.7: Can look at top cards
            if active_player.can_look_at_top(1) {
                actions.push(Action {
                    description: "Look at top card of main deck".to_string(),
                    action_type: "look_at_top".to_string(),
                });
            }
            
            // Rule 5.8: Can swap cards between areas
            if active_player.can_swap_cards(
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::Center,
            ) {
                actions.push(Action {
                    description: "Swap left side and center members".to_string(),
                    action_type: "swap_left_center".to_string(),
                });
            }
            if active_player.can_swap_cards(
                crate::zones::MemberArea::Center,
                crate::zones::MemberArea::RightSide,
            ) {
                actions.push(Action {
                    description: "Swap center and right side members".to_string(),
                    action_type: "swap_center_right".to_string(),
                });
            }
            if active_player.can_swap_cards(
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::RightSide,
            ) {
                actions.push(Action {
                    description: "Swap left side and right side members".to_string(),
                    action_type: "swap_left_right".to_string(),
                });
            }
            
            // Rule 5.9: Can pay energy
            if active_player.can_pay_energy(1) {
                actions.push(Action {
                    description: "Pay 1 energy".to_string(),
                    action_type: "pay_energy_1".to_string(),
                });
            }
            if active_player.can_pay_energy(2) {
                actions.push(Action {
                    description: "Pay 2 energy".to_string(),
                    action_type: "pay_energy_2".to_string(),
                });
            }
            
            // Can place energy under members
            if active_player.can_place_energy_under_member(crate::zones::MemberArea::LeftSide) {
                actions.push(Action {
                    description: "Place energy under left side member".to_string(),
                    action_type: "place_energy_under_left".to_string(),
                });
            }
            if active_player.can_place_energy_under_member(crate::zones::MemberArea::Center) {
                actions.push(Action {
                    description: "Place energy under center member".to_string(),
                    action_type: "place_energy_under_center".to_string(),
                });
            }
            if active_player.can_place_energy_under_member(crate::zones::MemberArea::RightSide) {
                actions.push(Action {
                    description: "Place energy under right side member".to_string(),
                    action_type: "place_energy_under_right".to_string(),
                });
            }
        }
        crate::game_state::Phase::LiveCardSet => {
            // Rule 9.1: Live Card Set Phase - Can place cards in live zone
            if active_player.can_place_in_live_zone() {
                actions.push(Action {
                    description: "Place card in Live Card Zone".to_string(),
                    action_type: "place_in_live_zone".to_string(),
                });
            }
        }
        crate::game_state::Phase::FirstAttackerPerformance
        | crate::game_state::Phase::SecondAttackerPerformance
        | crate::game_state::Phase::LiveVictoryDetermination => {
            // Live phase actions - currently no specific actions
        }
    }
    
    actions
}
