import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_ability

with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

for idx in [353, 621]:
    a = data['unique_abilities'][idx]
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    r = parse_ability(t)
    eff = r.get('effect', {})
    print('=== #%d ===' % idx)
    print('TEXT:', t[:150])
    print('EFFECT action:', eff.get('action'))
    if eff.get('action') == 'gain_ability':
        print('  ability_gain:', repr(eff.get('ability_gain','')[:80]))
        print('  gained_effect:', eff.get('gained_effect') is not None)
    elif eff.get('action') == 'sequential':
        for i, ac in enumerate(eff.get('actions', [])):
            print('  [%d] %s ability_gain=%s' % (i, ac.get('action'), repr(ac.get('ability_gain','')[:80])))
    else:
        print('  keys:', sorted(eff.keys()))
    print()
