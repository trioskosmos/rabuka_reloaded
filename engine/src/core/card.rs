use crate::ability::ability_store::AbilityRef;
pub(crate) use crate::ability::enums::{ActionType, ConditionType, EffectState};
use crate::core::types::ArcStr;
#[cfg(feature = "serde_support")]
use crate::BTreeMap;
use crate::HashMap;
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String, string::ToString, vec::Vec};

#[cfg(not(feature = "no_std"))]
pub(crate) use crate::core::pool::EkBox;
#[cfg(feature = "no_std")]
pub(crate) type EkBox = alloc::boxed::Box<EffectKind>;

pub(crate) fn ek_box_new(val: EffectKind) -> EkBox {
    #[cfg(not(feature = "no_std"))]
    {
        crate::core::pool::EkBox::new(val)
    }
    #[cfg(feature = "no_std")]
    {
        alloc::boxed::Box::new(val)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]

pub enum CardType {
    // Rule 4.1: Member cards are placed on the stage and used for performance
    Member,
    // Rule 4.2: Live cards are placed in Live Card Zone and used for live performance
    Live,
    // Rule 4.3: Energy cards are placed in Energy Zone and used to pay costs
    Energy,
}

impl CardType {
    pub fn from_card_str(s: &str) -> Option<Self> {
        match s {
            "member_card" => Some(CardType::Member),
            "live_card" => Some(CardType::Live),
            "energy_card" => Some(CardType::Energy),
            _ => None,
        }
    }
    pub fn as_card_str(&self) -> &'static str {
        match self {
            CardType::Member => "member_card",
            CardType::Live => "live_card",
            CardType::Energy => "energy_card",
        }
    }
}

#[cfg(feature = "serde_support")]
impl serde::Serialize for CardType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_card_str())
    }
}

#[cfg(feature = "serde_support")]
impl<'de> serde::Deserialize<'de> for CardType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        match s.as_str() {
            "member_card" | "メンバー" => Ok(CardType::Member),
            "live_card" | "ライブ" => Ok(CardType::Live),
            "energy_card" | "エネルギー" => Ok(CardType::Energy),
            other => Err(serde::de::Error::custom(format!(
                "unknown card_type: {}",
                other
            ))),
        }
    }
}

impl core::fmt::Display for CardType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_card_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum HeartColor {
    #[cfg_attr(feature = "serde_support", serde(rename = "heart00", alias = "heart0"))]
    // Index 0 - COLORLESS heart (heart_00.png, rule 2.1.1.2: "a heart icon that
    // does not specify a color"). A colorless heart can ONLY satisfy the heart0
    // requirement bucket of a live card (the total-count bucket of rule 2.11.3);
    // it can NEVER be used as a specific color (heart01-heart06). This is the
    // opposite of the icon_all heart (HeartColor::All, index 7, rule 2.1.1.3)
    // which CAN be treated as any one color during the performance check.
    Heart00,
    #[cfg_attr(feature = "serde_support", serde(rename = "heart01"))]
    Heart01,
    #[cfg_attr(feature = "serde_support", serde(rename = "heart02"))]
    Heart02,
    #[cfg_attr(feature = "serde_support", serde(rename = "heart03"))]
    Heart03,
    #[cfg_attr(feature = "serde_support", serde(rename = "heart04"))]
    Heart04,
    #[cfg_attr(feature = "serde_support", serde(rename = "heart05"))]
    Heart05,
    #[cfg_attr(feature = "serde_support", serde(rename = "heart06"))]
    Heart06,
    #[cfg_attr(feature = "serde_support", serde(rename = "b_all"))]
    BAll, // Blade heart wildcard
    #[cfg_attr(feature = "serde_support", serde(rename = "draw"))]
    Draw, // Special heart type for drawing cards
    #[cfg_attr(feature = "serde_support", serde(rename = "score"))]
    Score, // Special heart type for score bonus
    #[cfg_attr(feature = "serde_support", serde(rename = "all"))]
    All, // All-heart (icon_all.png) — can be treated as any one color during performance check
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum BladeColor {
    #[cfg_attr(feature = "serde_support", serde(rename = "桃"))]
    Peach,
    #[cfg_attr(feature = "serde_support", serde(rename = "赤"))]
    Red,
    #[cfg_attr(feature = "serde_support", serde(rename = "黄"))]
    Yellow,
    #[cfg_attr(feature = "serde_support", serde(rename = "緑"))]
    Green,
    #[cfg_attr(feature = "serde_support", serde(rename = "青"))]
    Blue,
    #[cfg_attr(feature = "serde_support", serde(rename = "紫"))]
    Purple,
    #[cfg_attr(feature = "serde_support", serde(rename = "all"))]
    All, // All blade types
}

// Rule 11: Keywords
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct HeartIcon {
    pub color: HeartColor,
    pub count: u8,
}

/// Efficient map of HeartColor→u32, backed by SmallVec (1-4 entries typical).
/// Serializes/deserializes as a flat JSON object (e.g. `{"heart01": 1, "heart03": 1}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeartMap(SmallVec<[(HeartColor, u8); 4]>);

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
    pub fn values_sum(&self) -> u8 {
        self.0.iter().map(|(_, v)| v).sum()
    }
    pub fn get(&self, color: &HeartColor) -> Option<&u8> {
        self.0.iter().find(|(c, _)| c == color).map(|(_, v)| v)
    }
    pub fn get_mut(&mut self, color: &HeartColor) -> Option<&mut u8> {
        self.0.iter_mut().find(|(c, _)| c == color).map(|(_, v)| v)
    }
    pub fn contains_key(&self, color: &HeartColor) -> bool {
        self.0.iter().any(|(c, _)| c == color)
    }
    pub fn insert(&mut self, color: HeartColor, val: u8) {
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
    pub fn entry_or_default(&mut self, color: HeartColor) -> &mut u8 {
        let idx = self.0.iter().position(|(c, _)| c == &color);
        if let Some(i) = idx {
            &mut self.0[i].1
        } else {
            self.0.push((color, 0));
            &mut self.0.last_mut().unwrap().1
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &(HeartColor, u8)> {
        self.0.iter()
    }
    pub fn keys(&self) -> impl Iterator<Item = &HeartColor> {
        self.0.iter().map(|(c, _)| c)
    }
    pub fn values(&self) -> impl Iterator<Item = &u8> {
        self.0.iter().map(|(_, v)| v)
    }
}

impl core::ops::Index<&HeartColor> for HeartMap {
    type Output = u8;
    fn index(&self, color: &HeartColor) -> &u8 {
        self.get(color).unwrap_or(&0)
    }
}

impl core::ops::IndexMut<&HeartColor> for HeartMap {
    fn index_mut(&mut self, color: &HeartColor) -> &mut u8 {
        self.entry_or_default(*color)
    }
}

impl<'a> IntoIterator for &'a HeartMap {
    type Item = &'a (HeartColor, u8);
    type IntoIter = core::slice::Iter<'a, (HeartColor, u8)>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl From<HashMap<HeartColor, u8>> for HeartMap {
    fn from(map: HashMap<HeartColor, u8>) -> Self {
        HeartMap(map.into_iter().collect())
    }
}

impl Default for HeartMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "serde_support")]
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

#[cfg(feature = "serde_support")]
impl<'de> Deserialize<'de> for HeartMap {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[cfg_attr(feature = "serde_support", derive(Deserialize))]
        struct Raw {
            // BTreeMap: deterministic iteration order (std HashMap uses a
            // random per-process seed, making baked output nondeterministic).
            #[cfg_attr(feature = "serde_support", serde(flatten))]
            hearts: BTreeMap<String, u32>,
        }
        let raw = Raw::deserialize(deserializer)?;
        let hearts = raw
            .hearts
            .into_iter()
            .map(|(k, v)| (parse_heart_color(&k), v as u8))
            .collect();
        Ok(HeartMap(hearts))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct BladeHeart {
    #[cfg_attr(feature = "serde_support", serde(flatten))]
    pub hearts: HeartMap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct BaseHeart {
    #[cfg_attr(feature = "serde_support", serde(flatten))]
    pub hearts: HeartMap,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize))]
pub struct Card {
    pub card_no: ArcStr,
    #[cfg(not(feature = "compact_cards"))]
    pub img: Option<ArcStr>,
    pub name: ArcStr,
    #[cfg(not(feature = "compact_cards"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub product: Box<str>,
    #[cfg_attr(feature = "serde_support", serde(rename = "type"))]
    pub card_type: CardType,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub series: Box<str>,
    #[cfg_attr(
        feature = "serde_support",
        serde(default = "default_group_from_series")
    )]
    pub group: Box<str>,
    pub unit: Option<ArcStr>,
    pub cost: Option<u8>,
    pub base_heart: Option<BaseHeart>,
    pub blade_heart: Option<BladeHeart>,
    #[cfg_attr(feature = "serde_support", serde(default = "default_blade"))]
    pub blade: u8,
    #[cfg(not(feature = "compact_cards"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub rare: Box<str>,
    #[cfg(not(feature = "compact_cards"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub ability: Box<str>,
    #[cfg(not(feature = "compact_cards"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub faq: Vec<FAQEntry>,
    // Live card fields
    pub score: Option<u8>,
    pub need_heart: Option<BaseHeart>,
    pub special_heart: Option<SpecialHeart>,
    // Parsed abilities from abilities.json
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub abilities: Vec<AbilityRef>,
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
}

#[cfg(feature = "serde_support")]
impl<'de> Deserialize<'de> for Card {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Debug, Clone)]
        #[cfg_attr(feature = "serde_support", derive(Deserialize))]
        struct CardHelper {
            pub card_no: String,
            #[cfg(not(feature = "compact_cards"))]
            pub img: Option<ArcStr>,
            pub name: String,
            #[cfg(not(feature = "compact_cards"))]
            #[cfg_attr(feature = "serde_support", serde(default))]
            pub product: String,
            #[cfg_attr(feature = "serde_support", serde(rename = "type"))]
            pub card_type: CardType,
            #[cfg_attr(feature = "serde_support", serde(default))]
            pub series: String,
            pub unit: Option<ArcStr>,
            pub cost: Option<u8>,
            pub base_heart: Option<BaseHeart>,
            pub blade_heart: Option<BladeHeart>,
            #[cfg_attr(feature = "serde_support", serde(default = "default_blade"))]
            pub blade: u8,
            #[cfg(not(feature = "compact_cards"))]
            #[cfg_attr(feature = "serde_support", serde(default))]
            pub rare: String,
            #[cfg(not(feature = "compact_cards"))]
            #[cfg_attr(feature = "serde_support", serde(default))]
            pub ability: String,
            #[cfg(not(feature = "compact_cards"))]
            #[cfg_attr(feature = "serde_support", serde(default))]
            pub faq: Vec<FAQEntry>,
            pub score: Option<u8>,
            pub need_heart: Option<BaseHeart>,
            pub special_heart: Option<SpecialHeart>,
        }

        let helper = CardHelper::deserialize(deserializer)?;
        let group = map_series_to_group(&helper.series);

        Ok(Card {
            card_no: ArcStr::from(helper.card_no),
            #[cfg(not(feature = "compact_cards"))]
            img: helper.img,
            name: ArcStr::from(helper.name),
            #[cfg(not(feature = "compact_cards"))]
            product: helper.product.into(),
            card_type: helper.card_type,
            series: helper.series.into(),
            group,
            unit: helper.unit,
            cost: helper.cost,
            base_heart: helper.base_heart,
            blade_heart: helper.blade_heart,
            blade: helper.blade,
            #[cfg(not(feature = "compact_cards"))]
            rare: helper.rare.into(),
            #[cfg(not(feature = "compact_cards"))]
            ability: helper.ability.into(),
            #[cfg(not(feature = "compact_cards"))]
            faq: helper.faq,
            score: helper.score,
            need_heart: helper.need_heart,
            special_heart: helper.special_heart,
            abilities: Vec::new(),
        })
    }
}

#[cfg(feature = "serde_support")]
fn map_series_to_group(series: &str) -> Box<str> {
    match series {
        "ラブライブ！" => "μ's".into(),
        "ラブライブ！サンシャイン!!" => "Aqours".into(),
        "ラブライブ！虹ヶ咲学園スクールアイドル同好会" => "虹ヶ咲".into(),
        "ラブライブ！スーパースター!!" => "Liella!".into(),
        "蓮ノ空女学院スクールアイドルクラブ" | "ラブライブ！蓮ノ空女学院スクールアイドルクラブ" => {
            "蓮ノ空".into()
        }
        _ => Box::from(""),
    }
}

#[cfg(feature = "serde_support")]
fn default_blade() -> u8 {
    0
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct SpecialHeart {
    #[cfg_attr(feature = "serde_support", serde(flatten))]
    pub hearts: HeartMap,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Ability {
    #[cfg_attr(feature = "serde_support", serde(default = "default_empty_string"))]
    pub full_text: String,
    /// Trigger prefix stripped from `full_text` (e.g. "【自】"). Usually derived
    /// on demand from `full_text`; only populated directly when the source JSON
    /// carries an explicit, non-derivable value. `None` means "derive from
    /// `full_text`" — see `Ability::triggerless_text`.
    #[cfg_attr(
        feature = "serde_support",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub triggerless_text: Option<String>,
    pub triggers: Option<ArcStr>,
    pub use_limit: Option<u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub is_null: bool,
    pub cost: Option<Box<AbilityCost>>,
    pub effect: Option<Box<AbilityEffect>>,
    pub keywords: Option<Vec<Keyword>>,
}

#[cfg(feature = "serde_support")]
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
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(transparent))]
pub struct AbilityCost(pub AbilityEffect);

impl AbilityCost {
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

impl AbilityCost {
    /// Build a `CardFilter` containing the same 7 base filter fields that
    /// `AbilityEffect::filter_subset` exposes. Mirrors that method so cost
    /// handlers can use the same consolidation pattern as effect handlers.
    pub fn filter_subset(&self) -> crate::ability::util::CardFilter<'_> {
        // Single filter builder: delegate to CardFilter::from_effect so the
        // full field set (characters, need_heart_*, cost_total, distinct, ...)
        // is always honored. Previously this was a hand-maintained minimal
        // subset that silently dropped fields (e.g. `characters`), causing
        // condition/effect mismatches (CLEAN-G15/D20).
        crate::ability::util::CardFilter::from_effect(self)
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct CompoundBranch {
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub look_action: Option<Box<AbilityEffect>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub select_action: Option<Box<AbilityEffect>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub actions: Option<Vec<Box<AbilityEffect>>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub primary_effect: Option<Box<AbilityEffect>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub alternative_condition: Option<Box<Condition>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub result_condition: Option<Box<Condition>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub followup_action: Option<Box<AbilityEffect>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub optional_action: Option<Box<AbilityEffect>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub conditional_action: Option<Box<AbilityEffect>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub conditional_negation: Option<bool>,
}

/// One branch of an OR'd ability filter. At least one branch must match
/// for the card to pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct AbilityFilterBranch {
    pub ability_filter: Option<AbilityFilter>,
    pub ability_filter_triggers: Option<Vec<String>>,
}

/// Shared filter/targeting fields extracted from EffectKind variants.
/// Boxed into each variant to reduce enum size from ~544 to ~140 bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize))]
pub struct EffectFilter {
    pub card_type: Option<CardType>,
    pub exclude_self: Option<bool>,
    pub same_name: Option<bool>,
    pub same_unit_name: Option<bool>,
    pub group_names: Option<Box<Vec<String>>>,
    pub self_target: Option<bool>,
    pub location: Option<ArcStr>,
    pub heart_colors: Box<Vec<String>>,
    pub source: Option<ArcStr>,
    pub target: Option<ArcStr>,
    pub destination: Option<ArcStr>,
    pub characters: Option<Box<Vec<String>>>,
    pub exclude_characters: Option<Box<Vec<String>>>,
    pub exclude_group_names: Option<Box<Vec<String>>>,
    pub activation_position: Option<ArcStr>,
    pub original_value: Option<bool>,
    pub target_count: Option<u8>,
    pub per_unit: Option<bool>,
    pub per_unit_count: Option<u8>,
    pub per_unit_type: Option<ArcStr>,
    pub group_reference: Option<ArcStr>,
    pub state: Option<Box<EffectState>>,
    pub distinct: Option<Box<DistinctType>>,
    pub position: Option<Box<PositionInfo>>,
    pub negation: Option<bool>,
    pub per_unit_heart_colors: Box<Vec<String>>,
    pub cost_limit: Option<u8>,
    pub cost_limit_operator: Option<Operator>,
    /// Blade-count target filter (e.g. "ブレードを4つ以上持つ"). Whether it
    /// means the printed/original value or the CURRENT total is decided by the
    /// effect's `original_value` flag when building a CardFilter.
    pub blade_limit: Option<u8>,
    pub blade_limit_operator: Option<Operator>,
    /// Dynamic blade limit: when set, the effective blade_limit is
    /// (energy cards under the activating member) + blade_limit_offset (C5).
    pub blade_limit_from_energy_under: Option<bool>,
    /// Dynamic blade limit referenced to the member paid as a wait cost:
    /// effective blade_limit = (original blade of the member waited as the cost) + offset.
    pub blade_limit_from_cost_member: Option<bool>,
    /// The addend used when `blade_limit_from_energy_under` is set (the
    /// "1を足した数" +N in the text), so the JSON makes the +N explicit.
    pub blade_limit_offset: Option<u8>,
    /// C6 keep-N-shuffle-rest: both players each select up to `count` hand
    /// cards to keep; the OTHER hand cards shuffle under their own deck.
    pub keep_shuffle_under: Option<bool>,
    pub need_heart_color: Option<ArcStr>,
    pub need_heart_operator: Option<Operator>,
    pub need_heart_total: Option<u8>,
    pub cost_reference: Option<ArcStr>,
    pub cost_offset: Option<i8>,
    pub target_member: Option<ArcStr>,
    pub target_from_selection: Option<bool>,
    pub source_position: Option<ArcStr>,
    pub exclude_selected: Option<bool>,
    pub discard_remaining: Option<bool>,
    pub self_cost: Option<bool>,
    pub shuffle: Option<bool>,
    pub effect_constraint: Option<ArcStr>,
    pub sign: Option<ArcStr>,
    pub heart_type: Option<ArcStr>,
    pub alternative_count_type: Option<ArcStr>,
    pub blind: Option<bool>,
    pub picker: Option<ArcStr>,
    pub reveal: Option<bool>,
    pub choice_type: Option<ArcStr>,
    pub choice_options: Option<Box<Vec<String>>>,
    pub replaces_event: Option<ArcStr>,
    pub choice_based: Option<bool>,
    pub use_limit: Option<u8>,
    pub triggers: Option<ArcStr>,
    pub original_count: Option<u8>,
    pub original_operator: Option<Operator>,
    pub quoted_text: Option<Box<QuotedText>>,
    pub allow_occupied_stage: Option<bool>,
    pub state_change: Option<Box<EffectState>>,
    pub exclude_position: Option<ArcStr>,
    pub duration: Option<ArcStr>,
    pub dynamic_count: Option<Box<DynamicCount>>,
    pub filter_targets_by_heart_colors: Option<bool>,
    pub card_property: Option<ArcStr>,
    pub per_unit_location: Option<ArcStr>,
    pub card_names: Box<Vec<String>>,
    pub all: Option<bool>,
    pub requires_under_card: Option<bool>,
    pub optional: Option<bool>,
    pub cost_total: Option<u8>,
    pub cost_total_operator: Option<Operator>,
    pub activation_condition_parsed: Option<Box<Condition>>,
    pub action_by: Option<ArcStr>,
    pub trigger_type: Option<ArcStr>,
    pub repeat_limit: Option<u8>,
    pub ability_filter: Option<AbilityFilter>,
    pub multiple_targets: Option<bool>,
    pub operation: Option<ArcStr>,
    pub options: Option<Box<Vec<Box<AbilityEffect>>>>,
    pub name_constraint: Option<ArcStr>,
    pub name_constraint_source: Option<ArcStr>,
    pub ability_filter_triggers: Option<Box<Vec<String>>>,
    pub or_ability_filters: Option<Box<Vec<AbilityFilterBranch>>>,
    pub energy_count: Option<u8>,
    pub any_number: Option<bool>,
    pub custom_type: Option<ArcStr>,
    pub yell_source: Option<ArcStr>,
    pub require_all_heart_colors: Option<bool>,
    pub heart_color_count: Option<u8>,
    pub value: Option<u8>,
    pub per_group: Option<bool>,
    pub per_group_count: Option<u8>,
    pub placement_order: Option<PlacementOrder>,
    pub remainder_destination: Option<ArcStr>,
    pub remainder_placement_order: Option<PlacementOrder>,
    pub or_card_types: Option<Box<Vec<String>>>,
    pub exclude_heart_colors: Option<Box<Vec<String>>>,
    pub cost_from_revealed: Option<bool>,
    pub timing_condition: Option<ArcStr>,
    pub identities: Option<Box<Vec<String>>>,
    pub all_regions: Option<bool>,
    pub trigger_filter: Option<Box<Vec<String>>>,
    pub effect_type: Option<ArcStr>,
    pub timing: Option<ArcStr>,
    pub treat_as: Option<ArcStr>,
    pub question: Option<ArcStr>,
    pub answers: Option<Box<Vec<String>>>,
    pub choice_maker: Option<ArcStr>,
    pub cost_limit_min: Option<u8>,
    pub cost_limit_max: Option<u8>,
    pub count: Option<u8>,
    pub exclude_by_name_source: Option<ArcStr>,
    pub baton_touch_trigger: Option<bool>,
    pub is_reveal: Option<bool>,
    pub resource_on_select: Option<Box<AbilityEffect>>,
    pub replace_all: Option<bool>,
    pub resource: Option<ArcStr>,
    pub heart_colors_from_selected_card: Option<bool>,
    pub heart_color: Option<ArcStr>,
    pub ability_gain: Option<ArcStr>,
    pub ability_gain_trigger: Option<ArcStr>,
    pub gained_effect: Option<Box<AbilityEffect>>,
    pub ability_text: Option<ArcStr>,
    pub target_trigger: Option<ArcStr>,
    pub source_card: Option<ArcStr>,
    pub suppressed_trigger: Option<ArcStr>,
    pub option: Option<ArcStr>,
    pub alternative_effect: Option<Box<AbilityEffect>>,
    pub choice_condition: Option<Box<Condition>>,
    pub alternative_condition: Option<Box<Condition>>,
    pub restriction_type: Option<ArcStr>,
    pub restricted_destination: Option<ArcStr>,
    pub delayed: Option<bool>,
    pub phase: Option<ArcStr>,
    pub non_stackable: Option<bool>,
    pub heart_selection: Option<bool>,
    pub blade_type: Option<ArcStr>,
    pub choice: Option<bool>,
    pub lose_blade_hearts: Option<bool>,
    pub original_cost: Option<u8>,
    pub parenthetical: Option<Box<Vec<String>>>,
    pub resource_icon_count: Option<u8>,
    pub ref_value: Option<ArcStr>,
    pub ref_offset: Option<i8>,
    pub id: Option<ArcStr>,
    pub opponent_action: Option<Box<AbilityEffect>>,
}

/// Tagged union of effect action types. Every field lives on the shared
/// `EffectFilter`; each variant is just a marker carrying the filter box so the
/// action-type dispatch in `filter()`/`filter_mut()` keeps working.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize))]
pub enum EffectKind {
    #[default]
    None,
    MoveCards {
        filter: Option<Box<EffectFilter>>,
    },
    DrawCards {
        filter: Option<Box<EffectFilter>>,
    },
    SelectTarget {
        filter: Option<Box<EffectFilter>>,
    },
    LookReveal {
        filter: Option<Box<EffectFilter>>,
    },
    ModifyScore {
        filter: Option<Box<EffectFilter>>,
    },
    ModifyHearts {
        filter: Option<Box<EffectFilter>>,
    },
    GainResource {
        filter: Option<Box<EffectFilter>>,
    },
    ChangeState {
        filter: Option<Box<EffectFilter>>,
    },
    AbilityOp {
        filter: Option<Box<EffectFilter>>,
    },
    CompoundEffect {
        filter: Option<Box<EffectFilter>>,
    },
    RestrictionOp {
        filter: Option<Box<EffectFilter>>,
    },
    PositionOp {
        filter: Option<Box<EffectFilter>>,
    },
    MiscOp {
        filter: Option<Box<EffectFilter>>,
    },
    CustomOp {
        filter: Option<Box<EffectFilter>>,
    },
}

impl EffectKind {
    pub(crate) fn filter(&self) -> Option<&EffectFilter> {
        match self {
            EffectKind::None => None,
            EffectKind::MoveCards { filter, .. }
            | EffectKind::DrawCards { filter, .. }
            | EffectKind::SelectTarget { filter, .. }
            | EffectKind::LookReveal { filter, .. }
            | EffectKind::ModifyScore { filter, .. }
            | EffectKind::ModifyHearts { filter, .. }
            | EffectKind::GainResource { filter, .. }
            | EffectKind::ChangeState { filter, .. }
            | EffectKind::AbilityOp { filter, .. }
            | EffectKind::CompoundEffect { filter, .. }
            | EffectKind::RestrictionOp { filter, .. }
            | EffectKind::PositionOp { filter, .. }
            | EffectKind::MiscOp { filter, .. }
            | EffectKind::CustomOp { filter, .. } => filter.as_deref(),
        }
    }

    pub(crate) fn filter_mut(&mut self) -> Option<&mut EffectFilter> {
        match self {
            EffectKind::None => None,
            EffectKind::MoveCards { filter, .. }
            | EffectKind::DrawCards { filter, .. }
            | EffectKind::SelectTarget { filter, .. }
            | EffectKind::LookReveal { filter, .. }
            | EffectKind::ModifyScore { filter, .. }
            | EffectKind::ModifyHearts { filter, .. }
            | EffectKind::GainResource { filter, .. }
            | EffectKind::ChangeState { filter, .. }
            | EffectKind::AbilityOp { filter, .. }
            | EffectKind::CompoundEffect { filter, .. }
            | EffectKind::RestrictionOp { filter, .. }
            | EffectKind::PositionOp { filter, .. }
            | EffectKind::MiscOp { filter, .. }
            | EffectKind::CustomOp { filter, .. } => filter.as_deref_mut(),
        }
    }
}

/// Recursively re-populate EffectKind for a serialization-deserialized
/// AbilityEffect that lost its `kind` (because kind is #[serde(skip)]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct AbilityEffect {
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub text: ArcStr,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub action: ActionType,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub source: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub destination: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub count: Option<u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub target: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub condition: Option<Box<Condition>>,
    #[cfg_attr(feature = "serde_support", serde(flatten))]
    pub compound: Box<CompoundBranch>,
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub kind: Option<EkBox>,
    pub non_stackable: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub conditional: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub is_further: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub optional: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub max: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub effect_steps: Option<Vec<Box<AbilityEffect>>>,
}

impl AbilityEffect {
    /// Build EffectKind from an action string and the matching effect JSON.
    /// Constructs EffectKind directly without serde Deserialize.
    #[cfg(feature = "json_path_test")]
    pub(crate) fn kind_from_action(
        action: &str,
        effect_json: &serde_json::Value,
    ) -> Option<EffectKind> {
        let obj = effect_json.as_object()?;
        macro_rules! str_field {
            ($key:expr) => {
                obj.get($key)
                    .and_then(|v| v.as_str())
                    .map(|s| ArcStr::from(s))
            };
        }
        macro_rules! bool_field {
            ($key:expr) => {
                obj.get($key).and_then(|v| v.as_bool())
            };
        }
        macro_rules! u8_field {
            ($key:expr) => {
                obj.get($key).and_then(|v| v.as_u64()).map(|n| n as u8)
            };
        }
        macro_rules! i8_field {
            ($key:expr) => {
                obj.get($key).and_then(|v| v.as_i64()).map(|n| n as i8)
            };
        }
        macro_rules! str_vec_field {
            ($key:expr) => {
                obj.get($key).and_then(|v| v.as_array()).map(|arr| {
                    Box::new(
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect::<Vec<_>>(),
                    )
                })
            };
        }
        macro_rules! opt_str_vec_field {
            ($key:expr) => {
                str_vec_field!($key)
            };
        }
        macro_rules! effect_field {
            ($key:expr) => {
                obj.get($key).map(|v| {
                    Box::new(AbilityEffect {
                        text: v
                            .get("text")
                            .and_then(|t| t.as_str())
                            .map(|s| ArcStr::from(s))
                            .unwrap_or_default(),
                        ..Default::default()
                    })
                })
            };
        }

        let filter = || EffectFilter {
            card_type: None,
            exclude_self: bool_field!("exclude_self"),
            same_name: bool_field!("same_name"),
            same_unit_name: bool_field!("same_unit_name"),
            group_names: opt_str_vec_field!("group_names"),
            self_target: bool_field!("self_target"),
            location: str_field!("location"),
            heart_colors: str_vec_field!("heart_colors").unwrap_or_default(),
            source: str_field!("source"),
            target: str_field!("target"),
            destination: str_field!("destination"),
            characters: opt_str_vec_field!("characters"),
            exclude_characters: opt_str_vec_field!("exclude_characters"),
            exclude_group_names: opt_str_vec_field!("exclude_group_names"),
            activation_position: str_field!("activation_position"),
            original_value: bool_field!("original_value"),
            target_count: u8_field!("target_count"),
            per_unit: bool_field!("per_unit"),
            per_unit_count: u8_field!("per_unit_count"),
            per_unit_type: str_field!("per_unit_type"),
            group_reference: str_field!("group_reference"),
            state: obj
                .get("state")
                .and_then(|v| v.as_str())
                .map(|s| Box::new(EffectState::from_str(s))),
            distinct: None,
            position: obj.get("position").and_then(|v| match v {
                serde_json::Value::String(s) if !s.is_empty() => {
                    Some(Box::new(PositionInfo::String(s.clone())))
                }
                serde_json::Value::Object(m) => {
                    let pos = m.get("position").and_then(|p| p.as_str()).map(String::from);
                    let tgt = m.get("target").and_then(|t| t.as_str()).map(ArcStr::from);
                    Some(Box::new(PositionInfo::Struct {
                        position: pos.map(ArcStr::from),
                        target: tgt,
                    }))
                }
                _ => None,
            }),
            negation: bool_field!("negation"),
            per_unit_heart_colors: str_vec_field!("per_unit_heart_colors").unwrap_or_default(),
            cost_limit: u8_field!("cost_limit"),
            cost_limit_operator: obj
                .get("cost_limit_operator")
                .and_then(|v| v.as_str())
                .and_then(parse_operator),
            blade_limit: u8_field!("blade_limit"),
            blade_limit_operator: obj
                .get("blade_limit_operator")
                .and_then(|v| v.as_str())
                .and_then(parse_operator),
            blade_limit_from_energy_under: bool_field!("blade_limit_from_energy_under"),
            blade_limit_from_cost_member: bool_field!("blade_limit_from_cost_member"),
            blade_limit_offset: u8_field!("blade_limit_offset"),
            keep_shuffle_under: bool_field!("keep_shuffle_under"),
            need_heart_color: str_field!("need_heart_color"),
            need_heart_operator: obj
                .get("need_heart_operator")
                .and_then(|v| v.as_str())
                .and_then(parse_operator),
            need_heart_total: u8_field!("need_heart_total"),
            cost_reference: str_field!("cost_reference"),
            cost_offset: i8_field!("cost_offset"),
            target_member: str_field!("target_member"),
            target_from_selection: bool_field!("target_from_selection"),
            source_position: str_field!("source_position"),
            exclude_selected: bool_field!("exclude_selected"),
            discard_remaining: bool_field!("discard_remaining"),
            self_cost: bool_field!("self_cost"),
            shuffle: bool_field!("shuffle"),
            effect_constraint: str_field!("effect_constraint"),
            sign: str_field!("sign"),
            heart_type: str_field!("heart_type"),
            picker: str_field!("picker"),
            reveal: bool_field!("reveal"),
            replaces_event: str_field!("replaces_event"),
            use_limit: u8_field!("use_limit"),
            triggers: str_field!("triggers"),
            original_count: u8_field!("original_count"),
            alternative_count_type: str_field!("alternative_count_type"),
            blind: bool_field!("blind"),
            allow_occupied_stage: bool_field!("allow_occupied_stage"),
            state_change: obj
                .get("state_change")
                .and_then(|v| v.as_str())
                .map(|s| Box::new(EffectState::from_str(s))),
            exclude_position: str_field!("exclude_position"),
            original_operator: obj
                .get("original_operator")
                .and_then(|v| v.as_str())
                .and_then(parse_operator),
            quoted_text: None,
            duration: str_field!("duration"),
            dynamic_count: obj
                .get("dynamic_count")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .map(Box::new),
            filter_targets_by_heart_colors: bool_field!("filter_targets_by_heart_colors"),
            card_property: str_field!("card_property"),
            per_unit_location: str_field!("per_unit_location"),
            card_names: str_vec_field!("card_names").unwrap_or_default(),
            all: bool_field!("all"),
            requires_under_card: bool_field!("requires_under_card"),
            optional: bool_field!("optional"),
            cost_total: u8_field!("cost_total"),
            cost_total_operator: None,
            activation_condition_parsed: None,
            action_by: str_field!("action_by"),
            trigger_type: str_field!("trigger_type"),
            repeat_limit: u8_field!("repeat_limit"),
            ability_filter: None,
            multiple_targets: bool_field!("multiple_targets"),
            operation: str_field!("operation"),
            options: None,
            name_constraint: str_field!("name_constraint"),
            name_constraint_source: str_field!("name_constraint_source"),
            ability_filter_triggers: opt_str_vec_field!("ability_filter_triggers"),
            or_ability_filters: None,
            energy_count: u8_field!("energy_count").or_else(|| u8_field!("energy")),
            any_number: bool_field!("any_number"),
            custom_type: str_field!("custom_type"),
            yell_source: str_field!("yell_source"),
            require_all_heart_colors: bool_field!("require_all_heart_colors"),
            heart_color_count: u8_field!("heart_color_count"),
            value: u8_field!("value"),
            per_group: bool_field!("per_group"),
            per_group_count: u8_field!("per_group_count"),
            placement_order: None,
            remainder_destination: None,
            remainder_placement_order: None,
            or_card_types: opt_str_vec_field!("or_card_types"),
            exclude_heart_colors: opt_str_vec_field!("exclude_heart_colors"),
            cost_from_revealed: bool_field!("cost_from_revealed"),
            timing_condition: str_field!("timing_condition"),
            identities: opt_str_vec_field!("identities"),
            all_regions: bool_field!("all_regions"),
            trigger_filter: opt_str_vec_field!("trigger_filter"),
            effect_type: str_field!("effect_type"),
            timing: str_field!("timing"),
            treat_as: str_field!("treat_as"),
            question: str_field!("question"),
            answers: opt_str_vec_field!("answers"),
            choice_maker: str_field!("choice_maker"),
            choice_based: bool_field!("choice_based"),
            choice_options: opt_str_vec_field!("choice_options"),
            choice_type: str_field!("choice_type"),
            cost_limit_min: u8_field!("cost_limit_min"),
            cost_limit_max: u8_field!("cost_limit_max"),
            count: u8_field!("count"),
            exclude_by_name_source: str_field!("exclude_by_name_source"),
            baton_touch_trigger: bool_field!("baton_touch_trigger"),
            is_reveal: bool_field!("is_reveal"),
            resource_on_select: effect_field!("resource_on_select"),
            replace_all: bool_field!("replace_all"),
            resource: str_field!("resource"),
            heart_colors_from_selected_card: bool_field!("heart_colors_from_selected_card"),
            heart_color: str_field!("heart_color"),
            ability_gain: str_field!("ability_gain"),
            ability_gain_trigger: str_field!("ability_gain_trigger"),
            gained_effect: effect_field!("gained_effect"),
            ability_text: str_field!("ability_text"),
            target_trigger: str_field!("target_trigger"),
            source_card: str_field!("source_card"),
            suppressed_trigger: str_field!("suppressed_trigger"),
            option: str_field!("option"),
            alternative_effect: effect_field!("alternative_effect"),
            choice_condition: obj
                .get("choice_condition")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            alternative_condition: obj
                .get("alternative_condition")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            restriction_type: str_field!("restriction_type"),
            restricted_destination: str_field!("restricted_destination"),
            delayed: bool_field!("delayed"),
            phase: str_field!("phase"),
            non_stackable: bool_field!("non_stackable"),
            heart_selection: bool_field!("heart_selection"),
            blade_type: str_field!("blade_type"),
            choice: bool_field!("choice"),
            lose_blade_hearts: bool_field!("lose_blade_hearts"),
            original_cost: u8_field!("original_cost"),
            parenthetical: opt_str_vec_field!("parenthetical"),
            resource_icon_count: u8_field!("resource_icon_count"),
            ref_value: str_field!("ref_value"),
            ref_offset: i8_field!("ref_offset"),
            id: str_field!("id"),
            opponent_action: effect_field!("opponent_action"),
        };

        let a = action.to_lowercase();
        Some(match a.as_str() {
            "move_cards"
            | "discard_card"
            | "discard_until_count"
            | "place_energy_under_member"
            | "re_yell"
            | "shuffle"
            | "play_baton_touch"
            | "double_baton_touch" => EffectKind::MoveCards {
                filter: Some(Box::new(filter())),
            },
            "draw" | "draw_card" | "draw_until_count" => EffectKind::DrawCards {
                filter: Some(Box::new(filter())),
            },
            "select" | "select_cards" | "select_number" | "choose_target_player" => {
                EffectKind::SelectTarget {
                    filter: Some(Box::new(filter())),
                }
            }
            "look"
            | "look_at"
            | "reveal"
            | "reveal_effect"
            | "reveal_per_group"
            | "reveal_until_live_card"
            | "reveal_until_chosen_card"
            | "look_and_select" => EffectKind::LookReveal {
                filter: Some(Box::new(filter())),
            },
            "modify_score" => EffectKind::ModifyScore {
                filter: Some(Box::new(filter())),
            },
            "modify_required_hearts"
            | "modify_required_hearts_global"
            | "modify_required_hearts_success" => EffectKind::ModifyHearts {
                filter: Some(Box::new(filter())),
            },
            "gain_resource" | "pay_energy" => EffectKind::GainResource {
                filter: Some(Box::new(filter())),
            },
            "change_state" | "set_card_identity" | "set_card_identity_all_regions" => {
                EffectKind::ChangeState {
                    filter: Some(Box::new(filter())),
                }
            }
            "gain_ability"
            | "gain_ability_from_source"
            | "invalidate_ability"
            | "suppress_ability_trigger"
            | "activate_ability" => EffectKind::AbilityOp {
                filter: Some(Box::new(filter())),
            },
            "sequential"
            | "choice"
            | "repeat_procedure"
            | "conditional_alternative"
            | "conditional_on_optional"
            | "conditional_on_result" => EffectKind::CompoundEffect {
                filter: Some(Box::new(filter())),
            },
            "restriction"
            | "activation_restriction"
            | "modify_limit"
            | "all_blade_timing"
            | "reduce_live_card_set_limit" => EffectKind::RestrictionOp {
                filter: Some(Box::new(filter())),
            },
            "position_change" | "rotation" => EffectKind::PositionOp {
                filter: Some(Box::new(filter())),
            },
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
            | "modify_yell_count" => EffectKind::MiscOp {
                filter: Some(Box::new(filter())),
            },
            "custom" | "do_nothing" | "action_by" | "opponent_action" => EffectKind::CustomOp {
                filter: Some(Box::new(filter())),
            },
            "" => EffectKind::SelectTarget {
                filter: Some(Box::new(filter())),
            },
            _ => return None,
        })
    }
}

// Macro-generated getters for EffectKind fields — all fields now live on the
// shared EffectFilter, so only the filter_* getter macros are used.
macro_rules! filter_str_getter {
    ($name:ident, $field:ident) => {
        pub fn $name(&self) -> Option<&str> {
            self.kind
                .as_deref()?
                .filter()?
                .$field
                .as_ref()
                .map(|s| -> &str { s })
        }
    };
}

macro_rules! filter_u8_getter {
    ($name:ident, $field:ident) => {
        pub fn $name(&self) -> Option<u8> {
            self.kind.as_deref()?.filter()?.$field
        }
    };
}

macro_rules! filter_bool_getter {
    ($name:ident, $field:ident) => {
        pub fn $name(&self) -> Option<bool> {
            self.kind.as_deref()?.filter()?.$field
        }
    };
}

macro_rules! filter_copy_getter {
    ($name:ident, $ty:ident, $field:ident) => {
        pub fn $name(&self) -> Option<$ty> {
            self.kind.as_deref()?.filter()?.$field
        }
    };
}

macro_rules! filter_box_vec_ref_getter {
    ($name:ident, $field:ident) => {
        pub fn $name(&self) -> Option<&Vec<String>> {
            self.kind.as_deref()?.filter()?.$field.as_deref()
        }
    };
}

macro_rules! filter_opt_vec_ref_getter {
    ($name:ident, $field:ident) => {
        pub fn $name(&self) -> Option<&Vec<String>> {
            self.kind.as_deref()?.filter()?.$field.as_deref()
        }
    };
}

macro_rules! filter_setter {
    ($fn:ident, $field:ident: $ty:ty) => {
        pub fn $fn(&mut self, val: Option<$ty>) {
            if let Some(f) = self.kind.as_deref_mut().and_then(|k| k.filter_mut()) {
                f.$field = val;
            }
        }
    };
}

impl AbilityEffect {
    pub fn ability_filter_any(&self) -> Option<AbilityFilter> {
        self.kind.as_deref()?.filter()?.ability_filter
    }

    filter_opt_vec_ref_getter!(ability_filter_triggers_any, ability_filter_triggers);

    filter_str_getter!(ability_gain_any, ability_gain);

    filter_str_getter!(ability_gain_trigger_any, ability_gain_trigger);

    filter_str_getter!(ability_text_any, ability_text);

    pub fn activation_condition_parsed_any(&self) -> Option<&Box<Condition>> {
        self.kind
            .as_deref()?
            .filter()?
            .activation_condition_parsed
            .as_ref()
    }

    filter_str_getter!(activation_position_any, activation_position);

    filter_bool_getter!(all_any, all);
    filter_bool_getter!(requires_under_card_any, requires_under_card);

    filter_bool_getter!(all_regions_any, all_regions);

    filter_bool_getter!(allow_occupied_stage_any, allow_occupied_stage);

    filter_str_getter!(alternative_count_type_any, alternative_count_type);

    pub fn alternative_effect_any(&self) -> Option<&Box<AbilityEffect>> {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.alternative_effect.as_ref())
    }

    filter_opt_vec_ref_getter!(answers_any, answers);

    filter_bool_getter!(any_number_any, any_number);

    filter_str_getter!(custom_type_any, custom_type);
    filter_str_getter!(yell_source_any, yell_source);

    filter_u8_getter!(blade_limit_any, blade_limit);
    filter_bool_getter!(blade_limit_from_energy_under_any, blade_limit_from_energy_under);
    filter_bool_getter!(blade_limit_from_cost_member_any, blade_limit_from_cost_member);
    filter_u8_getter!(blade_limit_offset_any, blade_limit_offset);
    filter_bool_getter!(keep_shuffle_under_any, keep_shuffle_under);

    filter_copy_getter!(blade_limit_operator_any, Operator, blade_limit_operator);

    filter_str_getter!(blade_type_any, blade_type);

    filter_bool_getter!(blind_any, blind);

    pub fn card_names_any(&self) -> Option<&Vec<String>> {
        Some(&*self.kind.as_deref()?.filter()?.card_names)
    }

    filter_str_getter!(card_property_any, card_property);

    pub fn card_type_any(&self) -> Option<&CardType> {
        self.kind.as_deref()?.filter()?.card_type.as_ref()
    }

    filter_opt_vec_ref_getter!(characters_any, characters);

    filter_bool_getter!(choice_any, choice);

    filter_bool_getter!(choice_based_any, choice_based);

    filter_str_getter!(choice_maker_any, choice_maker);

    filter_box_vec_ref_getter!(choice_options_any, choice_options);

    filter_str_getter!(choice_type_any, choice_type);

    filter_bool_getter!(cost_from_revealed_any, cost_from_revealed);

    filter_u8_getter!(cost_limit_any, cost_limit);

    filter_u8_getter!(cost_limit_min_any, cost_limit_min);

    filter_copy_getter!(cost_limit_operator_any, Operator, cost_limit_operator);

    pub fn cost_offset_any(&self) -> Option<i8> {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.cost_offset)
    }

    filter_str_getter!(cost_reference_any, cost_reference);

    filter_u8_getter!(cost_total_any, cost_total);

    filter_copy_getter!(cost_total_operator_any, Operator, cost_total_operator);

    filter_bool_getter!(delayed_any, delayed);

    filter_bool_getter!(discard_remaining_any, discard_remaining);

    pub fn distinct_any(&self) -> Option<DistinctType> {
        self.kind.as_deref()?.filter()?.distinct.as_deref().copied()
    }

    filter_str_getter!(duration_any, duration);

    pub fn dynamic_count_any(&self) -> Option<&DynamicCount> {
        self.kind.as_deref()?.filter()?.dynamic_count.as_deref()
    }

    filter_str_getter!(effect_constraint_any, effect_constraint);

    filter_u8_getter!(energy_count_any, energy_count);

    filter_str_getter!(exclude_by_name_source_any, exclude_by_name_source);

    filter_opt_vec_ref_getter!(exclude_characters_any, exclude_characters);

    filter_opt_vec_ref_getter!(exclude_group_names_any, exclude_group_names);

    pub fn exclude_heart_colors_any(&self) -> &[String] {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.exclude_heart_colors.as_ref())
            .map(|b| b.as_slice())
            .unwrap_or(&[])
    }

    pub fn exclude_position_any(&self) -> Option<&str> {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.exclude_position.as_deref())
    }

    filter_bool_getter!(exclude_selected_any, exclude_selected);

    filter_bool_getter!(exclude_self_any, exclude_self);

    filter_bool_getter!(
        filter_targets_by_heart_colors_any,
        filter_targets_by_heart_colors
    );

    pub fn gained_effect_any(&self) -> Option<&Box<AbilityEffect>> {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.gained_effect.as_ref())
    }

    filter_opt_vec_ref_getter!(group_names_any, group_names);

    filter_str_getter!(group_reference_any, group_reference);

    filter_u8_getter!(heart_color_count_any, heart_color_count);

    pub fn heart_colors_any(&self) -> &[String] {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .map(|f| f.heart_colors.as_slice())
            .unwrap_or(&[])
    }

    filter_bool_getter!(heart_colors_from_selected_card_any, heart_colors_from_selected_card);

    filter_str_getter!(heart_color_any, heart_color);

    filter_bool_getter!(heart_selection_any, heart_selection);

    filter_str_getter!(heart_type_any, heart_type);

    filter_str_getter!(ref_value_any, ref_value);

    filter_str_getter!(id_any, id);

    filter_opt_vec_ref_getter!(identities_any, identities);

    filter_str_getter!(location_any, location);

    filter_bool_getter!(lose_blade_hearts_any, lose_blade_hearts);

    filter_bool_getter!(multiple_targets_any, multiple_targets);

    filter_str_getter!(name_constraint_any, name_constraint);

    filter_str_getter!(name_constraint_source_any, name_constraint_source);

    filter_str_getter!(need_heart_color_any, need_heart_color);

    filter_copy_getter!(need_heart_operator_any, Operator, need_heart_operator);

    filter_u8_getter!(need_heart_total_any, need_heart_total);

    filter_bool_getter!(negation_any, negation);

    filter_str_getter!(operation_any, operation);

    filter_bool_getter!(optional_any, optional);

    pub fn options_any(&self) -> Option<&Vec<Box<AbilityEffect>>> {
        self.kind.as_deref()?.filter()?.options.as_deref()
    }

    pub fn or_ability_filters_any(&self) -> Option<&Vec<AbilityFilterBranch>> {
        self.kind
            .as_deref()?
            .filter()?
            .or_ability_filters
            .as_deref()
    }

    filter_opt_vec_ref_getter!(or_card_types_any, or_card_types);

    filter_u8_getter!(original_count_any, original_count);

    filter_copy_getter!(original_operator_any, Operator, original_operator);

    filter_bool_getter!(original_value_any, original_value);

    filter_u8_getter!(per_group_count_any, per_group_count);

    filter_bool_getter!(per_unit_any, per_unit);

    filter_u8_getter!(per_unit_count_any, per_unit_count);

    pub fn per_unit_heart_colors_any(&self) -> &[String] {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .map(|f| f.per_unit_heart_colors.as_slice())
            .unwrap_or(&[])
    }

    filter_str_getter!(per_unit_location_any, per_unit_location);

    filter_str_getter!(per_unit_type_any, per_unit_type);

    filter_copy_getter!(placement_order_any, PlacementOrder, placement_order);
    filter_str_getter!(remainder_destination_any, remainder_destination);
    filter_copy_getter!(remainder_placement_order_any, PlacementOrder, remainder_placement_order);

    pub fn position_any(&self) -> Option<&PositionInfo> {
        self.kind.as_deref()?.filter()?.position.as_deref()
    }

    pub fn repeat_limit_any(&self) -> Option<u8> {
        self.kind.as_deref()?.filter()?.repeat_limit
    }

    filter_str_getter!(replaces_event_any, replaces_event);

    filter_bool_getter!(require_all_heart_colors_any, require_all_heart_colors);

    filter_str_getter!(resource_any, resource);

    filter_u8_getter!(resource_icon_count_any, resource_icon_count);

    pub fn resource_on_select_any(&self) -> Option<&Box<AbilityEffect>> {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.resource_on_select.as_ref())
    }

    filter_str_getter!(restricted_destination_any, restricted_destination);

    filter_str_getter!(restriction_type_any, restriction_type);

    filter_bool_getter!(reveal_any, reveal);

    filter_bool_getter!(same_unit_name_any, same_unit_name);
    filter_bool_getter!(same_name_any, same_name);

    filter_bool_getter!(self_cost_any, self_cost);

    filter_bool_getter!(self_target_any, self_target);

    filter_bool_getter!(shuffle_any, shuffle);

    filter_str_getter!(sign_any, sign);

    filter_str_getter!(source_card_any, source_card);

    filter_str_getter!(source_position_any, source_position);

    filter_str_getter!(source_any, source);

    filter_str_getter!(destination_any, destination);

    pub fn count_any(&self) -> Option<u8> {
        let filter_count = self
            .kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.count);
        filter_count.or(self.count)
    }

    pub fn target_any(&self) -> Option<&str> {
        let variant_target = self
            .kind
            .as_deref()?
            .filter()?
            .target
            .as_ref()
            .map(|s| -> &str { s });
        variant_target.or_else(|| self.target.as_deref())
    }

    pub fn state_any(&self) -> Option<&str> {
        self.kind
            .as_deref()?
            .filter()?
            .state
            .as_ref()
            .map(|s| s.as_str())
    }

    pub fn state_change_any(&self) -> Option<&str> {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.state_change.as_ref())
            .map(|s| s.as_str())
    }

    filter_str_getter!(suppressed_trigger_any, suppressed_trigger);

    filter_u8_getter!(target_count_any, target_count);

    filter_bool_getter!(target_from_selection_any, target_from_selection);

    filter_str_getter!(target_member_any, target_member);

    filter_str_getter!(target_trigger_any, target_trigger);

    filter_str_getter!(timing_any, timing);

    filter_str_getter!(timing_condition_any, timing_condition);

    filter_str_getter!(treat_as_any, treat_as);

    filter_opt_vec_ref_getter!(trigger_filter_any, trigger_filter);

    filter_str_getter!(trigger_type_any, trigger_type);

    filter_u8_getter!(value_any, value);

    pub fn effect_type_any(&self) -> Option<&str> {
        self.kind
            .as_deref()?
            .filter()?
            .effect_type
            .as_ref()
            .map(|s| -> &str { s })
    }

    pub fn action_by_any(&self) -> Option<&str> {
        self.kind
            .as_deref()?
            .filter()?
            .action_by
            .as_ref()
            .map(|s| -> &str { s })
    }

    filter_str_getter!(question_any, question);

    filter_u8_getter!(cost_limit_max_any, cost_limit_max);

    filter_bool_getter!(non_stackable_any, non_stackable);
}

impl AbilityEffect {
    pub fn set_card_names(&mut self, val: Vec<String>) {
        if let Some(f) = self.kind.as_deref_mut().and_then(|k| k.filter_mut()) {
            f.card_names = Box::new(val);
        }
    }
    pub fn set_group_names(&mut self, val: Option<Box<Vec<String>>>) {
        if let Some(f) = self.kind.as_deref_mut().and_then(|k| k.filter_mut()) {
            f.group_names = val;
        }
    }
    pub fn set_optional(&mut self, val: Option<bool>) {
        self.optional = val;
    }
    filter_setter!(set_energy_count, energy_count: u8);
    filter_setter!(set_per_unit, per_unit: bool);
    filter_setter!(set_per_unit_count, per_unit_count: u8);
    filter_setter!(set_per_unit_type, per_unit_type: ArcStr);
    filter_setter!(set_self_target, self_target: bool);
    filter_setter!(set_target_count, target_count: u8);
    filter_setter!(set_target_member, target_member: ArcStr);
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
    /// exclude_self) that effect handlers most commonly need. Also carries the
    /// current-blade filter ("ブレードをNつ以上持つ", no 元々) which the caller
    /// applies post-`matches()` via util::filter_current_blade (matches() has no
    /// modifier access). heart_colors are only treated as a target filter when
    /// `filter_targets_by_heart_colors` is set (a gained resource color must not
    /// become a target filter).
    pub fn filter_subset(&self) -> crate::ability::util::CardFilter<'_> {
        let original = self.original_value_any().unwrap_or(false);
        crate::ability::util::CardFilter {
            card_type: self.card_type_any().map(|ct| ct.as_card_str()),
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
            exclude_group_names: self.exclude_group_names_any().map(Vec::as_slice),
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
            distinct: self.distinct_any(),
            original_blade_limit: if original {
                self.blade_limit_any()
            } else {
                None
            },
            original_blade_operator: if original {
                self.blade_limit_operator_any().map(Operator::as_str)
            } else {
                None
            },
            current_blade_limit: if original {
                None
            } else {
                self.blade_limit_any()
            },
            current_blade_operator: if original {
                None
            } else {
                self.blade_limit_operator_any().map(Operator::as_str)
            },
            ..Default::default()
        }
    }

    /// Returns the count with a caller-provided default.
    pub fn count_or(&self, n: u8) -> u8 {
        self.count.unwrap_or(n)
    }

    pub fn value_or_count(&self, default: u8) -> u8 {
        self.value_any().or(self.count).unwrap_or(default)
    }

    /// Returns the first group name, if any.
    pub fn group_name(&self) -> Option<&str> {
        self.group_names_any()
            .and_then(|gn| gn.first().map(|s| s.as_str()))
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
        self.kind
            .as_deref()?
            .filter()?
            .action_by
            .as_ref()
            .map(|s| -> &str { s })
    }

    pub fn opponent_action(&self) -> Option<&AbilityEffect> {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.opponent_action.as_deref())
    }

    pub fn effect_type(&self) -> Option<&str> {
        self.kind
            .as_deref()?
            .filter()?
            .effect_type
            .as_ref()
            .map(|s| -> &str { s })
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(untagged))]
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct DynamicCount {
    #[cfg_attr(feature = "serde_support", serde(rename = "type"))]
    pub count_type: String,
    pub reference: Option<ArcStr>,
    pub mode: Option<ArcStr>,
    pub base_reference: Option<ArcStr>,
    pub calculation: Option<ArcStr>,
    pub calculation_value: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize))]
pub struct QuotedText {
    pub text: String,
    pub quoted_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(untagged))]
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct PositionCharacter {
    pub position: String,
    pub character: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum CardState {
    #[cfg_attr(feature = "serde_support", serde(rename = "active"))]
    Active,
    #[cfg_attr(feature = "serde_support", serde(rename = "wait"))]
    Wait,
}

impl CardState {
    pub fn as_str(self) -> &'static str {
        match self {
            CardState::Active => "active",
            CardState::Wait => "wait",
        }
    }

    pub fn from_str(s: &str) -> CardState {
        match s {
            "active" => CardState::Active,
            _ => CardState::Wait,
        }
    }
}

impl_deref_str!(CardState);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum ComparisonTarget {
    #[cfg_attr(feature = "serde_support", serde(rename = "self"))]
    Self_,
    #[cfg_attr(feature = "serde_support", serde(rename = "opponent"))]
    Opponent,
}

impl ComparisonTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            ComparisonTarget::Self_ => "self",
            ComparisonTarget::Opponent => "opponent",
        }
    }

    pub fn from_str(s: &str) -> ComparisonTarget {
        match s {
            "opponent" => ComparisonTarget::Opponent,
            _ => ComparisonTarget::Self_,
        }
    }
}

impl_deref_str!(ComparisonTarget);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum CardProperty {
    #[cfg_attr(feature = "serde_support", serde(rename = "has_blade_heart"))]
    HasBladeHeart,
    #[cfg_attr(feature = "serde_support", serde(rename = "has_score_icon"))]
    HasScoreIcon,
    #[cfg_attr(feature = "serde_support", serde(rename = "has_all_blade"))]
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

    pub fn from_str(s: &str) -> CardProperty {
        match s {
            "has_score_icon" => CardProperty::HasScoreIcon,
            "has_all_blade" => CardProperty::HasAllBlade,
            _ => CardProperty::HasBladeHeart,
        }
    }
}

impl_deref_str!(CardProperty);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum PlacementOrder {
    #[cfg_attr(feature = "serde_support", serde(rename = "any_order"))]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum DistinctType {
    #[cfg_attr(feature = "serde_support", serde(rename = "card_name"))]
    CardName,
    #[cfg_attr(feature = "serde_support", serde(rename = "true"))]
    True,
    #[cfg_attr(feature = "serde_support", serde(rename = "distinct"))]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum Operator {
    #[cfg_attr(feature = "serde_support", serde(rename = ">="))]
    Gte,
    #[cfg_attr(feature = "serde_support", serde(rename = "<="))]
    Lte,
    #[cfg_attr(feature = "serde_support", serde(rename = ">"))]
    Gt,
    #[cfg_attr(feature = "serde_support", serde(rename = "<"))]
    Lt,
    #[cfg_attr(feature = "serde_support", serde(rename = "=", alias = "=="))]
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

pub(crate) fn parse_operator(s: &str) -> Option<Operator> {
    Some(match s {
        ">=" => Operator::Gte,
        "<=" => Operator::Lte,
        ">" => Operator::Gt,
        "<" => Operator::Lt,
        "=" | "==" => Operator::Eq,
        _ => return None,
    })
}

impl_deref_str!(Operator);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum ComparisonType {
    #[cfg_attr(feature = "serde_support", serde(rename = "score"))]
    Score,
    #[cfg_attr(feature = "serde_support", serde(rename = "cost"))]
    Cost,
    #[cfg_attr(feature = "serde_support", serde(rename = "count"))]
    Count,
    #[cfg_attr(feature = "serde_support", serde(rename = "equality"))]
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

    pub fn from_str(s: &str) -> ComparisonType {
        match s {
            "cost" => ComparisonType::Cost,
            "count" => ComparisonType::Count,
            "equality" => ComparisonType::Equality,
            _ => ComparisonType::Score,
        }
    }
}

impl_deref_str!(ComparisonType);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum AbilityFilter {
    #[cfg_attr(feature = "serde_support", serde(rename = "no_ability"))]
    NoAbility,
    #[cfg_attr(feature = "serde_support", serde(rename = "has_ability"))]
    HasAbility,
    #[cfg_attr(feature = "serde_support", serde(rename = "has_ability_type"))]
    HasAbilityType,
    #[cfg_attr(feature = "serde_support", serde(rename = "no_ability_type"))]
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

    pub fn from_str(s: &str) -> AbilityFilter {
        match s {
            "has_ability" => AbilityFilter::HasAbility,
            "has_ability_type" => AbilityFilter::HasAbilityType,
            "no_ability_type" => AbilityFilter::NoAbilityType,
            _ => AbilityFilter::NoAbility,
        }
    }
}

impl_deref_str!(AbilityFilter);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum ConditionTarget {
    #[cfg_attr(feature = "serde_support", serde(rename = "self"))]
    Self_,
    #[cfg_attr(feature = "serde_support", serde(rename = "opponent"))]
    Opponent,
    #[cfg_attr(feature = "serde_support", serde(rename = "both"))]
    Both,
    #[cfg_attr(feature = "serde_support", serde(rename = "either"))]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum ConditionCardType {
    #[cfg_attr(feature = "serde_support", serde(rename = "member_card"))]
    MemberCard,
    #[cfg_attr(feature = "serde_support", serde(rename = "live_card"))]
    LiveCard,
    #[cfg_attr(feature = "serde_support", serde(rename = "energy_card"))]
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

    pub fn from_str(s: &str) -> ConditionCardType {
        match s {
            "live_card" => ConditionCardType::LiveCard,
            "energy_card" => ConditionCardType::EnergyCard,
            _ => ConditionCardType::MemberCard,
        }
    }
}

impl_deref_str!(ConditionCardType);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum Location {
    #[cfg_attr(feature = "serde_support", serde(rename = "stage", alias = "ステージ"))]
    Stage,
    #[cfg_attr(feature = "serde_support", serde(rename = "hand"))]
    Hand,
    #[cfg_attr(feature = "serde_support", serde(rename = "deck"))]
    Deck,
    #[cfg_attr(feature = "serde_support", serde(rename = "deck_top"))]
    DeckTop,
    #[cfg_attr(feature = "serde_support", serde(rename = "discard"))]
    Discard,
    #[cfg_attr(feature = "serde_support", serde(rename = "energy_zone"))]
    EnergyZone,
    #[cfg_attr(feature = "serde_support", serde(rename = "live_card_zone"))]
    LiveCardZone,
    #[cfg_attr(
        feature = "serde_support",
        serde(rename = "success_live_card_zone", alias = "success_live_zone")
    )]
    SuccessLiveZone,
    #[cfg_attr(feature = "serde_support", serde(rename = "under_member"))]
    UnderMember,
    #[cfg_attr(feature = "serde_support", serde(rename = "revealed_cards"))]
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

/// Shared fields with consistent types across 2+ Condition variants,
/// inlined via #[serde(flatten)] so the JSON condition shape stays flat.
/// Fields with conflicting types across variants (state, ability_filter)
/// stay on their variants.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct ConditionCommon {
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub ability_filter_triggers: Option<Box<Vec<String>>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub activation_position: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub aggregate: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub all: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub all_areas: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub baton_touch_trigger: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub blade_greater_than_all: Option<bool>,
    pub cache: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub card_property: Option<CardProperty>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub card_type: Option<ConditionCardType>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub characters: Option<Box<Vec<String>>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub comparison_target: Option<ComparisonTarget>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub comparison_type: Option<ComparisonType>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub cost_limit: Option<u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub cost_limit_operator: Option<Operator>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub count: Option<u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub delta: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub destination: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub distinct: Option<Box<DistinctInfo>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub exclude_characters: Option<Box<Vec<String>>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub exclude_group_names: Option<Box<Vec<String>>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub exclude_self: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub from_state: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub group_names: Option<Box<Vec<String>>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub heart_colors: Option<Box<Vec<String>>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub heart_source: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub location: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub locations: Option<Box<Vec<String>>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub min_baton_touch_count: Option<u8>,
    pub negation: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub no_excess_heart: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub operator: Option<ArcStr>,
    pub phase: Option<ArcStr>,
    pub phase_target: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub position: Option<Box<PositionInfo>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub position_compare: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub require_position_cards: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub resource_type: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub same_name: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub scope: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub self_target: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub source: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub target: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub temporal: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    #[cfg(feature = "debug_conditions")]
    pub text: Option<String>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub to_state: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    #[cfg(feature = "debug_conditions")]
    pub trigger_event: Option<Box<TriggerEvent>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub yell_trigger: Option<bool>,
}

/// The distinct Condition type as a serde internally-tagged enum.
/// The Python parser already emits `"type": "card_count_condition"` etc.
/// in the JSON — this enum consumes that tag directly via `#[serde(tag = "type")]`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(tag = "type"))]
pub enum Condition {
    #[cfg_attr(
        feature = "serde_support",
        serde(rename = "compound", alias = "or_condition")
    )]
    Compound {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
        conditions: Option<Vec<Box<Condition>>>,
    },

    #[cfg_attr(
        feature = "serde_support",
        serde(rename = "card_count_condition", alias = "location_condition")
    )]
    Location {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
        unit: Option<ArcStr>,
        group_reference: Option<ArcStr>,
        heart_type: Option<ArcStr>,
        state: Option<CardState>,
        sub_checks: Option<Box<LocationSubChecks>>,
    },

    #[cfg_attr(
        feature = "serde_support",
        serde(
            rename = "comparison_condition",
            alias = "both_condition",
            alias = "all_cost_comparison_condition",
            alias = "highest_cost_on_stage_condition"
        )
    )]
    Comparison {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
        values: Option<Box<Vec<u8>>>,
        cost_total: Option<u8>,
        cost_total_operator: Option<Operator>,
        comparison_source: Option<ArcStr>,
        state: Option<CardState>,
        ability_filter: Option<ArcStr>,
    },

    #[cfg_attr(
        feature = "serde_support",
        serde(
            rename = "movement_condition",
            alias = "not_moved",
            alias = "has_moved"
        )
    )]
    Movement {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
        movement: Option<ArcStr>,
        baton_touch_source: Option<ArcStr>,
        self_effect_only: Option<bool>,
        energy_placed: Option<bool>,
        area_direction: Option<ArcStr>,
        ability_filter: Option<AbilityFilter>,
    },

    #[cfg_attr(feature = "serde_support", serde(rename = "group_condition"))]
    Group {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
        all_members: Option<bool>,
    },

    #[cfg_attr(feature = "serde_support", serde(rename = "appearance_condition"))]
    Appearance {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
        appearance: Option<bool>,
        positions_characters: Option<Box<Vec<PositionCharacter>>>,
        cost_reference_character: Option<ArcStr>,
        cost_reference_operator: Option<Operator>,
        appearance_source: Option<ArcStr>,
    },

    #[cfg_attr(feature = "serde_support", serde(rename = "temporal_condition"))]
    Temporal {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
        turn_number: Option<u8>,
        temporal_scope: Option<ArcStr>,
        condition: Option<Box<Condition>>,
    },

    #[cfg_attr(
        feature = "serde_support",
        serde(
            rename = "state_condition",
            alias = "energy_state_condition",
            alias = "state_change_condition"
        )
    )]
    State {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
        state: Option<EffectState>,
        energy_state: Option<ArcStr>,
    },

    #[cfg_attr(
        feature = "serde_support",
        serde(rename = "resource_condition", alias = "card_blade_condition")
    )]
    Resource {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
    },

    #[cfg_attr(feature = "serde_support", serde(rename = "ability_filter_condition"))]
    AbilityFilter {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
        ability_filter: Option<AbilityFilter>,
    },

    #[cfg_attr(feature = "serde_support", serde(rename = "score_threshold_condition"))]
    ScoreThreshold {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
    },

    #[cfg_attr(
        feature = "serde_support",
        serde(rename = "choice_condition", alias = "position_change_condition")
    )]
    Choice {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
        options: Option<Box<Vec<Box<AbilityEffect>>>>,
    },

    #[cfg_attr(feature = "serde_support", serde(rename = "complex_condition"))]
    Complex {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
        cause: Option<Box<Condition>>,
        effect: Option<Box<AbilityEffect>>,
    },

    #[cfg_attr(feature = "serde_support", serde(rename = "position_condition"))]
    PositionCond {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
    },

    #[cfg_attr(feature = "serde_support", serde(rename = "opponent_choice_condition"))]
    OpponentChoice {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
    },

    #[cfg_attr(feature = "serde_support", serde(rename = "opponent_live_success"))]
    OpponentLiveSuccess {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
    },

    #[cfg_attr(feature = "serde_support", serde(rename = "no_excess_heart"))]
    NoExcessHeart {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
    },

    #[cfg_attr(
        feature = "serde_support",
        serde(
            rename = "otherwise_condition",
            alias = "action_success_condition",
            alias = "custom"
        )
    )]
    AlwaysTrue {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
    },

    #[cfg_attr(feature = "serde_support", serde(rename = "any_of_condition"))]
    AnyOf {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
        any_of: Option<Vec<String>>,
    },

    #[cfg_attr(
        feature = "serde_support",
        serde(rename = "all_revealed_match_heart_color")
    )]
    AllRevealedMatchHeartColor {
        #[cfg_attr(feature = "serde_support", serde(flatten))]
        common: Box<ConditionCommon>,
    },
}

/// Sub-checks that can appear on Location conditions.
/// Boxed together because they're rarely all present at once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(default))]
pub struct LocationSubChecks {
    pub card_property: Option<CardProperty>,
    pub baton_touch_trigger: Option<bool>,
    pub baton_touch_source: Option<ArcStr>,
    pub min_baton_touch_count: Option<u8>,
    pub ability_filter: Option<AbilityFilter>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub ability_filter_triggers: Option<Vec<String>>,
    pub aggregate: Option<ArcStr>,
    pub no_excess_heart: Option<bool>,
    pub original_value: Option<bool>,
    pub activation_position: Option<ArcStr>,
    pub unit: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub values: Option<Vec<u8>>,
    pub group_reference: Option<ArcStr>,
    pub reference_card: Option<ArcStr>,
}

impl Default for Condition {
    fn default() -> Self {
        Condition::AlwaysTrue {
            common: Box::new(ConditionCommon::default()),
        }
    }
}

// ============== Condition field accessor methods ==============

/// Generate the 4 common Condition getters from a single variant list.
/// These fields exist on ALL 20 Condition variants.
impl Condition {
    /// Access the shared fields (present on every variant).
    pub fn common(&self) -> Option<&ConditionCommon> {
        match self {
            Condition::Compound { common, .. } => Some(common.as_ref()),
            Condition::Location { common, .. } => Some(common.as_ref()),
            Condition::Comparison { common, .. } => Some(common.as_ref()),
            Condition::Movement { common, .. } => Some(common.as_ref()),
            Condition::Group { common, .. } => Some(common.as_ref()),
            Condition::Appearance { common, .. } => Some(common.as_ref()),
            Condition::Temporal { common, .. } => Some(common.as_ref()),
            Condition::State { common, .. } => Some(common.as_ref()),
            Condition::Resource { common, .. } => Some(common.as_ref()),
            Condition::AbilityFilter { common, .. } => Some(common.as_ref()),
            Condition::ScoreThreshold { common, .. } => Some(common.as_ref()),
            Condition::Choice { common, .. } => Some(common.as_ref()),
            Condition::Complex { common, .. } => Some(common.as_ref()),
            Condition::PositionCond { common, .. } => Some(common.as_ref()),
            Condition::OpponentChoice { common, .. } => Some(common.as_ref()),
            Condition::OpponentLiveSuccess { common, .. } => Some(common.as_ref()),
            Condition::NoExcessHeart { common, .. } => Some(common.as_ref()),
            Condition::AlwaysTrue { common, .. } => Some(common.as_ref()),
            Condition::AnyOf { common, .. } => Some(common.as_ref()),
            Condition::AllRevealedMatchHeartColor { common, .. } => Some(common.as_ref()),
        }
    }

    /// Mutable access to the shared fields.
    pub fn common_mut(&mut self) -> Option<&mut ConditionCommon> {
        match self {
            Condition::Compound { common, .. } => Some(common.as_mut()),
            Condition::Location { common, .. } => Some(common.as_mut()),
            Condition::Comparison { common, .. } => Some(common.as_mut()),
            Condition::Movement { common, .. } => Some(common.as_mut()),
            Condition::Group { common, .. } => Some(common.as_mut()),
            Condition::Appearance { common, .. } => Some(common.as_mut()),
            Condition::Temporal { common, .. } => Some(common.as_mut()),
            Condition::State { common, .. } => Some(common.as_mut()),
            Condition::Resource { common, .. } => Some(common.as_mut()),
            Condition::AbilityFilter { common, .. } => Some(common.as_mut()),
            Condition::ScoreThreshold { common, .. } => Some(common.as_mut()),
            Condition::Choice { common, .. } => Some(common.as_mut()),
            Condition::Complex { common, .. } => Some(common.as_mut()),
            Condition::PositionCond { common, .. } => Some(common.as_mut()),
            Condition::OpponentChoice { common, .. } => Some(common.as_mut()),
            Condition::OpponentLiveSuccess { common, .. } => Some(common.as_mut()),
            Condition::NoExcessHeart { common, .. } => Some(common.as_mut()),
            Condition::AlwaysTrue { common, .. } => Some(common.as_mut()),
            Condition::AnyOf { common, .. } => Some(common.as_mut()),
            Condition::AllRevealedMatchHeartColor { common, .. } => Some(common.as_mut()),
        }
    }

    pub fn get_negation(&self) -> Option<bool> {
        self.common().and_then(|c| c.negation)
    }

    pub fn get_phase(&self) -> Option<&str> {
        self.common().and_then(|c| c.phase.as_deref())
    }

    pub fn get_phase_target(&self) -> Option<&str> {
        self.common().and_then(|c| c.phase_target.as_deref())
    }

    pub fn get_cache(&self) -> Option<bool> {
        self.common().and_then(|c| c.cache)
    }
}
impl Condition {
    pub fn get_text(&self) -> Option<&str> {
        #[cfg(feature = "debug_conditions")]
        {
            self.common().and_then(|c| c.text.as_deref())
        }
        #[cfg(not(feature = "debug_conditions"))]
        {
            None
        }
    }

    pub fn get_trigger_event(&self) -> Option<&TriggerEvent> {
        #[cfg(feature = "debug_conditions")]
        {
            self.common().and_then(|c| c.trigger_event.as_deref())
        }
        #[cfg(not(feature = "debug_conditions"))]
        {
            None
        }
    }

    pub fn get_location(&self) -> Option<&str> {
        self.common().and_then(|c| c.location.as_deref())
    }

    pub fn get_locations(&self) -> Option<&[String]> {
        self.common().and_then(|c| c.locations.as_deref()).map(|v| v.as_slice())
    }

    pub fn get_target(&self) -> Option<&str> {
        self.common().and_then(|c| c.target.as_deref())
    }

    pub fn get_count(&self) -> Option<u8> {
        self.common().and_then(|c| c.count)
    }

    pub fn get_operator(&self) -> Option<&str> {
        self.common().and_then(|c| c.operator.as_deref())
    }

    pub fn get_card_type(&self) -> Option<ConditionCardType> {
        self.common().and_then(|c| c.card_type)
    }

    pub fn get_group_names(&self) -> Option<&[String]> {
        self.common().and_then(|c| c.group_names.as_deref()).map(|v| v.as_slice())
    }

    pub fn get_characters(&self) -> Option<&[String]> {
        self.common().and_then(|c| c.characters.as_deref()).map(|v| v.as_slice())
    }

    pub fn get_exclude_characters(&self) -> Option<&[String]> {
        self.common().and_then(|c| c.exclude_characters.as_deref()).map(|v| v.as_slice())
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
        self.common().and_then(|c| c.position.as_deref())
    }

    pub fn get_movement(&self) -> Option<&str> {
        match self {
            Condition::Movement { movement, .. } => movement.as_deref(),
            _ => None,
        }
    }

    pub fn get_temporal(&self) -> Option<&str> {
        self.common().and_then(|c| c.temporal.as_deref())
    }

    pub fn get_source(&self) -> Option<&str> {
        self.common()
            .and_then(|c| c.source.as_deref())
            .or_else(|| self.get_trigger_event()?.source.as_deref())
    }

    pub fn get_destination(&self) -> Option<&str> {
        self.common()
            .and_then(|c| c.destination.as_deref())
            .or_else(|| self.get_trigger_event()?.destination.as_deref())
    }

    pub fn get_from_state(&self) -> Option<&str> {
        self.common()
            .and_then(|c| c.from_state.as_deref())
            .or_else(|| self.get_trigger_event()?.from_state.as_deref())
    }

    pub fn get_to_state(&self) -> Option<&str> {
        self.common()
            .and_then(|c| c.to_state.as_deref())
            .or_else(|| self.get_trigger_event()?.to_state.as_deref())
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
        self.common().and_then(|c| c.heart_colors.as_deref()).map(|v| v.as_slice())
    }

    pub fn get_exclude_self(&self) -> Option<bool> {
        self.common().and_then(|c| c.exclude_self)
    }

    pub fn get_self_target(&self) -> Option<bool> {
        self.common().and_then(|c| c.self_target)
    }

    pub fn get_cost_limit(&self) -> Option<u8> {
        self.common().and_then(|c| c.cost_limit)
    }

    pub fn get_cost_limit_operator(&self) -> Option<Operator> {
        self.common().and_then(|c| c.cost_limit_operator)
    }

    pub fn get_comparison_type(&self) -> Option<&str> {
        self.common().and_then(|c| c.comparison_type.as_deref())
    }

    pub fn get_resource_type(&self) -> Option<&str> {
        self.common().and_then(|c| c.resource_type.as_deref())
    }

    pub fn get_card_property(&self) -> Option<CardProperty> {
        self.common().and_then(|c| c.card_property)
    }

    pub fn get_aggregate(&self) -> Option<&str> {
        self.common().and_then(|c| c.aggregate.as_deref())
    }

    pub fn get_ability_filter(&self) -> Option<&AbilityFilter> {
        match self {
            Condition::Movement { ability_filter, .. } => ability_filter.as_ref(),
            Condition::AbilityFilter { ability_filter, .. } => ability_filter.as_ref(),
            _ => None,
        }
    }

    pub fn get_baton_touch_trigger(&self) -> Option<bool> {
        self.common().and_then(|c| c.baton_touch_trigger)
    }

    pub fn get_energy_state(&self) -> Option<&str> {
        match self {
            Condition::State { energy_state, .. } => energy_state.as_deref(),
            _ => None,
        }
    }

    pub fn get_heart_source(&self) -> Option<&str> {
        self.common().and_then(|c| c.heart_source.as_deref())
    }

    pub fn get_exclude_group_names(&self) -> Option<&[String]> {
        self.common().and_then(|c| c.exclude_group_names.as_deref()).map(|v| v.as_slice())
    }

    pub fn get_distinct(&self) -> Option<&DistinctInfo> {
        self.common().and_then(|c| c.distinct.as_deref())
    }

    pub fn get_position_compare(&self) -> Option<&str> {
        self.common().and_then(|c| c.position_compare.as_deref())
    }

    pub fn get_delta(&self) -> Option<bool> {
        self.common().and_then(|c| c.delta)
    }
}

/// Rich description of what event triggers a condition.
/// Parser-produced documentary field. The engine reads from this when
/// the corresponding flat Condition field is absent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(default))]
pub struct TriggerEvent {
    #[cfg_attr(feature = "serde_support", serde(rename = "type"))]
    pub event_type: Option<ArcStr>,
    pub tense: Option<ArcStr>,
    pub location: Option<ArcStr>,
    pub source_character: Option<ArcStr>,
    pub source_group: Option<ArcStr>,
    pub cost_comparison: Option<CostComparison>,
    pub min_count: Option<u8>,
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(default))]
pub struct CostComparison {
    pub operator: Option<Operator>,
    pub relative_to: Option<ArcStr>,
    pub cost_limit: Option<u8>,
    pub cost_limit_operator: Option<Operator>,
}

impl Condition {
    /// Build a `CardFilter` containing the 7 base filter fields, mirroring
    /// `AbilityEffect::filter_subset` and `AbilityCost::filter_subset`.
    pub fn filter_subset(&self) -> crate::ability::util::CardFilter<'_> {
        let c = self.common();
        let card_type = c.and_then(|c| c.card_type);
        let group_names = c.and_then(|c| c.group_names.as_ref());
        let cost_limit = c.and_then(|c| c.cost_limit);
        let cost_operator = c
            .and_then(|c| c.cost_limit_operator.as_deref())
            .or_else(|| c.and_then(|c| c.operator.as_deref()));
        let characters = c.and_then(|c| c.characters.as_ref());
        let exclude_characters = c.and_then(|c| c.exclude_characters.as_ref());
        let exclude_group_names = c.and_then(|c| c.exclude_group_names.as_ref());
        crate::ability::util::CardFilter {
            card_type: card_type.map(|ct| ct.as_str()),
            group: group_names
                .and_then(|v| v.first())
                .map(|s| s.as_str()),
            cost_limit,
            cost_operator,
            characters: characters.map(|b| b.as_ref()),
            exclude_characters: exclude_characters.map(|b| b.as_ref()),
            exclude_self: None,
            exclude_group_names: exclude_group_names.map(|b| b.as_slice()),
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
            } => positions_characters.as_deref().map(|v| v.as_slice()),
            _ => None,
        }
    }

    pub fn get_activation_position(&self) -> Option<&str> {
        self.common()
            .and_then(|c| c.activation_position.as_deref())
            .or_else(|| match self {
                Condition::Location { sub_checks, .. } => sub_checks
                    .as_ref()
                    .and_then(|sc| sc.activation_position.as_deref()),
                _ => None,
            })
    }

    pub fn set_position(&mut self, pos: PositionInfo) {
        if let Some(c) = self.common_mut() {
            c.position = Some(Box::new(pos));
        }
    }

    pub fn set_activation_position(&mut self, act_pos: String) {
        if let Some(c) = self.common_mut() {
            c.activation_position = Some(act_pos.clone().into());
        }
        if let Condition::Location { sub_checks, .. } = self {
            let mut checks = sub_checks.take().unwrap_or_default();
            checks.activation_position = Some(act_pos.into());
            *sub_checks = Some(checks);
        }
    }

    pub fn set_group_names(&mut self, gns: Vec<String>) {
        if let Some(c) = self.common_mut() {
            c.group_names = Some(Box::new(gns));
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
            Condition::Choice { options, .. } => options.as_ref().map(|b| &b[..]),
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
        self.common()
            .and_then(|c| c.no_excess_heart)
            .or_else(|| match self {
                Condition::Location { sub_checks, .. } => {
                    sub_checks.as_ref().and_then(|sc| sc.no_excess_heart)
                }
                _ => None,
            })
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
        self.common().and_then(|c| c.all_areas)
    }

    pub fn get_min_baton_touch_count(&self) -> Option<u8> {
        self.common().and_then(|c| c.min_baton_touch_count)
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

    pub fn get_turn_number(&self) -> Option<u8> {
        match self {
            Condition::Temporal { turn_number, .. } => *turn_number,
            _ => None,
        }
    }

    pub fn get_all(&self) -> Option<bool> {
        self.common().and_then(|c| c.all)
    }

    pub fn get_blade_greater_than_all(&self) -> Option<bool> {
        self.common().and_then(|c| c.blade_greater_than_all)
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
        self.common().and_then(|c| c.comparison_target)
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

    pub fn get_cost_total(&self) -> Option<u8> {
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
        self.common().and_then(|c| c.require_position_cards)
    }

    pub fn get_same_name(&self) -> Option<bool> {
        self.common().and_then(|c| c.same_name)
    }

    pub fn get_scope(&self) -> Option<&str> {
        self.common().and_then(|c| c.scope.as_deref())
    }

    pub fn get_values(&self) -> Option<&[u8]> {
        match self {
            Condition::Comparison { values, .. } => values.as_deref().map(|v| v.as_slice()),
            Condition::Location { sub_checks, .. } => {
                sub_checks.as_ref().and_then(|sc| sc.values.as_deref())
            }
            _ => None,
        }
    }

    pub fn get_yell_trigger(&self) -> Option<bool> {
        self.common().and_then(|c| c.yell_trigger)
    }

    pub fn get_ability_filter_triggers(&self) -> Option<&[String]> {
        self.common()
            .and_then(|c| c.ability_filter_triggers.as_ref())
            .map(|b| &b[..])
            .or_else(|| match self {
                Condition::Location { sub_checks, .. } => sub_checks
                    .as_ref()
                    .and_then(|sc| sc.ability_filter_triggers.as_ref().map(|b| &b[..])),
                _ => None,
            })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct FAQEntry {
    pub title: String,
    pub question: String,
    pub answer: String,
    pub relation: Vec<CardRelation>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
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

    /// Iterate abilities, decoding each from bytecode on demand.
    /// Each call to the iterator decodes one ability. The returned `Arc`s
    /// are dropped when the iterator or their binding goes out of scope.
    pub fn resolved_abilities(&self) -> impl Iterator<Item = crate::Arc<Ability>> + '_ {
        self.abilities.iter().map(|ar| ar.resolve())
    }

    /// Raw ability text for frontend display. Returns `""` when compact_cards
    /// feature is enabled (the text is stripped from the struct).
    pub fn ability_text(&self) -> &str {
        #[cfg(not(feature = "compact_cards"))]
        {
            &self.ability
        }
        #[cfg(feature = "compact_cards")]
        {
            ""
        }
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
    pub fn total_hearts(&self) -> u8 {
        if let Some(ref base_heart) = self.base_heart {
            base_heart.hearts.values_sum()
        } else if let Some(ref need_heart) = self.need_heart {
            need_heart.hearts.values_sum()
        } else {
            0
        }
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

    pub fn need_heart_satisfied(need: &BaseHeart, provided_hearts: &BaseHeart) -> bool {
        check_heart_requirement(need, provided_hearts)
    }
}

pub fn check_heart_requirement(need: &BaseHeart, provided: &BaseHeart) -> bool {
    if need.hearts.is_empty() {
        return true;
    }
    let total_provided: u8 = provided.hearts.values_sum().into();
    let total_required: u8 = need.hearts.values_sum().into();
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

/// Single macro table for HeartColor variants. Generates Display, as_str,
/// short_label, index, from_index, and the core FromStr match arms.
macro_rules! heart_color_table {
    ($macro:ident) => {
        $macro! {
            Heart00 => "heart00", "h00", 0,
            Heart01 => "heart01", "h01", 1,
            Heart02 => "heart02", "h02", 2,
            Heart03 => "heart03", "h03", 3,
            Heart04 => "heart04", "h04", 4,
            Heart05 => "heart05", "h05", 5,
            Heart06 => "heart06", "h06", 6,
            BAll   => "b_all",   "b_all", 0,
            Draw   => "draw",    "draw",  0,
            Score  => "score",   "score", 0,
            All    => "all",     "all",   7,
        }
    };
}

impl core::fmt::Display for HeartColor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        macro_rules! disp {
            ($($variant:ident => $s:expr, $sl:expr, $idx:expr),+ $(,)?) => {
                match self {
                    $(HeartColor::$variant => f.write_str($s),)+
                }
            };
        }
        heart_color_table!(disp)
    }
}

impl HeartColor {
    pub fn index(&self) -> usize {
        macro_rules! idx {
            ($($variant:ident => $s:expr, $sl:expr, $idx:expr),+ $(,)?) => {
                match self {
                    $(HeartColor::$variant => $idx,)+
                }
            };
        }
        heart_color_table!(idx)
    }

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

    pub fn short_label(&self) -> &'static str {
        macro_rules! sl {
            ($($variant:ident => $s:expr, $sl:expr, $idx:expr),+ $(,)?) => {
                match self {
                    $(HeartColor::$variant => $sl,)+
                }
            };
        }
        heart_color_table!(sl)
    }

    pub fn as_str(&self) -> &'static str {
        macro_rules! as_s {
            ($($variant:ident => $s:expr, $sl:expr, $idx:expr),+ $(,)?) => {
                match self {
                    $(HeartColor::$variant => $s,)+
                }
            };
        }
        heart_color_table!(as_s)
    }
}

impl core::str::FromStr for HeartColor {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        macro_rules! from_s {
            ($($variant:ident => $str:expr, $sl:expr, $idx:expr),+ $(,)?) => {
                Ok(match s {
                    $($str => HeartColor::$variant,)+
                    // `b_heart07` is a blade heart that produces COLORLESS
                    // (heart0 / heart00) hearts. Mechanic: `b_heart07: N` grants
                    // 2×N colorless hearts (the ×2 is applied at conversion in
                    // live.rs / phases.rs). Colorless hearts can ONLY satisfy
                    // heart0 requirements — never a specific color.
                    "heart07" | "b_heart07" => HeartColor::Heart00,
                    _ if s.starts_with("b_") => {
                        HeartColor::from_str(&s[2..]).unwrap_or(HeartColor::Heart00)
                    }
                    _ => HeartColor::Heart00,
                })
            };
        }
        heart_color_table!(from_s)
    }
}

/// Canonical string→HeartColor conversion. Use `s.parse::<HeartColor>()` instead.
pub fn parse_heart_color(s: &str) -> HeartColor {
    s.parse().unwrap_or(HeartColor::Heart00)
}

impl Card {
    pub fn get_score(&self) -> u8 {
        self.score.unwrap_or(0)
    }
}
