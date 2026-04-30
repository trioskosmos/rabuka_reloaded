use crate::card::AbilityEffect;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Choice {
    SelectCard {
        zone: String,
        card_type: Option<String>,
        count: usize,
        description: String,
        allow_skip: bool,
    },
    SelectTarget {
        target: String,
        description: String,
    },
    SelectPosition {
        position: String,
        description: String,
    },
    SelectHeartColor {
        count: usize,
        options: Vec<String>,
        description: String,
    },
    SelectHeartType {
        count: usize,
        options: Vec<String>,
        description: String,
    },
}

#[derive(Debug, Clone)]
pub enum ChoiceResult {
    CardSelected { indices: Vec<usize> },
    TargetSelected { target: String },
    PositionSelected { position: String },
    HeartColorSelected { colors: Vec<String> },
    HeartTypeSelected { types: Vec<String> },
    Skip,
}

#[derive(Debug, Clone)]
pub enum ExecutionContext {
    None,
    SingleEffect { effect_index: usize },
    SequentialEffects { current_index: usize, effects: Vec<AbilityEffect> },
    LookAndSelect { step: LookAndSelectStep },
}

#[derive(Debug, Clone)]
pub enum LookAndSelectStep {
    LookAt { count: usize, source: String },
    Select { count: usize },
    Finalize { destination: String },
}
