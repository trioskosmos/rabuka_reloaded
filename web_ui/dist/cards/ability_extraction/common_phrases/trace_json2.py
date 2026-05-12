import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_ability, parse_effect, normalize, split_cost_effect

with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

# Print first few ability triggerless_texts to find the right one
for i, a in enumerate(data['unique_abilities']):
    tt = a.get('triggerless_text', '')
    if 'センター' in tt and 'ウェイトに' in tt:
        print('Found at index', i)
        print('  triggerless:', tt[:100])
        print()
        
        # run through kD
        r = parse_ability(tt)
        eff = r.get('effect', {})
        print('  parse_ability action:', eff.get('action'))
        if eff.get('action') == 'sequential':
            for j, ac in enumerate(eff.get('actions', [])):
                print('  [%d] %s text=%s' % (j, ac.get('action'), ac.get('text','')[:40]))
        else:
            print('  parenthetical:', eff.get('parenthetical') is not None)
        print()
        
        # Now run parse_effect directly on the effect part
        cost, effect = split_cost_effect(tt.strip())
        print('  parse_effect on effect text:')
        r2 = parse_effect(effect)
        print('    action:', r2.get('action'))
        if r2.get('action') == 'sequential':
            for j, ac in enumerate(r2.get('actions', [])):
                print('    [%d] %s text=%s' % (j, ac.get('action'), ac.get('text','')[:40]))
        else:
            print('    parenthetical:', r2.get('parenthetical') is not None)
