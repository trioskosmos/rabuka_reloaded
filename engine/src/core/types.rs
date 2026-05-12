use crate::card::AbilityEffect;

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
    MulliganP1Turn,
    MulliganP2Turn,
    Active,
    Energy,
    Draw,
    Main,
    LiveCardSetP1Turn,
    LiveCardSetP2Turn,
    FirstAttackerPerformance,
    SecondAttackerPerformance,
    LiveVictoryDetermination,
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::RockPaperScissors => write!(f, "RockPaperScissors"),
            Phase::ChooseFirstAttacker => write!(f, "ChooseFirstAttacker"),
            Phase::MulliganP1Turn => write!(f, "MulliganP1Turn"),
            Phase::MulliganP2Turn => write!(f, "MulliganP2Turn"),
            Phase::Active => write!(f, "Active"),
            Phase::Energy => write!(f, "Energy"),
            Phase::Draw => write!(f, "Draw"),
            Phase::Main => write!(f, "Main"),
            Phase::LiveCardSetP1Turn => write!(f, "LiveCardSetP1Turn"),
            Phase::LiveCardSetP2Turn => write!(f, "LiveCardSetP2Turn"),
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
    pub total_hearts: [u32; 7],
    pub allocations: Vec<Allocation>,
    pub heart_sources: Vec<HeartSource>,
    pub blade_sources: Vec<BladeSource>,
    pub draw_effects_occurred: bool,
}

// ============== PERFORMANCE SNAPSHOT ==============

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceSnapshot {
    pub turn: u32,
    pub player_id: String,
    pub lives: Vec<LiveCardResult>,
    pub member_contributions: Vec<MemberContribution>,
    pub yell_cards: Vec<YellCardResult>,
    pub total_hearts: [u32; 7],
    pub total_score: u32,
    pub success: bool,
    pub note_icons: u32,
    pub yell_count: u32,
    pub breakdown: Breakdown,
    pub triggered_abilities: Vec<TriggeredAbility>,
    pub p0_wins: bool,
    pub p1_wins: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LiveCardResult {
    pub passed: bool,
    pub score: u32,
    pub spare: [u32; 7],
    pub required: [u32; 7],
    pub filled: [u32; 7],
    pub adjustments: Vec<Adjustment>,
    pub card_id: i16,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemberContribution {
    pub source_id: i16,
    pub slot: usize,
    pub base_hearts: [u32; 7],
    pub bonus_hearts: [u32; 7],
    pub base_blades: u32,
    pub bonus_blades: u32,
    pub base_notes: u32,
    pub bonus_notes: u32,
    pub draw_icons: u32,
    pub ability_heart_bonuses: Vec<AbilityBonus>,
    pub ability_blade_bonuses: Vec<AbilityBonus>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct YellCardResult {
    pub card_id: i16,
    pub blade_hearts: [u32; 7],
    pub note_icons: u32,
    pub draw_icons: u32,
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
    pub value: [u32; 7],
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TriggeredAbility {
    pub source_card_id: i16,
    pub name: String,
    pub card_name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Adjustment {
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
