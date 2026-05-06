import json
with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)
ua = data.get('unique_abilities', [])
for entry in ua:
    cards = entry.get('cards', [])
    for c in cards:
        if 'PR-004' in c and 'LL' in c:
            print('trigger:', entry.get('triggers'))
            effect = entry.get('effect')
            if effect:
                print('action:', effect.get('action'))
                opts = effect.get('options')
                if opts:
                    for i, opt in enumerate(opts):
                        act = opt.get('action')
                        print(f'  opt[{i}]: action={act}, optional={opt.get("optional")}')
            break
