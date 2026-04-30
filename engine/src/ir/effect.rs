use serde::{Deserialize, Serialize};
use crate::card::{AbilityEffect, CardType, HeartColor};

/// Target player for an effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    Self_,    // "self" is reserved keyword
    Opponent,
    Both,
    Either,
    Player(String),
}

/// Duration of a temporary effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Duration {
    ThisTurn,
    ThisLive,
    LiveEnd,
    Permanent,
    AsLongAs,
}

/// Types of resources that can be gained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resource {
    Blade,
    Heart(HeartColor),
    Energy,
    Score,
    Draw,
    Custom(String),
}

/// Count representation — either a fixed number or dynamic (per-unit, player choice, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Count {
    Fixed(u32),
    PerUnit { per_unit: u32, unit_type: String },
    Variable,
    All,
    UpTo(u32),
}

/// Frozen state representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateChange {
    Wait,
    Active,
}

/// Typed effect enum — one variant per action type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Effect {
    /// Move cards between zones.
    MoveCards {
        source: super::Zone,
        destination: super::Zone,
        count: Count,
        card_type: Option<CardType>,
        target: Target,
        group: Option<String>,
        cost_limit: Option<u32>,
        total_cost_limit: Option<u32>,
        placement_order: Option<String>, // "any_order"
        state_change: Option<StateChange>,
        shuffle: bool,
        optional: bool,
        distinct: Option<String>,
        self_cost: bool,
        exclude_self: bool,
    },
    /// Draw cards from deck to hand.
    DrawCards {
        count: Count,
        target: Target,
        optional: bool,
    },
    /// Gain resources (blades, hearts, energy).
    GainResource {
        resource: Resource,
        count: Count,
        target: Target,
        duration: Option<Duration>,
        group: Option<String>,
        per_unit: Option<Count>,
    },
    /// Change member state (wait → active or vice versa).
    ChangeState {
        state_change: StateChange,
        count: Count,
        target: Target,
        card_type: Option<CardType>,
        cost_limit: Option<u32>,
        group: Option<String>,
        optional: bool,
    },
    /// Look at cards from a zone.
    LookAt {
        source: super::Zone,
        count: Count,
        target: Target,
    },
    /// Reveal cards from a zone.
    Reveal {
        source: super::Zone,
        count: Count,
        target: Target,
        card_type: Option<CardType>,
        heart_colors: Option<Vec<HeartColor>>,
    },
    /// Select cards from looked-at/revealed area.
    Select {
        source: super::Zone,
        count: Count,
        target: Target,
        card_type: Option<CardType>,
        optional: bool,
    },
    /// Look at cards, then select and take action.
    LookAndSelect {
        look_action: Box<Effect>,
        select_action: Box<Effect>,
        target: Target,
    },
    /// Modify score of a live card.
    ModifyScore {
        operation: String, // "add", "subtract", "set"
        value: u32,
        target: Target,
        duration: Option<Duration>,
        card_type: Option<CardType>,
        group: Option<String>,
        per_unit: Option<Count>,
    },
    /// Modify required hearts for live success.
    ModifyRequiredHearts {
        operation: String, // "increase", "decrease"
        value: u32,
        heart_color: Option<HeartColor>,
        target: Target,
        duration: Option<Duration>,
    },
    /// Change member position on stage.
    PositionChange {
        count: Count,
        target: Target,
        card_type: Option<CardType>,
        group: Option<String>,
        optional: bool,
    },
    /// Cause a member to appear on stage (from hand or other zone).
    Appear {
        source: super::Zone,
        destination: super::Zone,
        count: Count,
        target: Target,
        card_type: Option<CardType>,
    },
    /// Gain an ability from another card.
    GainAbility {
        ability_text: String,
        target: Target,
        duration: Option<Duration>,
    },
    /// Present the player with a choice between effects.
    Choice {
        options: Vec<Effect>,
        choice_type: Option<String>,
        target: Target,
    },
    /// Conditional: if condition, do primary; otherwise do alternative.
    ConditionalAlternative {
        condition: super::Condition,
        primary: Box<Effect>,
        alternative: Option<Box<Effect>>,
        alternative_condition: Option<super::Condition>,
    },
    /// Sequential execution of multiple effects.
    Sequential(Vec<Effect>),
    /// Restriction / prohibition effect.
    Restriction {
        restriction_type: String, // "cannot_activate", "cannot_live", "cannot_place"
        target: Target,
        duration: Option<Duration>,
        card_type: Option<CardType>,
        restricted_destination: Option<super::Zone>,
        condition: Option<super::Condition>,
    },
    /// Re-do the cheer (yell) phase.
    ReYell {
        lose_blade_hearts: bool,
        condition: Option<super::Condition>,
    },
    /// Pay energy (cost variant).
    PayEnergy {
        energy: u32,
        target: Target,
        optional: bool,
    },
    /// Draw until a count condition is met.
    DrawUntilCount {
        target_count: u32,
        source: super::Zone,
        target: Target,
        condition: Option<super::Condition>,
    },
    /// Discard until a count condition is met.
    DiscardUntilCount {
        hand_size: u32,
        target: Target,
    },
    /// Modify cost of cards.
    ModifyCost {
        operation: String, // "add", "subtract", "set"
        value: i32,
        target: Target,
        per_unit: Option<Count>,
        duration: Option<Duration>,
    },
    /// Specify heart color.
    SpecifyHeartColor {
        choice: bool,
        target: Target,
    },
    /// Set the type of all blades on a card.
    SetBladeType {
        blade_type: String,
        target: Target,
    },
    /// Set all hearts to a specific type.
    SetHeartType {
        heart_type: String,
        target: Target,
    },
    /// Set score to a specific value.
    SetScore {
        value: u32,
        target: Target,
    },
    /// Set required hearts to a specific value.
    SetRequiredHearts {
        value: u32,
        target: Target,
    },
    /// Set blade count to a specific value.
    SetBladeCount {
        value: u32,
        target: Target,
    },
    /// Set card identity / group.
    SetCardIdentity {
        identities: Vec<String>,
        target: Target,
    },
    /// Modify the number of cards revealed during cheer.
    ModifyYellCount {
        operation: String, // "add", "subtract"
        count: u32,
        target: Target,
    },
    /// Modify hand size limit.
    ModifyLimit {
        operation: String,
        amount: u32,
        target: Target,
    },
    /// Invalidate an ability.
    InvalidateAbility {
        optional: bool,
        target: Target,
    },
    /// Choose required hearts from options.
    ChooseRequiredHearts {
        optional: bool,
        target: Target,
    },
    /// Baton touch (member replacement).
    PlayBatonTouch {
        count: Count,
        target: Target,
    },
    /// Place energy card under a member.
    PlaceEnergyUnderMember {
        count: Count,
        target: Target,
    },
    /// Activation cost (hand cost for activation abilities).
    ActivationCost {
        cost: u32,
        target: Target,
    },
    /// Activation restriction.
    ActivationRestriction {
        target: Target,
    },
    /// Modify required hearts globally for all live cards.
    ModifyRequiredHeartsGlobal {
        operation: String,
        value: u32,
        target: Target,
    },
    /// Custom / fallback for unparseable abilities.
    Custom {
        text: String,
    },
}

impl Effect {
    /// Convert from the flat `AbilityEffect` struct produced by the parser.
    pub fn from_ability_effect(e: &AbilityEffect) -> Self {
        // Map common fields
        let target = match e.target.as_deref() {
            Some("self") => Target::Self_,
            Some("opponent") => Target::Opponent,
            Some("both") => Target::Both,
            Some("either") => Target::Either,
            Some(t) => Target::Player(t.to_string()),
            None => Target::Self_,
        };

        let count = match e.count {
            Some(n) => Count::Fixed(n),
            None => Count::Fixed(1),
        };

        let dur = match e.duration.as_deref() {
            Some("this_turn") => Some(Duration::ThisTurn),
            Some("this_live") => Some(Duration::ThisLive),
            Some("live_end") => Some(Duration::LiveEnd),
            Some("permanent") | None => Some(Duration::Permanent),
            Some("as_long_as") => Some(Duration::AsLongAs),
            Some(_) => Some(Duration::Permanent),
        };

        match e.action.as_str() {
            "move_cards" => Effect::MoveCards {
                source: zone_from_str(e.source.as_deref().unwrap_or("")),
                destination: zone_from_str(e.destination.as_deref().unwrap_or("")),
                count,
                card_type: e.card_type.as_deref().and_then(parse_card_type),
                target,
                group: e.group.as_ref().map(|g| g.name.clone()),
                cost_limit: e.cost_limit,
                total_cost_limit: e.total_cost_limit,
                placement_order: e.placement_order.clone(),
                state_change: e.state_change.as_deref().and_then(parse_state_change),
                shuffle: e.shuffle_target.is_some(),
                optional: e.optional.unwrap_or(false),
                distinct: e.distinct.clone(),
                self_cost: e.self_cost.unwrap_or(false),
                exclude_self: e.exclude_self.unwrap_or(false),
            },
            "draw_card" | "draw" => Effect::DrawCards {
                count,
                target,
                optional: e.optional.unwrap_or(false),
            },
            "gain_resource" => Effect::GainResource {
                resource: parse_resource(e.resource.as_deref().unwrap_or(""), e.heart_color.as_deref()),
                count,
                target,
                duration: dur,
                group: e.group.as_ref().map(|g| g.name.clone()),
                per_unit: if e.per_unit.unwrap_or(false) {
                    Some(Count::PerUnit {
                        per_unit: e.per_unit_count.unwrap_or(1),
                        unit_type: e.per_unit_type.clone().unwrap_or_else(|| "member".into()),
                    })
                } else { None },
            },
            "change_state" => Effect::ChangeState {
                state_change: parse_state_change(e.state_change.as_deref().unwrap_or("")).unwrap_or(StateChange::Wait),
                count,
                target,
                card_type: e.card_type.as_deref().and_then(parse_card_type),
                cost_limit: e.cost_limit,
                group: e.group.as_ref().map(|g| g.name.clone()),
                optional: e.optional.unwrap_or(false),
            },
            "look_at" => Effect::LookAt {
                source: zone_from_str(e.source.as_deref().unwrap_or("")),
                count,
                target,
            },
            "reveal" => Effect::Reveal {
                source: zone_from_str(e.source.as_deref().unwrap_or("")),
                count,
                target,
                card_type: e.card_type.as_deref().and_then(parse_card_type),
                heart_colors: e.heart_colors.as_ref().map(|colors| {
                    colors.iter().filter_map(|s| parse_heart_color(s)).collect()
                }),
            },
            "select" => Effect::Select {
                source: zone_from_str(e.source.as_deref().unwrap_or("")),
                count,
                target,
                card_type: e.card_type.as_deref().and_then(parse_card_type),
                optional: e.optional.unwrap_or(false),
            },
            "look_and_select" => Effect::LookAndSelect {
                look_action: Box::new(if let Some(ref la) = e.look_action {
                    Effect::from_ability_effect(la)
                } else {
                    Effect::Custom { text: "missing look_action".into() }
                }),
                select_action: Box::new(if let Some(ref sa) = e.select_action {
                    Effect::from_ability_effect(sa)
                } else {
                    Effect::Custom { text: "missing select_action".into() }
                }),
                target,
            },
            "modify_score" => Effect::ModifyScore {
                operation: e.operation.clone().unwrap_or_else(|| "add".into()),
                value: e.value.unwrap_or(1),
                target,
                duration: dur,
                card_type: e.card_type.as_deref().and_then(parse_card_type),
                group: e.group.as_ref().map(|g| g.name.clone()),
                per_unit: if e.per_unit.unwrap_or(false) {
                    Some(Count::PerUnit { per_unit: e.per_unit_count.unwrap_or(1), unit_type: e.per_unit_type.clone().unwrap_or_else(|| "member".into()) })
                } else { None },
            },
            "modify_required_hearts" => Effect::ModifyRequiredHearts {
                operation: e.operation.clone().unwrap_or_else(|| "decrease".into()),
                value: e.value.unwrap_or(1),
                heart_color: e.heart_color.as_deref().and_then(parse_heart_color),
                target,
                duration: dur,
            },
            "position_change" => Effect::PositionChange {
                count,
                target,
                card_type: e.card_type.as_deref().and_then(parse_card_type),
                group: e.group.as_ref().map(|g| g.name.clone()),
                optional: e.optional.unwrap_or(false),
            },
            "appear" => Effect::Appear {
                source: zone_from_str(e.source.as_deref().unwrap_or("hand")),
                destination: zone_from_str(e.destination.as_deref().unwrap_or("stage")),
                count,
                target,
                card_type: e.card_type.as_deref().and_then(parse_card_type),
            },
            "gain_ability" => Effect::GainAbility {
                ability_text: e.ability_gain.clone().unwrap_or_default(),
                target,
                duration: dur,
            },
            "choice" => Effect::Choice {
                options: e.options.as_ref().map(|opts| opts.iter().map(Effect::from_ability_effect).collect()).unwrap_or_default(),
                choice_type: e.choice_type.clone(),
                target,
            },
            "conditional_alternative" => Effect::ConditionalAlternative {
                condition: e.condition.as_ref().map(|c| c.clone().into()).unwrap_or_default(),
                primary: Box::new(e.primary_effect.as_ref().map(|pe| Effect::from_ability_effect(pe)).unwrap_or_else(|| Effect::Custom { text: "missing primary".into() })),
                alternative: e.alternative_effect.as_ref().map(|ae| Box::new(Effect::from_ability_effect(ae))),
                alternative_condition: e.alternative_condition.as_ref().map(|c| c.clone().into()),
            },
            "sequential" => Effect::Sequential(
                e.actions.as_ref().map(|acts| acts.iter().map(Effect::from_ability_effect).collect()).unwrap_or_default()
            ),
            "restriction" => Effect::Restriction {
                restriction_type: e.restriction_type.clone().unwrap_or_else(|| "cannot_activate".into()),
                target,
                duration: dur,
                card_type: e.card_type.as_deref().and_then(parse_card_type),
                restricted_destination: e.restricted_destination.as_deref().map(zone_from_str),
                condition: e.condition.as_ref().map(|c| c.clone().into()),
            },
            "re_yell" => Effect::ReYell {
                lose_blade_hearts: e.lose_blade_hearts.unwrap_or(false),
                condition: e.condition.as_ref().map(|c| c.clone().into()),
            },
            "pay_energy" => Effect::PayEnergy {
                energy: e.count.unwrap_or(1),
                target,
                optional: e.optional.unwrap_or(false),
            },
            "draw_until_count" => Effect::DrawUntilCount {
                target_count: e.target_count.unwrap_or(e.count.unwrap_or(5)),
                source: zone_from_str(e.source.as_deref().unwrap_or("deck")),
                target,
                condition: e.condition.as_ref().map(|c| c.clone().into()),
            },
            "discard_until_count" => Effect::DiscardUntilCount {
                hand_size: e.count.unwrap_or(5),
                target,
            },
            "modify_cost" => Effect::ModifyCost {
                operation: e.operation.clone().unwrap_or_else(|| "add".into()),
                value: e.value.unwrap_or(0) as i32,
                target,
                per_unit: if e.per_unit.unwrap_or(false) {
                    Some(Count::PerUnit { per_unit: e.per_unit_count.unwrap_or(1), unit_type: e.per_unit_type.clone().unwrap_or_else(|| "member".into()) })
                } else { None },
                duration: dur,
            },
            "specify_heart_color" => Effect::SpecifyHeartColor {
                choice: e.choice.unwrap_or(false),
                target,
            },
            "set_blade_type" => Effect::SetBladeType {
                blade_type: e.blade_type.clone().unwrap_or_else(|| "all".into()),
                target,
            },
            "set_heart_type" => Effect::SetHeartType {
                heart_type: e.heart_type.clone().unwrap_or_else(|| "heart00".into()),
                target,
            },
            "set_score" => Effect::SetScore {
                value: e.value.unwrap_or(0),
                target,
            },
            "set_required_hearts" => Effect::SetRequiredHearts {
                value: e.value.unwrap_or(0),
                target,
            },
            "set_blade_count" => Effect::SetBladeCount {
                value: e.value.unwrap_or(0),
                target,
            },
            "set_card_identity" => Effect::SetCardIdentity {
                identities: e.identities.clone().unwrap_or_default(),
                target,
            },
            "modify_yell_count" => Effect::ModifyYellCount {
                operation: e.operation.clone().unwrap_or_else(|| "subtract".into()),
                count: e.count.unwrap_or(0),
                target,
            },
            "modify_limit" => Effect::ModifyLimit {
                operation: e.operation.clone().unwrap_or_else(|| "decrease".into()),
                amount: e.value.unwrap_or(1),
                target,
            },
            "invalidate_ability" => Effect::InvalidateAbility {
                optional: e.optional.unwrap_or(false),
                target,
            },
            "choose_required_hearts" => Effect::ChooseRequiredHearts {
                optional: e.optional.unwrap_or(false),
                target,
            },
            "play_baton_touch" => Effect::PlayBatonTouch {
                count,
                target,
            },
            "place_energy_under_member" => Effect::PlaceEnergyUnderMember {
                count,
                target,
            },
            "activation_cost" => Effect::ActivationCost {
                cost: e.count.unwrap_or(1),
                target,
            },
            "activation_restriction" => Effect::ActivationRestriction {
                target,
            },
            "modify_required_hearts_global" => Effect::ModifyRequiredHeartsGlobal {
                operation: e.operation.clone().unwrap_or_else(|| "increase".into()),
                value: e.value.unwrap_or(1),
                target,
            },
            _ => Effect::Custom {
                text: e.text.clone(),
            },
        }
    }
}

// ====== Conversion helpers ======

pub fn zone_from_str(s: &str) -> super::Zone {
    match s {
        "hand" => super::Zone::Hand,
        "deck" | "main_deck" => super::Zone::MainDeck,
        "deck_top" => super::Zone::DeckTop,
        "deck_bottom" => super::Zone::DeckBottom,
        "discard" | "waitroom" => super::Zone::Waitroom,
        "stage" | "stage_center" | "center" => super::Zone::StageCenter,
        "left_side" | "left" => super::Zone::StageLeft,
        "right_side" | "right" => super::Zone::StageRight,
        "energy_zone" => super::Zone::EnergyZone,
        "energy_deck" => super::Zone::EnergyDeck,
        "live_card_zone" => super::Zone::LiveCardZone,
        "success_live_zone" | "success_live_card_zone" => super::Zone::SuccessLiveZone,
        "looked_at" => super::Zone::LookedAt,
        "revealed" => super::Zone::Revealed,
        "exclusion" => super::Zone::Exclusion,
        "under_member" => super::Zone::UnderMember,
        "same_area" => super::Zone::SameArea,
        "empty_area" => super::Zone::EmptyArea,
        _ => super::Zone::LookedAt, // fallback
    }
}

fn parse_card_type(s: &str) -> Option<CardType> {
    match s {
        "member_card" | "メンバー" => Some(CardType::Member),
        "live_card" | "ライブ" => Some(CardType::Live),
        "energy_card" | "エネルギー" => Some(CardType::Energy),
        _ => None,
    }
}

fn parse_state_change(s: &str) -> Option<StateChange> {
    match s {
        "wait" => Some(StateChange::Wait),
        "active" => Some(StateChange::Active),
        _ => None,
    }
}

fn parse_resource(res: &str, heart_color: Option<&str>) -> Resource {
    match res {
        "blade" | "ブレード" => Resource::Blade,
        "energy" | "E" => Resource::Energy,
        "score" => Resource::Score,
        "draw" => Resource::Draw,
        s if s.starts_with("heart") || s == "ハート" => {
            if let Some(hc) = heart_color {
                if let Some(color) = parse_heart_color(hc) {
                    Resource::Heart(color)
                } else {
                    Resource::Heart(HeartColor::Heart00)
                }
            } else {
                Resource::Heart(HeartColor::Heart00)
            }
        }
        _ => Resource::Custom(res.to_string()),
    }
}

fn parse_heart_color(s: &str) -> Option<HeartColor> {
    match s {
        "heart00" | "heart_00" | "heart0" => Some(HeartColor::Heart00),
        "heart01" | "heart_01" => Some(HeartColor::Heart01),
        "heart02" | "heart_02" => Some(HeartColor::Heart02),
        "heart03" | "heart_03" => Some(HeartColor::Heart03),
        "heart04" | "heart_04" => Some(HeartColor::Heart04),
        "heart05" | "heart_05" => Some(HeartColor::Heart05),
        "heart06" | "heart_06" => Some(HeartColor::Heart06),
        _ => None,
    }
}
