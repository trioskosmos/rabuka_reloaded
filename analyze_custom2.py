import json
import re

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

def find_custom(obj, path=''):
    items = []
    if isinstance(obj, dict):
        if 'action' in obj and obj['action'] == 'custom':
            items.append((path, obj.get('text', ''), 'action'))
        if 'type' in obj and obj['type'] == 'custom':
            items.append((path, obj.get('text', ''), 'type'))
        for k, v in obj.items():
            if isinstance(v, (dict, list)):
                items.extend(find_custom(v, f'{path}.{k}'))
    elif isinstance(obj, list):
        for i, v in enumerate(obj):
            items.extend(find_custom(v, f'{path}[{i}]'))
    return items

# Need to look at raw JSON to find action values not defined in parser.py
# First: what actions appear in the JSON?
all_actions = set()

def collect_actions(obj):
    if isinstance(obj, dict):
        if 'action' in obj:
            all_actions.add(obj['action'])
        for v in obj.values():
            collect_actions(v)
    elif isinstance(obj, list):
        for v in obj:
            collect_actions(v)

for entry in data['unique_abilities']:
    collect_actions(entry.get('effect', {}))
    collect_actions(entry.get('cost', {}))

# What action values are defined in parser.py?
# Let me read the parser file
with open('cards/ability_extraction/parser.py', 'r', encoding='utf-8') as f:
    parser_text = f.read()

# Extract action strings from the dispatch table (R calls)
defined_actions = set(re.findall(r"'(draw_card|shuffle|position_change|pay_energy|place_energy_under_member|draw_until_count|discard_until_count|move_cards|change_state|activation_restriction|restriction|gain_resource|re_yell|reveal|select|look_at|appear|activate_ability|invalidate_ability|invalidate_ability_optional|modify_required_hearts|modify_score|choice|set_blade_type|set_required_hearts|modify_cost|repeat_procedure|do_nothing|play_baton_touch|set_score|modify_yell_count|gain_ability|set_card_identity|choose_required_hearts|all_blade_timing)'", parser_text))

# Also check for action types defined in patterns
action_pattern_actions = set(re.findall(r"'\w+'", parser_text))
# Manually add known action types from the patterns
all_defined = {'draw_card', 'shuffle', 'position_change', 'pay_energy', 'place_energy_under_member', 'draw_until_count', 'discard_until_count', 'move_cards', 'change_state', 'activation_restriction', 'restriction', 'gain_resource', 're_yell', 'look_at', 'reveal', 'select', 'appear', 'activate_ability', 'invalidate_ability', 'invalidate_ability_optional', 'modify_required_hearts', 'modify_score', 'choice', 'set_blade_type', 'set_required_hearts', 'modify_cost', 'repeat_procedure', 'do_nothing', 'play_baton_touch', 'set_score', 'modify_yell_count', 'gain_ability', 'set_card_identity', 'choose_required_hearts', 'all_blade_timing', 'sequential', 'look_and_select', 'conditional_alternative', 'custom'}

# Also check cost types
all_cost_types = set()
def collect_types(obj):
    if isinstance(obj, dict):
        if 'type' in obj:
            all_cost_types.add(obj['type'])
        for v in obj.values():
            collect_types(v)
    elif isinstance(obj, list):
        for v in obj:
            collect_types(v)

for entry in data['unique_abilities']:
    collect_types(entry)
    collect_types(entry.get('cost', {}))
    collect_types(entry.get('effect', {}))

unknown_actions = all_actions - all_defined
unknown_cost_types = all_cost_types - {'move_cards', 'pay_energy', 'change_state', 'sequential_cost', 'reveal', 'choice_condition', 'reveal_condition', 'energy_condition', 'place_energy_under_member', 'state_change', 'custom', None, 'comparison_condition', 'card_count_condition', 'location_condition', 'compound', 'temporal_condition', 'appearance_condition', 'movement_condition', 'position_condition', 'energy_state_condition', 'state_condition', 'reveal_condition', 'opponent_choice_condition', 'both_condition', 'or_condition', 'position_change_condition', 'ability_negation_condition', 'state_change_condition', 'heart_possession_condition', 'group_condition', 'complex_condition', 'score_threshold_condition', 'location_condition', 'position_condition', 'energy_state_condition'}

# Now list all custom entries with their full_text as raw JSON line reads
# Let me also get the raw positions from the JSON

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    raw = f.read()

print("=" * 80)
print("FULL ANALYSIS REPORT")
print("=" * 80)

print(f"\nTotal unique abilities: {len(data['unique_abilities'])}")

results = []
for idx, entry in enumerate(data['unique_abilities']):
    eff = entry.get('effect')
    cost = entry.get('cost')
    custom_items = []
    if eff:
        custom_items.extend(find_custom(eff, 'effect'))
    if cost:
        custom_items.extend(find_custom(cost, 'cost'))
    if custom_items:
        results.append((entry, custom_items))

print(f"\nAbilities with custom fields: {len(results)}")

custom_action_count = len([1 for entry, items in results 
    for path, txt, ctype in items if ctype == 'action' and 'effect' in path])
print(f"Custom actions in effect: {custom_action_count}")

print(f"\n--- Undefined action values ---")
print(f"All action values found: {sorted(all_actions)}")
# Let me just print unknown ones
known_actions = {'draw_card', 'shuffle', 'position_change', 'pay_energy', 'place_energy_under_member',
    'draw_until_count', 'discard_until_count', 'move_cards', 'change_state', 'activation_restriction',
    'restriction', 'gain_resource', 're_yell', 'look_at', 'reveal', 'select', 'appear', 'activate_ability',
    'invalidate_ability', 'invalidate_ability_optional', 'modify_required_hearts', 'modify_score', 'choice',
    'set_blade_type', 'set_required_hearts', 'modify_cost', 'repeat_procedure', 'do_nothing',
    'play_baton_touch', 'set_score', 'modify_yell_count', 'gain_ability', 'set_card_identity',
    'choose_required_hearts', 'all_blade_timing', 'sequential', 'look_and_select',
    'conditional_alternative', 'custom'}
unknown = all_actions - known_actions
print(f"Unknown: {unknown}")

print(f"\n--- Cost types found ---")
print(f"Cost types: {sorted(all_cost_types - {None})}")
unknown_ct = all_cost_types - {'move_cards', 'pay_energy', 'change_state', 'sequential_cost', 'reveal',
    'choice_condition', 'reveal_condition', 'energy_condition', 'place_energy_under_member', 'state_change',
    'custom', None}
print(f"Unknown cost types: {unknown_ct}")

print(f"\n{'=' * 80}")
print(f"DETAILED CUSTOM ABILITY LIST")
print(f"{'=' * 80}")

for i, (entry, items) in enumerate(results):
    print(f"\n{'=' * 80}")
    print(f"ENTRY {i+1}")
    print(f"Full text: {entry['full_text']}")
    print(f"Card count: {entry['card_count']}")
    print(f"Cards: {', '.join(entry['cards'][:3])}{'...' if len(entry['cards']) > 3 else ''}")
    for path, txt, ctype in items:
        print(f"  [{ctype}] {path}")
        print(f"  Sub-text: {txt}")
