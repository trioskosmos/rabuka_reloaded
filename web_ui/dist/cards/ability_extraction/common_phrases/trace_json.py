import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_ability, parse_effect, normalize, split_cost_effect

# Load the exact JSON text
with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

# Find the specific ability
for a in data['unique_abilities']:
    t = a.get('full_text', '')
    if '高海千歌' in t and '起動' in t and 'センター' in t:
        triggerless = a['triggerless_text']
        full = a['full_text']
        print('=== Original JSON triggerless_text ===')
        print('  text:', triggerless[:120])
        print()
        
        # Step 1: normalize
        norm = normalize(triggerless.strip())
        print('  After normalize: same as before?', norm == triggerless.strip())
        
        # Step 2: split
        cost, effect = split_cost_effect(norm)
        print('  Cost:', cost[:60])
        print('  Effect:', effect[:100])
        print()
        
        # Step 3: parse_effect on the effect text
        print('  parse_effect on effect_text:')
        r = parse_effect(effect)
        print('    action:', r.get('action'))
        if r.get('action') == 'sequential':
            for i, ac in enumerate(r.get('actions', [])):
                print('    [%d] %s text=%s' % (i, ac.get('action'), ac.get('text','')[:50]))
        else:
            print('    parenthetical:', r.get('parenthetical') is not None)
            print('    gained_effect:', r.get('gained_effect') is not None)
        break
