use crate::card::Condition;

#[derive(Debug, Clone)]
pub enum ConditionEnum {
    Compound { conditions: Vec<ConditionEnum>, operator: Option<String> },
    Comparison { count: u32, operator: Option<String>, values: Option<Vec<u32>>, comparison_target: Option<String>, location: Option<String>, card_type: Option<String> },
    Location { location: String, target: String, card_type: Option<String>, comparison_type: Option<String>, operator: Option<String>, count: u32, distinct: bool, all_areas: bool, no_excess_heart: bool, baton_touch_trigger: bool, cost_limit: Option<u32>, group_names: Option<Vec<String>>, characters: Option<Vec<String>>, appearance: Option<bool> },
    Position { target: String, position: String },
    Group { location: Option<String>, target: String, group_names: Option<Vec<String>>, count: u32, operator: Option<String> },
    CardCount { card_type: String, target: String, count: u32, operator: Option<String> },
    Appearance { appearance: bool, location: String, target: String, baton_touch_trigger: bool },
    Temporal { temporal: String, phase: Option<String> },
    State { state: String, target: String, resource_type: Option<String>, all: bool },
    EnergyState { energy_state: String, target: String },
    Movement { movement: String, movement_state: Option<String>, location: String, target: String, baton_touch_trigger: bool },
    AbilityNegation { negation: bool },
    Or { conditions: Vec<ConditionEnum> },
    AnyOf { any_of: Vec<String> },
    ScoreThreshold { count: u32, operator: Option<String>, target: String },
    Choice,
    PositionChange { optional: bool },
    StateChange { text: String },
    OpponentChoice { negation: bool, target: String },
    OpponentLiveSuccess,
    Complex,
}

impl ConditionEnum {
    pub fn from_condition(condition: &Condition) -> Self {
        match condition.condition_type.as_deref() {
            Some("compound") => {
                let conditions = condition.conditions.as_ref()
                    .map(|c| c.iter().map(Self::from_condition).collect())
                    .unwrap_or_default();
                ConditionEnum::Compound {
                    conditions,
                    operator: condition.operator.clone(),
                }
            }
            Some("comparison_condition") => ConditionEnum::Comparison {
                count: condition.count.unwrap_or(0),
                operator: condition.operator.clone(),
                values: condition.values.clone(),
                comparison_target: condition.comparison_target.clone(),
                location: condition.location.clone(),
                card_type: condition.card_type.clone(),
            },
            Some("location_condition") => ConditionEnum::Location {
                location: condition.location.as_deref().unwrap_or_default().to_string(),
                target: condition.target.as_deref().unwrap_or("self").to_string(),
                card_type: condition.card_type.clone(),
                comparison_type: condition.comparison_type.clone(),
                operator: condition.operator.clone(),
                count: condition.count.unwrap_or(if condition.cost_limit.is_some() || condition.card_type.is_some() || condition.group_names.is_some() || condition.characters.is_some() || condition.distinct.unwrap_or(false) || condition.appearance.unwrap_or(false) { 1 } else { 0 }),
                distinct: condition.distinct.unwrap_or(false),
                all_areas: condition.all_areas.unwrap_or(false),
                no_excess_heart: condition.no_excess_heart.unwrap_or(false),
                baton_touch_trigger: condition.baton_touch_trigger.unwrap_or(false),
                cost_limit: condition.cost_limit,
                group_names: condition.group_names.clone(),
                characters: condition.characters.clone(),
                appearance: condition.appearance,
            },
            Some("position_condition") => ConditionEnum::Position {
                target: condition.target.as_deref().unwrap_or("self").to_string(),
                position: condition.position.as_ref().and_then(|p| p.get_position()).unwrap_or_default().to_string(),
            },
            Some("group_condition") => ConditionEnum::Group {
                location: condition.location.clone(),
                target: condition.target.as_deref().unwrap_or("self").to_string(),
                group_names: condition.group_names.clone(),
                count: condition.count.unwrap_or(1),
                operator: condition.operator.clone(),
            },
            Some("card_count_condition") => ConditionEnum::CardCount {
                card_type: condition.card_type.as_deref().unwrap_or_default().to_string(),
                target: condition.target.as_deref().unwrap_or("self").to_string(),
                count: condition.count.unwrap_or(1),
                operator: condition.operator.clone(),
            },
            Some("appearance_condition") => ConditionEnum::Appearance {
                appearance: condition.appearance.unwrap_or(false),
                location: condition.location.as_deref().unwrap_or_default().to_string(),
                target: condition.target.as_deref().unwrap_or("self").to_string(),
                baton_touch_trigger: condition.baton_touch_trigger.unwrap_or(false),
            },
            Some("temporal_condition") => ConditionEnum::Temporal {
                temporal: condition.temporal.as_deref().unwrap_or_default().to_string(),
                phase: condition.phase.clone(),
            },
            Some("state_condition") => ConditionEnum::State {
                state: condition.state.as_deref().unwrap_or_default().to_string(),
                target: condition.target.as_deref().unwrap_or("self").to_string(),
                resource_type: condition.resource_type.clone(),
                all: condition.all.unwrap_or(false),
            },
            Some("energy_state_condition") => ConditionEnum::EnergyState {
                energy_state: condition.energy_state.as_deref().unwrap_or_default().to_string(),
                target: condition.target.as_deref().unwrap_or("self").to_string(),
            },
            Some("movement_condition") => ConditionEnum::Movement {
                movement: condition.movement.as_deref().unwrap_or_default().to_string(),
                movement_state: condition.movement_state.clone(),
                location: condition.location.as_deref().unwrap_or_default().to_string(),
                target: condition.target.as_deref().unwrap_or("self").to_string(),
                baton_touch_trigger: condition.baton_touch_trigger.unwrap_or(false),
            },
            Some("ability_negation_condition") => ConditionEnum::AbilityNegation {
                negation: condition.negation.unwrap_or(false),
            },
            Some("or_condition") => {
                let conditions = condition.conditions.as_ref()
                    .map(|c| c.iter().map(Self::from_condition).collect())
                    .unwrap_or_default();
                ConditionEnum::Or { conditions }
            }
            Some("any_of_condition") => ConditionEnum::AnyOf {
                any_of: condition.any_of.clone().unwrap_or_default(),
            },
            Some("score_threshold_condition") => ConditionEnum::ScoreThreshold {
                count: condition.count.unwrap_or(1),
                operator: condition.operator.clone(),
                target: condition.target.as_deref().unwrap_or("self").to_string(),
            },
            Some("choice_condition") => ConditionEnum::Choice,
            Some("position_change_condition") => ConditionEnum::PositionChange {
                optional: condition.options.is_some(),
            },
            Some("state_change_condition") => ConditionEnum::StateChange {
                text: condition.text.clone(),
            },
            Some("opponent_choice_condition") => ConditionEnum::OpponentChoice {
                negation: condition.negation.unwrap_or(false),
                target: condition.target.as_deref().unwrap_or("opponent").to_string(),
            },
            Some("opponent_live_success") => ConditionEnum::OpponentLiveSuccess,
            Some("complex_condition") => ConditionEnum::Complex,
            _ => {
                eprintln!("Unknown condition type: {:?}", condition.condition_type);
                ConditionEnum::Comparison { count: 0, operator: None, values: None, comparison_target: None, location: None, card_type: None }
            }
        }
    }
}
