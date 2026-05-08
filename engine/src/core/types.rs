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
