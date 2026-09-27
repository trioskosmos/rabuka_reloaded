// ============================================================================
// Hand edits are silently wiped by regeneration and fail CI freshness checks.
// Change cards/generate_condition_decoder.py instead, then re-run it.
// ============================================================================
// Re-run: python cards/generate_condition_decoder.py
//
// Direct decoder for the serde internally-tagged `Condition` enum.
// `text`/`trigger_event` exist only under the `debug_conditions` feature
// and are skipped otherwise.
use crate::card::{ConditionCommon, DistinctInfo};
#[cfg(feature = "debug_conditions")]
use crate::card::TriggerEvent;

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
/// Accumulator for Condition fields during direct decode.
#[derive(Default)]
struct ConditionLocals {
    pub ability_filter: Option<ArcStr>,
    pub ability_filter_triggers: Option<Vec<String>>,
    pub action_reference: Option<ArcStr>,
    pub activation_position: Option<ArcStr>,
    pub aggregate: Option<ArcStr>,
    pub all: Option<bool>,
    pub all_areas: Option<bool>,
    pub all_members: Option<bool>,
    pub any_of: Option<Vec<String>>,
    pub appearance: Option<bool>,
    pub appearance_source: Option<ArcStr>,
    pub area_direction: Option<ArcStr>,
    pub baton_touch_source: Option<ArcStr>,
    pub baton_touch_trigger: Option<bool>,
    pub blade_greater_than_all: Option<bool>,
    pub blade_limit: Option<u8>,
    pub blade_limit_operator: Option<Operator>,
    pub cache: Option<bool>,
    pub card_names: Option<Vec<String>>,
    pub card_property: Option<ArcStr>,
    pub card_type: Option<ArcStr>,
    pub cause: Option<Box<Condition>>,
    pub characters: Option<Vec<String>>,
    pub check_self: Option<bool>,
    pub comparison_source: Option<ArcStr>,
    pub comparison_target: Option<ArcStr>,
    pub comparison_type: Option<ArcStr>,
    pub condition: Option<Box<Condition>>,
    pub conditions: Option<Vec<Box<Condition>>>,
    pub cost_limit: Option<u8>,
    pub cost_limit_operator: Option<Operator>,
    pub cost_reference_character: Option<ArcStr>,
    pub cost_reference_operator: Option<Operator>,
    pub cost_total: Option<u8>,
    pub cost_total_operator: Option<Operator>,
    pub count: Option<u8>,
    pub delta: Option<bool>,
    pub destination: Option<ArcStr>,
    pub distinct: Option<Box<DistinctInfo>>,
    pub effect: Option<Box<AbilityEffect>>,
    pub energy_placed: Option<bool>,
    pub energy_state: Option<ArcStr>,
    pub exclude_characters: Option<Vec<String>>,
    pub exclude_group_names: Option<Vec<String>>,
    pub exclude_self: Option<bool>,
    pub from_state: Option<ArcStr>,
    pub group_names: Option<Vec<String>>,
    pub group_reference: Option<ArcStr>,
    pub heart_colors: Option<Vec<String>>,
    pub heart_source: Option<ArcStr>,
    pub heart_type: Option<ArcStr>,
    pub location: Option<ArcStr>,
    pub locations: Option<Vec<String>>,
    pub min_baton_touch_count: Option<u8>,
    pub movement: Option<ArcStr>,
    pub negation: Option<bool>,
    pub no_excess_heart: Option<bool>,
    pub operator: Option<ArcStr>,
    pub options: Option<Vec<Box<AbilityEffect>>>,
    pub original_value: Option<bool>,
    pub phase: Option<ArcStr>,
    pub phase_target: Option<ArcStr>,
    pub position: Option<Box<PositionInfo>>,
    pub position_compare: Option<ArcStr>,
    pub positions_characters: Option<Vec<PositionCharacter>>,
    pub reference_card: Option<ArcStr>,
    pub require_position_cards: Option<bool>,
    pub resource_type: Option<ArcStr>,
    pub same_name: Option<bool>,
    pub scope: Option<ArcStr>,
    pub self_effect_only: Option<bool>,
    pub self_target: Option<bool>,
    pub shuffle: Option<bool>,
    pub source: Option<ArcStr>,
    pub state: Option<ArcStr>,
    pub sub_checks: Option<Box<LocationSubChecks>>,
    pub target: Option<ArcStr>,
    pub temporal: Option<ArcStr>,
    pub temporal_scope: Option<ArcStr>,
    #[cfg(feature = "debug_conditions")]
    pub text: Option<String>,
    pub to_state: Option<ArcStr>,
    #[cfg(feature = "debug_conditions")]
    pub trigger_event: Option<Box<TriggerEvent>>,
    pub turn_number: Option<u8>,
    pub unit: Option<ArcStr>,
    pub values: Option<Vec<u8>>,
    pub yell_trigger: Option<bool>,
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
/// Read one field from a condition object.
/// Returns true if the field was recognized and consumed, false to skip.
fn decode_condition_field(
    bc: &mut BcReader,
    key: &str,
    l: &mut ConditionLocals,
) -> Option<bool> {
    match key {
            "ability_filter" => { l.ability_filter = bc.read_arc_str_value(); Some(true) }
            "ability_filter_triggers" => { l.ability_filter_triggers = bc.read_opt_str_vec_value().map(|b| *b); Some(true) }
            "action_reference" => { l.action_reference = bc.read_arc_str_value(); Some(true) }
            "activation_position" => { l.activation_position = bc.read_arc_str_value(); Some(true) }
            "aggregate" => { l.aggregate = bc.read_arc_str_value(); Some(true) }
            "all" => { l.all = bc.read_bool_value(); Some(true) }
            "all_areas" => { l.all_areas = bc.read_bool_value(); Some(true) }
            "all_members" => { l.all_members = bc.read_bool_value(); Some(true) }
            "any_of" => { l.any_of = bc.read_opt_str_vec_value().map(|b| *b); Some(true) }
            "appearance" => { l.appearance = bc.read_bool_value(); Some(true) }
            "appearance_source" => { l.appearance_source = bc.read_arc_str_value(); Some(true) }
            "area_direction" => { l.area_direction = bc.read_arc_str_value(); Some(true) }
            "baton_touch_source" => { l.baton_touch_source = bc.read_arc_str_value(); Some(true) }
            "baton_touch_trigger" => { l.baton_touch_trigger = bc.read_bool_value(); Some(true) }
            "blade_greater_than_all" => { l.blade_greater_than_all = bc.read_bool_value(); Some(true) }
            "blade_limit" => { l.blade_limit = bc.read_u8_value(); Some(true) }
            "blade_limit_operator" => { l.blade_limit_operator = bc.read_operator_value(); Some(true) }
            "cache" => { l.cache = bc.read_bool_value(); Some(true) }
            "card_names" => { l.card_names = bc.read_opt_str_vec_value().map(|b| *b); Some(true) }
            "card_property" => { l.card_property = bc.read_arc_str_value(); Some(true) }
            "card_type" => { l.card_type = bc.read_arc_str_value(); Some(true) }
            "cause" => { l.cause = bc.read_condition_value(); Some(true) }
            "characters" => { l.characters = bc.read_opt_str_vec_value().map(|b| *b); Some(true) }
            "check_self" => { l.check_self = bc.read_bool_value(); Some(true) }
            "comparison_source" => { l.comparison_source = bc.read_arc_str_value(); Some(true) }
            "comparison_target" => { l.comparison_target = bc.read_arc_str_value(); Some(true) }
            "comparison_type" => { l.comparison_type = bc.read_arc_str_value(); Some(true) }
            "condition" => { l.condition = bc.read_condition_value(); Some(true) }
            "conditions" => { l.conditions = bc.read_condition_vec_value(); Some(true) }
            "cost_limit" => { l.cost_limit = bc.read_u8_value(); Some(true) }
            "cost_limit_operator" => { l.cost_limit_operator = bc.read_operator_value(); Some(true) }
            "cost_reference_character" => { l.cost_reference_character = bc.read_arc_str_value(); Some(true) }
            "cost_reference_operator" => { l.cost_reference_operator = bc.read_operator_value(); Some(true) }
            "cost_total" => { l.cost_total = bc.read_u8_value(); Some(true) }
            "cost_total_operator" => { l.cost_total_operator = bc.read_operator_value(); Some(true) }
            "count" => { l.count = bc.read_u8_value(); Some(true) }
            "delta" => { l.delta = bc.read_bool_value(); Some(true) }
            "destination" => { l.destination = bc.read_arc_str_value(); Some(true) }
            "distinct" => { l.distinct = bc.read_distinct_info_value(); Some(true) }
            "effect" => { l.effect = bc.read_effect_value(); Some(true) }
            "energy_placed" => { l.energy_placed = bc.read_bool_value(); Some(true) }
            "energy_state" => { l.energy_state = bc.read_arc_str_value(); Some(true) }
            "exclude_characters" => { l.exclude_characters = bc.read_opt_str_vec_value().map(|b| *b); Some(true) }
            "exclude_group_names" => { l.exclude_group_names = bc.read_opt_str_vec_value().map(|b| *b); Some(true) }
            "exclude_self" => { l.exclude_self = bc.read_bool_value(); Some(true) }
            "from_state" => { l.from_state = bc.read_arc_str_value(); Some(true) }
            "group_names" => { l.group_names = bc.read_opt_str_vec_value().map(|b| *b); Some(true) }
            "group_reference" => { l.group_reference = bc.read_arc_str_value(); Some(true) }
            "heart_colors" => { l.heart_colors = bc.read_opt_str_vec_value().map(|b| *b); Some(true) }
            "heart_source" => { l.heart_source = bc.read_arc_str_value(); Some(true) }
            "heart_type" => { l.heart_type = bc.read_arc_str_value(); Some(true) }
            "location" => { l.location = bc.read_arc_str_value(); Some(true) }
            "locations" => { l.locations = bc.read_opt_str_vec_value().map(|b| *b); Some(true) }
            "min_baton_touch_count" => { l.min_baton_touch_count = bc.read_u8_value(); Some(true) }
            "movement" => { l.movement = bc.read_arc_str_value(); Some(true) }
            "negation" => { l.negation = bc.read_bool_value(); Some(true) }
            "no_excess_heart" => { l.no_excess_heart = bc.read_bool_value(); Some(true) }
            "operator" => { l.operator = bc.read_arc_str_value(); Some(true) }
            "options" => { l.options = bc.read_effect_vec_boxed_value().map(|b| *b); Some(true) }
            "original_value" => { l.original_value = bc.read_bool_value(); Some(true) }
            "phase" => { l.phase = bc.read_arc_str_value(); Some(true) }
            "phase_target" => { l.phase_target = bc.read_arc_str_value(); Some(true) }
            "position" => { l.position = bc.read_position_value(); Some(true) }
            "position_compare" => { l.position_compare = bc.read_arc_str_value(); Some(true) }
            "positions_characters" => { l.positions_characters = bc.read_positions_characters_value().map(|b| *b); Some(true) }
            "reference_card" => { l.reference_card = bc.read_arc_str_value(); Some(true) }
            "require_position_cards" => { l.require_position_cards = bc.read_bool_value(); Some(true) }
            "resource_type" => { l.resource_type = bc.read_arc_str_value(); Some(true) }
            "same_name" => { l.same_name = bc.read_bool_value(); Some(true) }
            "scope" => { l.scope = bc.read_arc_str_value(); Some(true) }
            "self_effect_only" => { l.self_effect_only = bc.read_bool_value(); Some(true) }
            "self_target" => { l.self_target = bc.read_bool_value(); Some(true) }
            "shuffle" => { l.shuffle = bc.read_bool_value(); Some(true) }
            "source" => { l.source = bc.read_arc_str_value(); Some(true) }
            "state" => { l.state = bc.read_arc_str_value(); Some(true) }
            "sub_checks" => { l.sub_checks = bc.read_location_sub_checks_value(); Some(true) }
            "target" => { l.target = bc.read_arc_str_value(); Some(true) }
            "temporal" => { l.temporal = bc.read_arc_str_value(); Some(true) }
            "temporal_scope" => { l.temporal_scope = bc.read_arc_str_value(); Some(true) }
            #[cfg(feature = "debug_conditions")] "text" => { l.text = bc.read_string_value(); Some(true) }
            #[cfg(not(feature = "debug_conditions"))] "text" => { bc.skip_value()?; Some(true) }
            "to_state" => { l.to_state = bc.read_arc_str_value(); Some(true) }
            #[cfg(feature = "debug_conditions")] "trigger_event" => { l.trigger_event = bc.read_trigger_event_value(); Some(true) }
            #[cfg(not(feature = "debug_conditions"))] "trigger_event" => { bc.skip_value()?; Some(true) }
            "turn_number" => { l.turn_number = bc.read_u8_value(); Some(true) }
            "unit" => { l.unit = bc.read_arc_str_value(); Some(true) }
            "values" => { l.values = bc.read_opt_u8_vec_value().map(|b| *b); Some(true) }
            "yell_trigger" => { l.yell_trigger = bc.read_bool_value(); Some(true) }
            "type" => { bc.skip_value()?; Some(true) }
            _ => { note_decode_fallback(Some(bc.idx.unwrap_or(usize::MAX)), "condition_field", key); bc.skip_value()?; Some(true) }
        }
    }

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_compound(l: &ConditionLocals) -> Condition {
    Condition::Compound {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
        conditions: l.conditions.clone(),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_location(l: &ConditionLocals) -> Condition {
    Condition::Location {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
        unit: l.unit.clone(),
        group_reference: l.group_reference.clone(),
        heart_type: l.heart_type.clone(),
        state: l.state.as_deref().map(CardState::from_str),
        sub_checks: l.sub_checks.clone(),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_comparison(l: &ConditionLocals) -> Condition {
    Condition::Comparison {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
        values: l.values.clone().map(Box::new),
        cost_total: l.cost_total,
        cost_total_operator: l.cost_total_operator,
        comparison_source: l.comparison_source.clone(),
        state: l.state.as_deref().map(CardState::from_str),
        ability_filter: l.ability_filter.clone(),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_movement(l: &ConditionLocals) -> Condition {
    Condition::Movement {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
        movement: l.movement.clone(),
        baton_touch_source: l.baton_touch_source.clone(),
        self_effect_only: l.self_effect_only,
        energy_placed: l.energy_placed,
        area_direction: l.area_direction.clone(),
        ability_filter: l.ability_filter.as_deref().map(AbilityFilter::from_str),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_group(l: &ConditionLocals) -> Condition {
    Condition::Group {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
        all_members: l.all_members,
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_appearance(l: &ConditionLocals) -> Condition {
    Condition::Appearance {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
        appearance: l.appearance,
        positions_characters: l.positions_characters.clone().map(Box::new),
        cost_reference_character: l.cost_reference_character.clone(),
        cost_reference_operator: l.cost_reference_operator,
        appearance_source: l.appearance_source.clone(),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_temporal(l: &ConditionLocals) -> Condition {
    Condition::Temporal {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
        turn_number: l.turn_number,
        temporal_scope: l.temporal_scope.clone(),
        condition: l.condition.clone(),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_state(l: &ConditionLocals) -> Condition {
    Condition::State {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
        state: l.state.as_deref().map(EffectState::from_str),
        energy_state: l.energy_state.clone(),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_resource(l: &ConditionLocals) -> Condition {
    Condition::Resource {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_abilityfilter(l: &ConditionLocals) -> Condition {
    Condition::AbilityFilter {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
        ability_filter: l.ability_filter.as_deref().map(AbilityFilter::from_str),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_scorethreshold(l: &ConditionLocals) -> Condition {
    Condition::ScoreThreshold {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_choice(l: &ConditionLocals) -> Condition {
    Condition::Choice {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
        options: l.options.clone().map(Box::new),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_complex(l: &ConditionLocals) -> Condition {
    Condition::Complex {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
        cause: l.cause.clone(),
        effect: l.effect.clone(),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_positioncond(l: &ConditionLocals) -> Condition {
    Condition::PositionCond {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_opponentchoice(l: &ConditionLocals) -> Condition {
    Condition::OpponentChoice {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_opponentlivesuccess(l: &ConditionLocals) -> Condition {
    Condition::OpponentLiveSuccess {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_noexcessheart(l: &ConditionLocals) -> Condition {
    Condition::NoExcessHeart {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_alwaystrue(l: &ConditionLocals) -> Condition {
    Condition::AlwaysTrue {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_anyof(l: &ConditionLocals) -> Condition {
    Condition::AnyOf {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
        any_of: l.any_of.clone(),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_allrevealedmatchheartcolor(l: &ConditionLocals) -> Condition {
    Condition::AllRevealedMatchHeartColor {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
fn build_unsupported(l: &ConditionLocals) -> Condition {
    Condition::Unsupported {
        common: Box::new(ConditionCommon {
            ability_filter_triggers: l.ability_filter_triggers.clone().map(Box::new),
            action_reference: l.action_reference.clone(),
            activation_position: l.activation_position.clone(),
            aggregate: l.aggregate.clone(),
            all: l.all,
            all_areas: l.all_areas,
            baton_touch_trigger: l.baton_touch_trigger,
            blade_greater_than_all: l.blade_greater_than_all,
            blade_limit: l.blade_limit,
            blade_limit_operator: l.blade_limit_operator,
            cache: l.cache,
            card_names: l.card_names.clone().map(Box::new),
            card_property: l.card_property.as_deref().map(CardProperty::from_str),
            card_type: l.card_type.as_deref().map(ConditionCardType::from_str),
            characters: l.characters.clone().map(Box::new),
            check_self: l.check_self,
            comparison_target: l.comparison_target.as_deref().map(ComparisonTarget::from_str),
            comparison_type: l.comparison_type.as_deref().map(ComparisonType::from_str),
            cost_limit: l.cost_limit,
            cost_limit_operator: l.cost_limit_operator,
            count: l.count,
            delta: l.delta,
            destination: l.destination.clone(),
            distinct: l.distinct.clone(),
            exclude_characters: l.exclude_characters.clone().map(Box::new),
            exclude_group_names: l.exclude_group_names.clone().map(Box::new),
            exclude_self: l.exclude_self,
            from_state: l.from_state.clone(),
            group_names: l.group_names.clone().map(Box::new),
            heart_colors: l.heart_colors.clone().map(Box::new),
            heart_source: l.heart_source.clone(),
            location: l.location.clone(),
            locations: l.locations.clone().map(Box::new),
            min_baton_touch_count: l.min_baton_touch_count,
            movement: l.movement.clone(),
            negation: l.negation,
            no_excess_heart: l.no_excess_heart,
            operator: l.operator.clone(),
            original_value: l.original_value,
            phase: l.phase.clone(),
            phase_target: l.phase_target.clone(),
            position: l.position.clone(),
            position_compare: l.position_compare.clone(),
            reference_card: l.reference_card.clone(),
            require_position_cards: l.require_position_cards,
            resource_type: l.resource_type.clone(),
            same_name: l.same_name,
            scope: l.scope.clone(),
            self_target: l.self_target,
            shuffle: l.shuffle,
            source: l.source.clone(),
            target: l.target.clone(),
            temporal: l.temporal.clone(),
            #[cfg(feature = "debug_conditions")] text: l.text.clone(),
            to_state: l.to_state.clone(),
            #[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone(),
            yell_trigger: l.yell_trigger,
        }),
    }
}

#[allow(
    // Recursive decode types need the Box inside the Vec. Rationale:
    // cards/generate_condition_decoder.py ITEM_ALLOW note.
    clippy::vec_box
)]
/// Direct decoder for TAG_OBJECT_VARIANT conditions.
fn decode_condition_direct(
    bc: &mut BcReader,
    variant: u8,
) -> Option<Condition> {
    let count = bc.read_len()?;
    let mut l = ConditionLocals::default();
    for _ in 0..count {
        let key = bc.key()?;
        decode_condition_field(bc, key, &mut l)?;
    }
    Some(match variant {
        0 => build_compound(&l),
        1 => build_location(&l),
        2 => build_comparison(&l),
        3 => build_movement(&l),
        4 => build_group(&l),
        5 => build_appearance(&l),
        6 => build_temporal(&l),
        7 => build_state(&l),
        8 => build_resource(&l),
        9 => build_abilityfilter(&l),
        10 => build_scorethreshold(&l),
        11 => build_choice(&l),
        12 => build_complex(&l),
        13 => build_positioncond(&l),
        14 => build_opponentchoice(&l),
        15 => build_opponentlivesuccess(&l),
        16 => build_noexcessheart(&l),
        17 => build_alwaystrue(&l),
        18 => build_anyof(&l),
        19 => build_allrevealedmatchheartcolor(&l),
        20 => build_unsupported(&l),
        _ => { note_decode_fallback(bc.idx, "condition_variant", &variant.to_string()); return None; }
    })
}
