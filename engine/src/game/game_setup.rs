// Game setup and initialization functions
// This module contains shared game setup logic used by both the web server and bot modules

use crate::game_state::GameState;
use crate::zones::MemberArea;

use crate::ability_resolver::Choice;
use serde::{Serialize, Deserialize};
use std::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    Pass,
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
    // Choice action types for ability cost/effect prompts
    ChoiceDecision,
    ChoiceSelect,
    ChoiceSkip,
    ChoiceOption,
    ChoicePosition,
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ActionType::Pass => write!(f, "pass"),
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
            ActionType::ChoiceDecision => write!(f, "decision"),
            ActionType::ChoiceSelect => write!(f, "select_card"),
            ActionType::ChoiceSkip => write!(f, "select_skip"),
            ActionType::ChoiceOption => write!(f, "choose_option"),
            ActionType::ChoicePosition => write!(f, "select_position"),
        }
    }
}

impl std::str::FromStr for ActionType {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pass" => Ok(ActionType::Pass),
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
            "decision" => Ok(ActionType::ChoiceDecision),
            "select_card" => Ok(ActionType::ChoiceSelect),
            "select_skip" => Ok(ActionType::ChoiceSkip),
            "choose_option" => Ok(ActionType::ChoiceOption),
            "select_position" => Ok(ActionType::ChoicePosition),
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

pub fn generate_possible_actions(game_state: &GameState) -> Vec<Action> {
    let _start = std::time::Instant::now();
    let mut actions = Vec::new();
    
    // If there's a pending choice, generate choice-resolution actions instead of phase actions
    if let Some(choice) = game_state.get_pending_choice() {
        match choice {
            Choice::SelectTarget { target, description } => {
                if target == "pay_optional_cost:skip_optional_cost" {
                    actions.push(Action {
                        description: "Pay optional cost".to_string(),
                        action_type: ActionType::ChoiceDecision,
                        parameters: Some(ActionParameters {
                            card_id: Some(1), card_index: None, card_indices: None,
                            stage_area: None, use_baton_touch: None,
                            card_name: None, card_no: Some("pay_optional_cost".to_string()),
                            base_cost: None, final_cost: None, available_areas: None,
                        }),
                    });
                    actions.push(Action {
                        description: "Skip optional cost".to_string(),
                        action_type: ActionType::ChoiceDecision,
                        parameters: Some(ActionParameters {
                            card_id: Some(0), card_index: None, card_indices: None,
                            stage_area: None, use_baton_touch: None,
                            card_name: None, card_no: Some("skip_optional_cost".to_string()),
                            base_cost: None, final_cost: None, available_areas: None,
                        }),
                    });
                    return actions;
                }
                if target == "choice" {
                    let options: Vec<&str> = description.split(" / ").collect();
                    for (i, opt) in options.iter().enumerate() {
                        actions.push(Action {
                            description: opt.to_string(),
                            action_type: ActionType::ChoiceOption,
                            parameters: Some(ActionParameters {
                                card_id: Some(i as i16), card_index: None, card_indices: None,
                                stage_area: None, use_baton_touch: None,
                                card_name: None, card_no: Some(i.to_string()),
                                base_cost: None, final_cost: None, available_areas: None,
                            }),
                        });
                    }
                    return actions;
                }
                if target == "primary|alternative" {
                    actions.push(Action {
                        description: format!("Primary: {}", description),
                        action_type: ActionType::ChoiceOption,
                        parameters: Some(ActionParameters {
                            card_id: Some(0), card_index: None, card_indices: None,
                            stage_area: None, use_baton_touch: None,
                            card_name: None, card_no: Some("primary".to_string()),
                            base_cost: None, final_cost: None, available_areas: None,
                        }),
                    });
                    actions.push(Action {
                        description: format!("Alternative: {}", description),
                        action_type: ActionType::ChoiceOption,
                        parameters: Some(ActionParameters {
                            card_id: Some(1), card_index: None, card_indices: None,
                            stage_area: None, use_baton_touch: None,
                            card_name: None, card_no: Some("alternative".to_string()),
                            base_cost: None, final_cost: None, available_areas: None,
                        }),
                    });
                    return actions;
                }
                // Generic SelectTarget: yes/no
                actions.push(Action {
                    description: format!("Yes — {}", description),
                    action_type: ActionType::ChoiceDecision,
                    parameters: Some(ActionParameters {
                        card_id: Some(1), card_index: None, card_indices: None,
                        stage_area: None, use_baton_touch: None,
                        card_name: None, card_no: Some("yes".to_string()),
                        base_cost: None, final_cost: None, available_areas: None,
                    }),
                });
                actions.push(Action {
                    description: format!("No — {}", description),
                    action_type: ActionType::ChoiceDecision,
                    parameters: Some(ActionParameters {
                        card_id: Some(0), card_index: None, card_indices: None,
                        stage_area: None, use_baton_touch: None,
                        card_name: None, card_no: Some("no".to_string()),
                        base_cost: None, final_cost: None, available_areas: None,
                    }),
                });
                return actions;
            }
            Choice::SelectCard { zone, card_type, count: _, description, allow_skip } => {
                let active = game_state.active_player();
                let card_ids: Vec<(usize, i16)> = match zone.as_str() {
                    "hand" => active.hand.cards.iter().copied().enumerate().map(|(i, id)| (i, id)).collect(),
                    "discard" => active.waitroom.cards.iter().copied().enumerate().map(|(i, id)| (i, id)).collect(),
                    "stage" => active.stage.stage.iter().copied().enumerate().filter(|&(_, id)| id != -1).map(|(i, id)| (i, id)).collect(),
                    "energy_zone" => active.energy_zone.cards.iter().copied().enumerate().map(|(i, id)| (i, id)).collect(),
                    "looked_at" => game_state.looked_at_cards.iter().copied().enumerate().map(|(i, id)| (i, id)).collect(),
                    _ => Vec::new(),
                };
                if !card_ids.is_empty() {
                    for (zone_index, card_id) in &card_ids {
                        let card_matches = match card_type.as_deref() {
                            Some("member_card") => game_state.card_database.get_card(*card_id).map(|c| c.is_member()).unwrap_or(false),
                            Some("live_card") => game_state.card_database.get_card(*card_id).map(|c| c.is_live()).unwrap_or(false),
                            Some("energy_card") => game_state.card_database.get_card(*card_id).map(|c| c.is_energy()).unwrap_or(false),
                            None => true,
                            _ => true,
                        };
                        if !card_matches { continue; }
                        let card_name = game_state.card_database.get_card(*card_id).map(|c| c.name.as_str()).unwrap_or("Unknown");
                        actions.push(Action {
                            description: format!("{} ({})", card_name, zone_index),
                            action_type: ActionType::ChoiceSelect,
                            parameters: Some(ActionParameters {
                                card_id: Some(*card_id), card_index: Some(*zone_index), card_indices: Some(vec![*zone_index]),
                                stage_area: None, use_baton_touch: None,
                                card_name: Some(card_name.to_string()), card_no: Some("select".to_string()),
                                base_cost: None, final_cost: None, available_areas: None,
                            }),
                        });
                    }
                } else {
                    actions.push(Action {
                        description: format!("Select card(s): {}", description),
                        action_type: ActionType::ChoiceSelect,
                        parameters: Some(ActionParameters {
                            card_id: None, card_index: None, card_indices: Some(Vec::new()),
                            stage_area: None, use_baton_touch: None,
                            card_name: None, card_no: Some("select".to_string()),
                            base_cost: None, final_cost: None, available_areas: None,
                        }),
                    });
                }
                if *allow_skip {
                    actions.push(Action {
                        description: "Skip".to_string(),
                        action_type: ActionType::ChoiceSkip,
                        parameters: Some(ActionParameters {
                            card_id: None, card_index: None, card_indices: None,
                            stage_area: None, use_baton_touch: None,
                            card_name: None, card_no: Some("skip".to_string()),
                            base_cost: None, final_cost: None, available_areas: None,
                        }),
                    });
                }
                return actions;
            }
            Choice::SelectPosition { position, description } => {
                let areas: Vec<&str> = position.split(',').map(|s| s.trim()).collect();
                for area in areas {
                    let stage_area_str = match area {
                        "left" | "left_side" | "左サイドエリア" => Some("left".to_string()),
                        "center" | "センターエリア" => Some("center".to_string()),
                        "right" | "right_side" | "右サイドエリア" => Some("right".to_string()),
                        _ => Some(area.to_string()),
                    };
                    let area_parsed = stage_area_str.as_deref().and_then(|s| s.parse::<MemberArea>().ok());
                    actions.push(Action {
                        description: format!("Place at {}: {}", area, description),
                        action_type: ActionType::ChoicePosition,
                        parameters: Some(ActionParameters {
                            card_id: None, card_index: None, card_indices: None,
                            stage_area: area_parsed, use_baton_touch: None,
                            card_name: None, card_no: Some("select".to_string()),
                            base_cost: None, final_cost: None, available_areas: None,
                        }),
                    });
                }
                return actions;
            }
            Choice::SelectHeartColor { count: _, options, description } => {
                for (i, color) in options.iter().enumerate() {
                    actions.push(Action {
                        description: format!("{} — {}", color, description),
                        action_type: ActionType::ChoiceOption,
                        parameters: Some(ActionParameters {
                            card_id: Some(i as i16), card_index: None, card_indices: None,
                            stage_area: None, use_baton_touch: None,
                            card_name: None, card_no: Some(color.clone()),
                            base_cost: None, final_cost: None, available_areas: None,
                        }),
                    });
                }
                return actions;
            }
            Choice::SelectHeartType { count: _, options, description } => {
                for (i, color) in options.iter().enumerate() {
                    actions.push(Action {
                        description: format!("{} — {}", color, description),
                        action_type: ActionType::ChoiceOption,
                        parameters: Some(ActionParameters {
                            card_id: Some(i as i16), card_index: None, card_indices: None,
                            stage_area: None, use_baton_touch: None,
                            card_name: None, card_no: Some(color.clone()),
                            base_cost: None, final_cost: None, available_areas: None,
                        }),
                    });
                }
                return actions;
            }
        }
    }

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
        crate::game_state::Phase::LiveStart => {
            // Live start phase - currently no specific actions
        }
        crate::game_state::Phase::LiveSuccess => {
            // Live success phase - currently no specific actions
        }
        crate::game_state::Phase::Cheer => {
            // Cheer phase - currently no specific actions
        }
        crate::game_state::Phase::ChooseFirstAttacker => {
            // Q16: RPS winner chooses whether to go first or second
            let _rps_winner = game_state.rps_winner.unwrap_or(1);
            println!("DEBUG: ChooseFirstAttacker phase, rps_winner: {:?}", game_state.rps_winner);
            
            actions.push(Action {
                description: "Go first".to_string(),
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
                description: "Go second".to_string(),
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
            
            println!("DEBUG: ChooseFirstAttacker actions generated: {} actions", actions.len());
        }
        crate::game_state::Phase::MulliganP1Turn |
        crate::game_state::Phase::MulliganP2Turn => {
            let mulligan_player = match game_state.current_phase {
                crate::game_state::Phase::MulliganP1Turn => &game_state.player1,
                crate::game_state::Phase::MulliganP2Turn => &game_state.player2,
                _ => &game_state.player1,
            };

            let player_name = match game_state.current_phase {
                crate::game_state::Phase::MulliganP1Turn => "Player 1",
                crate::game_state::Phase::MulliganP2Turn => "Player 2",
                _ => "Player 1",
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
                            let hand_count = active_player.hand.cards.len();
                            let hand_reduction = card.get_hand_cost_reduction(hand_count);
                            let effective_cost = card_cost.saturating_sub(hand_reduction);
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
                                } else if game_state.baton_touch_count >= 1 {
                                    // Baton touch already used this turn - only allowed once per turn
                                } else {
                                    // Baton touch - replace existing member
                                    // Rule 9.6.2.3.2: Baton touch requires sufficient energy for the final cost
                                    let member_cost = if let Some(existing_card) = game_state.card_database.get_card(existing_member_id) {
                                        existing_card.cost.unwrap_or(0)
                                    } else {
                                        0
                                    };
                                    let cost_to_pay = effective_cost.saturating_sub(member_cost);

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
                            } else {
                                // Play to empty area
                                if (active_energy_count as u32) >= effective_cost {
                                    area_info.available = true;
                                    area_info.cost = effective_cost;
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
                        let card_position: MemberArea = area_name.parse().unwrap_or(MemberArea::Center);
                        for (ability_index, ability) in card.abilities.iter().enumerate() {
                            // Check if ability can be activated (has activation trigger or main phase trigger)
                            // Only 起動 (activation), メイン (main), 自動 (auto) with cost, and baton touch
                            // are player-activatable. 常時 (constant) is passive — applies automatically.
                            let can_activate = ability.triggers.as_ref().map_or(false, |t| {
                                t.contains("main") || t.contains(crate::triggers::MAIN) || t.contains(crate::triggers::ACTIVATION)
                                || (t.contains(crate::triggers::AUTO) && ability.cost.is_some())
                                || t.contains(crate::triggers::BATON_TOUCH)
                            });

                            // Check position requirement (左サイド/右サイド/センター in trigger string)
                            if !crate::zones::check_trigger_position(ability.triggers.as_deref(), card_position) {
                                continue;
                            }
                            // Also check activation_position from parsed effect data
                            if !crate::zones::check_effect_position(
                                ability.effect.as_ref().and_then(|e| e.activation_position.as_deref()),
                                card_position,
                            ) {
                                continue;
                            }

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
        crate::game_state::Phase::LiveCardSetP1Turn |
        crate::game_state::Phase::LiveCardSetP2Turn => {
            actions.push(Action {
                description: "Pass - Finished setting live cards".to_string(),
                action_type: ActionType::Pass,
                parameters: None,
            });

            // Use the consolidated active_player() method to determine which player is currently setting cards
            let active_player = game_state.active_player();

            // Allow individual card selection (any card from hand, not just live cards)
            let cards_in_hand: Vec<_> = active_player.hand.cards.iter()
                .enumerate()
                .collect();

            let current_live_count = active_player.live_card_zone.cards.len();
            let can_add_more = current_live_count < 3;

            if can_add_more && !game_state.is_action_prohibited("cannot_live") {
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
        }
        crate::game_state::Phase::FirstAttackerPerformance
        | crate::game_state::Phase::SecondAttackerPerformance
        | crate::game_state::Phase::LiveVictoryDetermination => {
            // Live phase actions - currently no specific actions
        }
    }
    
    actions
}

