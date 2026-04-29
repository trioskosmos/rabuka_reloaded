#[derive(Debug, Clone)]
pub struct CostCalculation {
    pub payable: bool,
    pub reason: Option<String>,
    pub cost_description: String,
}

#[derive(Debug, Clone)]
pub struct AbilityValidation {
    pub can_execute: bool,
    pub conditions_met: bool,
    pub cost_payable: bool,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub enum Choice {
    SelectCard {
        zone: String,
        card_type: Option<String>,
        count: usize,
        description: String,
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
}
