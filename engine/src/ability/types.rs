use std::fmt;

use serde_json::Value;

use crate::card::AbilityEffect;

/// Contextual event data passed into `trigger_auto_abilities_for_player`.
/// Replaces the fragmented per-scan tracking flags with a single explicit
/// value, making it clear what triggered the scan.
#[derive(Clone, Debug, Default)]
pub struct TriggerEvent {
    /// Cards that moved in this batch (replaces recently_moved_cards).
    pub moved_cards: Vec<i16>,
    /// Zone the cards moved from (if tracked).
    pub moved_from_zone: Option<String>,
    /// Whether a position change occurred.
    pub position_change_occurred: bool,
    /// Cards that appeared on stage recently, with their source zone.
    pub appeared_cards: Vec<(i16, String)>,
    /// Whether energy was placed by a card effect.
    pub energy_placed_by_effect: bool,
    /// Which player's effect placed the energy.
    pub energy_placed_by_player: Option<String>,
}

/// Discriminator for routing choice results to the correct handler.
/// Statically known routes are enum variants; dynamic routes (e.g. position_change
/// with card_no) use `Raw`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChoiceRoute {
    Choice,
    ChoiceString,
    ChoiceCost,
    OptionalCost,
    ChangeState,
    Raw(String),
}

/// Actions queued for sequential execution after a choice resolves.
/// Plain AbilityEffect list — no Command enum wrapper needed.

impl fmt::Display for ChoiceRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChoiceRoute::Choice => write!(f, "choice"),
            ChoiceRoute::ChoiceString => write!(f, "choice_string"),
            ChoiceRoute::ChoiceCost => write!(f, "choice_cost"),
            ChoiceRoute::OptionalCost => write!(f, "optional_cost"),
            ChoiceRoute::ChangeState => write!(f, "change_state"),
            ChoiceRoute::Raw(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Choice {
    SelectCard {
        zone: String,
        card_type: Option<String>,
        count: usize,
        description: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description_en: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description_ja: Option<String>,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        require_all_heart_colors: Option<bool>,
        #[serde(default)]
        name_fragments: Option<Vec<String>>,
        #[serde(default)]
        target_player_id: Option<String>,
        #[serde(default)]
        blind: bool,
        #[serde(default)]
        is_reveal: bool,
        #[serde(default)]
        destination: Option<String>,
        #[serde(default)]
        discard_remaining: Option<bool>,
    },
    SelectTarget {
        target: String,
        description: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description_en: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description_ja: Option<String>,
        allow_skip: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        options: Option<Vec<String>>,
    },
    SelectPosition {
        position: String,
        description: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description_en: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description_ja: Option<String>,
        allow_skip: bool,
    },
    SelectHeartColor {
        count: usize,
        options: Vec<String>,
        description: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description_en: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description_ja: Option<String>,
    },
    SelectHeartType {
        count: usize,
        options: Vec<String>,
        description: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description_en: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description_ja: Option<String>,
    },
    /// Player chooses which of their standby auto abilities resolves first (Rule 9.5.3).
    SelectAutoAbility {
        player_id: String,
        options: Vec<AutoAbilityOption>,
        description: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description_en: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description_ja: Option<String>,
    },
    /// Player chooses which live card goes to success zone (Rule 8.4.7).
    SelectLiveSuccess {
        player_id: String,
        count: usize,
        options: Vec<LiveSuccessOption>,
        description: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description_en: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description_ja: Option<String>,
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
        source_zone: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum LookAndSelectStep {
    LookAt {
        count: usize,
        source: String,
    },
    Select {
        count: usize,
        max_per_group: Option<u32>,
    },
    Finalize {
        destination: String,
        source_zone: String,
    },
}

pub struct ChoiceBuilder {
    zone: String,
    card_type: Option<String>,
    count: usize,
    description: String,
    description_en: Option<String>,
    description_ja: Option<String>,
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
    require_all_heart_colors: Option<bool>,
    name_fragments: Option<Vec<String>>,
    target_player_id: Option<String>,
    blind: bool,
    is_reveal: bool,
    destination: Option<String>,
    discard_remaining: Option<bool>,
}

impl ChoiceBuilder {
    pub fn build(self) -> Choice {
        Choice::SelectCard {
            zone: self.zone,
            card_type: self.card_type,
            count: self.count,
            description: self.description,
            description_en: self.description_en,
            description_ja: self.description_ja,
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
            require_all_heart_colors: self.require_all_heart_colors,
            name_fragments: self.name_fragments,
            target_player_id: self.target_player_id,
            blind: self.blind,
            is_reveal: self.is_reveal,
            destination: self.destination,
            discard_remaining: self.discard_remaining,
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
    pub fn require_all_heart_colors(mut self, v: Option<bool>) -> Self {
        self.require_all_heart_colors = v;
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
    pub fn destination(mut self, v: Option<String>) -> Self {
        self.destination = v;
        self
    }
    pub fn discard_remaining(mut self, v: Option<bool>) -> Self {
        self.discard_remaining = v;
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
    pub fn description_en(mut self, v: Option<String>) -> Self {
        self.description_en = v;
        self
    }
    pub fn description_ja(mut self, v: Option<String>) -> Self {
        self.description_ja = v;
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
            description_en: None,
            description_ja: None,
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
            description_en: None,
            description_ja: None,
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
            description_en: None,
            description_ja: None,
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
            require_all_heart_colors: None,
            name_fragments: None,
            target_player_id: None,
            blind: false,
            is_reveal: false,
            destination: None,
            discard_remaining: None,
        }
    }

    /// Build a CardFilter from the basic filter fields this choice carries.
    /// Used by `handle_select_card` to validate the user's picks against
    /// the choice's advertised constraints. Falls back to a default filter
    /// for non-`SelectCard` variants.
    pub fn as_filter<'a>(&'a self) -> crate::ability::util::CardFilter<'a> {
        match self {
            Choice::SelectCard {
                card_type,
                cost_limit,
                cost_limit_operator,
                group,
                characters,
                ..
            } => crate::ability::util::CardFilter {
                card_type: card_type.as_deref(),
                group: group.as_deref(),
                groups: None,
                cost_limit: *cost_limit,
                cost_operator: cost_limit_operator.as_deref(),
                cost_limit_min: None,
                cost_total: None,
                cost_total_operator: None,
                need_heart_total: None,
                need_heart_operator: None,
                need_heart_color: None,
                characters: characters.as_ref(),
                exclude_characters: None,
                exclude_names: None,
                heart_colors: &[],
                require_all_heart_colors: false,
                name_fragments: None,
                distinct: None,
                exclude_self: None,
                original_blade_limit: None,
                original_blade_operator: None,
                exclude_cards: None,
                ability_filter: None,
                ability_filter_triggers: None,
                or_ability_filters: None,
                card_property: None,
                negation: false,
                exclude_group_names: None,
            },
            _ => crate::ability::util::CardFilter::default(),
        }
    }

    /// Replace the description field (used by sequential_cost to set combined cost text).
    pub fn set_description(&mut self, desc: String) {
        match self {
            Choice::SelectCard { description, .. } => *description = desc,
            Choice::SelectTarget { description, .. } => *description = desc,
            Choice::SelectPosition { description, .. } => *description = desc,
            Choice::SelectHeartColor { description, .. } => *description = desc,
            Choice::SelectHeartType { description, .. } => *description = desc,
            Choice::SelectAutoAbility { description, .. } => *description = desc,
            Choice::SelectLiveSuccess { description, .. } => *description = desc,
        }
    }

    /// Replace both bilingual prompt fields.
    pub fn set_bilingual_descriptions(&mut self, en: Option<String>, ja: Option<String>) {
        match self {
            Choice::SelectCard {
                ref mut description_en,
                ref mut description_ja,
                ..
            }
            | Choice::SelectTarget {
                ref mut description_en,
                ref mut description_ja,
                ..
            }
            | Choice::SelectPosition {
                ref mut description_en,
                ref mut description_ja,
                ..
            }
            | Choice::SelectHeartColor {
                ref mut description_en,
                ref mut description_ja,
                ..
            }
            | Choice::SelectHeartType {
                ref mut description_en,
                ref mut description_ja,
                ..
            }
            | Choice::SelectAutoAbility {
                ref mut description_en,
                ref mut description_ja,
                ..
            }
            | Choice::SelectLiveSuccess {
                ref mut description_en,
                ref mut description_ja,
                ..
            } => {
                *description_en = en;
                *description_ja = ja;
            }
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
                ..
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
                ..
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
        // Inject bilingual prompts from Choice-level description_en/description_ja
        if let Some(obj) = json.as_object_mut() {
            match self {
                Choice::SelectCard {
                    ref description_en,
                    ref description_ja,
                    ..
                }
                | Choice::SelectTarget {
                    ref description_en,
                    ref description_ja,
                    ..
                }
                | Choice::SelectPosition {
                    ref description_en,
                    ref description_ja,
                    ..
                }
                | Choice::SelectHeartColor {
                    ref description_en,
                    ref description_ja,
                    ..
                }
                | Choice::SelectHeartType {
                    ref description_en,
                    ref description_ja,
                    ..
                }
                | Choice::SelectAutoAbility {
                    ref description_en,
                    ref description_ja,
                    ..
                }
                | Choice::SelectLiveSuccess {
                    ref description_en,
                    ref description_ja,
                    ..
                } => {
                    if let Some(en) = description_en {
                        obj.insert("prompt_en".into(), Value::String(en.clone()));
                    }
                    if let Some(ja) = description_ja {
                        obj.insert("prompt_ja".into(), Value::String(ja.clone()));
                    }
                }
            }
        }
        Some(json)
    }
}

// ====================================================================
// EffectSpawnContext — carry spawn-time effect parameters explicitly
// ====================================================================

#[derive(Clone, Debug, Default)]
pub struct EffectSpawnContext {
    pub target: Option<String>,
    pub destination: Option<String>,
    pub source: Option<String>,
    pub position: Option<usize>,
}

// ====================================================================
// StepOutput — the output of one step in a sequential compound effect,
// stored in AbilityResolver::step_results under the step's `id`.
// ====================================================================

/// The set of values a step can produce for downstream `ref` resolution.
/// Empty variants represent "this step produced no value" (e.g. a no-op
/// `draw` with optional:false) and are simply not stored.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StepOutput {
    /// Card ids the step "produced" — selected by `select_cards`, revealed by
    /// `reveal`/`look_at`, moved by `move_cards` (for `self` targets), or
    /// otherwise identified as the result of the step. Most steps populate
    /// this; downstream steps that need a card reference read it.
    pub cards: Vec<i16>,
    /// Numeric value the step produced — for steps like "draw N cards" the
    /// count drawn, for `gain_resource` the count gained, for `modify_score`
    /// the score delta, etc. Downstream `ValueRef::StepValue` reads this.
    pub value: Option<i32>,
    /// True iff the step was a yes/no choice and the player chose "yes" /
    /// "perform". Used for `ValueRef::StepAccepted` patterns.
    pub accepted: Option<bool>,
}

impl StepOutput {
    pub fn from_cards(cards: Vec<i16>) -> Self {
        StepOutput {
            cards,
            value: None,
            accepted: None,
        }
    }
    pub fn from_value(value: i32) -> Self {
        StepOutput {
            cards: Vec::new(),
            value: Some(value),
            accepted: None,
        }
    }
    pub fn from_accepted(accepted: bool) -> Self {
        StepOutput {
            cards: Vec::new(),
            value: None,
            accepted: Some(accepted),
        }
    }
    pub fn merge(&mut self, other: &StepOutput) {
        self.cards.extend_from_slice(&other.cards);
        if other.value.is_some() {
            self.value = other.value;
        }
        if other.accepted.is_some() {
            self.accepted = other.accepted;
        }
    }
}

// ====================================================================
// ValueRef / TargetRef — refer to either a literal or a step output.
// ====================================================================

/// Discriminated union for a field that is either a literal value in JSON
/// or a reference to a step's output. Engine code resolves this against
/// the resolver's `step_results` map before using the underlying value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueRef {
    /// Plain literal value (parsed from JSON number, or from a step output
    /// during a transition where the type hasn't been upgraded yet).
    Literal(i32),
    /// Reference to step_results[step_id].value.
    StepValue(String),
    /// Reference to step_results[step_id].accepted (used for yes/no
    /// "perform the effect?" choices — true means 1, false means 0).
    StepAccepted(String),
    /// Reference to step_results[step_id].value with an offset added.
    /// Used for patterns like "selected card's score - 1".
    StepValueOffset { step: String, offset: i32 },
}

impl ValueRef {
    /// Resolve to a concrete i32 against the step_results map.
    /// If `step_id` is not present, returns `fallback`.
    pub fn resolve(
        &self,
        step_results: &std::collections::HashMap<String, StepOutput>,
        fallback: i32,
    ) -> i32 {
        match self {
            ValueRef::Literal(v) => *v,
            ValueRef::StepValue(id) => step_results
                .get(id)
                .and_then(|o| o.value)
                .unwrap_or(fallback),
            ValueRef::StepAccepted(id) => step_results
                .get(id)
                .and_then(|o| o.accepted)
                .map(|b| if b { 1 } else { 0 })
                .unwrap_or(fallback),
            ValueRef::StepValueOffset { step, offset } => step_results
                .get(step)
                .and_then(|o| o.value)
                .map(|v| v + offset)
                .unwrap_or(fallback),
        }
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
            active_energy_count: p1.energy_zone.active_count() + p2.energy_zone.active_count(),
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

/// Carries execution trace state explicitly.
#[derive(Clone, Debug)]
pub struct EffectPipeline {
    pub trace: AbilityTraceNode,
}

impl Default for EffectPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectPipeline {
    pub fn new() -> Self {
        EffectPipeline {
            trace: AbilityTraceNode::new("root"),
        }
    }
}

// ====================================================================
// StepState — cross-step data flow machinery, extracted from
// AbilityResolver so the resolver struct stays focused on dispatch.
// ====================================================================

use std::collections::HashMap;

/// Owns the state required for one effect's steps to communicate with later
/// steps. Created fresh for every `AbilityResolver` and reset at the start
/// of every sequential (so data never leaks between abilities).
#[derive(Clone, Debug, Default)]
pub struct StepState {
    /// Per-step output, keyed by the step's `id` field. Populated by
    /// `record_step_output` and read by `step_output` / `resolve_ref_*`.
    pub step_results: HashMap<String, StepOutput>,
    /// Number of cards drawn by the most recent draw step. Read by the
    /// sequential handler to populate `StepOutput::value` for the step.
    pub last_draw_count: u32,
    /// Total count of cards looked at by the most recent look step. Read
    /// by the sequential handler for `StepOutput::value` on look steps.
    pub looked_at_total_count: usize,
}

impl StepState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a step's output under its id. No-op if the effect has no id.
    /// If the id already has an output, the new output is merged in.
    pub fn record(&mut self, effect: &AbilityEffect, output: StepOutput) {
        if let Some(ref id) = effect.id {
            self.step_results
                .entry(id.clone())
                .or_insert_with(StepOutput::default)
                .merge(&output);
        }
    }

    /// Look up a step's stored output by id. Returns an empty output if
    /// not present (callers can chain `.cards` / `.value` without unwrap).
    pub fn get(&self, id: &str) -> StepOutput {
        self.step_results.get(id).cloned().unwrap_or_default()
    }

    /// Drop all stored step outputs. Called by the sequential handler at
    /// the start of every sequential so steps from one sequential never
    /// leak to another.
    pub fn clear(&mut self) {
        self.step_results.clear();
        self.last_draw_count = 0;
        self.looked_at_total_count = 0;
    }

    /// Resolve a `ref` field on an effect to the list of card ids the
    /// referenced step produced. Returns an empty Vec when the reference
    /// is not present in step_results (or when `ref` is None).
    pub fn resolve_ref_cards(&self, effect: &AbilityEffect) -> Vec<i16> {
        match &effect.r#ref {
            Some(id) => self.get(id).cards,
            None => Vec::new(),
        }
    }

    /// Resolve a `ref_value` field on an effect to the integer value the
    /// referenced step produced, plus an optional offset. Returns `fallback`
    /// when the reference is not present (or `ref_value` is None).
    pub fn resolve_ref_value(&self, effect: &AbilityEffect, fallback: i32) -> i32 {
        match &effect.ref_value {
            Some(id) => self
                .get(id)
                .value
                .map(|v| v + effect.ref_offset.unwrap_or(0))
                .unwrap_or(fallback),
            None => fallback,
        }
    }
}
