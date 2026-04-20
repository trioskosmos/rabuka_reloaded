"""Show unparsed patterns."""
import json

with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

print("=== UNPARSED CUSTOM ACTIONS ===\n")
custom_actions = [a for a in abilities if a.get('effect', {}).get('action') == 'custom']
for i, a in enumerate(custom_actions[:20], 1):
    text = a['triggerless_text']
    print(f"{i}. {text}")

print(f"\nTotal custom actions: {len(custom_actions)}")

print("\n=== UNPARSED CUSTOM COSTS ===\n")
custom_costs = [a for a in abilities if a.get('cost') and a.get('cost', {}).get('type') == 'custom']
for i, a in enumerate(custom_costs[:20], 1):
    text = a['triggerless_text']
    cost_part = text.split('：')[0] if '：' in text else text
    print(f"{i}. {cost_part}")

print(f"\nTotal custom costs: {len(custom_costs)}")
