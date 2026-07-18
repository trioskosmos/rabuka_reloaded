use crate::core::types::ArcStr;
use serde::ser::Serializer;

/// Strongly-typed zone identifiers to prevent stringly-typed bugs.
/// Replaces error-prone zone == "hand" patterns with Zone::Hand.
#[cfg(feature = "psp")]
use alloc::string::{String, ToString};
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
        }
    }

    /// Human-readable zone label for UI/messages.
    pub fn label(&self) -> &'static str {
        match self {
            Zone::Hand => "Hand",
            Zone::Stage => "Stage",
            Zone::StageCenter => "Center",
            Zone::StageLeft => "Left",
            Zone::StageRight => "Right",
            Zone::Discard => "Discard",
            Zone::Waitroom => "Waitroom",
            Zone::Energy => "Energy",
            Zone::EnergyZone => "Energy Zone",
            Zone::Deck => "Deck",
            Zone::DeckTop => "Deck Top",
            Zone::DeckBottom => "Deck Bottom",
            Zone::SuccessZone => "Success Zone",
            Zone::LiveCardZone => "Live Card Zone",
            Zone::SuccessLiveZone => "Success Live Zone",
            Zone::EnergyDeck => "Energy Deck",
            Zone::EmptyArea => "Empty Area",
            Zone::SameArea => "Same Area",
            Zone::UnderMember => "Under Member",
            Zone::LookedAt => "Looked At",
            Zone::RevealedCards => "Revealed Cards",
            Zone::SelectedCards => "Selected Cards",
            Zone::Resolution => "Resolution",
            Zone::ExclusionZone => "Exclusion Zone",
        }
    }
}

impl core::fmt::Display for Zone {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

/// Strongly-typed ability effect action types to prevent stringly-typed dispatch bugs.
/// Replaces error-prone action == "draw" patterns with ActionType::Draw.
/// ~60 variants cover all effect actions in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    // Card movement
    Draw,
    DrawCard,
    DrawUntilCount,
    MoveCards,
    DiscardCard,
    Select,
    SelectCards,
    LookAndSelect,
    LookAt,
    Look,
    Reveal,
    RevealEffect,
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
    SetCardIdentityAllRegions,
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
    Tap,
    Rest,
    Discard,
    ChoiceCondition,
    EnergyCondition,
}

impl ActionType {
    /// Convert a string action name to the typed enum.
    /// Returns None for unrecognized action names (makes typos detectable at parse time).
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "draw" => Some(ActionType::Draw),
            "draw_card" => Some(ActionType::DrawCard),
            "draw_until_count" => Some(ActionType::DrawUntilCount),
            "choose_target_player" => Some(ActionType::ChooseTargetPlayer),
            "move_cards" => Some(ActionType::MoveCards),
            "discard_card" => Some(ActionType::DiscardCard),
            "select" => Some(ActionType::Select),
            "select_number" => Some(ActionType::SelectNumber),
            "select_cards" => Some(ActionType::SelectCards),
            "look_and_select" => Some(ActionType::LookAndSelect),
            "look_at" => Some(ActionType::LookAt),
            "look" => Some(ActionType::Look),
            "reveal" => Some(ActionType::Reveal),
            "reveal_effect" => Some(ActionType::RevealEffect),
            "reveal_per_group" => Some(ActionType::RevealPerGroup),
            "reveal_until_live_card" => Some(ActionType::RevealUntilLiveCard),
            "reveal_until_chosen_card" => Some(ActionType::RevealUntilChosenCard),
            "change_state" => Some(ActionType::ChangeState),
            "position_change" => Some(ActionType::PositionChange),
            "rotation" => Some(ActionType::Rotation),
            "place_energy_under_member" => Some(ActionType::PlaceEnergyUnderMember),
            "modify_required_hearts_success" => Some(ActionType::ModifyRequiredHeartsSuccess),
            "gain_resource" => Some(ActionType::GainResource),
            "pay_energy" => Some(ActionType::PayEnergy),
            "gain_ability" => Some(ActionType::GainAbility),
            "gain_ability_from_source" => Some(ActionType::GainAbilityFromSource),
            "invalidate_ability" => Some(ActionType::InvalidateAbility),
            "suppress_ability_trigger" => Some(ActionType::SuppressAbilityTrigger),
            "activate_ability" => Some(ActionType::ActivateAbility),
            "modify_score" => Some(ActionType::ModifyScore),
            "modify_required_hearts" => Some(ActionType::ModifyRequiredHearts),
            "modify_cost" => Some(ActionType::ModifyCost),
            "set_cost" => Some(ActionType::SetCost),
            "set_card_identity" => Some(ActionType::SetCardIdentity),
            "set_cost_to_use" => Some(ActionType::SetCostToUse),
            "set_blade_type" => Some(ActionType::SetBladeType),
            "set_blade_count" => Some(ActionType::SetBladeCount),
            "set_heart_type" => Some(ActionType::SetHeartType),
            "specify_heart_color" => Some(ActionType::SpecifyHeartColor),
            "choose_required_hearts" => Some(ActionType::ChooseRequiredHearts),
            "sequential" => Some(ActionType::Sequential),
            "conditional_alternative" => Some(ActionType::ConditionalAlternative),
            "conditional_on_result" => Some(ActionType::ConditionalOnResult),
            "conditional_on_optional" => Some(ActionType::ConditionalOnOptional),
            "restriction" => Some(ActionType::Restriction),
            "activation_restriction" => Some(ActionType::ActivationRestriction),
            "modify_limit" => Some(ActionType::ModifyLimit),
            "shuffle" => Some(ActionType::Shuffle),
            "re_yell" => Some(ActionType::ReYell),
            "custom" => Some(ActionType::Custom),
            "do_nothing" => Some(ActionType::DoNothing),
            "choice" => Some(ActionType::Choice),
            "repeat_procedure" => Some(ActionType::RepeatProcedure),
            "discard_until_count" => Some(ActionType::DiscardUntilCount),
            "all_blade_timing" => Some(ActionType::AllBladeTiming),
            "set_card_identity_all_regions" => Some(ActionType::SetCardIdentityAllRegions),
            "reduce_live_card_set_limit" => Some(ActionType::ReduceLiveCardSetLimit),

            "play_baton_touch" => Some(ActionType::PlayBatonTouch),
            "modify_required_hearts_global" => Some(ActionType::ModifyRequiredHeartsGlobal),
            "modify_yell_count" => Some(ActionType::ModifyYellCount),
            "activation_cost" => Some(ActionType::ActivationCost),
            "perform_yell" => Some(ActionType::PerformYell),
            "conditional_optional" => Some(ActionType::ConditionalOptional),
            "compound_action" => Some(ActionType::CompoundAction),
            "opponent_action" => Some(ActionType::OpponentAction),
            "action_by" => Some(ActionType::ActionBy),
            "sequential_cost" => Some(ActionType::SequentialCost),
            "tap" => Some(ActionType::Tap),
            "rest" => Some(ActionType::Rest),
            "discard" => Some(ActionType::Discard),
            "choice_condition" => Some(ActionType::ChoiceCondition),
            "energy_condition" => Some(ActionType::EnergyCondition),
            _ => None,
        }
    }

    /// Convert the typed action back to a string representation.
    pub fn to_str(&self) -> &'static str {
        match self {
            ActionType::Draw => "draw",
            ActionType::DrawCard => "draw_card",
            ActionType::DrawUntilCount => "draw_until_count",
            ActionType::ChooseTargetPlayer => "choose_target_player",
            ActionType::MoveCards => "move_cards",
            ActionType::DiscardCard => "discard_card",
            ActionType::Select => "select",
            ActionType::SelectNumber => "select_number",
            ActionType::SelectCards => "select_cards",
            ActionType::LookAndSelect => "look_and_select",
            ActionType::LookAt => "look_at",
            ActionType::Look => "look",
            ActionType::Reveal => "reveal",
            ActionType::RevealEffect => "reveal_effect",
            ActionType::RevealPerGroup => "reveal_per_group",
            ActionType::RevealUntilLiveCard => "reveal_until_live_card",
            ActionType::RevealUntilChosenCard => "reveal_until_chosen_card",
            ActionType::ChangeState => "change_state",
            ActionType::PositionChange => "position_change",
            ActionType::Rotation => "rotation",
            ActionType::PlaceEnergyUnderMember => "place_energy_under_member",
            ActionType::ModifyRequiredHeartsSuccess => "modify_required_hearts_success",
            ActionType::GainResource => "gain_resource",
            ActionType::PayEnergy => "pay_energy",
            ActionType::GainAbility => "gain_ability",
            ActionType::GainAbilityFromSource => "gain_ability_from_source",
            ActionType::InvalidateAbility => "invalidate_ability",
            ActionType::SuppressAbilityTrigger => "suppress_ability_trigger",
            ActionType::ActivateAbility => "activate_ability",
            ActionType::ModifyScore => "modify_score",
            ActionType::ModifyRequiredHearts => "modify_required_hearts",
            ActionType::ModifyCost => "modify_cost",
            ActionType::SetCost => "set_cost",
            ActionType::SetCardIdentity => "set_card_identity",
            ActionType::SetCostToUse => "set_cost_to_use",
            ActionType::SetBladeType => "set_blade_type",
            ActionType::SetBladeCount => "set_blade_count",
            ActionType::SetHeartType => "set_heart_type",
            ActionType::SpecifyHeartColor => "specify_heart_color",
            ActionType::ChooseRequiredHearts => "choose_required_hearts",
            ActionType::Sequential => "sequential",
            ActionType::ConditionalAlternative => "conditional_alternative",
            ActionType::ConditionalOnResult => "conditional_on_result",
            ActionType::ConditionalOnOptional => "conditional_on_optional",
            ActionType::Restriction => "restriction",
            ActionType::ActivationRestriction => "activation_restriction",
            ActionType::ModifyLimit => "modify_limit",
            ActionType::Shuffle => "shuffle",
            ActionType::ReYell => "re_yell",
            ActionType::Custom => "custom",
            ActionType::DoNothing => "do_nothing",
            ActionType::Choice => "choice",
            ActionType::RepeatProcedure => "repeat_procedure",
            ActionType::DiscardUntilCount => "discard_until_count",
            ActionType::AllBladeTiming => "all_blade_timing",
            ActionType::SetCardIdentityAllRegions => "set_card_identity_all_regions",
            ActionType::ReduceLiveCardSetLimit => "reduce_live_card_set_limit",
            ActionType::PlayBatonTouch => "play_baton_touch",
            ActionType::ModifyRequiredHeartsGlobal => "modify_required_hearts_global",
            ActionType::ModifyYellCount => "modify_yell_count",
            ActionType::ActivationCost => "activation_cost",
            ActionType::PerformYell => "perform_yell",
            ActionType::ConditionalOptional => "conditional_optional",
            ActionType::CompoundAction => "compound_action",
            ActionType::OpponentAction => "opponent_action",
            ActionType::ActionBy => "action_by",
            ActionType::SequentialCost => "sequential_cost",
            ActionType::Tap => "tap",
            ActionType::Rest => "rest",
            ActionType::Discard => "discard",
            ActionType::ChoiceCondition => "choice_condition",
            ActionType::EnergyCondition => "energy_condition",
        }
    }

    /// Human-readable action label for debug/UI output.
    pub fn label(&self) -> &'static str {
        match self {
            ActionType::Draw => "Draw",
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
            ActionType::Look => "Look",
            ActionType::Reveal => "Reveal",
            ActionType::RevealEffect => "Reveal Effect",
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
            ActionType::SetCardIdentityAllRegions => "Set Card Identity All Regions",
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
            ActionType::Tap => "Tap",
            ActionType::Rest => "Rest",
            ActionType::Discard => "Discard",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "compound" => Some(Self::Compound),
            "comparison_condition" => Some(Self::ComparisonCondition),
            "location_condition" => Some(Self::LocationCondition),
            "card_count_condition" => Some(Self::CardCountCondition),
            "card_blade_condition" => Some(Self::CardBladeCondition),
            "group_condition" => Some(Self::GroupCondition),
            "position_condition" => Some(Self::PositionCondition),
            "appearance_condition" => Some(Self::AppearanceCondition),
            "temporal_condition" => Some(Self::TemporalCondition),
            "state_condition" => Some(Self::StateCondition),
            "energy_state_condition" => Some(Self::EnergyStateCondition),
            "movement_condition" => Some(Self::MovementCondition),
            "ability_filter_condition" => Some(Self::AbilityFilterCondition),
            "or_condition" => Some(Self::OrCondition),
            "any_of_condition" => Some(Self::AnyOfCondition),
            "score_threshold_condition" => Some(Self::ScoreThresholdCondition),
            "choice_condition" => Some(Self::ChoiceCondition),
            "position_change_condition" => Some(Self::PositionChangeCondition),
            "state_change_condition" => Some(Self::StateChangeCondition),
            "opponent_choice_condition" => Some(Self::OpponentChoiceCondition),
            "opponent_live_success" => Some(Self::OpponentLiveSuccess),
            "complex_condition" => Some(Self::ComplexCondition),
            "no_excess_heart" => Some(Self::NoExcessHeart),
            "otherwise_condition" => Some(Self::OtherwiseCondition),
            "both_condition" => Some(Self::BothCondition),
            "not_moved" => Some(Self::NotMoved),
            "has_moved" => Some(Self::HasMoved),
            "resource_condition" => Some(Self::ResourceCondition),
            "action_success_condition" => Some(Self::ActionSuccessCondition),
            "all_cost_comparison_condition" => Some(Self::AllCostComparisonCondition),
            "highest_cost_on_stage_condition" => Some(Self::HighestCostOnStageCondition),
            "all_revealed_match_heart_color" => Some(Self::AllRevealedMatchHeartColor),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            Self::Compound => "compound",
            Self::ComparisonCondition => "comparison_condition",
            Self::LocationCondition => "location_condition",
            Self::CardCountCondition => "card_count_condition",
            Self::CardBladeCondition => "card_blade_condition",
            Self::GroupCondition => "group_condition",
            Self::PositionCondition => "position_condition",
            Self::AppearanceCondition => "appearance_condition",
            Self::TemporalCondition => "temporal_condition",
            Self::StateCondition => "state_condition",
            Self::EnergyStateCondition => "energy_state_condition",
            Self::MovementCondition => "movement_condition",
            Self::AbilityFilterCondition => "ability_filter_condition",
            Self::OrCondition => "or_condition",
            Self::AnyOfCondition => "any_of_condition",
            Self::ScoreThresholdCondition => "score_threshold_condition",
            Self::ChoiceCondition => "choice_condition",
            Self::PositionChangeCondition => "position_change_condition",
            Self::StateChangeCondition => "state_change_condition",
            Self::OpponentChoiceCondition => "opponent_choice_condition",
            Self::OpponentLiveSuccess => "opponent_live_success",
            Self::ComplexCondition => "complex_condition",
            Self::NoExcessHeart => "no_excess_heart",
            Self::OtherwiseCondition => "otherwise_condition",
            Self::NotMoved => "not_moved",
            Self::HasMoved => "has_moved",
            Self::ResourceCondition => "resource_condition",
            Self::ActionSuccessCondition => "action_success_condition",
            Self::AllCostComparisonCondition => "all_cost_comparison_condition",
            Self::HighestCostOnStageCondition => "highest_cost_on_stage_condition",
            Self::BothCondition => "both_condition",
            Self::AllRevealedMatchHeartColor => "all_revealed_match_heart_color",
            Self::Custom => "custom",
        }
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
}

impl SelectTargetKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "choice" => Some(Self::Choice),
            "choice_string" => Some(Self::ChoiceString),
            "pay_optional_cost:skip_optional_cost" => Some(Self::PayOptionalCostSkipOptionalCost),
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
            _ => None,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            Self::Choice => "choice",
            Self::ChoiceString => "choice_string",
            Self::PayOptionalCostSkipOptionalCost => "pay_optional_cost:skip_optional_cost",
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

#[cfg(not(feature = "psp"))]
impl serde::Serialize for EffectCardType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(not(feature = "psp"))]
impl<'de> serde::Deserialize<'de> for EffectCardType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <ArcStr as serde::Deserialize>::deserialize(deserializer)?;
        Ok(EffectCardType::from_str(&s))
    }
}

#[cfg(feature = "psp")]
impl serde::Serialize for EffectCardType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "psp")]
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

#[cfg(not(feature = "psp"))]
impl serde::Serialize for EffectState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(not(feature = "psp"))]
impl<'de> serde::Deserialize<'de> for EffectState {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <ArcStr as serde::Deserialize>::deserialize(deserializer)?;
        Ok(EffectState::from_str(&s))
    }
}

#[cfg(feature = "psp")]
impl serde::Serialize for EffectState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "psp")]
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
