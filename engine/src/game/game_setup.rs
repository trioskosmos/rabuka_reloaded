// Game setup and initialization functions
// This module contains shared game setup logic used by both the web server and bot modules

use crate::ability::enums::Zone;
use crate::ability::types::Choice;
use crate::game_state::GameState;
use crate::game_state::{GameResult, Phase};
use crate::turn::TurnEngine;
use crate::zones::MemberArea;
use crate::HashSet;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use serde::{Deserialize, Serialize};
#[cfg(not(feature = "no_std"))]
use std::vec::Vec;

pub fn area_label_en(area: &str) -> &str {
    match area {
        "left" | "left_side" => "Left",
        "center" => "Center",
        "right" | "right_side" => "Right",
        other => other,
    }
}

pub fn area_label_ja(area: &str) -> &str {
    match area {
        "left" | "left_side" => "左",
        "center" => "センター",
        "right" | "right_side" => "右",
        other => other,
    }
}

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

impl core::fmt::Display for ActionType {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
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

impl core::str::FromStr for ActionType {
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

macro_rules! action_desc {
    ($($arg:tt)*) => {
        if cfg!(not(feature = "profiling")) {
            format!($($arg)*)
        } else {
            String::new()
        }
    };
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Action {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_ja: Option<String>,
    pub action_type: ActionType,
    pub parameters: Option<ActionParameters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
}

impl Action {
    pub fn with_ja(mut self, ja: impl Into<String>) -> Self {
        self.description_ja = Some(ja.into());
        self
    }

    pub fn display_desc(&self, is_ja: bool) -> &str {
        if is_ja {
            self.description_ja.as_deref().unwrap_or(&self.description)
        } else {
            &self.description
        }
    }
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
    pub base_cost: Option<u8>,
    pub final_cost: Option<u8>,
    pub available_areas: Option<Vec<AreaInfo>>,
    pub double_baton_pairs: Option<Vec<DoubleBatonPair>>, // Available double baton pair+placement options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>, // Card is visible but not selectable (greyscale)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AreaInfo {
    pub area: String,
    pub available: bool,
    pub cost: u8,
    pub is_baton_touch: bool,
    pub existing_member_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DoubleBatonPair {
    pub areas: Vec<String>, // The 2 members to replace (e.g., ["left", "center"])
    pub placement: String,  // Where the card ends up (e.g., "left")
    pub cost: u8,           // Effective cost after both cost reductions
}

pub fn setup_game(game_state: &mut GameState) {
    // Rule 6.2: Pre-Game Procedure
    // Rule 6.2.1.7: Each player moves top 3 cards of energy deck to energy zone
    crate::turn::TurnEngine::setup_initial_energy(game_state);
    // Start at RockPaperScissors phase - player will choose RPS option
    game_state.current_phase = crate::game_state::Phase::RockPaperScissors;
}

/// Returns true for phases that the engine advances automatically with no user input.
pub fn is_automatic_phase(game_state: &GameState) -> bool {
    matches!(
        game_state.current_phase,
        crate::game_state::Phase::Active
            | crate::game_state::Phase::Energy
            | crate::game_state::Phase::Draw
            | crate::game_state::Phase::FirstAttackerPerformance
            | crate::game_state::Phase::SecondAttackerPerformance
            | crate::game_state::Phase::LiveVictoryDetermination
    )
}

/// Returns true for the live-card-set phases (user must act, but it's a distinct
/// kind of "must stop" from a normal human-decision phase).
pub fn is_live_card_set_phase(game_state: &GameState) -> bool {
    matches!(
        game_state.current_phase,
        crate::game_state::Phase::LiveCardSetFirstAttacker
            | crate::game_state::Phase::LiveCardSetSecondAttacker
    )
}

/// Drives the engine forward through all automatic phases until it reaches
/// a phase that requires user input (Main, Mulligan, LiveCardSet, etc.)
/// or a pending ability choice appears.
/// This is identical to the logic used by web_server.rs.
pub fn settle_single_player_state(game_state: &mut GameState) {
    let mut iters = 0u32;
    loop {
        iters += 1;
        if iters > 500 {
            log::error!(
                "infinite-loop guard hit after 500 iters, phase={:?}",
                game_state.current_phase
            );
            break;
        }
        if game_state.has_pending_choice() {
            break;
        }
        if is_automatic_phase(game_state) {
            crate::turn::TurnEngine::advance_phase(game_state);
        } else if is_live_card_set_phase(game_state) {
            break;
        } else {
            break;
        }
    }
}

/// Advance automatic phases until a human choice is needed or game ends.
/// Used by all platform main loops after executing an action.
pub fn settle_auto(gs: &mut GameState) {
    for _ in 0..500 {
        if gs.has_pending_choice() || gs.game_result != GameResult::Ongoing {
            break;
        }
        if is_automatic_phase(gs)
            || matches!(
                gs.current_phase,
                Phase::RockPaperScissors | Phase::ChooseFirstAttacker
            )
        {
            TurnEngine::advance_phase(gs);
        } else {
            break;
        }
    }
}

/// Execute a game action extracted from the action parameters.
/// Returns Ok(()) on success, Err(message) on failure. Always resets loop detection.
pub fn execute_action(gs: &mut GameState, action: &Action) -> Result<(), String> {
    let params = action.parameters.clone();
    let result = TurnEngine::execute_main_phase_action(
        gs,
        &action.action_type,
        params.as_ref().and_then(|p| p.card_id),
        params.as_ref().and_then(|p| p.card_indices.clone()),
        params
            .as_ref()
            .and_then(|p| p.stage_area.as_ref().and_then(|s| s.parse().ok())),
        params.as_ref().and_then(|p| p.use_baton_touch),
    );
    gs.reset_loop_detection();
    result
}

/// Run a quick AI-vs-AI test game using the given cards and deck lists.
/// Returns the number of actions executed, or an error string.
#[cfg(not(feature = "no_std"))]
pub fn test_ai_vs_ai(
    cards: &[crate::card::Card],
    d1: &crate::deck_parser::DeckList,
    d2: &crate::deck_parser::DeckList,
    max_turns: u8,
) -> Result<usize, String> {
    use crate::card::CardDatabase;
    use crate::deck_parser::DeckParser;
    use crate::game::deck_builder::DeckBuilder;
    use crate::player::Player;
    use std::sync::Arc;

    let mut db = Arc::new(CardDatabase::load_or_create(cards.to_vec()));
    let n1 = DeckParser::deck_list_to_card_numbers(d1);
    let n2 = DeckParser::deck_list_to_card_numbers(d2);
    let mut pd1 =
        DeckBuilder::build_deck_from_database(&mut db, n1).map_err(|e| format!("D1:{}", e))?;
    DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
    let mut pd2 =
        DeckBuilder::build_deck_from_database(&mut db, n2).map_err(|e| format!("D2:{}", e))?;
    DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();
    pd1.shuffle_main_deck();
    pd1.shuffle_energy_deck();
    pd2.shuffle_main_deck();
    pd2.shuffle_energy_deck();
    let mut p1 = Player::new("p1".into(), "P1".into(), true);
    p1.set_main_deck(pd1.main_deck);
    p1.set_energy_deck(pd1.energy_deck);
    let mut p2 = Player::new("p2".into(), "P2".into(), false);
    p2.set_main_deck(pd2.main_deck);
    p2.set_energy_deck(pd2.energy_deck);
    let mut gs = GameState::new(p1, p2, db);
    setup_game(&mut gs);
    let mut count = 0usize;
    let mut turns = 0u8;
    let max_iter = (max_turns * 40) as usize;
    while gs.game_result == GameResult::Ongoing && turns < max_turns * 2 && count < max_iter {
        let acts = generate_possible_actions(&gs);
        if acts.is_empty() {
            break;
        }
        let _ = execute_action(&mut gs, &acts[0]);
        count += 1;
        while gs.game_result == GameResult::Ongoing && is_automatic_phase(&gs) {
            TurnEngine::advance_phase(&mut gs);
            turns += 1;
        }
        if gs.current_phase == Phase::Active || gs.current_phase == Phase::Draw {
            turns += 1;
        }
    }
    Ok(count)
}

fn make_action(action_type: ActionType, description: impl Into<String>) -> Action {
    Action {
        description: description.into(),
        description_ja: None,
        action_type,
        parameters: None,
        selected: None,
    }
}

fn make_action_params(
    action_type: ActionType,
    description: impl Into<String>,
    params: ActionParameters,
) -> Action {
    Action {
        description: description.into(),
        description_ja: None,
        action_type,
        parameters: Some(params),
        selected: None,
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
        disabled: None,
    }
}

pub fn generate_possible_actions(game_state: &GameState) -> Vec<Action> {
    #[cfg(not(feature = "no_std"))]
    let _timer = crate::timer::Timer::start("generate_possible_actions");
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
    #[cfg(not(feature = "no_std"))]
    let _timer = crate::timer::Timer::start("generate_pending_choice_actions");
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
                    )
                    .with_ja("オプショナルコストを支払う"),
                    make_action_params(
                        ActionType::ChoiceDecision,
                        "Skip optional cost",
                        ActionParameters {
                            card_id: Some(0),
                            card_no: Some("skip_optional_cost".to_string()),
                            ..make_params()
                        },
                    )
                    .with_ja("オプショナルコストをスキップ"),
                ];
            }
            if target == "position|destination" || target == "area_select" {
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
                            Some(0) => ("left".to_string(), i as i16),
                            Some(1) => ("center".to_string(), i as i16),
                            Some(2) => ("right".to_string(), i as i16),
                            _ => (pos.clone(), i as i16),
                        };
                        let capitalize = |s: &str| -> String {
                            let mut c = s.chars();
                            match c.next() {
                                None => String::new(),
                                Some(f) => f.to_uppercase().to_string() + c.as_str(),
                            }
                        };
                        let ja_area = area_label_ja(&stage_area);
                        let label = if is_source {
                            action_desc!("Select {}", capitalize(&stage_area))
                        } else if let Some(ref src) = from_pos {
                            action_desc!("{} → {}", capitalize(src), capitalize(&stage_area))
                        } else {
                            action_desc!("Move to {}", capitalize(&stage_area))
                        };
                        let label_ja = if is_source {
                            action_desc!("{}を選択", ja_area)
                        } else if let Some(ref src) = from_pos {
                            let ja_src = match src.as_str() {
                                "left" => "左",
                                "center" => "センター",
                                "right" => "右",
                                _ => src,
                            };
                            action_desc!("{} → {}", ja_src, ja_area)
                        } else {
                            action_desc!("{}に移動", ja_area)
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
                        .with_ja(label_ja)
                    })
                    .collect();
                if *allow_skip {
                    actions.push(
                        make_action_params(
                            ActionType::ChoiceSkip,
                            "Skip (don't change position)",
                            ActionParameters {
                                card_id: Some(-1),
                                card_no: Some("skip".to_string()),
                                ..make_params()
                            },
                        )
                        .with_ja("スキップ (ポジション変更なし)"),
                    );
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
                            action_desc!("Draw {}", n)
                        };
                        let label_ja = if n == 0 {
                            "0枚引く (スキップ)".to_string()
                        } else {
                            action_desc!("{}枚引く", n)
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
                        .with_ja(label_ja)
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
                        let label = action_desc!("Move card {} to top", n + 1);
                        let label_ja = action_desc!("{}番目を山札上に移動", n + 1);
                        make_action_params(
                            ActionType::ChoiceDecision,
                            &label,
                            ActionParameters {
                                card_id: Some(n as i16),
                                card_no: Some(n.to_string()),
                                ..make_params()
                            },
                        )
                        .with_ja(label_ja)
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
                    actions.push(
                        make_action_params(
                            ActionType::ChoiceSkip,
                            "Skip",
                            ActionParameters {
                                card_no: Some("skip".to_string()),
                                ..make_params()
                            },
                        )
                        .with_ja("スキップ"),
                    );
                }
                return actions;
            }
            if target == "primary|alternative" {
                return vec![
                    make_action_params(
                        ActionType::ChoiceOption,
                        action_desc!("Primary: {}", description),
                        ActionParameters {
                            card_id: Some(0),
                            card_no: Some("primary".to_string()),
                            ..make_params()
                        },
                    )
                    .with_ja(action_desc!("主: {}", description)),
                    make_action_params(
                        ActionType::ChoiceOption,
                        action_desc!("Alternative: {}", description),
                        ActionParameters {
                            card_id: Some(1),
                            card_no: Some("alternative".to_string()),
                            ..make_params()
                        },
                    )
                    .with_ja(action_desc!("副: {}", description)),
                ];
            }
            if target == "apply_replacement" {
                let mut pair =
                    make_choice_pair(ActionType::ChoiceOption, "Apply replacement", "Don't apply");
                pair[0].description_ja = Some("置き換えを適用".into());
                pair[1].description_ja = Some("適用しない".into());
                return pair;
            }
            if target == "self_or_opponent" {
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
            if target == "choice_string" || target == "conditional_optional" {
                if let Some(ref opts) = options {
                    // Enumerate all options directly — no fallback to Yes/No+description
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
                let mut pair = make_choice_pair(ActionType::ChoiceOption, "Yes", "No");
                pair[0].description_ja = Some("はい".into());
                pair[1].description_ja = Some("いいえ".into());
                return pair;
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
            // Generic fallback for unrecognized targets
            let mut pair = make_choice_pair(ActionType::ChoiceDecision, "Yes", "No");
            pair[0].description_ja = Some("はい".into());
            pair[1].description_ja = Some("いいえ".into());
            pair
        }
        Choice::SelectCard {
            zone,
            card_type,
            description,
            allow_skip,
            ref filtered_indices,
            cost_limit,
            ref cost_limit_operator,
            ref group,
            ref characters,
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
                        // Try to use game_state.revealed_cards when possible since it's
                        // kept in sync with ability resolution (cheer buffer may be stale
                        // after live_success check moves cards to waitroom).
                        // Only use cheer buffer if revealed_cards is empty but cheer isn't.
                        let use_gs_revealed =
                            !game_state.revealed_cards.is_empty() || cheer.is_empty();
                        if use_gs_revealed {
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
                        } else {
                            cheer.iter().copied().enumerate().collect()
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
            // filtered_indices (if set) narrows selectable set; otherwise all cards are candidates
            let fi_set: Option<HashSet<usize>> = match filtered_indices {
                Some(fi) if !fi.is_empty() => Some(fi.iter().copied().collect()),
                _ => None,
            };

            let is_look_zone = Zone::from_str(zone) == Some(Zone::LookedAt);

            for (zone_index, card_id) in &card_ids {
                let card = game_state.card_database.get_card(*card_id);
                let card_name = card.map(|c| c.name.as_ref()).unwrap_or("Unknown");
                let real_card_no = card.map(|c| c.card_no.to_string()).unwrap_or_default();

                // Hard filter: card_type mismatch → hide entirely
                // For looked_at zone (look abilities), skip this — show all cards,
                // non-matching ones get blacked out via disabled below.
                if !is_look_zone {
                    let matches_type = match card_type.as_deref() {
                        Some("member_card") => card.map(|c| c.is_member()).unwrap_or(false),
                        Some("live_card") => card.map(|c| c.is_live()).unwrap_or(false),
                        Some("energy_card") => card.map(|c| c.is_energy()).unwrap_or(false),
                        None => true,
                        _ => true,
                    };
                    if !matches_type {
                        continue;
                    }
                }

                // Soft filters: any failure → greyed out (look) or hidden (non-look)
                let in_fi = fi_set.as_ref().map_or(true, |s| s.contains(zone_index));
                let matches_chars = characters.as_ref().map_or(true, |chars| {
                    crate::ability::util::card_matches_characters(
                        &game_state.card_database,
                        *card_id,
                        Some(chars),
                    )
                });
                let matches_group = group.as_ref().map_or(true, |grp| {
                    crate::ability::util::card_matches_group_str(
                        &game_state.card_database,
                        *card_id,
                        Some(grp.as_str()),
                    )
                });
                let matches_cost = cost_limit.map_or(true, |lim| {
                    crate::ability::util::card_matches_cost_limit_op(
                        &game_state.card_database,
                        *card_id,
                        Some(lim),
                        cost_limit_operator.as_deref(),
                    )
                });
                let is_selectable = in_fi && matches_chars && matches_group && matches_cost;

                // For non-look zones: skip non-selectable cards entirely
                if !is_look_zone && !is_selectable {
                    continue;
                }

                // Map to filtered-index position for card_indices
                let fi_index = match filtered_indices {
                    Some(fi) if !fi.is_empty() => fi
                        .iter()
                        .position(|&x| x == *zone_index)
                        .unwrap_or(*zone_index),
                    _ => *zone_index,
                };

                actions.push(make_action_params(
                    ActionType::ChoiceSelect,
                    card_name,
                    ActionParameters {
                        card_id: Some(*card_id),
                        card_index: Some(*zone_index),
                        card_indices: if is_selectable {
                            Some(vec![fi_index])
                        } else {
                            None
                        },
                        card_name: Some(card_name.to_string()),
                        card_no: Some(real_card_no.clone()),
                        disabled: if is_selectable { None } else { Some(true) },
                        ..make_params()
                    },
                ));
            }

            if actions.is_empty() {
                let mut a = make_action_params(
                    ActionType::ChoiceSelect,
                    action_desc!("Select card(s): {}", description),
                    ActionParameters {
                        card_indices: Some(Vec::new()),
                        card_no: Some("select".to_string()),
                        ..make_params()
                    },
                );
                a.description_ja = Some(
                    choice
                        .description_ja()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "カードを選択".into()),
                );
                actions.push(a);
            }
            if *allow_skip {
                let mut a = make_action_params(
                    ActionType::ChoiceSkip,
                    "Skip",
                    ActionParameters {
                        card_no: Some("skip".to_string()),
                        ..make_params()
                    },
                );
                a.description_ja = Some("スキップ".into());
                actions.push(a);
            }
            actions
        }
        Choice::SelectPosition {
            position,
            description: _,
            ..
        } => {
            let choice_ja = choice.description_ja().map(|s| s.to_string());
            position
                .split(',')
                .map(|a| a.trim())
                .map(|area| {
                    let (stage_area_str, card_id) = match area {
                        "left" | "left_side" | "左サイドエリア" => {
                            (Some("left".to_string()), Some(0i16))
                        }
                        "center" | "センターエリア" => {
                            (Some("center".to_string()), Some(1i16))
                        }
                        "right" | "right_side" | "右サイドエリア" => {
                            (Some("right".to_string()), Some(2i16))
                        }
                        _ => (Some(area.to_string()), Some(1i16)),
                    };
                    let ja_area = match stage_area_str.as_deref() {
                        Some("left") => "左",
                        Some("center") => "センター",
                        Some("right") => "右",
                        _ => area,
                    };
                    let label = stage_area_str
                        .as_deref()
                        .map(|s| action_desc!("Place at {}", s))
                        .unwrap_or_else(|| action_desc!("Place at {}", area));
                    let mut a = make_action_params(
                        ActionType::ChoicePosition,
                        &label,
                        ActionParameters {
                            card_id,
                            stage_area: stage_area_str,
                            card_no: Some("select".to_string()),
                            ..make_params()
                        },
                    );
                    a.description_ja = Some(
                        choice_ja
                            .clone()
                            .unwrap_or_else(|| action_desc!("{}に配置", ja_area)),
                    );
                    a
                })
                .collect()
        }
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
                    action_desc!("{}  E{}", color, description),
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
                    action_desc!("{}: {}", opt.card_name, description),
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
                    action_desc!("{}: {}", opt.card_name, description),
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
        make_action(ActionType::RockChoice, "Rock").with_ja("グー"),
        make_action(ActionType::PaperChoice, "Paper").with_ja("チョキ"),
        make_action(ActionType::ScissorsChoice, "Scissors").with_ja("パー"),
    ]
}

fn generate_choose_first_attacker_actions(game_state: &GameState) -> Vec<Action> {
    log::debug!(
        "DEBUG: ChooseFirstAttacker phase, rps_winner: {:?}",
        game_state.rps_winner
    );
    vec![
        make_action_params(ActionType::ChooseFirstAttacker, "Go first", make_params())
            .with_ja("先攻"),
        make_action_params(ActionType::ChooseSecondAttacker, "Go second", make_params())
            .with_ja("後攻"),
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

    let mut actions = vec![{
        let mut a = make_action_params(
            ActionType::ConfirmMulligan,
            action_desc!("Confirm {}'s mulligan", player_name),
            ActionParameters { ..make_params() },
        );
        a.description_ja = Some(action_desc!("{}のマリガンを確定", player_name));
        a
    }];

    for (hand_index, card_id) in mulligan_player.hand.cards.iter().enumerate() {
        let is_selected = game_state.mulligan_selected_indices.contains(&hand_index);
        let card = game_state.card_database.get_card(*card_id);
        let card_name = card.map(|c| c.name.as_ref()).unwrap_or("Unknown");
        let card_no_str = card.map(|c| c.card_no.to_string()).unwrap_or_default();
        let sel_ja = if is_selected {
            "の選択解除"
        } else {
            "を選択"
        };
        let mut a = make_action_params(
            ActionType::SelectMulligan,
            action_desc!(
                "{} {} for mulligan",
                if is_selected { "Deselect" } else { "Select" },
                card_name
            ),
            ActionParameters {
                card_id: Some(*card_id),
                card_index: Some(hand_index),
                card_indices: Some(vec![hand_index]),
                card_name: Some(card_name.to_string()),
                card_no: Some(card_no_str),
                ..make_params()
            },
        );
        a.selected = Some(is_selected);
        a.description_ja = Some(action_desc!("{} {} マリガン", card_name, sel_ja));
        actions.push(a);
    }

    actions
}

/// Returns true if `existing_card` prevents a baton touch from `card_id`
/// (i.e. it has a `cannot_baton_touch` restriction that is not excluded by
/// `card_id`'s groups). Extracted from the action-generation hot path so the
/// per-hand-card, per-area scan runs only once per area.
fn has_cannot_baton_touch(
    card_db: &crate::card::CardDatabase,
    card_id: i16,
    existing_card: &crate::card::Card,
) -> bool {
    existing_card.resolved_abilities().any(|ability| {
        ability.effect.as_ref().is_some_and(|ef| {
            if ef.restriction_type_any().as_deref() != Some("cannot_baton_touch") {
                return false;
            }
            if let Some(ref exclude_groups) = ef.exclude_group_names_any() {
                if crate::ability::util::card_matches_any_group(card_db, card_id, exclude_groups) {
                    return false;
                }
            }
            true
        })
    })
}

fn generate_main_phase_actions(game_state: &GameState) -> Vec<Action> {
    #[cfg(not(feature = "no_std"))]
    let _timer = crate::timer::Timer::start("generate_main_phase_actions");
    let active_player = game_state.active_player();
    let mut actions =
        vec![make_action(ActionType::Pass, "Pass - End Main Phase")
            .with_ja("パス - メインフェーズ終了")];

    if !game_state.is_action_prohibited("play_member") {
        // Rule 7.7.2.2: Main Phase - Can play member cards to stage
        let estimated = active_player.hand.cards.len() * 3 + 1;
        actions.reserve(estimated);

        // Cache stage card data: read once, not once per hand card
        let stage_card_ids = [
            active_player.stage.stage[0],
            active_player.stage.stage[1],
            active_player.stage.stage[2],
        ];
        let stage_cards: [Option<&crate::card::Card>; 3] = [
            game_state.card_database.get_card(stage_card_ids[0]),
            game_state.card_database.get_card(stage_card_ids[1]),
            game_state.card_database.get_card(stage_card_ids[2]),
        ];

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

                    // Precompute per-area baton-touch protection once per hand card
                    // instead of re-scanning the stage member's abilities for every area.
                    let baton_touch_protected: [bool; 3] = [
                        stage_card_ids[0] != -1
                            && stage_cards[0].is_some_and(|existing_card| {
                                !active_player
                                    .deployed_this_turn
                                    .contains(&stage_card_ids[0])
                                    && has_cannot_baton_touch(
                                        &game_state.card_database,
                                        *card_id,
                                        existing_card,
                                    )
                            }),
                        stage_card_ids[1] != -1
                            && stage_cards[1].is_some_and(|existing_card| {
                                !active_player
                                    .deployed_this_turn
                                    .contains(&stage_card_ids[1])
                                    && has_cannot_baton_touch(
                                        &game_state.card_database,
                                        *card_id,
                                        existing_card,
                                    )
                            }),
                        stage_card_ids[2] != -1
                            && stage_cards[2].is_some_and(|existing_card| {
                                !active_player
                                    .deployed_this_turn
                                    .contains(&stage_card_ids[2])
                                    && has_cannot_baton_touch(
                                        &game_state.card_database,
                                        *card_id,
                                        existing_card,
                                    )
                            }),
                    ];

                    let mut available_areas = Vec::with_capacity(3);
                    let mut has_any_available = false;

                    for (area_idx, (_area, area_name)) in areas.iter().enumerate() {
                        let mut area_info = AreaInfo {
                            area: area_name.to_string(),
                            available: false,
                            cost: card_cost,
                            is_baton_touch: false,
                            existing_member_name: None,
                        };

                        if stage_card_ids[area_idx] != -1 {
                            let existing_member_id = stage_card_ids[area_idx];
                            // Rule 9.6.2.1.2.1: Check if the card at this area was deployed this turn.
                            // The check follows the member (R3/R4), not the area.
                            if !active_player
                                .deployed_this_turn
                                .contains(&existing_member_id)
                            {
                                // Check if existing member has cannot_baton_touch restriction
                                let has_baton_touch_protection = baton_touch_protected[area_idx];

                                if !has_baton_touch_protection {
                                    if let Some(existing_member_card) = stage_cards[area_idx] {
                                        let member_cost = existing_member_card.cost.unwrap_or(0);
                                        let cost_to_pay =
                                            effective_cost.saturating_sub(member_cost);
                                        if (active_energy_count as u8) >= cost_to_pay {
                                            area_info.available = true;
                                            area_info.cost = cost_to_pay;
                                            area_info.is_baton_touch = true;
                                            area_info.existing_member_name =
                                                Some(existing_member_card.name.to_string());
                                            has_any_available = true;
                                        }
                                    }
                                }
                            }
                        } else if (active_energy_count as u8) >= effective_cost {
                            area_info.available = true;
                            area_info.cost = effective_cost;
                            has_any_available = true;
                        }
                        available_areas.push(area_info);
                    }

                    // Check if this card has play_baton_touch with count > 1 (double baton)
                    let has_double_baton = card.resolved_abilities().any(|ability| {
                        ability.effect.as_ref().is_some_and(|ef| {
                            ef.action == crate::ability::enums::ActionType::PlayBatonTouch
                                && ef.count.unwrap_or(1) > 1
                        })
                    });

                    let (double_baton_pairs, any_double_baton_available) = if has_double_baton {
                        // Reuse the per-area protection computed above.
                        let cannot_baton_touch_protected = &baton_touch_protected;
                        let occupied: Vec<(usize, &str, i16)> = [0, 1, 2]
                            .iter()
                            .filter(|&&idx| stage_card_ids[idx] != -1)
                            // Rule 9.6.2.1.2.1: Check card identity, not area identity.
                            .filter(|&&idx| {
                                !active_player
                                    .deployed_this_turn
                                    .contains(&stage_card_ids[idx])
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
                                    if (active_energy_count as u8) >= pair_cost {
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
                        for area in &available_areas {
                            if !area.available {
                                continue;
                            }
                            let area_label = area_label_en(area.area.as_str());
                            let area_label_ja = area_label_ja(area.area.as_str());
                            let cost_display = area.cost;
                            let bt = if area.is_baton_touch {
                                action_desc!(
                                    " bt from {}",
                                    area.existing_member_name.as_deref().unwrap_or("?")
                                )
                            } else {
                                String::new()
                            };
                            let bt_ja = if area.is_baton_touch {
                                action_desc!(
                                    " バトン:{}から",
                                    area.existing_member_name.as_deref().unwrap_or("?")
                                )
                            } else {
                                String::new()
                            };
                            let cost_str = cost_display.to_string();
                            {
                                let mut a = make_action_params(
                                    ActionType::PlayMemberToStage,
                                    action_desc!(
                                        "{} → {} (cost:{}){}",
                                        card.name,
                                        area_label,
                                        cost_display,
                                        bt
                                    ),
                                    ActionParameters {
                                        card_id: Some(*card_id),
                                        card_index: Some(hand_index),
                                        card_name: {
                                            if cfg!(not(feature = "profiling")) {
                                                Some(card.name.to_string())
                                            } else {
                                                None
                                            }
                                        },
                                        card_no: {
                                            if cfg!(not(feature = "profiling")) {
                                                Some(card.card_no.to_string())
                                            } else {
                                                None
                                            }
                                        },
                                        base_cost: Some(card_cost),
                                        stage_area: Some(area.area.clone()),
                                        // available_areas is only consumed by the UI/web/main.rs
                                        // path; the profiling/bot decision path never reads it, so
                                        // skip the Vec<AreaInfo> clone in profiling builds.
                                        available_areas: if cfg!(feature = "profiling") {
                                            None
                                        } else {
                                            Some(available_areas.clone())
                                        },
                                        double_baton_pairs: if cfg!(feature = "profiling") {
                                            None
                                        } else {
                                            double_baton_pairs.clone()
                                        },
                                        ..make_params()
                                    },
                                );
                                a.description_ja = Some(action_desc!(
                                    "{} → {} (コスト:{}){}",
                                    card.name,
                                    area_label_ja,
                                    cost_str,
                                    bt_ja
                                ));
                                actions.push(a);
                            }
                        }
                        if let Some(ref pairs) = double_baton_pairs {
                            for pair in pairs {
                                let area_indices: Vec<usize> = pair
                                    .areas
                                    .iter()
                                    .map(|a| match a.as_str() {
                                        "left" => 0,
                                        "center" => 1,
                                        "right" => 2,
                                        _ => 0,
                                    })
                                    .collect();
                                let (src0_en, src1_en, dst_en) = (
                                    area_label_en(pair.areas[0].as_str()),
                                    area_label_en(pair.areas[1].as_str()),
                                    area_label_en(pair.placement.as_str()),
                                );
                                let (src0_ja, src1_ja, dst_ja) = (
                                    area_label_ja(pair.areas[0].as_str()),
                                    area_label_ja(pair.areas[1].as_str()),
                                    area_label_ja(pair.placement.as_str()),
                                );
                                {
                                    let mut a = make_action_params(
                                        ActionType::PlayMemberToStage,
                                        action_desc!(
                                            "{} ({}+{})→{} cost:{}",
                                            card.name,
                                            src0_en,
                                            src1_en,
                                            dst_en,
                                            pair.cost
                                        ),
                                        ActionParameters {
                                            card_id: Some(*card_id),
                                            card_index: Some(hand_index),
                                            card_name: if cfg!(not(feature = "profiling")) {
                                                Some(card.name.to_string())
                                            } else {
                                                None
                                            },
                                            card_no: if cfg!(not(feature = "profiling")) {
                                                Some(card.card_no.to_string())
                                            } else {
                                                None
                                            },
                                            base_cost: Some(pair.cost),
                                            stage_area: Some(pair.placement.clone()),
                                            card_indices: Some(area_indices),
                                            available_areas: if cfg!(feature = "profiling") {
                                                None
                                            } else {
                                                Some(available_areas.clone())
                                            },
                                            double_baton_pairs: if cfg!(feature = "profiling") {
                                                None
                                            } else {
                                                double_baton_pairs.clone()
                                            },
                                            ..make_params()
                                        },
                                    );
                                    a.description_ja = Some(action_desc!(
                                        "{} ({}+{})→{} コスト:{}",
                                        card.name,
                                        src0_ja,
                                        src1_ja,
                                        dst_ja,
                                        pair.cost
                                    ));
                                    actions.push(a);
                                }
                            }
                        }
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
            for (ability_index, ar) in card.abilities.iter().enumerate() {
                let ability = ar.resolve();
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
                        .and_then(|e| e.activation_position_any()),
                    card_position,
                ) {
                    continue;
                }

                // Skip abilities that can only activate from the discard pile
                let is_discard_only = ability
                    .effect
                    .as_ref()
                    .and_then(|e| e.activation_condition_parsed_any())
                    .is_some_and(|c| {
                        Zone::from_str(c.get_location().unwrap_or("")) == Some(Zone::Discard)
                    });
                if is_discard_only {
                    continue;
                }

                let ability_key = (card_id, ability_index, game_state.turn_number);
                if let Some(use_limit) = ability.use_limit {
                    let used = game_state
                        .turn_limited_abilities_used
                        .get(&ability_key)
                        .copied()
                        .unwrap_or(0);
                    if u8::from(used) >= use_limit {
                        continue;
                    }
                }

                let ability_cost = ability
                    .cost
                    .as_ref()
                    .and_then(|c| c.energy_count_any())
                    .unwrap_or(0);
                let trigger_info = ability
                    .triggers
                    .as_ref()
                    .map(|t| action_desc!(" ({})", t))
                    .unwrap_or_default();

                actions.push(make_action_params(
                    ActionType::UseAbility,
                    action_desc!(
                        "Use ability on {} ({}): {}{} - Cost: {}",
                        card.name,
                        area_name,
                        ability.full_text,
                        trigger_info,
                        ability_cost
                    ),
                    ActionParameters {
                        card_id: Some(card_id),
                        // Display-only fields; the profiling/bot path routes UseAbility by
                        // card_id alone (handle_use_ability), so skip the String allocs.
                        stage_area: if cfg!(not(feature = "profiling")) {
                            Some(area_name.to_string())
                        } else {
                            None
                        },
                        card_name: if cfg!(not(feature = "profiling")) {
                            Some(card.name.to_string())
                        } else {
                            None
                        },
                        card_no: if cfg!(not(feature = "profiling")) {
                            Some(card.card_no.to_string())
                        } else {
                            None
                        },
                        ability_index: Some(ability_index),
                        source_ability: if cfg!(not(feature = "profiling")) {
                            Some(ability.full_text.clone())
                        } else {
                            None
                        },
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
            for (ability_index, ar) in card.abilities.iter().enumerate() {
                let ability = ar.resolve();
                let is_discard_activation = ability
                    .effect
                    .as_ref()
                    .and_then(|e| e.activation_condition_parsed_any())
                    .is_some_and(|c| {
                        Zone::from_str(c.get_location().unwrap_or("")) == Some(Zone::Discard)
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

                let ability_key = (card_id, ability_index, game_state.turn_number);
                if let Some(use_limit) = ability.use_limit {
                    let used = game_state
                        .turn_limited_abilities_used
                        .get(&ability_key)
                        .copied()
                        .unwrap_or(0);
                    if u8::from(used) >= use_limit {
                        continue;
                    }
                }

                let ability_cost = ability
                    .cost
                    .as_ref()
                    .and_then(|c| c.energy_count_any())
                    .unwrap_or(0);

                actions.push(make_action_params(
                    ActionType::UseAbility,
                    action_desc!(
                        "Use ability on {} (discard): {} (起動) - Cost: {}",
                        card.name,
                        ability.full_text,
                        ability_cost
                    ),
                    ActionParameters {
                        card_id: Some(card_id),
                        // Display-only fields; profiling/bot routes UseAbility by card_id.
                        card_name: if cfg!(not(feature = "profiling")) {
                            Some(card.name.to_string())
                        } else {
                            None
                        },
                        card_no: if cfg!(not(feature = "profiling")) {
                            Some(card.card_no.to_string())
                        } else {
                            None
                        },
                        ability_index: Some(ability_index),
                        source_ability: if cfg!(not(feature = "profiling")) {
                            Some(ability.full_text.clone())
                        } else {
                            None
                        },
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
    #[cfg(not(feature = "no_std"))]
    let _timer = crate::timer::Timer::start("generate_live_card_set_actions");
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

    let mut actions = vec![{
        let mut a = make_action_params(
            ActionType::ConfirmLiveCardSet,
            action_desc!("Confirm {}'s live card set", player_name),
            ActionParameters { ..make_params() },
        );
        a.description_ja = Some(action_desc!("{}のライブカードセットを確定", player_name));
        a
    }];

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
        let card = game_state.card_database.get_card(*card_id);
        let card_name = card.map(|c| c.name.as_ref()).unwrap_or("Unknown");
        let card_no_str = card.map(|c| c.card_no.to_string()).unwrap_or_default();
        let sel_ja = if is_selected {
            "の選択解除"
        } else {
            "を選択"
        };
        let mut a = make_action_params(
            ActionType::SelectLiveCard,
            action_desc!(
                "{} {} for live set",
                if is_selected { "Deselect" } else { "Select" },
                card_name
            ),
            ActionParameters {
                card_id: Some(*card_id),
                card_index: Some(hand_index),
                card_indices: Some(vec![hand_index]),
                card_name: Some(card_name.to_string()),
                card_no: Some(card_no_str),
                ..make_params()
            },
        );
        a.selected = Some(is_selected);
        a.description_ja = Some(action_desc!("{} {} ライブカード", card_name, sel_ja));
        actions.push(a);
    }

    actions
}
