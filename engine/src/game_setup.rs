// Game setup and initialization functions
// This module contains shared game setup logic used by both the web server and bot modules

use crate::game_state::GameState;
use crate::player::Player;
use serde::{Serialize, Deserialize};
use std::vec::Vec;

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
    pub use_baton_touch: Option<bool>, // Whether to use baton touch cost reduction
    // Card grouping information for improved UI
    pub card_name: Option<String>,
    pub card_no: Option<String>,
    pub base_cost: Option<u32>,
    pub final_cost: Option<u32>,
    pub available_areas: Option<Vec<AreaInfo>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AreaInfo {
    pub area: String, // "left", "center", "right"
    pub available: bool,
    pub cost: u32,
    pub is_baton_touch: bool,
    pub existing_member_name: Option<String>,
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
                    use_baton_touch: None,
                    card_name: None,
                    card_no: None,
                    base_cost: None,
                    final_cost: None,
                    available_areas: None,
                }),
            });
            actions.push(Action {
                description: "Paper".to_string(),
                action_type: "rps_choice".to_string(),
                parameters: Some(ActionParameters {
                    card_index: Some(1), // 1 = paper
                    card_indices: None,
                    stage_area: Some("paper".to_string()),
                    use_baton_touch: None,
                    card_name: None,
                    card_no: None,
                    base_cost: None,
                    final_cost: None,
                    available_areas: None,
                }),
            });
            actions.push(Action {
                description: "Scissors".to_string(),
                action_type: "rps_choice".to_string(),
                parameters: Some(ActionParameters {
                    card_index: Some(2), // 2 = scissors
                    card_indices: None,
                    stage_area: Some("scissors".to_string()),
                    use_baton_touch: None,
                    card_name: None,
                    card_no: None,
                    base_cost: None,
                    final_cost: None,
                    available_areas: None,
                }),
            });
        }
        crate::game_state::Phase::Mulligan => {
            // Rule 6.2.1.6: Mulligan - player chooses cards to mulligan
            // Generate actions for the current mulligan player only
            let mulligan_player = if game_state.current_mulligan_player == "player1" {
                &game_state.player1
            } else {
                &game_state.player2
            };
            
            let player_name = if game_state.current_mulligan_player == "player1" {
                "Player 1"
            } else {
                "Player 2"
            };
            
            // Add header action to show whose turn it is
            actions.push(Action {
                description: format!("{}'s Mulligan Phase", player_name),
                action_type: "mulligan_header".to_string(),
                parameters: None,
            });
            
            // Generate actions for each card in hand to select/deselect for mulligan
            for (hand_index, card) in mulligan_player.hand.cards.iter().enumerate() {
                let is_selected = game_state.mulligan_selected_indices.contains(&hand_index);
                actions.push(Action {
                    description: format!("{} {} for mulligan", if is_selected { "Deselect" } else { "Select" }, card.name),
                    action_type: "select_mulligan".to_string(),
                    parameters: Some(ActionParameters {
                        card_index: Some(hand_index),
                        card_indices: None,
                        stage_area: None,
                        use_baton_touch: None,
                        card_name: None,
                        card_no: None,
                        base_cost: None,
                        final_cost: None,
                        available_areas: None,
                    }),
                });
            }
            
            // Add action to confirm mulligan selection
            actions.push(Action {
                description: format!("Confirm {}'s mulligan", player_name),
                action_type: "confirm_mulligan".to_string(),
                parameters: Some(ActionParameters {
                    card_index: None,
                    card_indices: Some(vec![]), // Will use tracked indices from game state
                    stage_area: None,
                    use_baton_touch: None,
                    card_name: None,
                    card_no: None,
                    base_cost: None,
                    final_cost: None,
                    available_areas: None,
                }),
            });
            
            // Add action to skip mulligan
            actions.push(Action {
                description: format!("Skip {}'s mulligan (keep all cards)", player_name),
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
            // Group actions by card with area information for improved UI
            // Only generate member cards (not live cards) - live cards are used in live phase
            for (hand_index, card) in active_player.hand.cards.iter().enumerate() {
                if card.is_member() && !card.is_live() {
                    let card_cost = card.cost.unwrap_or(0);
                    let active_energy_count = active_player.energy_zone.cards.iter()
                        .filter(|c| c.orientation == Some(crate::zones::Orientation::Active))
                        .count() as u32;
                    
                    // Check which areas are available
                    let areas = [
                        (crate::zones::MemberArea::LeftSide, "left"),
                        (crate::zones::MemberArea::Center, "center"),
                        (crate::zones::MemberArea::RightSide, "right"),
                    ];
                    
                    let mut available_areas = Vec::new();
                    let mut has_any_available = false;
                    
                    for (area, area_name) in areas {
                        let mut area_info = AreaInfo {
                            area: area_name.to_string(),
                            available: false,
                            cost: card_cost,
                            is_baton_touch: false,
                            existing_member_name: None,
                        };
                        
                        // Check if area is occupied for baton touch
                        if let Some(existing_member) = active_player.stage.get_area(area) {
                            // Rule 9.6.2.1.2.1: Cannot baton touch to an area that had a card moved from non-stage to stage this turn
                            if active_player.areas_locked_this_turn.contains(&area) {
                                // Area locked, not available
                            } else {
                                // Baton touch - replace existing member
                                // Rule 9.6.2.3.2: Baton touch requires 1+ active energy to trigger
                                let is_wait_state = existing_member.orientation != Some(crate::zones::Orientation::Active);
                                
                                if active_energy_count >= 1 && !is_wait_state {
                                    let member_cost = existing_member.card.cost.unwrap_or(0);
                                    let cost_to_pay = card_cost.saturating_sub(member_cost);
                                    
                                    if active_energy_count >= cost_to_pay {
                                        area_info.available = true;
                                        area_info.cost = cost_to_pay;
                                        area_info.is_baton_touch = true;
                                        area_info.existing_member_name = Some(existing_member.card.name.clone());
                                        has_any_available = true;
                                    }
                                }
                            }
                        } else {
                            // Play to empty area
                            if active_energy_count >= card_cost {
                                area_info.available = true;
                                area_info.cost = card_cost;
                                has_any_available = true;
                            }
                        }
                        
                        available_areas.push(area_info);
                    }
                    
                    // Only add card action if at least one area is available
                    if has_any_available {
                        actions.push(Action {
                            description: format!("{} ({}) - Cost: {}", card.name, card.card_no, card_cost),
                            action_type: "play_member_to_stage".to_string(),
                            parameters: Some(ActionParameters {
                                card_index: Some(hand_index),
                                card_indices: None,
                                stage_area: None, // Will be selected from available_areas
                                use_baton_touch: None, // Web app will set based on selected area's is_baton_touch
                                card_name: Some(card.name.clone()),
                                card_no: Some(card.card_no.clone()),
                                base_cost: Some(card_cost),
                                final_cost: None, // Will be determined by area selection
                                available_areas: Some(available_areas),
                            }),
                        });
                    }
                }
            }
            
            // Check stage cards for abilities that can be activated
            let stage_positions = [
                (&active_player.stage.center, crate::zones::MemberArea::Center, "center"),
                (&active_player.stage.left_side, crate::zones::MemberArea::LeftSide, "left"),
                (&active_player.stage.right_side, crate::zones::MemberArea::RightSide, "right"),
            ];
            
            for (card_in_zone_opt, _area, area_name) in stage_positions {
                if let Some(card_in_zone) = card_in_zone_opt {
                    let card = &card_in_zone.card;
                    for (_ability_index, ability) in card.abilities.iter().enumerate() {
                        // Check if ability can be activated (has main phase trigger)
                        // triggers is a String field, check if it contains "main"
                        let can_activate = ability.triggers.as_ref().map_or(false, |t| {
                            t.contains("main") || t.contains("メイン")
                        });
                        
                        if can_activate {
                            let _ability_name = if ability.full_text.len() > 30 {
                                format!("{}...", &ability.full_text[..30])
                            } else {
                                ability.full_text.clone()
                            };
                            let ability_cost = ability.cost.as_ref().and_then(|c| c.energy).unwrap_or(0);
                            
                            actions.push(Action {
                                description: format!("Use ability on {} ({})", card.name, area_name),
                                action_type: "use_ability".to_string(),
                                parameters: Some(ActionParameters {
                                    card_index: None,
                                    card_indices: None,
                                    stage_area: Some(area_name.to_string()),
                                    use_baton_touch: None,
                                    card_name: Some(card.name.clone()),
                                    card_no: Some(card.card_no.clone()),
                                    base_cost: Some(ability_cost),
                                    final_cost: Some(ability_cost),
                                    available_areas: None,
                                }),
                            });
                        }
                    }
                }
            }
        }
        crate::game_state::Phase::LiveCardSet => {
            // Rule 8.2: Live Card Set Phase - Can place up to 3 cards face-down
            // Allow individual card selection (any card from hand, not just live cards)
            let cards_in_hand: Vec<_> = active_player.hand.cards.iter()
                .enumerate()
                .collect();
            
            let current_live_count = active_player.live_card_zone.cards.len();
            let can_add_more = current_live_count < 3;
            
            // Always show finish action
            actions.push(Action {
                description: "Finish live card set".to_string(),
                action_type: "place_live_cards".to_string(),
                parameters: Some(ActionParameters {
                    card_index: None,
                    card_indices: None,
                    stage_area: None,
                    use_baton_touch: None,
                    card_name: None,
                    card_no: None,
                    base_cost: None,
                    final_cost: None,
                    available_areas: None,
                }),
            });
            
            if can_add_more {
                // Generate individual card selection actions
                for (hand_index, card) in cards_in_hand {
                    actions.push(Action {
                        description: format!("Place {} ({}) to live zone", card.name, card.card_no),
                        action_type: "place_live_cards".to_string(),
                        parameters: Some(ActionParameters {
                            card_index: Some(hand_index),
                            card_indices: Some(vec![hand_index]),
                            stage_area: None,
                            use_baton_touch: None,
                            card_name: None,
                            card_no: None,
                            base_cost: None,
                            final_cost: None,
                            available_areas: None,
                        }),
                    });
                }
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

