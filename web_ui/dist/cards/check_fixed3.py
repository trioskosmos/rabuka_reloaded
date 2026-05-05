import json
d = json.load(open('cards/abilities.json', encoding='utf-8'))
e = d['unique_abilities'][523]['effect']['actions']
for i, a in enumerate(e):
    print('Branch {}: action={}'.format(i, a.get('action')))
    cond = a.get('condition')
    if cond:
        print('  condition type={}'.format(cond.get('type')))
        if cond.get('conditions'):
            for j, c in enumerate(cond['conditions']):
                print('  sub-cond[{}]: type={} count={} op={} loc={} ct={}'.format(
                    j, c.get('type'), c.get('count'), c.get('operator'),
                    c.get('location'), c.get('card_type')))
