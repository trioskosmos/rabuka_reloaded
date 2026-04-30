use serde::{Deserialize, Serialize};
use crate::card::HeartColor;

/// Typed condition enum — one variant per condition type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Condition {
    /// Cards exist at location matching filter.
    LocationCondition {
        location: String,
        count: Option<u32>,
        operator: Option<String>,
        card_type: Option<String>,
        target: String,
        group: Option<String>,
        negation: bool,
        exclude_self: bool,
    },
    /// Count of cards matching a criterion.
    CardCountCondition {
        count: u32,
        operator: String,
        card_type: Option<String>,
        location: Option<String>,
        target: String,
        group: Option<String>,
    },
    /// Match by group name.
    GroupCondition {
        group: String,
        target: String,
        count: Option<u32>,
        operator: Option<String>,
        location: Option<String>,
    },
    /// Comparison between two values (score, cost, etc).
    ComparisonCondition {
        comparison_type: String,
        comparison_target: String,
        operator: String,
        value: Option<u32>,
        aggregate: Option<String>,
    },
    /// Temporal condition (this turn, this live, etc).
    TemporalCondition {
        temporal: String,
        count: Option<u32>,
        location: Option<String>,
        card_type: Option<String>,
        target: String,
    },
    /// Position condition (center, left, right).
    PositionCondition {
        position: String,
        target: String,
        location: Option<String>,
    },
    /// Complex condition with cause-effect.
    ComplexCondition {
        cause: Box<Condition>,
        effect: Box<Condition>,
    },
    /// Compound (AND) of multiple conditions.
    Compound(Vec<Condition>),
    /// OR of multiple conditions.
    Or(Vec<Condition>),
    /// Appearance condition (card debuted).
    AppearanceCondition {
        count: u32,
        target: String,
        location: Option<String>,
    },
    /// Position change condition.
    PositionChangeCondition {
        target: String,
        movement: String,
    },
    /// Movement condition.
    MovementCondition {
        movement: String,
        target: String,
        movement_state: Option<String>,
    },
    /// State condition (wait/active).
    StateCondition {
        state: String,
        target: String,
        count: Option<u32>,
    },
    /// Score threshold condition.
    ScoreThresholdCondition {
        value: u32,
        operator: String,
        target: String,
        aggregate: Option<String>,
    },
    /// Opponent choice condition.
    OpponentChoiceCondition {
        target: String,
    },
    /// Energy state condition.
    EnergyStateCondition {
        energy_state: String,
        count: Option<u32>,
    },
    /// Catch-all for unparseable.
    Custom {
        text: String,
    },
}

impl From<Condition> for crate::card::Condition {
    fn from(ic: Condition) -> Self {
        match ic {
            Condition::LocationCondition { location, count, operator, card_type, target, group, negation, exclude_self } => crate::card::Condition {
                condition_type: Some("location_condition".into()),
                location: Some(location), count, operator, card_type, target: Some(target),
                group_names: group.map(|g| g.split(',').map(String::from).collect()),
                negation: Some(negation),
                exclude_self: Some(exclude_self),
                ..Default::default()
            },
            Condition::CardCountCondition { count, operator, card_type, location, target, group } => crate::card::Condition {
                condition_type: Some("card_count_condition".into()),
                count: Some(count), operator: Some(operator), card_type, location, target: Some(target),
                group_names: group.map(|g| g.split(',').map(String::from).collect()),
                ..Default::default()
            },
            Condition::CardCountCondition { count, operator, card_type, location, target, group } => crate::card::Condition {
                condition_type: Some("card_count_condition".into()),
                count: Some(count), operator: Some(operator), card_type, location, target: Some(target),
                group_names: group.map(|g| g.split(',').map(String::from).collect()),
                ..Default::default()
            },
            Condition::GroupCondition { group, target, count, operator, location } => crate::card::Condition {
                condition_type: Some("group_condition".into()),
                target: Some(target), count, operator, location,
                group_names: Some(group.split(',').map(String::from).collect()),
                ..Default::default()
            },
            Condition::ComparisonCondition { comparison_type, comparison_target, operator, value, aggregate } => crate::card::Condition {
                condition_type: Some("comparison_condition".into()),
                comparison_type: Some(comparison_type), comparison_target: Some(comparison_target),
                operator: Some(operator), count: value, aggregate,
                ..Default::default()
            },
            Condition::TemporalCondition { temporal, count, location, card_type, target } => crate::card::Condition {
                condition_type: Some("temporal_condition".into()),
                temporal: Some(temporal), count, location, card_type, target: Some(target),
                ..Default::default()
            },
            Condition::PositionCondition { position, target, location } => crate::card::Condition {
                condition_type: Some("position_condition".into()),
                position: Some(crate::card::PositionInfo::String(position)),
                target: Some(target), location,
                ..Default::default()
            },
            Condition::ComplexCondition { .. } => crate::card::Condition {
                condition_type: Some("complex_condition".into()),
                ..Default::default()
            },
            Condition::Compound(conditions) => crate::card::Condition {
                condition_type: Some("compound".into()),
                conditions: Some(conditions.into_iter().map(|c| c.into()).collect()),
                ..Default::default()
            },
            Condition::Or(conditions) => crate::card::Condition {
                condition_type: Some("or_condition".into()),
                conditions: Some(conditions.into_iter().map(|c| c.into()).collect()),
                ..Default::default()
            },
            Condition::AppearanceCondition { count, target, location } => crate::card::Condition {
                condition_type: Some("appearance_condition".into()),
                count: Some(count), target: Some(target), location,
                ..Default::default()
            },
            Condition::PositionChangeCondition { target, movement } => crate::card::Condition {
                condition_type: Some("position_change_condition".into()),
                target: Some(target), movement: Some(movement),
                ..Default::default()
            },
            Condition::MovementCondition { movement, target, movement_state } => crate::card::Condition {
                condition_type: Some("movement_condition".into()),
                movement: Some(movement), target: Some(target), movement_state,
                ..Default::default()
            },
            Condition::StateCondition { state, target, count } => crate::card::Condition {
                condition_type: Some("state_condition".into()),
                state: Some(state), target: Some(target), count,
                ..Default::default()
            },
            Condition::ScoreThresholdCondition { value, operator, target, aggregate } => crate::card::Condition {
                condition_type: Some("score_threshold_condition".into()),
                count: Some(value), operator: Some(operator), target: Some(target), aggregate,
                ..Default::default()
            },
            Condition::OpponentChoiceCondition { target } => crate::card::Condition {
                condition_type: Some("opponent_choice_condition".into()),
                target: Some(target),
                ..Default::default()
            },
            Condition::EnergyStateCondition { energy_state, count } => crate::card::Condition {
                condition_type: Some("energy_state_condition".into()),
                energy_state: Some(energy_state), count,
                ..Default::default()
            },
            Condition::Custom { text } => crate::card::Condition {
                condition_type: Some("custom".into()),
                text,
                ..Default::default()
            },
        }
    }
}

impl Default for Condition {
    fn default() -> Self {
        Condition::Custom { text: String::new() }
    }
}

impl From<crate::card::Condition> for Condition {
    fn from(c: crate::card::Condition) -> Self {
        match c.condition_type.as_deref() {
            Some("location_condition") => Condition::LocationCondition {
                location: c.location.unwrap_or_default(),
                count: c.count,
                operator: c.operator,
                card_type: c.card_type,
                target: c.target.unwrap_or_else(|| "self".into()),
                group: c.group_names.map(|names| names.join(",")),
                negation: c.negation.unwrap_or(false),
                exclude_self: c.exclude_self.unwrap_or(false),
            },
            Some("card_count_condition") => Condition::CardCountCondition {
                count: c.count.unwrap_or(1),
                operator: c.operator.unwrap_or_else(|| ">=".into()),
                card_type: c.card_type,
                location: c.location,
                target: c.target.unwrap_or_else(|| "self".into()),
                group: c.group_names.map(|names| names.join(",")),
            },
            Some("group_condition") => Condition::GroupCondition {
                group: c.group_names.map(|names| names.join(",")).unwrap_or_default(),
                target: c.target.unwrap_or_else(|| "self".into()),
                count: c.count,
                operator: c.operator,
                location: c.location,
            },
            Some("comparison_condition") => Condition::ComparisonCondition {
                comparison_type: c.comparison_type.unwrap_or_else(|| "score".into()),
                comparison_target: c.comparison_target.unwrap_or_else(|| "opponent".into()),
                operator: c.operator.unwrap_or_else(|| ">".into()),
                value: c.count,
                aggregate: c.aggregate,
            },
            Some("temporal_condition") => Condition::TemporalCondition {
                temporal: c.temporal.unwrap_or_else(|| "this_turn".into()),
                count: c.count,
                location: c.location,
                card_type: c.card_type,
                target: c.target.unwrap_or_else(|| "self".into()),
            },
            Some("position_condition") => Condition::PositionCondition {
                position: c.position.as_ref().and_then(|p| p.get_position().map(String::from)).unwrap_or_default(),
                target: c.target.unwrap_or_else(|| "self".into()),
                location: c.location,
            },
            Some("complex_condition") => Condition::ComplexCondition {
                cause: Box::new(Condition::Custom { text: c.text.clone() }),
                effect: Box::new(Condition::Custom { text: String::new() }),
            },
            Some("compound") => Condition::Compound(
                c.conditions.unwrap_or_default().into_iter().map(|cc| cc.into()).collect()
            ),
            Some("or_condition") => Condition::Or(
                c.conditions.unwrap_or_default().into_iter().map(|cc| cc.into()).collect()
            ),
            Some("appearance_condition") => Condition::AppearanceCondition {
                count: c.count.unwrap_or(1),
                target: c.target.unwrap_or_else(|| "self".into()),
                location: c.location,
            },
            Some("position_change_condition") => Condition::PositionChangeCondition {
                target: c.target.unwrap_or_else(|| "self".into()),
                movement: c.movement.clone().unwrap_or_else(|| "moved".into()),
            },
            Some("movement_condition") => Condition::MovementCondition {
                movement: c.movement.clone().unwrap_or_else(|| "moved".into()),
                target: c.target.unwrap_or_else(|| "self".into()),
                movement_state: c.movement_state.clone(),
            },
            Some("state_condition") => Condition::StateCondition {
                state: c.state.unwrap_or_else(|| "wait".into()),
                target: c.target.unwrap_or_else(|| "self".into()),
                count: c.count,
            },
            Some("score_threshold_condition") => Condition::ScoreThresholdCondition {
                value: c.count.unwrap_or(1),
                operator: c.operator.unwrap_or_else(|| ">=".into()),
                target: c.target.unwrap_or_else(|| "self".into()),
                aggregate: c.aggregate,
            },
            Some("opponent_choice_condition") => Condition::OpponentChoiceCondition {
                target: c.target.unwrap_or_else(|| "self".into()),
            },
            Some("energy_state_condition") => Condition::EnergyStateCondition {
                energy_state: c.energy_state.clone().unwrap_or_else(|| "active".into()),
                count: c.count,
            },
            _ => Condition::Custom { text: c.text },
        }
    }
}
