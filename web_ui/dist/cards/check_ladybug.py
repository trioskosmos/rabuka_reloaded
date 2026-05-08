import json
with open('abilities.json', encoding='utf-8') as f:
    data = json.load(f)
for ab in data['unique_abilities']:
    for c in ab.get('cards', []):
        if 'PL!HS-bp2-024-L' in c:
            eff = ab.get('effect', {})
            print('heart_color:', eff.get('heart_color'))
            cond = eff.get('condition', {})
            if 'conditions' in cond:
                for i, sub in enumerate(cond['conditions']):
                    print(f'Sub-condition {i}: type={sub.get("type")} appearance={sub.get("appearance")} chars={sub.get("characters")}')
            break
