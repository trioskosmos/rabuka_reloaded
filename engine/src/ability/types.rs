use crate::card::CardDatabase;
use std::sync::Arc;

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
        cost_total: Option<u32>,
        cost_total_operator: Option<String>,
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
        #[serde(default)]
        target_player_id: Option<String>,
        #[serde(default)]
        blind: bool,
        #[serde(default)]
        is_reveal: bool,
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
    /// Player chooses which of their standby auto abilities resolves first (Rule 9.5.3).
    SelectAutoAbility {
        player_id: String,
        options: Vec<AutoAbilityOption>,
        description: String,
    },
    /// Player chooses which live card goes to success zone (Rule 8.4.7).
    SelectLiveSuccess {
        player_id: String,
        count: usize,
        options: Vec<LiveSuccessOption>,
        description: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LiveSuccessOption {
    pub card_name: String,
    pub card_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AutoAbilityOption {
    pub card_name: String,
    pub ability_text: String,
    pub queue_index: usize,
}

#[derive(Debug, Clone)]
pub enum ChoiceResult {
    CardSelected { indices: Vec<usize> },
    TargetSelected { target: String },
    PositionSelected { position: String },
    HeartColorSelected { colors: Vec<String> },
    HeartTypeSelected { types: Vec<String> },
    AutoAbilitySelected { queue_index: usize },
    LiveSuccessSelected { card_index: usize },
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

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Raw effect (fallback for types that don't have a compile method yet)
    Effect(crate::card::AbilityEffect),
    /// Move a specific card that was selected by a previous SelectCards
    MoveCard {
        card_id: i16,
        destination: String,
        target: String,
        state_change: Option<String>,
    },
    /// Inline choice (for the rare case of creating a choice mid-command)
    Choice(Choice),
}

pub struct ChoiceBuilder {
    zone: String,
    card_type: Option<String>,
    count: usize,
    description: String,
    allow_skip: bool,
    cost_limit: Option<u32>,
    cost_limit_operator: Option<String>,
    cost_total: Option<u32>,
    cost_total_operator: Option<String>,
    group: Option<String>,
    characters: Option<Vec<String>>,
    filtered_indices: Option<Vec<usize>>,
    is_select_action: bool,
    heart_colors: Vec<String>,
    name_fragments: Option<Vec<String>>,
    target_player_id: Option<String>,
    blind: bool,
    is_reveal: bool,
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
            cost_total: self.cost_total,
            cost_total_operator: self.cost_total_operator,
            group: self.group,
            characters: self.characters,
            filtered_indices: self.filtered_indices,
            is_select_action: self.is_select_action,
            heart_colors: self.heart_colors,
            name_fragments: self.name_fragments,
            target_player_id: self.target_player_id,
            blind: self.blind,
            is_reveal: self.is_reveal,
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
    pub fn cost_total(mut self, v: Option<u32>, op: Option<String>) -> Self {
        self.cost_total = v;
        self.cost_total_operator = op;
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
    pub fn target_player_id(mut self, v: Option<String>) -> Self {
        self.target_player_id = v;
        self
    }
    pub fn blind(mut self, v: bool) -> Self {
        self.blind = v;
        self
    }
    pub fn is_reveal(mut self, v: bool) -> Self {
        self.is_reveal = v;
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
            cost_total: None,
            cost_total_operator: None,
            group: None,
            characters: None,
            filtered_indices: None,
            is_select_action: false,
            heart_colors: vec![],
            name_fragments: None,
            target_player_id: None,
            blind: false,
            is_reveal: false,
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
            Choice::SelectAutoAbility {
                options,
                description,
                ..
            } => {
                if let Some(obj) = json.as_object_mut() {
                    if let Some(inner) = obj.remove("SelectAutoAbility") {
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
            Choice::SelectLiveSuccess {
                options,
                description,
                ..
            } => {
                if let Some(obj) = json.as_object_mut() {
                    if let Some(inner) = obj.remove("SelectLiveSuccess") {
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

// ====================================================================
// Execution trace nodes
// ====================================================================

/// Snapshot of key zone counts for debugging ability execution.
#[derive(Clone, Debug)]
pub struct ZoneSnapshot {
    pub hand_count: usize,
    pub stage_count: usize,
    pub waitroom_count: usize,
    pub energy_count: usize,
    pub active_energy_count: usize,
    pub deck_count: usize,
}

impl ZoneSnapshot {
    pub fn from_game_state(gs: &crate::game_state::GameState) -> Self {
        let p1 = &gs.player1;
        let p2 = &gs.player2;
        ZoneSnapshot {
            hand_count: p1.hand.len() + p2.hand.len(),
            stage_count: p1.stage.stage.iter().filter(|&&id| id != -1).count()
                + p2.stage.stage.iter().filter(|&&id| id != -1).count(),
            waitroom_count: p1.waitroom.len() + p2.waitroom.len(),
            energy_count: p1.energy_zone.cards.len() + p2.energy_zone.cards.len(),
            active_energy_count: p1.energy_zone.active_energy_count
                + p2.energy_zone.active_energy_count,
            deck_count: p1.main_deck.cards.len() + p2.main_deck.cards.len(),
        }
    }
}

/// A node in the ability execution trace tree.
#[derive(Clone, Debug)]
pub struct AbilityTraceNode {
    pub label: String,
    pub card: Option<String>,
    pub before: Option<ZoneSnapshot>,
    pub after: Option<ZoneSnapshot>,
    pub children: Vec<AbilityTraceNode>,
}

impl AbilityTraceNode {
    pub fn new(label: impl Into<String>) -> Self {
        AbilityTraceNode {
            label: label.into(),
            card: None,
            before: None,
            after: None,
            children: Vec::new(),
        }
    }

    pub fn with_card(mut self, card: Option<String>) -> Self {
        self.card = card;
        self
    }

    pub fn with_before(mut self, before: ZoneSnapshot) -> Self {
        self.before = Some(before);
        self
    }

    pub fn add_child(&mut self, node: AbilityTraceNode) {
        self.children.push(node);
    }
}

// ====================================================================
// EffectPipeline — the data that flows between sequential effects.
// ====================================================================

/// Carries state between sequential effect steps explicitly.
/// Instead of hiding `selected_cards`, `moved_cards` etc. on the
/// resolver or queue entry, the pipeline makes the handoff traceable.
#[derive(Clone, Debug)]
pub struct EffectPipeline {
    pub selected_card_ids: Vec<i16>,
    pub moved_cards: Vec<i16>,
    pub activating_card_id: Option<i16>,
    pub card_database: Arc<CardDatabase>,
    pub trace: AbilityTraceNode,
}

impl EffectPipeline {
    pub fn new(db: Arc<CardDatabase>) -> Self {
        EffectPipeline {
            selected_card_ids: Vec::new(),
            moved_cards: Vec::new(),
            activating_card_id: None,
            card_database: db,
            trace: AbilityTraceNode::new("root"),
        }
    }

    pub fn fmt_card(&self, cid: i16) -> String {
        self.card_database
            .get_card(cid)
            .map(|c| c.name.as_str())
            .unwrap_or("?")
            .to_string()
    }

    pub fn fmt_ids(&self, ids: &[i16]) -> String {
        if ids.is_empty() {
            "[]".into()
        } else {
            ids.iter()
                .map(|&id| self.fmt_card(id))
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}
