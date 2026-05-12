use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CardType {
    #[serde(rename = "メンバー")]
    // Rule 4.1: Member cards are placed on the stage and used for performance
    Member,
    #[serde(rename = "ライブ")]
    // Rule 4.2: Live cards are placed in Live Card Zone and used for live performance
    Live,
    #[serde(rename = "エネルギー")]
    // Rule 4.3: Energy cards are placed in Energy Zone and used to pay costs
    Energy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HeartColor {
    #[serde(rename = "heart00")]
    Heart00,  // Index 0 - wildcard, can be treated as any heart01-heart06
    #[serde(rename = "heart01")]
    Heart01,
    #[serde(rename = "heart02")]
    Heart02,
    #[serde(rename = "heart03")]
    Heart03,
    #[serde(rename = "heart04")]
    Heart04,
    #[serde(rename = "heart05")]
    Heart05,
    #[serde(rename = "heart06")]
    Heart06,
    #[serde(rename = "b_all")]
    BAll,  // Blade heart wildcard
    #[serde(rename = "draw")]
    Draw,  // Special heart type for drawing cards
    #[serde(rename = "score")]
    Score,  // Special heart type for score bonus
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BladeColor {
    #[serde(rename = "桃")]
    Peach,
    #[serde(rename = "赤")]
    Red,
    #[serde(rename = "黄")]
    Yellow,
    #[serde(rename = "緑")]
    Green,
    #[serde(rename = "青")]
    Blue,
    #[serde(rename = "紫")]
    Purple,
    #[serde(rename = "all")]
    All,  // All blade types
}

// Rule 11: Keywords
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Keyword {
    Turn1,          // Rule 11.1: First turn only
    Turn2,          // Rule 11.2: Second turn only
    Debut,          // Rule 11.3: First time this member is placed on stage
    LiveStart,      // Rule 11.4: When live card set phase begins
    LiveSuccess,    // Rule 11.5: When live is successful
    Center,         // Rule 11.6: Center position
    LeftSide,       // Rule 11.7: Left side position
    RightSide,      // Rule 11.8: Right side position
    PositionChange, // Rule 11.9: Member moves to different position
    FormationChange, // Rule 11.10: Multiple members move simultaneously
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartIcon {
    pub color: HeartColor,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct BladeHeart {
    pub hearts: HashMap<HeartColor, u32>,
}

impl<'de> Deserialize<'de> for BladeHeart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawBladeHeart {
            #[serde(flatten)]
            hearts: HashMap<String, u32>,
        }
        
        let raw = RawBladeHeart::deserialize(deserializer)?;
        let hearts = raw.hearts.into_iter().map(|(k, v)| {
            (parse_heart_color(&k), v)
        }).collect();
        
        Ok(BladeHeart { hearts })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BaseHeart {
    pub hearts: HashMap<HeartColor, u32>,
}

impl<'de> Deserialize<'de> for BaseHeart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawBaseHeart {
            #[serde(flatten)]
            hearts: HashMap<String, u32>,
        }
        
        let raw = RawBaseHeart::deserialize(deserializer)?;
        let hearts = raw.hearts.into_iter().map(|(k, v)| {
            (parse_heart_color(&k), v)
        }).collect();
        
        Ok(BaseHeart { hearts })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Card {
    pub card_no: String,
    pub img: Option<String>,
    pub name: String,
    #[serde(default)]
    pub product: String,
    #[serde(rename = "type")]
    pub card_type: CardType,
    #[serde(default)]
    pub series: String,
    #[serde(default = "default_group_from_series")]
    pub group: String,
    pub unit: Option<String>,
    pub cost: Option<u32>,
    pub base_heart: Option<BaseHeart>,
    pub blade_heart: Option<BladeHeart>,
    #[serde(default = "default_blade")]
    pub blade: u32,
    #[serde(default)]
    pub rare: String,
    #[serde(default)]
    pub ability: String,
    #[serde(default)]
    pub faq: Vec<FAQEntry>,
    #[serde(rename = "_img")]
    pub _img: Option<String>,
    // Live card fields
    pub score: Option<u32>,
    pub need_heart: Option<BaseHeart>,
    pub special_heart: Option<SpecialHeart>,
    // Parsed abilities from abilities.json
    #[serde(skip)]
    pub abilities: Vec<Ability>,
    #[serde(skip)]
    pub card_id: i16,  // Database ID for optimization
}

#[derive(Debug, Clone)]
pub struct CardDatabase {
    pub cards: HashMap<i16, Card>,
    pub card_no_to_id: HashMap<String, i16>,
    pub next_id: i16,
}

impl CardDatabase {
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
            card_no_to_id: HashMap::new(),
            next_id: 0,
        }
    }

    /// Create a copy of an existing card with a new unique ID.
    /// Used to give each card copy its own ID for per-copy modifier tracking.
    pub fn create_copy(&mut self, template_id: i16) -> i16 {
        let card = self.cards.get(&template_id)
            .expect("Template card not found")
            .clone();
        let copy_id = self.next_id;
        self.next_id += 1;
        self.cards.insert(copy_id, card);
        copy_id
    }

    pub fn load_or_create(cards: Vec<Card>) -> Self {
        let mut db = Self::new();
        
        // Try to load existing mapping
        if let Ok(mapping) = std::fs::read_to_string("card_id_mapping.json") {
            if let Ok(loaded_mapping) = serde_json::from_str::<HashMap<String, i16>>(&mapping) {
                db.card_no_to_id = loaded_mapping;
                db.next_id = db.card_no_to_id.values().max().copied().unwrap_or(0) + 1;
            }
        }
        
        // Add cards, assigning IDs if not already mapped
        for card in cards {
            if !db.card_no_to_id.contains_key(&card.card_no) {
                db.card_no_to_id.insert(card.card_no.clone(), db.next_id);
                db.next_id += 1;
            }
            let card_id = db.card_no_to_id[&card.card_no];
            db.cards.insert(card_id, card);
        }
        
        // Save mapping
        
        db

    }

    pub fn save_mapping(&self) {
        if let Ok(mapping) = serde_json::to_string_pretty(&self.card_no_to_id) {
            let _ = std::fs::write("card_id_mapping.json", mapping);
        }
    }
    
    pub fn get_card(&self, card_id: i16) -> Option<&Card> {
        self.cards.get(&card_id)
    }

    pub fn get_card_by_no(&self, card_no: &str) -> Option<&Card> {
        if let Some(&card_id) = self.card_no_to_id.get(card_no) {
            self.cards.get(&card_id)
        } else {
            None
        }
    }

    pub fn get_card_id(&self, card_no: &str) -> Option<i16> {
        // Try exact match first
        if let Some(&id) = self.card_no_to_id.get(card_no) {
            return Some(id);
        }
        // Normalize: uppercase + convert fullwidth characters to halfwidth
        let normalized = Self::normalize_card_no(card_no);
        for (key, &id) in &self.card_no_to_id {
            if Self::normalize_card_no(key) == normalized {
                return Some(id);
            }
        }
        None
    }
    
    /// Normalize card_no for lookup: uppercase, fullwidth → halfwidth
    fn normalize_card_no(card_no: &str) -> String {
        card_no
            .to_uppercase()
            .replace('＋', "+")
            .replace('－', "-")
            .replace('＊', "*")
            .replace('＃', "#")
    }

    /// Check if a card's name contains the given name fragment
    /// Used for cost payment and ability targeting (Q90, Q81, Q74)
    pub fn card_name_contains(&self, card_id: i16, name_fragment: &str) -> bool {
        if let Some(card) = self.cards.get(&card_id) {
            card.name.contains(name_fragment)
        } else {
            false
        }
    }

    /// Get all names from a multi-name card (e.g., "A&B&C" -> ["A", "B", "C"])
    /// Used for multi-name card handling (Q65, Q69, Q81)
    pub fn get_card_names(&self, card_id: i16) -> Vec<String> {
        if let Some(card) = self.cards.get(&card_id) {
            // Handle both regular '&' and full-width '＆' separators
            card.name.replace('＆', "&").split('&').map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        }
    }

    /// Check if card has any of the given names (for multi-name cards)
    pub fn card_has_any_name(&self, card_id: i16, names: &[&str]) -> bool {
        let card_names = self.get_card_names(card_id);
        names.iter().any(|&name| card_names.iter().any(|cn| cn.contains(name)))
    }
}

impl<'de> Deserialize<'de> for Card {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Debug, Clone, Deserialize)]
        struct CardHelper {
            pub card_no: String,
            pub img: Option<String>,
            pub name: String,
            #[serde(default)]
            pub product: String,
            #[serde(rename = "type")]
            pub card_type: CardType,
            #[serde(default)]
            pub series: String,
            pub unit: Option<String>,
            pub cost: Option<u32>,
            pub base_heart: Option<BaseHeart>,
            pub blade_heart: Option<BladeHeart>,
            #[serde(default = "default_blade")]
            pub blade: u32,
            #[serde(default)]
            pub rare: String,
            #[serde(default)]
            pub ability: String,
            #[serde(default)]
            pub faq: Vec<FAQEntry>,
            #[serde(rename = "_img")]
            pub _img: Option<String>,
            pub score: Option<u32>,
            pub need_heart: Option<BaseHeart>,
            pub special_heart: Option<SpecialHeart>,
        }

        let helper = CardHelper::deserialize(deserializer)?;
        let group = map_series_to_group(&helper.series);

        Ok(Card {
            card_no: helper.card_no,
            img: helper.img,
            name: helper.name,
            product: helper.product,
            card_type: helper.card_type,
            series: helper.series,
            group,
            unit: helper.unit,
            cost: helper.cost,
            base_heart: helper.base_heart,
            blade_heart: helper.blade_heart,
            blade: helper.blade,
            rare: helper.rare,
            ability: helper.ability,
            faq: helper.faq,
            _img: helper._img,
            score: helper.score,
            need_heart: helper.need_heart,
            special_heart: helper.special_heart,
            abilities: Vec::new(),
            card_id: 0,
        })
    }
}

fn map_series_to_group(series: &str) -> String {
    match series {
        "ラブライブ！" => "μ's".to_string(),
        "ラブライブ！サンシャイン!!" => "Aqours".to_string(),
        "ラブライブ！虹ヶ咲学園スクールアイドル同好会" => "虹ヶ咲".to_string(),
        "ラブライブ！スーパースター!!" => "Liella!".to_string(),
        "蓮ノ空女学院スクールアイドルクラブ" => "蓮ノ空".to_string(),
        _ => String::new(),
    }
}

fn default_blade() -> u32 {
    0
}

#[derive(Debug, Clone, Serialize)]
pub struct SpecialHeart {
    pub hearts: HashMap<HeartColor, u32>,
}

impl<'de> Deserialize<'de> for SpecialHeart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawSpecialHeart {
            #[serde(flatten)]
            hearts: HashMap<String, u32>,
        }
        
        let raw = RawSpecialHeart::deserialize(deserializer)?;
        let hearts = raw.hearts.into_iter().map(|(k, v)| {
            (parse_heart_color(&k), v)
        }).collect();
        
        Ok(SpecialHeart { hearts })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Ability {
    #[serde(default = "default_empty_string")]
    pub full_text: String,
    #[serde(default = "default_empty_string")]
    pub triggerless_text: String,
    pub triggers: Option<String>,
    pub use_limit: Option<u32>,
    #[serde(default)]
    pub is_null: bool,
    pub cost: Option<AbilityCost>,
    pub effect: Option<AbilityEffect>,
    pub keywords: Option<Vec<Keyword>>,
}

fn default_empty_string() -> String {
    String::new()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AbilityCost {
    #[serde(default = "default_empty_string")]
    pub text: String,
    #[serde(rename = "type")]
    pub cost_type: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub count: Option<u32>,
    pub card_type: Option<String>,
    pub target: Option<String>,
    pub optional: Option<bool>,
    pub energy: Option<u32>,
    pub state_change: Option<String>,
    pub position: Option<PositionInfo>,
    #[serde(default)]
    pub options: Option<Vec<AbilityCost>>,
    #[serde(default)]
    pub self_cost: Option<bool>,
    #[serde(default)]
    pub exclude_self: Option<bool>,
    #[serde(default)]
    pub same_unit_name: Option<bool>,
    #[serde(default)]
    pub costs: Option<Vec<AbilityCost>>,
    #[serde(default)]
    pub cost_limit: Option<u32>,
    #[serde(default)]
    pub cost_limit_operator: Option<String>,
    #[serde(default)]
    pub characters: Option<Vec<String>>,
    #[serde(default)]
    pub group_names: Option<Vec<String>>,
    #[serde(default)]
    pub placement_order: Option<String>,
    #[serde(default)]
    pub shuffle: Option<bool>,
}

/// Grouped sub-effect fields used by compound action handlers
/// (Sequential, ConditionalAlternative, ConditionalOnResult, ConditionalOnOptional, LookAndSelect).
/// Flattened into AbilityEffect via `#[serde(flatten)]` for JSON backward compat.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompoundBranch {
    #[serde(default)]
    pub look_action: Option<Box<AbilityEffect>>,
    #[serde(default)]
    pub select_action: Option<Box<AbilityEffect>>,
    #[serde(default)]
    pub actions: Option<Vec<AbilityEffect>>,
    #[serde(default)]
    pub primary_effect: Option<Box<AbilityEffect>>,
    #[serde(default)]
    pub alternative_condition: Option<Condition>,
    #[serde(default)]
    pub alternative_effect: Option<Box<AbilityEffect>>,
    #[serde(default)]
    pub result_condition: Option<Condition>,
    #[serde(default)]
    pub followup_action: Option<Box<AbilityEffect>>,
    #[serde(default)]
    pub optional_action: Option<Box<AbilityEffect>>,
    #[serde(default)]
    pub conditional_action: Option<Box<AbilityEffect>>,
    #[serde(default)]
    pub conditional_negation: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AbilityEffect {
    #[serde(default = "default_empty_string")]
    pub text: String,
    #[serde(default = "default_empty_string")]
    pub action: String,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub count: Option<u32>,
    pub target_count: Option<u32>,
    pub card_type: Option<String>,
    pub target: Option<String>,
    pub duration: Option<String>,
    pub resource: Option<String>,
    pub position: Option<PositionInfo>,
    pub state_change: Option<String>,
    pub optional: Option<bool>,
    pub max: Option<bool>,
    pub effect_constraint: Option<String>,
    pub resource_icon_count: Option<u32>,
    pub ability_gain: Option<String>,
    pub quoted_text: Option<QuotedText>,
    pub per_unit: Option<bool>,
    pub condition: Option<Condition>,
    /// Compound sub-effects (sequential, conditional, look_and_select branches).
    /// Flat fields in JSON are collected here via `#[serde(flatten)]`.
    #[serde(flatten)]
    pub compound: CompoundBranch,
    pub operation: Option<String>,
    pub value: Option<u32>,
    /// Heart colors specification.
    /// Semantics depend on action:
    /// - gain_resource: choice options (len>1) or fixed single color (len==1)
    /// - modify_required_hearts: which color's requirement to modify (uses first)
    /// - set_required_hearts: each color's requirement is set individually
    /// - select/reveal/look_and_select: filter — card must match ANY listed color
    /// - modify_score (per_unit): count cards matching ANY listed color
    /// - Empty: use default (heart00 for single-value ops, no filter for filter ops)
    #[serde(default)]
    pub heart_colors: Vec<String>,
    pub blade_type: Option<String>,
    #[serde(alias = "energy")]
    pub energy_count: Option<u32>,
    pub target_member: Option<String>,
    // Fields from parser improvements
    pub choice_options: Option<Vec<String>>,
    pub options: Option<Vec<AbilityEffect>>,
    pub per_unit_count: Option<u32>,
    pub per_unit_type: Option<String>,
    #[serde(alias = "max_repeats")]
    pub repeat_limit: Option<u32>,
    pub is_further: Option<bool>,
    pub restriction_type: Option<String>,
    pub restricted_destination: Option<String>,
    pub dynamic_count: Option<DynamicCount>,
    pub placement_order: Option<String>,
    pub cost_limit: Option<u32>,
    #[serde(default)]
    pub cost_limit_operator: Option<String>,
    #[serde(default)]
    pub any_number: Option<bool>,
    #[serde(default)]
    pub discard_remaining: Option<bool>,
    #[serde(default)]
    pub reveal: Option<bool>,
    pub distinct: Option<String>,
    // Card name matching constraints
    #[serde(default)]
    pub name_constraint: Option<String>,
    #[serde(default)]
    pub name_constraint_source: Option<String>,
    pub activation_condition_parsed: Option<Condition>,
    pub ability_text: Option<String>,
    pub use_limit: Option<u32>,
    pub triggers: Option<String>,
    #[serde(default)]
    pub self_cost: Option<bool>,
    #[serde(default)]
    pub exclude_self: Option<bool>,
    #[serde(default)]
    pub exclude_selected: Option<bool>,
    // Effect type for replacement/continuous effects
    #[serde(default)]
    pub effect_type: Option<String>,
    // Heart color specification
    #[serde(default)]
    pub choice: Option<bool>,
    // ALL blade timing
    #[serde(default)]
    pub timing: Option<String>,
    #[serde(default)]
    pub treat_as: Option<String>,
    // Replacement effect metadata
    #[serde(default)]
    pub replaces_event: Option<String>,
    #[serde(default)]
    pub choice_based: Option<bool>,
    // Card identity
    #[serde(default)]
    pub identities: Option<Vec<String>>,
    // Opponent action handling
    #[serde(default)]
    pub action_by: Option<String>,
    #[serde(default)]
    pub opponent_action: Option<Box<AbilityEffect>>,
    // Missing fields from parser
    #[serde(default)]
    pub lose_blade_hearts: Option<bool>,
    #[serde(default)]
    pub conditional: Option<bool>,
    #[serde(default)]
    pub choice_type: Option<String>,
    #[serde(default)]
    #[serde(alias = "heart_color")]
    pub heart_type: Option<String>,
    /// parenthetical annotations (e.g. rule clarifications in parentheses)
    #[serde(default)]
    pub parenthetical: Option<Vec<String>>,
    /// characters filter for card selection
    #[serde(default)]
    pub characters: Option<Vec<String>>,
    /// source_card reference (e.g. "cost_card" for activate_ability)
    #[serde(default)]
    pub source_card: Option<String>,
    // Parser-only fields that were missing struct fields
    #[serde(default)]
    pub or_card_types: Option<Vec<String>>,
    #[serde(default)]
    pub activation_position: Option<String>,
    /// Source position: member AT this position gets moved (e.g. "センターにいる")
    #[serde(default)]
    pub source_position: Option<String>,
    /// Excluded destination position (e.g. "センターエリア以外" → don't allow center)
    #[serde(default)]
    pub exclude_position: Option<String>,
    #[serde(default)]
    pub all_regions: Option<bool>,
    #[serde(default)]
    pub character_effects: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub group_names: Option<Vec<String>>,
    #[serde(default)]
    pub heart_selection: Option<bool>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub multiple_targets: Option<bool>,
    #[serde(default)]
    pub question: Option<String>,
    #[serde(default)]
    pub answers: Option<Vec<String>>,
    #[serde(default)]
    pub choice_maker: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub target_trigger: Option<String>,
    #[serde(default)]
    pub timing_condition: Option<String>,
    #[serde(default)]
    pub self_target: Option<bool>,
    #[serde(default)]
    pub trigger_type: Option<String>,
    /// "+" or "-" sign for resource operations (gain_resource with sign: "negative" = lose)
    #[serde(default)]
    pub sign: Option<String>,
    /// Phase restriction for restriction actions (e.g., "active_phase")
    #[serde(default)]
    pub phase: Option<String>,
    /// "すべての" — apply to ALL cards in zone, not just matching count
    #[serde(default)]
    pub all: Option<bool>,
    /// "元々持つ" — refers to original/natural value, not current modified value
    #[serde(default)]
    pub original_value: Option<bool>,
    /// Dynamic group reference (same_group_name, different_group_names)
    #[serde(default)]
    pub group_reference: Option<String>,
    /// Conditional trigger for each_time / たび patterns (e.g. OR conditions)
    #[serde(default)]
    pub trigger_condition: Option<Box<Condition>>,
}

impl AbilityEffect {
    /// Returns the target player string, defaulting to "self".
    pub fn target_name(&self) -> &str { self.target.as_deref().unwrap_or("self") }

    /// Returns the source zone string with a static default.
    pub fn source_or(&self, default: &'static str) -> &str { self.source.as_deref().unwrap_or(default) }

    /// Returns the count with a caller-provided default.
    pub fn count_or(&self, n: u32) -> u32 { self.count.unwrap_or(n) }

    /// Returns the first group name, if any.
    pub fn group_name(&self) -> Option<&str> {
        self.group_names.as_ref().and_then(|gn| gn.first().map(|s| s.as_str()))
    }

    /// Returns the first heart color as a string reference, or a static default.
    /// For single-color operations like modify_required_hearts.
    pub fn heart_color_or(&self, default: &'static str) -> &str {
        self.heart_colors.first().map(|s| s.as_str()).unwrap_or(default)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PositionInfo {
    String(String),
    Struct {
        position: Option<String>,
        target: Option<String>,
    },
}

impl PositionInfo {
    pub fn get_position(&self) -> Option<&str> {
        match self {
            PositionInfo::String(s) => Some(s.as_str()),
            PositionInfo::Struct { position, .. } => position.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicCount {
    #[serde(rename = "type")]
    pub count_type: String,
    pub reference: Option<String>,
    pub mode: Option<String>,
    pub base_reference: Option<String>,
    pub calculation: Option<String>,
    pub calculation_value: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotedText {
    pub text: String,
    pub quoted_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Condition {
    #[serde(default = "default_empty_string")]
    pub text: String,
    #[serde(rename = "type")]
    pub condition_type: Option<String>,
    pub location: Option<String>,
    pub locations: Option<Vec<String>>,
    pub count: Option<u32>,
    pub operator: Option<String>,
    pub card_type: Option<String>,
    pub target: Option<String>,
    pub group_names: Option<Vec<String>>,
    pub characters: Option<Vec<String>>,
    pub state: Option<String>,
    pub position: Option<PositionInfo>,
    pub temporal_scope: Option<String>,
    pub distinct: Option<bool>,
    pub exclude_self: Option<bool>,
    pub any_of: Option<Vec<String>>,
    pub cost_limit: Option<u32>,
    pub negation: Option<bool>,
    pub baton_touch_trigger: Option<bool>,
    pub baton_touch_source: Option<String>,
    pub movement_state: Option<String>,
    pub energy_state: Option<String>,
    pub comparison_target: Option<String>,
    pub movement: Option<String>,
    pub temporal: Option<String>,
    pub phase: Option<String>,
    pub comparison_type: Option<String>,
    pub appearance: Option<bool>,
    pub conditions: Option<Vec<Condition>>,
    pub options: Option<Vec<AbilityEffect>>,
    #[serde(default)]
    pub condition: Option<Box<Condition>>,
    pub card_property: Option<String>,
    // New fields from parser improvements
    pub all_areas: Option<bool>,
    pub no_excess_heart: Option<bool>,
    pub resource_type: Option<String>,
    pub all: Option<bool>,
    pub unit: Option<String>,
    pub values: Option<Vec<u32>>,
    // Complex condition fields
    #[serde(default)]
    pub cause: Option<Box<Condition>>,
    #[serde(default)]
    pub effect: Option<Box<AbilityEffect>>,
    // Parser-only fields that were missing struct fields
    #[serde(default)]
    pub from_state: Option<String>,
    #[serde(default)]
    pub heart_type: Option<String>,
    #[serde(default)]
    pub to_state: Option<String>,
    #[serde(default)]
    pub aggregate: Option<String>,
    /// Heart colors required collectively from stage members (e.g. all 6 colors)
    #[serde(default)]
    pub heart_colors: Option<Vec<String>>,
    /// ability_negation flag for "能力を持たない" (does not have ability) conditions
    #[serde(default)]
    pub ability_negation: Option<bool>,
    /// "元々持つ" — compare against original/natural value, not current modified value
    #[serde(default)]
    pub original_value: Option<bool>,
    /// scope: "both" for conditions that check both players (e.g. energy total of both)
    #[serde(default)]
    pub scope: Option<String>,
    /// "のみ" — ALL members on stage must match the group (not just any)
    #[serde(default)]
    pub all_members: Option<bool>,
    /// Explicit source reference for condition evaluation.
    /// "preceding_moved" — check against the most recently moved cards from a prior action.
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FAQEntry {
    pub title: String,
    pub question: String,
    pub answer: String,
    pub relation: Vec<CardRelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardRelation {
    pub card_no: String,
    pub name: String,
}

impl Card {
    pub fn is_member(&self) -> bool {
        matches!(self.card_type, CardType::Member)
    }

    pub fn is_live(&self) -> bool {
        matches!(self.card_type, CardType::Live)
    }

    pub fn is_energy(&self) -> bool {
        matches!(self.card_type, CardType::Energy)
    }

    pub fn total_hearts(&self) -> u32 {
        if let Some(ref base_heart) = self.base_heart {
            base_heart.hearts.values().sum()
        } else if let Some(ref need_heart) = self.need_heart {
            need_heart.hearts.values().sum()
        } else {
            0
        }
    }

    pub fn has_blade_heart(&self) -> bool {
        self.blade_heart.is_some() || self.blade > 0
    }
    
    pub fn satisfies_heart_requirement(&self, provided_hearts: &BaseHeart) -> bool {
        // Rule 8.2.8: Check if provided hearts satisfy card's need_heart requirement
        // Heart00 (index 0) is wildcard and can substitute for any heart01-heart06
        if let Some(ref need_heart) = self.need_heart {
            let wildcard_count = *provided_hearts.hearts.get(&HeartColor::Heart00).unwrap_or(&0);
            
            // Count total hearts available for heart0 requirements (any color can be used)
            let total_hearts_for_heart0: u32 = provided_hearts.hearts.values().sum();
            
            for (color, needed_amount) in &need_heart.hearts {
                if *color == HeartColor::Heart00 {
                    // heart0 requirement: any heart color can fulfill this
                    if total_hearts_for_heart0 < *needed_amount {
                        return false;
                    }
                } else {
                    // Specific heart color requirement
                    let wildcard_count = wildcard_count;
                    if let Some(&provided_amount) = provided_hearts.hearts.get(color) {
                        if provided_amount + wildcard_count >= *needed_amount {
                            // Subtract the specific hearts first, then use wildcard if needed
                            let remaining_needed = if provided_amount >= *needed_amount {
                                0
                            } else {
                                *needed_amount - provided_amount
                            };
                            if remaining_needed > wildcard_count {
                                return false;
                            }
                        } else {
                            // Not enough even with wildcard
                            if *needed_amount > wildcard_count {
                                return false;
                            }
                        }
                    } else {
                        // No specific hearts available, use wildcard
                        if *needed_amount > wildcard_count {
                            return false;
                        }
                    }
                }
            }
            true
        } else {
            // No heart requirement
            true
        }
    }
}

pub fn parse_heart_color(s: &str) -> HeartColor {
    match s {
        "heart00" => HeartColor::Heart00,
        "heart01" => HeartColor::Heart01,
        "heart02" => HeartColor::Heart02,
        "heart03" => HeartColor::Heart03,
        "heart04" => HeartColor::Heart04,
        "heart05" => HeartColor::Heart05,
        "heart06" => HeartColor::Heart06,
        "b_all" => HeartColor::BAll,
        "draw" => HeartColor::Draw,
        "score" => HeartColor::Score,
        _ => HeartColor::Heart00,
    }
}

impl Card {
    pub fn get_score(&self) -> u32 {
        self.score.unwrap_or(0)
    }

    // ============== RESOURCE MODIFICATION METHODS ==============

    /// Add blades to card
    pub fn add_blades(&mut self, amount: u32) {
        self.blade += amount;
    }

    /// Remove blades from card (minimum 0)
    pub fn remove_blades(&mut self, amount: u32) {
        self.blade = self.blade.saturating_sub(amount);
    }

    /// Set blades to specific value
    pub fn set_blades(&mut self, amount: u32) {
        self.blade = amount;
    }

    /// Add hearts of specific color
    pub fn add_heart(&mut self, heart_color: &str, amount: u32) {
        if let Some(ref mut base_heart) = self.base_heart {
            let color = parse_heart_color(heart_color);
            *base_heart.hearts.entry(color).or_insert(0) += amount;
        }
    }

    /// Remove hearts of specific color (minimum 0)
    pub fn remove_heart(&mut self, heart_color: &str, amount: u32) {
        if let Some(ref mut base_heart) = self.base_heart {
            let color = parse_heart_color(heart_color);
            let current = base_heart.hearts.get(&color).copied().unwrap_or(0);
            if current <= amount {
                base_heart.hearts.remove(&color);
            } else {
                base_heart.hearts.insert(color, current - amount);
            }
        }
    }

    /// Set hearts to specific value
    pub fn set_heart(&mut self, heart_color: &str, amount: u32) {
        if let Some(ref mut base_heart) = self.base_heart {
            let color = parse_heart_color(heart_color);
            base_heart.hearts.insert(color, amount);
        }
    }

    /// Add score to card
    pub fn add_score(&mut self, amount: u32) {
        if self.score.is_none() {
            self.score = Some(0);
        }
        if let Some(ref mut score) = self.score {
            *score += amount;
        }
    }

    /// Remove score from card (minimum 0)
    pub fn remove_score(&mut self, amount: u32) {
        if let Some(ref mut score) = self.score {
            *score = score.saturating_sub(amount);
        }
    }

    /// Set score to specific value
    pub fn set_score(&mut self, amount: u32) {
        self.score = Some(amount);
    }

    /// Modify cost by amount (minimum 0)
    pub fn modify_cost(&mut self, amount: i32) {
        if self.cost.is_none() {
            self.cost = Some(0);
        }
        if let Some(ref mut cost) = self.cost {
            if amount >= 0 {
                *cost += amount as u32;
            } else {
                *cost = cost.saturating_sub((-amount) as u32);
            }
        }
    }

    /// Set cost to specific value
    pub fn set_cost(&mut self, amount: u32) {
        self.cost = Some(amount);
    }

    pub fn get_hand_cost_reduction(&self, hand_size: usize) -> u32 {
        for ability in &self.abilities {
            if let Some(ref effect) = ability.effect {
                if effect.action == "modify_cost"
                    && effect.operation.as_deref() == Some("subtract")
                    && effect.location.as_deref() == Some("hand")
                {
                    let per_unit = effect.per_unit_count.unwrap_or(1) as usize;
                    return (hand_size.saturating_sub(1) * per_unit) as u32;
                }
            }
        }
        0
    }
}
