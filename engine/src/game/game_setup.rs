// Game setup and initialization functions
// This module contains shared game setup logic used by both the web server and bot modules

use crate::ability::enums::Zone;
use crate::ability::types::Choice;
use crate::game_state::GameState;
use crate::game_state::{GameResult, Phase};
use crate::turn::TurnEngine;
use crate::zones::MemberArea;
use crate::types::ArcStr;
use crate::{Arc, HashSet};
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};
#[cfg(not(feature = "no_std"))]
use std::vec::Vec;

#[cfg(feature = "no_std")]
use core::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(feature = "no_std"))]
use std::sync::atomic::{AtomicBool, Ordering};

/// Runtime display gate for action-generation UI strings (EN/JA descriptions,
/// card_name/card_no/source_ability copies, existing_member_name labels).
/// Default ON so web/UI paths are unchanged. Headless benches (`sim_bench`)
/// turn it off to skip `format!`/`to_string` on every decision step.
/// Decision fields (`card_id`, `stage_area`, `available_areas`, costs,
/// `ability_index`, `card_indices`) are always built.
static ACTION_DISPLAY: AtomicBool = AtomicBool::new(true);

/// Enable/disable display-only fields on generated `Action`s.
/// Thread-safe; process-global. Default is enabled.
pub fn set_action_display(enabled: bool) {
    ACTION_DISPLAY.store(enabled, Ordering::Relaxed);
}

/// Whether display-only action fields should be materialized.
#[inline]
pub fn action_display_enabled() -> bool {
    ACTION_DISPLAY.load(Ordering::Relaxed)
}

/// Runtime gate for rule/structured/debug log materialization (format!,
/// LogEntry construction, choice offer/resolve payloads). Default ON so
/// web/tests keep full history. Training/sim turn it off.
static LOGGING: AtomicBool = AtomicBool::new(true);

/// Enable/disable log string materialization. Thread-safe; process-global.
pub fn set_logging_enabled(enabled: bool) {
    LOGGING.store(enabled, Ordering::Relaxed);
}

/// Whether hot-path log strings should be built.
#[inline]
pub fn logging_enabled() -> bool {
    LOGGING.load(Ordering::Relaxed)
}

/// Training/headless speed: strip UI Action strings and log materialization.
/// Web/UI paths must leave this at the default (full display + logs).
pub fn set_training_mode(enabled: bool) {
    set_action_display(!enabled);
    set_logging_enabled(!enabled);
}

/// Area label in both languages from ONE table. Previously two
/// copy-pasted matches (`area_label_en` / `area_label_ja`) that could
/// drift apart; the `ja` flag selects the column like describe.rs's
/// `*_inner` helpers.
fn area_label_inner(area: &str, ja: bool) -> &str {
    match (area, ja) {
        ("left" | "left_side", false) => "Left",
        ("center", false) => "Center",
        ("right" | "right_side", false) => "Right",
        ("left" | "left_side", true) => "左",
        ("center", true) => "センター",
        ("right" | "right_side", true) => "右",
        (other, _) => other,
    }
}

pub fn area_label_en(area: &str) -> &str {
    area_label_inner(area, false)
}

pub fn area_label_ja(area: &str) -> &str {
    area_label_inner(area, true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
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

impl ActionType {
    /// Wire tag used by the 3DS multiplayer protocol (see `ActionSync::to_bytes`).
    /// Kept on the engine so the mapping stays in one place and breaks loudly
    /// (exhaustive match) if a new variant is added.
    pub fn to_tag(&self) -> u16 {
        match self {
            ActionType::RockChoice => 0,
            ActionType::PaperChoice => 1,
            ActionType::ScissorsChoice => 2,
            ActionType::ChooseFirstAttacker => 3,
            ActionType::SelectMulligan => 4,
            ActionType::SkipMulligan => 5,
            ActionType::PlayMemberToStage => 6,
            ActionType::SetLiveCard => 7,
            ActionType::FinishLiveCardSet => 8,
            ActionType::EnergyCharge => 9,
            ActionType::ChoiceDecision => 10,
            ActionType::ChoiceSelect => 11,
            ActionType::ChoiceSkip => 12,
            ActionType::ChoiceOption => 13,
            ActionType::ChoicePosition => 14,
            ActionType::UseAbility => 15,
            ActionType::ChooseSecondAttacker => 16,
            ActionType::ConfirmMulligan => 17,
            ActionType::SelectLiveCard => 18,
            ActionType::ConfirmLiveCardSet => 19,
            ActionType::SkipLiveCardSet => 20,
            ActionType::PassRemaining => 21,
            ActionType::Pass => 22,
            // Non-wire variants (menu headers) have no tag.
            ActionType::MulliganHeader | ActionType::LiveCardHeader => 0,
        }
    }

    /// Inverse of [`ActionType::to_tag`]. Unknown tags decode to `Pass`.
    pub fn from_tag(tag: u16) -> ActionType {
        match tag {
            0 => ActionType::RockChoice,
            1 => ActionType::PaperChoice,
            2 => ActionType::ScissorsChoice,
            3 => ActionType::ChooseFirstAttacker,
            4 => ActionType::SelectMulligan,
            5 => ActionType::SkipMulligan,
            6 => ActionType::PlayMemberToStage,
            7 => ActionType::SetLiveCard,
            8 => ActionType::FinishLiveCardSet,
            9 => ActionType::EnergyCharge,
            10 => ActionType::ChoiceDecision,
            11 => ActionType::ChoiceSelect,
            12 => ActionType::ChoiceSkip,
            13 => ActionType::ChoiceOption,
            14 => ActionType::ChoicePosition,
            15 => ActionType::UseAbility,
            16 => ActionType::ChooseSecondAttacker,
            17 => ActionType::ConfirmMulligan,
            18 => ActionType::SelectLiveCard,
            19 => ActionType::ConfirmLiveCardSet,
            20 => ActionType::SkipLiveCardSet,
            21 => ActionType::PassRemaining,
            22 => ActionType::Pass,
            _ => ActionType::Pass,
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
        if cfg!(not(feature = "profiling")) && action_display_enabled() {
            format!($($arg)*)
        } else {
            String::new()
        }
    };
}

#[derive(Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Action {
    pub description: String,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub description_ja: Option<String>,
    pub action_type: ActionType,
    pub parameters: Option<ActionParameters>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub selected: Option<bool>,
}

impl Action {
    pub fn with_ja(mut self, ja: impl Into<String>) -> Self {
        if action_display_enabled() {
            self.description_ja = Some(ja.into());
        }
        self
    }

    /// Resolve the display text for a UI language. Japanese falls back to
    /// the English description when no translation exists; every other
    /// language shows English (extension point: add `description_<code>`
    /// fields and match them here when a third language ships).
    pub fn display_desc_for(&self, lang: crate::game::language::Lang) -> &str {
        if lang == crate::game::language::Lang::Japanese {
            self.description_ja.as_deref().unwrap_or(&self.description)
        } else {
            &self.description
        }
    }

    pub fn display_desc(&self, is_ja: bool) -> &str {
        self.display_desc_for(if is_ja {
            crate::game::language::Lang::Japanese
        } else {
            crate::game::language::Lang::English
        })
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct ActionParameters {
    pub card_id: Option<i16>,      // Database card ID - reliable identifier
    pub card_index: Option<usize>, // Array position - kept for backward compatibility
    pub card_indices: Option<Vec<usize>>, // For selecting multiple cards (e.g., live cards)
    pub stage_area: Option<String>, // "left", "center", "right"
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub stage_area_index: Option<u8>,
    pub use_baton_touch: Option<bool>, // Whether to use baton touch cost reduction
    // Card grouping information for improved UI
    pub card_name: Option<String>,
    pub card_no: Option<String>,
    pub ability_index: Option<usize>, // Which ability on the card (for use_ability actions)
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub source_ability: Option<String>, // Full ability text block (for display)
    pub base_cost: Option<u8>,
    pub final_cost: Option<u8>,
    pub available_areas: Option<Arc<Vec<AreaInfo>>>,
    pub double_baton_pairs: Option<Arc<Vec<DoubleBatonPair>>>, // Available double baton pair+placement options
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub disabled: Option<bool>, // Card is visible but not selectable (greyscale)
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct AreaInfo {
    pub area: ArcStr,
    pub available: bool,
    pub cost: u8,
    pub is_baton_touch: bool,
    pub existing_member_name: Option<ArcStr>,
}

#[derive(Clone, Copy)]
struct DoubleBatonOption {
    areas: [MemberArea; 2],
    placement: MemberArea,
    cost: u8,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct DoubleBatonPair {
    pub areas: Vec<String>, // The 2 members to replace (e.g., ["left", "center"])
    pub placement: String,  // Where the card ends up (e.g., "left")
    pub cost: u8,           // Effective cost after both cost reductions
}

/// Build two (unshuffled) deck templates from card-number lists, with default
/// energy cards added.
/// Every engine bin previously inlined this same 6-step block; callers clone + shuffle
/// templates per game as needed.
pub fn build_two_decks(
    db: &mut crate::compat::Arc<crate::card::CardDatabase>,
    n1: &[String],
    n2: &[String],
) -> Result<(crate::deck_builder::Deck, crate::deck_builder::Deck), String> {
    // NOTE: `db` must be the caller's own uniquely-referenced Arc, passed as
    // `&mut`. The deck builders register per-copy card IDs via
    // `Arc::make_mut`, which silently deep-clones a shared Arc and throws the
    // copies away — leaving decks full of dangling card IDs if a temporary
    // clone is used here (that bug made every bin-built game unplayable).
    use crate::deck_builder::DeckBuilder;
    let mut t1 = DeckBuilder::build_deck_from_database(db, n1.to_vec())
        .map_err(|e| format!("P1 deck: {e}"))?;
    let mut t2 = DeckBuilder::build_deck_from_database(db, n2.to_vec())
        .map_err(|e| format!("P2 deck: {e}"))?;
    DeckBuilder::add_default_energy_cards_from_database(&mut t1, db).ok();
    DeckBuilder::add_default_energy_cards_from_database(&mut t2, db).ok();
    Ok((t1, t2))
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

/// Advance exactly one automatic phase (no pending choice), returning true if
/// it did. Semantics identical to the inline `if !has_pending_choice { match
/// phase { auto => advance; continue } }` blocks each engine bin used to carry.
pub fn auto_advance_one(game_state: &mut GameState) -> bool {
    if game_state.has_pending_choice()
        || game_state.game_result != crate::game_state::GameResult::Ongoing
    {
        return false;
    }
    if is_automatic_phase(game_state) {
        crate::turn::TurnEngine::advance_phase(game_state);
        true
    } else {
        false
    }
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
    for i in 0..500 {
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
        if i == 499 {
            // The 500-iteration cap tripped: the auto-advance loop is spinning
            // without reaching a decision or a terminal result. Log the state
            // so the spinning phase/choice is diagnosable from RUST_LOG alone.
            log::debug!(
                "[SETTLE_AUTO_CAP] phase={:?} turn={} pending_choice={} result={:?} rule_log={}",
                gs.current_phase,
                gs.turn_number,
                gs.has_pending_choice(),
                gs.game_result,
                gs.rule_log.len()
            );
        }
    }
}

fn stage_area_from_parameters(params: &ActionParameters) -> Option<MemberArea> {
    params
        .stage_area_index
        .and_then(|index| MemberArea::from_index(index as usize))
        .or_else(|| params.stage_area.as_deref()?.parse().ok())
}

/// Execute a game action extracted from the action parameters.
/// Returns Ok(()) on success, Err(message) on failure. Always resets loop detection.
pub fn execute_action(gs: &mut GameState, action: &Action) -> Result<(), String> {
    let (card_id, card_indices, stage_area, use_baton_touch, ability_index) = action
        .parameters
        .as_ref()
        .map(|p| {
            (
                p.card_id,
                p.card_indices.clone(),
                stage_area_from_parameters(p),
                p.use_baton_touch,
                p.ability_index,
            )
        })
        .unwrap_or((None, None, None, None, None));
    let result = TurnEngine::execute_main_phase_action_with_ability_index(
        gs,
        &action.action_type,
        card_id,
        card_indices,
        stage_area,
        use_baton_touch,
        ability_index,
    );
    gs.reset_loop_detection();
    result
}

/// Run a quick AI-vs-AI test game using the given cards and deck lists.
/// Returns the number of actions executed, or an error string.
pub fn test_ai_vs_ai(
    cards: &[crate::card::Card],
    d1: &crate::deck_parser::DeckList,
    d2: &crate::deck_parser::DeckList,
    max_turns: u8,
) -> Result<usize, String> {
    use crate::card::CardDatabase;
    use crate::compat::Arc;
    use crate::deck_parser::DeckParser;
    use crate::game::deck_builder::DeckBuilder;
    use crate::player::Player;

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
        description: if action_display_enabled() {
            description.into()
        } else {
            String::new()
        },
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
        description: if action_display_enabled() {
            description.into()
        } else {
            String::new()
        },
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
        stage_area_index: None,
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

/// Pay/skip (or yes/no) decision pair. ONE builder for the PAY_SKIP and
/// pay_cost_all arms of `generate_pending_choice_actions` below, which
/// only differ in labels and the pay card_no.
fn make_pay_skip_pair(
    pay_label: String,
    pay_label_ja: String,
    pay_card_no: &str,
    skip_label: String,
    skip_label_ja: String,
) -> Vec<Action> {
    vec![
        make_action_params(
            ActionType::ChoiceDecision,
            &pay_label,
            ActionParameters {
                card_id: Some(1),
                card_no: Some(pay_card_no.to_string()),
                ..make_params()
            },
        )
        .with_ja(pay_label_ja),
        make_action_params(
            ActionType::ChoiceDecision,
            &skip_label,
            ActionParameters {
                card_id: Some(0),
                card_no: Some("skip_optional_cost".to_string()),
                ..make_params()
            },
        )
        .with_ja(skip_label_ja),
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
            if target == crate::ability::types::PAY_SKIP_TARGET {
                let desc_en = choice.description_en().unwrap_or(description);
                let desc_ja = choice.description_ja().unwrap_or(desc_en);
                // Use allow_skip to determine if this is a cost (skip) or yes/no (no)
                let is_cost = *allow_skip;
                let pay_label = if desc_en.is_empty() {
                    if is_cost { "Pay optional cost" } else { "Yes" }.to_string()
                } else {
                    desc_en.to_string()
                };
                let skip_label = if is_cost {
                    "Skip".to_string()
                } else {
                    "No".to_string()
                };
                let pay_label_ja = if desc_ja.is_empty() {
                    if is_cost { "オプショナルコストを支払う" } else { "はい" }.to_string()
                } else {
                    desc_ja.to_string()
                };
                let skip_label_ja = if is_cost {
                    "スキップ".to_string()
                } else {
                    "いいえ".to_string()
                };
                return make_pay_skip_pair(
                    pay_label,
                    pay_label_ja,
                    "pay_optional_cost",
                    skip_label,
                    skip_label_ja,
                );
            }
            if target == "pay_cost_all:discard_all" {
                let desc_en = choice.description_en().unwrap_or(description);
                let desc_ja = choice.description_ja().unwrap_or(desc_en);
                // This is always a cost, so use "Skip"
                let pay_label = if desc_en.is_empty() {
                    "Discard all hand".to_string()
                } else {
                    desc_en.to_string()
                };
                let skip_label = "Skip".to_string();
                let pay_label_ja = if desc_ja.is_empty() {
                    "手札をすべて控え室に置く".to_string()
                } else {
                    desc_ja.to_string()
                };
                let skip_label_ja = "スキップ".to_string();
                return make_pay_skip_pair(
                    pay_label,
                    pay_label_ja,
                    "pay_cost_all",
                    skip_label,
                    skip_label_ja,
                );
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
                        // Rows are bare area names (the prompt already says what
                        // is being chosen); "Select " on every row is pure noise
                        // on 30-column screens.
                        let label = if is_source {
                            action_desc!("{}", capitalize(&stage_area))
                        } else if let Some(ref src) = from_pos {
                            action_desc!("{} → {}", capitalize(src), capitalize(&stage_area))
                        } else {
                            action_desc!("Move to {}", capitalize(&stage_area))
                        };
                        let label_ja = if is_source {
                            action_desc!("{}", ja_area)
                        } else if let Some(ref src) = from_pos {
                            // Source names here are Title-Case display names
                            // ("Center") or member names, lowercased above —
                            // all covered by the shared area table.
                            action_desc!("{} → {}", area_label_ja(src), ja_area)
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
                // Bilingual options: EN from description_en, JA from
                // description (source text). Both split on " / " symmetric
                // with Choice creation; on any shape mismatch fall back to
                // the legacy JA-only actions so the choice never breaks.
                let en_parts: Vec<&str> = choice
                    .description_en()
                    .map(|en| en.split(" / ").collect())
                    .unwrap_or_default();
                let ja_parts: Vec<&str> = description.split(" / ").collect();
                let bilingual = en_parts.len() == ja_parts.len() && !ja_parts.is_empty();
                log::debug!(
                    "[CHOICE_BILINGUAL] target=choice opts={} bilingual={}",
                    ja_parts.len(),
                    bilingual
                );
                let mut actions: Vec<Action> = ja_parts
                    .iter()
                    .enumerate()
                    .map(|(i, opt_ja)| {
                        let a = make_action_params(
                            ActionType::ChoiceOption,
                            if bilingual { en_parts[i] } else { *opt_ja },
                            ActionParameters {
                                card_id: Some(i as i16),
                                card_no: Some(i.to_string()),
                                ..make_params()
                            },
                        );
                        if bilingual {
                            a.with_ja(*opt_ja)
                        } else {
                            a
                        }
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
            ref cost_values,
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
                let matches_cost_values = match cost_values {
                    Some(vals) if !vals.is_empty() => game_state
                        .card_database
                        .get_card(*card_id)
                        .and_then(|c| c.cost.or(c.score))
                        .is_some_and(|v| vals.contains(&v)),
                    _ => true,
                };
                let is_selectable =
                    in_fi && matches_chars && matches_group && matches_cost && matches_cost_values;

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
        make_action(ActionType::PaperChoice, "Paper").with_ja("パー"),
        make_action(ActionType::ScissorsChoice, "Scissors").with_ja("チョキ"),
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
    // The deciding player is always the active one (first attacker in the
    // first window, second attacker in the second).
    let player_name = if game_state.active_player_index() == 0 {
        "Player 1"
    } else {
        "Player 2"
    };
    let mulligan_player = game_state.active_player();

    let mut actions = vec![{
        let mut a = make_action_params(
            ActionType::ConfirmMulligan,
            action_desc!("Confirm {}'s mulligan", player_name),
            ActionParameters { ..make_params() },
        );
        if action_display_enabled() {
            a.description_ja = Some(action_desc!("{}のマリガンを確定", player_name));
        }
        a
    }];

    for (hand_index, card_id) in mulligan_player.hand.cards.iter().enumerate() {
        let is_selected = game_state
            .mulligan_selected_indices
            .contains(&(hand_index as u8));
        let card = game_state.card_database.get_card(*card_id);
        let card_name = card.map(|c| c.name.as_ref()).unwrap_or("Unknown");
        let display = action_display_enabled();
        let card_no_str = if display {
            card.map(|c| c.card_no.to_string()).unwrap_or_default()
        } else {
            String::new()
        };
        let mut a = make_action_params(
            ActionType::SelectMulligan,
            // Marker-prefix convention (shared with the multi-pick menus):
            // state first, name second — the title already says mulligan.
            action_desc!(
                "[{}] {}",
                if is_selected { "x" } else { " " },
                card_name
            ),
            ActionParameters {
                card_id: Some(*card_id),
                card_index: Some(hand_index),
                card_indices: Some(vec![hand_index]),
                card_name: if display {
                    Some(card_name.to_string())
                } else {
                    None
                },
                card_no: if display { Some(card_no_str) } else { None },
                ..make_params()
            },
        );
        a.selected = Some(is_selected);
        if display {
            a.description_ja = Some(action_desc!(
                "[{}] {}",
                if is_selected { "x" } else { " " },
                card_name
            ));
        }
        actions.push(a);
    }

    actions
}

fn generate_main_phase_actions(game_state: &GameState) -> Vec<Action> {
    #[cfg(not(feature = "no_std"))]
    let _timer = crate::timer::Timer::start("generate_main_phase_actions");
    let active_player = game_state.active_player();
    let mut actions =
        vec![make_action(ActionType::Pass, "Pass - End Main Phase")
            .with_ja("パス - メインフェーズ終了")];
    let stage_groups = game_state.distinct_stage_groups(&active_player.id);

    if !game_state.is_action_prohibited("play_member") {
        // Rule 7.7.2.2: Main Phase - Can play member cards to stage.
        // Bound the reserve so the largest burst right at the Main boundary
        // can't over-allocate past the 4MB DS heap before settling.
        let estimated = (active_player.hand.cards.len() * 3 + 1).min(64);
        actions.reserve(estimated);

        let display = action_display_enabled();

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
                    let waited_stage_cards: Vec<i16> = active_player
                        .stage
                        .stage
                        .iter()
                        .copied()
                        .filter(|card_id| {
                            *card_id != -1
                                && game_state.mods.get_orientation_modifier(*card_id) == Some("wait")
                        })
                        .collect();
                    let reduction = crate::ability::util::calculate_play_cost_reduction(
                        &active_player.stage,
                        &active_player.success_live_card_zone.cards,
                        hand_count,
                        *card_id,
                        &game_state.card_database,
                        &waited_stage_cards,
                    );
                    let effective_cost = card_cost.saturating_sub(reduction);
                    let active_energy_count = active_player.energy_zone.active_count();

                    // Precompute per-area baton-touch protection once per hand card
                    // instead of re-scanning the stage member's abilities for every area.
                    let baton_touch_protected: [bool; 3] = core::array::from_fn(|slot| {
                        stage_card_ids[slot] != -1
                            && stage_cards[slot].is_some_and(|existing_card| {
                                !active_player
                                    .deployed_this_turn
                                    .contains(&stage_card_ids[slot])
                                    && crate::ability::util::has_cannot_baton_touch_protection(
                                        &game_state.card_database,
                                        *card_id,
                                        existing_card,
                                    )
                            })
                    });

                    let area_names = ["left", "center", "right"];
                    let mut area_candidates: [(usize, &'static str, bool, u8, bool, Option<&str>); 3] =
                        [(0, "left", false, card_cost, false, None), (1, "center", false, card_cost, false, None), (2, "right", false, card_cost, false, None)];
                    let mut has_any_available = false;

                    for (area_idx, area_name) in area_names.iter().enumerate() {
                        let mut cost = card_cost;
                        let mut available = false;
                        let mut is_baton_touch = false;
                        let mut existing_member_name = None;

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
                                        // Include constant cost modifiers (e.g. 唐 可可 +2):
                                        // parity with core/player.rs baton payment.
                                        let member_cost = crate::constants::floored_cost(
                                            existing_member_card.cost.unwrap_or(0),
                                            game_state.mods.get_cost_modifier(existing_member_id),
                                        );
                                        let cost_to_pay =
                                            effective_cost.saturating_sub(member_cost);
                                        if (active_energy_count as u8) >= cost_to_pay {
                                            available = true;
                                            cost = cost_to_pay;
                                            is_baton_touch = true;
                                            if display {
                                                existing_member_name =
                                                    Some(existing_member_card.name.as_ref());
                                            }
                                            has_any_available = true;
                                        }
                                    }
                                }
                            }
                        } else if (active_energy_count as u8) >= effective_cost {
                            available = true;
                            cost = effective_cost;
                            has_any_available = true;
                        }
                        area_candidates[area_idx] =
                            (area_idx, area_name, available, cost, is_baton_touch, existing_member_name);
                    }

                    // Check if this card has play_baton_touch with count > 1 (double baton)
                    let has_double_baton = card.resolved_abilities().any(|ability| {
                        ability.effect.as_ref().is_some_and(|ef| {
                            ef.action == crate::ability::enums::ActionType::PlayBatonTouch
                                && ef.count.unwrap_or(1) > 1
                        })
                    });

                    let (double_baton_pairs, any_double_baton_available) = if has_double_baton {
                        let mut occupied = [None; 3];
                        let mut occupied_count = 0;
                        for idx in 0..3 {
                            if stage_card_ids[idx] != -1
                                && !active_player
                                    .deployed_this_turn
                                    .contains(&stage_card_ids[idx])
                                && !baton_touch_protected[idx]
                            {
                                occupied[occupied_count] =
                                    Some((idx, MemberArea::ALL[idx], stage_card_ids[idx]));
                                occupied_count += 1;
                            }
                        }
                        let mut pairs = [None; 6];
                        let mut pair_count = 0;
                        for i in 0..occupied_count {
                            for j in (i + 1)..occupied_count {
                                let (_idx1, area1, cid1) = occupied[i].unwrap();
                                let (_idx2, area2, cid2) = occupied[j].unwrap();
                                let baton_cost_for = |cid: i16| {
                                    game_state.card_database.get_card(cid)
                                        .and_then(|c| c.cost)
                                        .map(|base| {
                                            crate::constants::floored_cost(
                                                base,
                                                game_state.mods.get_cost_modifier(cid),
                                            )
                                        })
                                        .unwrap_or(0)
                                };
                                let pair_cost =
                                    effective_cost.saturating_sub(baton_cost_for(cid1) + baton_cost_for(cid2));
                                if (active_energy_count as u8) >= pair_cost {
                                    pairs[pair_count] = Some(DoubleBatonOption {
                                        areas: [area1, area2],
                                        placement: area1,
                                        cost: pair_cost,
                                    });
                                    pair_count += 1;
                                    pairs[pair_count] = Some(DoubleBatonOption {
                                        areas: [area1, area2],
                                        placement: area2,
                                        cost: pair_cost,
                                    });
                                    pair_count += 1;
                                }
                            }
                        }
                        let available = pair_count > 0;
                        (
                            if available {
                                Some(pairs.into_iter().flatten().collect::<Vec<_>>())
                            } else {
                                None
                            },
                            available,
                        )
                    } else {
                        (None, false)
                    };
                    let display_pairs = if display && cfg!(not(feature = "profiling")) {
                        double_baton_pairs.as_ref().map(|pairs| {
                            Arc::new(
                                pairs
                                    .iter()
                                    .map(|pair| DoubleBatonPair {
                                        areas: pair
                                            .areas
                                            .iter()
                                            .map(|area| area.to_string())
                                            .collect(),
                                        placement: pair.placement.to_string(),
                                        cost: pair.cost,
                                    })
                                    .collect(),
                            )
                        })
                    } else {
                        None
                    };
                    let available_areas = if has_any_available || any_double_baton_available {
                        Some(Arc::new(
                            area_candidates
                                .iter()
                                .map(|(_, area_name, available, cost, is_baton_touch, existing_member_name)| {
                                    AreaInfo {
                                        area: ArcStr::from(*area_name),
                                        available: *available,
                                        cost: *cost,
                                        is_baton_touch: *is_baton_touch,
                                        existing_member_name: existing_member_name
                                            .map(ArcStr::from),
                                    }
                                })
                                .collect::<Vec<_>>(),
                        ))
                    } else {
                        None
                    };

                    if has_any_available || any_double_baton_available {
                        for area in available_areas.as_ref().unwrap().iter() {
                            if !area.available {
                                continue;
                            }
                            let area_label = area_label_en(area.area.as_ref());
                            let area_label_ja = area_label_ja(area.area.as_ref());
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
                            let cost_str = if display {
                                cost_display.to_string()
                            } else {
                                String::new()
                            };
                            {
                                let mut a = make_action_params(
                                    ActionType::PlayMemberToStage,
                                    action_desc!(
                                        "E{} {} → {}{}",
                                        cost_display,
                                        card.name,
                                        area_label,
                                        bt
                                    ),
                                    ActionParameters {
                                        card_id: Some(*card_id),
                                        card_index: Some(hand_index),
                                        card_name: {
                                            if cfg!(not(feature = "profiling")) && display {
                                                Some(card.name.to_string())
                                            } else {
                                                None
                                            }
                                        },
                                        card_no: {
                                            if cfg!(not(feature = "profiling")) && display {
                                                Some(card.card_no.to_string())
                                            } else {
                                                None
                                            }
                                        },
                                        base_cost: Some(card_cost),
                                        // Area-specific price (baton discount + play-cost
                                        // reductions applied). Mini buttons and headers
                                        // read final_cost first (see web_ui ActionButtons).
                                        final_cost: Some(cost_display),
                                        stage_area: if display {
                                            Some(area.area.to_string())
                                        } else {
                                            None
                                        },
                                        stage_area_index: Some(
                                            area.area
                                                .as_ref()
                                                .parse::<MemberArea>()
                                                .map(|area| area.to_index() as u8)
                                                .unwrap_or(0),
                                        ),
                                        // available_areas is decision data for v7 baton
                                        // vision (is_baton_touch on the chosen stage_area);
                                        // always built. double_baton_pairs is UI-only.
                                        available_areas: if cfg!(feature = "profiling") {
                                            None
                                        } else {
                                            Some(Arc::clone(available_areas.as_ref().unwrap()))
                                        },
                                        double_baton_pairs: if cfg!(feature = "profiling")
                                            || !display
                                        {
                                            None
                                        } else {
                                            display_pairs.clone()
                                        },
                                        ..make_params()
                                    },
                                );
                                if display {
                                    a.description_ja = Some(action_desc!(
                                        "E{} {} → {}{}",
                                        cost_str,
                                        card.name,
                                        area_label_ja,
                                        bt_ja
                                    ));
                                }
                                actions.push(a);
                            }
                        }
                        if let Some(ref pairs) = double_baton_pairs {
                            for pair in pairs {
                                let area_indices: Vec<usize> = pair
                                    .areas
                                    .iter()
                                    .map(|area| area.to_index())
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
                                            "E{} {} ({}+{})→{}",
                                            pair.cost,
                                            card.name,
                                            src0_en,
                                            src1_en,
                                            dst_en
                                        ),
                                        ActionParameters {
                                            card_id: Some(*card_id),
                                            card_index: Some(hand_index),
                                            card_name: if cfg!(not(feature = "profiling")) && display {
                                                Some(card.name.to_string())
                                            } else {
                                                None
                                            },
                                            card_no: if cfg!(not(feature = "profiling")) && display {
                                                Some(card.card_no.to_string())
                                            } else {
                                                None
                                            },
                                            base_cost: Some(pair.cost),
                                            final_cost: Some(pair.cost),
                                            stage_area: if display {
                                                Some(pair.placement.to_string())
                                            } else {
                                                None
                                            },
                                            stage_area_index: Some(pair.placement.to_index() as u8),
                                            card_indices: Some(area_indices),
                                            available_areas: if cfg!(feature = "profiling") {
                                                None
                                            } else {
                                                Some(Arc::clone(available_areas.as_ref().unwrap()))
                                            },
                                            double_baton_pairs: if cfg!(feature = "profiling")
                                                || !display
                                            {
                                                None
                                            } else {
                                                display_pairs.clone()
                                            },
                                            ..make_params()
                                        },
                                    );
                                    if display {
                                        a.description_ja = Some(action_desc!(
                                            "E{} {} ({}+{})→{}",
                                            pair.cost,
                                            card.name,
                                            src0_ja,
                                            src1_ja,
                                            dst_ja
                                        ));
                                    }
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
                if !crate::ability::util::ability_trigger_can_activate(&ability) {
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
                if crate::ability::util::activates_from_discard(&ability) {
                    continue;
                }

                let ability_key = (card_id, ability_index, game_state.turn_number);
                if let Some(use_limit) = ability.use_limit {
                    if !crate::ability::util::ability_under_use_limit(game_state, &ability_key, use_limit) {
                        continue;
                    }
                }

                let trigger_info = ability
                    .triggers
                    .as_ref()
                    .map(|t| action_desc!(" ({})", t))
                    .unwrap_or_default();

                // Effective cost: printed total minus per-group reductions
                // (single source of truth shared with execution). Mandatory
                // costs that cannot be paid are NOT offered (rules 9.6.2.3);
                // optional-payment components keep the ability offered so the
                // player can skip just that part (wakana bp2-008).
                let (base_cost, effective_cost) = if ability.cost.is_some() {
                    crate::ability::util::ability_effective_cost(
                        game_state,
                        &ability,
                        stage_groups,
                    )
                } else {
                    (0, 0)
                };
                if let Some(c) = ability.cost.as_ref() {
                    if !c.has_optional_payment()
                        && effective_cost > active_player.energy_zone.active_count()
                    {
                        continue;
                    }
                    // Mandatory non-energy costs (discard, wait, ...) must
                    // also be payable or the ability is not offered
                    // (Rule 9.4.2.3/Q56). Optional components stay offerable
                    // (wakana/umi Q228 skip at pay time).
                    if crate::ability::resolver::AbilityResolver::validate_mandatory_cost(
                        game_state,
                        &c.0,
                        Some(card_id),
                    )
                    .is_err()
                    {
                        continue;
                    }
                }
                let mut ua = make_action_params(
                    ActionType::UseAbility,
                    action_desc!(
                        "E{} {} ({}): {}{}",
                        effective_cost,
                        card.name,
                        area_name,
                        ability.full_text,
                        trigger_info
                    ),
                    ActionParameters {
                        card_id: Some(card_id),
                        // Display-only fields; bot routes UseAbility by card_id alone.
                        stage_area: if cfg!(not(feature = "profiling")) && action_display_enabled() {
                            Some(area_name.to_string())
                        } else {
                            None
                        },
                        card_name: if cfg!(not(feature = "profiling")) && action_display_enabled() {
                            Some(card.name.to_string())
                        } else {
                            None
                        },
                        card_no: if cfg!(not(feature = "profiling")) && action_display_enabled() {
                            Some(card.card_no.to_string())
                        } else {
                            None
                        },
                        ability_index: Some(ability_index),
                        source_ability: if cfg!(not(feature = "profiling")) && action_display_enabled() {
                            Some(ability.full_text.clone())
                        } else {
                            None
                        },
                        base_cost: Some(base_cost),
                        final_cost: Some(effective_cost),
                        ..make_params()
                    },
                );
                if action_display_enabled() {
                    ua.description_ja = Some(action_desc!(
                        "E{} {} ({}): {}{}",
                        effective_cost,
                        card.name,
                        area_label_ja(area_name),
                        ability.full_text,
                        trigger_info
                    ));
                }
                actions.push(ua);
            }
        }
    }

    // Also check discard pile for cards that activate from discard
    // (activation_condition_parsed with location = discard)
    for &card_id in &active_player.waitroom.cards {
        if let Some(card) = game_state.card_database.get_card(card_id) {
            for (ability_index, ar) in card.abilities.iter().enumerate() {
                let ability = ar.resolve();
                if !crate::ability::util::activates_from_discard(&ability) {
                    continue;
                }
                if !crate::ability::util::ability_trigger_can_activate(&ability) {
                    continue;
                }

                let ability_key = (card_id, ability_index, game_state.turn_number);
                if let Some(use_limit) = ability.use_limit {
                    if !crate::ability::util::ability_under_use_limit(game_state, &ability_key, use_limit) {
                        continue;
                    }
                }

                // Same effective-cost gate as stage activations: mandatory
                // unpayable costs are withheld, optional ones stay offered.
                let (base_cost, effective_cost) = if ability.cost.is_some() {
                    crate::ability::util::ability_effective_cost(
                        game_state,
                        &ability,
                        stage_groups,
                    )
                } else {
                    (0, 0)
                };
                if let Some(c) = ability.cost.as_ref() {
                    if !c.has_optional_payment()
                        && effective_cost > active_player.energy_zone.active_count()
                    {
                        continue;
                    }
                    // Same mandatory-cost gate as stage activations above.
                    if crate::ability::resolver::AbilityResolver::validate_mandatory_cost(
                        game_state,
                        &c.0,
                        Some(card_id),
                    )
                    .is_err()
                    {
                        continue;
                    }
                }
                let mut ua = make_action_params(
                    ActionType::UseAbility,
                    action_desc!(
                        "E{} {} (discard, 起動): {}",
                        effective_cost,
                        card.name,
                        ability.full_text
                    ),
                    ActionParameters {
                        card_id: Some(card_id),
                        // Display-only fields; profiling/bot routes UseAbility by card_id.
                        card_name: if cfg!(not(feature = "profiling")) && action_display_enabled() {
                            Some(card.name.to_string())
                        } else {
                            None
                        },
                        card_no: if cfg!(not(feature = "profiling")) && action_display_enabled() {
                            Some(card.card_no.to_string())
                        } else {
                            None
                        },
                        ability_index: Some(ability_index),
                        source_ability: if cfg!(not(feature = "profiling")) && action_display_enabled() {
                            Some(ability.full_text.clone())
                        } else {
                            None
                        },
                        base_cost: Some(base_cost),
                        final_cost: Some(effective_cost),
                        ..make_params()
                    },
                );
                if action_display_enabled() {
                    ua.description_ja = Some(action_desc!(
                        "E{} {} (控え室, 起動): {}",
                        effective_cost,
                        card.name,
                        ability.full_text
                    ));
                }
                actions.push(ua);
            }
        }
    }

    actions
}

fn generate_live_card_set_actions(game_state: &GameState) -> Vec<Action> {
    #[cfg(not(feature = "no_std"))]
    let _timer = crate::timer::Timer::start("generate_live_card_set_actions");
    let active_player = game_state.active_player();

    // The deciding player is always the active one (first attacker in the
    // first window, second attacker in the second).
    let player_name = if game_state.active_player_index() == 0 {
        "Player 1"
    } else {
        "Player 2"
    };

    let mut actions = vec![{
        let mut a = make_action_params(
            ActionType::ConfirmLiveCardSet,
            action_desc!("Confirm {}'s live card set", player_name),
            ActionParameters { ..make_params() },
        );
        if action_display_enabled() {
            a.description_ja = Some(action_desc!("{}のライブカードセットを確定", player_name));
        }
        a
    }];

    let max_live_cards =
        3i32 - i32::try_from(active_player.live_card_set_limit_reduction).unwrap_or(0);
    let already_selected = game_state.live_card_selected_indices.len();
    let max_allowed = max_live_cards.max(0) as usize;
    for (hand_index, card_id) in active_player.hand.cards.iter().enumerate() {
        let is_selected = game_state
            .live_card_selected_indices
            .contains(&(hand_index as u8));
        let at_limit = already_selected >= max_allowed;
        if at_limit && !is_selected {
            continue;
        }
        let card = game_state.card_database.get_card(*card_id);
        let card_name = card.map(|c| c.name.as_ref()).unwrap_or("Unknown");
        let display = action_display_enabled();
        let card_no_str = if display {
            card.map(|c| c.card_no.to_string()).unwrap_or_default()
        } else {
            String::new()
        };
        let mut a = make_action_params(
            ActionType::SelectLiveCard,
            // Same marker-prefix convention as mulligan rows.
            action_desc!(
                "[{}] {}",
                if is_selected { "x" } else { " " },
                card_name
            ),
            ActionParameters {
                card_id: Some(*card_id),
                card_index: Some(hand_index),
                card_indices: Some(vec![hand_index]),
                card_name: if display {
                    Some(card_name.to_string())
                } else {
                    None
                },
                card_no: if display { Some(card_no_str) } else { None },
                ..make_params()
            },
        );
        a.selected = Some(is_selected);
        if display {
            a.description_ja = Some(action_desc!(
                "[{}] {}",
                if is_selected { "x" } else { " " },
                card_name
            ));
        }
        actions.push(a);
    }

    actions
}
