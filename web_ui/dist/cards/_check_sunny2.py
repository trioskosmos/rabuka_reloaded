import json
data = json.load(open('abilities.json', 'r', encoding='utf-8'))
for e in data['unique_abilities']:
    for card in e['cards']:
        if 'PL!-bp5-021-L' in card:
            eff = e.get('effect', {})
            for i, act in enumerate(eff.get('actions', [])):
                if i == 2:
                    c = act.get('condition', {})
                    print('Condition type:', c.get('type'))
                    print('Operator:', c.get('operator'))
                    for j, subc in enumerate(c.get('conditions', [])):
                        print(f'\nSub-condition {j}:')
                        for key, val in subc.items():
                            print(f'  {key}: {val}')
            break
