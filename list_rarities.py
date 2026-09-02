import json
data = json.load(open('cards/cards.json', encoding='utf-8'))
rarities = set()
for k in data:
    r = data[k].get('rare')
    if r:
        rarities.add(r)
for r in sorted(rarities):
    print(repr(r))