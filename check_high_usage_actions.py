import json

# High usage actions to examine
high_usage_actions = [
    'move_cards', 'gain_resource', 'draw_card', 'sequential', 'change_state',
    'look_and_select', 'modify_score'
]

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

def find_action_examples(obj, action, path="", results=None, max_results=5):
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

for action in high_usage_actions:
    examples = find_action_examples(data, action)
    print(f"\n{'='*60}")
    print(f"Action: {action}")
    print(f"{'='*60}")
    for i, ex in enumerate(examples[:3]):
        print(f"\nExample {i+1}: {ex['path']}")
        # Show all fields in this action
        for key, value in ex['data'].items():
            if key != 'text':
                print(f"  {key}: {value}")
