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
        #[serde(default)]
        is_select_action: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        heart_colors: Vec<String>,
        #[serde(default)]
        name_fragments: Option<Vec<String>>,
    },
    SelectTarget {
        target: String,
        description: String,
        allow_skip: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        options: Option<Vec<String>>,
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
    SingleEffect {
        effect_index: usize,
    },
    LookAndSelect {
        step: LookAndSelectStep,
    },
    MoveCardsPosition {
        card_id: i16,
        state_change: Option<String>,
        target: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum LookAndSelectStep {
    LookAt { count: usize, source: String },
    Select { count: usize },
    Finalize { destination: String },
}

pub struct ChoiceBuilder {
    zone: String,
    card_type: Option<String>,
    count: usize,
    description: String,
    allow_skip: bool,
    cost_limit: Option<u32>,
    cost_limit_operator: Option<String>,
    group: Option<String>,
    characters: Option<Vec<String>>,
    filtered_indices: Option<Vec<usize>>,
    is_select_action: bool,
    heart_colors: Vec<String>,
    name_fragments: Option<Vec<String>>,
}

impl ChoiceBuilder {
    pub fn build(self) -> Choice {
        Choice::SelectCard {
            zone: self.zone,
            card_type: self.card_type,
            count: self.count,
            description: self.description,
            allow_skip: self.allow_skip,
            cost_limit: self.cost_limit,
            cost_limit_operator: self.cost_limit_operator,
            group: self.group,
            characters: self.characters,
            filtered_indices: self.filtered_indices,
            is_select_action: self.is_select_action,
            heart_colors: self.heart_colors,
            name_fragments: self.name_fragments,
        }
    }

    pub fn card_type(mut self, v: Option<String>) -> Self {
        self.card_type = v;
        self
    }
    pub fn cost_limit(mut self, v: Option<u32>, op: Option<String>) -> Self {
        self.cost_limit = v;
        self.cost_limit_operator = op;
        self
    }
    pub fn group(mut self, v: Option<String>) -> Self {
        self.group = v;
        self
    }
    pub fn characters(mut self, v: Option<Vec<String>>) -> Self {
        self.characters = v;
        self
    }
    pub fn heart_colors(mut self, v: Vec<String>) -> Self {
        self.heart_colors = v;
        self
    }
    pub fn name_fragments(mut self, v: Option<Vec<String>>) -> Self {
        self.name_fragments = v;
        self
    }
    pub fn filtered_indices(mut self, v: Option<Vec<usize>>) -> Self {
        self.filtered_indices = v;
        self
    }
    pub fn is_select_action(mut self, v: bool) -> Self {
        self.is_select_action = v;
        self
    }
}

impl Choice {
    pub fn select_target(
        target: impl Into<String>,
        description: impl Into<String>,
        allow_skip: bool,
    ) -> Self {
        Choice::SelectTarget {
            target: target.into(),
            description: description.into(),
            allow_skip,
            options: None,
        }
    }

    pub fn select_target_with_options(
        target: impl Into<String>,
        description: impl Into<String>,
        allow_skip: bool,
        options: Vec<String>,
    ) -> Self {
        Choice::SelectTarget {
            target: target.into(),
            description: description.into(),
            allow_skip,
            options: Some(options),
        }
    }

    pub fn select_cards(
        zone: impl Into<String>,
        count: usize,
        description: impl Into<String>,
        allow_skip: bool,
    ) -> ChoiceBuilder {
        ChoiceBuilder {
            zone: zone.into(),
            card_type: None,
            count,
            description: description.into(),
            allow_skip,
            cost_limit: None,
            cost_limit_operator: None,
            group: None,
            characters: None,
            filtered_indices: None,
            is_select_action: false,
            heart_colors: vec![],
            name_fragments: None,
        }
    }

    /// Convert to the JSON format expected by the frontend.
    /// Flattens enum variants and adds frontend-specific fields (choose_count, v_remaining, title).
    pub fn to_frontend_json(&self) -> Option<Value> {
        let mut json = serde_json::to_value(self).ok()?;
        match self {
            Choice::SelectCard {
                count, description, ..
            } => {
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
            Choice::SelectTarget {
                target: _,
                description,
                allow_skip,
                options,
            } => {
                if let Some(obj) = json.as_object_mut() {
                    if let Some(inner) = obj.remove("SelectTarget") {
                        if let Some(mut fields) = inner.as_object().cloned() {
                            fields.insert("title".into(), Value::String(description.clone()));
                            fields.insert("allow_skip".into(), Value::Bool(*allow_skip));
                            if let Some(opts) = options {
                                fields.insert(
                                    "options".into(),
                                    Value::Array(
                                        opts.iter().map(|o| Value::String(o.clone())).collect(),
                                    ),
                                );
                            }
                            *obj = fields;
                        }
                    }
                }
            }
            Choice::SelectPosition {
                position: _,
                description,
                allow_skip,
            } => {
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
            Choice::SelectHeartColor {
                options,
                description,
                ..
            } => {
                if let Some(obj) = json.as_object_mut() {
                    if let Some(inner) = obj.remove("SelectHeartColor") {
                        if let Some(mut fields) = inner.as_object().cloned() {
                            fields.insert("title".into(), Value::String(description.clone()));
                            fields.insert(
                                "options".into(),
                                serde_json::to_value(options).unwrap_or_default(),
                            );
                            *obj = fields;
                        }
                    }
                }
            }
            Choice::SelectHeartType {
                options,
                description,
                ..
            } => {
                if let Some(obj) = json.as_object_mut() {
                    if let Some(inner) = obj.remove("SelectHeartType") {
                        if let Some(mut fields) = inner.as_object().cloned() {
                            fields.insert("title".into(), Value::String(description.clone()));
                            fields.insert(
                                "options".into(),
                                serde_json::to_value(options).unwrap_or_default(),
                            );
                            *obj = fields;
                        }
                    }
                }
            }
        }
        Some(json)
    }
}
