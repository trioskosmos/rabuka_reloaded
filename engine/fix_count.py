import json
with open('../cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
modified = False
for entry in data['unique_abilities']:
    for c in entry.get('cards', []):
        if 'LL-bp3-001' in c and 'ab#0' in c:
            cost = entry.get('cost', {})
            if cost.get('action') == 'move_cards' and 'count' not in cost:
                cost['count'] = 6
                modified = True
if modified:
    with open('../cards/abilities.json', 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    print('Added count=6')
else:
    print('No change needed')
