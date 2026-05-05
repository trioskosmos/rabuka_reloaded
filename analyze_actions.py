# -*- coding: utf-8 -*-
import json
import re

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

# Collect all action values
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

# Collect all type values (cost types)
all_types = set()
def collect_types(obj):
    if isinstance(obj, dict):
        if 'type' in obj:
            all_types.add(obj['type'])
        for v in obj.values():
            collect_types(v)
    elif isinstance(obj, list):
        for v in obj:
            collect_types(v)

for entry in data['unique_abilities']:
    collect_types(entry)
    collect_types(entry.get('cost', {}))
    collect_types(entry.get('effect', {}))

print("Action values found in JSON:")
for a in sorted(all_actions):
    print("  " + a)

print()
print("Type values found in JSON:")
for t in sorted(all_types - {None}):
    print("  " + t)

# Now read parser to see what's defined
with open('cards/ability_extraction/parser.py', 'r', encoding='utf-8') as f:
    parser_text = f.read()

# Action values defined in parser dispatch table
dispatch_actions = set()
# Find all R('...', 'action_name', ...) or R(lambda..., 'action_name', ...)
for m in re.finditer(r"R\([^,]+,\s*'([^']+)'", parser_text):
    dispatch_actions.add(m.group(1))

# Also find action assignments
for m in re.finditer(r"action\['action'\]\s*=\s*'([^']+)'", parser_text):
    dispatch_actions.add(m.group(1))

print("\nAction values defined in parser.py dispatch table:")
for a in sorted(dispatch_actions):
    print("  " + a)

print("\nActions in JSON NOT in parser dispatch:")
for a in sorted(all_actions - dispatch_actions - {'custom'}):
    print("  " + a)

# Cost types defined
cost_types_defined = ['move_cards', 'pay_energy', 'change_state', 'sequential_cost', 'reveal', 
    'choice_condition', 'reveal_condition', 'energy_condition', 'place_energy_under_member', 'state_change', 'custom']
print("\nCost types in JSON NOT in parser cost parsing:")
for t in sorted(all_types - set(cost_types_defined) - {None}):
    print("  " + t)

print("\n---")
print("Actions in JSON not in known set:")
known = dispatch_actions | {'custom', 'sequential', 'look_and_select', 'conditional_alternative'}
for a in sorted(all_actions - known):
    print("  UNKNOWN: " + a)

# Count custom entries by card count
print("\nCustom entries with their card counts:")
for entry in data['unique_abilities']:
    eff = entry.get('effect', {})
    cost = entry.get('cost', {})
    found = []
    def check(obj):
        if isinstance(obj, dict):
            if obj.get('action') == 'custom':
                found.append('action:' + obj.get('text', '')[:40])
            if obj.get('type') == 'custom':
                found.append('type:' + obj.get('text', '')[:40])
            for v in obj.values():
                check(v)
        elif isinstance(obj, list):
            for v in obj:
                check(v)
    check(eff)
    check(cost)
    if found:
        print("  Cards: {}".format(entry['card_count']))
        print("    Full: {}".format(entry['full_text'][:120]))
        for f in found:
            print("    Custom: {}".format(f))
