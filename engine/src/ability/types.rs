use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Choice {
    SelectCard {
        zone: String,
        card_type: Option<String>,
        count: usize,
        description: String,
        allow_skip: bool,
        cost_limit: Option<u32>,
        cost_limit_operator: Option<String>,
        group: Option<String>,
        characters: Option<Vec<String>>,
        #[serde(default)]
        filtered_indices: Option<Vec<usize>>,
    },
    SelectTarget {
        target: String,
        description: String,
        allow_skip: bool,
    },
    SelectPosition {
        position: String,
        description: String,
        allow_skip: bool,
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

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionContext {
    None,
    SingleEffect { effect_index: usize },
    LookAndSelect { step: LookAndSelectStep },
}

#[derive(Debug, Clone, PartialEq)]
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
            Choice::SelectTarget { target: _, description, allow_skip } => {
                if let Some(obj) = json.as_object_mut() {
                    if let Some(inner) = obj.remove("SelectTarget") {
                        if let Some(mut fields) = inner.as_object().cloned() {
                            fields.insert("title".into(), Value::String(description.clone()));
                            fields.insert("allow_skip".into(), Value::Bool(*allow_skip));
                            *obj = fields;
                        }
                    }
                }
            }
            Choice::SelectPosition { position: _, description, allow_skip } => {
                if let Some(obj) = json.as_object_mut() {
                    if let Some(inner) = obj.remove("SelectPosition") {
                        if let Some(mut fields) = inner.as_object().cloned() {
                            fields.insert("title".into(), Value::String(description.clone()));
                            fields.insert("allow_skip".into(), Value::Bool(*allow_skip));
                            *obj = fields;
                        }
                    }
                }
            }
            Choice::SelectHeartColor { options, description, .. } => {
                if let Some(obj) = json.as_object_mut() {
                    if let Some(inner) = obj.remove("SelectHeartColor") {
                        if let Some(mut fields) = inner.as_object().cloned() {
                            fields.insert("title".into(), Value::String(description.clone()));
                            fields.insert("options".into(), serde_json::to_value(options).unwrap_or_default());
                            *obj = fields;
                        }
                    }
                }
            }
            Choice::SelectHeartType { options, description, .. } => {
                if let Some(obj) = json.as_object_mut() {
                    if let Some(inner) = obj.remove("SelectHeartType") {
                        if let Some(mut fields) = inner.as_object().cloned() {
                            fields.insert("title".into(), Value::String(description.clone()));
                            fields.insert("options".into(), serde_json::to_value(options).unwrap_or_default());
                            *obj = fields;
                        }
                    }
                }
            }
        }
        Some(json)
    }
}
