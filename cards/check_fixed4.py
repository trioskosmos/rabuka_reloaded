import json
d = json.load(open('cards/abilities.json', encoding='utf-8'))
e = d['unique_abilities'][523]['effect']['actions']
for i, a in enumerate(e):
    print('Branch {}: action={}'.format(i, a.get('action')))
    cond = a.get('condition')
    if cond:
        print('  type={} count={} op={} unit={} ct={}'.format(
            cond.get('type'), cond.get('count'), cond.get('operator'),
            cond.get('unit'), cond.get('card_type')))
    if a.get('actions'):
        print('  sub-actions:')
        for j, sa in enumerate(a['actions']):
            sc = sa.get('condition')
            if sc:
                print('    [{}] type={} count={} op={} unit={} ct={} target={}'.format(
                    j, sc.get('type'), sc.get('count'), sc.get('operator'),
                    sc.get('unit'), sc.get('card_type'), sc.get('target')))
            else:
                print('    [{}] no condition'.format(j))
