use crate::ability::enums::{ActionType, Zone};
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
    #[serde(rename = "heart00", alias = "heart0")]
    Heart00, // Index 0 - wildcard, can be treated as any heart01-heart06
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
    BAll, // Blade heart wildcard
    #[serde(rename = "draw")]
    Draw, // Special heart type for drawing cards
    #[serde(rename = "score")]
    Score, // Special heart type for score bonus
    #[serde(rename = "all")]
    All, // All-heart (icon_all.png) — can be treated as any one color during performance check
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
    All, // All blade types
}

// Rule 11: Keywords
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Keyword {
    Turn1,           // Rule 11.1: First turn only
    Turn2,           // Rule 11.2: Second turn only
    Debut,           // Rule 11.3: First time this member is placed on stage
    LiveStart,       // Rule 11.4: When live card set phase begins
    LiveSuccess,     // Rule 11.5: When live is successful
    Center,          // Rule 11.6: Center position
    LeftSide,        // Rule 11.7: Left side position
    RightSide,       // Rule 11.8: Right side position
    PositionChange,  // Rule 11.9: Member moves to different position
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
        let hearts = raw
            .hearts
            .into_iter()
            .map(|(k, v)| (parse_heart_color(&k), v))
            .collect();

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
        let hearts = raw
            .hearts
            .into_iter()
            .map(|(k, v)| (parse_heart_color(&k), v))
            .collect();

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
}

#[derive(Debug, Clone)]
pub struct CardDatabase {
    pub cards: HashMap<i16, Card>,
    pub card_no_to_id: HashMap<String, i16>,
    pub next_id: i16,
}

impl Default for CardDatabase {
    fn default() -> Self {
        Self::new()
    }
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
        let card = self
            .cards
            .get(&template_id)
            .expect("Template card not found")
            .clone();
        let copy_id = self.next_id;
        self.next_id += 1;
        self.cards.insert(copy_id, card);
        copy_id
    }

    pub fn load_or_create(mut cards: Vec<Card>) -> Self {
        let mut db = Self::new();

        // Sort by card_no for deterministic ID assignment across runs
        cards.sort_by(|a, b| a.card_no.cmp(&b.card_no));

        for card in cards {
            if !db.card_no_to_id.contains_key(&card.card_no) {
                db.card_no_to_id.insert(card.card_no.clone(), db.next_id);
                db.next_id += 1;
            }
            let card_id = db.card_no_to_id[&card.card_no];
            db.cards.insert(card_id, card);
        }

        db
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
            .replace('！', "!")
            .replace('－', "-")
            .replace('＊', "*")
            .replace('＃', "#")
    }

    /// Strip all whitespace from a card name so that inconsistent spacing
    /// (e.g. "南 ことり" vs "南ことり") does not break ability conditions.
    pub fn normalize_name(name: &str) -> String {
        name.chars().filter(|c| !c.is_whitespace()).collect()
    }

    /// Check if a card's name contains the given name fragment
    /// Used for cost payment and ability targeting (Q90, Q81, Q74)
    pub fn card_name_contains(&self, card_id: i16, name_fragment: &str) -> bool {
        if let Some(card) = self.cards.get(&card_id) {
            Self::normalize_name(&card.name).contains(&Self::normalize_name(name_fragment))
        } else {
            false
        }
    }

    /// Get all names from a multi-name card (e.g., "A&B&C" -> ["A", "B", "C"])
    /// Used for multi-name card handling (Q65, Q69, Q81)
    pub fn get_card_names(&self, card_id: i16) -> Vec<String> {
        if let Some(card) = self.cards.get(&card_id) {
            // Handle both regular '&' and full-width '＆' separators
            Self::normalize_name(&card.name)
                .replace('＆', "&")
                .split('&')
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Check if card has any of the given names (for multi-name cards)
    pub fn card_has_any_name(&self, card_id: i16, names: &[&str]) -> bool {
        let card_names = self.get_card_names(card_id);
        names.iter().any(|&name| {
            let norm = Self::normalize_name(name);
            card_names.iter().any(|cn| cn.contains(&norm))
        })
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
        })
    }
}

fn map_series_to_group(series: &str) -> String {
    match series {
        "ラブライブ！" => "μ's".to_string(),
        "ラブライブ！サンシャイン!!" => "Aqours".to_string(),
        "ラブライブ！虹ヶ咲学園スクールアイドル同好会" => {
            "虹ヶ咲".to_string()
        }
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
        let hearts = raw
            .hearts
            .into_iter()
            .map(|(k, v)| (parse_heart_color(&k), v))
            .collect();

        Ok(SpecialHeart { hearts })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AbilityCost(pub AbilityEffect);

impl AbilityCost {
    /// Borrow the inner effect.
    pub fn as_effect(&self) -> &AbilityEffect {
        &self.0
    }

    /// Consume the cost and return the inner effect.
    pub fn into_effect(self) -> AbilityEffect {
        self.0
    }
}

impl From<AbilityCost> for AbilityEffect {
    fn from(cost: AbilityCost) -> Self {
        cost.0
    }
}

impl From<AbilityEffect> for AbilityCost {
    fn from(effect: AbilityEffect) -> Self {
        AbilityCost(effect)
    }
}

impl std::ops::Deref for AbilityCost {
    type Target = AbilityEffect;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for AbilityCost {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl serde::Serialize for AbilityCost {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Serialize as the legacy cost JSON shape so any existing consumers
        // (frontend, debug tooling) keep working. Map the few renames:
        //   action        → "type"
        //   energy_count  → "energy"
        //   sub-effects via compound.actions → "options" (preferred) or "costs"
        use serde::ser::SerializeMap;
        let inner = &self.0;
        let mut map = s.serialize_map(None)?;
        map.serialize_entry("text", &inner.text)?;
        if let Some(ref v) = inner.source {
            map.serialize_entry("source", v)?;
        }
        if let Some(ref v) = inner.source {
            // legacy duplicate key kept for compat
            map.serialize_entry("zone", v)?;
        }
        if let Some(ref v) = inner.destination {
            map.serialize_entry("destination", v)?;
        }
        if let Some(ref v) = inner.count {
            map.serialize_entry("count", v)?;
        }
        if let Some(ref v) = inner.card_type {
            map.serialize_entry("card_type", v)?;
        }
        if let Some(ref v) = inner.target {
            map.serialize_entry("target", v)?;
        }
        if let Some(v) = inner.optional {
            map.serialize_entry("optional", &v)?;
        }
        if let Some(ref v) = inner.energy_count {
            map.serialize_entry("energy", v)?;
        }
        if let Some(ref v) = inner.state_change {
            map.serialize_entry("state_change", v)?;
        }
        if let Some(ref v) = inner.position {
            map.serialize_entry("position", v)?;
        }
        if let Some(ref v) = inner.self_cost {
            map.serialize_entry("self_cost", v)?;
        }
        if let Some(v) = inner.exclude_self {
            map.serialize_entry("exclude_self", &v)?;
        }
        if let Some(ref v) = inner.same_unit_name {
            map.serialize_entry("same_unit_name", v)?;
        }
        if let Some(ref v) = inner.shuffle {
            map.serialize_entry("shuffle", v)?;
        }
        if let Some(ref v) = inner.any_number {
            map.serialize_entry("any_number", v)?;
        }
        if let Some(ref v) = inner.cost_limit {
            map.serialize_entry("cost_limit", v)?;
        }
        if let Some(ref v) = inner.cost_limit_operator {
            map.serialize_entry("cost_limit_operator", v)?;
        }
        if let Some(ref v) = inner.characters {
            map.serialize_entry("characters", v)?;
        }
        if let Some(ref v) = inner.exclude_characters {
            map.serialize_entry("exclude_characters", v)?;
        }
        if let Some(ref v) = inner.group_names {
            map.serialize_entry("group_names", v)?;
        }
        if let Some(ref v) = inner.placement_order {
            map.serialize_entry("placement_order", v)?;
        }
        if let Some(ref v) = inner.alternative_effect {
            map.serialize_entry("alternative_effect", v)?;
        }
        if !inner.action.is_empty() {
            map.serialize_entry("type", &inner.action)?;
        }
        // sub-costs: emit as both "options" and "costs" (legacy treated them
        // as the same field)
        if let Some(ref actions) = inner.compound.actions {
            map.serialize_entry("options", actions)?;
            map.serialize_entry("costs", actions)?;
        }
        map.end()
    }
}

impl<'de> serde::Deserialize<'de> for AbilityCost {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::MapAccess;
        #[derive(Default)]
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = AbilityCost;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("an ability cost object (legacy or unified form)")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<AbilityCost, M::Error> {
                let mut effect = AbilityEffect::default();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "text" => effect.text = map.next_value()?,
                        "type" | "action" | "cost_type" => {
                            effect.action = map.next_value()?;
                        }
                        "source" | "zone" => {
                            effect.source = map.next_value()?;
                        }
                        "destination" => effect.destination = map.next_value()?,
                        "count" => effect.count = map.next_value()?,
                        "card_type" => effect.card_type = map.next_value()?,
                        "target" => effect.target = map.next_value()?,
                        "optional" => effect.optional = map.next_value()?,
                        "energy" | "energy_count" => {
                            effect.energy_count = map.next_value()?;
                        }
                        "state_change" => effect.state_change = map.next_value()?,
                        "position" => effect.position = map.next_value()?,
                        "self_cost" => effect.self_cost = map.next_value()?,
                        "exclude_self" => effect.exclude_self = map.next_value()?,
                        "same_unit_name" => effect.same_unit_name = map.next_value()?,
                        "shuffle" => effect.shuffle = map.next_value()?,
                        "any_number" => effect.any_number = map.next_value()?,
                        "cost_limit" => effect.cost_limit = map.next_value()?,
                        "cost_limit_operator" => effect.cost_limit_operator = map.next_value()?,
                        "characters" => effect.characters = map.next_value()?,
                        "exclude_characters" => effect.exclude_characters = map.next_value()?,
                        "group_names" => effect.group_names = map.next_value()?,
                        "placement_order" => effect.placement_order = map.next_value()?,
                        "alternative_effect" => {
                            effect.alternative_effect = map.next_value()?;
                        }
                        "options" | "costs" => {
                            // Sub-costs become sub-effects in compound.actions.
                            // Deserialize as AbilityCost so the rename mappings
                            // (type→action, energy→energy_count) are applied.
                            let sub: Vec<AbilityCost> = map.next_value()?;
                            effect.compound.actions =
                                Some(sub.into_iter().map(AbilityCost::into_effect).collect());
                        }
                        // Ignore unknown legacy cost fields rather than failing
                        // — the parser has been adding fields over time.
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                Ok(AbilityCost(effect))
            }
        }
        d.deserialize_map(Visitor)
    }
}

impl AbilityCost {
    /// Build a `CardFilter` containing the same 7 base filter fields that
    /// `AbilityEffect::filter_subset` exposes. Mirrors that method so cost
    /// handlers can use the same consolidation pattern as effect handlers.
    pub fn filter_subset(&self) -> crate::ability::util::CardFilter<'_> {
        crate::ability::util::CardFilter {
            card_type: self.card_type.as_deref(),
            group: self
                .group_names
                .as_ref()
                .and_then(|v| v.first())
                .map(|s| s.as_str()),
            cost_limit: self.cost_limit,
            cost_operator: self.cost_limit_operator.as_deref(),
            characters: self.characters.as_ref(),
            exclude_characters: self.exclude_characters.as_ref(),
            exclude_self: if self.exclude_self.unwrap_or(false) {
                Some(-1)
            } else {
                None
            },
            exclude_group_names: self.exclude_group_names.as_ref(),
            ..Default::default()
        }
    }
}

/// Grouped sub-effect fields used by compound action handlers
/// (Sequential, ConditionalAlternative, ConditionalOnResult, ConditionalOnOptional, LookAndSelect).
/// Flattened into AbilityEffect via `#[serde(flatten)]` for JSON backward compat.
///
/// The 4 specialized compound shapes (look_and_select, conditional_alternative,
/// conditional_on_result, conditional_on_optional) are all normalized into the
/// unified `effect_steps` form by the engine on dispatch. The legacy fields
/// (look_action/select_action/primary_effect/...) are kept here for backward
/// compatibility with previously-generated `abilities.json` files; new
/// parsers should emit only `effect_steps`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
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

/// One branch of an OR'd ability filter. At least one branch must match
/// for the card to pass.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AbilityFilterBranch {
    pub ability_filter: Option<String>,
    pub ability_filter_triggers: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
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
    #[serde(default)]
    pub negation: Option<bool>,
    pub max: Option<bool>,
    pub effect_constraint: Option<String>,
    pub resource_icon_count: Option<u32>,
    pub ability_gain: Option<String>,
    #[serde(default)]
    pub gained_effect: Option<Box<AbilityEffect>>,
    pub quoted_text: Option<QuotedText>,
    pub per_unit: Option<bool>,
    pub condition: Option<Condition>,
    /// Compound sub-effects (sequential, conditional, look_and_select branches).
    /// Flat fields in JSON are collected here via `#[serde(flatten)]`.
    #[serde(flatten)]
    pub compound: CompoundBranch,
    /// Cost-only: "this is a same-unit cost" (used by pay_cost handlers).
    /// Carried on AbilityEffect because AbilityCost is now a newtype around
    /// AbilityEffect — there is no separate type to put it on.
    #[serde(default)]
    pub same_unit_name: Option<bool>,
    /// Cost-only: shuffle the zone after paying.
    #[serde(default)]
    pub shuffle: Option<bool>,
    /// Effect that fires instead when this optional cost is skipped
    /// ("unless you pay"). Lives on AbilityEffect so both cost-as-effect
    /// and effect-as-effect contexts can carry it. The `alias` accepts
    /// the old `compound.alternative_effect` key from pre-unification JSON.
    #[serde(default, alias = "alternative_effect_legacy")]
    pub alternative_effect: Option<Box<AbilityEffect>>,
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
    /// Zone to count cards in for per_unit calculations. When absent, falls back
    /// to the effect's `location` field (or "hand" as default).
    pub per_unit_location: Option<String>,
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
    /// Minimum cost bound for range filters (e.g. "コスト4以上9以下" → min=4)
    #[serde(default)]
    pub cost_limit_min: Option<u32>,
    /// Maximum cost bound for range filters (e.g. "コスト4以上9以下" → max=9)
    #[serde(default)]
    pub cost_limit_max: Option<u32>,
    /// Sum-total cost constraint (e.g. "total cost ≤ 4")
    #[serde(default)]
    pub cost_total: Option<u32>,
    #[serde(default)]
    pub cost_total_operator: Option<String>,
    #[serde(default)]
    pub any_number: Option<bool>,
    #[serde(default)]
    pub alternative_count_type: Option<String>,
    #[serde(default)]
    pub discard_remaining: Option<bool>,
    /// Required hearts sum filter (e.g. "total need_heart ≥ 8")
    #[serde(default)]
    pub need_heart_total: Option<u32>,
    #[serde(default)]
    pub need_heart_operator: Option<String>,
    /// Per-color need_heart filter (e.g. "heart06 >= 3")
    #[serde(default)]
    pub need_heart_color: Option<String>,
    #[serde(default)]
    pub reveal: Option<bool>,
    #[serde(default)]
    pub per_group: Option<bool>,
    #[serde(default)]
    pub per_group_count: Option<u32>,
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
    /// Cost lookup reference for relative cost filters (e.g. previous moved card + 2).
    #[serde(default)]
    pub cost_reference: Option<String>,
    #[serde(default)]
    pub cost_offset: Option<i32>,
    /// parenthetical annotations (e.g. rule clarifications in parentheses)
    #[serde(default)]
    pub parenthetical: Option<Vec<String>>,
    /// characters filter for card selection
    #[serde(default)]
    pub characters: Option<Vec<String>>,
    pub exclude_characters: Option<Vec<String>>,
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
    /// Card property filter (e.g. "has_blade_heart")
    #[serde(default)]
    pub card_property: Option<String>,
    #[serde(default)]
    pub exclude_group_names: Option<Vec<String>>,
    #[serde(default)]
    pub heart_selection: Option<bool>,
    #[serde(default)]
    pub filter_targets_by_heart_colors: Option<bool>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub trigger_filter: Option<Vec<String>>,
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
    /// When true, this effect can place cards on occupied stage slots
    /// (replacing existing cards). Used for Q76-style rulings.
    #[serde(default)]
    pub allow_occupied_stage: Option<bool>,
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
    /// "この効果は重複しない" — this effect does not stack with copies of itself
    #[serde(default)]
    pub non_stackable: Option<bool>,
    /// Delayed restriction — applied at a later timing (e.g. アクティブしない)
    #[serde(default)]
    pub delayed: Option<bool>,
    /// Blind selection: "見ないで" — the selecting player should not see card identities
    #[serde(default)]
    pub blind: Option<bool>,
    /// Dynamic group reference (same_group_name, different_group_names)
    #[serde(default)]
    pub group_reference: Option<String>,
    /// Original blade limit filter (元々持つブレードの数)
    #[serde(default)]
    pub blade_limit: Option<u32>,
    #[serde(default)]
    pub blade_limit_operator: Option<String>,
    /// Original cost threshold (e.g. "元々のコストが17以上" → original_count: 17)
    #[serde(default)]
    pub original_count: Option<u32>,
    /// Operator for original cost threshold (e.g. ">=", "<=", "==")
    #[serde(default)]
    pub original_operator: Option<String>,
    /// Resolve cost_limit from the first revealed card at runtime.
    /// Used by followup actions that reference "これにより公開したカードのコスト以下".
    #[serde(default)]
    pub cost_from_revealed: Option<bool>,
    /// Step identifier. When this effect runs inside a `sequential` and has
    /// an `id`, its outputs (selected card ids, revealed card ids, etc.) are
    /// stored under this key in the resolver's `step_results` map, and later
    /// steps in the same sequential can reference them via `ref: "<id>"`.
    #[serde(default)]
    pub id: Option<String>,
    /// Cross-step card reference. When set on a field like `source`,
    /// `destination`, or a filter, the engine resolves it against
    /// `step_results[<ref>].cards` and substitutes the referenced card ids
    /// (e.g. as the `source` zone of a `move_cards`, or as the card names
    /// to filter by). This replaces the implicit global-state handoffs
    /// (`gs.revealed_cards`, `gs.recently_moved_cards`) with explicit links.
    #[serde(default)]
    pub r#ref: Option<String>,
    /// Cross-step value reference. When set on a field like `value` or
    /// `count`, the engine resolves it against `step_results[<ref>].value`.
    /// Optional offset for patterns like "selected card's score - 1".
    #[serde(default)]
    pub ref_value: Option<String>,
    /// Offset to add to `ref_value` after resolution. Defaults to 0.
    #[serde(default)]
    pub ref_offset: Option<i32>,
    /// Unified sub-effect steps. When this is `Some`, the 4 specialized
    /// compound shapes (look_and_select, conditional_alternative,
    /// conditional_on_result, conditional_on_optional) all reduce to a
    /// single `actions` list of `AbilityEffect`s, each with an optional
    /// per-step `condition` and an `id` for cross-step references. This is
    /// the consolidation target; new parsers should emit only this field
    /// for compound effects. The legacy `look_action`/`select_action`/
    /// `primary_effect`/`alternative_effect`/... fields are still parsed
    /// for backward compat but should not be emitted alongside.
    #[serde(default)]
    pub effect_steps: Option<Vec<AbilityEffect>>,
    /// ability_filter: "no_ability" / "has_ability" / "no_ability_type"
    /// Filters cards by presence or absence of abilities / trigger types.
    #[serde(default)]
    pub ability_filter: Option<String>,
    /// Trigger types excluded when ability_filter is "no_ability_type"
    #[serde(default)]
    pub ability_filter_triggers: Option<Vec<String>>,
    /// OR'd ability filter branches. When present, a card passes if ANY
    /// branch matches (replaces the single ability_filter check).
    /// Used for patterns like "no ability OR has 常時 ability".
    #[serde(default)]
    pub or_ability_filters: Option<Vec<AbilityFilterBranch>>,
}

impl AbilityEffect {
    /// Returns the target player string, defaulting to "self".
    pub fn target_name(&self) -> &str {
        self.target.as_deref().unwrap_or("self")
    }

    /// Returns the source zone string with a static default.
    pub fn source_or(&self, default: &'static str) -> &str {
        self.source.as_deref().unwrap_or(default)
    }

    /// Build a `CardFilter` containing the 7 base filter fields (card_type,
    /// group, cost_limit, cost_operator, characters, exclude_characters,
    /// exclude_self) that effect handlers most commonly need. This is the
    /// subset of `CardFilter::from_effect` that matches the field set
    /// `filter_from_parts` exposes — handlers that need the full filter
    /// (heart_colors, distinct, exclude_cards, etc.) should call
    /// `CardFilter::from_effect` instead. Handlers that need to override
    /// individual fields can mutate the returned `CardFilter` directly or
    /// use the builder methods (`.card_type_opt`, `.group_opt`, etc.).
    pub fn filter_subset(&self) -> crate::ability::util::CardFilter<'_> {
        crate::ability::util::CardFilter {
            card_type: self.card_type.as_deref(),
            group: self.group_name(),
            cost_limit: self.cost_limit,
            cost_operator: self.cost_limit_operator.as_deref(),
            characters: self.characters.as_ref(),
            exclude_characters: self.exclude_characters.as_ref(),
            exclude_self: if self.exclude_self.unwrap_or(false) {
                Some(-1)
            } else {
                None
            },
            exclude_group_names: self.exclude_group_names.as_ref(),
            ..Default::default()
        }
    }

    /// Returns the count with a caller-provided default.
    pub fn count_or(&self, n: u32) -> u32 {
        self.count.unwrap_or(n)
    }

    /// Returns the first group name, if any.
    pub fn group_name(&self) -> Option<&str> {
        self.group_names
            .as_ref()
            .and_then(|gn| gn.first().map(|s| s.as_str()))
    }

    /// Returns the first heart color as a string reference, or a static default.
    /// For single-color operations like modify_required_hearts.
    pub fn heart_color_or(&self, default: &'static str) -> &str {
        self.heart_colors
            .first()
            .map(|s| s.as_str())
            .unwrap_or(default)
    }

    /// Parses the action string into a typed ActionType for type-safe matching.
    pub fn action_type(&self) -> Option<ActionType> {
        ActionType::from_str(&self.action)
    }

    /// Returns true if the action matches the given ActionType variant.
    pub fn is_action(&self, at: ActionType) -> bool {
        self.action_type() == Some(at)
    }

    /// Returns the numeric value from `value` or `count`, in that priority.
    /// Consolidates the many `effect.value.or(effect.count)` patterns in dispatch.
    pub fn value_or_count(&self, default: u32) -> u32 {
        self.value.or(self.count).unwrap_or(default)
    }

    /// Like `value_or_count`, but if a `ref_value` is set, resolves against
    /// the supplied step_results to a value the referenced step produced
    /// (plus any `ref_offset`). Falls back to `value_or_count(default)` when
    /// the reference is absent or unresolvable. This is the type-safe
    /// replacement for the implicit dynamic_count / cost_reference lookups
    /// that currently live in handlers like `execute_modify_cost` and
    /// `execute_draw_wrapper`.
    pub fn value_or_count_resolved(
        &self,
        step_results: &std::collections::HashMap<String, crate::ability::types::StepOutput>,
        default: u32,
    ) -> i32 {
        if let Some(ref id) = self.ref_value {
            if let Some(out) = step_results.get(id) {
                if let Some(v) = out.value {
                    return v + self.ref_offset.unwrap_or(0);
                }
            }
        }
        self.value.or(self.count).unwrap_or(default) as i32
    }

    /// Returns the normalized sub-effect steps for this effect, preferring
    /// the unified `effect_steps` form when present and otherwise building
    /// the steps list from the legacy compound fields
    /// (`look_action`/`select_action`/etc.). This is the consolidation point
    /// for the 4 specialized compound shapes — once the parser emits only
    /// `effect_steps`, the legacy branches in this function become dead code
    /// and can be removed.
    pub fn normalized_steps(&self) -> Vec<AbilityEffect> {
        if let Some(ref steps) = self.effect_steps {
            return steps.clone();
        }
        match self.action.as_str() {
            "sequential" => self.compound.actions.clone().unwrap_or_default(),
            "look_and_select" => {
                let mut out = Vec::new();
                if let Some(ref la) = self.compound.look_action {
                    out.push((**la).clone());
                }
                if let Some(ref sa) = self.compound.select_action {
                    out.push((**sa).clone());
                }
                // Followup action must be a step in the sequential pipeline
                // so it runs after the selection completes — the legacy
                // handler saved it as a pending_command manually, but the
                // sequential pipeline expects it as an effect_steps entry.
                if let Some(ref fu) = self.compound.followup_action {
                    out.push((**fu).clone());
                }
                out
            }
            "conditional_alternative" => {
                let mut out = Vec::new();
                let mut primary = self.compound.primary_effect.clone().map(|b| *b);
                let mut alternative = self.alternative_effect.clone().map(|b| *b);
                let condition = self.compound.alternative_condition.clone();
                if let Some(mut alt) = alternative.take() {
                    if let Some(cond) = condition {
                        // Per-step condition: run alternative if cond met,
                        // else fall through to primary.
                        alt.condition = Some(cond);
                    }
                    out.push(alt);
                }
                if let Some(pri) = primary.take() {
                    out.push(pri);
                }
                out
            }
            "conditional_on_result" => {
                let mut out = Vec::new();
                if let Some(ref pri) = self.compound.primary_effect {
                    out.push((**pri).clone());
                }
                if let Some(ref follow) = self.compound.followup_action {
                    let mut f = (**follow).clone();
                    if let Some(rc) = self.compound.result_condition.clone() {
                        f.condition = Some(rc);
                    }
                    out.push(f);
                }
                out
            }
            "conditional_on_optional" => {
                // Optional yes/no choice between optional_action and
                // conditional_action. The legacy handler creates a
                // SelectTarget("conditional_optional") choice and dispatches
                // based on the answer. We model that as a single step with
                // a special action name that the engine still routes to the
                // existing execute_conditional_on_optional handler, but
                // accessed through the unified sequential pipeline.
                let mut step = AbilityEffect::default();
                step.action = "conditional_optional".to_string();
                if let Some(ref oa) = self.compound.optional_action {
                    step.text = oa.text.clone();
                }
                // Reuse the existing compound fields via direct dispatch
                // in the engine. We embed the legacy handler's needed
                // inputs into a single step that the engine recognizes.
                step.compound.optional_action = self.compound.optional_action.clone();
                step.compound.conditional_action = self.compound.conditional_action.clone();
                step.compound.conditional_negation = self.compound.conditional_negation;
                step.compound.conditional_negation = self.compound.conditional_negation;
                vec![step]
            }
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DynamicCount {
    #[serde(rename = "type")]
    pub count_type: String,
    pub reference: Option<String>,
    pub mode: Option<String>,
    pub base_reference: Option<String>,
    pub calculation: Option<String>,
    pub calculation_value: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuotedText {
    pub text: String,
    pub quoted_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum DistinctInfo {
    Boolean(bool),
    String(String),
}

impl DistinctInfo {
    pub fn is_distinct(&self) -> bool {
        match self {
            DistinctInfo::Boolean(b) => *b,
            DistinctInfo::String(s) => s != "false" && !s.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Condition {
    #[serde(default = "default_empty_string")]
    pub text: String,
    #[serde(rename = "type")]
    pub condition_type: Option<crate::ability::enums::ConditionType>,
    pub location: Option<String>,
    pub locations: Option<Vec<String>>,
    pub count: Option<u32>,
    pub operator: Option<String>,
    pub card_type: Option<String>,
    pub target: Option<String>,
    pub group_names: Option<Vec<String>>,
    #[serde(default)]
    pub exclude_group_names: Option<Vec<String>>,
    pub characters: Option<Vec<String>>,
    pub exclude_characters: Option<Vec<String>>,
    pub state: Option<String>,
    pub position: Option<PositionInfo>,
    /// Cross-position comparison target (e.g. "right_side" when position is "left_side")
    #[serde(default)]
    pub position_compare: Option<String>,
    pub temporal_scope: Option<String>,
    #[serde(default)]
    pub distinct: Option<DistinctInfo>,
    pub exclude_self: Option<bool>,
    pub any_of: Option<Vec<String>>,
    pub cost_limit: Option<u32>,
    #[serde(default)]
    pub cost_limit_operator: Option<String>,
    pub negation: Option<bool>,
    pub baton_touch_trigger: Option<bool>,
    pub baton_touch_source: Option<String>,
    pub min_baton_touch_count: Option<u32>,
    pub movement_state: Option<String>,
    pub energy_state: Option<String>,
    pub comparison_target: Option<String>,
    #[serde(default)]
    pub comparison_source: Option<String>,
    pub movement: Option<String>,
    pub temporal: Option<String>,
    pub phase: Option<String>,
    pub comparison_type: Option<String>,
    pub appearance: Option<bool>,
    #[serde(default)]
    pub appearance_source: Option<String>,
    pub conditions: Option<Vec<Condition>>,
    pub options: Option<Vec<AbilityEffect>>,
    #[serde(default)]
    pub condition: Option<Box<Condition>>,
    pub card_property: Option<String>,
    // New fields from parser improvements
    pub all_areas: Option<bool>,
    pub no_excess_heart: Option<bool>,
    pub resource_type: Option<String>,
    pub turn_number: Option<u32>,
    pub activation_position: Option<String>,
    pub all: Option<bool>,
    pub unit: Option<String>,
    pub values: Option<Vec<u32>>,
    // Complex condition fields
    #[serde(default)]
    pub cause: Option<Box<Condition>>,
    #[serde(default)]
    pub effect: Option<Box<AbilityEffect>>,
    // Same-name constraint: members must share a character name
    #[serde(default)]
    pub same_name: Option<bool>,
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
    /// ability_filter: "no_ability" for "能力を持たない" (card does not have abilities)
    /// or "has_ability" for "能力を持つ" (card has abilities)
    /// or "no_ability_type" for "能力も...能力も持たない" (card has neither type)
    #[serde(default)]
    pub ability_filter: Option<String>,
    /// Trigger types excluded by ability_filter (e.g. ["live_start", "live_success"])
    /// Used when ability_filter is "no_ability_type"
    #[serde(default)]
    pub ability_filter_triggers: Option<Vec<String>>,
    /// Sum-total cost comparison value (e.g. "コストの合計がN")
    #[serde(default)]
    pub cost_total: Option<u32>,
    #[serde(default)]
    pub cost_total_operator: Option<String>,
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
    /// "自分のカードの効果" — only trigger if the event was caused by the player's own card effect.
    #[serde(default)]
    pub self_effect_only: Option<bool>,
    /// "エネルギーが置かれた" — trigger is specifically about energy being placed in the energy zone.
    #[serde(default)]
    pub energy_placed: Option<bool>,
    /// Cost comparison between characters: 「A」よりコストの(大きい|高い)「B」
    /// When set, the subject character (characters list) must have cost greater
    /// than cost_reference_character on the target's stage.
    #[serde(default)]
    pub cost_reference_character: Option<String>,
    #[serde(default)]
    pub cost_reference_operator: Option<String>,
    #[serde(default)]
    pub cost_reference_type: Option<String>,
    /// delta: true — check the change (difference) caused by preceding action,
    /// not the absolute current value. Used for surplus heart loss tracking.
    #[serde(default)]
    pub delta: Option<bool>,
    /// reference_card: "previous_selected" — compare card names against the
    /// card selected by the preceding select action. Used for "同じカード名"
    /// (same card name) conditions.
    #[serde(default)]
    pub reference_card: Option<String>,
    /// self_target: condition refers to this specific card ("このメンバーが" / "このカードが").
    #[serde(default)]
    pub self_target: Option<bool>,
}

impl Condition {
    /// Build a `CardFilter` containing the 7 base filter fields, mirroring
    /// `AbilityEffect::filter_subset` and `AbilityCost::filter_subset`.
    /// Note: Condition uses `operator` where AbilityEffect uses
    /// `cost_limit_operator`; the mapping is direct since both are
    /// comparison operators for the cost filter.
    pub fn filter_subset(&self) -> crate::ability::util::CardFilter<'_> {
        crate::ability::util::CardFilter {
            card_type: self.card_type.as_deref(),
            group: self
                .group_names
                .as_ref()
                .and_then(|v| v.first())
                .map(|s| s.as_str()),
            cost_limit: self.cost_limit,
            cost_operator: self.operator.as_deref(),
            characters: self.characters.as_ref(),
            exclude_characters: self.exclude_characters.as_ref(),
            exclude_self: None,
            exclude_group_names: self.exclude_group_names.as_ref(),
            ..Default::default()
        }
    }
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

    /// Total required hearts (sum of all need_heart values).
    pub fn need_heart_total(&self) -> u32 {
        self.need_heart
            .as_ref()
            .map(|nh| nh.hearts.values().sum())
            .unwrap_or(0)
    }

    pub fn has_blade_heart(&self) -> bool {
        self.blade_heart.is_some()
            || self
                .special_heart
                .as_ref()
                .is_some_and(|sh| !sh.hearts.is_empty())
    }

    pub fn has_score_icon(&self) -> bool {
        self.special_heart
            .as_ref()
            .is_some_and(|sh| sh.hearts.contains_key(&HeartColor::Score))
    }

    /// Check if a given need_heart is satisfied by provided hearts.
    /// This is identical to satisfies_heart_requirement but allows an
    /// externally-adjusted need_heart (e.g. with modifiers applied).
    pub fn need_heart_satisfied(need: &BaseHeart, provided_hearts: &BaseHeart) -> bool {
        if need.hearts.is_empty() {
            return true;
        }
        // Rule 2.11.3 bullet 2: total provided must be >= total required.
        let total_provided: u32 = provided_hearts.hearts.values().sum();
        let total_required: u32 = need.hearts.values().sum();
        if total_provided < total_required {
            return false;
        }
        // Both Heart00 (wildcard) and HeartColor::All (all-heart) act as
        // flexible supply that can fill any specific-color deficit.
        let mut wildcard_remaining = *provided_hearts
            .hearts
            .get(&HeartColor::Heart00)
            .unwrap_or(&0) as i32
            + *provided_hearts.hearts.get(&HeartColor::All).unwrap_or(&0) as i32;
        // Track remaining hearts per color. As specific colors are fulfilled,
        // the used hearts are deducted from this map so that the heart0 check
        // sees only hearts that have NOT already been allocated.
        let mut remaining = provided_hearts.hearts.clone();
        // Process specific colors first (Heart01-Heart06) before Heart00 (Fix B).
        // This avoids non-deterministic double-counting from HashMap iteration order.
        for (color, &needed_amount) in &need.hearts {
            if *color == HeartColor::Heart00 {
                continue;
            }
            let provided = *remaining.get(color).unwrap_or(&0) as i32;
            if provided + wildcard_remaining < needed_amount as i32 {
                return false;
            }
            let shortfall = (needed_amount as i32 - provided).max(0);
            wildcard_remaining -= shortfall;
            let consumed = needed_amount.min(*remaining.get(color).unwrap_or(&0));
            if let Some(rem) = remaining.get_mut(color) {
                *rem -= consumed;
            }
        }
        // Then process Heart00 (wildcard requirement) using remaining hearts.
        if let Some(&heart00_needed) = need.hearts.get(&HeartColor::Heart00) {
            let leftover_sum: i32 = remaining
                .iter()
                .filter(|(c, _)| **c != HeartColor::Heart00 && **c != HeartColor::All)
                .map(|(_, v)| *v as i32)
                .sum();
            if leftover_sum + wildcard_remaining.max(0) < heart00_needed as i32 {
                return false;
            }
        }
        true
    }

    pub fn satisfies_heart_requirement(&self, provided_hearts: &BaseHeart) -> bool {
        if let Some(ref need_heart) = self.need_heart {
            check_heart_requirement(need_heart, provided_hearts)
        } else {
            true
        }
    }
}

pub fn check_heart_requirement(need: &BaseHeart, provided: &BaseHeart) -> bool {
    if need.hearts.is_empty() {
        return true;
    }
    // Rule 2.11.3 bullet 2: total provided must be >= total required.
    let total_provided: u32 = provided.hearts.values().sum();
    let total_required: u32 = need.hearts.values().sum();
    if total_provided < total_required {
        return false;
    }
    let mut wildcard_remaining = *provided.hearts.get(&HeartColor::Heart00).unwrap_or(&0) as i32
        + *provided.hearts.get(&HeartColor::All).unwrap_or(&0) as i32;
    let mut remaining = provided.hearts.clone();
    // Process specific colors first (Heart01-Heart06) before Heart00 (Fix B).
    for (color, &needed_amount) in &need.hearts {
        if *color == HeartColor::Heart00 {
            continue;
        }
        let provided_val = *remaining.get(color).unwrap_or(&0) as i32;
        if provided_val + wildcard_remaining < needed_amount as i32 {
            return false;
        }
        let shortfall = (needed_amount as i32 - provided_val).max(0);
        wildcard_remaining -= shortfall;
        let consumed = needed_amount.min(*remaining.get(color).unwrap_or(&0));
        if let Some(rem) = remaining.get_mut(color) {
            *rem -= consumed;
        }
    }
    // Then process Heart00 (wildcard requirement) using remaining hearts.
    if let Some(&heart00_needed) = need.hearts.get(&HeartColor::Heart00) {
        let leftover_sum: i32 = remaining
            .iter()
            .filter(|(c, _)| **c != HeartColor::Heart00 && **c != HeartColor::All)
            .map(|(_, v)| *v as i32)
            .sum();
        if leftover_sum + wildcard_remaining.max(0) < heart00_needed as i32 {
            return false;
        }
    }
    true
}

impl std::fmt::Display for HeartColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeartColor::Heart00 => write!(f, "heart00"),
            HeartColor::Heart01 => write!(f, "heart01"),
            HeartColor::Heart02 => write!(f, "heart02"),
            HeartColor::Heart03 => write!(f, "heart03"),
            HeartColor::Heart04 => write!(f, "heart04"),
            HeartColor::Heart05 => write!(f, "heart05"),
            HeartColor::Heart06 => write!(f, "heart06"),
            HeartColor::BAll => write!(f, "b_all"),
            HeartColor::Draw => write!(f, "draw"),
            HeartColor::Score => write!(f, "score"),
            HeartColor::All => write!(f, "all"),
        }
    }
}

impl HeartColor {
    /// Returns the index of this heart color (0-6, 0=Heart00 wildcard, 1-6=Heart01-Heart06).
    pub fn index(&self) -> usize {
        match self {
            HeartColor::Heart00 => 0,
            HeartColor::Heart01 => 1,
            HeartColor::Heart02 => 2,
            HeartColor::Heart03 => 3,
            HeartColor::Heart04 => 4,
            HeartColor::Heart05 => 5,
            HeartColor::Heart06 => 6,
            HeartColor::All => 7,
            _ => 0,
        }
    }

    /// Reconstruct a HeartColor from an index (0-7).
    pub fn from_index(i: usize) -> Self {
        match i {
            0 => HeartColor::Heart00,
            1 => HeartColor::Heart01,
            2 => HeartColor::Heart02,
            3 => HeartColor::Heart03,
            4 => HeartColor::Heart04,
            5 => HeartColor::Heart05,
            6 => HeartColor::Heart06,
            7 => HeartColor::All,
            _ => HeartColor::Heart00,
        }
    }

    /// Returns the short label used in display output ("h00"-"h06").
    pub fn short_label(&self) -> &'static str {
        match self {
            HeartColor::Heart00 => "h00",
            HeartColor::Heart01 => "h01",
            HeartColor::Heart02 => "h02",
            HeartColor::Heart03 => "h03",
            HeartColor::Heart04 => "h04",
            HeartColor::Heart05 => "h05",
            HeartColor::Heart06 => "h06",
            HeartColor::BAll => "b_all",
            HeartColor::Draw => "draw",
            HeartColor::Score => "score",
            HeartColor::All => "all",
        }
    }
}

impl std::str::FromStr for HeartColor {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
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
            "all" => HeartColor::All,
            _ if s.starts_with("b_") => {
                HeartColor::from_str(&s[2..]).unwrap_or(HeartColor::Heart00)
            }
            _ => HeartColor::Heart00,
        })
    }
}

/// Canonical string→HeartColor conversion. Use `s.parse::<HeartColor>()` instead.
pub fn parse_heart_color(s: &str) -> HeartColor {
    s.parse().unwrap_or(HeartColor::Heart00)
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
                if crate::ability::enums::ActionType::from_str(&effect.action)
                    == Some(crate::ability::enums::ActionType::ModifyCost)
                    && effect.operation.as_deref() == Some("subtract")
                    && Zone::from_str(effect.location.as_deref().unwrap_or("")) == Some(Zone::Hand)
                {
                    let per_unit = effect.per_unit_count.unwrap_or(1) as usize;
                    return (hand_size.saturating_sub(1) * per_unit) as u32;
                }
            }
        }
        0
    }
}
