import json
import re

# Fields extracted from abilities.json
json_fields = {
    'ability_gain', 'action', 'actions', 'activation_condition', 'activation_condition_parsed',
    'activation_position', 'aggregate', 'all_areas', 'alternative_condition', 'alternative_effect',
    'any_number', 'appearance', 'baton_touch_trigger', 'blade_type', 'card_type', 'choice_type',
    'comparison_operator', 'comparison_target', 'comparison_type', 'condition', 'conditional',
    'conditions', 'cost_limit', 'costs', 'count', 'destination', 'destination_choice', 'distinct',
    'duration', 'effect_constraint', 'energy', 'energy_count', 'exclude_self', 'group',
    'group_names', 'heart_type', 'identities', 'includes', 'includes_pattern', 'location',
    'look_action', 'lose_blade_hearts', 'max', 'movement', 'movement_condition', 'negation',
    'no_excess_heart', 'operation', 'operator', 'optional', 'options', 'parenthetical',
    'per_unit', 'per_unit_count', 'per_unit_type', 'phase', 'placement_order', 'position',
    'primary_effect', 'quoted_text', 'resource', 'resource_type', 'restricted_destination',
    'restriction_type', 'select_action', 'self_cost', 'source', 'state', 'state_change',
    'target', 'target_count', 'target_member', 'temporal', 'temporal_scope', 'text',
    'trigger_type', 'type', 'unit', 'value', 'values'
}

# Fields from Rust AbilityEffect struct (from card.rs lines 482-591)
rust_effect_fields = {
    'text', 'action', 'source', 'destination', 'count', 'target_count', 'card_type', 'target',
    'duration', 'parenthetical', 'look_action', 'select_action', 'actions', 'resource',
    'position', 'state_change', 'optional', 'max', 'effect_constraint', 'shuffle_target',
    'icon_count', 'resource_icon_count', 'ability_gain', 'quoted_text', 'per_unit',
    'destination_choice', 'condition', 'primary_effect', 'alternative_condition',
    'alternative_effect', 'result_condition', 'followup_action', 'optional_action',
    'conditional_action', 'operation', 'value', 'aggregate', 'comparison_type',
    'heart_color', 'blade_type', 'energy_count', 'target_member', 'choice_options',
    'group', 'per_unit_count', 'per_unit_type', 'per_unit_reference', 'group_matching',
    'repeat_limit', 'repeat_optional', 'is_further', 'restriction_type',
    'restricted_destination', 'cost_result_reference', 'dynamic_count', 'placement_order',
    'cost_limit', 'any_number', 'unit', 'distinct', 'target_player', 'target_location',
    'target_scope', 'target_card_type', 'activation_condition',
    'activation_condition_parsed', 'gained_ability', 'ability_text', 'swap_action',
    'has_member_swapping', 'group_options', 'card_count', 'use_limit', 'triggers',
    'self_cost', 'exclude_self', 'effect_type', 'choice', 'timing', 'treat_as',
    'identities', 'action_by', 'opponent_action'
}

# Fields from Rust AbilityCost struct (from card.rs lines 454-479)
rust_cost_fields = {
    'text', 'cost_type', 'source', 'destination', 'count', 'card_type', 'target', 'action',
    'optional', 'energy', 'state_change', 'position', 'options', 'self_cost', 'exclude_self',
    'costs', 'cost_limit'
}

# Find fields in JSON but not in Rust
missing_in_rust = json_fields - rust_effect_fields - rust_cost_fields

# Find fields in Rust but not in JSON (these might be unused)
extra_in_rust = (rust_effect_fields | rust_cost_fields) - json_fields

print("Fields in abilities.json but NOT in Rust structs:")
for field in sorted(missing_in_rust):
    print(f"  - {field}")

print("\nFields in Rust structs but NOT in abilities.json:")
for field in sorted(extra_in_rust):
    print(f"  - {field}")

print(f"\nTotal fields in JSON: {len(json_fields)}")
print(f"Total fields in Rust: {len(rust_effect_fields | rust_cost_fields)}")
print(f"Missing in Rust: {len(missing_in_rust)}")
print(f"Extra in Rust: {len(extra_in_rust)}")
