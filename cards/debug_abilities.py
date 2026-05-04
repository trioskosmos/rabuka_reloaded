"""Debug: check the ACTUAL abilities for card PL!-bp5-021-L"""
import json
d = json.load(open('cards/abilities.json', encoding='utf-8'))
target = 'PL!-bp5-021-L'
for i, entry in enumerate(d['unique_abilities']):
    for c in entry.get('cards', []):
        if target in c:
            print(f"Entry {i}:")
            print(f"  effect action: {entry['effect'].get('action')}")
            print(f"  effect text: {entry['effect'].get('text','')[:80]}")
            actions = entry['effect'].get('actions', [])
            for j, a in enumerate(actions):
                cond = a.get('condition', {})
                ct = cond.get('type', 'no-cond')
                print(f"  action[{j}]: action={a.get('action')} condition={ct}")
                if ct == 'compound':
                    for k, sub in enumerate(cond.get('conditions', [])):
                        print(f"    sub[{k}]: type={sub.get('type')} count={sub.get('count')} ct={sub.get('card_type')} unit={sub.get('unit')} distinct={sub.get('distinct')}")
                if a.get('action') == 'sequential':
                    for k, sa in enumerate(a.get('actions', [])):
                        print(f"    inner[{k}]: action={sa.get('action')}")
            print()
