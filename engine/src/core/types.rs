use crate::card::{AbilityEffect, HeartColor};
use crate::core::game_modifiers::ModifierEntry;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbilityTrigger {
    Activation,
    Debut,
    LiveStart,
    LiveSuccess,
    Constant,
    Auto,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnPhase {
    FirstAttackerNormal,
    SecondAttackerNormal,
    Live,
}

impl std::fmt::Display for TurnPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TurnPhase::FirstAttackerNormal => write!(f, "FirstAttackerNormal"),
            TurnPhase::SecondAttackerNormal => write!(f, "SecondAttackerNormal"),
            TurnPhase::Live => write!(f, "Live"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    RockPaperScissors,
    ChooseFirstAttacker,
    MulliganFirstAttacker,
    MulliganSecondAttacker,
    Active,
    Energy,
    Draw,
    Main,
    LiveCardSetFirstAttacker,
    LiveCardSetSecondAttacker,
    FirstAttackerPerformance,
    SecondAttackerPerformance,
    LiveVictoryDetermination,
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::RockPaperScissors => write!(f, "RockPaperScissors"),
            Phase::ChooseFirstAttacker => write!(f, "ChooseFirstAttacker"),
            Phase::MulliganFirstAttacker => write!(f, "MulliganFirstAttacker"),
            Phase::MulliganSecondAttacker => write!(f, "MulliganSecondAttacker"),
            Phase::Active => write!(f, "Active"),
            Phase::Energy => write!(f, "Energy"),
            Phase::Draw => write!(f, "Draw"),
            Phase::Main => write!(f, "Main"),
            Phase::LiveCardSetFirstAttacker => write!(f, "LiveCardSetFirstAttacker"),
            Phase::LiveCardSetSecondAttacker => write!(f, "LiveCardSetSecondAttacker"),
            Phase::FirstAttackerPerformance => write!(f, "FirstAttackerPerformance"),
            Phase::SecondAttackerPerformance => write!(f, "SecondAttackerPerformance"),
            Phase::LiveVictoryDetermination => write!(f, "LiveVictoryDetermination"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameResult {
    FirstAttackerWins,
    SecondAttackerWins,
    Draw,
    Ongoing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Duration {
    LiveEnd,
    ThisTurn,
    ThisLive,
    Permanent,
    AsLongAs,
    Unless,
}

#[derive(Debug, Clone)]
pub struct TemporaryEffect {
    pub effect_type: String,
    pub duration: Duration,
    pub created_turn: u32,
    pub created_phase: Phase,
    pub target_player_id: String,
    pub description: String,
    pub creation_order: u32,
    pub effect_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct ReplacementEffect {
    pub card_id: i16,
    pub player_id: String,
    pub original_event: String,
    pub replacement_effects: Vec<AbilityEffect>,
    pub is_choice_based: bool,
    pub applied_this_event: bool,
}

// ============== LIVE PERFORMANCE DATA (intermediate, from player_perform_live) ==============

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LivePerformanceData {
    pub yell_count: u32,
    pub note_icons: u32,
    pub revealed_ids: Vec<i16>,
    pub member_contributions: Vec<MemberContribution>,
    pub yell_cards: Vec<YellCardResult>,
    pub total_hearts: [u32; 8],
    pub allocations: Vec<Allocation>,
    pub heart_sources: Vec<HeartSource>,
    pub blade_sources: Vec<BladeSource>,
    pub draw_effects_occurred: bool,
    pub live_card_ids: Vec<i16>,
}

// ============== PERFORMANCE SNAPSHOT ==============

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceSnapshot {
    pub turn: u32,
    pub player_id: String,
    pub lives: Vec<LiveCardResult>,
    pub member_contributions: Vec<MemberContribution>,
    pub yell_cards: Vec<YellCardResult>,
    pub total_hearts: [u32; 8],
    pub total_score: u32,
    pub success: bool,
    pub note_icons: u32,
    pub yell_count: u32,
    pub breakdown: Breakdown,
    pub triggered_abilities: Vec<TriggeredAbility>,
    pub p0_wins: bool,
    pub p1_wins: bool,
    /// Snapshot of need_heart_modifiers at performance time (includes live_start
    /// and other non-constant changes that get cleared by
    /// evaluate_success_zone_heart_reductions during victory determination).
    /// Merged back during snapshot finalization for correct required/adjustments/passed.
    pub performance_need_heart_modifiers: HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LiveCardResult {
    pub passed: bool,
    pub score: u32,
    pub base_score: u32,
    pub spare: [u32; 8],
    pub required: [u32; 8],
    pub filled: [u32; 8],
    pub adjustments: Vec<Adjustment>,
    pub card_id: i16,
    pub card_no: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemberContribution {
    pub source_id: i16,
    pub slot: usize,
    pub base_hearts: [u32; 8],
    pub bonus_hearts: [u32; 8],
    pub base_blades: u32,
    pub bonus_blades: u32,
    pub base_notes: u32,
    pub bonus_notes: u32,
    pub draw_icons: u32,
    pub ability_heart_bonuses: Vec<AbilityBonus>,
    pub ability_blade_bonuses: Vec<AbilityBonus>,
    pub card_no: String,
    pub is_wait: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct YellCardResult {
    pub card_id: i16,
    pub blade_hearts: [u32; 8],
    pub note_icons: u32,
    pub draw_icons: u32,
    pub card_no: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Breakdown {
    pub hearts: Vec<HeartSource>,
    pub blades: Vec<BladeSource>,
    pub allocations: Vec<Allocation>,
    pub requirements: Vec<EffectEntry>,
    pub transforms: Vec<EffectEntry>,
    pub scores: Vec<ScoreLine>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeartSource {
    pub source_type: String,
    pub source: String,
    pub value: [u32; 8],
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BladeSource {
    pub source_type: String,
    pub source: String,
    pub value: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Allocation {
    pub target_idx: usize,
    pub target_name: String,
    pub source_type: String,
    pub source_name: String,
    pub source_slot: Option<usize>,
    pub wildcard: bool,
    pub color: usize,
    pub amount: u32,
    pub is_bonus: bool,
    /// Phase tag emitted by the engine's compute_allocations so the UI
    /// can display steps without re-deriving allocation logic.
    /// Values (current engine):
    ///   "1a_colored"        — matching colored → specific color req
    ///   "1b_h00_wild"       — Heart00 wildcard → remaining color deficit
    ///   "2_wildcard"        — Heart00 wildcard → color deficit (second pass)
    ///   "3a_colored_surplus" — colored surplus → Heart00 req (demand-aware)
    ///   "3b_h00"            — Heart00 → remaining Heart00 req
    ///   "4_all_cleanup"     — icon_all → ANY remaining deficit (color first)
    /// Legacy values (old engine, kept for backward compat):
    ///   "1c_all_wild", "3c_all"
    pub phase: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EffectEntry {
    pub source: String,
    pub value: String,
    pub desc: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScoreLine {
    pub source: String,
    pub value: u32,
}

/// A structured record of a card movement event, capturing not just what moved
/// but WHAT CAUSED the move (which card's effect/ability). Replaces the old
/// pattern of separate tracking fields (recently_moved_cards, last_area_move_card_id,
/// last_area_move_by_player, etc.) with a unified event log.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MovementEvent {
    /// The card that physically moved.
    pub moved_card_id: i16,
    /// The zone the card moved FROM (e.g. "stage", "hand", "deck").
    pub source_zone: String,
    /// The zone the card moved TO (e.g. "waitroom", "hand", "stage").
    pub dest_zone: String,
    /// The card whose ability/action caused the move (None = rule/cost with no source card).
    pub cause_card_id: Option<i16>,
    /// The player whose effect/action caused the move.
    pub cause_player_id: String,
    /// Whether the move was caused by a card effect (true) vs cost/rules (false).
    pub effect_only: bool,
    /// Monotonically increasing counter for ordering events.
    pub timestamp: u32,
}

/// Records that a specific ability applied a modifier during execution,
/// so the source can be traced back to the originating card + ability text.
/// Populated in effect handlers (execute_gain_resource etc.) and consumed
/// by build_snapshot to fill ability_heart_bonuses, ability_blade_bonuses,
/// Breakdown.scores, Breakdown.transforms, and TriggeredAbility.
#[derive(Debug, Clone)]
pub struct AbilityApplication {
    pub source_card_id: i16,
    pub ability_text: String,
    pub effect_type: String, // "heart_bonus", "blade_bonus", "score_bonus", "transform", "need_heart_mod"
    pub target_card_id: i16,
    pub heart_color: Option<usize>,
    pub amount: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TriggeredAbility {
    pub source_card_id: i16,
    pub name: String,
    pub card_name: String,
    pub effect_text: String,
    pub condition_text: Option<String>,
    pub is_public: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Adjustment {
    #[serde(rename = "type")]
    pub adjustment_type: String,
    pub desc: String,
    pub value: i32,
    pub color: usize,
    pub source: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AbilityBonus {
    pub source: String,
    pub amount: u32,
    pub color: Option<usize>,
    pub ability_text: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    pub text: String,
    pub turn: u32,
    pub player_label: String,
    pub source_card_id: Option<i16>,
    pub source_card_name: Option<String>,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Condition evaluation result for a single condition on a jyouji ability.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConditionResult {
    pub text: String,
    pub passed: bool,
}

/// Status of a single constant (常時) ability on the board.
/// Exposed to the frontend for the jyouji summary bar.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConstantAbilityStatus {
    pub card_id: i16,
    pub card_name: String,
    pub owner: String,
    pub zone: String,
    pub ability_text: String,
    pub all_conditions_met: bool,
    pub conditions: Vec<ConditionResult>,
}
