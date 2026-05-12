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
            print('Trigger:', ab.get('triggers'))
            eff = ab.get('effect', {})
            if isinstance(eff, dict):
                print('Action:', eff.get('action'))
                print('Target:', eff.get('target'))
                print('Options count:', len(eff.get('choice_options', [])))
                for i, opt in enumerate(eff.get('choice_options', [])):
                    if isinstance(opt, dict):
                        a = opt.get('action')
                        t = opt.get('target')
                        tx = opt.get('text', '')[:50]
                        print(f'Option {i}: action={a}, target={t}, text={tx}')
            print()
