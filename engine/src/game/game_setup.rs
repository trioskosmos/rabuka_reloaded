// Game setup and initialization functions
// This module contains shared game setup logic used by both the web server and bot modules

use crate::ability::enums::Zone;
use crate::ability::types::Choice;
use crate::game_state::GameState;
use crate::zones::MemberArea;
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
    LiveCardHeader,
    SelectLiveCard,
    ConfirmLiveCardSet,
    SkipLiveCardSet,
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
    EnergyCharge,
    PassRemaining,
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
            ActionType::LiveCardHeader => write!(f, "live_card_header"),
            ActionType::SelectLiveCard => write!(f, "select_live_card"),
            ActionType::ConfirmLiveCardSet => write!(f, "confirm_live_card_set"),
            ActionType::SkipLiveCardSet => write!(f, "skip_live_card_set"),
            ActionType::PlayMemberToStage => write!(f, "play_member_to_stage"),
            ActionType::UseAbility => write!(f, "use_ability"),
            ActionType::SetLiveCard => write!(f, "set_live_card"),
            ActionType::FinishLiveCardSet => write!(f, "finish_live_card_set"),
            ActionType::ChoiceDecision => write!(f, "decision"),
            ActionType::ChoiceSelect => write!(f, "select_card"),
            ActionType::ChoiceSkip => write!(f, "select_skip"),
            ActionType::ChoiceOption => write!(f, "choose_option"),
            ActionType::ChoicePosition => write!(f, "select_position"),
            ActionType::EnergyCharge => write!(f, "energy_charge"),
            ActionType::PassRemaining => write!(f, "pass_remaining"),
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
            "live_card_header" => Ok(ActionType::LiveCardHeader),
            "select_live_card" => Ok(ActionType::SelectLiveCard),
            "confirm_live_card_set" => Ok(ActionType::ConfirmLiveCardSet),
            "skip_live_card_set" => Ok(ActionType::SkipLiveCardSet),
            "play_member_to_stage" => Ok(ActionType::PlayMemberToStage),
            "use_ability" => Ok(ActionType::UseAbility),
            "set_live_card" => Ok(ActionType::SetLiveCard),
            "finish_live_card_set" => Ok(ActionType::FinishLiveCardSet),
            "decision" => Ok(ActionType::ChoiceDecision),
            "select_card" => Ok(ActionType::ChoiceSelect),
            "select_skip" => Ok(ActionType::ChoiceSkip),
            "choose_option" => Ok(ActionType::ChoiceOption),
            "select_position" => Ok(ActionType::ChoicePosition),
            "energy_charge" => Ok(ActionType::EnergyCharge),
            "pass_remaining" => Ok(ActionType::PassRemaining),
            _ => Err(format!("Unknown action type: {}", s)),
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
    pub ability_index: Option<usize>, // Which ability on the card (for use_ability actions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ability: Option<String>, // Full ability text block (for display)
    pub base_cost: Option<u32>,
    pub final_cost: Option<u32>,
    pub available_areas: Option<Vec<AreaInfo>>,
    pub double_baton_pairs: Option<Vec<DoubleBatonPair>>, // Available double baton pair+placement options
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AreaInfo {
    pub area: String,
    pub available: bool,
    pub cost: u32,
    pub is_baton_touch: bool,
    pub existing_member_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DoubleBatonPair {
    pub areas: Vec<String>, // The 2 members to replace (e.g., ["left", "center"])
    pub placement: String,  // Where the card ends up (e.g., "left")
    pub cost: u32,          // Effective cost after both cost reductions
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
        ability_index: None,
        source_ability: None,
        base_cost: None,
        final_cost: None,
        available_areas: None,
        double_baton_pairs: None,
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
        crate::game_state::Phase::MulliganFirstAttacker
        | crate::game_state::Phase::MulliganSecondAttacker => generate_mulligan_actions(game_state),
        crate::game_state::Phase::Active
        | crate::game_state::Phase::Energy
        | crate::game_state::Phase::Draw => Vec::new(),
        crate::game_state::Phase::Main => generate_main_phase_actions(game_state),
        crate::game_state::Phase::LiveCardSetFirstAttacker
        | crate::game_state::Phase::LiveCardSetSecondAttacker => {
            generate_live_card_set_actions(game_state)
        }
        crate::game_state::Phase::FirstAttackerPerformance
        | crate::game_state::Phase::SecondAttackerPerformance
        | crate::game_state::Phase::LiveVictoryDetermination => Vec::new(),
    }
}

fn make_choice_pair(action_type: ActionType, yes_text: &str, no_text: &str) -> Vec<Action> {
    vec![
        make_action_params(
            action_type,
            yes_text,
            ActionParameters {
                card_id: Some(1),
                card_no: Some("yes".to_string()),
                ..make_params()
            },
        ),
        make_action_params(
            action_type,
            no_text,
            ActionParameters {
                card_id: Some(0),
                card_no: Some("no".to_string()),
                ..make_params()
            },
        ),
    ]
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
                let is_source = description == "Choose which member to move";
                // Extract source position from description e.g. "(currently at Center)"
                let from_pos = if is_source {
                    None
                } else {
                    description
                        .rsplit_once("currently at ")
                        .and_then(|(_, after)| after.split(')').next())
                        .map(|s| s.trim().to_lowercase())
                };
                let default_positions = vec!["left".into(), "center".into(), "right".into()];
                let positions = options.as_deref().unwrap_or(&default_positions);
                let mut actions: Vec<Action> = positions
                    .iter()
                    .enumerate()
                    .map(|(i, pos)| {
                        let idx = crate::ability::util::stage_position_index(pos);
                        let (stage_area, card_id) = match idx {
                            Some(0) => ("left".to_string(), 0),
                            Some(1) => ("center".to_string(), 1),
                            Some(2) => ("right".to_string(), 2),
                            _ => (pos.clone(), i as i16),
                        };
                        let capitalize = |s: &str| -> String {
                            let mut c = s.chars();
                            match c.next() {
                                None => String::new(),
                                Some(f) => f.to_uppercase().to_string() + c.as_str(),
                            }
                        };
                        let label = if is_source {
                            format!("Select {}", capitalize(&stage_area))
                        } else if let Some(ref src) = from_pos {
                            format!("{} → {}", capitalize(src), capitalize(&stage_area))
                        } else {
                            format!("Move to {}", capitalize(&stage_area))
                        };
                        make_action_params(
                            ActionType::ChoicePosition,
                            &label,
                            ActionParameters {
                                card_id: Some(card_id),
                                stage_area: Some(stage_area),
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
                    .next_back()
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
                                card_id: Some(n),
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
                    .next_back()
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
                let mut actions: Vec<Action> = description
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
                return actions;
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
                return make_choice_pair(
                    ActionType::ChoiceOption,
                    "Apply replacement",
                    "Don't apply",
                );
            }
            if target == "choice_string" || target == "conditional_optional" {
                if let Some(ref opts) = options {
                    if opts.len() > 2 {
                        // Multi-option choice_string: enumerate options
                        return opts
                            .iter()
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
                }
                return make_choice_pair(
                    ActionType::ChoiceOption,
                    &format!("Yes E{}", description),
                    &format!("No E{}", description),
                );
            }
            if target == "choice_condition" {
                let opts = options.as_ref().map(|v| v.as_slice()).unwrap_or(&[]);
                if !opts.is_empty() {
                    return opts
                        .iter()
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
            }
            make_choice_pair(
                ActionType::ChoiceDecision,
                &format!("Yes  E{}", description),
                &format!("No  E{}", description),
            )
        }
        Choice::SelectCard {
            zone,
            card_type,
            count: _,
            description,
            allow_skip,
            cost_limit,
            cost_limit_operator,
            group,
            characters,
            ref filtered_indices,
            ref target_player_id,
            ..
        } => {
            let mut actions = Vec::new();
            let target = target_player_id.as_deref().unwrap_or("self");
            let master = game_state.ability_master_id();
            let card_ids: Vec<(usize, i16)> = {
                let player = match (target, master.as_deref()) {
                    ("self", Some("player2") | Some("p2")) => &game_state.player2,
                    ("self", _) => &game_state.player1,
                    ("opponent", Some("player2") | Some("p2")) => &game_state.player1,
                    ("opponent", _) => &game_state.player2,
                    _ => &game_state.player1,
                };
                match Zone::from_str(zone.as_str()) {
                    Some(Zone::Hand) => player.hand.cards.iter().copied().enumerate().collect(),
                    Some(Zone::Discard) | Some(Zone::Waitroom) => {
                        player.waitroom.cards.iter().copied().enumerate().collect()
                    }
                    Some(Zone::Stage) => player
                        .stage
                        .stage
                        .iter()
                        .copied()
                        .enumerate()
                        .filter(|&(_, id)| id != -1)
                        .collect(),
                    Some(Zone::Energy) | Some(Zone::EnergyZone) => player
                        .energy_zone
                        .cards
                        .iter()
                        .copied()
                        .enumerate()
                        .collect(),
                    Some(Zone::UnderMember) => {
                        let mut ids = Vec::new();
                        for si in 0..3 {
                            for &cid in &player.stage.under_cards[si] {
                                ids.push((ids.len(), cid));
                            }
                        }
                        ids
                    }
                    Some(Zone::LookedAt) => game_state
                        .looked_at_cards
                        .iter()
                        .copied()
                        .enumerate()
                        .collect(),
                    Some(Zone::RevealedCards) => {
                        let cheer = game_state.cheer_revealed_cards();
                        if !cheer.is_empty() {
                            cheer.iter().copied().enumerate().collect()
                        } else {
                            let player = game_state.resolve_target_player(target);
                            game_state
                                .revealed_cards
                                .iter()
                                .copied()
                                .enumerate()
                                .filter(|(_, cid)| {
                                    player.hand.cards.contains(cid)
                                        || player.waitroom.cards.contains(cid)
                                        || player.stage.stage.contains(cid)
                                        || player.stage.under_cards.iter().any(|v| v.contains(cid))
                                        || player.energy_zone.cards.contains(cid)
                                        || player.main_deck.cards.contains(cid)
                                        || player.energy_deck.cards.contains(cid)
                                        || player.live_card_zone.cards.contains(cid)
                                        || player.success_live_card_zone.cards.contains(cid)
                                        || game_state.resolution_zone.cards.contains(cid)
                                })
                                .collect()
                        }
                    }
                    Some(Zone::SelectedCards) => game_state
                        .ability_queue
                        .current_entry()
                        .and_then(|e| e.resolver.as_ref())
                        .map(|r| r.selected_cards.clone())
                        .unwrap_or_default()
                        .into_iter()
                        .enumerate()
                        .collect(),
                    _ => Vec::new(),
                }
            };
            // Apply filtered_indices: exclude already-selected or ineligible cards
            let card_ids: Vec<(usize, i16)> = match &filtered_indices {
                Some(fi) if !fi.is_empty() => card_ids
                    .into_iter()
                    .filter(|(idx, _)| fi.contains(idx))
                    .collect(),
                Some(_) => Vec::new(),
                None => card_ids,
            };

            if !card_ids.is_empty() {
                for (zone_index, card_id) in &card_ids {
                    let card = game_state.card_database.get_card(*card_id);
                    let card_matches = match card_type.as_deref() {
                        Some("member_card") => card.map(|c| c.is_member()).unwrap_or(false),
                        Some("live_card") => card.map(|c| c.is_live()).unwrap_or(false),
                        Some("energy_card") => card.map(|c| c.is_energy()).unwrap_or(false),
                        None => true,
                        _ => true,
                    };
                    if !card_matches {
                        continue;
                    }
                    // Apply per-card cost_limit filter (NOT sum cost_total)
                    if let Some(lim) = cost_limit {
                        if !crate::ability::util::card_matches_cost_limit_op(
                            &game_state.card_database,
                            *card_id,
                            Some(*lim),
                            cost_limit_operator.as_deref(),
                        ) {
                            continue;
                        }
                    }
                    if !crate::ability::util::card_matches_characters(
                        &game_state.card_database,
                        *card_id,
                        characters.as_ref(),
                    ) {
                        continue;
                    }
                    // Apply group filter
                    if let Some(ref grp) = group {
                        if !crate::ability::util::card_matches_group_str(
                            &game_state.card_database,
                            *card_id,
                            Some(grp.as_str()),
                        ) {
                            continue;
                        }
                    }
                    let card_name = card.map(|c| c.name.as_str()).unwrap_or("Unknown");
                    let real_card_no = card.map(|c| c.card_no.clone()).unwrap_or_default();
                    // Map zone position to index within filtered_indices
                    let fi_index = match &filtered_indices {
                        Some(fi) if !fi.is_empty() => fi
                            .iter()
                            .position(|&x| x == *zone_index)
                            .unwrap_or(*zone_index),
                        _ => *zone_index,
                    };
                    actions.push(make_action_params(
                        ActionType::ChoiceSelect,
                        &format!("{} ({})", card_name, zone_index),
                        ActionParameters {
                            card_id: Some(*card_id),
                            card_index: Some(*zone_index),
                            card_indices: Some(vec![fi_index]),
                            card_name: Some(card_name.to_string()),
                            card_no: Some(real_card_no.clone()),
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
            description: _,
            ..
        } => position
            .split(',')
            .map(|a| a.trim())
            .map(|area| {
                let (stage_area_str, card_id) = match area {
                    "left" | "left_side" | "左サイドエリア" => {
                        (Some("left".to_string()), Some(0i16))
                    }
                    "center" | "センターエリア" => (Some("center".to_string()), Some(1i16)),
                    "right" | "right_side" | "右サイドエリア" => {
                        (Some("right".to_string()), Some(2i16))
                    }
                    _ => (Some(area.to_string()), Some(1i16)),
                };
                let label = stage_area_str
                    .as_deref()
                    .map(|s| format!("Place at {}", s))
                    .unwrap_or_else(|| format!("Place at {}", area));
                make_action_params(
                    ActionType::ChoicePosition,
                    &label,
                    ActionParameters {
                        card_id,
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
            ..
        }
        | Choice::SelectHeartType {
            count: _,
            options,
            description,
            ..
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
        Choice::SelectLiveSuccess {
            options,
            description,
            ..
        } => options
            .iter()
            .enumerate()
            .map(|(i, opt)| {
                make_action_params(
                    ActionType::ChoiceOption,
                    &format!("{}: {}", opt.card_name, description),
                    ActionParameters {
                        card_id: Some(i as i16),
                        card_no: Some(opt.card_name.clone()),
                        ..make_params()
                    },
                )
            })
            .collect(),
        Choice::SelectAutoAbility {
            options,
            description,
            ..
        } => options
            .iter()
            .enumerate()
            .map(|(i, opt)| {
                make_action_params(
                    ActionType::ChoiceOption,
                    &format!("{}: {}", opt.card_name, description),
                    ActionParameters {
                        card_id: Some(i as i16),
                        card_no: Some(opt.card_name.clone()),
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
    let is_first = matches!(
        game_state.current_phase,
        crate::game_state::Phase::MulliganFirstAttacker
    );
    let player_name = if is_first {
        if game_state.first_attacker().id == game_state.player1.id {
            "Player 1"
        } else {
            "Player 2"
        }
    } else if game_state.first_attacker().id == game_state.player1.id {
        "Player 2"
    } else {
        "Player 1"
    };
    let mulligan_player = game_state.active_player();

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
                    let reduction = crate::ability::util::calculate_play_cost_reduction(
                        &active_player.stage,
                        &active_player.success_live_card_zone.cards,
                        hand_count,
                        *card_id,
                        &game_state.card_database,
                    );
                    let effective_cost = card_cost.saturating_sub(reduction);
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
                            if !active_player.areas_locked_this_turn.contains(area) {
                                // Check if existing member has cannot_baton_touch restriction
                                let has_baton_touch_protection = game_state
                                    .card_database
                                    .get_card(existing_member_id)
                                    .is_some_and(|existing_card| {
                                        existing_card.abilities.iter().any(|a| {
                                            a.effect.as_ref().is_some_and(|ef| {
                                                if ef.restriction_type.as_deref()
                                                    != Some("cannot_baton_touch")
                                                {
                                                    return false;
                                                }
                                                if let Some(ref exclude_groups) =
                                                    ef.exclude_group_names
                                                {
                                                    if crate::ability::util::card_matches_any_group(
                                                        &game_state.card_database,
                                                        *card_id,
                                                        exclude_groups,
                                                    ) {
                                                        return false;
                                                    }
                                                }
                                                true
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

                    // Check if this card has play_baton_touch with count > 1 (double baton)
                    let has_double_baton = card.abilities.iter().any(|a| {
                        a.effect.as_ref().is_some_and(|ef| {
                            ef.action == "play_baton_touch" && ef.count.unwrap_or(1) > 1
                        })
                    });

                    let (double_baton_pairs, any_double_baton_available) = if has_double_baton {
                        let area_enums = [
                            crate::zones::MemberArea::LeftSide,
                            crate::zones::MemberArea::Center,
                            crate::zones::MemberArea::RightSide,
                        ];
                        // Pre-compute which occupied areas have cannot_baton_touch protection
                        let cannot_baton_touch_protected: Vec<bool> = (0..3)
                            .map(|idx| {
                                let member_id = stage_card_ids[idx];
                                if member_id == -1 {
                                    return false;
                                }
                                game_state
                                    .card_database
                                    .get_card(member_id)
                                    .is_some_and(|card| {
                                        card.abilities.iter().any(|a| {
                                            a.effect.as_ref().is_some_and(|ef| {
                                                if ef.restriction_type.as_deref()
                                                    != Some("cannot_baton_touch")
                                                {
                                                    return false;
                                                }
                                                if let Some(ref exclude_groups) =
                                                    ef.exclude_group_names
                                                {
                                                    if crate::ability::util::card_matches_any_group(
                                                        &game_state.card_database,
                                                        *card_id,
                                                        exclude_groups,
                                                    ) {
                                                        return false;
                                                    }
                                                }
                                                true
                                            })
                                        })
                                    })
                            })
                            .collect();
                        let occupied: Vec<(usize, &str, i16)> = [0, 1, 2]
                            .iter()
                            .filter(|&&idx| stage_card_ids[idx] != -1)
                            .filter(|&&idx| {
                                !active_player
                                    .areas_locked_this_turn
                                    .contains(&area_enums[idx])
                            })
                            .filter(|&&idx| !cannot_baton_touch_protected[idx])
                            .map(|&idx| {
                                let area_names = ["left", "center", "right"];
                                (idx, area_names[idx], stage_card_ids[idx])
                            })
                            .collect();
                        let mut pairs = Vec::new();
                        for i in 0..occupied.len() {
                            for j in (i + 1)..occupied.len() {
                                let (_idx1, name1, cid1) = occupied[i];
                                let (_idx2, name2, cid2) = occupied[j];
                                let cost1 = game_state
                                    .card_database
                                    .get_card(cid1)
                                    .and_then(|c| c.cost)
                                    .unwrap_or(0);
                                let cost2 = game_state
                                    .card_database
                                    .get_card(cid2)
                                    .and_then(|c| c.cost)
                                    .unwrap_or(0);
                                let combined = cost1 + cost2;
                                let pair_cost = effective_cost.saturating_sub(combined);
                                let area_names = [name1.to_string(), name2.to_string()];
                                for placement in [name1.to_string(), name2.to_string()] {
                                    if (active_energy_count as u32) >= pair_cost {
                                        pairs.push(DoubleBatonPair {
                                            areas: area_names.to_vec(),
                                            placement,
                                            cost: pair_cost,
                                        });
                                    }
                                }
                            }
                        }
                        let available = !pairs.is_empty();
                        (if available { Some(pairs) } else { None }, available)
                    } else {
                        (None, false)
                    };

                    if has_any_available || any_double_baton_available {
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
                        let db_cost_str = double_baton_pairs.as_ref().map(|pairs| {
                            let parts: Vec<String> = pairs
                                .iter()
                                .map(|p| {
                                    format!(
                                        "{} (baton touch {}): {}",
                                        p.placement,
                                        p.areas.join(" & "),
                                        p.cost
                                    )
                                })
                                .collect();
                            parts.join(", ")
                        });
                        let cost_str = match (cost_details.is_empty(), &db_cost_str) {
                            (true, None) => format!("Cost: {}", card_cost),
                            (true, Some(db)) => format!("Cost: {} (Double: {})", card_cost, db),
                            (false, None) => format!("Cost: {}", cost_details.join(", ")),
                            (false, Some(db)) => {
                                format!("Cost: {} (Double: {})", cost_details.join(", "), db)
                            }
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
                                double_baton_pairs,
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
                let can_activate = ability.triggers.as_ref().is_some_and(|t| {
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
                if let Some(use_limit) = ability.use_limit {
                    let used = game_state
                        .turn_limited_abilities_used
                        .get(&ability_key)
                        .copied()
                        .unwrap_or(0);
                    if u32::from(used) >= use_limit {
                        continue;
                    }
                }

                let ability_cost = ability
                    .cost
                    .as_ref()
                    .and_then(|c| c.energy_count)
                    .unwrap_or(0);
                let trigger_info = ability
                    .triggers
                    .as_ref()
                    .map(|t| format!(" ({})", t))
                    .unwrap_or_default();

                actions.push(make_action_params(
                    ActionType::UseAbility,
                    &format!(
                        "Use ability on {} ({}): {}{} - Cost: {}",
                        card.name, area_name, ability.full_text, trigger_info, ability_cost
                    ),
                    ActionParameters {
                        card_id: Some(card_id),
                        stage_area: Some(area_name.to_string()),
                        card_name: Some(card.name.clone()),
                        card_no: Some(card.card_no.clone()),
                        ability_index: Some(ability_index),
                        source_ability: Some(ability.full_text.clone()),
                        base_cost: Some(ability_cost),
                        final_cost: Some(ability_cost),
                        ..make_params()
                    },
                ));
            }
        }
    }

    // Also check discard pile for cards that activate from discard
    // (activation_condition_parsed with location = discard)
    for &card_id in &active_player.waitroom.cards {
        if let Some(card) = game_state.card_database.get_card(card_id) {
            for (ability_index, ability) in card.abilities.iter().enumerate() {
                let is_discard_activation = ability
                    .effect
                    .as_ref()
                    .and_then(|e| e.activation_condition_parsed.as_ref())
                    .is_some_and(|c| {
                        Zone::from_str(c.location.as_deref().unwrap_or("")) == Some(Zone::Discard)
                    });
                if !is_discard_activation {
                    continue;
                }
                let can_activate = ability.triggers.as_ref().is_some_and(|t| {
                    t.contains("main")
                        || t.contains(crate::triggers::MAIN)
                        || t.contains(crate::triggers::ACTIVATION)
                        || (t.contains(crate::triggers::AUTO) && ability.cost.is_some())
                        || t.contains(crate::triggers::BATON_TOUCH)
                });
                if !can_activate {
                    continue;
                }

                let ability_key =
                    format!("{}_{}_{}", card_id, ability_index, game_state.turn_number);
                if let Some(use_limit) = ability.use_limit {
                    let used = game_state
                        .turn_limited_abilities_used
                        .get(&ability_key)
                        .copied()
                        .unwrap_or(0);
                    if u32::from(used) >= use_limit {
                        continue;
                    }
                }

                let ability_cost = ability
                    .cost
                    .as_ref()
                    .and_then(|c| c.energy_count)
                    .unwrap_or(0);

                actions.push(make_action_params(
                    ActionType::UseAbility,
                    &format!(
                        "Use ability on {} (discard): {} (起動) - Cost: {}",
                        card.name, ability.full_text, ability_cost
                    ),
                    ActionParameters {
                        card_id: Some(card_id),
                        card_name: Some(card.name.clone()),
                        card_no: Some(card.card_no.clone()),
                        ability_index: Some(ability_index),
                        source_ability: Some(ability.full_text.clone()),
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

    let is_first = matches!(
        game_state.current_phase,
        crate::game_state::Phase::LiveCardSetFirstAttacker
    );
    let player_name = if is_first {
        if game_state.first_attacker().id == game_state.player1.id {
            "Player 1"
        } else {
            "Player 2"
        }
    } else if game_state.first_attacker().id == game_state.player1.id {
        "Player 2"
    } else {
        "Player 1"
    };

    let mut actions = vec![make_action(
        ActionType::LiveCardHeader,
        &format!("{}'s Live Card Set", player_name),
    )];

    let max_live_cards =
        3i32 - i32::try_from(active_player.live_card_set_limit_reduction).unwrap_or(0);
    let already_selected = game_state.live_card_selected_indices.len();
    let max_allowed = max_live_cards.max(0) as usize;
    for (hand_index, card_id) in active_player.hand.cards.iter().enumerate() {
        let is_selected = game_state.live_card_selected_indices.contains(&hand_index);
        let at_limit = already_selected >= max_allowed;
        if at_limit && !is_selected {
            continue;
        }
        let card_name = game_state
            .card_database
            .get_card(*card_id)
            .map(|c| c.name.as_str())
            .unwrap_or("Unknown");
        actions.push(make_action_params(
            ActionType::SelectLiveCard,
            &format!(
                "{} {} for live set",
                if is_selected { "Deselect" } else { "Select" },
                card_name
            ),
            ActionParameters {
                card_id: Some(*card_id),
                card_index: Some(hand_index),
                card_indices: Some(vec![hand_index]),
                ..make_params()
            },
        ));
    }

    actions.push(make_action_params(
        ActionType::ConfirmLiveCardSet,
        &format!("Confirm {}'s live card set", player_name),
        ActionParameters {
            card_indices: Some(vec![]),
            ..make_params()
        },
    ));
    actions
}
