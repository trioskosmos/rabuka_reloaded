import json
with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)
ua = data.get('unique_abilities', [])
for entry in ua:
    cards = entry.get('cards', [])
    for c in cards:
        if 'PR-004' in c and 'LL' in c:
            effect = entry.get('effect')
            print('action:', effect.get('action'))
            opts = effect.get('options', [])
            for i, opt in enumerate(opts):
                act = opt.get('action')
                optional = opt.get('optional')
                count = opt.get('count')
                print(f'opt[{i}]: action={act}, optional={optional}, count={count}')
                if act == 'draw_card':
                    print(f'  source={opt.get("source")}, dest={opt.get("destination")}')
            break
