import json

with open('../cards/abilities.json', encoding='utf-8') as f:
    abilities = json.load(f)

ab_list = abilities.get('unique_abilities', [])
for ab in ab_list:
    if not isinstance(ab, dict):
        continue
    cards = ab.get('cards', [])
    for c in cards:
        if isinstance(c, str) and 'LL-PR-004-PR' in c:
            import pprint
            pprint.pprint(ab)
            print()
            break
