// Game setup and initialization functions
// This module contains shared game setup logic used by both the web server and bot modules

use crate::game_state::GameState;
use crate::zones::MemberArea;

use crate::ability::types::Choice;
use serde::{Deserialize, Serialize};
use std::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    Pass,
    RockChoice,           // Q16: RPS - choose Rock
    PaperChoice,          // Q16: RPS - choose Paper
    ScissorsChoice,       // Q16: RPS - choose Scissors
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActionParameters {
    pub card_id: Option<i16>,      // Database card ID - reliable identifier
    pub card_index: Option<usize>, // Array position - kept for backward compatibility
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AreaInfo {
    pub area: String,
    pub available: bool,
    pub cost: u32,
    pub is_baton_touch: bool,
    pub existing_member_name: Option<String>,
}

impl ActionParameters {
    pub fn stage_area_member(&self) -> Option<MemberArea> {
        self.stage_area
            .as_ref()
            .and_then(|s| s.parse::<MemberArea>().ok())
    }
}

pub fn setup_game(game_state: &mut GameState) {
    // Rule 6.2: Pre-Game Procedure
    // Rule 6.2.1.7: Each player moves top 3 cards of energy deck to energy zone
    crate::turn::TurnEngine::setup_initial_energy(game_state);
    // Start at RockPaperScissors phase - player will choose RPS option
    game_state.current_phase = crate::game_state::Phase::RockPaperScissors;
}

fn make_action(action_type: ActionType, description: &str) -> Action {
    Action {
        description: description.to_string(),
        action_type,
        parameters: None,
    }
}

fn make_action_params(
    action_type: ActionType,
    description: &str,
    params: ActionParameters,
) -> Action {
    Action {
        description: description.to_string(),
        action_type,
        parameters: Some(params),
    }
}

fn make_params() -> ActionParameters {
    ActionParameters {
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
    }
}

pub fn generate_possible_actions(game_state: &GameState) -> Vec<Action> {
    if let Some(choice) = game_state.get_pending_choice() {
        return generate_pending_choice_actions(game_state, choice);
    }

    match game_state.current_phase {
        crate::game_state::Phase::RockPaperScissors => generate_rps_actions(),
        crate::game_state::Phase::ChooseFirstAttacker => {
            generate_choose_first_attacker_actions(game_state)
        }
        crate::game_state::Phase::MulliganP1Turn | crate::game_state::Phase::MulliganP2Turn => {
            generate_mulligan_actions(game_state)
        }
        crate::game_state::Phase::Active
        | crate::game_state::Phase::Energy
        | crate::game_state::Phase::Draw => Vec::new(),
        crate::game_state::Phase::Main => generate_main_phase_actions(game_state),
        crate::game_state::Phase::LiveCardSetP1Turn
        | crate::game_state::Phase::LiveCardSetP2Turn => generate_live_card_set_actions(game_state),
        crate::game_state::Phase::FirstAttackerPerformance
        | crate::game_state::Phase::SecondAttackerPerformance
        | crate::game_state::Phase::LiveVictoryDetermination => Vec::new(),
    }
}

fn generate_pending_choice_actions(game_state: &GameState, choice: &Choice) -> Vec<Action> {
    match choice {
        Choice::SelectTarget {
            target,
            description,
            allow_skip,
            options,
            ..
        } => {
            if target == "pay_optional_cost:skip_optional_cost" {
                return vec![
                    make_action_params(
                        ActionType::ChoiceDecision,
                        "Pay optional cost",
                        ActionParameters {
                            card_id: Some(1),
                            card_no: Some("pay_optional_cost".to_string()),
                            ..make_params()
                        },
                    ),
                    make_action_params(
                        ActionType::ChoiceDecision,
                        "Skip optional cost",
                        ActionParameters {
                            card_id: Some(0),
                            card_no: Some("skip_optional_cost".to_string()),
                            ..make_params()
                        },
                    ),
                ];
            }
            if target == "position|destination" {
                let default_positions = vec!["left".into(), "center".into(), "right".into()];
                let positions = options.as_deref().unwrap_or(&default_positions);
                let mut actions: Vec<Action> = positions
                    .iter()
                    .map(|pos| {
                        let (label, stage_area, card_id) = match pos.as_str() {
                            "left_side" | "left" => ("Move to Left", "left", 0),
                            "center" => ("Move to Center", "center", 1),
                            "right_side" | "right" => ("Move to Right", "right", 2),
                            _ => ("Move", pos.as_str(), -1),
                        };
                        make_action_params(
                            ActionType::ChoicePosition,
                            label,
                            ActionParameters {
                                card_id: Some(card_id),
                                stage_area: Some(stage_area.to_string()),
                                card_no: Some("select".to_string()),
                                ..make_params()
                            },
                        )
                    })
                    .collect();
                if *allow_skip {
                    actions.push(make_action_params(
                        ActionType::ChoiceSkip,
                        "Skip (don't change position)",
                        ActionParameters {
                            card_id: Some(-1),
                            card_no: Some("skip".to_string()),
                            ..make_params()
                        },
                    ));
                }
                return actions;
            }
            if target == "draw_any_number" {
                let max_count = description
                    .matches(char::is_numeric)
                    .last()
                    .and_then(|s| s.parse::<i16>().ok())
                    .unwrap_or(5);
                return (0..=max_count)
                    .map(|n| {
                        let label = if n == 0 {
                            "Draw 0 (skip)".to_string()
                        } else {
                            format!("Draw {}", n)
                        };
                        make_action_params(
                            ActionType::ChoiceDecision,
                            &label,
                            ActionParameters {
                                card_id: Some(n as i16),
                                card_no: Some(n.to_string()),
                                ..make_params()
                            },
                        )
                    })
                    .collect();
            }
            if target == "order" {
                let count = description
                    .matches(char::is_numeric)
                    .last()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(3);
                return (0..count)
                    .map(|n| {
                        let label = format!("Move card {} to top", n + 1);
                        make_action_params(
                            ActionType::ChoiceDecision,
                            &label,
                            ActionParameters {
                                card_id: Some(n as i16),
                                card_no: Some(n.to_string()),
                                ..make_params()
                            },
                        )
                    })
                    .collect();
            }
            if target == "choice" {
                return description
                    .split(" / ")
                    .enumerate()
                    .map(|(i, opt)| {
                        make_action_params(
                            ActionType::ChoiceOption,
                            opt,
                            ActionParameters {
                                card_id: Some(i as i16),
                                card_no: Some(i.to_string()),
                                ..make_params()
                            },
                        )
                    })
                    .collect();
            }
            if target == "primary|alternative" {
                return vec![
                    make_action_params(
                        ActionType::ChoiceOption,
                        &format!("Primary: {}", description),
                        ActionParameters {
                            card_id: Some(0),
                            card_no: Some("primary".to_string()),
                            ..make_params()
                        },
                    ),
                    make_action_params(
                        ActionType::ChoiceOption,
                        &format!("Alternative: {}", description),
                        ActionParameters {
                            card_id: Some(1),
                            card_no: Some("alternative".to_string()),
                            ..make_params()
                        },
                    ),
                ];
            }
            if target == "apply_replacement" {
                return vec![
                    make_action_params(
                        ActionType::ChoiceOption,
                        "Apply replacement",
                        ActionParameters {
                            card_id: Some(1),
                            card_no: Some("yes".to_string()),
                            ..make_params()
                        },
                    ),
                    make_action_params(
                        ActionType::ChoiceOption,
                        "Don't apply",
                        ActionParameters {
                            card_id: Some(0),
                            card_no: Some("no".to_string()),
                            ..make_params()
                        },
                    ),
                ];
            }
            if target == "choice_string" || target == "conditional_optional" {
                return vec![
                    make_action_params(
                        ActionType::ChoiceOption,
                        &format!("Yes  E{}", description),
                        ActionParameters {
                            card_id: Some(1),
                            card_no: Some("yes".to_string()),
                            ..make_params()
                        },
                    ),
                    make_action_params(
                        ActionType::ChoiceOption,
                        &format!("No  E{}", description),
                        ActionParameters {
                            card_id: Some(0),
                            card_no: Some("no".to_string()),
                            ..make_params()
                        },
                    ),
                ];
            }
            vec![
                make_action_params(
                    ActionType::ChoiceDecision,
                    &format!("Yes  E{}", description),
                    ActionParameters {
                        card_id: Some(1),
                        card_no: Some("yes".to_string()),
                        ..make_params()
                    },
                ),
                make_action_params(
                    ActionType::ChoiceDecision,
                    &format!("No  E{}", description),
                    ActionParameters {
                        card_id: Some(0),
                        card_no: Some("no".to_string()),
                        ..make_params()
                    },
                ),
            ]
        }
        Choice::SelectCard {
            zone,
            card_type,
            count: _,
            description,
            allow_skip,
            ..
        } => {
            let mut actions = Vec::new();
            let active = game_state.active_player();
            let card_ids: Vec<(usize, i16)> = match zone.as_str() {
                "hand" => active
                    .hand
                    .cards
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(i, id)| (i, id))
                    .collect(),
                "discard" => active
                    .waitroom
                    .cards
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(i, id)| (i, id))
                    .collect(),
                "stage" => active
                    .stage
                    .stage
                    .iter()
                    .copied()
                    .enumerate()
                    .filter(|&(_, id)| id != -1)
                    .map(|(i, id)| (i, id))
                    .collect(),
                "energy_zone" => active
                    .energy_zone
                    .cards
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(i, id)| (i, id))
                    .collect(),
                "looked_at" => game_state
                    .looked_at_cards
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(i, id)| (i, id))
                    .collect(),
                "revealed_cards" => game_state
                    .revealed_cards
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(i, id)| (i, id))
                    .collect(),
                _ => Vec::new(),
            };
            if !card_ids.is_empty() {
                for (zone_index, card_id) in &card_ids {
                    let card_matches = match card_type.as_deref() {
                        Some("member_card") => game_state
                            .card_database
                            .get_card(*card_id)
                            .map(|c| c.is_member())
                            .unwrap_or(false),
                        Some("live_card") => game_state
                            .card_database
                            .get_card(*card_id)
                            .map(|c| c.is_live())
                            .unwrap_or(false),
                        Some("energy_card") => game_state
                            .card_database
                            .get_card(*card_id)
                            .map(|c| c.is_energy())
                            .unwrap_or(false),
                        None => true,
                        _ => true,
                    };
                    if !card_matches {
                        continue;
                    }
                    let card_name = game_state
                        .card_database
                        .get_card(*card_id)
                        .map(|c| c.name.as_str())
                        .unwrap_or("Unknown");
                    actions.push(make_action_params(
                        ActionType::ChoiceSelect,
                        &format!("{} ({})", card_name, zone_index),
                        ActionParameters {
                            card_id: Some(*card_id),
                            card_index: Some(*zone_index),
                            card_indices: Some(vec![*zone_index]),
                            card_name: Some(card_name.to_string()),
                            card_no: Some("select".to_string()),
                            ..make_params()
                        },
                    ));
                }
            } else {
                actions.push(make_action_params(
                    ActionType::ChoiceSelect,
                    &format!("Select card(s): {}", description),
                    ActionParameters {
                        card_indices: Some(Vec::new()),
                        card_no: Some("select".to_string()),
                        ..make_params()
                    },
                ));
            }
            if *allow_skip {
                actions.push(make_action_params(
                    ActionType::ChoiceSkip,
                    "Skip",
                    ActionParameters {
                        card_no: Some("skip".to_string()),
                        ..make_params()
                    },
                ));
            }
            actions
        }
        Choice::SelectPosition {
            position,
            description,
            ..
        } => position
            .split(',')
            .map(|a| a.trim())
            .map(|area| {
                let stage_area_str = match area {
                    "left" | "left_side" | "左サイドエリア" => Some("left".to_string()),
                    "center" | "センターエリア" => Some("center".to_string()),
                    "right" | "right_side" | "右サイドエリア" => Some("right".to_string()),
                    _ => Some(area.to_string()),
                };
                make_action_params(
                    ActionType::ChoicePosition,
                    &format!("Place at {}: {}", area, description),
                    ActionParameters {
                        stage_area: stage_area_str,
                        card_no: Some("select".to_string()),
                        ..make_params()
                    },
                )
            })
            .collect(),
        Choice::SelectHeartColor {
            count: _,
            options,
            description,
        }
        | Choice::SelectHeartType {
            count: _,
            options,
            description,
        } => options
            .iter()
            .enumerate()
            .map(|(i, color)| {
                make_action_params(
                    ActionType::ChoiceOption,
                    &format!("{}  E{}", color, description),
                    ActionParameters {
                        card_id: Some(i as i16),
                        card_no: Some(color.clone()),
                        ..make_params()
                    },
                )
            })
            .collect(),
    }
}

fn generate_rps_actions() -> Vec<Action> {
    vec![
        make_action(ActionType::RockChoice, "Rock"),
        make_action(ActionType::PaperChoice, "Paper"),
        make_action(ActionType::ScissorsChoice, "Scissors"),
    ]
}

fn generate_choose_first_attacker_actions(game_state: &GameState) -> Vec<Action> {
    println!(
        "DEBUG: ChooseFirstAttacker phase, rps_winner: {:?}",
        game_state.rps_winner
    );
    vec![
        make_action_params(ActionType::ChooseFirstAttacker, "Go first", make_params()),
        make_action_params(ActionType::ChooseSecondAttacker, "Go second", make_params()),
    ]
}

fn generate_mulligan_actions(game_state: &GameState) -> Vec<Action> {
    let (mulligan_player, player_name) = match game_state.current_phase {
        crate::game_state::Phase::MulliganP1Turn => (&game_state.player1, "Player 1"),
        _ => (&game_state.player2, "Player 2"),
    };

    let mut actions = vec![make_action(
        ActionType::MulliganHeader,
        &format!("{}'s Mulligan Phase", player_name),
    )];

    for (hand_index, card_id) in mulligan_player.hand.cards.iter().enumerate() {
        let is_selected = game_state.mulligan_selected_indices.contains(&hand_index);
        let card_name = game_state
            .card_database
            .get_card(*card_id)
            .map(|c| c.name.as_str())
            .unwrap_or("Unknown");
        actions.push(make_action_params(
            ActionType::SelectMulligan,
            &format!(
                "{} {} for mulligan",
                if is_selected { "Deselect" } else { "Select" },
                card_name
            ),
            ActionParameters {
                card_id: Some(*card_id),
                card_indices: Some(vec![hand_index]),
                ..make_params()
            },
        ));
    }

    actions.push(make_action_params(
        ActionType::ConfirmMulligan,
        &format!("Confirm {}'s mulligan", player_name),
        ActionParameters {
            card_indices: Some(vec![]),
            ..make_params()
        },
    ));
    actions.push(make_action(
        ActionType::SkipMulligan,
        &format!("Skip {}'s mulligan (keep all cards)", player_name),
    ));
    actions
}

fn generate_main_phase_actions(game_state: &GameState) -> Vec<Action> {
    let active_player = game_state.active_player();
    let mut actions = vec![make_action(ActionType::Pass, "Pass - End Main Phase")];

    if !game_state.is_action_prohibited("play_member") {
        // Rule 7.7.2.2: Main Phase - Can play member cards to stage
        let estimated = active_player.hand.cards.len() * 3 + 1;
        actions.reserve(estimated);

        for (hand_index, card_id) in active_player.hand.cards.iter().enumerate() {
            if let Some(card) = game_state.card_database.get_card(*card_id) {
                if card.is_member() && !card.is_live() {
                    let card_cost = card.cost.unwrap_or(0);
                    let hand_count = active_player.hand.cards.len();
                    let hand_reduction = card.get_hand_cost_reduction(hand_count);
                    let effective_cost = card_cost.saturating_sub(hand_reduction);
                    let active_energy_count = active_player.energy_zone.active_count();

                    let areas = [
                        (crate::zones::MemberArea::LeftSide, "left"),
                        (crate::zones::MemberArea::Center, "center"),
                        (crate::zones::MemberArea::RightSide, "right"),
                    ];

                    let mut available_areas = Vec::with_capacity(3);
                    let mut has_any_available = false;
                    let stage_card_ids = [
                        active_player.stage.stage[0],
                        active_player.stage.stage[1],
                        active_player.stage.stage[2],
                    ];

                    for (area_idx, (area, area_name)) in areas.iter().enumerate() {
                        let mut area_info = AreaInfo {
                            area: area_name.to_string(),
                            available: false,
                            cost: card_cost,
                            is_baton_touch: false,
                            existing_member_name: None,
                        };

                        if stage_card_ids[area_idx] != -1 {
                            let existing_member_id = stage_card_ids[area_idx];
                            if !active_player.areas_locked_this_turn.contains(area)
                                && game_state.baton_touch_count < 1
                            {
                                // Check if existing member has cannot_baton_touch restriction
                                let has_baton_touch_protection = game_state
                                    .card_database
                                    .get_card(existing_member_id)
                                    .map_or(false, |existing_card| {
                                        existing_card.abilities.iter().any(|a| {
                                            a.effect.as_ref().map_or(false, |ef| {
                                                ef.restriction_type.as_deref()
                                                    == Some("cannot_baton_touch")
                                            })
                                        })
                                    });

                                if !has_baton_touch_protection {
                                    let member_cost = game_state
                                        .card_database
                                        .get_card(existing_member_id)
                                        .and_then(|c| c.cost)
                                        .unwrap_or(0);
                                    let cost_to_pay = effective_cost.saturating_sub(member_cost);
                                    if (active_energy_count as u32) >= cost_to_pay {
                                        area_info.available = true;
                                        area_info.cost = cost_to_pay;
                                        area_info.is_baton_touch = true;
                                        area_info.existing_member_name = game_state
                                            .card_database
                                            .get_card(existing_member_id)
                                            .map(|c| c.name.clone());
                                        has_any_available = true;
                                    }
                                }
                            }
                        } else if (active_energy_count as u32) >= effective_cost {
                            area_info.available = true;
                            area_info.cost = effective_cost;
                            has_any_available = true;
                        }
                        available_areas.push(area_info);
                    }

                    if has_any_available {
                        let cost_details: Vec<String> = available_areas
                            .iter()
                            .filter(|a| a.available)
                            .map(|a| {
                                let name = match a.area.as_str() {
                                    "left" => "Left",
                                    "center" => "Center",
                                    "right" => "Right",
                                    o => o,
                                };
                                if a.is_baton_touch {
                                    format!(
                                        "{}: {} (baton touch from {})",
                                        name,
                                        a.cost,
                                        a.existing_member_name.as_deref().unwrap_or("existing")
                                    )
                                } else {
                                    format!("{}: {}", name, a.cost)
                                }
                            })
                            .collect();
                        let cost_str = if cost_details.is_empty() {
                            format!("Cost: {}", card_cost)
                        } else {
                            format!("Cost: {}", cost_details.join(", "))
                        };

                        actions.push(make_action_params(
                            ActionType::PlayMemberToStage,
                            &format!("{} ({}) - {}", card.name, card.card_no, cost_str),
                            ActionParameters {
                                card_id: Some(*card_id),
                                card_index: Some(hand_index),
                                card_name: Some(card.name.clone()),
                                card_no: Some(card.card_no.clone()),
                                base_cost: Some(card_cost),
                                available_areas: Some(available_areas),
                                ..make_params()
                            },
                        ));
                    }
                }
            }
        }
    }

    // Check stage cards for abilities that can be activated
    let stage_positions = [
        (active_player.stage.stage[0], "left"),
        (active_player.stage.stage[1], "center"),
        (active_player.stage.stage[2], "right"),
    ];

    for (card_id, area_name) in stage_positions {
        if card_id == -1 {
            continue;
        }
        if let Some(card) = game_state.card_database.get_card(card_id) {
            let card_position: MemberArea = area_name.parse().unwrap_or(MemberArea::Center);
            for (ability_index, ability) in card.abilities.iter().enumerate() {
                let can_activate = ability.triggers.as_ref().map_or(false, |t| {
                    t.contains("main")
                        || t.contains(crate::triggers::MAIN)
                        || t.contains(crate::triggers::ACTIVATION)
                        || (t.contains(crate::triggers::AUTO) && ability.cost.is_some())
                        || t.contains(crate::triggers::BATON_TOUCH)
                });
                if !can_activate {
                    continue;
                }
                if !crate::zones::check_trigger_position(ability.triggers.as_deref(), card_position)
                {
                    continue;
                }
                if !crate::zones::check_effect_position(
                    ability
                        .effect
                        .as_ref()
                        .and_then(|e| e.activation_position.as_deref()),
                    card_position,
                ) {
                    continue;
                }

                let ability_key =
                    format!("{}_{}_{}", card_id, ability_index, game_state.turn_number);
                if ability.use_limit.is_some()
                    && game_state
                        .turn_limited_abilities_used
                        .contains(&ability_key)
                {
                    continue;
                }

                let ability_name = if ability.full_text.chars().count() > 30 {
                    format!(
                        "{}...",
                        ability.full_text.chars().take(30).collect::<String>()
                    )
                } else {
                    ability.full_text.clone()
                };
                let ability_cost = ability.cost.as_ref().and_then(|c| c.energy).unwrap_or(0);
                let trigger_info = ability
                    .triggers
                    .as_ref()
                    .map(|t| format!(" ({})", t))
                    .unwrap_or_default();

                actions.push(make_action_params(
                    ActionType::UseAbility,
                    &format!(
                        "Use ability on {} ({}): {}{} - Cost: {}",
                        card.name, area_name, ability_name, trigger_info, ability_cost
                    ),
                    ActionParameters {
                        card_id: Some(card_id),
                        stage_area: Some(area_name.to_string()),
                        card_name: Some(card.name.clone()),
                        card_no: Some(card.card_no.clone()),
                        base_cost: Some(ability_cost),
                        final_cost: Some(ability_cost),
                        ..make_params()
                    },
                ));
            }
        }
    }

    actions
}

fn generate_live_card_set_actions(game_state: &GameState) -> Vec<Action> {
    let active_player = game_state.active_player();
    let mut actions = vec![make_action(
        ActionType::Pass,
        "Pass - Finished setting live cards",
    )];

    let can_add_more = active_player.live_card_zone.cards.len() < 3;
    if can_add_more && !game_state.is_action_prohibited("cannot_live") {
        for (hand_index, card_id) in active_player.hand.cards.iter().enumerate() {
            let card_name = game_state
                .card_database
                .get_card(*card_id)
                .map(|c| c.name.as_str())
                .unwrap_or("Unknown");
            actions.push(make_action_params(
                ActionType::SetLiveCard,
                &format!("Place {} to live zone", card_name),
                ActionParameters {
                    card_id: Some(*card_id),
                    card_index: Some(hand_index),
                    card_indices: Some(vec![hand_index]),
                    ..make_params()
                },
            ));
        }
    }
    actions
}
