import json

# Remaining actions to check
remaining_actions = [
    'appear', 'position_change', 'select', 'gain_ability', 'reveal',
    'play_baton_touch', 'restriction', 'set_blade_type', 'set_blade_count',
    'set_card_identity', 'discard_until_count', 'place_energy_under_member',
    'modify_cost', 'modify_limit', 'modify_required_hearts', 'modify_required_hearts_global',
    'modify_yell_count', 'set_cost', 'set_required_hearts', 'set_score',
    'activation_cost', 'activation_restriction', 'choose_required_hearts',
    'invalidate_ability', 'pay_energy'
]

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

def find_action_examples(obj, action, path="", results=None, max_results=3):
    if results is None:
        results = []
    
    if len(results) >= max_results:
        return results
    
    if isinstance(obj, dict):
        if 'action' in obj and obj['action'] == action:
            results.append({
                'path': path,
                'data': obj
            })
        for key, value in obj.items():
            find_action_examples(value, action, f"{path}.{key}" if path else key, results, max_results)
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            find_action_examples(item, action, f"{path}[{i}]", results, max_results)
    
    return results

for action in remaining_actions:
    examples = find_action_examples(data, action)
    if examples:
        print(f"\n{'='*60}")
        print(f"Action: {action}")
        print(f"{'='*60}")
        for i, ex in enumerate(examples):
            print(f"\nExample {i+1}: {ex['path']}")
            for key, value in ex['data'].items():
                if key != 'text':
                    print(f"  {key}: {value}")
    else:
        print(f"\n{action}: No examples found")
