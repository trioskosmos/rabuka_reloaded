// Game setup and initialization functions
// This module contains shared game setup logic used by both the web server and bot modules

use crate::game_state::GameState;
use crate::player::Player;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Action {
    pub description: String,
    pub action_type: String,
    pub parameters: Option<ActionParameters>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ActionParameters {
    pub card_index: Option<usize>,
    pub card_indices: Option<Vec<usize>>, // For selecting multiple cards (e.g., live cards)
    pub stage_area: Option<String>, // "left", "center", "right"
}

pub fn setup_game(game_state: &mut GameState) {
    // Rule 6.2: Pre-Game Procedure
    // Start at RockPaperScissors phase - player will choose RPS option
    game_state.current_phase = crate::game_state::Phase::RockPaperScissors;
}

/// Rock-paper-scissors for determining first attacker
/// Returns 1 if player 1 wins, 2 if player 2 wins
#[allow(dead_code)]
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
#[allow(dead_code)]
enum RockPaperScissorsChoice {
    Rock,
    Paper,
    Scissors,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
fn perform_player_mulligan(player: &mut Player) {
    use rand::seq::SliceRandom;
    
    // Rule 6.2.1.6: Starting from first attacker, each player chooses any number of cards from hand
    // Simple rule-based strategy: mulligan if hand has no member cards
    let has_member = player.hand.cards.iter().any(|c| !c.is_energy() && !c.is_live());
    
    if !has_member && !player.hand.cards.is_empty() {
        // Mulligan all cards
        let num_to_mulligan = player.hand.cards.len();
        let cards_to_set_aside: Vec<_> = player.hand.cards.drain(..).collect();
        
        // Rule 6.2.1.6: Place cards face-down to the side (waitroom temporarily)
        // We'll hold them in a temporary vector
        
        // Rule 6.2.1.6: Draw the same number of cards from main deck to hand
        for _ in 0..num_to_mulligan {
            let _ = player.draw_card();
        }
        
        // Rule 6.2.1.6: Move set-aside cards to main deck
        for card in cards_to_set_aside {
            player.main_deck.cards.push_back(card);
        }
        
        // Rule 6.2.1.6: Shuffle if 1+ cards were moved
        let mut deck_vec: Vec<_> = player.main_deck.cards.drain(..).collect();
        deck_vec.shuffle(&mut rand::thread_rng());
        for card in deck_vec {
            player.main_deck.cards.push_back(card);
        }
        
        println!("Player mulliganed {} cards", num_to_mulligan);
    } else {
        println!("Player kept their hand");
    }
}

pub fn generate_possible_actions(game_state: &GameState) -> Vec<Action> {
    let mut actions = Vec::new();
    let active_player = game_state.active_player();
    
    match game_state.current_phase {
        crate::game_state::Phase::RockPaperScissors => {
            // Rule 6.2.2: Rock Paper Scissors to determine who chooses to go first
            // Generate actions for player 1 to choose RPS option
            actions.push(Action {
                description: "Rock".to_string(),
                action_type: "rps_choice".to_string(),
                parameters: Some(ActionParameters {
                    card_index: Some(0), // 0 = rock
                    card_indices: None,
                    stage_area: Some("rock".to_string()),
                }),
            });
            actions.push(Action {
                description: "Paper".to_string(),
                action_type: "rps_choice".to_string(),
                parameters: Some(ActionParameters {
                    card_index: Some(1), // 1 = paper
                    card_indices: None,
                    stage_area: Some("paper".to_string()),
                }),
            });
            actions.push(Action {
                description: "Scissors".to_string(),
                action_type: "rps_choice".to_string(),
                parameters: Some(ActionParameters {
                    card_index: Some(2), // 2 = scissors
                    card_indices: None,
                    stage_area: Some("scissors".to_string()),
                }),
            });
        }
        crate::game_state::Phase::Mulligan => {
            // Rule 6.2.1.6: Mulligan - player chooses cards to mulligan
            // Generate actions for each card in hand to select/deselect for mulligan
            for (hand_index, card) in active_player.hand.cards.iter().enumerate() {
                actions.push(Action {
                    description: format!("Select {} for mulligan", card.name),
                    action_type: "select_mulligan".to_string(),
                    parameters: Some(ActionParameters {
                        card_index: Some(hand_index),
                        card_indices: None,
                        stage_area: None,
                    }),
                });
            }
            
            // Add action to confirm mulligan selection
            actions.push(Action {
                description: "Confirm mulligan (keep selected cards)".to_string(),
                action_type: "confirm_mulligan".to_string(),
                parameters: Some(ActionParameters {
                    card_index: None,
                    card_indices: Some(vec![]), // Will be populated with selected indices
                    stage_area: None,
                }),
            });
            
            // Add action to skip mulligan
            actions.push(Action {
                description: "Skip mulligan (keep all cards)".to_string(),
                action_type: "skip_mulligan".to_string(),
                parameters: None,
            });
        }
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
                parameters: None,
            });
            
            // Rule 7.7.2.2: Main Phase - Can play member cards to stage
            // Generate specific actions for each member card and each available area
            // Only generate member cards (not live cards) - live cards are used in live phase
            for (hand_index, card) in active_player.hand.cards.iter().enumerate() {
                if card.is_member() && !card.is_live() {
                    // Check which areas are available
                    let areas = [
                        (crate::zones::MemberArea::LeftSide, "left"),
                        (crate::zones::MemberArea::Center, "center"),
                        (crate::zones::MemberArea::RightSide, "right"),
                    ];
                    
                    for (area, area_name) in areas {
                        let card_cost = card.cost.unwrap_or(0);
                        let mut cost_to_pay = card_cost;
                        
                        // Check if area is occupied for baton touch
                        if let Some(existing_member) = active_player.stage.get_area(area) {
                            // Baton touch - replace existing member
                            let member_cost = existing_member.card.cost.unwrap_or(0);
                            cost_to_pay = cost_to_pay.saturating_sub(member_cost);
                            
                            let active_energy_count = active_player.energy_zone.cards.iter()
                                .filter(|c| c.orientation == Some(crate::zones::Orientation::Active))
                                .count() as u32;
                            
                            if active_energy_count >= cost_to_pay {
                                actions.push(Action {
                                    description: format!("Baton Touch: Play {} ({}) to {} (replaces {})", card.name, card.card_no, area_name, existing_member.card.name),
                                    action_type: "play_member_to_stage".to_string(),
                                    parameters: Some(ActionParameters {
                                        card_index: Some(hand_index),
                                        card_indices: None,
                                        stage_area: Some(area_name.to_string()),
                                    }),
                                });
                            }
                        } else {
                            // Play to empty area
                            let active_energy_count = active_player.energy_zone.cards.iter()
                                .filter(|c| c.orientation == Some(crate::zones::Orientation::Active))
                                .count() as u32;
                            
                            if active_energy_count >= cost_to_pay {
                                actions.push(Action {
                                    description: format!("Play {} ({}) to {} area", card.name, card.card_no, area_name),
                                    action_type: "play_member_to_stage".to_string(),
                                    parameters: Some(ActionParameters {
                                        card_index: Some(hand_index),
                                        card_indices: None,
                                        stage_area: Some(area_name.to_string()),
                                    }),
                                });
                            }
                        }
                    }
                }
            }
            
            // Basic abilityless play - only pass and play member cards
            // Advanced actions (swap, pay energy, place energy under members) removed for basic play
        }
        crate::game_state::Phase::LiveCardSet => {
            // Rule 8.2: Live Card Set Phase - Can place up to 3 live cards face-down
            // Generate actions for placing specific live cards
            let live_cards: Vec<_> = active_player.hand.cards.iter()
                .enumerate()
                .filter(|(_, c)| c.is_live())
                .collect();
            
            if !live_cards.is_empty() {
                // Add action to place 0 cards (skip live)
                actions.push(Action {
                    description: "Place 0 live cards (skip)".to_string(),
                    action_type: "place_live_cards".to_string(),
                    parameters: Some(ActionParameters {
                        card_index: None,
                        card_indices: Some(vec![]),
                        stage_area: None,
                    }),
                });
                
                // Generate actions for placing 1-3 specific cards
                // Generate all combinations of 1-3 cards from available live cards
                let max_cards = std::cmp::min(3, live_cards.len());
                
                // Generate combinations using simple approach
                for count in 1..=max_cards {
                    generate_live_card_combinations(&live_cards, count, &mut actions);
                }
            } else {
                // No live cards, must place 0
                actions.push(Action {
                    description: "Place 0 live cards (no live cards in hand)".to_string(),
                    action_type: "place_live_cards".to_string(),
                    parameters: Some(ActionParameters {
                        card_index: None,
                        card_indices: Some(vec![]),
                        stage_area: None,
                    }),
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

fn generate_live_card_combinations(live_cards: &[(usize, &crate::card::Card)], count: usize, actions: &mut Vec<Action>) {
    // Generate all combinations of 'count' cards from live_cards
    // Use a simple recursive approach
    let mut indices = Vec::new();
    generate_combinations_recursive(live_cards, count, 0, &mut indices, actions);
}

fn generate_combinations_recursive(
    live_cards: &[(usize, &crate::card::Card)],
    count: usize,
    start: usize,
    current: &mut Vec<usize>,
    actions: &mut Vec<Action>,
) {
    if current.len() == count {
        // Generate action for this combination
        let card_names: Vec<String> = current.iter()
            .map(|&idx| live_cards[idx].1.name.clone())
            .collect();
        let description = format!("Place live card(s): {} face-down", card_names.join(", "));
        let indices: Vec<usize> = current.iter().map(|&idx| live_cards[idx].0).collect();
        
        actions.push(Action {
            description,
            action_type: "place_live_cards".to_string(),
            parameters: Some(ActionParameters {
                card_index: None,
                card_indices: Some(indices),
                stage_area: None,
            }),
        });
        return;
    }
    
    for i in start..live_cards.len() {
        current.push(i);
        generate_combinations_recursive(live_cards, count, i + 1, current, actions);
        current.pop();
    }
}
