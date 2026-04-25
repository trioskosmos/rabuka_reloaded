import json

missing_fields = [
    'activation_position', 'all_areas', 'appearance', 'baton_touch_trigger',
    'choice_type', 'comparison_operator', 'comparison_target', 'conditional',
    'conditions', 'group_names', 'heart_type', 'includes', 'includes_pattern',
    'location', 'lose_blade_hearts', 'movement', 'movement_condition', 'negation',
    'no_excess_heart', 'operator', 'phase', 'resource_type', 'state',
    'temporal', 'temporal_scope', 'trigger_type', 'type', 'values'
]

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

for field in missing_fields:
    examples = find_field_examples(data, field)
    if examples:
        print(f"\n{'='*60}")
        print(f"Field: {field}")
        print(f"Found {len(examples)} examples")
        print(f"{'='*60}")
        for i, ex in enumerate(examples[:3]):  # Show first 3 examples
            print(f"\nExample {i+1}:")
            print(f"  Path: {ex['path']}")
            print(f"  Value: {ex['value']}")
            if ex['context']:
                print(f"  Context: {ex['context']}")
