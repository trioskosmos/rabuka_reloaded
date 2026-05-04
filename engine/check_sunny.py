import json
with open('../cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
for entry in data['unique_abilities']:
    for c in entry.get('cards', []):
        if 'PL!-bp5-021-L' in c:
            eff = entry.get('effect', {})
            for i, a in enumerate(eff.get('actions', [])):
                cnd = a.get('condition', {})
                ct = cnd.get('card_type', 'MISSING')
                u = cnd.get('unit', 'MISSING')
                print(f'Action {i}: card_type={ct} unit={u}')
            break
