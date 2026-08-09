use crate::card::{AbilityEffect, HeartColor};
use crate::core::game_modifiers::ModifierEntry;
use crate::Arc;
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
#[cfg(feature = "serde_support")]
use serde::de::Deserializer;
#[cfg(feature = "serde_support")]
use serde::ser::Serializer;
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

/// Like `Box<str>` but `Arc`-backed for cheap clone (refcount bump, no str copy).
/// Used in `EffectKind` fields where the same string value may be accessed
/// across multiple effect evaluations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ArcStr(pub Arc<str>);

impl core::ops::Deref for ArcStr {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<String> for ArcStr {
    fn from(s: String) -> Self {
        ArcStr(Arc::from(s))
    }
}

impl From<&str> for ArcStr {
    fn from(s: &str) -> Self {
        ArcStr(Arc::from(s.to_string()))
    }
}

impl From<Box<str>> for ArcStr {
    fn from(b: Box<str>) -> Self {
        ArcStr(Arc::from(b))
    }
}

impl core::fmt::Display for ArcStr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq<&str> for ArcStr {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_ref() == *other
    }
}

impl PartialEq<str> for ArcStr {
    fn eq(&self, other: &str) -> bool {
        self.0.as_ref() == other
    }
}

impl AsRef<str> for ArcStr {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

#[cfg(feature = "serde_support")]
impl Serialize for ArcStr {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.as_ref().serialize(serializer)
    }
}

#[cfg(feature = "serde_support")]
impl<'de> Deserialize<'de> for ArcStr {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(ArcStr(Arc::from(s)))
    }
}

impl ArcStr {
    pub fn as_deref(&self) -> &str {
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum AbilityTrigger {
    Activation,
    Debut,
    LiveStart,
    LiveSuccess,
    Constant,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum TurnPhase {
    FirstAttackerNormal,
    SecondAttackerNormal,
    Live,
}

impl core::fmt::Display for TurnPhase {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TurnPhase::FirstAttackerNormal => write!(f, "FirstAttackerNormal"),
            TurnPhase::SecondAttackerNormal => write!(f, "SecondAttackerNormal"),
            TurnPhase::Live => write!(f, "Live"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum Phase {
    RockPaperScissors,
    ChooseFirstAttacker,
    MulliganFirstAttacker,
    MulliganSecondAttacker,
    Active,
    Energy,
    Draw,
    Main,
    LiveCardSetFirstAttacker,
    LiveCardSetSecondAttacker,
    FirstAttackerPerformance,
    SecondAttackerPerformance,
    LiveVictoryDetermination,
}

impl core::fmt::Display for Phase {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Phase::RockPaperScissors => write!(f, "RPS"),
            Phase::ChooseFirstAttacker => write!(f, "Choose 1st"),
            Phase::MulliganFirstAttacker => write!(f, "Mulligan (1st)"),
            Phase::MulliganSecondAttacker => write!(f, "Mulligan (2nd)"),
            Phase::Active => write!(f, "Active"),
            Phase::Energy => write!(f, "Energy"),
            Phase::Draw => write!(f, "Draw"),
            Phase::Main => write!(f, "Main"),
            Phase::LiveCardSetFirstAttacker => write!(f, "LiveCardSet (1st)"),
            Phase::LiveCardSetSecondAttacker => write!(f, "LiveCardSet (2nd)"),
            Phase::FirstAttackerPerformance => write!(f, "Perform (1st)"),
            Phase::SecondAttackerPerformance => write!(f, "Perform (2nd)"),
            Phase::LiveVictoryDetermination => write!(f, "Live Result"),
        }
    }
}

impl Phase {
    pub fn label_jp(&self) -> &'static str {
        match self {
            Phase::RockPaperScissors => "ジャンケン",
            Phase::ChooseFirstAttacker => "先攻選択",
            Phase::MulliganFirstAttacker => "マリガン（先攻）",
            Phase::MulliganSecondAttacker => "マリガン（後攻）",
            Phase::Active => "アクティブ",
            Phase::Energy => "エネルギー",
            Phase::Draw => "ドロー",
            Phase::Main => "メイン",
            Phase::LiveCardSetFirstAttacker => "ライブセット（先攻）",
            Phase::LiveCardSetSecondAttacker => "ライブセット（後攻）",
            Phase::FirstAttackerPerformance => "パフォーマンス（先攻）",
            Phase::SecondAttackerPerformance => "パフォーマンス（後攻）",
            Phase::LiveVictoryDetermination => "ライブ勝敗判定",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum GameResult {
    FirstAttackerWins,
    SecondAttackerWins,
    Draw,
    Ongoing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum Duration {
    LiveEnd,
    ThisTurn,
    ThisLive,
    Permanent,
    AsLongAs,
    Unless,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct CardEffectItem {
    pub card_id: i16,
    pub amount: i16,
    #[cfg_attr(
        feature = "serde_support",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(
    feature = "serde_support",
    serde(tag = "type", rename_all = "snake_case")
)]
pub enum EffectData {
    HeartOverride {
        card_id: i16,
        color: String,
        count: u8,
    },
    SingleCard {
        card_id: i16,
        amount: i16,
        #[cfg_attr(
            feature = "serde_support",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        color: Option<String>,
    },
    MultiCard {
        items: Vec<CardEffectItem>,
    },
    AllCards {
        amount: i16,
    },
    SetBladeCount {
        card_id: i16,
    },
    SurplusHeart {
        is_p1: bool,
        old_value: u8,
    },
}

impl EffectData {
    pub fn card_id(&self) -> Option<i16> {
        match self {
            EffectData::HeartOverride { card_id, .. } => Some(*card_id),
            EffectData::SingleCard { card_id, .. } => Some(*card_id),
            EffectData::SetBladeCount { card_id } => Some(*card_id),
            _ => None,
        }
    }

    pub fn items(&self) -> Vec<CardEffectItemRef<'_>> {
        match self {
            EffectData::SingleCard {
                card_id,
                amount,
                color,
            } => {
                vec![CardEffectItemRef {
                    card_id: *card_id,
                    amount: *amount,
                    color: color.as_deref(),
                }]
            }
            EffectData::MultiCard { items } => items
                .iter()
                .map(|i| CardEffectItemRef {
                    card_id: i.card_id,
                    amount: i.amount,
                    color: i.color.as_deref(),
                })
                .collect(),
            _ => vec![],
        }
    }

    pub fn is_p1(&self) -> Option<bool> {
        match self {
            EffectData::SurplusHeart { is_p1, .. } => Some(*is_p1),
            _ => None,
        }
    }

    pub fn old_value(&self) -> Option<u8> {
        match self {
            EffectData::SurplusHeart { old_value, .. } => Some(*old_value),
            _ => None,
        }
    }

    pub fn count(&self) -> Option<u8> {
        match self {
            EffectData::HeartOverride { count, .. } => Some(*count),
            _ => None,
        }
    }

    pub fn color(&self) -> Option<&str> {
        match self {
            EffectData::HeartOverride { color, .. } => Some(color.as_str()),
            EffectData::SingleCard { color, .. } => color.as_deref(),
            _ => None,
        }
    }

    pub fn amount(&self) -> Option<i16> {
        match self {
            EffectData::SingleCard { amount, .. } => Some(*amount),
            EffectData::AllCards { amount } => Some(*amount),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]

pub struct CardEffectItemRef<'a> {
    pub card_id: i16,
    pub amount: i16,
    pub color: Option<&'a str>,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct TemporaryEffect {
    pub effect_type: String,
    pub duration: Duration,
    pub created_turn: u8,
    pub created_phase: Phase,
    pub target_player_id: String,
    pub description: String,
    pub creation_order: u8,
    pub effect_data: Option<EffectData>,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct ReplacementEffect {
    pub card_id: i16,
    pub player_id: String,
    pub original_event: String,
    pub replacement_effects: Vec<AbilityEffect>,
    pub is_choice_based: bool,
    pub applied_this_event: bool,
}

// ============== LIVE PERFORMANCE DATA (intermediate, from player_perform_live) ==============

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct LivePerformanceData {
    pub yell_count: u8,
    pub note_icons: u8,
    pub revealed_ids: Vec<i16>,
    pub member_contributions: Vec<MemberContribution>,
    pub yell_cards: Vec<YellCardResult>,
    pub total_hearts: [u8; 8],
    pub allocations: Vec<Allocation>,
    pub heart_sources: Vec<HeartSource>,
    pub blade_sources: Vec<BladeSource>,
    pub draw_effects_occurred: bool,
    pub live_card_ids: Vec<i16>,
    /// Live cards that moved from live_card_zone to waitroom during this phase.
    /// Used by caller (execute_performance_phase) to set recently_moved_cards
    /// so auto abilities (e.g. Riko BP6) can trigger on the zone change.
    pub moved_live_card_ids: Vec<i16>,
}

// ============== PERFORMANCE SNAPSHOT ==============

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct PerformanceSnapshot {
    pub turn: u8,
    pub player_id: String,
    pub lives: Vec<LiveCardResult>,
    pub member_contributions: Vec<MemberContribution>,
    pub yell_cards: Vec<YellCardResult>,
    pub total_hearts: [u8; 8],
    pub total_score: u8,
    pub success: bool,
    pub note_icons: u8,
    pub yell_count: u8,
    pub breakdown: Breakdown,
    pub triggered_abilities: Vec<TriggeredAbility>,
    pub p0_wins: bool,
    pub p1_wins: bool,
    /// Snapshot of need_heart_modifiers at performance time (includes live_start
    /// and other non-constant changes that get cleared by
    /// evaluate_success_zone_heart_reductions during victory determination).
    /// Merged back during snapshot finalization for correct required/adjustments/passed.
    /// Flat (card_id, color, entry) — only iterated, never keyed.
    pub performance_need_heart_modifiers: Vec<(i16, HeartColor, ModifierEntry)>,
    /// Per-color surplus hearts remaining after filling all live card requirements.
    /// Computed as total_hearts[color] - sum(live.filled[color]) across all lives.
    pub surplus_hearts: [u8; 8],
    /// Card IDs revealed during yell/cheer (from resolution zone).
    pub revealed_ids: Vec<i16>,
    /// Sum of individual live card scores (l.score) for passed lives only.
    pub base_score_total: u8,
    /// Sum of triggered score bonuses (l.score - l.base_score) for passed lives.
    pub card_bonus_total: u8,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct LiveCardResult {
    pub passed: bool,
    pub score: u8,
    pub base_score: u8,
    pub spare: [u8; 8],
    pub required: [u8; 8],
    pub filled: [u8; 8],
    pub adjustments: Vec<Adjustment>,
    pub card_id: i16,
    pub card_no: ArcStr,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct MemberContribution {
    pub source_id: i16,
    pub slot: u8,
    pub base_hearts: [u8; 8],
    pub bonus_hearts: [u8; 8],
    pub base_blades: u8,
    pub bonus_blades: u8,
    pub base_notes: u8,
    pub bonus_notes: u8,
    pub draw_icons: u8,
    pub ability_heart_bonuses: Vec<AbilityBonus>,
    pub ability_blade_bonuses: Vec<AbilityBonus>,
    pub card_no: ArcStr,
    pub is_wait: bool,
    /// Per-color heart delta from color transforms (bonus_hearts minus ability bonuses).
    pub transform_delta: [u8; 8],
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct YellCardResult {
    pub card_id: i16,
    pub blade_hearts: [u8; 8],
    pub note_icons: u8,
    pub draw_icons: u8,
    pub card_no: ArcStr,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct Breakdown {
    pub hearts: Vec<HeartSource>,
    pub blades: Vec<BladeSource>,
    pub allocations: Vec<Allocation>,
    pub requirements: Vec<EffectEntry>,
    pub transforms: Vec<EffectEntry>,
    pub scores: Vec<ScoreLine>,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct HeartSource {
    pub source_type: SourceType,
    pub source: ArcStr,
    pub value: [u8; 8],
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct BladeSource {
    pub source_type: SourceType,
    pub source: ArcStr,
    pub value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum AllocPhase {
    #[cfg_attr(feature = "serde_support", serde(rename = "1a_colored"))]
    Colored,
    #[cfg_attr(feature = "serde_support", serde(rename = "1b_h00_wild"))]
    H00Wild,
    #[cfg_attr(feature = "serde_support", serde(rename = "1c_all_wild"))]
    AllWild,
    #[cfg_attr(feature = "serde_support", serde(rename = "2_wildcard"))]
    Wildcard,
    #[cfg_attr(feature = "serde_support", serde(rename = "3c_all"))]
    CAll,
    #[cfg_attr(feature = "serde_support", serde(rename = "3a_colored_surplus"))]
    ColoredSurplus,
    #[cfg_attr(feature = "serde_support", serde(rename = "3b_h00"))]
    H00,
    #[cfg_attr(feature = "serde_support", serde(rename = "4_all_cleanup"))]
    AllCleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum SourceType {
    #[cfg_attr(feature = "serde_support", serde(rename = "stage"))]
    Stage,
    #[cfg_attr(feature = "serde_support", serde(rename = "yell"))]
    Yell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum SourceName {
    #[cfg_attr(feature = "serde_support", serde(rename = "Stage hearts"))]
    StageHearts,
    #[cfg_attr(feature = "serde_support", serde(rename = "Wildcard (Heart00)"))]
    WildcardHeart00,
    #[cfg_attr(feature = "serde_support", serde(rename = "All heart (icon_all)"))]
    AllHeartIconAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum AdjustmentType {
    #[cfg_attr(feature = "serde_support", serde(rename = "requirement"))]
    Requirement,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct Allocation {
    pub target_idx: u8,
    pub target_name: ArcStr,
    pub source_type: SourceType,
    pub source_name: SourceName,
    pub source_slot: Option<u8>,
    pub wildcard: bool,
    pub color: u8,
    pub amount: u8,
    pub is_bonus: bool,
    pub phase: AllocPhase,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct EffectEntry {
    pub source: String,
    pub value: String,
    pub desc: String,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct ScoreLine {
    pub source: String,
    pub value: u8,
}

/// Compact zone identifier — replaces String fields in MovementEvent to avoid
/// heap allocations. Covers all zone names used in the game engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "serde_support", serde(rename_all = "snake_case"))]
pub enum ZoneId {
    Stage,
    Hand,
    Deck,
    DeckTop,
    DeckBottom,
    Discard,
    Waitroom,
    Energy,
    EnergyZone,
    EnergyDeck,
    SuccessZone,
    LiveCardZone,
    SuccessLiveZone,
    EmptyArea,
    SameArea,
    UnderMember,
    LookedAt,
    RevealedCards,
    SelectedCards,
    Resolution,
    ExclusionZone,
    Unknown,
}

impl ZoneId {
    pub fn from_str(s: &str) -> Self {
        match s {
            "stage" => ZoneId::Stage,
            "hand" => ZoneId::Hand,
            "deck" => ZoneId::Deck,
            "deck_top" => ZoneId::DeckTop,
            "deck_bottom" => ZoneId::DeckBottom,
            "discard" => ZoneId::Discard,
            "waitroom" => ZoneId::Waitroom,
            "energy" | "energy_zone" => ZoneId::Energy,
            "energy_deck" => ZoneId::EnergyDeck,
            "success_zone" => ZoneId::SuccessZone,
            "live_card_zone" => ZoneId::LiveCardZone,
            "success_live_zone" | "success_live_card_zone" => ZoneId::SuccessLiveZone,
            "empty_area" => ZoneId::EmptyArea,
            "same_area" => ZoneId::SameArea,
            "under_member" | "under" => ZoneId::UnderMember,
            "looked_at" => ZoneId::LookedAt,
            "revealed_cards" => ZoneId::RevealedCards,
            "selected_cards" => ZoneId::SelectedCards,
            "resolution" | "resolution_zone" => ZoneId::Resolution,
            "exclusion_zone" => ZoneId::ExclusionZone,
            _ => ZoneId::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ZoneId::Stage => "stage",
            ZoneId::Hand => "hand",
            ZoneId::Deck => "deck",
            ZoneId::DeckTop => "deck_top",
            ZoneId::DeckBottom => "deck_bottom",
            ZoneId::Discard => "discard",
            ZoneId::Waitroom => "waitroom",
            ZoneId::Energy => "energy",
            ZoneId::EnergyZone => "energy_zone",
            ZoneId::EnergyDeck => "energy_deck",
            ZoneId::SuccessZone => "success_zone",
            ZoneId::LiveCardZone => "live_card_zone",
            ZoneId::SuccessLiveZone => "success_live_zone",
            ZoneId::EmptyArea => "empty_area",
            ZoneId::SameArea => "same_area",
            ZoneId::UnderMember => "under_member",
            ZoneId::LookedAt => "looked_at",
            ZoneId::RevealedCards => "revealed_cards",
            ZoneId::SelectedCards => "selected_cards",
            ZoneId::Resolution => "resolution",
            ZoneId::ExclusionZone => "exclusion_zone",
            ZoneId::Unknown => "unknown",
        }
    }

    /// Check zone equivalence: "discard" and "waitroom" are treated as the same zone
    /// in many game rules (card movement between them is considered same-area).
    pub fn equivalent(&self, other: &ZoneId) -> bool {
        self == other
            || (*self == ZoneId::Discard && *other == ZoneId::Waitroom)
            || (*self == ZoneId::Waitroom && *other == ZoneId::Discard)
            || (*self == ZoneId::Energy && *other == ZoneId::EnergyZone)
            || (*self == ZoneId::EnergyZone && *other == ZoneId::Energy)
    }
}

impl PartialEq<&str> for ZoneId {
    fn eq(&self, other: &&str) -> bool {
        *self == ZoneId::from_str(other)
    }
}

impl PartialEq<ZoneId> for &str {
    fn eq(&self, other: &ZoneId) -> bool {
        ZoneId::from_str(*self) == *other
    }
}

impl PartialEq<String> for ZoneId {
    fn eq(&self, other: &String) -> bool {
        *self == ZoneId::from_str(other)
    }
}

impl PartialEq<ZoneId> for String {
    fn eq(&self, other: &ZoneId) -> bool {
        ZoneId::from_str(self) == *other
    }
}

/// A structured record of a card movement event, capturing not just what moved
/// but WHAT CAUSED the move (which card's effect/ability). Replaces the old
/// pattern of separate tracking fields (recently_moved_cards, last_area_move_card_id,
/// last_area_move_by_player, etc.) with a unified event log.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct MovementEvent {
    /// The card that physically moved.
    pub moved_card_id: i16,
    /// The zone the card moved FROM.
    pub source_zone: ZoneId,
    /// The zone the card moved TO.
    pub dest_zone: ZoneId,
    /// The card whose ability/action caused the move (None = rule/cost with no source card).
    pub cause_card_id: Option<i16>,
    /// The player whose effect/action caused the move.
    pub cause_player_id: String,
    /// Whether the move was caused by a card effect (true) vs cost/rules (false).
    pub effect_only: bool,
    /// Monotonically increasing counter for ordering events.
    pub timestamp: u8,
}

/// A structured record of a stage-area-to-stage-area position change.
/// Captures the old/new positions and what caused the move.
/// Replaces the fragile snapshot-based detection with direct event tracking.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct PositionChangeEvent {
    /// The card that changed position.
    pub moved_card_id: i16,
    /// The position index before the change (0=left, 1=center, 2=right).
    pub old_position: u8,
    /// The position index after the change (0=left, 1=center, 2=right).
    pub new_position: u8,
    /// The card whose ability/action caused the move (None = rule/cost with no source card).
    pub cause_card_id: Option<i16>,
    /// The player whose effect/action caused the move.
    pub cause_player_id: String,
    /// Whether the move was caused by a card effect (true) vs cost/rules (false).
    pub effect_only: bool,
}

/// Records that a specific ability applied a modifier during execution,
/// so the source can be traced back to the originating card + ability text.
/// Populated in effect handlers (execute_gain_resource etc.) and consumed
/// by build_snapshot to fill ability_heart_bonuses, ability_blade_bonuses,
/// Breakdown.scores, Breakdown.transforms, and TriggeredAbility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EffectType {
    HeartBonus,
    BladeBonus,
    ScoreBonus,
    ScoreSet,
    Transform,
    NeedHeartMod,
    HeartOverride,
}

impl EffectType {
    pub fn as_str(self) -> &'static str {
        match self {
            EffectType::HeartBonus => "heart_bonus",
            EffectType::BladeBonus => "blade_bonus",
            EffectType::ScoreBonus => "score_bonus",
            EffectType::ScoreSet => "score_set",
            EffectType::Transform => "transform",
            EffectType::NeedHeartMod => "need_heart_mod",
            EffectType::HeartOverride => "heart_override",
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct AbilityApplication {
    pub source_card_id: i16,
    pub ability_text: ArcStr,
    pub effect_type: EffectType,
    pub target_card_id: i16,
    pub heart_color: Option<u8>,
    pub amount: i16,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct TriggeredAbility {
    pub source_card_id: i16,
    pub name: String,
    pub card_name: ArcStr,
    pub effect_text: ArcStr,
    pub condition_text: Option<String>,
    pub is_public: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct Adjustment {
    #[cfg_attr(feature = "serde_support", serde(rename = "type"))]
    pub adjustment_type: AdjustmentType,
    pub desc: String,
    pub value: i16,
    pub color: u8,
    pub source: String,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct AbilityBonus {
    pub source: ArcStr,
    pub amount: u8,
    pub color: Option<u8>,
    pub ability_text: ArcStr,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(
    feature = "serde_support",
    serde(tag = "type", rename_all = "snake_case")
)]
pub enum LogMetadata {
    TriggerEvaluation {
        trigger: String,
        zone: String,
        result: String,
        ability_index: usize,
        ability_text: String,
    },
    TurnStart {
        turn: u8,
    },
    RpsResult {
        p1_choice: String,
        p2_choice: String,
        p1_value: u8,
        p2_value: u8,
        winner: String,
    },
    AbilityResolution {
        result: String,
        /// Canonical trigger key (debut/live_start/live_success/activation/
        /// constant/auto) so the web renderer can still identify the trigger even
        /// after the trigger_evaluation entry is committed in place as a
        /// resolution.
        trigger: String,
        #[cfg(feature = "serde_support")]
        #[cfg_attr(
            feature = "serde_support",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        items: Vec<serde_json::Value>,
        ability_text: String,
        zone: String,
        #[cfg_attr(
            feature = "serde_support",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        error: Option<String>,
        #[cfg_attr(
            feature = "serde_support",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        resolved: Option<bool>,
    },
    /// A player-facing choice was offered. `offered` holds the legal options
    /// (card names, option labels, heart colors, etc.). Consumed by a later
    /// `ChoiceResolved` entry or directly closed when skipped.
    ChoiceOffered {
        offered: Vec<String>,
        skip_allowed: bool,
    },
    /// The resolved outcome of a previously-offered choice: what the player
    /// actually picked (`chosen`) and whether they skipped. Only the number of
    /// offered options is stored — the full offered array lives in the
    /// preceding `ChoiceOffered` entry, so a `ChoiceResolved` entry stays compact.
    ChoiceResolved {
        offered_count: usize,
        chosen: Vec<String>,
        skipped: bool,
    },
}

impl Default for LogMetadata {
    fn default() -> Self {
        LogMetadata::AbilityResolution {
            result: String::new(),
            trigger: String::new(),
            #[cfg(feature = "serde_support")]
            items: Vec::new(),
            ability_text: String::new(),
            zone: String::new(),
            error: None,
            resolved: None,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct LogEntry {
    pub text: String,
    pub turn: u8,
    pub player_label: String,
    pub source_card_id: Option<i16>,
    pub source_card_name: Option<String>,
    pub category: String,
    #[cfg_attr(
        feature = "serde_support",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub metadata: Option<LogMetadata>,
}

/// Condition evaluation result for a single condition on a jyouji ability.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct ConditionResult {
    pub text: String,
    pub passed: bool,
}

/// Status of a single constant (常時) ability on the board.
/// Exposed to the frontend for the jyouji summary bar.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct ConstantAbilityStatus {
    pub card_id: i16,
    pub card_name: String,
    pub owner: String,
    pub zone: String,
    pub ability_text: String,
    pub all_conditions_met: bool,
    pub conditions: Vec<ConditionResult>,
}
