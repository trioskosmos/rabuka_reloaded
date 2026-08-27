use crate::core::types::ArcStr;
#[cfg(feature = "serde_support")]
use serde::ser::Serializer;

/// Strongly-typed zone identifiers to prevent stringly-typed bugs.
/// Replaces error-prone zone == "hand" patterns with Zone::Hand.
#[cfg(feature = "no_std")]
use alloc::string::{String, ToString};

/// Generates the wire-string tables (`from_str` / `to_str`) for an enum from
/// one `Variant => "wire_name"` list, so parsing and serialization can never
/// drift apart. Invoke inside the enum's `impl` block; the receiver of
/// `to_str` is uniformly `&self`. Human-facing label tables stay hand-written
/// because their phrasing is intentionally irregular.
macro_rules! wire_tables {
    ($($variant:ident => $wire:literal),+ $(,)?) => {
        /// Convert a wire string to the typed value.
        /// Returns None for unrecognized names (makes typos detectable at parse time).
        pub fn from_str(s: &str) -> Option<Self> {
            match s {
                $($wire => Some(Self::$variant),)+
                _ => None,
            }
        }

        /// Convert the typed value back to its wire string.
        pub fn to_str(&self) -> &'static str {
            match self {
                $(Self::$variant => $wire,)+
            }
        }
    };
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Zone {
    Hand,
    Stage,
    StageCenter,
    StageLeft,
    StageRight,
    Discard,
    Waitroom,
    Energy,
    EnergyZone,
    Deck,
    DeckTop,
    DeckBottom,
    SuccessZone,
    LiveCardZone,
    SuccessLiveZone,
    EnergyDeck,
    EmptyArea,
    SameArea,
    UnderMember,
    LookedAt,
    RevealedCards,
    SelectedCards,
    Resolution,
    ExclusionZone,
    // Non-zone "source"/"destination" marker values found in card data.
    // Kept as Zone variants (rather than an Other/String catch-all) so the
    // source/destination fields can be typed without losing these markers.
    PrecedingMoved,
    RecentlyMoved,
    ThoseCards,
    LookedAtRemaining,
    DeckTopOrBottom,
    Front,
    /// Fallback for source/destination strings not among the known markers.
    /// Never produced by real card data; exists so `From<&str>`/decode can
    /// always succeed without an owned `Other(String)` (keeps `Zone: Copy`).
    Unknown,
}

impl Zone {
    /// Convert a string zone name to the typed enum.
    /// Returns None for unrecognized zone names (prevents silent typos from becoming silent no-ops).
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "hand" => Some(Zone::Hand),
            "stage" => Some(Zone::Stage),
            "center" => Some(Zone::StageCenter),
            "left" | "left_side" => Some(Zone::StageLeft),
            "right" | "right_side" => Some(Zone::StageRight),
            "discard" => Some(Zone::Discard),
            "waitroom" => Some(Zone::Waitroom),
            "energy" | "energy_zone" => Some(Zone::Energy),
            "deck" => Some(Zone::Deck),
            "deck_top" => Some(Zone::DeckTop),
            "deck_bottom" => Some(Zone::DeckBottom),
            "success_zone" => Some(Zone::SuccessZone),
            "live_card_zone" => Some(Zone::LiveCardZone),
            "success_live_zone" | "success_live_card_zone" => Some(Zone::SuccessLiveZone),
            "energy_deck" => Some(Zone::EnergyDeck),
            "empty_area" => Some(Zone::EmptyArea),
            "same_area" => Some(Zone::SameArea),
            "under_member" | "under" => Some(Zone::UnderMember),
            "looked_at" => Some(Zone::LookedAt),
            "revealed_cards" => Some(Zone::RevealedCards),
            "selected_cards" => Some(Zone::SelectedCards),
            "resolution" | "resolution_zone" => Some(Zone::Resolution),
            "exclusion_zone" => Some(Zone::ExclusionZone),
            "preceding_moved" => Some(Zone::PrecedingMoved),
            "recently_moved" => Some(Zone::RecentlyMoved),
            "those_cards" => Some(Zone::ThoseCards),
            "looked_at_remaining" => Some(Zone::LookedAtRemaining),
            "deck_top_or_bottom" => Some(Zone::DeckTopOrBottom),
            "front" => Some(Zone::Front),
            _ => None,
        }
    }

    /// Convert the typed zone back to a string representation (for JSON/display).
    pub fn to_str(&self) -> &'static str {
        match self {
            Zone::Hand => "hand",
            Zone::Stage => "stage",
            Zone::StageCenter => "center",
            Zone::StageLeft => "left",
            Zone::StageRight => "right",
            Zone::Discard => "discard",
            Zone::Waitroom => "waitroom",
            Zone::Energy => "energy",
            Zone::EnergyZone => "energy_zone",
            Zone::Deck => "deck",
            Zone::DeckTop => "deck_top",
            Zone::DeckBottom => "deck_bottom",
            Zone::SuccessZone => "success_zone",
            Zone::LiveCardZone => "live_card_zone",
            Zone::SuccessLiveZone => "success_live_zone",
            Zone::EnergyDeck => "energy_deck",
            Zone::EmptyArea => "empty_area",
            Zone::SameArea => "same_area",
            Zone::UnderMember => "under_member",
            Zone::LookedAt => "looked_at",
            Zone::RevealedCards => "revealed_cards",
            Zone::SelectedCards => "selected_cards",
            Zone::Resolution => "resolution",
            Zone::ExclusionZone => "exclusion_zone",
            Zone::PrecedingMoved => "preceding_moved",
            Zone::RecentlyMoved => "recently_moved",
            Zone::ThoseCards => "those_cards",
            Zone::LookedAtRemaining => "looked_at_remaining",
            Zone::DeckTopOrBottom => "deck_top_or_bottom",
            Zone::Front => "front",
            Zone::Unknown => "unknown",
        }
    }

    /// Always-succeed conversion for the effect `source`/`destination` fields.
    /// Unlike `from_str` (which returns `None` for unrecognized strings so the
    /// ~100 condition/location call sites keep their `None` handling), this
    /// maps any known zone/marker to its typed variant and everything else to
    /// `Zone::Unknown`. Real card data only ever contains known values, so
    /// round-trip fidelity is preserved.
    pub fn from_source_str(s: &str) -> Self {
        Zone::from_str(s).unwrap_or(Zone::Unknown)
    }

    /// String form, matching `to_str`. Exists so `Option<Zone>` fields can be
    /// read via `.map(Zone::as_str)` mirroring the old `Option<ArcStr>.as_deref()`.
    pub fn as_str(&self) -> &'static str {
        self.to_str()
    }
}

impl core::fmt::Display for Zone {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl From<&str> for Zone {
    fn from(s: &str) -> Self {
        Zone::from_source_str(s)
    }
}

impl From<String> for Zone {
    fn from(s: String) -> Self {
        Zone::from_source_str(&s)
    }
}

impl From<ArcStr> for Zone {
    fn from(s: ArcStr) -> Self {
        Zone::from_source_str(s.as_ref())
    }
}

#[cfg(feature = "serde_support")]
impl serde::Serialize for Zone {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.to_str())
    }
}

#[cfg(feature = "serde_support")]
impl<'de> serde::Deserialize<'de> for Zone {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <ArcStr as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Zone::from_source_str(&s))
    }
}

/// Which player an effect targets. The effect-level `target` string currently
/// overloads a player ("self"/"opponent"/"both"/"either") with a destination
/// zone ("deck") and rare ability-reference strings; `TargetPlayer` captures
/// the player subset so comparisons become typed instead of string re-parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetPlayer {
    Self_,
    Opponent,
    Both,
    Either,
}

impl TargetPlayer {
    /// Parse a player-target string. Returns `None` for values that are not a
    /// player target ("deck", ability-reference strings, etc.), so callers can
    /// fall back to the raw string for those cases.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "self" => Some(TargetPlayer::Self_),
            "opponent" => Some(TargetPlayer::Opponent),
            "both" => Some(TargetPlayer::Both),
            "either" => Some(TargetPlayer::Either),
            _ => None,
        }
    }

    /// Convert back to the string form used in card data / bytecode.
    pub fn to_str(&self) -> &'static str {
        match self {
            TargetPlayer::Self_ => "self",
            TargetPlayer::Opponent => "opponent",
            TargetPlayer::Both => "both",
            TargetPlayer::Either => "either",
        }
    }

    pub fn as_str(&self) -> &'static str {
        self.to_str()
    }
}

impl core::fmt::Display for TargetPlayer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

/// How an effect designates the member that receives the placed card.
/// Folds the previously-separate `self_target` + `under_self` booleans into one
/// typed enum. The two booleans never co-occur in card data and were read with
/// different semantics (filtering vs. placement), so folding into a single
/// enum removes the overlap ambiguity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlacementTarget {
    /// The effect's target is the activating card itself (source-filtering
    /// semantics — the activating card is a valid selection target).
    FilterSelfAsSource,
    /// The placed card auto-places under the activating member (no stage-member
    /// choice), i.e. "…をこのメンバーの下に置く".
    UnderThisMember,
    /// The player chooses which stage member receives the placed card.
    UnderChosenMember,
}

/// Strongly-typed ability effect action types to prevent stringly-typed dispatch bugs.
/// Action type for ability effects.
/// ~60 variants cover all effect actions in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "serde_support", serde(rename_all = "snake_case"))]
pub enum ActionType {
    // Card movement
    DrawCard,
    DrawUntilCount,
    MoveCards,
    DiscardCard,
    Select,
    SelectCards,
    LookAndSelect,
    LookAt,
    Reveal,
    RevealPerGroup,
    RevealUntilLiveCard,
    RevealUntilChosenCard,

    // State changes
    ChangeState,
    PositionChange,
    Rotation,
    PlaceEnergyUnderMember,
    SetCardIdentity,
    ModifyRequiredHeartsSuccess,
    GainResource,
    PayEnergy,

    // Ability modifications
    GainAbility,
    GainAbilityFromSource,
    InvalidateAbility,
    SuppressAbilityTrigger,
    ActivateAbility,

    // Cost modifications
    ModifyCost,
    ModifyYellSource,
    SetCost,
    SetCostToUse,

    // Score and hearts
    ModifyScore,
    ModifyRequiredHearts,

    // Blade and heart
    SetBladeType,
    SetBladeCount,
    SetHeartType,
    SpecifyHeartColor,
    ChooseRequiredHearts,

    // Compound effects
    Sequential,
    ConditionalAlternative,
    ConditionalOnResult,
    ConditionalOnOptional,

    // Restrictions and limits
    Restriction,
    ActivationRestriction,
    ModifyLimit,

    // Utility
    Shuffle,
    ReYell,
    Custom,
    DoNothing,
    Choice,
    RepeatProcedure,
    DiscardUntilCount,

    // Replacement and triggers
    AllBladeTiming,
    ReduceLiveCardSetLimit,

    // Player target choice
    ChooseTargetPlayer,

    // Number selection (e.g. Kosuzu: "choose a number")
    SelectNumber,

    // Missing variants from effects/mod.rs dispatch
    PlayBatonTouch,
    ModifyRequiredHeartsGlobal,
    ModifyYellCount,
    ActivationCost,
    PerformYell,

    // Internal/procedural action types (used within the engine, not from JSON)
    ConditionalOptional,
    CompoundAction,
    OpponentAction,
    ActionBy,
    SequentialCost,
    ChoiceCondition,
    EnergyCondition,
}

impl ActionType {
    // Wire-string tables generated from a single list — see [`wire_tables`].
    wire_tables! {
        // Card movement
        DrawCard => "draw_card",
        DrawUntilCount => "draw_until_count",
        MoveCards => "move_cards",
        DiscardCard => "discard_card",
        Select => "select",
        SelectNumber => "select_number",
        SelectCards => "select_cards",
        LookAndSelect => "look_and_select",
        LookAt => "look_at",
        Reveal => "reveal",
        RevealPerGroup => "reveal_per_group",
        RevealUntilLiveCard => "reveal_until_live_card",
        RevealUntilChosenCard => "reveal_until_chosen_card",
        // State changes
        ChangeState => "change_state",
        PositionChange => "position_change",
        Rotation => "rotation",
        PlaceEnergyUnderMember => "place_energy_under_member",
        ModifyRequiredHeartsSuccess => "modify_required_hearts_success",
        GainResource => "gain_resource",
        PayEnergy => "pay_energy",
        // Ability modifications
        GainAbility => "gain_ability",
        GainAbilityFromSource => "gain_ability_from_source",
        InvalidateAbility => "invalidate_ability",
        SuppressAbilityTrigger => "suppress_ability_trigger",
        ActivateAbility => "activate_ability",
        // Cost modifications
        ModifyCost => "modify_cost",
        ModifyYellSource => "modify_yell_source",
        SetCost => "set_cost",
        SetCardIdentity => "set_card_identity",
        SetCostToUse => "set_cost_to_use",
        // Score and hearts
        ModifyScore => "modify_score",
        ModifyRequiredHearts => "modify_required_hearts",
        // Blade and heart
        SetBladeType => "set_blade_type",
        SetBladeCount => "set_blade_count",
        SetHeartType => "set_heart_type",
        SpecifyHeartColor => "specify_heart_color",
        ChooseRequiredHearts => "choose_required_hearts",
        // Compound effects
        Sequential => "sequential",
        ConditionalAlternative => "conditional_alternative",
        ConditionalOnResult => "conditional_on_result",
        ConditionalOnOptional => "conditional_on_optional",
        // Restrictions and limits
        Restriction => "restriction",
        ActivationRestriction => "activation_restriction",
        ModifyLimit => "modify_limit",
        // Utility
        Shuffle => "shuffle",
        ReYell => "re_yell",
        Custom => "custom",
        DoNothing => "do_nothing",
        Choice => "choice",
        RepeatProcedure => "repeat_procedure",
        DiscardUntilCount => "discard_until_count",
        // Replacement and triggers
        AllBladeTiming => "all_blade_timing",
        ReduceLiveCardSetLimit => "reduce_live_card_set_limit",
        // Player target choice / selection / missing variants
        ChooseTargetPlayer => "choose_target_player",
        PlayBatonTouch => "play_baton_touch",
        ModifyRequiredHeartsGlobal => "modify_required_hearts_global",
        ModifyYellCount => "modify_yell_count",
        ActivationCost => "activation_cost",
        PerformYell => "perform_yell",
        // Internal/procedural action types (used within the engine, not from JSON)
        ConditionalOptional => "conditional_optional",
        CompoundAction => "compound_action",
        OpponentAction => "opponent_action",
        ActionBy => "action_by",
        SequentialCost => "sequential_cost",
        ChoiceCondition => "choice_condition",
        EnergyCondition => "energy_condition",
    }

    /// Human-readable action label for debug/UI output.
    pub fn label(&self) -> &'static str {
        match self {
            ActionType::DrawCard => "Draw Card",
            ActionType::DrawUntilCount => "Draw Until Count",
            ActionType::ChooseTargetPlayer => "Choose Target Player",
            ActionType::MoveCards => "Move Cards",
            ActionType::DiscardCard => "Discard Card",
            ActionType::Select => "Select",
            ActionType::SelectNumber => "Select Number",
            ActionType::SelectCards => "Select Cards",
            ActionType::LookAndSelect => "Look and Select",
            ActionType::LookAt => "Look At",
            ActionType::Reveal => "Reveal",
            ActionType::RevealPerGroup => "Reveal Per Group",
            ActionType::RevealUntilLiveCard => "Reveal Until Live Card",
            ActionType::RevealUntilChosenCard => "Reveal Until Chosen Card",
            ActionType::ChangeState => "Change State",
            ActionType::PositionChange => "Position Change",
            ActionType::Rotation => "Rotation",
            ActionType::PlaceEnergyUnderMember => "Place Energy Under Member",
            ActionType::SetCardIdentity => "Set Card Identity",
            ActionType::ModifyRequiredHeartsSuccess => "Modify Required Hearts (Success)",
            ActionType::GainResource => "Gain Resource",
            ActionType::PayEnergy => "Pay Energy",
            ActionType::GainAbility => "Gain Ability",
            ActionType::GainAbilityFromSource => "Gain Ability from Source",
            ActionType::InvalidateAbility => "Invalidate Ability",
            ActionType::SuppressAbilityTrigger => "Suppress Ability Trigger",
            ActionType::ActivateAbility => "Activate Ability",
            ActionType::ModifyScore => "Modify Score",
            ActionType::ModifyRequiredHearts => "Modify Required Hearts",
            ActionType::ModifyYellSource => "Modify Yell Source",
            ActionType::ModifyCost => "Modify Cost",
            ActionType::SetCost => "Set Cost",
            ActionType::SetCostToUse => "Set Cost to Use",
            ActionType::SetBladeType => "Set Blade Type",
            ActionType::SetBladeCount => "Set Blade Count",
            ActionType::SetHeartType => "Set Heart Type",
            ActionType::SpecifyHeartColor => "Specify Heart Color",
            ActionType::ChooseRequiredHearts => "Choose Required Hearts",
            ActionType::Sequential => "Sequential",
            ActionType::ConditionalAlternative => "Conditional Alternative",
            ActionType::ConditionalOnResult => "Conditional on Result",
            ActionType::ConditionalOnOptional => "Conditional on Optional",
            ActionType::Restriction => "Restriction",
            ActionType::ActivationRestriction => "Activation Restriction",
            ActionType::ModifyLimit => "Modify Limit",
            ActionType::Shuffle => "Shuffle",
            ActionType::ReYell => "Re Yell",
            ActionType::Custom => "Custom",
            ActionType::DoNothing => "Do Nothing",
            ActionType::Choice => "Choice",
            ActionType::RepeatProcedure => "Repeat Procedure",
            ActionType::DiscardUntilCount => "Discard Until Count",
            ActionType::AllBladeTiming => "All Blade Timing",
            ActionType::ReduceLiveCardSetLimit => "Reduce Live Card Set Limit",
            ActionType::PlayBatonTouch => "Play Baton Touch",
            ActionType::ModifyRequiredHeartsGlobal => "Modify Required Hearts (Global)",
            ActionType::ModifyYellCount => "Modify Yell Count",
            ActionType::ActivationCost => "Activation Cost",
            ActionType::PerformYell => "Perform Yell",
            ActionType::ConditionalOptional => "Conditional Optional",
            ActionType::CompoundAction => "Compound Action",
            ActionType::OpponentAction => "Opponent Action",
            ActionType::ActionBy => "Action By",
            ActionType::SequentialCost => "Sequential Cost",
            ActionType::ChoiceCondition => "Choice Condition",
            ActionType::EnergyCondition => "Energy Condition",
        }
    }
}

impl Default for ActionType {
    fn default() -> Self {
        ActionType::Custom
    }
}

impl core::fmt::Display for ActionType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

// ============== CONDITION TYPE ==============

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "serde_support", serde(rename_all = "snake_case"))]
pub enum ConditionType {
    Compound,
    ComparisonCondition,
    LocationCondition,
    CardCountCondition,
    CardBladeCondition,
    GroupCondition,
    PositionCondition,
    AppearanceCondition,
    TemporalCondition,
    StateCondition,
    EnergyStateCondition,
    MovementCondition,
    AbilityFilterCondition,
    OrCondition,
    AnyOfCondition,
    ScoreThresholdCondition,
    ChoiceCondition,
    PositionChangeCondition,
    StateChangeCondition,
    OpponentChoiceCondition,
    OpponentLiveSuccess,
    ComplexCondition,
    NoExcessHeart,
    OtherwiseCondition,
    NotMoved,
    HasMoved,
    ResourceCondition,
    ActionSuccessCondition,
    AllCostComparisonCondition,
    HighestCostOnStageCondition,
    BothCondition,
    AllRevealedMatchHeartColor,
    Custom,
}

impl ConditionType {
    // Wire-string tables generated from a single list — see [`wire_tables`].
    wire_tables! {
        Compound => "compound",
        ComparisonCondition => "comparison_condition",
        LocationCondition => "location_condition",
        CardCountCondition => "card_count_condition",
        CardBladeCondition => "card_blade_condition",
        GroupCondition => "group_condition",
        PositionCondition => "position_condition",
        AppearanceCondition => "appearance_condition",
        TemporalCondition => "temporal_condition",
        StateCondition => "state_condition",
        EnergyStateCondition => "energy_state_condition",
        MovementCondition => "movement_condition",
        AbilityFilterCondition => "ability_filter_condition",
        OrCondition => "or_condition",
        AnyOfCondition => "any_of_condition",
        ScoreThresholdCondition => "score_threshold_condition",
        ChoiceCondition => "choice_condition",
        PositionChangeCondition => "position_change_condition",
        StateChangeCondition => "state_change_condition",
        OpponentChoiceCondition => "opponent_choice_condition",
        OpponentLiveSuccess => "opponent_live_success",
        ComplexCondition => "complex_condition",
        NoExcessHeart => "no_excess_heart",
        OtherwiseCondition => "otherwise_condition",
        BothCondition => "both_condition",
        NotMoved => "not_moved",
        HasMoved => "has_moved",
        ResourceCondition => "resource_condition",
        ActionSuccessCondition => "action_success_condition",
        AllCostComparisonCondition => "all_cost_comparison_condition",
        HighestCostOnStageCondition => "highest_cost_on_stage_condition",
        AllRevealedMatchHeartColor => "all_revealed_match_heart_color",
        Custom => "custom",
    }
}

// ============== SELECT TARGET KIND ==============

/// Typed alternatives for `Choice::SelectTarget.target` string field.
/// Keeps the struct field as `String` for JSON compat, but matching code uses
/// this enum for type safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectTargetKind {
    Choice,
    ChoiceString,
    PayOptionalCostSkipOptionalCost,
    DoubleBatonTouch,
    PrimaryAlternative,
    ApplyReplacement,
    ChooseRequiredHearts,
    PositionDestination,
    HeartColor,
    ChoiceType,
    ChoiceCondition,
    ConditionalOptional,
    DrawAnyNumber,
    Order,
    SelfOrOpponent,
    PayCostAllDiscard,
}

impl SelectTargetKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "choice" => Some(Self::Choice),
            "choice_string" => Some(Self::ChoiceString),
            crate::ability::types::PAY_SKIP_TARGET => Some(Self::PayOptionalCostSkipOptionalCost),
            "double_baton_touch" => Some(Self::DoubleBatonTouch),
            "primary|alternative" => Some(Self::PrimaryAlternative),
            "apply_replacement" => Some(Self::ApplyReplacement),
            "choose_required_hearts" => Some(Self::ChooseRequiredHearts),
            "position|destination" => Some(Self::PositionDestination),
            "heart_color" => Some(Self::HeartColor),
            "choice_type" => Some(Self::ChoiceType),
            "choice_condition" => Some(Self::ChoiceCondition),
            "conditional_optional" => Some(Self::ConditionalOptional),
            "draw_any_number" => Some(Self::DrawAnyNumber),
            "order" => Some(Self::Order),
            "self_or_opponent" => Some(Self::SelfOrOpponent),
            "pay_cost_all:discard_all" => Some(Self::PayCostAllDiscard),
            _ => None,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            Self::Choice => "choice",
            Self::ChoiceString => "choice_string",
            Self::PayOptionalCostSkipOptionalCost => crate::ability::types::PAY_SKIP_TARGET,
            Self::DoubleBatonTouch => "double_baton_touch",
            Self::PrimaryAlternative => "primary|alternative",
            Self::ApplyReplacement => "apply_replacement",
            Self::ChooseRequiredHearts => "choose_required_hearts",
            Self::PositionDestination => "position|destination",
            Self::HeartColor => "heart_color",
            Self::ChoiceType => "choice_type",
            Self::ChoiceCondition => "choice_condition",
            Self::ConditionalOptional => "conditional_optional",
            Self::DrawAnyNumber => "draw_any_number",
            Self::Order => "order",
            Self::SelfOrOpponent => "self_or_opponent",
            Self::PayCostAllDiscard => "pay_cost_all:discard_all",
        }
    }
}

impl TryFrom<String> for ActionType {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s).ok_or_else(|| format!("Unknown action type: {}", s))
    }
}

impl From<ActionType> for String {
    fn from(at: ActionType) -> String {
        at.to_str().to_string()
    }
}

// ============== EFFECT CARD TYPE ==============
//
// Typed replacement for the raw `card_type` string field found on effect
// kinds. The known values (`member_card`, `live_card`, `energy_card`) become
// variants; any unrecognized value is preserved verbatim in `Other(ArcStr)`
// so existing JSON / bytecode assets keep loading without regeneration.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]

pub enum EffectCardType {
    MemberCard,
    LiveCard,
    EnergyCard,
    /// Catch-all for values not part of the known set. Preserves round-trip
    /// fidelity for legacy / unexpected `card_type` strings.
    Other(ArcStr),
}

impl EffectCardType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "member_card" => EffectCardType::MemberCard,
            "live_card" => EffectCardType::LiveCard,
            "energy_card" => EffectCardType::EnergyCard,
            other => EffectCardType::Other(ArcStr::from(other)),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            EffectCardType::MemberCard => "member_card",
            EffectCardType::LiveCard => "live_card",
            EffectCardType::EnergyCard => "energy_card",
            EffectCardType::Other(s) => s.as_ref(),
        }
    }
}

#[cfg(all(not(feature = "no_std"), feature = "serde_support"))]
impl serde::Serialize for EffectCardType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(all(not(feature = "no_std"), feature = "serde_support"))]
impl<'de> serde::Deserialize<'de> for EffectCardType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <ArcStr as serde::Deserialize>::deserialize(deserializer)?;
        Ok(EffectCardType::from_str(&s))
    }
}

#[cfg(all(feature = "no_std", feature = "serde_support"))]
impl serde::Serialize for EffectCardType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(all(feature = "no_std", feature = "serde_support"))]
impl<'de> serde::Deserialize<'de> for EffectCardType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        Ok(EffectCardType::from_str(&s))
    }
}

impl Default for EffectCardType {
    fn default() -> Self {
        EffectCardType::Other(ArcStr::from(""))
    }
}

impl core::fmt::Display for EffectCardType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============== EFFECT STATE ==============
//
// Typed replacement for the raw `state` / `state_change` string fields on
// effect kinds. Known values are `active` / `wait`; any unrecognized value
// is preserved verbatim in `Other(ArcStr)` so legacy JSON keeps loading.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]

pub enum EffectState {
    Active,
    Wait,
    Other(ArcStr),
}

impl EffectState {
    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => EffectState::Active,
            "wait" => EffectState::Wait,
            other => EffectState::Other(ArcStr::from(other)),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            EffectState::Active => "active",
            EffectState::Wait => "wait",
            EffectState::Other(s) => s.as_ref(),
        }
    }
}

#[cfg(all(not(feature = "no_std"), feature = "serde_support"))]
impl serde::Serialize for EffectState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(all(not(feature = "no_std"), feature = "serde_support"))]
impl<'de> serde::Deserialize<'de> for EffectState {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <ArcStr as serde::Deserialize>::deserialize(deserializer)?;
        Ok(EffectState::from_str(&s))
    }
}

#[cfg(all(feature = "no_std", feature = "serde_support"))]
impl serde::Serialize for EffectState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(all(feature = "no_std", feature = "serde_support"))]
impl<'de> serde::Deserialize<'de> for EffectState {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        Ok(EffectState::from_str(&s))
    }
}

impl Default for EffectState {
    fn default() -> Self {
        EffectState::Other(ArcStr::from(""))
    }
}

impl core::fmt::Display for EffectState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
