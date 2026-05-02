use crate::card::AbilityEffect;
use serde_json::Value;

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
#[allow(dead_code)]
pub enum ExecutionContext {
    None,
    SingleEffect { effect_index: usize },
    SequentialEffects { current_index: usize, effects: Vec<AbilityEffect> },
    LookAndSelect { step: LookAndSelectStep },
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum LookAndSelectStep {
    LookAt { count: usize, source: String },
    Select { count: usize },
    Finalize { destination: String },
}

impl Choice {
    /// Convert to the JSON format expected by the frontend.
    /// Flattens enum variants and adds frontend-specific fields (choose_count, v_remaining, title).
    pub fn to_frontend_json(&self) -> Option<Value> {
        let mut json = serde_json::to_value(self).ok()?;
        match self {
            Choice::SelectCard { count, description, .. } => {
                let obj = json.as_object_mut()?;
                if let Some(inner) = obj.remove("SelectCard") {
                    if let Some(mut fields) = inner.as_object().cloned() {
                        fields.insert("choose_count".into(), Value::Number((*count).into()));
                        fields.insert("v_remaining".into(), Value::Number((-1i64).into()));
                        fields.insert("title".into(), Value::String(description.clone()));
                        *obj = fields;
                    }
                }
            }
            Choice::SelectTarget { description, .. } => {
                if let Some(obj) = json.as_object_mut() {
                    if let Some(inner) = obj.remove("SelectTarget") {
                        if let Some(mut fields) = inner.as_object().cloned() {
                            fields.insert("title".into(), Value::String(description.clone()));
                            *obj = fields;
                        }
                    }
                }
            }
            _ => {}
        }
        Some(json)
    }
}
