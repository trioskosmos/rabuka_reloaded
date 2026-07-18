use crate::ability::enums::{ActionType, ConditionType, EffectCardType, EffectState, Zone};
use crate::core::types::ArcStr;
use crate::Arc;
use crate::HashMap;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[cfg(feature = "psp")]
use alloc::{boxed::Box, string::String, string::ToString, vec::Vec};

#[cfg(not(feature = "psp"))]
pub(crate) use crate::core::pool::EkBox;
#[cfg(feature = "psp")]
pub(crate) type EkBox = alloc::boxed::Box<EffectKind>;

pub(crate) fn ek_box_new(val: EffectKind) -> EkBox {
    #[cfg(not(feature = "psp"))]
    {
        crate::core::pool::EkBox::new(val)
    }
    #[cfg(feature = "psp")]
    {
        alloc::boxed::Box::new(val)
    }
}

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

/// Efficient map of HeartColor→u32, backed by SmallVec (1-4 entries typical).
/// Serializes/deserializes as a flat JSON object (e.g. `{"heart01": 1, "heart03": 1}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeartMap(SmallVec<[(HeartColor, u32); 4]>);

impl HeartMap {
    pub fn new() -> Self {
        HeartMap(SmallVec::new())
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn values_sum(&self) -> u32 {
        self.0.iter().map(|(_, v)| v).sum()
    }
    pub fn get(&self, color: &HeartColor) -> Option<&u32> {
        self.0.iter().find(|(c, _)| c == color).map(|(_, v)| v)
    }
    pub fn get_mut(&mut self, color: &HeartColor) -> Option<&mut u32> {
        self.0.iter_mut().find(|(c, _)| c == color).map(|(_, v)| v)
    }
    pub fn contains_key(&self, color: &HeartColor) -> bool {
        self.0.iter().any(|(c, _)| c == color)
    }
    pub fn insert(&mut self, color: HeartColor, val: u32) {
        if let Some((_, v)) = self.0.iter_mut().find(|(c, _)| *c == color) {
            *v = val;
        } else {
            self.0.push((color, val));
        }
    }
    pub fn remove(&mut self, color: &HeartColor) {
        self.0.retain(|(c, _)| c != color);
    }
    pub fn clear(&mut self) {
        self.0.clear();
    }
    pub fn entry_or_default(&mut self, color: HeartColor) -> &mut u32 {
        let idx = self.0.iter().position(|(c, _)| c == &color);
        if let Some(i) = idx {
            &mut self.0[i].1
        } else {
            self.0.push((color, 0));
            &mut self.0.last_mut().unwrap().1
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &(HeartColor, u32)> {
        self.0.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut (HeartColor, u32)> {
        self.0.iter_mut()
    }
    pub fn keys(&self) -> impl Iterator<Item = &HeartColor> {
        self.0.iter().map(|(c, _)| c)
    }
    pub fn values(&self) -> impl Iterator<Item = &u32> {
        self.0.iter().map(|(_, v)| v)
    }
}

impl core::ops::Index<&HeartColor> for HeartMap {
    type Output = u32;
    fn index(&self, color: &HeartColor) -> &u32 {
        self.get(color).unwrap_or(&0)
    }
}

impl core::ops::IndexMut<&HeartColor> for HeartMap {
    fn index_mut(&mut self, color: &HeartColor) -> &mut u32 {
        self.entry_or_default(*color)
    }
}

impl<'a> IntoIterator for &'a HeartMap {
    type Item = &'a (HeartColor, u32);
    type IntoIter = core::slice::Iter<'a, (HeartColor, u32)>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl From<HashMap<HeartColor, u32>> for HeartMap {
    fn from(map: HashMap<HeartColor, u32>) -> Self {
        HeartMap(map.into_iter().collect())
    }
}

impl Default for HeartMap {
    fn default() -> Self {
        Self::new()
    }
}

impl Serialize for HeartMap {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (color, count) in &self.0 {
            map.serialize_entry(color, count)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for HeartMap {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(flatten)]
            hearts: HashMap<String, u32>,
        }
        let raw = Raw::deserialize(deserializer)?;
        let hearts = raw
            .hearts
            .into_iter()
            .map(|(k, v)| (parse_heart_color(&k), v))
            .collect();
        Ok(HeartMap(hearts))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BladeHeart {
    #[serde(flatten)]
    pub hearts: HeartMap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseHeart {
    #[serde(flatten)]
    pub hearts: HeartMap,
}

#[derive(Debug, Clone, Serialize)]
pub struct Card {
    pub card_no: ArcStr,
    pub img: Option<ArcStr>,
    pub name: ArcStr,
    #[serde(default)]
    pub product: Box<str>,
    #[serde(rename = "type")]
    pub card_type: CardType,
    #[serde(default)]
    pub series: Box<str>,
    #[serde(default = "default_group_from_series")]
    pub group: Box<str>,
    pub unit: Option<ArcStr>,
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
    pub _img: Option<ArcStr>,
    // Live card fields
    pub score: Option<u32>,
    pub need_heart: Option<BaseHeart>,
    pub special_heart: Option<SpecialHeart>,
    // Parsed abilities from abilities.json
    #[serde(skip)]
    pub abilities: Vec<Arc<Ability>>,
    /// Pre-baked ability data (populated by bake tool for PSP).
    /// Serialized so that PSP can avoid parsing abilities.json at runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baked_abilities: Option<Vec<Ability>>,
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
            cards: HashMap::default(),
            card_no_to_id: HashMap::default(),
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
            if !db.card_no_to_id.contains_key(card.card_no.as_ref()) {
                db.card_no_to_id
                    .insert(card.card_no.to_string(), db.next_id);
                db.next_id += 1;
            }
            let card_id = db.card_no_to_id[card.card_no.as_ref()];
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

    /// Normalize card_no for lookup: uppercase, fullwidth → halfwidth.
    /// Avoids allocation when input is already ASCII uppercase with no
    /// fullwidth characters (the common case after initial load).
    fn normalize_card_no(card_no: &str) -> String {
        let mut result = String::with_capacity(card_no.len());
        let mut changed = false;
        for ch in card_no.chars() {
            match ch {
                'a'..='z' => {
                    result.push((ch as u8 - b'a' + b'A') as char);
                    changed = true;
                }
                'ａ'..='ｚ' => {
                    result.push((ch as u32 - 'ａ' as u32 + 'A' as u32) as u8 as char);
                    changed = true;
                }
                '＋' => {
                    result.push('+');
                    changed = true;
                }
                '！' => {
                    result.push('!');
                    changed = true;
                }
                '－' => {
                    result.push('-');
                    changed = true;
                }
                '＊' => {
                    result.push('*');
                    changed = true;
                }
                '＃' => {
                    result.push('#');
                    changed = true;
                }
                _ => result.push(ch),
            }
        }
        if !changed {
            card_no.to_string()
        } else {
            result
        }
    }

    /// Strip all whitespace from a card name so that inconsistent spacing
    /// (e.g. "南 ことり" vs "南ことり") does not break ability conditions.
    /// Avoids allocation when no whitespace is present.
    pub fn normalize_name(name: &str) -> String {
        if name.bytes().all(|b| !b.is_ascii_whitespace()) {
            // Fast path: no ASCII whitespace — but still check for Unicode whitespace.
            // In practice card names rarely have Unicode whitespace, so this
            // covers the vast majority of calls without allocation.
            if !name.contains(|c: char| c.is_whitespace()) {
                return name.to_string();
            }
        }
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
            pub img: Option<ArcStr>,
            pub name: String,
            #[serde(default)]
            pub product: String,
            #[serde(rename = "type")]
            pub card_type: CardType,
            #[serde(default)]
            pub series: String,
            pub unit: Option<ArcStr>,
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
            pub _img: Option<ArcStr>,
            pub score: Option<u32>,
            pub need_heart: Option<BaseHeart>,
            pub special_heart: Option<SpecialHeart>,
            #[serde(default)]
            pub baked_abilities: Option<Vec<Ability>>,
        }

        let helper = CardHelper::deserialize(deserializer)?;
        let group = map_series_to_group(&helper.series);

        Ok(Card {
            card_no: ArcStr::from(helper.card_no),
            img: helper.img,
            name: ArcStr::from(helper.name),
            product: helper.product.into(),
            card_type: helper.card_type,
            series: helper.series.into(),
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
            baked_abilities: helper.baked_abilities,
        })
    }
}

fn map_series_to_group(series: &str) -> Box<str> {
    match series {
        "ラブライブ！" => "μ's".into(),
        "ラブライブ！サンシャイン!!" => "Aqours".into(),
        "ラブライブ！虹ヶ咲学園スクールアイドル同好会" => "虹ヶ咲".into(),
        "ラブライブ！スーパースター!!" => "Liella!".into(),
        "蓮ノ空女学院スクールアイドルクラブ" => "蓮ノ空".into(),
        _ => Box::from(""),
    }
}

fn default_blade() -> u32 {
    0
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecialHeart {
    #[serde(flatten)]
    pub hearts: HeartMap,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Ability {
    #[serde(default = "default_empty_string")]
    pub full_text: String,
    /// Trigger prefix stripped from `full_text` (e.g. "【自】"). Usually derived
    /// on demand from `full_text`; only populated directly when the source JSON
    /// carries an explicit, non-derivable value. `None` means "derive from
    /// `full_text`" — see `Ability::triggerless_text`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggerless_text: Option<String>,
    pub triggers: Option<ArcStr>,
    pub use_limit: Option<u32>,
    #[serde(default)]
    pub is_null: bool,
    pub cost: Option<Box<AbilityCost>>,
    pub effect: Option<Box<AbilityEffect>>,
    pub keywords: Option<Vec<Keyword>>,
}

fn default_empty_string() -> String {
    String::new()
}

impl Ability {
    /// Return the text with any leading trigger clause (e.g. `【自】`) stripped.
    /// When `triggerless_text` was explicitly set it is returned directly;
    /// otherwise it is derived from `full_text` on demand.
    pub fn triggerless_text(&self) -> &str {
        match &self.triggerless_text {
            Some(t) => t.as_str(),
            None => {
                let ft = self.full_text.trim_start();
                if let Some(rest) = ft.strip_prefix("【") {
                    if let Some(idx) = rest.find('】') {
                        return &ft[idx + '】'.len_utf8()..];
                    }
                }
                ft
            }
        }
    }
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

impl core::ops::Deref for AbilityCost {
    type Target = AbilityEffect;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for AbilityCost {
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
        if let Some(v) = inner.card_type_any() {
            map.serialize_entry("card_type", v)?;
        }
        if let Some(ref v) = inner.target {
            map.serialize_entry("target", v)?;
        }
        if let Some(v) = inner.optional_any() {
            map.serialize_entry("optional", &v)?;
        }
        if let Some(v) = inner.energy_count_any() {
            map.serialize_entry("energy", &v)?;
        }
        if let Some(v) = inner.state_change_any() {
            map.serialize_entry("state_change", v)?;
        }
        if let Some(v) = inner.position_any() {
            map.serialize_entry("position", v)?;
        }
        if let Some(v) = inner.self_cost_any() {
            map.serialize_entry("self_cost", &v)?;
        }
        if let Some(v) = inner.exclude_self_any() {
            map.serialize_entry("exclude_self", &v)?;
        }
        if let Some(v) = inner.same_unit_name_any() {
            map.serialize_entry("same_unit_name", &v)?;
        }
        if let Some(v) = inner.shuffle_any() {
            map.serialize_entry("shuffle", &v)?;
        }
        if let Some(v) = inner.any_number_any() {
            map.serialize_entry("any_number", &v)?;
        }
        if let Some(v) = inner.cost_limit_any() {
            map.serialize_entry("cost_limit", &v)?;
        }
        if let Some(v) = inner.cost_limit_operator_any() {
            map.serialize_entry("cost_limit_operator", v.as_str())?;
        }
        if let Some(v) = inner.characters_any() {
            map.serialize_entry("characters", v)?;
        }
        if let Some(v) = inner.exclude_characters_any() {
            map.serialize_entry("exclude_characters", v)?;
        }
        if let Some(v) = inner.group_names_any() {
            map.serialize_entry("group_names", v)?;
        }
        if let Some(v) = inner.placement_order_any() {
            map.serialize_entry("placement_order", v.as_str())?;
        }
        if let Some(v) = inner.alternative_effect_any() {
            map.serialize_entry("alternative_effect", v)?;
        }
        if inner.action != ActionType::Custom {
            map.serialize_entry("type", inner.action.to_str())?;
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
            fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.write_str("an ability cost object (legacy or unified form)")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<AbilityCost, M::Error> {
                let mut effect = AbilityEffect::default();
                let mut all_fields = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    let value: serde_json::Value = map.next_value()?;
                    all_fields.insert(key.clone(), value.clone());
                    match key.as_str() {
                        "text" => {
                            if let Some(s) = value.as_str() {
                                effect.text = s.to_string();
                            }
                        }
                        "type" | "action" | "cost_type" => {
                            if let Some(s) = value.as_str() {
                                effect.action = ActionType::from_str(s).unwrap_or_default();
                            }
                        }
                        "source" | "zone" => {
                            if let Some(s) = value.as_str() {
                                effect.source = Some(s.into());
                            }
                        }
                        "destination" => {
                            if let Some(s) = value.as_str() {
                                effect.destination = Some(s.into());
                            }
                        }
                        "count" => {
                            if let Some(n) = value.as_u64() {
                                effect.count = Some(n as u32);
                            }
                        }
                        "target" => {
                            if let Some(s) = value.as_str() {
                                effect.target = Some(s.into());
                            }
                        }
                        "optional" => {
                            if let Some(b) = value.as_bool() {
                                effect.optional = Some(b);
                            }
                        }
                        "max" => {
                            if let Some(b) = value.as_bool() {
                                effect.max = Some(b);
                            }
                        }
                        "options" | "costs" => {
                            if let Ok(sub) =
                                serde_json::from_value::<Option<Vec<AbilityCost>>>(value)
                            {
                                if let Some(sub) = sub {
                                    effect.compound.actions = Some(
                                        sub.into_iter()
                                            .map(|c| Box::new(AbilityCost::into_effect(c)))
                                            .collect(),
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
                if !all_fields.is_empty() {
                    if let Some(kind) = AbilityEffect::kind_from_action(
                        effect.action.to_str(),
                        &serde_json::Value::Object(all_fields),
                    ) {
                        effect.kind = Some(crate::card::ek_box_new(kind));
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
            card_type: self.card_type_any(),
            group: self
                .group_names_any()
                .as_ref()
                .and_then(|v| v.first())
                .map(|s| s.as_str()),
            cost_limit: self.cost_limit_any(),
            cost_operator: self.cost_limit_operator_any().map(Operator::as_str),
            characters: self.characters_any(),
            exclude_characters: self.exclude_characters_any(),
            exclude_self: if self.exclude_self_any().unwrap_or(false) {
                Some(-1)
            } else {
                None
            },
            exclude_group_names: self.exclude_group_names_any(),
            card_property: self.card_property_any(),
            negation: self.negation_any().unwrap_or(false),
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
    pub actions: Option<Vec<Box<AbilityEffect>>>,
    #[serde(default)]
    pub primary_effect: Option<Box<AbilityEffect>>,
    #[serde(default)]
    pub alternative_condition: Option<Box<Condition>>,
    #[serde(default)]
    pub result_condition: Option<Box<Condition>>,
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
    pub ability_filter: Option<AbilityFilter>,
    pub ability_filter_triggers: Option<Vec<String>>,
}

/// Macro to generate accessor methods on AbilityEffect that delegate to EffectKind.
/// Usage: ekf!(field_name: return_type => VariantName)
/// This expands to a method `pub fn field_name(&self) -> &return_type`
/// that checks self.kind for the given variant and returns a reference
/// or default.
#[macro_export]
macro_rules! ekf {
    ($effect:expr, $variant:path, $field:ident) => {{
        match &$effect.kind {
            Some($variant { $field, .. }) => $field,
            _ => &None,
        }
    }};
}

/// Tagged union of effect-specific fields, indexed by effect action type.
/// Each variant holds only the fields relevant to its group of actions,
/// replacing the 142-field flat AbilityEffect struct.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectKind {
    #[default]
    None,
    /// MoveCards effect fields
    MoveCards {
        #[serde(default)]
        source: Option<ArcStr>,
        #[serde(default)]
        target: Option<ArcStr>,
        #[serde(default)]
        destination: Option<ArcStr>,
        #[serde(default)]
        count: Option<u32>,
        #[serde(default)]
        card_type: Option<EffectCardType>,
        #[serde(default)]
        target_count: Option<u32>,
        #[serde(default)]
        characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        cost_limit: Option<u32>,
        #[serde(default)]
        cost_limit_operator: Option<Operator>,
        #[serde(default)]
        cost_limit_min: Option<u32>,
        #[serde(default)]
        cost_limit_max: Option<u32>,
        #[serde(default)]
        card_names: Box<Vec<String>>,
        #[serde(default)]
        placement_order: Option<PlacementOrder>,
        #[serde(default)]
        shuffle: Option<bool>,
        #[serde(default)]
        any_number: Option<bool>,
        #[serde(default)]
        discard_remaining: Option<bool>,
        #[serde(default)]
        multiple_targets: Option<bool>,
        #[serde(default)]
        exclude_self: Option<bool>,
        #[serde(default)]
        exclude_selected: Option<bool>,
        #[serde(default)]
        exclude_by_name_source: Option<ArcStr>,
        #[serde(default)]
        name_constraint: Option<ArcStr>,
        #[serde(default)]
        name_constraint_source: Option<ArcStr>,
        #[serde(default)]
        ability_filter: Option<AbilityFilter>,
        #[serde(default)]
        ability_filter_triggers: Option<Vec<String>>,
        #[serde(default)]
        or_ability_filters: Option<Vec<AbilityFilterBranch>>,
        #[serde(default)]
        card_property: Option<ArcStr>,
        #[serde(default)]
        original_value: Option<bool>,
        #[serde(default)]
        source_position: Option<ArcStr>,
        #[serde(default)]
        exclude_position: Option<ArcStr>,
        #[serde(default)]
        allow_occupied_stage: Option<bool>,
        #[serde(default)]
        target_from_selection: Option<bool>,
        #[serde(default)]
        group_reference: Option<ArcStr>,
        #[serde(default)]
        cost_from_revealed: Option<bool>,
        #[serde(default)]
        self_target: Option<bool>,
        #[serde(default)]
        per_group: Option<bool>,
        #[serde(default)]
        per_group_count: Option<u32>,
        #[serde(default)]
        state: Option<EffectState>,
        #[serde(default)]
        negation: Option<bool>,
        #[serde(default)]
        location: Option<ArcStr>,
        #[serde(default)]
        activation_position: Option<ArcStr>,
        #[serde(default)]
        exclude_heart_colors: Option<Box<Vec<String>>>,
        #[serde(default)]
        filter_targets_by_heart_colors: Option<bool>,
        #[serde(default)]
        cost_total: Option<u32>,
        #[serde(default)]
        cost_total_operator: Option<Operator>,
        #[serde(default)]
        need_heart_total: Option<u32>,
        #[serde(default)]
        need_heart_operator: Option<Operator>,
        #[serde(default)]
        need_heart_color: Option<ArcStr>,
        #[serde(default)]
        distinct: Option<DistinctType>,
        #[serde(default)]
        state_change: Option<EffectState>,
        #[serde(default)]
        self_cost: Option<bool>,
        #[serde(default)]
        dynamic_count: Option<DynamicCount>,
        #[serde(default)]
        or_card_types: Option<Box<Vec<String>>>,
        #[serde(default)]
        position: Option<PositionInfo>,
        #[serde(default)]
        cost_reference: Option<ArcStr>,
        #[serde(default)]
        cost_offset: Option<i32>,
        #[serde(default)]
        all: Option<bool>,
        #[serde(default)]
        energy_count: Option<u32>,
        #[serde(default)]
        heart_colors: Box<Vec<String>>,
        #[serde(default)]
        baton_touch_trigger: Option<bool>,
        #[serde(default)]
        target_member: Option<ArcStr>,
        #[serde(default)]
        same_unit_name: Option<bool>,
        #[serde(default)]
        action_by: Option<ArcStr>,
        #[serde(default)]
        activation_condition_parsed: Option<Box<Condition>>,
        #[serde(default)]
        quoted_text: Option<QuotedText>,
    },
    /// DrawCards effect fields
    DrawCards {
        #[serde(default)]
        source: Option<ArcStr>,
        #[serde(default)]
        target: Option<ArcStr>,
        #[serde(default)]
        destination: Option<ArcStr>,
        #[serde(default)]
        target_count: Option<u32>,
        #[serde(default)]
        count: Option<u32>,
        #[serde(default)]
        card_type: Option<EffectCardType>,
        #[serde(default)]
        dynamic_count: Option<DynamicCount>,
        #[serde(default)]
        card_names: Box<Vec<String>>,
        #[serde(default)]
        location: Option<ArcStr>,
        #[serde(default)]
        exclude_self: Option<bool>,
        #[serde(default)]
        per_unit: Option<bool>,
        #[serde(default)]
        per_unit_count: Option<u32>,
        #[serde(default)]
        per_unit_type: Option<ArcStr>,
        #[serde(default)]
        per_unit_heart_colors: Box<Vec<String>>,
        #[serde(default)]
        per_unit_location: Option<ArcStr>,
        #[serde(default)]
        position: Option<PositionInfo>,
        #[serde(default)]
        state: Option<EffectState>,
        #[serde(default)]
        heart_colors: Box<Vec<String>>,
        #[serde(default)]
        trigger_type: Option<ArcStr>,
        #[serde(default)]
        original_value: Option<bool>,
        #[serde(default)]
        action_by: Option<ArcStr>,
    },
    /// SelectTarget effect fields
    SelectTarget {
        #[serde(default)]
        source: Option<ArcStr>,
        #[serde(default)]
        target: Option<ArcStr>,
        #[serde(default)]
        destination: Option<ArcStr>,
        #[serde(default)]
        target_count: Option<u32>,
        #[serde(default)]
        card_type: Option<EffectCardType>,
        #[serde(default)]
        characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        cost_limit: Option<u32>,
        #[serde(default)]
        cost_limit_operator: Option<Operator>,
        #[serde(default)]
        cost_limit_min: Option<u32>,
        #[serde(default)]
        cost_limit_max: Option<u32>,
        #[serde(default)]
        card_names: Box<Vec<String>>,
        #[serde(default)]
        exclude_self: Option<bool>,
        #[serde(default)]
        exclude_selected: Option<bool>,
        #[serde(default)]
        placement_order: Option<PlacementOrder>,
        #[serde(default)]
        distinct: Option<DistinctType>,
        #[serde(default)]
        name_constraint: Option<ArcStr>,
        #[serde(default)]
        name_constraint_source: Option<ArcStr>,
        #[serde(default)]
        ability_filter: Option<AbilityFilter>,
        #[serde(default)]
        ability_filter_triggers: Option<Vec<String>>,
        #[serde(default)]
        or_ability_filters: Option<Vec<AbilityFilterBranch>>,
        #[serde(default)]
        card_property: Option<ArcStr>,
        #[serde(default)]
        original_value: Option<bool>,
        #[serde(default)]
        negation: Option<bool>,
        #[serde(default)]
        self_target: Option<bool>,
        #[serde(default)]
        per_unit: Option<bool>,
        #[serde(default)]
        per_unit_count: Option<u32>,
        #[serde(default)]
        per_unit_type: Option<ArcStr>,
        #[serde(default)]
        per_unit_heart_colors: Box<Vec<String>>,
        #[serde(default)]
        per_unit_location: Option<ArcStr>,
        #[serde(default)]
        optional: Option<bool>,
        #[serde(default)]
        location: Option<ArcStr>,
        #[serde(default)]
        state: Option<EffectState>,
        #[serde(default)]
        activation_position: Option<ArcStr>,
        #[serde(default)]
        group_reference: Option<ArcStr>,
        #[serde(default)]
        multiple_targets: Option<bool>,
        #[serde(default)]
        question: Option<ArcStr>,
        #[serde(default)]
        answers: Option<Box<Vec<String>>>,
        #[serde(default)]
        choice_maker: Option<ArcStr>,
        #[serde(default)]
        choice_type: Option<ArcStr>,
        #[serde(default)]
        choice_options: Option<Box<Vec<String>>>,
        #[serde(default)]
        filter_targets_by_heart_colors: Option<bool>,
        #[serde(default)]
        heart_colors: Box<Vec<String>>,
        #[serde(default)]
        cost_total: Option<u32>,
        #[serde(default)]
        cost_total_operator: Option<Operator>,
        #[serde(default)]
        or_card_types: Option<Box<Vec<String>>>,
        #[serde(default)]
        action_by: Option<ArcStr>,
        #[serde(default)]
        require_all_heart_colors: Option<bool>,
        #[serde(default)]
        heart_color_count: Option<u32>,
        #[serde(default)]
        options: Option<Vec<Box<AbilityEffect>>>,
        #[serde(default)]
        per_group: Option<bool>,
        #[serde(default)]
        per_group_count: Option<u32>,
        #[serde(default)]
        reveal: Option<bool>,
        #[serde(default)]
        any_number: Option<bool>,
        #[serde(default)]
        discard_remaining: Option<bool>,
    },
    /// LookReveal effect fields
    LookReveal {
        #[serde(default)]
        source: Option<ArcStr>,
        #[serde(default)]
        target: Option<ArcStr>,
        #[serde(default)]
        destination: Option<ArcStr>,
        #[serde(default)]
        card_type: Option<EffectCardType>,
        #[serde(default)]
        characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        cost_limit: Option<u32>,
        #[serde(default)]
        cost_limit_operator: Option<Operator>,
        #[serde(default)]
        cost_limit_min: Option<u32>,
        #[serde(default)]
        cost_limit_max: Option<u32>,
        #[serde(default)]
        card_names: Box<Vec<String>>,
        #[serde(default)]
        exclude_self: Option<bool>,
        #[serde(default)]
        distinct: Option<DistinctType>,
        #[serde(default)]
        name_constraint: Option<ArcStr>,
        #[serde(default)]
        name_constraint_source: Option<ArcStr>,
        #[serde(default)]
        ability_filter: Option<AbilityFilter>,
        #[serde(default)]
        ability_filter_triggers: Option<Vec<String>>,
        #[serde(default)]
        or_ability_filters: Option<Vec<AbilityFilterBranch>>,
        #[serde(default)]
        card_property: Option<ArcStr>,
        #[serde(default)]
        original_value: Option<bool>,
        #[serde(default)]
        negation: Option<bool>,
        #[serde(default)]
        self_target: Option<bool>,
        #[serde(default)]
        per_unit: Option<bool>,
        #[serde(default)]
        per_unit_count: Option<u32>,
        #[serde(default)]
        per_unit_type: Option<ArcStr>,
        #[serde(default)]
        per_unit_heart_colors: Box<Vec<String>>,
        #[serde(default)]
        per_unit_location: Option<ArcStr>,
        #[serde(default)]
        location: Option<ArcStr>,
        #[serde(default)]
        group_reference: Option<ArcStr>,
        #[serde(default)]
        dynamic_count: Option<DynamicCount>,
        #[serde(default)]
        heart_colors: Box<Vec<String>>,
        #[serde(default)]
        reveal: Option<bool>,
        #[serde(default)]
        filter_targets_by_heart_colors: Option<bool>,
        #[serde(default)]
        activation_position: Option<ArcStr>,
        #[serde(default)]
        state: Option<EffectState>,
        #[serde(default)]
        optional: Option<bool>,
        #[serde(default)]
        blind: Option<bool>,
        #[serde(default)]
        is_reveal: Option<bool>,
        #[serde(default)]
        picker: Option<ArcStr>,
        #[serde(default)]
        multiple_targets: Option<bool>,
        #[serde(default)]
        options: Option<Vec<Box<AbilityEffect>>>,
        #[serde(default)]
        resource_on_select: Option<Box<AbilityEffect>>,
        #[serde(default)]
        require_all_heart_colors: Option<bool>,
        #[serde(default)]
        heart_color_count: Option<u32>,
    },
    /// ModifyScore effect fields
    ModifyScore {
        #[serde(default)]
        source: Option<ArcStr>,
        #[serde(default)]
        target: Option<ArcStr>,
        #[serde(default)]
        destination: Option<ArcStr>,
        #[serde(default)]
        operation: Option<ArcStr>,
        #[serde(default)]
        value: Option<u32>,
        #[serde(default)]
        duration: Option<ArcStr>,
        #[serde(default)]
        card_type: Option<EffectCardType>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        per_unit: Option<bool>,
        #[serde(default)]
        per_unit_count: Option<u32>,
        #[serde(default)]
        per_unit_type: Option<ArcStr>,
        #[serde(default)]
        per_unit_location: Option<ArcStr>,
        #[serde(default)]
        per_unit_heart_colors: Box<Vec<String>>,
        #[serde(default)]
        location: Option<ArcStr>,
        #[serde(default)]
        effect_constraint: Option<ArcStr>,
        #[serde(default)]
        self_target: Option<bool>,
        #[serde(default)]
        heart_colors: Box<Vec<String>>,
        #[serde(default)]
        exclude_self: Option<bool>,
        #[serde(default)]
        target_count: Option<u32>,
        #[serde(default)]
        repeat_limit: Option<u32>,
        #[serde(default)]
        filter_targets_by_heart_colors: Option<bool>,
        #[serde(default)]
        cost_total: Option<u32>,
        #[serde(default)]
        cost_total_operator: Option<Operator>,
        #[serde(default)]
        distinct: Option<DistinctType>,
        #[serde(default)]
        position: Option<PositionInfo>,
        #[serde(default)]
        activation_position: Option<ArcStr>,
        #[serde(default)]
        card_names: Box<Vec<String>>,
        #[serde(default)]
        card_property: Option<ArcStr>,
        #[serde(default)]
        state: Option<EffectState>,
        #[serde(default)]
        negation: Option<bool>,
        #[serde(default)]
        max_repeats: Option<u32>,
        #[serde(default)]
        need_heart_operator: Option<Operator>,
        #[serde(default)]
        need_heart_total: Option<u32>,
    },
    /// ModifyHearts effect fields
    ModifyHearts {
        #[serde(default)]
        operation: Option<ArcStr>,
        #[serde(default)]
        value: Option<u32>,
        #[serde(default)]
        duration: Option<ArcStr>,
        #[serde(default)]
        heart_colors: Box<Vec<String>>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        per_unit: Option<bool>,
        #[serde(default)]
        per_unit_count: Option<u32>,
        #[serde(default)]
        per_unit_heart_colors: Box<Vec<String>>,
        #[serde(default)]
        location: Option<ArcStr>,
        #[serde(default)]
        timing_condition: Option<ArcStr>,
        #[serde(default)]
        original_value: Option<bool>,
        #[serde(default)]
        original_count: Option<u32>,
        #[serde(default)]
        original_operator: Option<Operator>,
        #[serde(default)]
        exclude_self: Option<bool>,
        #[serde(default)]
        self_target: Option<bool>,
        #[serde(default)]
        exclude_heart_colors: Option<Box<Vec<String>>>,
        #[serde(default)]
        repeat_limit: Option<u32>,
        #[serde(default)]
        card_type: Option<EffectCardType>,
        #[serde(default)]
        target_count: Option<u32>,
        #[serde(default)]
        filter_targets_by_heart_colors: Option<bool>,
        #[serde(default)]
        cost_total: Option<u32>,
        #[serde(default)]
        cost_total_operator: Option<Operator>,
        #[serde(default)]
        group_reference: Option<ArcStr>,
        #[serde(default)]
        negation: Option<bool>,
        #[serde(default)]
        replace_all: Option<bool>,
        #[serde(default)]
        position: Option<PositionInfo>,
        #[serde(default)]
        all: Option<bool>,
        #[serde(default)]
        per_unit_type: Option<ArcStr>,
        #[serde(default)]
        distinct: Option<DistinctType>,
    },
    /// GainResource effect fields
    GainResource {
        #[serde(default)]
        resource: Option<ArcStr>,
        #[serde(default)]
        heart_colors: Box<Vec<String>>,
        #[serde(default)]
        heart_colors_from_selected_card: Option<bool>,
        #[serde(default)]
        sign: Option<ArcStr>,
        #[serde(default)]
        operation: Option<ArcStr>,
        #[serde(default)]
        value: Option<u32>,
        #[serde(default, alias = "energy")]
        energy_count: Option<u32>,
        #[serde(default)]
        dynamic_count: Option<DynamicCount>,
        #[serde(default)]
        optional: Option<bool>,
        #[serde(default)]
        duration: Option<ArcStr>,
        #[serde(default)]
        position: Option<PositionInfo>,
        #[serde(default)]
        any_number: Option<bool>,
        #[serde(default)]
        per_unit: Option<bool>,
        #[serde(default)]
        per_unit_count: Option<u32>,
        #[serde(default)]
        per_unit_type: Option<ArcStr>,
        #[serde(default)]
        per_unit_heart_colors: Box<Vec<String>>,
        #[serde(default)]
        per_unit_location: Option<ArcStr>,
        #[serde(default)]
        location: Option<ArcStr>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        card_type: Option<EffectCardType>,
        #[serde(default)]
        cost_limit: Option<u32>,
        #[serde(default)]
        cost_limit_operator: Option<Operator>,
        #[serde(default)]
        characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        self_target: Option<bool>,
        #[serde(default)]
        target_from_selection: Option<bool>,
        #[serde(default)]
        card_property: Option<ArcStr>,
        #[serde(default)]
        original_value: Option<bool>,
        #[serde(default)]
        negation: Option<bool>,
        #[serde(default)]
        filter_targets_by_heart_colors: Option<bool>,
        #[serde(default)]
        activation_position: Option<ArcStr>,
        #[serde(default)]
        state: Option<EffectState>,
        #[serde(default)]
        heart_type: Option<ArcStr>,
        #[serde(default)]
        target_count: Option<u32>,
        #[serde(default)]
        all: Option<bool>,
        #[serde(default)]
        same_name: Option<bool>,
        #[serde(default)]
        exclude_self: Option<bool>,
        #[serde(default)]
        group_reference: Option<ArcStr>,
        #[serde(default)]
        trigger_type: Option<ArcStr>,
        #[serde(default)]
        distinct: Option<DistinctType>,
        #[serde(default)]
        heart_color: Option<ArcStr>,
        #[serde(default)]
        action_by: Option<ArcStr>,
        #[serde(default)]
        activation_condition_parsed: Option<Box<Condition>>,
        #[serde(default)]
        multiple_targets: Option<bool>,
        #[serde(default, alias = "max_repeats")]
        repeat_limit: Option<u32>,
        #[serde(default)]
        timing_condition: Option<ArcStr>,
        #[serde(default)]
        require_all_heart_colors: Option<bool>,
        #[serde(default)]
        heart_color_count: Option<u32>,
    },
    /// ChangeState effect fields
    ChangeState {
        #[serde(default)]
        source: Option<ArcStr>,
        #[serde(default)]
        target: Option<ArcStr>,
        #[serde(default)]
        destination: Option<ArcStr>,
        #[serde(default)]
        state_change: Option<EffectState>,
        #[serde(default)]
        card_type: Option<EffectCardType>,
        #[serde(default)]
        cost_limit: Option<u32>,
        #[serde(default)]
        cost_limit_operator: Option<Operator>,
        #[serde(default)]
        cost_from_revealed: Option<bool>,
        #[serde(default)]
        optional: Option<bool>,
        #[serde(default)]
        self_cost: Option<bool>,
        #[serde(default)]
        characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        blade_limit: Option<u32>,
        #[serde(default)]
        blade_limit_operator: Option<Operator>,
        #[serde(default)]
        per_unit: Option<bool>,
        #[serde(default)]
        per_unit_count: Option<u32>,
        #[serde(default)]
        per_unit_type: Option<ArcStr>,
        #[serde(default)]
        per_unit_heart_colors: Box<Vec<String>>,
        #[serde(default)]
        per_unit_location: Option<ArcStr>,
        #[serde(default)]
        location: Option<ArcStr>,
        #[serde(default)]
        distinct: Option<DistinctType>,
        #[serde(default)]
        exclude_self: Option<bool>,
        #[serde(default)]
        self_target: Option<bool>,
        #[serde(default)]
        identities: Option<Box<Vec<String>>>,
        #[serde(default)]
        all_regions: Option<bool>,
        #[serde(default)]
        card_names: Box<Vec<String>>,
        #[serde(default)]
        negation: Option<bool>,
        #[serde(default)]
        cost_total: Option<u32>,
        #[serde(default)]
        cost_total_operator: Option<Operator>,
        #[serde(default)]
        card_property: Option<ArcStr>,
        #[serde(default)]
        original_value: Option<bool>,
        #[serde(default)]
        name_constraint: Option<ArcStr>,
        #[serde(default)]
        name_constraint_source: Option<ArcStr>,
        #[serde(default)]
        filter_targets_by_heart_colors: Option<bool>,
        #[serde(default)]
        group_reference: Option<ArcStr>,
        #[serde(default)]
        activation_position: Option<ArcStr>,
        #[serde(default)]
        ability_filter: Option<AbilityFilter>,
        #[serde(default)]
        ability_filter_triggers: Option<Vec<String>>,
        #[serde(default)]
        or_ability_filters: Option<Vec<AbilityFilterBranch>>,
        #[serde(default)]
        exclude_heart_colors: Option<Box<Vec<String>>>,
        #[serde(default)]
        heart_colors: Box<Vec<String>>,
        #[serde(default)]
        all: Option<bool>,
        #[serde(default)]
        position: Option<PositionInfo>,
        #[serde(default)]
        state: Option<EffectState>,
        #[serde(default)]
        action_by: Option<ArcStr>,
        #[serde(default)]
        activation_condition_parsed: Option<Box<Condition>>,
    },
    /// AbilityOp effect fields
    AbilityOp {
        #[serde(default)]
        source: Option<ArcStr>,
        #[serde(default)]
        target: Option<ArcStr>,
        #[serde(default)]
        destination: Option<ArcStr>,
        #[serde(default)]
        ability_gain: Option<ArcStr>,
        #[serde(default)]
        ability_gain_trigger: Option<ArcStr>,
        #[serde(default)]
        gained_effect: Option<Box<AbilityEffect>>,
        #[serde(default)]
        ability_text: Option<ArcStr>,
        #[serde(default)]
        target_trigger: Option<ArcStr>,
        #[serde(default)]
        source_card: Option<ArcStr>,
        #[serde(default)]
        suppressed_trigger: Option<ArcStr>,
        #[serde(default)]
        card_type: Option<EffectCardType>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        cost_limit: Option<u32>,
        #[serde(default)]
        cost_limit_operator: Option<Operator>,
        #[serde(default)]
        location: Option<ArcStr>,
        #[serde(default)]
        trigger_filter: Option<Box<Vec<String>>>,
        #[serde(default)]
        trigger_type: Option<ArcStr>,
        #[serde(default)]
        duration: Option<ArcStr>,
        #[serde(default)]
        self_target: Option<bool>,
        #[serde(default)]
        exclude_self: Option<bool>,
        #[serde(default)]
        effect_type: Option<ArcStr>,
        #[serde(default)]
        use_limit: Option<u32>,
        #[serde(default)]
        triggers: Option<ArcStr>,
        #[serde(default)]
        activation_condition_parsed: Option<Box<Condition>>,
        #[serde(default)]
        option: Option<ArcStr>,
        #[serde(default)]
        all: Option<bool>,
        #[serde(default)]
        activation_position: Option<ArcStr>,
        #[serde(default)]
        heart_colors: Box<Vec<String>>,
        #[serde(default)]
        dynamic_count: Option<DynamicCount>,
    },
    /// CompoundEffect effect fields
    CompoundEffect {
        #[serde(default)]
        source: Option<ArcStr>,
        #[serde(default)]
        target: Option<ArcStr>,
        #[serde(default)]
        destination: Option<ArcStr>,
        #[serde(default, alias = "max_repeats")]
        repeat_limit: Option<u32>,
        #[serde(default)]
        options: Option<Vec<Box<AbilityEffect>>>,
        #[serde(default)]
        choice_type: Option<ArcStr>,
        #[serde(default)]
        choice_options: Option<Box<Vec<String>>>,
        #[serde(default)]
        question: Option<ArcStr>,
        #[serde(default)]
        answers: Option<Box<Vec<String>>>,
        #[serde(default)]
        choice_maker: Option<ArcStr>,
        #[serde(default)]
        alternative_effect: Option<Box<AbilityEffect>>,
        #[serde(default)]
        optional: Option<bool>,
        #[serde(default)]
        target_count: Option<u32>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        heart_colors: Box<Vec<String>>,
        #[serde(default)]
        exclude_self: Option<bool>,
        #[serde(default)]
        duration: Option<ArcStr>,
        #[serde(default)]
        position: Option<PositionInfo>,
        #[serde(default)]
        all: Option<bool>,
        #[serde(default)]
        activation_position: Option<ArcStr>,
        #[serde(default)]
        card_type: Option<EffectCardType>,
        #[serde(default)]
        trigger_type: Option<ArcStr>,
        #[serde(default)]
        activation_condition_parsed: Option<Box<Condition>>,
        #[serde(default)]
        original_value: Option<bool>,
        #[serde(default)]
        shuffle: Option<bool>,
        #[serde(default)]
        distinct: Option<DistinctType>,
        #[serde(default)]
        group_reference: Option<ArcStr>,
        #[serde(default)]
        per_unit: Option<bool>,
        #[serde(default)]
        per_unit_count: Option<u32>,
        #[serde(default)]
        per_unit_type: Option<ArcStr>,
        #[serde(default)]
        alternative_count_type: Option<ArcStr>,
        #[serde(default)]
        choice_condition: Option<Box<Condition>>,
        #[serde(default)]
        alternative_condition: Option<Box<Condition>>,
    },
    /// RestrictionOp effect fields
    RestrictionOp {
        #[serde(default)]
        restriction_type: Option<ArcStr>,
        #[serde(default)]
        restricted_destination: Option<ArcStr>,
        #[serde(default)]
        delayed: Option<bool>,
        #[serde(default)]
        timing: Option<ArcStr>,
        #[serde(default)]
        treat_as: Option<ArcStr>,
        #[serde(default)]
        timing_condition: Option<ArcStr>,
        #[serde(default)]
        phase: Option<ArcStr>,
        #[serde(default)]
        non_stackable: Option<bool>,
        #[serde(default)]
        operation: Option<ArcStr>,
        #[serde(default)]
        card_type: Option<EffectCardType>,
        #[serde(default)]
        location: Option<ArcStr>,
        #[serde(default)]
        effect_type: Option<ArcStr>,
        #[serde(default)]
        replaces_event: Option<ArcStr>,
        #[serde(default)]
        choice_based: Option<bool>,
        #[serde(default)]
        trigger_type: Option<ArcStr>,
        #[serde(default)]
        trigger_filter: Option<Box<Vec<String>>>,
        #[serde(default)]
        duration: Option<ArcStr>,
        #[serde(default)]
        self_target: Option<bool>,
        #[serde(default)]
        exclude_self: Option<bool>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_characters: Option<Box<Vec<String>>>,
    },
    /// PositionOp effect fields
    PositionOp {
        #[serde(default)]
        source: Option<ArcStr>,
        #[serde(default)]
        target: Option<ArcStr>,
        #[serde(default)]
        destination: Option<ArcStr>,
        #[serde(default)]
        position: Option<PositionInfo>,
        #[serde(default)]
        target_member: Option<ArcStr>,
        #[serde(default)]
        source_position: Option<ArcStr>,
        #[serde(default)]
        exclude_position: Option<ArcStr>,
        #[serde(default)]
        allow_occupied_stage: Option<bool>,
        #[serde(default)]
        optional: Option<bool>,
        #[serde(default)]
        card_type: Option<EffectCardType>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        cost_limit: Option<u32>,
        #[serde(default)]
        cost_limit_operator: Option<Operator>,
        #[serde(default)]
        energy_count: Option<u32>,
        #[serde(default)]
        dynamic_count: Option<DynamicCount>,
        #[serde(default)]
        any_number: Option<bool>,
        #[serde(default)]
        cost_from_revealed: Option<bool>,
        #[serde(default)]
        exclude_self: Option<bool>,
        #[serde(default)]
        multiple_targets: Option<bool>,
        #[serde(default)]
        self_target: Option<bool>,
        #[serde(default)]
        state: Option<EffectState>,
        #[serde(default)]
        activation_position: Option<ArcStr>,
        #[serde(default)]
        group_reference: Option<ArcStr>,
    },
    /// MiscOp effect fields
    MiscOp {
        #[serde(default)]
        source: Option<ArcStr>,
        #[serde(default)]
        target: Option<ArcStr>,
        #[serde(default)]
        destination: Option<ArcStr>,
        #[serde(default)]
        operation: Option<ArcStr>,
        #[serde(default)]
        value: Option<u32>,
        #[serde(default)]
        card_type: Option<EffectCardType>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        cost_limit: Option<u32>,
        #[serde(default)]
        cost_limit_operator: Option<Operator>,
        #[serde(default)]
        location: Option<ArcStr>,
        #[serde(default)]
        duration: Option<ArcStr>,
        #[serde(default)]
        heart_colors: Box<Vec<String>>,
        #[serde(default)]
        heart_type: Option<ArcStr>,
        #[serde(default)]
        heart_selection: Option<bool>,
        #[serde(default)]
        blade_type: Option<ArcStr>,
        #[serde(default)]
        self_target: Option<bool>,
        #[serde(default)]
        exclude_self: Option<bool>,
        #[serde(default)]
        choice: Option<bool>,
        #[serde(default)]
        lose_blade_hearts: Option<bool>,
        #[serde(default)]
        dynamic_count: Option<DynamicCount>,
        #[serde(default)]
        per_unit: Option<bool>,
        #[serde(default)]
        per_unit_count: Option<u32>,
        #[serde(default)]
        per_unit_type: Option<ArcStr>,
        #[serde(default)]
        per_unit_heart_colors: Box<Vec<String>>,
        #[serde(default)]
        per_unit_location: Option<ArcStr>,
        #[serde(default)]
        repeat_limit: Option<u32>,
        #[serde(default)]
        identities: Option<Box<Vec<String>>>,
        #[serde(default)]
        all_regions: Option<bool>,
        #[serde(default)]
        timing: Option<ArcStr>,
        #[serde(default)]
        treat_as: Option<ArcStr>,
        #[serde(default)]
        effect_constraint: Option<ArcStr>,
        #[serde(default)]
        original_value: Option<bool>,
        #[serde(default)]
        original_count: Option<u32>,
        #[serde(default)]
        original_operator: Option<Operator>,
        #[serde(default)]
        original_cost: Option<u32>,
        #[serde(default)]
        blade_limit: Option<u32>,
        #[serde(default)]
        blade_limit_operator: Option<Operator>,
        #[serde(default)]
        negation: Option<bool>,
        #[serde(default)]
        activation_position: Option<ArcStr>,
        #[serde(default)]
        target_count: Option<u32>,
        #[serde(default)]
        group_reference: Option<ArcStr>,
        #[serde(default)]
        parenthetical: Option<Vec<String>>,
        #[serde(default)]
        quoted_text: Option<QuotedText>,
        #[serde(default)]
        same_unit_name: Option<bool>,
        #[serde(default)]
        alternative_count_type: Option<ArcStr>,
        #[serde(default)]
        per_group: Option<bool>,
        #[serde(default)]
        per_group_count: Option<u32>,
        #[serde(default)]
        resource_icon_count: Option<u32>,
        #[serde(default)]
        cost_total: Option<u32>,
        #[serde(default)]
        cost_total_operator: Option<Operator>,
        #[serde(default)]
        cost_reference: Option<ArcStr>,
        #[serde(default)]
        cost_offset: Option<i32>,
        #[serde(default)]
        blind: Option<bool>,
        #[serde(default)]
        picker: Option<ArcStr>,
        #[serde(default)]
        all: Option<bool>,
        #[serde(default)]
        sign: Option<ArcStr>,
        #[serde(default)]
        heart_color_count: Option<u32>,
        #[serde(default)]
        require_all_heart_colors: Option<bool>,
        #[serde(default)]
        energy_count: Option<u32>,
        #[serde(default)]
        placement_order: Option<PlacementOrder>,
        #[serde(default)]
        ref_value: Option<ArcStr>,
        #[serde(default)]
        ref_offset: Option<i32>,
        #[serde(default)]
        id: Option<ArcStr>,
        #[serde(default)]
        card_names: Box<Vec<String>>,
        #[serde(default)]
        character_effects: Option<Box<Vec<serde_json::Value>>>,
        #[serde(default)]
        or_card_types: Option<Box<Vec<String>>>,
        #[serde(default)]
        options: Option<Vec<Box<AbilityEffect>>>,
        #[serde(default)]
        position: Option<PositionInfo>,
        #[serde(default)]
        ability_filter: Option<AbilityFilter>,
    },
    /// CustomOp effect fields
    CustomOp {
        #[serde(default)]
        action_by: Option<ArcStr>,
        #[serde(default)]
        opponent_action: Option<Box<AbilityEffect>>,
        #[serde(default)]
        effect_type: Option<ArcStr>,
        #[serde(default)]
        replaces_event: Option<ArcStr>,
        #[serde(default)]
        choice_based: Option<bool>,
        #[serde(default)]
        card_type: Option<EffectCardType>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        identities: Option<Box<Vec<String>>>,
        #[serde(default)]
        all_regions: Option<bool>,
        #[serde(default)]
        question: Option<ArcStr>,
        #[serde(default)]
        answers: Option<Box<Vec<String>>>,
        #[serde(default)]
        choice_maker: Option<ArcStr>,
        #[serde(default)]
        options: Option<Vec<Box<AbilityEffect>>>,
        #[serde(default)]
        card_property: Option<ArcStr>,
        #[serde(default)]
        location: Option<ArcStr>,
        #[serde(default)]
        duration: Option<ArcStr>,
        #[serde(default)]
        self_target: Option<bool>,
        #[serde(default)]
        exclude_self: Option<bool>,
        #[serde(default)]
        original_value: Option<bool>,
        #[serde(default)]
        timing: Option<ArcStr>,
        #[serde(default)]
        treat_as: Option<ArcStr>,
        #[serde(default)]
        trigger_type: Option<ArcStr>,
        #[serde(default)]
        trigger_filter: Option<Box<Vec<String>>>,
        #[serde(default)]
        activation_condition_parsed: Option<Box<Condition>>,
        #[serde(default)]
        use_limit: Option<u32>,
        #[serde(default)]
        triggers: Option<ArcStr>,
    },
}

/// Recursively re-populate EffectKind for a serialization-deserialized
/// AbilityEffect that lost its `kind` (because kind is #[serde(skip)]).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AbilityEffect {
    #[serde(default = "default_empty_string")]
    pub text: String,
    #[serde(default)]
    pub action: ActionType,
    #[serde(default)]
    pub source: Option<ArcStr>,
    #[serde(default)]
    pub destination: Option<ArcStr>,
    #[serde(default)]
    pub count: Option<u32>,
    #[serde(default)]
    pub target: Option<ArcStr>,
    #[serde(default)]
    pub condition: Option<Box<Condition>>,
    #[serde(flatten)]
    pub compound: CompoundBranch,
    #[serde(default)]
    pub kind: Option<EkBox>,
    pub non_stackable: Option<bool>,
    #[serde(default)]
    pub conditional: Option<bool>,
    #[serde(default)]
    pub is_further: Option<bool>,
    #[serde(default)]
    pub r#ref: Option<ArcStr>,
    #[serde(default)]
    pub optional: Option<bool>,
    #[serde(default)]
    pub max: Option<bool>,
    #[serde(default)]
    pub effect_steps: Option<Vec<Box<AbilityEffect>>>,
}

impl AbilityEffect {
    /// Build EffectKind from an action string and the matching effect JSON.
    pub(crate) fn kind_from_action(
        action: &str,
        effect_json: &serde_json::Value,
    ) -> Option<EffectKind> {
        let a = action.to_lowercase();
        let tag = match a.as_str() {
            "move_cards"
            | "discard_card"
            | "discard_until_count"
            | "place_energy_under_member"
            | "re_yell"
            | "shuffle"
            | "play_baton_touch"
            | "double_baton_touch" => "MoveCards",
            "draw" | "draw_card" | "draw_until_count" => "DrawCards",
            "select" | "select_cards" | "select_number" | "choose_target_player" => "SelectTarget",
            "look"
            | "look_at"
            | "reveal"
            | "reveal_effect"
            | "reveal_per_group"
            | "reveal_until_live_card"
            | "reveal_until_chosen_card"
            | "look_and_select" => "LookReveal",
            "modify_score" => "ModifyScore",
            "modify_required_hearts"
            | "modify_required_hearts_global"
            | "modify_required_hearts_success" => "ModifyHearts",
            "gain_resource" | "pay_energy" => "GainResource",
            "change_state" | "set_card_identity" | "set_card_identity_all_regions" => "ChangeState",
            "gain_ability"
            | "gain_ability_from_source"
            | "invalidate_ability"
            | "suppress_ability_trigger"
            | "activate_ability" => "AbilityOp",
            "sequential"
            | "choice"
            | "repeat_procedure"
            | "conditional_alternative"
            | "conditional_on_optional"
            | "conditional_on_result" => "CompoundEffect",
            "restriction"
            | "activation_restriction"
            | "modify_limit"
            | "all_blade_timing"
            | "reduce_live_card_set_limit" => "RestrictionOp",
            "position_change" | "rotation" => "PositionOp",
            "set_cost"
            | "set_cost_to_use"
            | "modify_cost"
            | "activation_cost"
            | "set_blade_type"
            | "set_blade_count"
            | "set_heart_type"
            | "specify_heart_color"
            | "choose_required_hearts"
            | "perform_yell"
            | "modify_yell_count" => "MiscOp",
            "custom" | "do_nothing" | "action_by" | "opponent_action" => "CustomOp",
            "" => "SelectTarget",
            _ => return None,
        };
        let tagged = serde_json::json!({tag: effect_json});
        serde_json::from_value(tagged).ok()
    }

    /// Populate `kind` from this effect's JSON value. Recurses into sub-effects.
    pub fn populate_from_json(&mut self, json_val: &serde_json::Value) {
        if let Some(kind) = Self::kind_from_action(self.action.to_str(), json_val) {
            self.kind = Some(ek_box_new(kind));
        }
        if let Some(ref mut sub) = self.compound.look_action {
            if let Some(sub_json) = json_val.get("look_action") {
                sub.populate_from_json(sub_json);
            }
        }
        if let Some(ref mut sub) = self.compound.select_action {
            if let Some(sub_json) = json_val.get("select_action") {
                sub.populate_from_json(sub_json);
            }
        }
        if let Some(ref mut sub) = self.compound.followup_action {
            if let Some(sub_json) = json_val.get("followup_action") {
                sub.populate_from_json(sub_json);
            }
        }
        if let Some(ref mut sub) = self.compound.primary_effect {
            if let Some(sub_json) = json_val.get("primary_effect") {
                sub.populate_from_json(sub_json);
            }
        }
        if let Some(ref mut sub) = self.compound.optional_action {
            if let Some(sub_json) = json_val.get("optional_action") {
                sub.populate_from_json(sub_json);
            }
        }
        if let Some(ref mut sub) = self.compound.conditional_action {
            if let Some(sub_json) = json_val.get("conditional_action") {
                sub.populate_from_json(sub_json);
            }
        }
        if let Some(ref mut actions) = self.compound.actions {
            if let Some(json_actions) = json_val.get("actions").and_then(|a| a.as_array()) {
                for (i, action) in actions.iter_mut().enumerate() {
                    if i < json_actions.len() {
                        action.populate_from_json(&json_actions[i]);
                    }
                }
            }
        }
        if let Some(ref mut steps) = self.effect_steps {
            if let Some(json_steps) = json_val.get("effect_steps").and_then(|a| a.as_array()) {
                for (i, step) in steps.iter_mut().enumerate() {
                    if i < json_steps.len() {
                        step.populate_from_json(&json_steps[i]);
                    }
                }
            }
        }
        if let Some(ref mut cond) = self.condition {
            if let Some(cond_json) = json_val.get("condition") {
                condition_populate_from_json(cond, cond_json);
            }
        }
        match self.kind.as_deref_mut() {
            Some(EffectKind::LookReveal {
                ref mut options,
                ref mut resource_on_select,
                ..
            }) => {
                if let Some(ref mut opts) = options {
                    if let Some(json_opts) = json_val.get("options").and_then(|a| a.as_array()) {
                        for (i, opt) in opts.iter_mut().enumerate() {
                            if i < json_opts.len() {
                                opt.populate_from_json(&json_opts[i]);
                            }
                        }
                    }
                }
                if let Some(ref mut ros) = resource_on_select {
                    if let Some(ros_json) = json_val.get("resource_on_select") {
                        ros.populate_from_json(ros_json);
                    }
                }
            }
            Some(EffectKind::CompoundEffect {
                ref mut options,
                ref mut alternative_effect,
                ..
            }) => {
                if let Some(ref mut opts) = options {
                    if let Some(json_opts) = json_val.get("options").and_then(|a| a.as_array()) {
                        for (i, opt) in opts.iter_mut().enumerate() {
                            if i < json_opts.len() {
                                opt.populate_from_json(&json_opts[i]);
                            }
                        }
                    }
                }
                if let Some(ref mut ae) = alternative_effect {
                    if let Some(ae_json) = json_val.get("alternative_effect") {
                        ae.populate_from_json(ae_json);
                    }
                }
            }
            Some(EffectKind::AbilityOp {
                ref mut gained_effect,
                ..
            }) => {
                if let Some(ref mut ge) = gained_effect {
                    if let Some(ge_json) = json_val.get("gained_effect") {
                        ge.populate_from_json(ge_json);
                    }
                }
            }
            Some(EffectKind::CustomOp {
                ref mut opponent_action,
                ..
            }) => {
                if let Some(ref mut oa) = opponent_action {
                    if let Some(oa_json) = json_val.get("opponent_action") {
                        oa.populate_from_json(oa_json);
                    }
                }
            }
            Some(EffectKind::MiscOp {
                ref mut options, ..
            }) => {
                if let Some(ref mut opts) = options {
                    if let Some(json_opts) = json_val.get("options").and_then(|a| a.as_array()) {
                        for (i, opt) in opts.iter_mut().enumerate() {
                            if i < json_opts.len() {
                                opt.populate_from_json(&json_opts[i]);
                            }
                        }
                    }
                }
            }
            Some(EffectKind::SelectTarget {
                ref mut options, ..
            }) => {
                if let Some(ref mut opts) = options {
                    if let Some(json_opts) = json_val.get("options").and_then(|a| a.as_array()) {
                        for (i, opt) in opts.iter_mut().enumerate() {
                            if i < json_opts.len() {
                                opt.populate_from_json(&json_opts[i]);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Populate EffectKind for sub-effects inside Condition variants.
pub fn condition_populate_from_json(cond: &mut Condition, cond_json: &serde_json::Value) {
    if let Condition::Choice {
        ref mut options, ..
    } = cond
    {
        if let Some(ref mut opts) = options {
            if let Some(json_opts) = cond_json.get("options").and_then(|a| a.as_array()) {
                for (i, opt) in opts.iter_mut().enumerate() {
                    if i < json_opts.len() {
                        opt.populate_from_json(&json_opts[i]);
                    }
                }
            }
        }
    }
    if let Condition::Complex { ref mut effect, .. } = cond {
        if let Some(ref mut eff) = effect {
            if let Some(eff_json) = cond_json.get("effect") {
                eff.populate_from_json(eff_json);
            }
        }
    }
    if let Condition::Compound {
        ref mut conditions,
        ref mut operator,
        ..
    } = cond
    {
        if operator.is_none()
            && cond_json.get("type").and_then(|t| t.as_str()) == Some("or_condition")
        {
            *operator = Some("or".into());
        }
        if let Some(ref mut conditions) = conditions {
            if let Some(json_conditions) = cond_json.get("conditions").and_then(|a| a.as_array()) {
                for (i, sub_cond) in conditions.iter_mut().enumerate() {
                    if i < json_conditions.len() {
                        condition_populate_from_json(sub_cond, &json_conditions[i]);
                    }
                }
            }
        }
    }
    if let Condition::Temporal {
        ref mut condition, ..
    } = cond
    {
        if let Some(ref mut sub_cond) = condition {
            if let Some(sub_cond_json) = cond_json.get("condition") {
                condition_populate_from_json(sub_cond, sub_cond_json);
            }
        }
    }
}

// Macro-generated getters for EffectKind fields
macro_rules! str_getter {
    ($name:ident, [$($variant:ident => $field:ident),+]) => {
        pub fn $name(&self) -> Option<&str> {
            match self.kind.as_deref() {
                $(Some(EffectKind::$variant { $field, .. }) => $field.as_deref(),)+
                _ => None,
            }
        }
    };
}

macro_rules! u32_getter {
    ($name:ident, [$($variant:ident => $field:ident),+]) => {
        pub fn $name(&self) -> Option<u32> {
            match self.kind.as_deref() {
                $(Some(EffectKind::$variant { $field, .. }) => *$field,)+
                _ => None,
            }
        }
    };
}

macro_rules! bool_getter {
    ($name:ident, [$($variant:ident => $field:ident),+]) => {
        pub fn $name(&self) -> Option<bool> {
            match self.kind.as_deref() {
                $(Some(EffectKind::$variant { $field, .. }) => *$field,)+
                _ => None,
            }
        }
    };
}

macro_rules! vec_ref_getter {
    ($name:ident, [$($variant:ident => $field:ident),+]) => {
        pub fn $name(&self) -> Option<&Vec<String>> {
            match self.kind.as_deref() {
                $(Some(EffectKind::$variant { $field, .. }) => $field.as_ref().map(|b| b.as_ref()),)+
                _ => None,
            }
        }
    };
}

macro_rules! vec_ref_getter_unboxed {
    ($name:ident, [$($variant:ident => $field:ident),+]) => {
        pub fn $name(&self) -> Option<&Vec<String>> {
            match self.kind.as_deref() {
                $(Some(EffectKind::$variant { $field, .. }) => $field.as_ref(),)+
                _ => None,
            }
        }
    };
}

macro_rules! box_vec_ref_getter {
    ($name:ident, [$($variant:ident => $field:ident),+]) => {
        pub fn $name(&self) -> Option<&Vec<String>> {
            match self.kind.as_deref() {
                $(Some(EffectKind::$variant { $field, .. }) => Some($field.as_ref()),)+
                _ => None,
            }
        }
    };
}

macro_rules! setter {
    ($fn:ident, $field:ident: $ty:ty => [$($variant:ident),+]) => {
        pub fn $fn(&mut self, val: Option<$ty>) {
            match self.kind.as_deref_mut() {
                $(Some(EffectKind::$variant { ref mut $field, .. }) => *$field = val,)+
                _ => {}
            }
        }
    };
}

macro_rules! box_setter {
    ($fn:ident, $field:ident: $ty:ty => [$($variant:ident),+]) => {
        pub fn $fn(&mut self, val: Option<$ty>) {
            match self.kind.as_deref_mut() {
                $(Some(EffectKind::$variant { ref mut $field, .. }) => *$field = val.map(|v| Box::new(v)),)+
                _ => {}
            }
        }
    };
}

impl AbilityEffect {
    pub fn ability_filter_any(&self) -> Option<AbilityFilter> {
        match self.kind.as_deref() {
            Some(EffectKind::MoveCards { ability_filter, .. }) => *ability_filter,
            Some(EffectKind::SelectTarget { ability_filter, .. }) => *ability_filter,
            Some(EffectKind::LookReveal { ability_filter, .. }) => *ability_filter,
            Some(EffectKind::ChangeState { ability_filter, .. }) => *ability_filter,
            Some(EffectKind::MiscOp { ability_filter, .. }) => *ability_filter,
            _ => None,
        }
    }

    // ability_filter_triggers is Option<Vec<String>> (unboxed)
    vec_ref_getter_unboxed!(ability_filter_triggers_any, [MoveCards => ability_filter_triggers, SelectTarget => ability_filter_triggers, LookReveal => ability_filter_triggers, ChangeState => ability_filter_triggers]);

    str_getter!(ability_gain_any, [AbilityOp => ability_gain]);

    str_getter!(ability_gain_trigger_any, [AbilityOp => ability_gain_trigger]);

    str_getter!(ability_text_any, [AbilityOp => ability_text]);

    str_getter!(action_by_any, [CustomOp => action_by, SelectTarget => action_by, MoveCards => action_by, DrawCards => action_by, ChangeState => action_by, GainResource => action_by]);

    pub fn activation_condition_parsed_any(&self) -> Option<&Box<Condition>> {
        match self.kind.as_deref() {
            Some(EffectKind::AbilityOp {
                activation_condition_parsed,
                ..
            }) => activation_condition_parsed.as_ref(),
            Some(EffectKind::CompoundEffect {
                activation_condition_parsed,
                ..
            }) => activation_condition_parsed.as_ref(),
            Some(EffectKind::CustomOp {
                activation_condition_parsed,
                ..
            }) => activation_condition_parsed.as_ref(),
            Some(EffectKind::MoveCards {
                activation_condition_parsed,
                ..
            }) => activation_condition_parsed.as_ref(),
            Some(EffectKind::ChangeState {
                activation_condition_parsed,
                ..
            }) => activation_condition_parsed.as_ref(),
            Some(EffectKind::GainResource {
                activation_condition_parsed,
                ..
            }) => activation_condition_parsed.as_ref(),
            _ => None,
        }
    }

    str_getter!(activation_position_any, [MoveCards => activation_position, SelectTarget => activation_position, LookReveal => activation_position, GainResource => activation_position, CompoundEffect => activation_position, ChangeState => activation_position, AbilityOp => activation_position, PositionOp => activation_position, MiscOp => activation_position]);

    bool_getter!(all_any, [MoveCards => all, GainResource => all, ChangeState => all, CompoundEffect => all, MiscOp => all, AbilityOp => all, ModifyHearts => all]);

    bool_getter!(all_regions_any, [ChangeState => all_regions, MiscOp => all_regions, CustomOp => all_regions]);

    bool_getter!(allow_occupied_stage_any, [MoveCards => allow_occupied_stage, PositionOp => allow_occupied_stage]);

    str_getter!(alternative_count_type_any, [MiscOp => alternative_count_type, CompoundEffect => alternative_count_type]);

    pub fn alternative_effect_any(&self) -> Option<&Box<AbilityEffect>> {
        match self.kind.as_deref() {
            Some(EffectKind::CompoundEffect {
                alternative_effect, ..
            }) => alternative_effect.as_ref(),
            _ => None,
        }
    }

    vec_ref_getter!(answers_any, [SelectTarget => answers, CompoundEffect => answers, CustomOp => answers]);

    bool_getter!(any_number_any, [MoveCards => any_number, GainResource => any_number, PositionOp => any_number, SelectTarget => any_number]);

    u32_getter!(blade_limit_any, [ChangeState => blade_limit, MiscOp => blade_limit]);

    pub fn blade_limit_operator_any(&self) -> Option<Operator> {
        match self.kind.as_deref() {
            Some(EffectKind::ChangeState {
                blade_limit_operator,
                ..
            }) => *blade_limit_operator,
            Some(EffectKind::MiscOp {
                blade_limit_operator,
                ..
            }) => *blade_limit_operator,
            _ => None,
        }
    }

    str_getter!(blade_type_any, [MiscOp => blade_type]);

    bool_getter!(blind_any, [MiscOp => blind, LookReveal => blind]);

    bool_getter!(is_reveal_any, [LookReveal => is_reveal]);

    box_vec_ref_getter!(card_names_any, [MoveCards => card_names, DrawCards => card_names, SelectTarget => card_names, LookReveal => card_names, ChangeState => card_names, MiscOp => card_names, ModifyScore => card_names]);

    str_getter!(card_property_any, [MoveCards => card_property, SelectTarget => card_property, LookReveal => card_property, GainResource => card_property, ChangeState => card_property, ModifyScore => card_property, CustomOp => card_property]);

    pub fn card_type_any(&self) -> Option<&str> {
        match self.kind.as_deref() {
            Some(EffectKind::MoveCards { card_type, .. }) => card_type.as_ref().map(|c| c.as_str()),
            Some(EffectKind::DrawCards { card_type, .. }) => card_type.as_ref().map(|c| c.as_str()),
            Some(EffectKind::SelectTarget { card_type, .. }) => {
                card_type.as_ref().map(|c| c.as_str())
            }
            Some(EffectKind::LookReveal { card_type, .. }) => {
                card_type.as_ref().map(|c| c.as_str())
            }
            Some(EffectKind::ModifyScore { card_type, .. }) => {
                card_type.as_ref().map(|c| c.as_str())
            }
            Some(EffectKind::ModifyHearts { card_type, .. }) => {
                card_type.as_ref().map(|c| c.as_str())
            }
            Some(EffectKind::GainResource { card_type, .. }) => {
                card_type.as_ref().map(|c| c.as_str())
            }
            Some(EffectKind::ChangeState { card_type, .. }) => {
                card_type.as_ref().map(|c| c.as_str())
            }
            Some(EffectKind::AbilityOp { card_type, .. }) => card_type.as_ref().map(|c| c.as_str()),
            Some(EffectKind::CompoundEffect { card_type, .. }) => {
                card_type.as_ref().map(|c| c.as_str())
            }
            Some(EffectKind::RestrictionOp { card_type, .. }) => {
                card_type.as_ref().map(|c| c.as_str())
            }
            Some(EffectKind::PositionOp { card_type, .. }) => {
                card_type.as_ref().map(|c| c.as_str())
            }
            Some(EffectKind::MiscOp { card_type, .. }) => card_type.as_ref().map(|c| c.as_str()),
            Some(EffectKind::CustomOp { card_type, .. }) => card_type.as_ref().map(|c| c.as_str()),
            _ => None,
        }
    }

    pub fn character_effects_any(&self) -> Option<&Vec<serde_json::Value>> {
        match self.kind.as_deref() {
            Some(EffectKind::MiscOp {
                character_effects, ..
            }) => character_effects.as_ref().map(|b| b.as_ref()),
            _ => None,
        }
    }

    vec_ref_getter!(characters_any, [MoveCards => characters, SelectTarget => characters, LookReveal => characters, GainResource => characters, ChangeState => characters, AbilityOp => characters, RestrictionOp => characters, PositionOp => characters, MiscOp => characters, CustomOp => characters]);

    bool_getter!(choice_any, [MiscOp => choice]);

    bool_getter!(choice_based_any, [RestrictionOp => choice_based, CustomOp => choice_based]);

    str_getter!(choice_maker_any, [SelectTarget => choice_maker, CompoundEffect => choice_maker, CustomOp => choice_maker]);

    vec_ref_getter!(choice_options_any, [SelectTarget => choice_options, CompoundEffect => choice_options]);

    str_getter!(choice_type_any, [SelectTarget => choice_type, CompoundEffect => choice_type]);

    bool_getter!(cost_from_revealed_any, [MoveCards => cost_from_revealed, ChangeState => cost_from_revealed, PositionOp => cost_from_revealed]);

    u32_getter!(cost_limit_any, [MoveCards => cost_limit, SelectTarget => cost_limit, LookReveal => cost_limit, GainResource => cost_limit, ChangeState => cost_limit, AbilityOp => cost_limit, PositionOp => cost_limit, MiscOp => cost_limit]);

    u32_getter!(cost_limit_max_any, [MoveCards => cost_limit_max, SelectTarget => cost_limit_max, LookReveal => cost_limit_max]);

    u32_getter!(cost_limit_min_any, [MoveCards => cost_limit_min, SelectTarget => cost_limit_min, LookReveal => cost_limit_min]);

    pub fn cost_limit_operator_any(&self) -> Option<Operator> {
        match self.kind.as_deref() {
            Some(EffectKind::MoveCards {
                cost_limit_operator,
                ..
            }) => *cost_limit_operator,
            Some(EffectKind::SelectTarget {
                cost_limit_operator,
                ..
            }) => *cost_limit_operator,
            Some(EffectKind::LookReveal {
                cost_limit_operator,
                ..
            }) => *cost_limit_operator,
            Some(EffectKind::GainResource {
                cost_limit_operator,
                ..
            }) => *cost_limit_operator,
            Some(EffectKind::ChangeState {
                cost_limit_operator,
                ..
            }) => *cost_limit_operator,
            Some(EffectKind::AbilityOp {
                cost_limit_operator,
                ..
            }) => *cost_limit_operator,
            Some(EffectKind::PositionOp {
                cost_limit_operator,
                ..
            }) => *cost_limit_operator,
            Some(EffectKind::MiscOp {
                cost_limit_operator,
                ..
            }) => *cost_limit_operator,
            _ => None,
        }
    }

    pub fn cost_offset_any(&self) -> Option<i32> {
        match self.kind.as_deref() {
            Some(EffectKind::MoveCards { cost_offset, .. }) => *cost_offset,
            Some(EffectKind::MiscOp { cost_offset, .. }) => *cost_offset,
            _ => None,
        }
    }

    str_getter!(cost_reference_any, [MoveCards => cost_reference, MiscOp => cost_reference]);

    u32_getter!(cost_total_any, [MoveCards => cost_total, SelectTarget => cost_total, ModifyScore => cost_total, ModifyHearts => cost_total, ChangeState => cost_total, MiscOp => cost_total]);

    pub fn cost_total_operator_any(&self) -> Option<Operator> {
        match self.kind.as_deref() {
            Some(EffectKind::MoveCards {
                cost_total_operator,
                ..
            }) => *cost_total_operator,
            Some(EffectKind::SelectTarget {
                cost_total_operator,
                ..
            }) => *cost_total_operator,
            Some(EffectKind::ModifyScore {
                cost_total_operator,
                ..
            }) => *cost_total_operator,
            Some(EffectKind::ModifyHearts {
                cost_total_operator,
                ..
            }) => *cost_total_operator,
            Some(EffectKind::ChangeState {
                cost_total_operator,
                ..
            }) => *cost_total_operator,
            Some(EffectKind::MiscOp {
                cost_total_operator,
                ..
            }) => *cost_total_operator,
            _ => None,
        }
    }

    bool_getter!(delayed_any, [RestrictionOp => delayed]);

    bool_getter!(discard_remaining_any, [MoveCards => discard_remaining, SelectTarget => discard_remaining]);

    pub fn distinct_any(&self) -> Option<DistinctType> {
        match self.kind.as_deref() {
            Some(EffectKind::MoveCards { distinct, .. }) => *distinct,
            Some(EffectKind::SelectTarget { distinct, .. }) => *distinct,
            Some(EffectKind::LookReveal { distinct, .. }) => *distinct,
            Some(EffectKind::ModifyScore { distinct, .. }) => *distinct,
            Some(EffectKind::GainResource { distinct, .. }) => *distinct,
            Some(EffectKind::ChangeState { distinct, .. }) => *distinct,
            Some(EffectKind::CompoundEffect { distinct, .. }) => *distinct,
            Some(EffectKind::ModifyHearts { distinct, .. }) => *distinct,
            _ => None,
        }
    }

    str_getter!(duration_any, [ModifyScore => duration, ModifyHearts => duration, GainResource => duration, AbilityOp => duration, RestrictionOp => duration, CompoundEffect => duration, MiscOp => duration, CustomOp => duration]);

    pub fn dynamic_count_any(&self) -> Option<&DynamicCount> {
        match self.kind.as_deref() {
            Some(EffectKind::DrawCards { dynamic_count, .. }) => dynamic_count.as_ref(),
            Some(EffectKind::LookReveal { dynamic_count, .. }) => dynamic_count.as_ref(),
            Some(EffectKind::GainResource { dynamic_count, .. }) => dynamic_count.as_ref(),
            Some(EffectKind::MoveCards { dynamic_count, .. }) => dynamic_count.as_ref(),
            Some(EffectKind::PositionOp { dynamic_count, .. }) => dynamic_count.as_ref(),
            Some(EffectKind::MiscOp { dynamic_count, .. }) => dynamic_count.as_ref(),
            Some(EffectKind::AbilityOp { dynamic_count, .. }) => dynamic_count.as_ref(),
            _ => None,
        }
    }

    str_getter!(effect_constraint_any, [ModifyScore => effect_constraint, MiscOp => effect_constraint]);

    str_getter!(effect_type_any, [AbilityOp => effect_type, RestrictionOp => effect_type, CustomOp => effect_type]);

    u32_getter!(energy_count_any, [GainResource => energy_count, PositionOp => energy_count, MiscOp => energy_count, MoveCards => energy_count]);

    str_getter!(exclude_by_name_source_any, [MoveCards => exclude_by_name_source]);

    vec_ref_getter!(exclude_characters_any, [MoveCards => exclude_characters, SelectTarget => exclude_characters, LookReveal => exclude_characters, GainResource => exclude_characters, ChangeState => exclude_characters, AbilityOp => exclude_characters, RestrictionOp => exclude_characters, PositionOp => exclude_characters, MiscOp => exclude_characters, CustomOp => exclude_characters]);

    vec_ref_getter!(exclude_group_names_any, [MoveCards => exclude_group_names, SelectTarget => exclude_group_names, LookReveal => exclude_group_names, GainResource => exclude_group_names, ChangeState => exclude_group_names, AbilityOp => exclude_group_names, RestrictionOp => exclude_group_names, PositionOp => exclude_group_names, MiscOp => exclude_group_names, CustomOp => exclude_group_names]);

    pub fn exclude_heart_colors_any(&self) -> &[String] {
        match self.kind.as_deref() {
            Some(EffectKind::MoveCards {
                exclude_heart_colors,
                ..
            }) => exclude_heart_colors
                .as_ref()
                .map(|b| b.as_slice())
                .unwrap_or(&[]),
            Some(EffectKind::ModifyHearts {
                exclude_heart_colors,
                ..
            }) => exclude_heart_colors
                .as_ref()
                .map(|b| b.as_slice())
                .unwrap_or(&[]),
            Some(EffectKind::ChangeState {
                exclude_heart_colors,
                ..
            }) => exclude_heart_colors
                .as_ref()
                .map(|b| b.as_slice())
                .unwrap_or(&[]),
            _ => &[],
        }
    }

    pub fn exclude_position_any(&self) -> Option<&str> {
        match self.kind.as_deref() {
            Some(EffectKind::MoveCards {
                exclude_position, ..
            }) => exclude_position.as_deref(),
            Some(EffectKind::PositionOp {
                exclude_position, ..
            }) => exclude_position.as_deref(),
            _ => None,
        }
    }

    bool_getter!(exclude_selected_any, [MoveCards => exclude_selected, SelectTarget => exclude_selected]);

    bool_getter!(exclude_self_any, [MoveCards => exclude_self, DrawCards => exclude_self, SelectTarget => exclude_self, LookReveal => exclude_self, ModifyScore => exclude_self, ModifyHearts => exclude_self, GainResource => exclude_self, ChangeState => exclude_self, AbilityOp => exclude_self, RestrictionOp => exclude_self, PositionOp => exclude_self, CompoundEffect => exclude_self, MiscOp => exclude_self, CustomOp => exclude_self]);

    bool_getter!(filter_targets_by_heart_colors_any, [MoveCards => filter_targets_by_heart_colors, SelectTarget => filter_targets_by_heart_colors, LookReveal => filter_targets_by_heart_colors, ModifyScore => filter_targets_by_heart_colors, ModifyHearts => filter_targets_by_heart_colors, GainResource => filter_targets_by_heart_colors, ChangeState => filter_targets_by_heart_colors]);

    pub fn gained_effect_any(&self) -> Option<&Box<AbilityEffect>> {
        match self.kind.as_deref() {
            Some(EffectKind::AbilityOp { gained_effect, .. }) => gained_effect.as_ref(),
            _ => None,
        }
    }

    vec_ref_getter!(group_names_any, [MoveCards => group_names, SelectTarget => group_names, LookReveal => group_names, ModifyScore => group_names, ModifyHearts => group_names, GainResource => group_names, ChangeState => group_names, AbilityOp => group_names, CompoundEffect => group_names, RestrictionOp => group_names, PositionOp => group_names, MiscOp => group_names, CustomOp => group_names]);

    str_getter!(group_reference_any, [MoveCards => group_reference, SelectTarget => group_reference, LookReveal => group_reference, ModifyHearts => group_reference, GainResource => group_reference, ChangeState => group_reference, MiscOp => group_reference, CompoundEffect => group_reference]);

    u32_getter!(heart_color_count_any, [MiscOp => heart_color_count, SelectTarget => heart_color_count, LookReveal => heart_color_count, GainResource => heart_color_count]);

    pub fn heart_colors_any(&self) -> &[String] {
        match self.kind.as_deref() {
            Some(EffectKind::DrawCards { heart_colors, .. }) => heart_colors.as_slice(),
            Some(EffectKind::SelectTarget { heart_colors, .. }) => heart_colors.as_slice(),
            Some(EffectKind::LookReveal { heart_colors, .. }) => heart_colors.as_slice(),
            Some(EffectKind::ModifyScore { heart_colors, .. }) => heart_colors.as_slice(),
            Some(EffectKind::ModifyHearts { heart_colors, .. }) => heart_colors.as_slice(),
            Some(EffectKind::GainResource { heart_colors, .. }) => heart_colors.as_slice(),
            Some(EffectKind::ChangeState { heart_colors, .. }) => heart_colors.as_slice(),
            Some(EffectKind::CompoundEffect { heart_colors, .. }) => heart_colors.as_slice(),
            Some(EffectKind::MiscOp { heart_colors, .. }) => heart_colors.as_slice(),
            Some(EffectKind::MoveCards { heart_colors, .. }) => heart_colors.as_slice(),
            Some(EffectKind::AbilityOp { heart_colors, .. }) => heart_colors.as_slice(),
            _ => &[],
        }
    }

    pub fn heart_colors_from_selected_card_any(&self) -> Option<bool> {
        match self.kind.as_deref() {
            Some(EffectKind::GainResource {
                heart_colors_from_selected_card,
                ..
            }) => *heart_colors_from_selected_card,
            _ => None,
        }
    }

    str_getter!(heart_color_any, [GainResource => heart_color]);

    bool_getter!(heart_selection_any, [MiscOp => heart_selection]);

    str_getter!(heart_type_any, [GainResource => heart_type, MiscOp => heart_type]);

    str_getter!(id_any, [MiscOp => id]);

    vec_ref_getter!(identities_any, [ChangeState => identities, MiscOp => identities, CustomOp => identities]);

    str_getter!(location_any, [MoveCards => location, DrawCards => location, SelectTarget => location, LookReveal => location, ModifyScore => location, ModifyHearts => location, GainResource => location, ChangeState => location, AbilityOp => location, RestrictionOp => location, MiscOp => location, CustomOp => location]);

    bool_getter!(lose_blade_hearts_any, [MiscOp => lose_blade_hearts]);

    bool_getter!(multiple_targets_any, [MoveCards => multiple_targets, SelectTarget => multiple_targets, PositionOp => multiple_targets, LookReveal => multiple_targets, GainResource => multiple_targets]);

    str_getter!(name_constraint_any, [MoveCards => name_constraint, SelectTarget => name_constraint, LookReveal => name_constraint, ChangeState => name_constraint]);

    str_getter!(name_constraint_source_any, [MoveCards => name_constraint_source, SelectTarget => name_constraint_source, LookReveal => name_constraint_source, ChangeState => name_constraint_source]);

    str_getter!(need_heart_color_any, [MoveCards => need_heart_color]);

    pub fn need_heart_operator_any(&self) -> Option<Operator> {
        match self.kind.as_deref() {
            Some(EffectKind::MoveCards {
                need_heart_operator,
                ..
            }) => *need_heart_operator,
            Some(EffectKind::ModifyScore {
                need_heart_operator,
                ..
            }) => *need_heart_operator,
            _ => None,
        }
    }

    u32_getter!(need_heart_total_any, [MoveCards => need_heart_total, ModifyScore => need_heart_total]);

    bool_getter!(negation_any, [MoveCards => negation, SelectTarget => negation, LookReveal => negation, ModifyHearts => negation, GainResource => negation, ChangeState => negation, MiscOp => negation, ModifyScore => negation]);

    bool_getter!(non_stackable_any, [RestrictionOp => non_stackable]);

    str_getter!(operation_any, [ModifyScore => operation, ModifyHearts => operation, GainResource => operation, RestrictionOp => operation, MiscOp => operation]);

    pub fn opponent_action_any(&self) -> Option<&Box<AbilityEffect>> {
        match self.kind.as_deref() {
            Some(EffectKind::CustomOp {
                opponent_action, ..
            }) => opponent_action.as_ref(),
            _ => None,
        }
    }

    str_getter!(option_any, [AbilityOp => option]);

    bool_getter!(optional_any, [SelectTarget => optional, LookReveal => optional, GainResource => optional, ChangeState => optional, CompoundEffect => optional, PositionOp => optional]);

    pub fn options_any(&self) -> Option<&Vec<Box<AbilityEffect>>> {
        match self.kind.as_deref() {
            Some(EffectKind::LookReveal { options, .. }) => options.as_ref(),
            Some(EffectKind::CompoundEffect { options, .. }) => options.as_ref(),
            Some(EffectKind::MiscOp { options, .. }) => options.as_ref(),
            Some(EffectKind::CustomOp { options, .. }) => options.as_ref(),
            Some(EffectKind::SelectTarget { options, .. }) => options.as_ref(),
            _ => None,
        }
    }

    pub fn or_ability_filters_any(&self) -> Option<&Vec<AbilityFilterBranch>> {
        match self.kind.as_deref() {
            Some(EffectKind::MoveCards {
                or_ability_filters, ..
            }) => or_ability_filters.as_ref(),
            Some(EffectKind::SelectTarget {
                or_ability_filters, ..
            }) => or_ability_filters.as_ref(),
            Some(EffectKind::LookReveal {
                or_ability_filters, ..
            }) => or_ability_filters.as_ref(),
            Some(EffectKind::ChangeState {
                or_ability_filters, ..
            }) => or_ability_filters.as_ref(),
            _ => None,
        }
    }

    vec_ref_getter!(or_card_types_any, [MoveCards => or_card_types, SelectTarget => or_card_types, MiscOp => or_card_types]);

    u32_getter!(original_cost_any, [MiscOp => original_cost]);

    u32_getter!(original_count_any, [ModifyHearts => original_count, MiscOp => original_count]);

    pub fn original_operator_any(&self) -> Option<Operator> {
        match self.kind.as_deref() {
            Some(EffectKind::ModifyHearts {
                original_operator, ..
            }) => *original_operator,
            Some(EffectKind::MiscOp {
                original_operator, ..
            }) => *original_operator,
            _ => None,
        }
    }

    bool_getter!(original_value_any, [MoveCards => original_value, SelectTarget => original_value, LookReveal => original_value, ModifyHearts => original_value, GainResource => original_value, ChangeState => original_value, MiscOp => original_value, CustomOp => original_value, DrawCards => original_value, CompoundEffect => original_value]);

    bool_getter!(replace_all_any, [ModifyHearts => replace_all]);

    vec_ref_getter_unboxed!(parenthetical_any, [MiscOp => parenthetical]);

    bool_getter!(per_group_any, [MoveCards => per_group, MiscOp => per_group, SelectTarget => per_group]);

    u32_getter!(per_group_count_any, [MoveCards => per_group_count, MiscOp => per_group_count, SelectTarget => per_group_count]);

    bool_getter!(per_unit_any, [SelectTarget => per_unit, LookReveal => per_unit, ModifyScore => per_unit, ModifyHearts => per_unit, GainResource => per_unit, ChangeState => per_unit, DrawCards => per_unit, MiscOp => per_unit, CompoundEffect => per_unit]);

    u32_getter!(per_unit_count_any, [SelectTarget => per_unit_count, DrawCards => per_unit_count, LookReveal => per_unit_count, ModifyScore => per_unit_count, ModifyHearts => per_unit_count, GainResource => per_unit_count, ChangeState => per_unit_count, MiscOp => per_unit_count, CompoundEffect => per_unit_count]);

    pub fn per_unit_heart_colors_any(&self) -> &[String] {
        match self.kind.as_deref() {
            Some(EffectKind::SelectTarget {
                per_unit_heart_colors,
                ..
            }) => per_unit_heart_colors.as_slice(),
            Some(EffectKind::LookReveal {
                per_unit_heart_colors,
                ..
            }) => per_unit_heart_colors.as_slice(),
            Some(EffectKind::ModifyScore {
                per_unit_heart_colors,
                ..
            }) => per_unit_heart_colors.as_slice(),
            Some(EffectKind::ModifyHearts {
                per_unit_heart_colors,
                ..
            }) => per_unit_heart_colors.as_slice(),
            Some(EffectKind::GainResource {
                per_unit_heart_colors,
                ..
            }) => per_unit_heart_colors.as_slice(),
            Some(EffectKind::DrawCards {
                per_unit_heart_colors,
                ..
            }) => per_unit_heart_colors.as_slice(),
            Some(EffectKind::ChangeState {
                per_unit_heart_colors,
                ..
            }) => per_unit_heart_colors.as_slice(),
            Some(EffectKind::MiscOp {
                per_unit_heart_colors,
                ..
            }) => per_unit_heart_colors.as_slice(),
            _ => &[],
        }
    }

    pub fn per_unit_location_any(&self) -> Option<&str> {
        match self.kind.as_deref() {
            Some(EffectKind::SelectTarget {
                per_unit_location, ..
            }) => per_unit_location.as_deref(),
            Some(EffectKind::LookReveal {
                per_unit_location, ..
            }) => per_unit_location.as_deref(),
            Some(EffectKind::ModifyScore {
                per_unit_location, ..
            }) => per_unit_location.as_deref(),
            Some(EffectKind::GainResource {
                per_unit_location, ..
            }) => per_unit_location.as_deref(),
            Some(EffectKind::DrawCards {
                per_unit_location, ..
            }) => per_unit_location.as_deref(),
            Some(EffectKind::ChangeState {
                per_unit_location, ..
            }) => per_unit_location.as_deref(),
            Some(EffectKind::MiscOp {
                per_unit_location, ..
            }) => per_unit_location.as_deref(),
            _ => None,
        }
    }

    str_getter!(per_unit_type_any, [SelectTarget => per_unit_type, LookReveal => per_unit_type, ModifyScore => per_unit_type, GainResource => per_unit_type, DrawCards => per_unit_type, ChangeState => per_unit_type, MiscOp => per_unit_type, CompoundEffect => per_unit_type, ModifyHearts => per_unit_type]);

    str_getter!(phase_any, [RestrictionOp => phase]);

    str_getter!(picker_any, [MiscOp => picker, LookReveal => picker]);

    pub fn placement_order_any(&self) -> Option<PlacementOrder> {
        match self.kind.as_deref() {
            Some(EffectKind::MoveCards {
                placement_order, ..
            }) => *placement_order,
            Some(EffectKind::SelectTarget {
                placement_order, ..
            }) => *placement_order,
            Some(EffectKind::MiscOp {
                placement_order, ..
            }) => *placement_order,
            _ => None,
        }
    }

    pub fn position_any(&self) -> Option<&PositionInfo> {
        match self.kind.as_deref() {
            Some(EffectKind::MoveCards { position, .. }) => position.as_ref(),
            Some(EffectKind::DrawCards { position, .. }) => position.as_ref(),
            Some(EffectKind::GainResource { position, .. }) => position.as_ref(),
            Some(EffectKind::ModifyScore { position, .. }) => position.as_ref(),
            Some(EffectKind::ChangeState { position, .. }) => position.as_ref(),
            Some(EffectKind::CompoundEffect { position, .. }) => position.as_ref(),
            Some(EffectKind::PositionOp { position, .. }) => position.as_ref(),
            Some(EffectKind::ModifyHearts { position, .. }) => position.as_ref(),
            Some(EffectKind::MiscOp { position, .. }) => position.as_ref(),
            _ => None,
        }
    }

    str_getter!(question_any, [SelectTarget => question, CompoundEffect => question, CustomOp => question]);

    pub fn quoted_text_any(&self) -> Option<&QuotedText> {
        match self.kind.as_deref() {
            Some(EffectKind::MiscOp { quoted_text, .. }) => quoted_text.as_ref(),
            _ => None,
        }
    }

    pub fn ref_offset_any(&self) -> Option<i32> {
        match self.kind.as_deref() {
            Some(EffectKind::MiscOp { ref_offset, .. }) => *ref_offset,
            _ => None,
        }
    }

    str_getter!(ref_value_any, [MiscOp => ref_value]);

    pub fn repeat_limit_any(&self) -> Option<u32> {
        match self.kind.as_deref() {
            Some(EffectKind::ModifyScore {
                repeat_limit,
                max_repeats,
                ..
            }) => repeat_limit.or(*max_repeats),
            Some(EffectKind::ModifyHearts { repeat_limit, .. }) => *repeat_limit,
            Some(EffectKind::CompoundEffect { repeat_limit, .. }) => *repeat_limit,
            Some(EffectKind::MiscOp { repeat_limit, .. }) => *repeat_limit,
            Some(EffectKind::GainResource { repeat_limit, .. }) => *repeat_limit,
            _ => None,
        }
    }

    str_getter!(replaces_event_any, [RestrictionOp => replaces_event, CustomOp => replaces_event]);

    bool_getter!(require_all_heart_colors_any, [MiscOp => require_all_heart_colors, SelectTarget => require_all_heart_colors, LookReveal => require_all_heart_colors, GainResource => require_all_heart_colors]);

    str_getter!(resource_any, [GainResource => resource]);

    u32_getter!(resource_icon_count_any, [MiscOp => resource_icon_count]);

    pub fn resource_on_select_any(&self) -> Option<&Box<AbilityEffect>> {
        match self.kind.as_deref() {
            Some(EffectKind::LookReveal {
                resource_on_select, ..
            }) => resource_on_select.as_ref(),
            _ => None,
        }
    }

    str_getter!(restricted_destination_any, [RestrictionOp => restricted_destination]);

    str_getter!(restriction_type_any, [RestrictionOp => restriction_type]);

    bool_getter!(reveal_any, [LookReveal => reveal, SelectTarget => reveal]);

    bool_getter!(same_unit_name_any, [MiscOp => same_unit_name, MoveCards => same_unit_name]);

    bool_getter!(self_cost_any, [ChangeState => self_cost, MoveCards => self_cost]);

    bool_getter!(self_target_any, [MoveCards => self_target, SelectTarget => self_target, LookReveal => self_target, ModifyScore => self_target, ModifyHearts => self_target, GainResource => self_target, ChangeState => self_target, AbilityOp => self_target, RestrictionOp => self_target, PositionOp => self_target, MiscOp => self_target, CustomOp => self_target]);

    bool_getter!(shuffle_any, [MoveCards => shuffle, CompoundEffect => shuffle]);

    str_getter!(sign_any, [GainResource => sign, MiscOp => sign]);

    str_getter!(source_card_any, [AbilityOp => source_card]);

    str_getter!(source_position_any, [MoveCards => source_position, PositionOp => source_position]);

    str_getter!(source_any, [MoveCards => source, DrawCards => source, SelectTarget => source, LookReveal => source, ChangeState => source, PositionOp => source, ModifyScore => source, CompoundEffect => source, AbilityOp => source, MiscOp => source]);

    str_getter!(destination_any, [MoveCards => destination, DrawCards => destination, SelectTarget => destination, LookReveal => destination, ChangeState => destination, PositionOp => destination, ModifyScore => destination, CompoundEffect => destination, AbilityOp => destination, MiscOp => destination]);

    pub fn count_any(&self) -> Option<u32> {
        let variant_count = match self.kind.as_deref() {
            Some(EffectKind::MoveCards { count, .. }) => *count,
            Some(EffectKind::DrawCards { count, .. }) => *count,
            _ => None,
        };
        variant_count.or(self.count)
    }

    pub fn target_any(&self) -> Option<&str> {
        let variant_target = match self.kind.as_deref() {
            Some(EffectKind::MoveCards { target, .. }) => target.as_deref(),
            Some(EffectKind::DrawCards { target, .. }) => target.as_deref(),
            Some(EffectKind::SelectTarget { target, .. }) => target.as_deref(),
            Some(EffectKind::LookReveal { target, .. }) => target.as_deref(),
            Some(EffectKind::ChangeState { target, .. }) => target.as_deref(),
            Some(EffectKind::PositionOp { target, .. }) => target.as_deref(),
            Some(EffectKind::ModifyScore { target, .. }) => target.as_deref(),
            Some(EffectKind::CompoundEffect { target, .. }) => target.as_deref(),
            Some(EffectKind::AbilityOp { target, .. }) => target.as_deref(),
            Some(EffectKind::MiscOp { target, .. }) => target.as_deref(),
            _ => None,
        };
        variant_target.or_else(|| self.target.as_deref())
    }

    pub fn state_any(&self) -> Option<&str> {
        match self.kind.as_deref() {
            Some(EffectKind::MoveCards { state, .. }) => state.as_ref().map(|s| s.as_str()),
            Some(EffectKind::DrawCards { state, .. }) => state.as_ref().map(|s| s.as_str()),
            Some(EffectKind::LookReveal { state, .. }) => state.as_ref().map(|s| s.as_str()),
            Some(EffectKind::GainResource { state, .. }) => state.as_ref().map(|s| s.as_str()),
            Some(EffectKind::PositionOp { state, .. }) => state.as_ref().map(|s| s.as_str()),
            Some(EffectKind::ChangeState { state, .. }) => state.as_ref().map(|s| s.as_str()),
            Some(EffectKind::ModifyScore { state, .. }) => state.as_ref().map(|s| s.as_str()),
            _ => None,
        }
    }

    pub fn state_change_any(&self) -> Option<&str> {
        match self.kind.as_deref() {
            Some(EffectKind::ChangeState { state_change, .. }) => {
                state_change.as_ref().map(|s| s.as_str())
            }
            Some(EffectKind::MoveCards { state_change, .. }) => {
                state_change.as_ref().map(|s| s.as_str())
            }
            _ => None,
        }
    }

    str_getter!(suppressed_trigger_any, [AbilityOp => suppressed_trigger]);

    u32_getter!(target_count_any, [MoveCards => target_count, DrawCards => target_count, SelectTarget => target_count, ModifyScore => target_count, ModifyHearts => target_count, GainResource => target_count, CompoundEffect => target_count, MiscOp => target_count]);

    bool_getter!(target_from_selection_any, [MoveCards => target_from_selection, GainResource => target_from_selection]);

    str_getter!(target_member_any, [PositionOp => target_member, MoveCards => target_member]);

    str_getter!(target_trigger_any, [AbilityOp => target_trigger]);

    str_getter!(timing_any, [RestrictionOp => timing, MiscOp => timing, CustomOp => timing]);

    str_getter!(timing_condition_any, [ModifyHearts => timing_condition, RestrictionOp => timing_condition, GainResource => timing_condition]);

    str_getter!(treat_as_any, [RestrictionOp => treat_as, MiscOp => treat_as, CustomOp => treat_as]);

    vec_ref_getter!(trigger_filter_any, [AbilityOp => trigger_filter, RestrictionOp => trigger_filter, CustomOp => trigger_filter]);

    str_getter!(trigger_type_any, [DrawCards => trigger_type, GainResource => trigger_type, AbilityOp => trigger_type, CompoundEffect => trigger_type, RestrictionOp => trigger_type, CustomOp => trigger_type]);

    str_getter!(triggers_any, [AbilityOp => triggers, CustomOp => triggers]);

    u32_getter!(use_limit_any, [AbilityOp => use_limit, CustomOp => use_limit]);

    u32_getter!(value_any, [ModifyScore => value, ModifyHearts => value, GainResource => value, MiscOp => value]);
}

impl AbilityEffect {
    setter!(set_ability_filter, ability_filter: AbilityFilter => [MoveCards, SelectTarget, LookReveal, ChangeState]);
    setter!(set_ability_filter_triggers, ability_filter_triggers: Vec<String> => [MoveCards, SelectTarget, LookReveal, ChangeState]);
    setter!(set_ability_gain, ability_gain: ArcStr => [AbilityOp]);
    setter!(set_ability_gain_trigger, ability_gain_trigger: ArcStr => [AbilityOp]);
    setter!(set_ability_text, ability_text: ArcStr => [AbilityOp]);
    setter!(set_action_by, action_by: ArcStr => [CustomOp]);
    setter!(set_activation_position, activation_position: ArcStr => [MoveCards, SelectTarget, LookReveal, GainResource, ChangeState, PositionOp, MiscOp]);
    setter!(set_all, all: bool => [MiscOp]);
    setter!(set_all_regions, all_regions: bool => [ChangeState, MiscOp, CustomOp]);
    setter!(set_allow_occupied_stage, allow_occupied_stage: bool => [MoveCards, PositionOp]);
    setter!(set_alternative_count_type, alternative_count_type: ArcStr => [MiscOp]);
    box_setter!(set_answers, answers: Vec<String> => [SelectTarget, CompoundEffect, CustomOp]);
    setter!(set_any_number, any_number: bool => [MoveCards, GainResource, PositionOp, SelectTarget]);
    setter!(set_blade_limit, blade_limit: u32 => [ChangeState, MiscOp]);
    setter!(set_blade_limit_operator, blade_limit_operator: Operator => [ChangeState, MiscOp]);
    setter!(set_blade_type, blade_type: ArcStr => [MiscOp]);
    setter!(set_blind, blind: bool => [MiscOp]);
    pub fn set_card_names(&mut self, val: Vec<String>) {
        match self.kind.as_deref_mut() {
            Some(EffectKind::MoveCards {
                ref mut card_names, ..
            }) => *card_names = Box::new(val.clone()),
            Some(EffectKind::DrawCards {
                ref mut card_names, ..
            }) => *card_names = Box::new(val.clone()),
            Some(EffectKind::SelectTarget {
                ref mut card_names, ..
            }) => *card_names = Box::new(val.clone()),
            Some(EffectKind::LookReveal {
                ref mut card_names, ..
            }) => *card_names = Box::new(val.clone()),
            Some(EffectKind::ChangeState {
                ref mut card_names, ..
            }) => *card_names = Box::new(val.clone()),
            Some(EffectKind::MiscOp {
                ref mut card_names, ..
            }) => *card_names = Box::new(val),
            _ => {}
        }
    }
    setter!(set_card_property, card_property: ArcStr => [MoveCards, SelectTarget, LookReveal, GainResource, ChangeState]);
    pub fn set_card_type(&mut self, val: Option<ArcStr>) {
        let parsed = val.map(|s| EffectCardType::from_str(&s));
        match self.kind.as_deref_mut() {
            Some(EffectKind::MoveCards {
                ref mut card_type, ..
            }) => *card_type = parsed,
            Some(EffectKind::DrawCards {
                ref mut card_type, ..
            }) => *card_type = parsed,
            Some(EffectKind::SelectTarget {
                ref mut card_type, ..
            }) => *card_type = parsed,
            Some(EffectKind::LookReveal {
                ref mut card_type, ..
            }) => *card_type = parsed,
            Some(EffectKind::ModifyScore {
                ref mut card_type, ..
            }) => *card_type = parsed,
            Some(EffectKind::ModifyHearts {
                ref mut card_type, ..
            }) => *card_type = parsed,
            Some(EffectKind::GainResource {
                ref mut card_type, ..
            }) => *card_type = parsed,
            Some(EffectKind::ChangeState {
                ref mut card_type, ..
            }) => *card_type = parsed,
            Some(EffectKind::AbilityOp {
                ref mut card_type, ..
            }) => *card_type = parsed,
            Some(EffectKind::CompoundEffect {
                ref mut card_type, ..
            }) => *card_type = parsed,
            Some(EffectKind::RestrictionOp {
                ref mut card_type, ..
            }) => *card_type = parsed,
            Some(EffectKind::PositionOp {
                ref mut card_type, ..
            }) => *card_type = parsed,
            Some(EffectKind::MiscOp {
                ref mut card_type, ..
            }) => *card_type = parsed,
            Some(EffectKind::CustomOp {
                ref mut card_type, ..
            }) => *card_type = parsed,
            _ => {}
        }
    }
    box_setter!(set_characters, characters: Vec<String> => [MoveCards, SelectTarget, LookReveal, GainResource, ChangeState, AbilityOp, RestrictionOp, PositionOp, MiscOp, CustomOp]);
    setter!(set_choice, choice: bool => [MiscOp]);
    setter!(set_choice_based, choice_based: bool => [RestrictionOp, CustomOp]);
    setter!(set_choice_maker, choice_maker: ArcStr => [SelectTarget, CompoundEffect, CustomOp]);
    box_setter!(set_choice_options, choice_options: Vec<String> => [SelectTarget, CompoundEffect]);
    setter!(set_choice_type, choice_type: ArcStr => [SelectTarget, CompoundEffect]);
    setter!(set_cost_from_revealed, cost_from_revealed: bool => [MoveCards, ChangeState, PositionOp]);
    setter!(set_cost_limit, cost_limit: u32 => [MoveCards, SelectTarget, LookReveal, GainResource, ChangeState, AbilityOp, PositionOp, MiscOp]);
    setter!(set_cost_limit_max, cost_limit_max: u32 => [MoveCards, SelectTarget, LookReveal]);
    setter!(set_cost_limit_min, cost_limit_min: u32 => [MoveCards, SelectTarget, LookReveal]);
    setter!(set_cost_limit_operator, cost_limit_operator: Operator => [MoveCards, SelectTarget, LookReveal, GainResource, ChangeState, AbilityOp, PositionOp, MiscOp]);
    setter!(set_cost_offset, cost_offset: i32 => [MiscOp]);
    setter!(set_cost_reference, cost_reference: ArcStr => [MiscOp]);
    setter!(set_cost_total, cost_total: u32 => [MoveCards, SelectTarget, ModifyScore, ModifyHearts, ChangeState, MiscOp]);
    setter!(set_cost_total_operator, cost_total_operator: Operator => [MoveCards, SelectTarget, ModifyScore, ModifyHearts, ChangeState, MiscOp]);
    setter!(set_delayed, delayed: bool => [RestrictionOp]);
    setter!(set_discard_remaining, discard_remaining: bool => [MoveCards]);
    setter!(set_distinct, distinct: DistinctType => [MoveCards, SelectTarget, LookReveal, ModifyScore, ChangeState]);
    setter!(set_duration, duration: ArcStr => [ModifyScore, ModifyHearts, GainResource, AbilityOp, RestrictionOp, MiscOp, CustomOp]);
    setter!(set_effect_constraint, effect_constraint: ArcStr => [ModifyScore, MiscOp]);
    setter!(set_effect_type, effect_type: ArcStr => [AbilityOp, RestrictionOp, CustomOp]);
    setter!(set_energy_count, energy_count: u32 => [GainResource, PositionOp, MiscOp]);
    setter!(set_exclude_by_name_source, exclude_by_name_source: ArcStr => [MoveCards]);
    box_setter!(set_exclude_characters, exclude_characters: Vec<String> => [MoveCards, SelectTarget, LookReveal, GainResource, ChangeState, AbilityOp, RestrictionOp, PositionOp, MiscOp, CustomOp]);
    box_setter!(set_exclude_group_names, exclude_group_names: Vec<String> => [MoveCards, SelectTarget, LookReveal, GainResource, ChangeState, AbilityOp, RestrictionOp, PositionOp, MiscOp, CustomOp]);
    box_setter!(set_exclude_heart_colors, exclude_heart_colors: Vec<String> => [MoveCards, ModifyHearts, ChangeState]);
    setter!(set_exclude_position, exclude_position: ArcStr => [MoveCards, PositionOp]);
    setter!(set_exclude_selected, exclude_selected: bool => [MoveCards, SelectTarget]);
    setter!(set_exclude_self, exclude_self: bool => [MoveCards, SelectTarget, LookReveal, ModifyScore, ModifyHearts, ChangeState, AbilityOp, RestrictionOp, PositionOp, MiscOp, CustomOp]);
    setter!(set_filter_targets_by_heart_colors, filter_targets_by_heart_colors: bool => [MoveCards, SelectTarget, LookReveal, ModifyScore, ModifyHearts, GainResource, ChangeState]);
    box_setter!(set_group_names, group_names: Vec<String> => [MoveCards, SelectTarget, LookReveal, ModifyScore, ModifyHearts, GainResource, ChangeState, AbilityOp, RestrictionOp, PositionOp, MiscOp, CustomOp]);
    setter!(set_group_reference, group_reference: ArcStr => [MoveCards, SelectTarget, LookReveal, ModifyHearts, ChangeState, PositionOp, MiscOp]);
    setter!(set_heart_color_count, heart_color_count: u32 => [MiscOp]);
    setter!(set_heart_color, heart_color: ArcStr => [GainResource]);
    setter!(set_heart_colors_from_selected_card, heart_colors_from_selected_card: bool => [GainResource]);
    setter!(set_heart_selection, heart_selection: bool => [MiscOp]);
    setter!(set_heart_type, heart_type: ArcStr => [MiscOp]);
    setter!(set_id, id: ArcStr => [MiscOp]);
    box_setter!(set_identities, identities: Vec<String> => [ChangeState, MiscOp, CustomOp]);
    setter!(set_location, location: ArcStr => [MoveCards, DrawCards, SelectTarget, LookReveal, ModifyScore, ModifyHearts, GainResource, ChangeState, AbilityOp, RestrictionOp, MiscOp, CustomOp]);
    setter!(set_lose_blade_hearts, lose_blade_hearts: bool => [MiscOp]);
    setter!(set_multiple_targets, multiple_targets: bool => [MoveCards, SelectTarget, PositionOp]);
    setter!(set_name_constraint, name_constraint: ArcStr => [MoveCards, SelectTarget, LookReveal, ChangeState]);
    setter!(set_name_constraint_source, name_constraint_source: ArcStr => [MoveCards, SelectTarget, LookReveal, ChangeState]);
    setter!(set_need_heart_color, need_heart_color: ArcStr => [MoveCards]);
    setter!(set_need_heart_operator, need_heart_operator: Operator => [MoveCards]);
    setter!(set_need_heart_total, need_heart_total: u32 => [MoveCards]);
    setter!(set_negation, negation: bool => [MoveCards, SelectTarget, LookReveal, ModifyHearts, GainResource, ChangeState, MiscOp]);
    setter!(set_non_stackable, non_stackable: bool => [RestrictionOp]);
    setter!(set_operation, operation: ArcStr => [ModifyScore, ModifyHearts, GainResource, RestrictionOp, MiscOp]);
    setter!(set_option, option: ArcStr => [AbilityOp]);
    pub fn set_optional(&mut self, val: Option<bool>) {
        self.optional = val;
        match self.kind.as_deref_mut() {
            Some(EffectKind::SelectTarget {
                ref mut optional, ..
            }) => *optional = val,
            Some(EffectKind::LookReveal {
                ref mut optional, ..
            }) => *optional = val,
            Some(EffectKind::GainResource {
                ref mut optional, ..
            }) => *optional = val,
            Some(EffectKind::ChangeState {
                ref mut optional, ..
            }) => *optional = val,
            Some(EffectKind::CompoundEffect {
                ref mut optional, ..
            }) => *optional = val,
            Some(EffectKind::PositionOp {
                ref mut optional, ..
            }) => *optional = val,
            _ => {}
        }
    }
    box_setter!(set_or_card_types, or_card_types: Vec<String> => [MoveCards, SelectTarget, MiscOp]);
    setter!(set_original_cost, original_cost: u32 => [MiscOp]);
    setter!(set_original_count, original_count: u32 => [ModifyHearts, MiscOp]);
    setter!(set_original_operator, original_operator: Operator => [ModifyHearts, MiscOp]);
    setter!(set_original_value, original_value: bool => [MoveCards, SelectTarget, LookReveal, ModifyHearts, GainResource, ChangeState, MiscOp, CustomOp]);
    setter!(set_parenthetical, parenthetical: Vec<String> => [MiscOp]);
    setter!(set_per_group, per_group: bool => [MoveCards, MiscOp]);
    setter!(set_per_group_count, per_group_count: u32 => [MoveCards, MiscOp]);
    setter!(set_per_unit, per_unit: bool => [SelectTarget, LookReveal, ModifyScore, ModifyHearts, GainResource, ChangeState, MiscOp]);
    setter!(set_per_unit_count, per_unit_count: u32 => [SelectTarget, LookReveal, ModifyScore, ModifyHearts, GainResource, ChangeState, MiscOp]);
    setter!(set_per_unit_location, per_unit_location: ArcStr => [SelectTarget, LookReveal, ModifyScore, GainResource, ChangeState, MiscOp]);
    setter!(set_per_unit_type, per_unit_type: ArcStr => [SelectTarget, LookReveal, ModifyScore, GainResource, ChangeState, MiscOp]);
    setter!(set_phase, phase: ArcStr => [RestrictionOp]);
    setter!(set_picker, picker: ArcStr => [MiscOp]);
    setter!(set_placement_order, placement_order: PlacementOrder => [MoveCards, SelectTarget, MiscOp]);
    setter!(set_question, question: ArcStr => [SelectTarget, CompoundEffect, CustomOp]);
    setter!(set_ref_offset, ref_offset: i32 => [MiscOp]);
    setter!(set_ref_value, ref_value: ArcStr => [MiscOp]);
    setter!(set_repeat_limit, repeat_limit: u32 => [ModifyScore, ModifyHearts, CompoundEffect, MiscOp]);
    setter!(set_replaces_event, replaces_event: ArcStr => [RestrictionOp, CustomOp]);
    setter!(set_replace_all, replace_all: bool => [ModifyHearts]);
    setter!(set_require_all_heart_colors, require_all_heart_colors: bool => [MiscOp]);
    setter!(set_resource, resource: ArcStr => [GainResource]);
    setter!(set_resource_icon_count, resource_icon_count: u32 => [MiscOp]);
    setter!(set_restricted_destination, restricted_destination: ArcStr => [RestrictionOp]);
    setter!(set_restriction_type, restriction_type: ArcStr => [RestrictionOp]);
    setter!(set_reveal, reveal: bool => [LookReveal, SelectTarget]);
    setter!(set_same_unit_name, same_unit_name: bool => [MiscOp]);
    setter!(set_self_cost, self_cost: bool => [ChangeState]);
    setter!(set_self_target, self_target: bool => [MoveCards, SelectTarget, LookReveal, ModifyScore, ModifyHearts, GainResource, ChangeState, AbilityOp, RestrictionOp, PositionOp, MiscOp, CustomOp]);
    setter!(set_shuffle, shuffle: bool => [MoveCards]);
    setter!(set_sign, sign: ArcStr => [GainResource, MiscOp]);
    setter!(set_source_card, source_card: ArcStr => [AbilityOp]);
    setter!(set_source_position, source_position: ArcStr => [MoveCards, PositionOp]);
    pub fn set_state(&mut self, val: Option<ArcStr>) {
        let parsed = val.map(|s| EffectState::from_str(&s));
        match self.kind.as_deref_mut() {
            Some(EffectKind::MoveCards { ref mut state, .. }) => *state = parsed,
            Some(EffectKind::LookReveal { ref mut state, .. }) => *state = parsed,
            Some(EffectKind::GainResource { ref mut state, .. }) => *state = parsed,
            Some(EffectKind::PositionOp { ref mut state, .. }) => *state = parsed,
            _ => {}
        }
    }

    pub fn set_state_change(&mut self, val: Option<ArcStr>) {
        let parsed = val.map(|s| EffectState::from_str(&s));
        match self.kind.as_deref_mut() {
            Some(EffectKind::ChangeState {
                ref mut state_change,
                ..
            }) => *state_change = parsed,
            _ => {}
        }
    }
    setter!(set_suppressed_trigger, suppressed_trigger: ArcStr => [AbilityOp]);
    setter!(set_target_count, target_count: u32 => [MoveCards, DrawCards, SelectTarget, ModifyScore, ModifyHearts, CompoundEffect, MiscOp]);
    setter!(set_target_from_selection, target_from_selection: bool => [MoveCards, GainResource]);
    setter!(set_target_member, target_member: ArcStr => [PositionOp]);
    setter!(set_target_trigger, target_trigger: ArcStr => [AbilityOp]);
    setter!(set_timing, timing: ArcStr => [RestrictionOp, MiscOp, CustomOp]);
    setter!(set_timing_condition, timing_condition: ArcStr => [ModifyHearts, RestrictionOp]);
    setter!(set_treat_as, treat_as: ArcStr => [RestrictionOp, MiscOp, CustomOp]);
    box_setter!(set_trigger_filter, trigger_filter: Vec<String> => [AbilityOp, RestrictionOp, CustomOp]);
    setter!(set_trigger_type, trigger_type: ArcStr => [AbilityOp, RestrictionOp, CustomOp]);
    setter!(set_triggers, triggers: ArcStr => [AbilityOp, CustomOp]);
    setter!(set_use_limit, use_limit: u32 => [AbilityOp, CustomOp]);
    setter!(set_value, value: u32 => [ModifyScore, ModifyHearts, GainResource, MiscOp]);
}

impl AbilityEffect {
    pub fn target_name(&self) -> &str {
        self.target.as_deref().unwrap_or("self")
    }

    /// Returns the source zone string with a static default.
    pub fn source_or(&self, default: &'static str) -> &str {
        self.source_any().unwrap_or(default)
    }

    /// Build a `CardFilter` containing the 7 base filter fields (card_type,
    /// group, cost_limit, cost_operator, characters, exclude_characters,
    /// exclude_self) that effect handlers most commonly need.
    pub fn filter_subset(&self) -> crate::ability::util::CardFilter<'_> {
        crate::ability::util::CardFilter {
            card_type: self.card_type_any(),
            group: self.group_name(),
            cost_limit: self.cost_limit_any(),
            cost_operator: self.cost_limit_operator_any().map(Operator::as_str),
            characters: self.characters_any(),
            exclude_characters: self.exclude_characters_any(),
            exclude_self: if self.exclude_self_any().unwrap_or(false) {
                Some(-1)
            } else {
                None
            },
            exclude_group_names: self.exclude_group_names_any(),
            name_fragments: match self.card_names_any() {
                Some(names) if !names.is_empty() => Some(names),
                _ => None,
            },
            card_property: self.card_property_any(),
            negation: self.negation_any().unwrap_or(false),
            heart_colors: if self.filter_targets_by_heart_colors_any().unwrap_or(false)
                && !self.heart_colors_any().is_empty()
            {
                self.heart_colors_any()
            } else {
                &[]
            },
            ..Default::default()
        }
    }

    /// Returns the count with a caller-provided default.
    pub fn count_or(&self, n: u32) -> u32 {
        self.count.unwrap_or(n)
    }

    /// Returns the first group name, if any.
    pub fn group_name(&self) -> Option<&str> {
        self.group_names_any()
            .and_then(|gn| gn.first().map(|s| s.as_str()))
    }

    /// Returns the group_names slice, or `&[]` if absent.
    pub fn group_names_slice(&self) -> &[String] {
        self.group_names_any().map_or(&[], |v| v.as_slice())
    }

    /// Returns true if `card_id` matches this effect's group filter.
    pub fn matches_group_filter(&self, card_db: &CardDatabase, card_id: i16) -> bool {
        crate::ability::util::card_matches_any_group(card_db, card_id, self.group_names_slice())
    }

    /// Returns the first heart color as a string reference, or a static default.
    pub fn heart_color_or(&self, default: &'static str) -> &str {
        self.heart_colors_any()
            .first()
            .map(|s| s.as_str())
            .unwrap_or(default)
    }

    /// Returns the typed ActionType for this effect.
    pub fn action_type(&self) -> ActionType {
        self.action
    }

    /// Returns true if the action matches the given ActionType variant.
    pub fn is_action(&self, at: ActionType) -> bool {
        self.action == at
    }

    /// Returns the numeric value from `value` or `count`, in that priority.
    pub fn value_or_count(&self, default: u32) -> u32 {
        self.value_any().or(self.count).unwrap_or(default)
    }

    /// Like `value_or_count`, but if a `ref_value` is set, resolves against
    /// the supplied step_results to a value the referenced step produced.
    pub fn value_or_count_resolved(
        &self,
        step_results: &HashMap<String, crate::ability::types::StepOutput>,
        default: u32,
    ) -> i32 {
        if let Some(id) = self.ref_value_any() {
            if let Some(out) = step_results.get(id) {
                if let Some(v) = out.value {
                    return v + self.ref_offset_any().unwrap_or(0);
                }
            }
        }
        self.value_any().or(self.count).unwrap_or(default) as i32
    }

    /// Normalized sub-effect steps
    pub fn normalized_steps(&self) -> Vec<Box<AbilityEffect>> {
        if let Some(ref steps) = self.effect_steps {
            return steps.clone();
        }
        match self.action {
            ActionType::Sequential => self.compound.actions.clone().unwrap_or_default(),
            ActionType::LookAndSelect => {
                let mut out: Vec<Box<AbilityEffect>> = Vec::new();
                if let Some(ref la) = self.compound.look_action {
                    out.push(la.clone());
                }
                if let Some(ref sa) = self.compound.select_action {
                    out.push(sa.clone());
                }
                if let Some(ref fu) = self.compound.followup_action {
                    out.push(fu.clone());
                }
                out
            }
            ActionType::ConditionalAlternative => {
                let mut out: Vec<Box<AbilityEffect>> = Vec::new();
                let mut primary = self.compound.primary_effect.clone();
                let mut alternative = self.alternative_effect_any().cloned();
                let condition = self.compound.alternative_condition.clone();
                if let Some(mut alt) = alternative.take() {
                    if let Some(cond) = condition {
                        alt.condition = Some(cond);
                    }
                    out.push(alt);
                }
                if let Some(pri) = primary.take() {
                    out.push(pri);
                }
                out
            }
            ActionType::ConditionalOnResult => {
                let mut out: Vec<Box<AbilityEffect>> = Vec::new();
                if let Some(ref pri) = self.compound.primary_effect {
                    out.push(pri.clone());
                }
                if let Some(ref follow) = self.compound.followup_action {
                    let mut f = follow.clone();
                    if let Some(rc) = self.compound.result_condition.clone() {
                        f.condition = Some(rc);
                    }
                    out.push(f);
                }
                out
            }
            ActionType::ConditionalOnOptional => {
                let mut step = AbilityEffect::default();
                step.action = ActionType::ConditionalOptional;
                if let Some(ref oa) = self.compound.optional_action {
                    step.text = oa.text.clone();
                }
                step.compound.optional_action = self.compound.optional_action.clone();
                step.compound.conditional_action = self.compound.conditional_action.clone();
                step.compound.conditional_negation = self.compound.conditional_negation;
                vec![Box::new(step)]
            }
            _ => Vec::new(),
        }
    }

    pub fn action_by(&self) -> Option<&str> {
        match self.kind.as_deref() {
            Some(EffectKind::CustomOp { action_by, .. }) => action_by.as_deref(),
            Some(EffectKind::SelectTarget { action_by, .. }) => action_by.as_deref(),
            Some(EffectKind::MoveCards { action_by, .. }) => action_by.as_deref(),
            Some(EffectKind::DrawCards { action_by, .. }) => action_by.as_deref(),
            Some(EffectKind::ChangeState { action_by, .. }) => action_by.as_deref(),
            Some(EffectKind::GainResource { action_by, .. }) => action_by.as_deref(),
            _ => None,
        }
    }

    pub fn opponent_action(&self) -> Option<&AbilityEffect> {
        match self.kind.as_deref() {
            Some(EffectKind::CustomOp {
                opponent_action, ..
            }) => opponent_action.as_deref(),
            _ => None,
        }
    }

    pub fn effect_type(&self) -> Option<&str> {
        match self.kind.as_deref() {
            Some(EffectKind::AbilityOp { effect_type, .. }) => effect_type.as_deref(),
            Some(EffectKind::RestrictionOp { effect_type, .. }) => effect_type.as_deref(),
            Some(EffectKind::CustomOp { effect_type, .. }) => effect_type.as_deref(),
            _ => None,
        }
    }
}

macro_rules! impl_deref_str {
    ($t:ty) => {
        impl core::ops::Deref for $t {
            type Target = str;
            fn deref(&self) -> &str {
                self.as_str()
            }
        }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum PositionInfo {
    String(String),
    Struct {
        position: Option<ArcStr>,
        target: Option<ArcStr>,
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
    pub reference: Option<ArcStr>,
    pub mode: Option<ArcStr>,
    pub base_reference: Option<ArcStr>,
    pub calculation: Option<ArcStr>,
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

/// Maps a stage position to the character name required at that position.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PositionCharacter {
    pub position: String,
    pub character: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardState {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "wait")]
    Wait,
}

impl CardState {
    pub fn as_str(self) -> &'static str {
        match self {
            CardState::Active => "active",
            CardState::Wait => "wait",
        }
    }
}

impl_deref_str!(CardState);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonTarget {
    #[serde(rename = "self")]
    Self_,
    #[serde(rename = "opponent")]
    Opponent,
}

impl ComparisonTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            ComparisonTarget::Self_ => "self",
            ComparisonTarget::Opponent => "opponent",
        }
    }
}

impl_deref_str!(ComparisonTarget);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardProperty {
    #[serde(rename = "has_blade_heart")]
    HasBladeHeart,
    #[serde(rename = "has_score_icon")]
    HasScoreIcon,
    #[serde(rename = "has_all_blade")]
    HasAllBlade,
}

impl CardProperty {
    pub fn as_str(self) -> &'static str {
        match self {
            CardProperty::HasBladeHeart => "has_blade_heart",
            CardProperty::HasScoreIcon => "has_score_icon",
            CardProperty::HasAllBlade => "has_all_blade",
        }
    }
}

impl_deref_str!(CardProperty);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlacementOrder {
    #[serde(rename = "any_order")]
    AnyOrder,
}

impl PlacementOrder {
    pub fn as_str(self) -> &'static str {
        match self {
            PlacementOrder::AnyOrder => "any_order",
        }
    }
}

impl_deref_str!(PlacementOrder);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistinctType {
    #[serde(rename = "card_name")]
    CardName,
    #[serde(rename = "true")]
    True,
    #[serde(rename = "distinct")]
    Distinct,
}

impl DistinctType {
    pub fn as_str(self) -> &'static str {
        match self {
            DistinctType::CardName => "card_name",
            DistinctType::True => "true",
            DistinctType::Distinct => "distinct",
        }
    }
}

impl_deref_str!(DistinctType);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operator {
    #[serde(rename = ">=")]
    Gte,
    #[serde(rename = "<=")]
    Lte,
    #[serde(rename = ">")]
    Gt,
    #[serde(rename = "<")]
    Lt,
    #[serde(rename = "=", alias = "==")]
    Eq,
}

impl Operator {
    pub fn as_str(self) -> &'static str {
        match self {
            Operator::Gte => ">=",
            Operator::Lte => "<=",
            Operator::Gt => ">",
            Operator::Lt => "<",
            Operator::Eq => "=",
        }
    }
}

impl_deref_str!(Operator);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonType {
    #[serde(rename = "score")]
    Score,
    #[serde(rename = "cost")]
    Cost,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "equality")]
    Equality,
}

impl ComparisonType {
    pub fn as_str(self) -> &'static str {
        match self {
            ComparisonType::Score => "score",
            ComparisonType::Cost => "cost",
            ComparisonType::Count => "count",
            ComparisonType::Equality => "equality",
        }
    }
}

impl_deref_str!(ComparisonType);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbilityFilter {
    #[serde(rename = "no_ability")]
    NoAbility,
    #[serde(rename = "has_ability")]
    HasAbility,
    #[serde(rename = "has_ability_type")]
    HasAbilityType,
    #[serde(rename = "no_ability_type")]
    NoAbilityType,
}

impl AbilityFilter {
    pub fn as_str(self) -> &'static str {
        match self {
            AbilityFilter::NoAbility => "no_ability",
            AbilityFilter::HasAbility => "has_ability",
            AbilityFilter::HasAbilityType => "has_ability_type",
            AbilityFilter::NoAbilityType => "no_ability_type",
        }
    }
}

impl_deref_str!(AbilityFilter);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionTarget {
    #[serde(rename = "self")]
    Self_,
    #[serde(rename = "opponent")]
    Opponent,
    #[serde(rename = "both")]
    Both,
    #[serde(rename = "either")]
    Either,
}

impl ConditionTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            ConditionTarget::Self_ => "self",
            ConditionTarget::Opponent => "opponent",
            ConditionTarget::Both => "both",
            ConditionTarget::Either => "either",
        }
    }
}

impl_deref_str!(ConditionTarget);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionCardType {
    #[serde(rename = "member_card")]
    MemberCard,
    #[serde(rename = "live_card")]
    LiveCard,
    #[serde(rename = "energy_card")]
    EnergyCard,
}

impl ConditionCardType {
    pub fn as_str(self) -> &'static str {
        match self {
            ConditionCardType::MemberCard => "member_card",
            ConditionCardType::LiveCard => "live_card",
            ConditionCardType::EnergyCard => "energy_card",
        }
    }
}

impl_deref_str!(ConditionCardType);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Location {
    #[serde(rename = "stage", alias = "ステージ")]
    Stage,
    #[serde(rename = "hand")]
    Hand,
    #[serde(rename = "deck")]
    Deck,
    #[serde(rename = "deck_top")]
    DeckTop,
    #[serde(rename = "discard")]
    Discard,
    #[serde(rename = "energy_zone")]
    EnergyZone,
    #[serde(rename = "live_card_zone")]
    LiveCardZone,
    #[serde(rename = "success_live_card_zone", alias = "success_live_zone")]
    SuccessLiveZone,
    #[serde(rename = "under_member")]
    UnderMember,
    #[serde(rename = "revealed_cards")]
    RevealedCards,
}

impl Location {
    pub fn as_str(self) -> &'static str {
        match self {
            Location::Stage => "stage",
            Location::Hand => "hand",
            Location::Deck => "deck",
            Location::DeckTop => "deck_top",
            Location::Discard => "discard",
            Location::EnergyZone => "energy_zone",
            Location::LiveCardZone => "live_card_zone",
            Location::SuccessLiveZone => "success_live_card_zone",
            Location::UnderMember => "under_member",
            Location::RevealedCards => "revealed_cards",
        }
    }
}

impl_deref_str!(Location);

/// The distinct Condition type as a serde internally-tagged enum.
/// The Python parser already emits `"type": "card_count_condition"` etc.
/// in the JSON — this enum consumes that tag directly via `#[serde(tag = "type")]`.
///
/// Each variant holds only the fields relevant to that condition kind.
/// The common fields (text, negation, phase, cache, trigger_event) appear
/// on every variant because any condition can carry them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum Condition {
    #[serde(rename = "compound", alias = "or_condition")]
    Compound {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        operator: Option<ArcStr>,
        target: Option<ArcStr>,
        #[serde(default)]
        conditions: Option<Vec<Box<Condition>>>,
    },
    #[serde(rename = "card_count_condition", alias = "location_condition")]
    Location {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        // Core location fields (commonly accessed)
        location: Option<ArcStr>,
        #[serde(default)]
        locations: Option<Vec<String>>,
        target: Option<ArcStr>,
        count: Option<u32>,
        operator: Option<ArcStr>,
        card_type: Option<ConditionCardType>,
        #[serde(default)]
        unit: Option<ArcStr>,
        #[serde(default)]
        comparison_target: Option<ComparisonTarget>,
        #[serde(default)]
        comparison_type: Option<ComparisonType>,
        #[serde(default)]
        aggregate: Option<ArcStr>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        group_reference: Option<ArcStr>,
        #[serde(default)]
        exclude_group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_characters: Option<Box<Vec<String>>>,
        cost_limit: Option<u32>,
        cost_limit_operator: Option<Operator>,
        #[serde(default)]
        heart_colors: Option<Vec<String>>,
        heart_type: Option<ArcStr>,
        heart_source: Option<ArcStr>,
        distinct: Option<DistinctInfo>,
        exclude_self: Option<bool>,
        self_target: Option<bool>,
        source: Option<ArcStr>,
        #[serde(default)]
        activation_position: Option<ArcStr>,
        destination: Option<ArcStr>,
        state: Option<CardState>,
        position: Option<PositionInfo>,
        position_compare: Option<ArcStr>,
        require_position_cards: Option<bool>,
        all: Option<bool>,
        all_areas: Option<bool>,
        temporal: Option<ArcStr>,
        yell_trigger: Option<bool>,
        same_name: Option<bool>,
        #[serde(default)]
        card_property: Option<CardProperty>,
        scope: Option<ArcStr>,
        // Boxed sub-checks (rarely accessed together)
        #[serde(default)]
        sub_checks: Option<Box<LocationSubChecks>>,
        #[serde(default)]
        baton_touch_trigger: Option<bool>,
        #[serde(default)]
        min_baton_touch_count: Option<u32>,
    },
    #[serde(
        rename = "comparison_condition",
        alias = "both_condition",
        alias = "all_cost_comparison_condition",
        alias = "highest_cost_on_stage_condition"
    )]
    Comparison {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        comparison_type: Option<ComparisonType>,
        comparison_target: Option<ComparisonTarget>,
        target: Option<ArcStr>,
        location: Option<ArcStr>,
        operator: Option<ArcStr>,
        count: Option<u32>,
        #[serde(default)]
        values: Option<Vec<u32>>,
        card_type: Option<ConditionCardType>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        position: Option<PositionInfo>,
        position_compare: Option<ArcStr>,
        #[serde(default)]
        aggregate: Option<ArcStr>,
        #[serde(default)]
        heart_colors: Option<Vec<String>>,
        #[serde(default)]
        scope: Option<ArcStr>,
        cost_total: Option<u32>,
        cost_total_operator: Option<Operator>,
        resource_type: Option<ArcStr>,
        #[serde(default)]
        delta: Option<bool>,
        cost_limit: Option<u32>,
        source: Option<ArcStr>,
        #[serde(default)]
        comparison_source: Option<ArcStr>,
        #[serde(default)]
        locations: Option<Vec<String>>,
        #[serde(default)]
        exclude_group_names: Option<Box<Vec<String>>>,
        #[serde(default)]
        characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        exclude_characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        same_name: Option<bool>,
        #[serde(default)]
        distinct: Option<DistinctInfo>,
        #[serde(default)]
        all: Option<bool>,
        #[serde(default)]
        all_areas: Option<bool>,
        #[serde(default)]
        exclude_self: Option<bool>,
        #[serde(default)]
        self_target: Option<bool>,
        #[serde(default)]
        destination: Option<ArcStr>,
        #[serde(default)]
        state: Option<CardState>,
        #[serde(default)]
        require_position_cards: Option<bool>,
        #[serde(default)]
        temporal: Option<ArcStr>,
        #[serde(default)]
        yell_trigger: Option<bool>,
        #[serde(default)]
        no_excess_heart: Option<bool>,
        #[serde(default)]
        card_property: Option<CardProperty>,
        #[serde(default)]
        ability_filter: Option<ArcStr>,
        #[serde(default)]
        ability_filter_triggers: Option<Vec<String>>,
        #[serde(default)]
        baton_touch_trigger: Option<bool>,
        #[serde(default)]
        min_baton_touch_count: Option<u32>,
        #[serde(default)]
        from_state: Option<ArcStr>,
        #[serde(default)]
        to_state: Option<ArcStr>,
    },
    #[serde(
        rename = "movement_condition",
        alias = "not_moved",
        alias = "has_moved"
    )]
    Movement {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        movement: Option<ArcStr>,
        location: Option<ArcStr>,
        target: Option<ArcStr>,
        cost_limit: Option<u32>,
        cost_limit_operator: Option<Operator>,
        baton_touch_trigger: Option<bool>,
        min_baton_touch_count: Option<u32>,
        exclude_self: Option<bool>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        card_type: Option<ConditionCardType>,
        #[serde(default)]
        characters: Option<Box<Vec<String>>>,
        baton_touch_source: Option<ArcStr>,
        card_property: Option<CardProperty>,
        comparison_type: Option<ComparisonType>,
        operator: Option<ArcStr>,
        self_effect_only: Option<bool>,
        energy_placed: Option<bool>,
        area_direction: Option<ArcStr>,
        position: Option<PositionInfo>,
        self_target: Option<bool>,
        ability_filter: Option<AbilityFilter>,
        source: Option<ArcStr>,
        destination: Option<ArcStr>,
        from_state: Option<ArcStr>,
        to_state: Option<ArcStr>,
    },
    #[serde(rename = "group_condition")]
    Group {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        all_members: Option<bool>,
        location: Option<ArcStr>,
        target: Option<ArcStr>,
        #[serde(default)]
        heart_colors: Option<Vec<String>>,
        card_type: Option<ConditionCardType>,
        operator: Option<ArcStr>,
        count: Option<u32>,
        aggregate: Option<ArcStr>,
        #[serde(default)]
        exclude_characters: Option<Box<Vec<String>>>,
        temporal: Option<ArcStr>,
        self_target: Option<bool>,
        exclude_self: Option<bool>,
        heart_source: Option<ArcStr>,
        source: Option<ArcStr>,
        #[serde(default)]
        locations: Option<Vec<String>>,
        position: Option<PositionInfo>,
    },
    #[serde(rename = "appearance_condition")]
    Appearance {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        appearance: Option<bool>,
        baton_touch_trigger: Option<bool>,
        location: Option<ArcStr>,
        target: Option<ArcStr>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        cost_limit: Option<u32>,
        card_type: Option<ConditionCardType>,
        #[serde(default)]
        characters: Option<Box<Vec<String>>>,
        #[serde(default)]
        positions_characters: Option<Vec<PositionCharacter>>,
        min_baton_touch_count: Option<u32>,
        activation_position: Option<ArcStr>,
        exclude_self: Option<bool>,
        position_compare: Option<ArcStr>,
        position: Option<PositionInfo>,
        card_property: Option<CardProperty>,
        #[serde(default)]
        all_areas: Option<bool>,
        cost_reference_character: Option<ArcStr>,
        cost_reference_operator: Option<Operator>,
        appearance_source: Option<ArcStr>,
        operator: Option<ArcStr>,
    },
    #[serde(rename = "temporal_condition")]
    Temporal {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        temporal: Option<ArcStr>,
        turn_number: Option<u32>,
        count: Option<u32>,
        location: Option<ArcStr>,
        card_type: Option<ConditionCardType>,
        target: Option<ArcStr>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        temporal_scope: Option<ArcStr>,
        position: Option<PositionInfo>,
        #[serde(default)]
        locations: Option<Vec<String>>,
        #[serde(default)]
        heart_colors: Option<Vec<String>>,
        aggregate: Option<ArcStr>,
        self_target: Option<bool>,
        condition: Option<Box<Condition>>,
    },
    #[serde(
        rename = "state_condition",
        alias = "energy_state_condition",
        alias = "state_change_condition"
    )]
    State {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        state: Option<EffectState>,
        energy_state: Option<ArcStr>,
        target: Option<ArcStr>,
        resource_type: Option<ArcStr>,
        all: Option<bool>,
        #[serde(default)]
        group_names: Option<Box<Vec<String>>>,
        card_type: Option<ConditionCardType>,
        #[serde(default)]
        characters: Option<Box<Vec<String>>>,
        cost_limit: Option<u32>,
        cost_limit_operator: Option<Operator>,
        from_state: Option<ArcStr>,
        to_state: Option<ArcStr>,
        count: Option<u32>,
        operator: Option<ArcStr>,
    },
    #[serde(rename = "resource_condition", alias = "card_blade_condition")]
    Resource {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        resource_type: Option<ArcStr>,
        target: Option<ArcStr>,
        location: Option<ArcStr>,
        operator: Option<ArcStr>,
        count: Option<u32>,
        delta: Option<bool>,
        #[serde(default)]
        heart_colors: Option<Vec<String>>,
        position: Option<PositionInfo>,
        source: Option<ArcStr>,
    },
    #[serde(rename = "ability_filter_condition")]
    AbilityFilter {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        ability_filter: Option<AbilityFilter>,
        #[serde(default)]
        ability_filter_triggers: Option<Vec<String>>,
        target: Option<ArcStr>,
        location: Option<ArcStr>,
        operator: Option<ArcStr>,
        count: Option<u32>,
    },
    #[serde(rename = "score_threshold_condition")]
    ScoreThreshold {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        count: Option<u32>,
        operator: Option<ArcStr>,
        target: Option<ArcStr>,
    },
    #[serde(rename = "choice_condition", alias = "position_change_condition")]
    Choice {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        #[serde(default)]
        options: Option<Vec<Box<AbilityEffect>>>,
    },
    #[serde(rename = "complex_condition")]
    Complex {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        cause: Option<Box<Condition>>,
        effect: Option<Box<AbilityEffect>>,
    },
    #[serde(rename = "position_condition")]
    PositionCond {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        target: Option<ArcStr>,
        position: Option<PositionInfo>,
    },
    #[serde(rename = "opponent_choice_condition")]
    OpponentChoice {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        target: Option<ArcStr>,
    },
    #[serde(rename = "opponent_live_success")]
    OpponentLiveSuccess {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        no_excess_heart: Option<bool>,
    },
    #[serde(rename = "no_excess_heart")]
    NoExcessHeart {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        target: Option<ArcStr>,
    },
    #[serde(
        rename = "otherwise_condition",
        alias = "action_success_condition",
        alias = "custom"
    )]
    AlwaysTrue {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
    },
    #[serde(rename = "any_of_condition")]
    AnyOf {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        #[serde(default)]
        any_of: Option<Vec<String>>,
    },
    #[serde(rename = "all_revealed_match_heart_color")]
    AllRevealedMatchHeartColor {
        #[serde(default)]
        #[cfg(feature = "debug_conditions")]
        text: Option<String>,
        negation: Option<bool>,
        phase: Option<ArcStr>,
        phase_target: Option<ArcStr>,
        cache: Option<bool>,
        #[cfg(feature = "debug_conditions")]
        trigger_event: Option<Box<TriggerEvent>>,
        #[serde(default)]
        count: Option<u32>,
        #[serde(default)]
        operator: Option<ArcStr>,
    },
}

/// Sub-checks that can appear on Location conditions.
/// Boxed together because they're rarely all present at once.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct LocationSubChecks {
    pub card_property: Option<CardProperty>,
    pub baton_touch_trigger: Option<bool>,
    pub baton_touch_source: Option<ArcStr>,
    pub min_baton_touch_count: Option<u32>,
    pub ability_filter: Option<AbilityFilter>,
    #[serde(default)]
    pub ability_filter_triggers: Option<Vec<String>>,
    pub aggregate: Option<ArcStr>,
    pub no_excess_heart: Option<bool>,
    pub original_value: Option<bool>,
    pub activation_position: Option<ArcStr>,
    pub unit: Option<ArcStr>,
    #[serde(default)]
    pub values: Option<Vec<u32>>,
    pub group_reference: Option<ArcStr>,
    pub reference_card: Option<ArcStr>,
}

impl Default for Condition {
    fn default() -> Self {
        Condition::AlwaysTrue {
            #[cfg(feature = "debug_conditions")]
            text: None,
            negation: None,
            phase: None,
            phase_target: None,
            cache: None,
            #[cfg(feature = "debug_conditions")]
            trigger_event: None,
        }
    }
}

// ============== Condition field accessor methods ==============

impl Condition {
    pub fn get_text(&self) -> Option<&str> {
        #[cfg(feature = "debug_conditions")]
        {
            let t: Option<&str> = match self {
                Condition::Compound { text, .. }
                | Condition::Location { text, .. }
                | Condition::Comparison { text, .. }
                | Condition::Movement { text, .. }
                | Condition::Group { text, .. }
                | Condition::Appearance { text, .. }
                | Condition::Temporal { text, .. }
                | Condition::State { text, .. }
                | Condition::Resource { text, .. }
                | Condition::AbilityFilter { text, .. }
                | Condition::ScoreThreshold { text, .. }
                | Condition::Choice { text, .. }
                | Condition::Complex { text, .. }
                | Condition::PositionCond { text, .. }
                | Condition::OpponentChoice { text, .. }
                | Condition::OpponentLiveSuccess { text, .. }
                | Condition::NoExcessHeart { text, .. }
                | Condition::AlwaysTrue { text, .. }
                | Condition::AnyOf { text, .. }
                | Condition::AllRevealedMatchHeartColor { text, .. } => text.as_deref(),
            };
            // Preserve old behavior: text was always "" even when absent.
            // Code that compares condition text (e.g. same_as_prev in compound.rs)
            // relies on None == None matching the old "" == "".
            if t.is_none() {
                Some("")
            } else {
                t
            }
        }
        #[cfg(not(feature = "debug_conditions"))]
        {
            None
        }
    }

    pub fn get_negation(&self) -> Option<bool> {
        match self {
            Condition::Compound { negation, .. }
            | Condition::Location { negation, .. }
            | Condition::Comparison { negation, .. }
            | Condition::Movement { negation, .. }
            | Condition::Group { negation, .. }
            | Condition::Appearance { negation, .. }
            | Condition::Temporal { negation, .. }
            | Condition::State { negation, .. }
            | Condition::Resource { negation, .. }
            | Condition::AbilityFilter { negation, .. }
            | Condition::ScoreThreshold { negation, .. }
            | Condition::Choice { negation, .. }
            | Condition::Complex { negation, .. }
            | Condition::PositionCond { negation, .. }
            | Condition::OpponentChoice { negation, .. }
            | Condition::OpponentLiveSuccess { negation, .. }
            | Condition::NoExcessHeart { negation, .. }
            | Condition::AlwaysTrue { negation, .. }
            | Condition::AnyOf { negation, .. }
            | Condition::AllRevealedMatchHeartColor { negation, .. } => *negation,
        }
    }

    pub fn get_phase(&self) -> Option<&str> {
        match self {
            Condition::Compound { phase, .. }
            | Condition::Location { phase, .. }
            | Condition::Comparison { phase, .. }
            | Condition::Movement { phase, .. }
            | Condition::Group { phase, .. }
            | Condition::Appearance { phase, .. }
            | Condition::Temporal { phase, .. }
            | Condition::State { phase, .. }
            | Condition::Resource { phase, .. }
            | Condition::AbilityFilter { phase, .. }
            | Condition::ScoreThreshold { phase, .. }
            | Condition::Choice { phase, .. }
            | Condition::Complex { phase, .. }
            | Condition::PositionCond { phase, .. }
            | Condition::OpponentChoice { phase, .. }
            | Condition::OpponentLiveSuccess { phase, .. }
            | Condition::NoExcessHeart { phase, .. }
            | Condition::AlwaysTrue { phase, .. }
            | Condition::AnyOf { phase, .. }
            | Condition::AllRevealedMatchHeartColor { phase, .. } => phase.as_deref(),
        }
    }

    pub fn get_phase_target(&self) -> Option<&str> {
        match self {
            Condition::Compound { phase_target, .. }
            | Condition::Location { phase_target, .. }
            | Condition::Comparison { phase_target, .. }
            | Condition::Movement { phase_target, .. }
            | Condition::Group { phase_target, .. }
            | Condition::Appearance { phase_target, .. }
            | Condition::Temporal { phase_target, .. }
            | Condition::State { phase_target, .. }
            | Condition::Resource { phase_target, .. }
            | Condition::AbilityFilter { phase_target, .. }
            | Condition::ScoreThreshold { phase_target, .. }
            | Condition::Choice { phase_target, .. }
            | Condition::Complex { phase_target, .. }
            | Condition::PositionCond { phase_target, .. }
            | Condition::OpponentChoice { phase_target, .. }
            | Condition::OpponentLiveSuccess { phase_target, .. }
            | Condition::NoExcessHeart { phase_target, .. }
            | Condition::AlwaysTrue { phase_target, .. }
            | Condition::AnyOf { phase_target, .. }
            | Condition::AllRevealedMatchHeartColor { phase_target, .. } => phase_target.as_deref(),
        }
    }

    pub fn get_cache(&self) -> Option<bool> {
        match self {
            Condition::Compound { cache, .. }
            | Condition::Location { cache, .. }
            | Condition::Comparison { cache, .. }
            | Condition::Movement { cache, .. }
            | Condition::Group { cache, .. }
            | Condition::Appearance { cache, .. }
            | Condition::Temporal { cache, .. }
            | Condition::State { cache, .. }
            | Condition::Resource { cache, .. }
            | Condition::AbilityFilter { cache, .. }
            | Condition::ScoreThreshold { cache, .. }
            | Condition::Choice { cache, .. }
            | Condition::Complex { cache, .. }
            | Condition::PositionCond { cache, .. }
            | Condition::OpponentChoice { cache, .. }
            | Condition::OpponentLiveSuccess { cache, .. }
            | Condition::NoExcessHeart { cache, .. }
            | Condition::AlwaysTrue { cache, .. }
            | Condition::AnyOf { cache, .. }
            | Condition::AllRevealedMatchHeartColor { cache, .. } => *cache,
        }
    }

    pub fn get_trigger_event(&self) -> Option<&TriggerEvent> {
        #[cfg(feature = "debug_conditions")]
        {
            match self {
                Condition::Compound { trigger_event, .. }
                | Condition::Location { trigger_event, .. }
                | Condition::Comparison { trigger_event, .. }
                | Condition::Movement { trigger_event, .. }
                | Condition::Group { trigger_event, .. }
                | Condition::Appearance { trigger_event, .. }
                | Condition::Temporal { trigger_event, .. }
                | Condition::State { trigger_event, .. }
                | Condition::Resource { trigger_event, .. }
                | Condition::AbilityFilter { trigger_event, .. }
                | Condition::ScoreThreshold { trigger_event, .. }
                | Condition::Choice { trigger_event, .. }
                | Condition::Complex { trigger_event, .. }
                | Condition::PositionCond { trigger_event, .. }
                | Condition::OpponentChoice { trigger_event, .. }
                | Condition::OpponentLiveSuccess { trigger_event, .. }
                | Condition::NoExcessHeart { trigger_event, .. }
                | Condition::AlwaysTrue { trigger_event, .. }
                | Condition::AnyOf { trigger_event, .. }
                | Condition::AllRevealedMatchHeartColor { trigger_event, .. } => {
                    trigger_event.as_deref()
                }
            }
        }
        #[cfg(not(feature = "debug_conditions"))]
        {
            None
        }
    }

    pub fn get_location(&self) -> Option<&str> {
        match self {
            Condition::Location { location, .. } => location.as_deref(),
            Condition::Comparison { location, .. } => location.as_deref(),
            Condition::Movement { location, .. } => location.as_deref(),
            Condition::Group { location, .. } => location.as_deref(),
            Condition::Appearance { location, .. } => location.as_deref(),
            Condition::Temporal { location, .. } => location.as_deref(),
            Condition::Resource { location, .. } => location.as_deref(),
            Condition::AbilityFilter { location, .. } => location.as_deref(),
            _ => None,
        }
    }

    pub fn get_locations(&self) -> Option<&[String]> {
        match self {
            Condition::Location { locations, .. } => locations.as_deref(),
            Condition::Group { locations, .. } => locations.as_deref(),
            Condition::Temporal { locations, .. } => locations.as_deref(),
            _ => None,
        }
    }

    pub fn get_target(&self) -> Option<&str> {
        match self {
            Condition::Compound { target, .. } => target.as_deref(),
            Condition::Location { target, .. } => target.as_deref(),
            Condition::Comparison { target, .. } => target.as_deref(),
            Condition::Movement { target, .. } => target.as_deref(),
            Condition::Group { target, .. } => target.as_deref(),
            Condition::Appearance { target, .. } => target.as_deref(),
            Condition::Temporal { target, .. } => target.as_deref(),
            Condition::State { target, .. } => target.as_deref(),
            Condition::Resource { target, .. } => target.as_deref(),
            Condition::AbilityFilter { target, .. } => target.as_deref(),
            Condition::ScoreThreshold { target, .. } => target.as_deref(),
            Condition::PositionCond { target, .. } => target.as_deref(),
            Condition::OpponentChoice { target, .. } => target.as_deref(),
            Condition::NoExcessHeart { target, .. } => target.as_deref(),
            _ => None,
        }
    }

    pub fn get_count(&self) -> Option<u32> {
        match self {
            Condition::Location { count, .. } => *count,
            Condition::Comparison { count, .. } => *count,
            Condition::Group { count, .. } => *count,
            Condition::Appearance { .. } => None,
            Condition::Temporal { count, .. } => *count,
            Condition::State { count, .. } => *count,
            Condition::Resource { count, .. } => *count,
            Condition::AbilityFilter { count, .. } => *count,
            Condition::ScoreThreshold { count, .. } => *count,
            Condition::AllRevealedMatchHeartColor { count, .. } => *count,
            _ => None,
        }
    }

    pub fn get_operator(&self) -> Option<&str> {
        match self {
            Condition::Compound { operator, .. }
            | Condition::Location { operator, .. }
            | Condition::Comparison { operator, .. }
            | Condition::Movement { operator, .. }
            | Condition::Group { operator, .. }
            | Condition::Appearance { operator, .. }
            | Condition::State { operator, .. }
            | Condition::Resource { operator, .. }
            | Condition::AbilityFilter { operator, .. }
            | Condition::ScoreThreshold { operator, .. }
            | Condition::AllRevealedMatchHeartColor { operator, .. } => operator.as_deref(),
            _ => None,
        }
    }

    pub fn get_card_type(&self) -> Option<ConditionCardType> {
        match self {
            Condition::Location { card_type, .. } => *card_type,
            Condition::Comparison { card_type, .. } => *card_type,
            Condition::Movement { card_type, .. } => *card_type,
            Condition::Group { card_type, .. } => *card_type,
            Condition::Appearance { card_type, .. } => *card_type,
            Condition::Temporal { card_type, .. } => *card_type,
            Condition::State { card_type, .. } => *card_type,
            _ => None,
        }
    }

    pub fn get_group_names(&self) -> Option<&[String]> {
        match self {
            Condition::Location { group_names, .. } => group_names.as_ref().map(|b| b.as_slice()),
            Condition::Comparison { group_names, .. } => group_names.as_ref().map(|b| b.as_slice()),
            Condition::Movement { group_names, .. } => group_names.as_ref().map(|b| b.as_slice()),
            Condition::Group { group_names, .. } => group_names.as_ref().map(|b| b.as_slice()),
            Condition::Appearance { group_names, .. } => group_names.as_ref().map(|b| b.as_slice()),
            Condition::Temporal { group_names, .. } => group_names.as_ref().map(|b| b.as_slice()),
            Condition::State { group_names, .. } => group_names.as_ref().map(|b| b.as_slice()),
            _ => None,
        }
    }

    pub fn get_characters(&self) -> Option<&[String]> {
        match self {
            Condition::Location { characters, .. } => characters.as_ref().map(|b| b.as_slice()),
            Condition::Movement { characters, .. } => characters.as_ref().map(|b| b.as_slice()),
            Condition::Group { .. } => None,
            Condition::Appearance { characters, .. } => characters.as_ref().map(|b| b.as_slice()),
            Condition::State { characters, .. } => characters.as_ref().map(|b| b.as_slice()),
            _ => None,
        }
    }

    pub fn get_exclude_characters(&self) -> Option<&[String]> {
        match self {
            Condition::Location {
                exclude_characters, ..
            } => exclude_characters.as_ref().map(|b| b.as_slice()),
            Condition::Group {
                exclude_characters, ..
            } => exclude_characters.as_ref().map(|b| b.as_slice()),
            Condition::Movement { .. } => None,
            _ => None,
        }
    }

    pub fn get_state(&self) -> Option<CardState> {
        match self {
            Condition::Location { state, .. } => *state,
            Condition::State { state, .. } => state.as_ref().and_then(|s| match s.as_str() {
                "active" => Some(CardState::Active),
                "wait" => Some(CardState::Wait),
                _ => None,
            }),
            _ => None,
        }
    }

    pub fn get_position(&self) -> Option<&PositionInfo> {
        match self {
            Condition::Location { position, .. } => position.as_ref(),
            Condition::Comparison { position, .. } => position.as_ref(),
            Condition::Movement { position, .. } => position.as_ref(),
            Condition::Group { position, .. } => position.as_ref(),
            Condition::Appearance { position, .. } => position.as_ref(),
            Condition::Temporal { position, .. } => position.as_ref(),
            Condition::Resource { position, .. } => position.as_ref(),
            Condition::PositionCond { position, .. } => position.as_ref(),
            _ => None,
        }
    }

    pub fn get_movement(&self) -> Option<&str> {
        match self {
            Condition::Movement { movement, .. } => movement.as_deref(),
            _ => None,
        }
    }

    pub fn get_temporal(&self) -> Option<&str> {
        match self {
            Condition::Location { temporal, .. } => temporal.as_deref(),
            Condition::Group { temporal, .. } => temporal.as_deref(),
            Condition::Temporal { temporal, .. } => temporal.as_deref(),
            _ => None,
        }
    }

    pub fn get_source(&self) -> Option<&str> {
        let direct = match self {
            Condition::Location { source, .. } => source.as_deref(),
            Condition::Movement { source, .. } => source.as_deref(),
            Condition::Group { source, .. } => source.as_deref(),
            Condition::Comparison { source, .. } => source.as_deref(),
            Condition::Resource { source, .. } => source.as_deref(),
            _ => None,
        };
        direct.or_else(|| self.get_trigger_event()?.source.as_deref())
    }

    pub fn get_destination(&self) -> Option<&str> {
        let direct = match self {
            Condition::Location { destination, .. } => destination.as_deref(),
            Condition::Movement { destination, .. } => destination.as_deref(),
            _ => None,
        };
        direct.or_else(|| self.get_trigger_event()?.destination.as_deref())
    }

    pub fn get_from_state(&self) -> Option<&str> {
        let direct = match self {
            Condition::Movement { from_state, .. } => from_state.as_deref(),
            Condition::State { from_state, .. } => from_state.as_deref(),
            _ => None,
        };
        direct.or_else(|| self.get_trigger_event()?.from_state.as_deref())
    }

    pub fn get_to_state(&self) -> Option<&str> {
        let direct = match self {
            Condition::Movement { to_state, .. } => to_state.as_deref(),
            Condition::State { to_state, .. } => to_state.as_deref(),
            _ => None,
        };
        direct.or_else(|| self.get_trigger_event()?.to_state.as_deref())
    }

    pub fn get_self_effect_only(&self) -> Option<bool> {
        let direct = match self {
            Condition::Movement {
                self_effect_only, ..
            } => *self_effect_only,
            _ => None,
        };
        direct.or_else(|| self.get_trigger_event()?.self_effect_only)
    }

    pub fn get_heart_colors(&self) -> Option<&[String]> {
        match self {
            Condition::Location { heart_colors, .. } => heart_colors.as_deref(),
            Condition::Comparison { heart_colors, .. } => heart_colors.as_deref(),
            Condition::Group { heart_colors, .. } => heart_colors.as_deref(),
            Condition::Temporal { heart_colors, .. } => heart_colors.as_deref(),
            Condition::Resource { heart_colors, .. } => heart_colors.as_deref(),
            _ => None,
        }
    }

    pub fn get_exclude_self(&self) -> Option<bool> {
        match self {
            Condition::Location { exclude_self, .. } => *exclude_self,
            Condition::Movement { exclude_self, .. } => *exclude_self,
            Condition::Appearance { exclude_self, .. } => *exclude_self,
            Condition::Group { exclude_self, .. } => *exclude_self,
            _ => None,
        }
    }

    pub fn get_self_target(&self) -> Option<bool> {
        match self {
            Condition::Location { self_target, .. } => *self_target,
            Condition::Movement { self_target, .. } => *self_target,
            Condition::Group { self_target, .. } => *self_target,
            Condition::Temporal { self_target, .. } => *self_target,
            _ => None,
        }
    }

    pub fn get_cost_limit(&self) -> Option<u32> {
        match self {
            Condition::Location { cost_limit, .. } => *cost_limit,
            Condition::Comparison { cost_limit, .. } => *cost_limit,
            Condition::Movement { cost_limit, .. } => *cost_limit,
            Condition::Appearance { cost_limit, .. } => *cost_limit,
            Condition::State { cost_limit, .. } => *cost_limit,
            _ => None,
        }
    }

    pub fn get_cost_limit_operator(&self) -> Option<Operator> {
        match self {
            Condition::Location {
                cost_limit_operator,
                ..
            } => *cost_limit_operator,
            Condition::Movement {
                cost_limit_operator,
                ..
            } => *cost_limit_operator,
            Condition::State {
                cost_limit_operator,
                ..
            } => *cost_limit_operator,
            _ => None,
        }
    }

    pub fn get_comparison_type(&self) -> Option<&str> {
        match self {
            Condition::Comparison {
                comparison_type, ..
            } => comparison_type.map(|ct| ct.as_str()),
            Condition::Movement {
                comparison_type, ..
            } => comparison_type.map(|ct| ct.as_str()),
            Condition::Location {
                comparison_type, ..
            } => comparison_type.map(|ct| ct.as_str()),
            _ => None,
        }
    }

    pub fn get_resource_type(&self) -> Option<&str> {
        match self {
            Condition::Comparison { resource_type, .. } => resource_type.as_deref(),
            Condition::State { resource_type, .. } => resource_type.as_deref(),
            Condition::Resource { resource_type, .. } => resource_type.as_deref(),
            _ => None,
        }
    }

    pub fn get_card_property(&self) -> Option<CardProperty> {
        match self {
            Condition::Location { card_property, .. } => *card_property,
            Condition::Comparison { card_property, .. } => *card_property,
            Condition::Movement { card_property, .. } => *card_property,
            Condition::Appearance { card_property, .. } => *card_property,
            _ => None,
        }
    }

    pub fn get_aggregate(&self) -> Option<&str> {
        match self {
            Condition::Location { aggregate, .. } => aggregate.as_deref(),
            Condition::Comparison { aggregate, .. } => aggregate.as_deref(),
            Condition::Group { aggregate, .. } => aggregate.as_deref(),
            Condition::Temporal { aggregate, .. } => aggregate.as_deref(),
            _ => None,
        }
    }

    pub fn get_ability_filter(&self) -> Option<&AbilityFilter> {
        match self {
            Condition::Movement { ability_filter, .. } => ability_filter.as_ref(),
            Condition::AbilityFilter { ability_filter, .. } => ability_filter.as_ref(),
            _ => None,
        }
    }

    pub fn get_baton_touch_trigger(&self) -> Option<bool> {
        match self {
            Condition::Movement {
                baton_touch_trigger,
                ..
            } => *baton_touch_trigger,
            Condition::Appearance {
                baton_touch_trigger,
                ..
            } => *baton_touch_trigger,
            Condition::Location {
                baton_touch_trigger,
                ..
            } => *baton_touch_trigger,
            _ => None,
        }
    }

    pub fn get_energy_state(&self) -> Option<&str> {
        match self {
            Condition::State { energy_state, .. } => energy_state.as_deref(),
            _ => None,
        }
    }

    pub fn get_heart_source(&self) -> Option<&str> {
        match self {
            Condition::Location { heart_source, .. } => heart_source.as_deref(),
            Condition::Group { heart_source, .. } => heart_source.as_deref(),
            _ => None,
        }
    }

    pub fn get_exclude_group_names(&self) -> Option<&[String]> {
        match self {
            Condition::Location {
                exclude_group_names,
                ..
            } => exclude_group_names.as_ref().map(|b| b.as_slice()),
            _ => None,
        }
    }

    pub fn get_distinct(&self) -> Option<&DistinctInfo> {
        match self {
            Condition::Location { distinct, .. } => distinct.as_ref(),
            _ => None,
        }
    }

    pub fn get_position_compare(&self) -> Option<&str> {
        match self {
            Condition::Location {
                position_compare, ..
            } => position_compare.as_deref(),
            Condition::Comparison {
                position_compare, ..
            } => position_compare.as_deref(),
            Condition::Appearance {
                position_compare, ..
            } => position_compare.as_deref(),
            _ => None,
        }
    }

    pub fn get_delta(&self) -> Option<bool> {
        match self {
            Condition::Comparison { delta, .. } => *delta,
            Condition::Resource { delta, .. } => *delta,
            _ => None,
        }
    }
}

/// Rich description of what event triggers a condition.
/// Parser-produced documentary field. The engine reads from this when
/// the corresponding flat Condition field is absent.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TriggerEvent {
    #[serde(rename = "type")]
    pub event_type: Option<ArcStr>,
    pub tense: Option<ArcStr>,
    pub location: Option<ArcStr>,
    pub source_character: Option<ArcStr>,
    pub source_group: Option<ArcStr>,
    pub cost_comparison: Option<CostComparison>,
    pub min_count: Option<u32>,
    pub exclude_characters: Option<Box<Vec<String>>>,
    pub ability_filter: Option<AbilityFilter>,
    pub self_effect_only: Option<bool>,
    pub energy_placed: Option<bool>,
    pub phase: Option<ArcStr>,
    pub phase_target: Option<ArcStr>,
    pub recurrence: Option<ArcStr>,
    pub events: Option<Vec<TriggerEvent>>,
    pub source: Option<ArcStr>,
    pub destination: Option<ArcStr>,
    pub from_state: Option<ArcStr>,
    pub to_state: Option<ArcStr>,
}

/// Cost comparison for baton touch: e.g. "このメンバーよりコストが低い" (cost < activating).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CostComparison {
    pub operator: Option<Operator>,
    pub relative_to: Option<ArcStr>,
    pub cost_limit: Option<u32>,
    pub cost_limit_operator: Option<Operator>,
}

impl Condition {
    /// Build a `CardFilter` containing the 7 base filter fields, mirroring
    /// `AbilityEffect::filter_subset` and `AbilityCost::filter_subset`.
    pub fn filter_subset(&self) -> crate::ability::util::CardFilter<'_> {
        let (
            card_type,
            group,
            cost_limit,
            cost_operator,
            characters,
            exclude_characters,
            exclude_group_names,
        ) = match self {
            Condition::Location {
                card_type,
                group_names,
                exclude_group_names: egn,
                characters: ch,
                exclude_characters: ec,
                cost_limit: cl,
                cost_limit_operator: clo,
                operator,
                ..
            } => (
                *card_type,
                group_names.as_ref(),
                *cl,
                clo.as_deref().or(operator.as_deref()),
                ch.as_ref(),
                ec.as_ref(),
                egn.as_ref(),
            ),
            Condition::Group {
                card_type,
                group_names,
                operator,
                ..
            } => (
                *card_type,
                group_names.as_ref(),
                None,
                operator.as_deref(),
                None,
                None,
                None,
            ),
            Condition::Movement {
                card_type,
                group_names,
                characters,
                ..
            } => (
                *card_type,
                group_names.as_ref(),
                None,
                None,
                characters.as_ref(),
                None,
                None,
            ),
            Condition::Comparison {
                card_type,
                group_names,
                cost_limit,
                operator,
                ..
            } => (
                *card_type,
                group_names.as_ref(),
                *cost_limit,
                operator.as_deref(),
                None,
                None,
                None,
            ),
            Condition::Appearance {
                card_type,
                group_names,
                cost_limit,
                characters,
                operator,
                ..
            } => (
                *card_type,
                group_names.as_ref(),
                *cost_limit,
                operator.as_deref(),
                characters.as_ref(),
                None,
                None,
            ),
            _ => (None, None, None, None, None, None, None),
        };
        crate::ability::util::CardFilter {
            card_type: card_type.map(|ct| ct.as_str()),
            group: group.and_then(|v| v.first()).map(|s| s.as_str()),
            cost_limit,
            cost_operator: cost_operator,
            characters: characters.map(|b| b.as_ref()),
            exclude_characters: exclude_characters.map(|b| b.as_ref()),
            exclude_self: None,
            exclude_group_names: exclude_group_names.map(|b| b.as_ref()),
            ..Default::default()
        }
    }

    pub fn condition_type(&self) -> Option<ConditionType> {
        match self {
            Condition::Compound { .. } => Some(ConditionType::Compound),
            Condition::Location { .. } => Some(ConditionType::LocationCondition),
            Condition::Comparison { .. } => Some(ConditionType::ComparisonCondition),
            Condition::Movement { movement: None, .. } => Some(ConditionType::NotMoved),
            Condition::Movement {
                movement: Some(m), ..
            } if m.as_ref() == "has_moved" => Some(ConditionType::HasMoved),
            Condition::Movement { .. } => Some(ConditionType::MovementCondition),
            Condition::Group { .. } => Some(ConditionType::GroupCondition),
            Condition::Appearance { .. } => Some(ConditionType::AppearanceCondition),
            Condition::Temporal { .. } => Some(ConditionType::TemporalCondition),
            Condition::State { .. } => Some(ConditionType::StateCondition),
            Condition::Resource { .. } => Some(ConditionType::ResourceCondition),
            Condition::AbilityFilter { .. } => Some(ConditionType::AbilityFilterCondition),
            Condition::ScoreThreshold { .. } => Some(ConditionType::ScoreThresholdCondition),
            Condition::Choice { .. } => Some(ConditionType::ChoiceCondition),
            Condition::Complex { .. } => Some(ConditionType::ComplexCondition),
            Condition::PositionCond { .. } => Some(ConditionType::PositionCondition),
            Condition::OpponentChoice { .. } => Some(ConditionType::OpponentChoiceCondition),
            Condition::OpponentLiveSuccess { .. } => Some(ConditionType::OpponentLiveSuccess),
            Condition::NoExcessHeart { .. } => Some(ConditionType::NoExcessHeart),
            Condition::AlwaysTrue { .. } => Some(ConditionType::OtherwiseCondition),
            Condition::AnyOf { .. } => Some(ConditionType::AnyOfCondition),
            Condition::AllRevealedMatchHeartColor { .. } => {
                Some(ConditionType::AllRevealedMatchHeartColor)
            }
        }
    }

    pub fn get_positions_characters(&self) -> Option<&[PositionCharacter]> {
        match self {
            Condition::Appearance {
                positions_characters,
                ..
            } => positions_characters.as_deref(),
            _ => None,
        }
    }

    pub fn get_activation_position(&self) -> Option<&str> {
        match self {
            Condition::Appearance {
                activation_position,
                ..
            } => activation_position.as_deref(),
            Condition::Location { sub_checks, .. } => sub_checks
                .as_ref()
                .and_then(|sc| sc.activation_position.as_deref()),
            _ => None,
        }
    }

    pub fn set_position(&mut self, pos: PositionInfo) {
        match self {
            Condition::Location { position, .. }
            | Condition::Comparison { position, .. }
            | Condition::Movement { position, .. }
            | Condition::Group { position, .. }
            | Condition::Appearance { position, .. }
            | Condition::Temporal { position, .. }
            | Condition::Resource { position, .. }
            | Condition::PositionCond { position, .. } => {
                *position = Some(pos);
            }
            _ => {}
        }
    }

    pub fn set_activation_position(&mut self, act_pos: String) {
        match self {
            Condition::Appearance {
                activation_position,
                ..
            } => {
                *activation_position = Some(act_pos.into());
            }
            Condition::Location { sub_checks, .. } => {
                let mut checks = sub_checks.take().unwrap_or_default();
                checks.activation_position = Some(act_pos.into());
                *sub_checks = Some(checks);
            }
            _ => {}
        }
    }

    pub fn set_group_names(&mut self, gns: Vec<String>) {
        match self {
            Condition::Location { group_names, .. }
            | Condition::Comparison { group_names, .. }
            | Condition::Movement { group_names, .. }
            | Condition::Group { group_names, .. }
            | Condition::Appearance { group_names, .. }
            | Condition::Temporal { group_names, .. }
            | Condition::State { group_names, .. } => {
                *group_names = Some(Box::new(gns));
            }
            _ => {}
        }
    }

    pub fn get_conditions_mut(&mut self) -> Option<&mut Vec<Box<Condition>>> {
        match self {
            Condition::Compound { conditions, .. } => conditions.as_mut(),
            _ => None,
        }
    }

    pub fn get_options(&self) -> Option<&[Box<AbilityEffect>]> {
        match self {
            Condition::Choice { options, .. } => options.as_deref(),
            _ => None,
        }
    }

    pub fn get_effect(&self) -> Option<&AbilityEffect> {
        match self {
            Condition::Complex { effect, .. } => effect.as_deref(),
            _ => None,
        }
    }

    pub fn get_cause(&self) -> Option<&Condition> {
        match self {
            Condition::Complex { cause, .. } => cause.as_deref(),
            _ => None,
        }
    }

    pub fn get_condition(&self) -> Option<&Condition> {
        match self {
            Condition::Temporal { condition, .. } => condition.as_deref(),
            _ => None,
        }
    }

    pub fn get_conditions(&self) -> Option<&[Box<Condition>]> {
        match self {
            Condition::Compound { conditions, .. } => conditions.as_deref(),
            _ => None,
        }
    }

    pub fn get_no_excess_heart(&self) -> Option<bool> {
        match self {
            Condition::OpponentLiveSuccess {
                no_excess_heart, ..
            } => *no_excess_heart,
            Condition::Location { sub_checks, .. } => {
                sub_checks.as_ref().and_then(|sc| sc.no_excess_heart)
            }
            _ => None,
        }
    }

    pub fn get_group_reference(&self) -> Option<&str> {
        match self {
            Condition::Location {
                group_reference,
                sub_checks,
                ..
            } => group_reference.as_deref().or_else(|| {
                sub_checks
                    .as_ref()
                    .and_then(|sc| sc.group_reference.as_deref())
            }),
            _ => None,
        }
    }

    pub fn get_unit(&self) -> Option<&str> {
        match self {
            Condition::Location {
                unit, sub_checks, ..
            } => unit
                .as_deref()
                .or_else(|| sub_checks.as_ref().and_then(|sc| sc.unit.as_deref())),
            _ => None,
        }
    }

    pub fn get_all_areas(&self) -> Option<bool> {
        match self {
            Condition::Location { all_areas, .. } => *all_areas,
            Condition::Appearance { all_areas, .. } => *all_areas,
            _ => None,
        }
    }

    pub fn get_min_baton_touch_count(&self) -> Option<u32> {
        match self {
            Condition::Movement {
                min_baton_touch_count,
                ..
            } => *min_baton_touch_count,
            Condition::Appearance {
                min_baton_touch_count,
                ..
            } => *min_baton_touch_count,
            Condition::Location {
                min_baton_touch_count,
                ..
            } => *min_baton_touch_count,
            _ => None,
        }
    }

    pub fn get_baton_touch_source(&self) -> Option<&str> {
        match self {
            Condition::Movement {
                baton_touch_source, ..
            } => baton_touch_source.as_deref(),
            Condition::Location { sub_checks, .. } => sub_checks
                .as_ref()
                .and_then(|sc| sc.baton_touch_source.as_deref()),
            _ => None,
        }
    }

    pub fn get_energy_placed(&self) -> Option<bool> {
        match self {
            Condition::Movement { energy_placed, .. } => *energy_placed,
            _ => None,
        }
    }

    pub fn get_turn_number(&self) -> Option<u32> {
        match self {
            Condition::Temporal { turn_number, .. } => *turn_number,
            _ => None,
        }
    }

    pub fn get_all(&self) -> Option<bool> {
        match self {
            Condition::Comparison { all, .. } => *all,
            Condition::State { all, .. } => *all,
            Condition::Location { all, .. } => *all,
            _ => None,
        }
    }

    pub fn get_area_direction(&self) -> Option<&str> {
        match self {
            Condition::Movement { area_direction, .. } => area_direction.as_deref(),
            _ => None,
        }
    }

    pub fn get_any_of(&self) -> Option<&[String]> {
        match self {
            Condition::AnyOf { any_of, .. } => any_of.as_deref(),
            _ => None,
        }
    }

    pub fn get_temporal_scope(&self) -> Option<&str> {
        match self {
            Condition::Temporal { temporal_scope, .. } => temporal_scope.as_deref(),
            _ => None,
        }
    }

    pub fn get_appearance(&self) -> Option<bool> {
        match self {
            Condition::Appearance { appearance, .. } => *appearance,
            _ => None,
        }
    }

    pub fn get_appearance_source(&self) -> Option<&str> {
        match self {
            Condition::Appearance {
                appearance_source, ..
            } => appearance_source.as_deref(),
            _ => None,
        }
    }

    pub fn get_all_members(&self) -> Option<bool> {
        match self {
            Condition::Group { all_members, .. } => *all_members,
            _ => None,
        }
    }

    pub fn get_comparison_target(&self) -> Option<ComparisonTarget> {
        match self {
            Condition::Comparison {
                comparison_target, ..
            } => *comparison_target,
            Condition::Location {
                comparison_target, ..
            } => *comparison_target,
            _ => None,
        }
    }

    pub fn get_cost_reference_character(&self) -> Option<&str> {
        match self {
            Condition::Appearance {
                cost_reference_character,
                ..
            } => cost_reference_character.as_deref(),
            _ => None,
        }
    }

    pub fn get_cost_reference_operator(&self) -> Option<&Operator> {
        match self {
            Condition::Appearance {
                cost_reference_operator,
                ..
            } => cost_reference_operator.as_ref(),
            _ => None,
        }
    }

    pub fn get_cost_total(&self) -> Option<u32> {
        match self {
            Condition::Comparison { cost_total, .. } => *cost_total,
            _ => None,
        }
    }

    pub fn get_cost_total_operator(&self) -> Option<&Operator> {
        match self {
            Condition::Comparison {
                cost_total_operator,
                ..
            } => cost_total_operator.as_ref(),
            _ => None,
        }
    }

    pub fn get_heart_type(&self) -> Option<&str> {
        match self {
            Condition::Location { heart_type, .. } => heart_type.as_deref(),
            _ => None,
        }
    }

    pub fn get_original_value(&self) -> Option<bool> {
        match self {
            Condition::Location { sub_checks, .. } => {
                sub_checks.as_ref().and_then(|sc| sc.original_value)
            }
            _ => None,
        }
    }

    pub fn get_reference_card(&self) -> Option<&str> {
        match self {
            Condition::Location { sub_checks, .. } => sub_checks
                .as_ref()
                .and_then(|sc| sc.reference_card.as_deref()),
            _ => None,
        }
    }

    pub fn get_require_position_cards(&self) -> Option<bool> {
        match self {
            Condition::Location {
                require_position_cards,
                ..
            } => *require_position_cards,
            _ => None,
        }
    }

    pub fn get_same_name(&self) -> Option<bool> {
        match self {
            Condition::Location { same_name, .. } => *same_name,
            _ => None,
        }
    }

    pub fn get_scope(&self) -> Option<&str> {
        match self {
            Condition::Location { scope, .. } => scope.as_deref(),
            Condition::Comparison { scope, .. } => scope.as_deref(),
            _ => None,
        }
    }

    pub fn get_values(&self) -> Option<&[u32]> {
        match self {
            Condition::Comparison { values, .. } => values.as_deref(),
            Condition::Location { sub_checks, .. } => {
                sub_checks.as_ref().and_then(|sc| sc.values.as_deref())
            }
            _ => None,
        }
    }

    pub fn get_yell_trigger(&self) -> Option<bool> {
        match self {
            Condition::Location { yell_trigger, .. } => *yell_trigger,
            _ => None,
        }
    }

    pub fn get_ability_filter_triggers(&self) -> Option<&[String]> {
        match self {
            Condition::AbilityFilter {
                ability_filter_triggers,
                ..
            } => ability_filter_triggers.as_deref(),
            Condition::Location { sub_checks, .. } => sub_checks
                .as_ref()
                .and_then(|sc| sc.ability_filter_triggers.as_deref()),
            _ => None,
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

    /// Total hearts this card has (printed hearts for member cards).
    ///
    /// Returns base_heart (printed hearts) for member cards, falling back to
    /// need_heart (live-card cost hearts) for live cards.
    ///
    /// Per QA Q149 (qa_data.json:1957-1958): when conditions check "ハートの総数",
    /// they count "メンバーが持つ基本ハート" (members' basic hearts), which is
    /// base_heart.  Per Q172 (lines 1405-1406): ability-granted hearts ARE
    /// included but blade hearts from yell are NOT ("基本ハートとエールで獲得した
    /// ブレードハート").  Note: this method returns only the printed value and
    /// does NOT include runtime heart_modifiers from GameModifiers.
    /// Rules 9.9.1.4→9.9.1.5 (rules.txt:1196-1212) defines the application order:
    /// printed base → set-to-value → add/subtract.
    pub fn total_hearts(&self) -> u32 {
        if let Some(ref base_heart) = self.base_heart {
            base_heart.hearts.values_sum()
        } else if let Some(ref need_heart) = self.need_heart {
            need_heart.hearts.values_sum()
        } else {
            0
        }
    }

    /// Total required hearts (sum of all need_heart values).
    ///
    /// This is the live card's cost hearts (not member base_heart).
    /// For member cards this always returns 0 since members don't have
    /// need_heart.  For condition checks involving members' printed hearts,
    /// use total_hearts() instead (per Q149: 基本ハート).
    pub fn need_heart_total(&self) -> u32 {
        self.need_heart
            .as_ref()
            .map(|nh| nh.hearts.values_sum())
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

    pub fn has_all_blade(&self) -> bool {
        self.blade_heart
            .as_ref()
            .is_some_and(|bh| bh.hearts.contains_key(&HeartColor::BAll))
    }

    /// Check if a given need_heart is satisfied by provided hearts.
    /// This is identical to satisfies_heart_requirement but allows an
    /// externally-adjusted need_heart (e.g. with modifiers applied).
    pub fn need_heart_satisfied(need: &BaseHeart, provided_hearts: &BaseHeart) -> bool {
        check_heart_requirement(need, provided_hearts)
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
    let total_provided: u32 = provided.hearts.values_sum();
    let total_required: u32 = need.hearts.values_sum();
    if total_provided < total_required {
        return false;
    }
    let wildcard_00 = provided
        .hearts
        .get(&HeartColor::Heart00)
        .copied()
        .unwrap_or(0);
    let wildcard_all = provided.hearts.get(&HeartColor::All).copied().unwrap_or(0);
    let mut wildcard_remaining = (wildcard_00 + wildcard_all) as i32;
    let mut remaining = provided.hearts.clone();
    for &(color, needed_amount) in &need.hearts {
        if color == HeartColor::Heart00 {
            continue;
        }
        let provided_val = remaining.get(&color).copied().unwrap_or(0) as i32;
        if provided_val + wildcard_remaining < needed_amount as i32 {
            return false;
        }
        let shortfall = (needed_amount as i32 - provided_val).max(0);
        wildcard_remaining -= shortfall;
        let consumed = needed_amount.min(remaining.get(&color).copied().unwrap_or(0));
        if let Some(rem) = remaining.get_mut(&color) {
            *rem -= consumed;
        }
    }
    if let Some(heart00_needed) = need.hearts.get(&HeartColor::Heart00).copied() {
        let leftover_sum: i32 = remaining
            .iter()
            .filter(|&(c, _)| c != &HeartColor::Heart00 && c != &HeartColor::All)
            .map(|(_, v)| *v as i32)
            .sum();
        if leftover_sum + wildcard_remaining.max(0) < heart00_needed as i32 {
            return false;
        }
    }
    true
}

impl core::fmt::Display for HeartColor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

impl core::str::FromStr for HeartColor {
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
            *base_heart.hearts.entry_or_default(color) += amount;
        }
    }

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
                if effect.action == crate::ability::enums::ActionType::ModifyCost
                    && effect.operation_any() == Some("subtract")
                    && Zone::from_str(effect.location_any().unwrap_or("")) == Some(Zone::Hand)
                    && effect.cost_limit_any().is_none()
                {
                    let per_unit = effect.per_unit_count_any().unwrap_or(1) as usize;
                    return (hand_size.saturating_sub(1) * per_unit) as u32;
                }
            }
        }
        0
    }
}
