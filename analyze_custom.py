import json
import sys

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

def get_first_card(entry):
    cards = entry.get('cards', [])
    if cards:
        return cards[0]
    return ''

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

out = []
out.append(f'Total unique abilities with ANY custom field: {len(results)}')
out.append(f'')

custom_action_effect = 0
for entry, items in results:
    for path, txt, ctype in items:
        if ctype == 'action' and 'effect' in path:
            custom_action_effect += 1

out.append(f'Custom action entries in effect: {custom_action_effect}')
out.append(f'')
out.append(f'=== DETAILED LIST ===')
out.append(f'')

for entry, items in results:
    ft = entry['full_text']
    cc = entry['card_count']
    first = get_first_card(entry)
    out.append(f'=' * 80)
    out.append(f'FULL_TEXT: {ft}')
    out.append(f'CARD_COUNT: {cc}')
    out.append(f'EXAMPLE_CARD: {first}')
    out.append(f'')
    for path, txt, ctype in items:
        out.append(f'  [{ctype}] PATH: {path}')
        out.append(f'  SUBTEXT: {txt}')
        out.append(f'')
    out.append(f'')

sys.stdout.reconfigure(encoding='utf-8')
print('\n'.join(out))
