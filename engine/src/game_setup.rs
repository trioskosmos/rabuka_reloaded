// Game setup and initialization functions
// This module contains shared game setup logic used by both the web server and bot modules

use crate::game_state::GameState;
use crate::player::Player;
use crate::zones::MemberArea;
use serde::{Serialize, Deserialize};
use std::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    RockChoice,        // Q16: RPS - choose Rock
    PaperChoice,       // Q16: RPS - choose Paper
    ScissorsChoice,    // Q16: RPS - choose Scissors
    ChooseFirstAttacker,  // Q16: RPS winner chooses to go first
    ChooseSecondAttacker, // Q16: RPS winner chooses to go second
    MulliganHeader,
    SelectMulligan,
    ConfirmMulligan,
    SkipMulligan,
    PlayMemberToStage,
    UseAbility,
    SetLiveCard,
    FinishLiveCardSet,
    Pass,
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ActionType::RockChoice => write!(f, "rock_choice"),
            ActionType::PaperChoice => write!(f, "paper_choice"),
            ActionType::ScissorsChoice => write!(f, "scissors_choice"),
            ActionType::ChooseFirstAttacker => write!(f, "choose_first_attacker"),
            ActionType::ChooseSecondAttacker => write!(f, "choose_second_attacker"),
            ActionType::MulliganHeader => write!(f, "mulligan_header"),
            ActionType::SelectMulligan => write!(f, "select_mulligan"),
            ActionType::ConfirmMulligan => write!(f, "confirm_mulligan"),
            ActionType::SkipMulligan => write!(f, "skip_mulligan"),
            ActionType::PlayMemberToStage => write!(f, "play_member_to_stage"),
            ActionType::UseAbility => write!(f, "use_ability"),
            ActionType::SetLiveCard => write!(f, "set_live_card"),
            ActionType::FinishLiveCardSet => write!(f, "finish_live_card_set"),
            ActionType::Pass => write!(f, "pass"),
        }
    }
}

impl std::str::FromStr for ActionType {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rock_choice" => Ok(ActionType::RockChoice),
            "paper_choice" => Ok(ActionType::PaperChoice),
            "scissors_choice" => Ok(ActionType::ScissorsChoice),
            "choose_first_attacker" => Ok(ActionType::ChooseFirstAttacker),
            "choose_second_attacker" => Ok(ActionType::ChooseSecondAttacker),
            "mulligan_header" => Ok(ActionType::MulliganHeader),
            "select_mulligan" => Ok(ActionType::SelectMulligan),
            "confirm_mulligan" => Ok(ActionType::ConfirmMulligan),
            "skip_mulligan" => Ok(ActionType::SkipMulligan),
            "play_member_to_stage" => Ok(ActionType::PlayMemberToStage),
            "use_ability" => Ok(ActionType::UseAbility),
            "set_live_card" => Ok(ActionType::SetLiveCard),
            "finish_live_card_set" => Ok(ActionType::FinishLiveCardSet),
            "pass" => Ok(ActionType::Pass),
            _ => Err(format!("Invalid action type: {}", s)),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Action {
    pub description: String,
    pub action_type: ActionType,
    pub parameters: Option<ActionParameters>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ActionParameters {
    pub card_id: Option<i16>, // Database card ID - reliable identifier
    pub card_index: Option<usize>, // Array position - kept for backward compatibility
    pub card_indices: Option<Vec<usize>>, // For selecting multiple cards (e.g., live cards)
    pub stage_area: Option<MemberArea>, // "left", "center", "right"
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
    pub area: MemberArea,
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
    perform_player_mulligan(first_player, &game_state.card_database);
    perform_player_mulligan(second_player, &game_state.card_database);
}

#[allow(dead_code)]
fn perform_player_mulligan(player: &mut Player, card_db: &crate::card::CardDatabase) {
    use rand::seq::SliceRandom;
    
    // Rule 6.2.1.6: Starting from first attacker, each player chooses any number of cards from hand
    // Simple rule-based strategy: mulligan if hand has no member cards
    let has_member = player.hand.cards.iter().any(|&id| {
        if let Some(card) = card_db.get_card(id) {
            !card.is_energy() && !card.is_live()
        } else {
            false
        }
    });
    
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
            player.main_deck.cards.push(card);
        }
        
        // Rule 6.2.1.6: Shuffle if 1+ cards were moved
        let mut deck_vec: Vec<_> = player.main_deck.cards.drain(..).collect();
        deck_vec.shuffle(&mut rand::thread_rng());
        for card in deck_vec {
            player.main_deck.cards.push(card);
        }
        
        println!("Player mulliganed {} cards", num_to_mulligan);
    } else {
        println!("Player kept their hand");
    }
}

pub fn generate_possible_actions(game_state: &GameState) -> Vec<Action> {
    let _start = std::time::Instant::now();
    let mut actions = Vec::new();
    let active_player = game_state.active_player();
    
    match game_state.current_phase {
        crate::game_state::Phase::RockPaperScissors => {
            // Q16 from qa_data.json: "じゃんけんで勝ったプレイヤーが先攻か後攻を決めます"
            // Generate actions for player 1 to choose RPS option
            actions.push(Action {
                description: "Rock".to_string(),
                action_type: ActionType::RockChoice,
                parameters: None,
            });
            actions.push(Action {
                description: "Paper".to_string(),
                action_type: ActionType::PaperChoice,
                parameters: None,
            });
            actions.push(Action {
                description: "Scissors".to_string(),
                action_type: ActionType::ScissorsChoice,
                parameters: None,
            });
        }
        crate::game_state::Phase::ChooseFirstAttacker => {
            // Q16: RPS winner chooses whether to go first or second
            let rps_winner = game_state.rps_winner.unwrap_or(1);
            let winner_name = if rps_winner == 1 { "Player 1" } else { "Player 2" };
            
            actions.push(Action {
                description: format!("{} goes first", winner_name),
                action_type: ActionType::ChooseFirstAttacker,
                parameters: Some(ActionParameters {
                    card_id: None,
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
            actions.push(Action {
                description: format!("{} goes second", winner_name),
                action_type: ActionType::ChooseSecondAttacker,
                parameters: Some(ActionParameters {
                    card_id: None,
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
                action_type: ActionType::MulliganHeader,
                parameters: None,
            });
            
            // Generate actions for each card in hand to select/deselect for mulligan
            for (hand_index, card_id) in mulligan_player.hand.cards.iter().enumerate() {
                let is_selected = game_state.mulligan_selected_indices.contains(&hand_index);
                let card_name = if let Some(card) = game_state.card_database.get_card(*card_id) {
                    card.name.clone()
                } else {
                    format!("Unknown card {}", card_id)
                };
                actions.push(Action {
                    description: format!("{} {} for mulligan", if is_selected { "Deselect" } else { "Select" }, card_name),
                    action_type: ActionType::SelectMulligan,
                    parameters: Some(ActionParameters {
                        card_id: Some(*card_id),
                        card_index: Some(hand_index),
                        card_indices: None,
                        stage_area: Some(MemberArea::LeftSide),
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
                action_type: ActionType::ConfirmMulligan,
                parameters: Some(ActionParameters {
                    card_id: None,
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
                action_type: ActionType::SkipMulligan,
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
                action_type: ActionType::Pass,
                parameters: None,
            });
            
            // Check if playing member cards is prohibited
            if !game_state.is_action_prohibited("play_member") {
                // Rule 7.7.2.2: Main Phase - Can play member cards to stage
                // Group actions by card with area information for improved UI
                // Only generate member cards (not live cards) - live cards are used in live phase
                
                // Pre-allocate actions Vec with estimated capacity
                let estimated_actions = active_player.hand.cards.len() * 3 + 1; // Each card can have up to 3 area options + pass
                actions.reserve(estimated_actions);
                
                for (hand_index, card_id) in active_player.hand.cards.iter().enumerate() {
                    if let Some(card) = game_state.card_database.get_card(*card_id) {
                        if card.is_member() && !card.is_live() {
                            let card_cost = card.cost.unwrap_or(0);
                            // Use actual active energy count
                            let active_energy_count = active_player.energy_zone.active_count();
                        
                        // Check which areas are available
                        let areas = [
                            (crate::zones::MemberArea::LeftSide, "left"),
                            (crate::zones::MemberArea::Center, "center"),
                            (crate::zones::MemberArea::RightSide, "right"),
                        ];
                        
                        let mut available_areas = Vec::with_capacity(3);
                        let mut has_any_available = false;
                        
                        // Cache stage card lookups to avoid repeated database queries
                        let stage_card_ids = [
                            active_player.stage.stage[0],
                            active_player.stage.stage[1],
                            active_player.stage.stage[2],
                        ];
                        
                        for (area_idx, (area, _area_name)) in areas.iter().enumerate() {
                            let mut area_info = AreaInfo {
                                area: *area,
                                available: false,
                                cost: card_cost,
                                is_baton_touch: false,
                                existing_member_name: None,
                            };
                            
                            // Check if area is occupied for baton touch
                            if stage_card_ids[area_idx] != -1 {
                                let existing_member_id = stage_card_ids[area_idx];
                                // Rule 9.6.2.1.2.1: Cannot baton touch to an area that had a card moved from non-stage to stage this turn
                                if active_player.areas_locked_this_turn.contains(area) {
                                    // Area locked, not available
                                } else {
                                    // Baton touch - replace existing member
                                    // Rule 9.6.2.3.2: Baton touch requires 1+ active energy to trigger
                                    if active_energy_count >= 1 {
                                        let member_cost = if let Some(existing_card) = game_state.card_database.get_card(existing_member_id) {
                                            existing_card.cost.unwrap_or(0)
                                        } else {
                                            0
                                        };
                                        let cost_to_pay = card_cost.saturating_sub(member_cost);

                                        if (active_energy_count as u32) >= cost_to_pay {
                                            area_info.available = true;
                                            area_info.cost = cost_to_pay;
                                            area_info.is_baton_touch = true;
                                            area_info.existing_member_name = if let Some(existing_card) = game_state.card_database.get_card(existing_member_id) {
                                                Some(existing_card.name.clone())
                                            } else {
                                                Some(format!("Unknown card {}", existing_member_id))
                                            };
                                            has_any_available = true;
                                        }
                                    }
                                }
                            } else {
                                // Play to empty area
                                if (active_energy_count as u32) >= card_cost {
                                    area_info.available = true;
                                    area_info.cost = card_cost;
                                    has_any_available = true;
                                }
                            }
                            
                            available_areas.push(area_info);
                        }
                        
                        // Only add card action if at least one area is available
                        if has_any_available {
                            // Build description with cost details
                            let mut cost_details = Vec::with_capacity(available_areas.len());
                            for area in &available_areas {
                                if area.available {
                                    let area_name = match area.area {
                                        crate::zones::MemberArea::LeftSide => "Left",
                                        crate::zones::MemberArea::Center => "Center",
                                        crate::zones::MemberArea::RightSide => "Right",
                                    };
                                    if area.is_baton_touch {
                                        cost_details.push(format!("{}: {} (baton touch from {})", area_name, area.cost, area.existing_member_name.as_deref().unwrap_or("existing")));
                                    } else {
                                        cost_details.push(format!("{}: {}", area_name, area.cost));
                                    }
                                }
                            }
                            
                            let cost_str = if cost_details.is_empty() {
                                format!("Cost: {}", card_cost)
                            } else {
                                format!("Cost: {}", cost_details.join(", "))
                            };
                            
                            actions.push(Action {
                                description: format!("{} ({}) - {}", card.name, card.card_no, cost_str),
                                action_type: ActionType::PlayMemberToStage,
                                parameters: Some(ActionParameters {
                                    card_id: Some(*card_id), // Use actual card ID for reliable identification
                                    card_index: Some(hand_index), // Keep for backward compatibility
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
            }
            } // Close if has_any_available
            // Check stage cards for abilities that can be activated
            let stage_positions = [
                (active_player.stage.stage[0], "left"),
                (active_player.stage.stage[1], "center"),
                (active_player.stage.stage[2], "right"),
            ];

            for (card_id, area_name) in stage_positions {
                if card_id != -1 {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        for (ability_index, ability) in card.abilities.iter().enumerate() {
                            // Check if ability can be activated (has activation trigger or main phase trigger)
                            // triggers is a String field, check if it contains "main", "メイン", or "起動" (activation)
                            let can_activate = ability.triggers.as_ref().map_or(false, |t| {
                                t.contains("main") || t.contains("メイン") || t.contains("起動")
                            });

                            // Check use_limit (e.g., once per turn)
                            let ability_key = format!("{}_{}_{}", card_id, ability_index, game_state.turn_number);
                            let can_use = if let Some(_use_limit) = ability.use_limit {
                                // Check if this ability has already been used this turn
                                !game_state.turn_limited_abilities_used.contains(&ability_key)
                            } else {
                                true
                            };

                            if can_activate && can_use {
                                let ability_name = if ability.full_text.chars().count() > 30 {
                                    format!("{}...", ability.full_text.chars().take(30).collect::<String>())
                                } else {
                                    ability.full_text.clone()
                                };
                                let ability_cost = ability.cost.as_ref().and_then(|c| c.energy).unwrap_or(0);
                                let trigger_info = ability.triggers.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default();

                                actions.push(Action {
                                    description: format!("Use ability on {} ({}): {}{} - Cost: {}", card.name, area_name, ability_name, trigger_info, ability_cost),
                                    action_type: ActionType::UseAbility,
                                    parameters: Some(ActionParameters {
                                        card_id: Some(card_id),
                                        card_index: None,
                                        card_indices: None,
                                        stage_area: area_name.parse::<MemberArea>().ok(),
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
        }
        crate::game_state::Phase::LiveCardSet => {
            // Rule 8.2: Live Card Set Phase - Can place up to 3 cards face-down
            // Determine which player is currently taking their turn based on completion flags
            let p1_is_first = game_state.player1.is_first_attacker;
            let p1_done = game_state.live_card_set_player1_done;
            let p2_done = game_state.live_card_set_player2_done;
            
            // Determine the active player for live card set
            let active_player = if !p1_done && p2_done {
                // P1 is currently taking their turn (P2 already done)
                &game_state.player1
            } else if !p2_done && p1_done {
                // P2 is currently taking their turn (P1 already done)
                &game_state.player2
            } else if !p1_done && !p2_done {
                // Neither has finished yet - first attacker goes first
                if p1_is_first {
                    &game_state.player1
                } else {
                    &game_state.player2
                }
            } else {
                // Both done - don't generate any actions, let phase auto-advance
                return actions;
            };
            
            // Allow individual card selection (any card from hand, not just live cards)
            let cards_in_hand: Vec<_> = active_player.hand.cards.iter()
                .enumerate()
                .collect();
            
            let current_live_count = active_player.live_card_zone.cards.len();
            let can_add_more = current_live_count < 3;
            
            if can_add_more {
                // Generate individual card selection actions
                for (hand_index, card_id) in cards_in_hand {
                    let card_name = if let Some(card) = game_state.card_database.get_card(*card_id) {
                        card.name.clone()
                    } else {
                        format!("Unknown card {}", card_id)
                    };
                    let card_no = if let Some(card) = game_state.card_database.get_card(*card_id) {
                        card.card_no.clone()
                    } else {
                        format!("unknown:{}", card_id)
                    };
                    
                    actions.push(Action {
                        description: format!("Place {} ({}) to live zone", card_name, card_no),
                        action_type: ActionType::SetLiveCard,
                        parameters: Some(ActionParameters {
                            card_id: Some(*card_id),
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
            
            // Add action to finish live card set
            let player_name = if active_player.id == "player1" { "Player 1" } else { "Player 2" };
            actions.push(Action {
                description: format!("Finish {}'s live card set", player_name),
                action_type: ActionType::FinishLiveCardSet,
                parameters: None,
            });
        }
        crate::game_state::Phase::FirstAttackerPerformance
        | crate::game_state::Phase::SecondAttackerPerformance
        | crate::game_state::Phase::LiveVictoryDetermination => {
            // Live phase actions - currently no specific actions
        }
    }
    
    actions
}

