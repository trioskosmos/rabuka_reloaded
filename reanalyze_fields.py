import json

# Fields that appeared missing but are actually in Condition struct
fields_in_condition = {
    'activation_position', 'all_areas', 'appearance', 'baton_touch_trigger',
    'comparison_operator', 'comparison_target', 'comparison_type', 'conditions',
    'group_names', 'includes', 'includes_pattern', 'location', 'movement',
    'movement_condition', 'negation', 'no_excess_heart', 'operator', 'phase',
    'resource_type', 'state', 'temporal', 'temporal_scope', 'trigger_type'
}

# Fields still truly missing
truly_missing = {
    'choice_type', 'conditional', 'heart_type', 'lose_blade_hearts', 'type', 'values'
}

# Note: 'type' field in JSON maps to:
# - cost_type in AbilityCost
# - action in AbilityEffect  
# - condition_type in Condition

# Check for choice_type, conditional, heart_type, lose_blade_hearts, values
with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

def find_field_examples(obj, field, path="", results=None):
    if results is None:
        results = []
    
    if isinstance(obj, dict):
        if field in obj:
            results.append({
                'path': path,
                'value': obj[field],
                'context': {k: v for k, v in obj.items() if k in ['text', 'action', 'type']}
            })
        for key, value in obj.items():
            find_field_examples(value, field, f"{path}.{key}" if path else key, results)
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            find_field_examples(item, field, f"{path}[{i}]", results)
    
    return results

print("Checking truly missing fields:")
for field in truly_missing:
    examples = find_field_examples(data, field)
    if examples:
        print(f"\n{field}: Found {len(examples)} examples")
        for i, ex in enumerate(examples[:2]):
            print(f"  Example {i+1}: {ex['path']} = {ex['value']}")
    else:
        print(f"\n{field}: No examples found (may not be used)")
