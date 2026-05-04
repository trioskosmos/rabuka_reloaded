import json
with open('../cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
for entry in data['unique_abilities']:
    for c in entry.get('cards', []):
        if 'PL!-bp5-021-L' in c:
            eff = entry.get('effect', {})
            for i, a in enumerate(eff.get('actions', [])):
                print(f'\n=== Action {i} ===')
                print(json.dumps(a, ensure_ascii=False, indent=2)[:800])
            break
