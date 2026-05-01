import json
a = json.load(open('../../../cards/abilities.json', encoding='utf-8'))

for ab in a['unique_abilities']:
    ft = ab.get('full_text', '')
    if 'カードを1枚引き、手札を1枚控え室に置く' in ft:
        cards = ab.get('cards', [])
        cost = ab.get('cost')
        eff = ab.get('effect', {})
        print('full_text:', ft)
        print('cost:', json.dumps(cost, ensure_ascii=False))
        print('effect action:', eff.get('action') if eff else 'none')
        print('effect has_actions:', len(eff.get('actions', [])) if eff else 0)
        print('sample card:', cards[0] if cards else 'N/A')
        print()
