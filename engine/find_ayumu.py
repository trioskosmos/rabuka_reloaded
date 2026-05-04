import json, sys
sys.stdout.reconfigure(encoding='utf-8')
with open('../cards/cards.json', 'r', encoding='utf-8') as f:
    cards = json.load(f)
c = cards.get('PL!N-bp3-001-R\uFF0B')
if c:
    print('Ability:', c.get('ability', 'MISSING')[:300])
else:
    # Try alternative keys
    for k in cards:
        if 'N-bp3-001-R' in k:
            print('Found:', k)
            c = cards[k]
            print('Ability:', c.get('ability', 'MISSING')[:300])
            break
