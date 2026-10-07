use super::*;

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
            .map(|(k, v)| (parse_heart_color(&k), u8::try_from(v).unwrap_or(0)))
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
    /// Canonical, equality-only form of `name`: whitespace stripped and a
    /// full-width '＆' folded to '&'. Written by `cards/scrape_expansion.py`,
    /// backfilled by `cards/normalise_names.py`, and proven output-neutral for
    /// the ability corpus (regenerate + `git diff --numstat cards/abilities.json`
    /// shows only `generated_at` and `engine_commit`).
    ///
    /// `name` is the DISPLAY string and keeps its spacing — 'trial vol.1' must
    /// not become 'trialvol.1' — which is why the comparison form is a separate
    /// field rather than a normalised `name`. 792 of the 2639 names differ from
    /// their key (779 contain whitespace, 29 contain '＆'), so comparing against
    /// this instead of rebuilding the form is real work removed: name matching
    /// used to allocate four times per call and measured 2.99us/call on
    /// multi-name cards in a debug build.
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub name_key: ArcStr,
    #[cfg(not(feature = "compact_cards"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub product: Box<str>,
    #[cfg_attr(feature = "serde_support", serde(rename = "type"))]
    pub card_type: CardType,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub series: Box<str>,
    // `group` is derived from `series` by the manual
    // `Deserialize` impl below (`map_series_to_group`).
    // The struct itself derives only `Serialize`, which
    // ignores field defaults, so no serde attribute
    // belongs here (the old one referenced a function
    // that does not exist).
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

/// Typed card identity. Replaces bare `i16` card IDs so the compiler can
/// separate "a card ID" from "an arbitrary small integer" — IDs must be
/// explicitly converted before they can index arithmetic or narrow to other
/// types, which funnels every conversion into one auditable place.
///
/// Migration is incremental (see docs/CODE_AUDIT_2026-08-23.md QUEUE B2):
/// new APIs take/return `CardId`; legacy i16 call sites bridge via
/// [`CardId::from_raw`] / [`CardId::raw`]. The sentinel [`CardId::EMPTY`]
/// (-1) preserves existing "no card" conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct CardId(i16);

impl CardId {
    /// The "no card" placeholder (legacy -1).
    pub const EMPTY: CardId = CardId(-1);

    pub const fn from_raw(v: i16) -> Self {
        CardId(v)
    }

    pub const fn raw(self) -> i16 {
        self.0
    }

    /// True for both the EMPTY sentinel and any negative value (which has
    /// never been a valid ID).
    pub fn is_empty(self) -> bool {
        self.0 < 0
    }
}

impl From<i16> for CardId {
    fn from(v: i16) -> Self {
        CardId(v)
    }
}

/// For indexing into per-card arrays. Negative IDs (EMPTY) would wrap under
/// `as usize`; callers at array boundaries should check `is_empty()` first —
/// this impl deliberately does NOT hide that decision.
impl From<CardId> for usize {
    fn from(id: CardId) -> Self {
        id.0.usize_count()
    }
}

#[derive(Debug, Clone)]

pub struct CardDatabase {
    pub cards: HashMap<i16, Card>,
    pub card_no_to_id: HashMap<String, i16>,
    /// Normalized (uppercase, fullwidth→halfwidth) card_no → id. Built once at
    /// load so `get_card_id` fallback lookups are O(1) instead of an
    /// O(n)-allocation scan over every key.
    pub normalized_no_to_id: HashMap<String, i16>,
    /// Card ids carrying at least one **自動** ability. Built once at load; the AUTO scan
    /// consults it to skip whole cards whose abilities can never fire. 135 of the pool's
    /// 1574 members qualify, so a typical board skips most of them.
    pub auto_abilities: HashMap<i16, Vec<u16>>,
    /// Card ids carrying at least one **常時** ability. See
    /// [CardDatabase::has_constant_ability].
    pub constant_cards: HashSet<i16>,
    /// `is_live` and `is_energy` per card id, as a dense array indexed by the id.
    ///
    /// Card ids are assigned densely from 0, so a direct array index answers "is this a
    /// live card?" with no hashing at all — against the `get_card(id).is_some_and(..)`
    /// hash lookup it replaces.
    ///
    /// This exists because of `check_timing`, not for `is_live()` calls in general. That
    /// function runs ~550k times per 80-game sweep and its own frame measured 6.01 us/call
    /// — MORE than the `recalculate_constants` it wraps (3.52 us) — and most of that is
    /// repair steps that resolve every card in a zone through the database:
    /// `check_invalid_energy_cards` alone walks both energy zones on every call.
    ///
    /// Those steps are hot, not a rarely-taken safety net: skipping all five of them
    /// (`ab_flag("repair_steps")`) changed the game by 16% of its actions, so there is no
    /// empty-input shortcut available here and the only lever is making the work cheaper.
    ///
    /// Measured on identical games (`total_actions` equal in both arms): **-0.7%, -0.1%,
    /// -0.8%** over three interleaved runs. Each reading is inside noise; the consistent
    /// sign is the signal. That figure covers the energy-zone walk and the live-zone
    /// filter — the two sites that run on EVERY call. Extending the same lookup to
    /// `check_duplicate_members`, `check_orphaned_under_cards` and the resolution-zone
    /// cleanup measured WITHIN NOISE, because those loops only execute when there are
    /// pending duplicates or orphans to move.
    pub live_ids: Vec<bool>,
    pub energy_ids: Vec<bool>,
    pub next_id: i16,
}

impl CardDatabase {
    /// Is `card_id` a ライブ card? Array index; see [`Self::live_ids`].
    #[inline]
    pub fn is_live_card(&self, card_id: i16) -> bool {
        self.live_ids.get(card_id.max(0).usize_count()).copied() == Some(true)
    }

    /// Is `card_id` an energy card? Array index; see [`Self::energy_ids`].
    #[inline]
    pub fn is_energy_card(&self, card_id: i16) -> bool {
        self.energy_ids
            .get(card_id.max(0).usize_count())
            .copied() == Some(true)
    }

    fn index_card_types(&mut self, card_id: i16, card: &Card) {
        let i = card_id.max(0).usize_count();
        if i >= self.live_ids.len() {
            self.live_ids.resize(i + 1, false);
            self.energy_ids.resize(i + 1, false);
        }
        self.live_ids[i] = card.is_live();
        self.energy_ids[i] = card.is_energy();
    }

    pub fn new() -> Self {
        Self {
            cards: HashMap::default(),
            card_no_to_id: HashMap::default(),
            normalized_no_to_id: HashMap::default(),
            auto_abilities: HashMap::default(),
            constant_cards: HashSet::default(),
            live_ids: Vec::new(),
            energy_ids: Vec::new(),
            next_id: 0,
        }
    }

    /// The **自動** ability indices of a card, in ability order.
    ///
    /// The AUTO scan (`trigger_auto_abilities_for_player_with_event`) runs four times per
    /// card play and walked every ability of every card on the stage, resolving each one
    /// and string-comparing its trigger. Of the pool's 1574 members only **135** carry a
    /// 自動 at all, so most of that work could never produce a trigger. Indexing the
    /// INDICES as well as the cards takes the remaining per-card loop down to just the
    /// abilities that can fire, which removes the `triggers == "自動"` comparison from
    /// the innermost loop entirely.
    ///
    /// This halved the 自動 system's share of engine time: 706 ms of 14.0 s instrumented
    /// (5.0%) down to 277 ms (2.0%).
    #[inline]
    pub fn auto_ability_indices(&self, card_id: i16) -> &[u16] {
        self.auto_abilities
            .get(&card_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Does this card carry ANY **自動** ability? O(1) form of
    /// [`Self::auto_ability_indices`], for callers that only need the boolean.
    #[inline]
    pub fn has_auto_ability(&self, card_id: i16) -> bool {
        self.auto_abilities.contains_key(&card_id)
    }

    /// Does this card carry ANY **常時** ability?
    ///
    /// Used by the `recalculate_constants` fast path. 255 cards carry a 常時 and a
    /// 40-card deck averages 3.9 of them, but a deck is not a board: only the stage
    /// slots, hands and live zones are in play, so most games have a 常時 on the board
    /// only a fraction of the time. `recalculate_constants` runs ~520k times per 80-game
    /// bench at 4.7 µs each, and with no 常時 in play it has nothing to derive.
    #[inline]
    pub fn has_constant_ability(&self, card_id: i16) -> bool {
        self.constant_cards.contains(&card_id)
    }

    fn index_constant_ability(&mut self, card_id: i16, card: &Card) {
        if card
            .abilities
            .iter()
            .any(|ar| ar.resolve().triggers.as_deref() == Some(crate::triggers::CONSTANT))
        {
            self.constant_cards.insert(card_id);
        }
    }

    /// `resolve()` here, not `peek()`: at load time no decode slot has been populated, so
    /// `peek()` is `None` for every ability and the index would come out EMPTY — which
    /// reads as a clean index and silently disables every 自動. The `Arc` cost is
    /// irrelevant: this runs once per card at startup, not on a hot path.
    fn index_auto_ability(&mut self, card_id: i16, card: &Card) {
        let indices: Vec<u16> = card
            .abilities
            .iter()
            .enumerate()
            .filter(|(_, ar)| ar.resolve().triggers.as_deref() == Some(crate::triggers::AUTO))
            .map(|(i, _)| i as u16)
            .collect();
        if !indices.is_empty() {
            self.auto_abilities.insert(card_id, indices);
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
        if let Some(indices) = self.auto_abilities.get(&template_id).cloned() {
            self.auto_abilities.insert(copy_id, indices);
        }
        if self.has_constant_ability(template_id) {
            self.constant_cards.insert(copy_id);
        }
        // A copy has the template's TYPE, so it inherits the type bits — the same way it
        // inherits the ability indices above. Without this the new id reads as neither
        // live nor energy, and `check_invalid_energy_cards` would mill it straight back
        // out of the energy zone.
        let t = template_id.max(0).usize_count();
        let i = copy_id.max(0).usize_count();
        if i >= self.live_ids.len() {
            self.live_ids.resize(i + 1, false);
            self.energy_ids.resize(i + 1, false);
        }
        self.live_ids[i] = self.live_ids.get(t).copied() == Some(true);
        self.energy_ids[i] = self.energy_ids.get(t).copied() == Some(true);
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
            db.index_auto_ability(card_id, &card);
            db.index_constant_ability(card_id, &card);
            db.index_card_types(card_id, &card);
            db.cards.insert(card_id, card);
        }

        // R1-adjacent perf fix: pre-build the normalized lookup index once so
        // get_card_id's fallback is O(1) instead of an allocating scan over
        // every key on each miss.
        db.normalized_no_to_id = db
            .card_no_to_id
            .keys()
            .map(|k| {
                (
                    Self::normalize_card_no(k),
                    *db.card_no_to_id.get(k).unwrap(),
                )
            })
            .collect();

        db
    }

    pub fn get_card(&self, card_id: i16) -> Option<&Card> {
        self.cards.get(&card_id)
    }

    /// Look up a card by its printed number.
    ///
    /// Thin wrapper over [`Self::get_card_id`], which owns the resolution
    /// algorithm. This used to be a second, hand-maintained copy of the same
    /// five steps, and the two drifted: `get_card_id`'s "any rarity of this
    /// base" and "contains" fallbacks had been fixed to take the LOWEST matching
    /// key (HashMap iteration order is randomised per process, so first-match
    /// made the answer non-deterministic), while this copy still returned
    /// whichever key the HashMap happened to yield first. The same card_no could
    /// therefore resolve to different cards depending on which accessor was used
    /// or which run you were in. Delegating makes that divergence impossible.
    pub fn get_card_by_no(&self, card_no: &str) -> Option<&Card> {
        self.get_card_id(card_no).and_then(|id| self.cards.get(&id))
    }

    /// Parse card_no into (base, rarity) where base is everything before last dash.
    fn parse_base_and_rarity(s: &str) -> Option<(String, String)> {
        s.rfind('-').map(|idx| (s[..idx].to_string(), s[idx + 1..].to_string()))
    }

    /// Return equivalent rarities for fallback lookup.
    /// Order matters: prefer closer matches.
    fn equivalent_rarities(rarity: &str) -> Vec<String> {
        match rarity {
            // Parallel family (canonical: P＋)
            "P+" | "P＋" | "P2" => vec!["P＋".to_string()],
            // Rare family (canonical: R＋)
            "R+" | "R＋" | "R2" => vec!["R＋".to_string()],
            // Love/Launch family (canonical: L＋)
            "L+" | "L＋" | "L2" => vec!["L＋".to_string()],
            // Normal family (canonical: N＋)
            "N+" | "N＋" | "N2" => vec!["N＋".to_string()],
            // Promo family (canonical: PR＋)
            "PR+" | "PR＋" => vec!["PR＋".to_string()],
            // Others map to themselves (no equivalents)
            _ => vec![],
        }
    }

    pub fn get_card_id(&self, card_no: &str) -> Option<i16> {
        // 1. Exact match
        if let Some(&id) = self.card_no_to_id.get(card_no) {
            return Some(id);
        }
        // 2. Normalized (fullwidth→halfwidth, lowercase→uppercase)
        let normalized = Self::normalize_card_no(card_no);
        if let Some(&id) = self.normalized_no_to_id.get(&normalized) {
            return Some(id);
        }
        // 3. Parse base + requested rarity, try exact, then equivalents, then any
        if let Some((base, requested_rarity)) = Self::parse_base_and_rarity(&normalized) {
            let exact = format!("{}-{}", base, requested_rarity);
            if let Some(&id) = self.normalized_no_to_id.get(&exact) {
                return Some(id);
            }
            for eq in Self::equivalent_rarities(&requested_rarity) {
                let variant = format!("{}-{}", base, eq);
                if let Some(&id) = self.normalized_no_to_id.get(&variant) {
                    return Some(id);
                }
            }
            // "Any rarity of this base" fallback. The lowest key wins, NOT the
            // first one HashMap iteration happens to yield: HashMap order is
            // randomised per process, so taking the first match made
            // get_card_id non-deterministic — the same card_no could resolve to
            // a different print on different runs, and nothing in the engine or
            // the test suite may depend on which print that is.
            let prefix = format!("{}-", base);
            if let Some(id) = self
                .normalized_no_to_id
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .min_by(|a, b| a.0.cmp(b.0))
                .map(|(_, &id)| id)
            {
                return Some(id);
            }
        }
        // 4. Strip trailing rarity suffixes and retry normalized (legacy)
        for stripped in Self::strip_rarity_suffixes(&normalized) {
            if let Some(&id) = self.normalized_no_to_id.get(&stripped) {
                return Some(id);
            }
        }
        // 5. Contains fallback (last resort). Same determinism rule as step 3:
        // lowest matching key, not the first HashMap iteration yields.
        let narrow = normalized.replace('+', "＋");
        self.card_no_to_id
            .iter()
            .filter(|(k, _)| k.contains(&normalized) || k.contains(&narrow))
            .min_by(|a, b| a.0.cmp(b.0))
            .map(|(_, &id)| id)
    }

    /// Normalize card_no for lookup: uppercase, fullwidth → halfwidth.
    /// Avoids allocation when input is already ASCII uppercase with no
    /// fullwidth characters (the common case after initial load).
    pub fn normalize_card_no(card_no: &str) -> String {
        let mut result = String::with_capacity(card_no.len());
        let mut changed = false;
        for ch in card_no.chars() {
            match ch {
                'a'..='z' => {
                    result.push(ch.to_ascii_uppercase());
                    changed = true;
                }
                'ａ'..='ｚ' => {
                    // Fullwidth lowercase → ASCII uppercase: the offset from
                    // 'ａ' maps 1:1 onto 'A'..'Z'.
                    let upper = u32::from(ch) - u32::from('ａ') + u32::from('A');
                    // 'ａ'..='ｚ' maps 1:1 onto 'A'..='Z', so `upper` is always
                    // a valid scalar value; the fallback is unreachable.
                    result.push(char::from_u32(upper).unwrap_or('?'));
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

    /// Strip trailing rarity suffixes for fallback lookup.
/// Uses parse_base_and_rarity to extract base card number.
    fn strip_rarity_suffixes(s: &str) -> Vec<String> {
        if let Some((base, _)) = Self::parse_base_and_rarity(s) {
            vec![base]
        } else {
            vec![s.to_string()]
        }
    }

    /// Strip all whitespace from a card name so that inconsistent spacing
    /// (e.g. "南 ことり" vs "南ことり") does not break ability conditions.
    ///
    /// NOTE: this always returns an owned `String`. The "fast path" below used to
    /// be commented as allocation-free; it is not, because `name.to_string()`
    /// allocates. The comment was wrong for as long as it stood.
    ///
    /// WHAT THIS IS NOW FOR: the CARD side of a name comparison no longer needs
    /// it at all. `Card::name_key` carries the canonical form precomputed at load
    /// time (`cards/scrape_expansion.py`, backfilled by
    /// `cards/normalise_names.py`), so comparing a card against a needle is a
    /// plain string equality. What remains is the NEEDLE side — a name parsed out
    /// of printed ability text, which has no key of its own — and that is what
    /// this is still called for.
    ///
    /// Returning `Cow<'_, str>` would make the clean path genuinely free. It is
    /// deliberately left undone: it is ~18 call-site edits across
    /// `ability/condition.rs` and `ability/util.rs`, several matching on `&String`
    /// patterns, which is regression risk in ability evaluation for a saving
    /// whose release-build cost is still unmeasured.
    pub fn normalize_name(name: &str) -> String {
        if name.bytes().all(|b| !b.is_ascii_whitespace()) {
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
            // `name_key` is already whitespace-free and has '＆' folded to '&'
            // (cards/scrape_expansion.py), so this is a plain split. It used to be
            // `normalize_name(name).replace('＆', "&").split('&')` — four heap
            // allocations, the last two unconditional no-ops for 2610 of the
            // 2639 cards, measured at 2.99us/call on multi-name cards in a debug
            // build and reached during ability-condition evaluation.
            card.name_key.split('&').map(|s| s.to_string()).collect()
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
            #[cfg_attr(feature = "serde_support", serde(default))]
            pub name_key: Option<String>,
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

        // Computed BEFORE the struct literal because `name:` moves
        // `helper.name`. A card added by hand may lack the key, so derive it
        // rather than leaving every name comparison silently failing to match.
        let name_key = ArcStr::from(
            helper
                .name_key
                .filter(|k| !k.is_empty())
                .unwrap_or_else(|| CardDatabase::normalize_name(&helper.name))
                .as_str(),
        );

        Ok(Card {
            card_no: ArcStr::from(helper.card_no),
            #[cfg(not(feature = "compact_cards"))]
            img: helper.img,
            name: ArcStr::from(helper.name),
            name_key,
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

impl Default for CardDatabase {
    fn default() -> Self {
        Self::new()
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

    pub fn has_blade_heart_strict(&self) -> bool {
        self.blade_heart.is_some()
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

    /// The ドロー icon, on `special_heart` — the same field as スコア, and a
    /// different one from 全ブレード。
    pub fn has_draw_icon(&self) -> bool {
        self.special_heart
            .as_ref()
            .is_some_and(|sh| sh.hearts.contains_key(&HeartColor::Draw))
    }

    pub fn get_score(&self) -> u8 {
        self.score.unwrap_or(0)
    }
}

pub fn check_heart_requirement(need: &BaseHeart, provided: &BaseHeart) -> bool {
    if need.hearts.is_empty() {
        return true;
    }
    let total_provided: u8 = provided.hearts.values_sum();
    let total_required: u8 = need.hearts.values_sum();
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

