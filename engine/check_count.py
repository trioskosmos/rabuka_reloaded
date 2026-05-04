import json, os.path
files = [
    '../cards/abilities.json',
    '../engine/data/abilities.json',
    '../web_ui/dist/cards/abilities.json'
]
for f in files:
    if os.path.exists(f):
        with open(f, 'r', encoding='utf-8') as fh:
            data = json.load(fh)
        for entry in data['unique_abilities']:
            for c in entry.get('cards', []):
                if 'LL-bp3-001' in c and 'ab#0' in c:
                    cost = entry.get('cost', {})
                    has = 'count' in cost
                    val = cost.get('count', 'MISSING')
                    print(f'{f}: has_count={has} count={val}')
                    break
