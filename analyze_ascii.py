# -*- coding: utf-8 -*-
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

def escape(s):
    """Escape non-ASCII characters for safe terminal output."""
    if s is None:
        return 'None'
    return s.encode('ascii', 'backslashreplace').decode('ascii')

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

print("=" * 80)
print("TOTAL unique abilities with custom fields: {}".format(len(results)))
print()

custom_action_effect = sum(1 for entry, items in results 
    for path, txt, ctype in items if ctype == 'action' and 'effect' in path)
print("Custom action entries in effect: {}".format(custom_action_effect))
print()

# Also count cost type custom
cost_custom = sum(1 for entry, items in results 
    for path, txt, ctype in items if ctype == 'type' and 'cost' in path)
print("Custom type in cost: {}".format(cost_custom))
print()

print("=" * 80)
print("ALL 9 ENTRIES WITH CUSTOM")
print("=" * 80)

for i, (entry, items) in enumerate(results):
    print()
    print("--- ENTRY {} ---".format(i+1))
    print("FULL_TEXT: {}".format(escape(entry['full_text'])))
    print("CARD_COUNT: {}".format(entry['card_count']))
    for path, txt, ctype in items:
        print("  [{}] PATH: {}".format(ctype, path))
        print("  SUBTEXT: {}".format(escape(txt)))
    print()

# Stats
print("=" * 80)
print("PATTERN SUMMARY")
print("=" * 80)

# Group by pattern
patterns = {}
for entry, items in results:
    ft = entry['full_text']
    key = None
    for path, txt, ctype in items:
        key = escape(ft[:60])
    if key:
        if key not in patterns:
            patterns[key] = []
        patterns[key].append((entry, items))

print("\n9 entries found. See above for details.")
