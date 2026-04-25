import json

# All action types from abilities.json
action_types = [
    'activation_cost', 'activation_restriction', 'appear', 'change_state', 'choice',
    'choose_required_hearts', 'conditional_alternative', 'conditional_on_optional',
    'conditional_on_result', 'discard_until_count', 'draw_card', 'draw_until_count',
    'gain_ability', 'gain_resource', 'invalidate_ability', 'look_and_select', 'look_at',
    'modify_cost', 'modify_limit', 'modify_required_hearts', 'modify_required_hearts_global',
    'modify_required_hearts_success', 'modify_score', 'modify_yell_count', 'move_cards',
    'pay_energy', 'place_energy_under_member', 'play_baton_touch', 'position_change',
    're_yell', 'restriction', 'reveal', 'reveal_per_group', 'select', 'sequential',
    'set_blade_count', 'set_blade_type', 'set_card_identity', 'set_card_identity_all_regions',
    'set_cost', 'set_cost_to_use', 'set_heart_type', 'set_required_hearts', 'set_score',
    'specify_heart_color', 'shuffle', 'all_blade_timing'
]

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

# Count occurrences of each action type
action_counts = {action: 0 for action in action_types}

def count_actions(obj):
    if isinstance(obj, dict):
        if 'action' in obj:
            action = obj['action']
            if action in action_counts:
                action_counts[action] += 1
        for value in obj.values():
            count_actions(value)
    elif isinstance(obj, list):
        for item in obj:
            count_actions(item)

count_actions(data)

print("Action type usage counts:")
for action in sorted(action_types):
    print(f"  {action}: {action_counts[action]}")

# Find actions with 0 occurrences (may not be used)
unused_actions = [action for action, count in action_counts.items() if count == 0]
print(f"\nUnused actions ({len(unused_actions)}):")
for action in unused_actions:
    print(f"  {action}")
