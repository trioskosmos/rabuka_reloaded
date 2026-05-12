import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_ability

with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

# Investigate each move_cards_incomplete
for idx in [290, 505, 605, 628]:
    a = data['unique_abilities'][idx]
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    r = parse_ability(t)
    eff = r.get('effect', {})
    
    print('=== #%d move_cards_incomplete ===' % idx)
    print('  TEXT:', t[:120])
    print('  EFFECT:')
    def walk(d, depth=0):
        if isinstance(d, dict):
            if d.get('action') == 'move_cards':
                src = d.get('source', 'MISSING')
                dst = d.get('destination', 'MISSING')
                print('  ' * depth + 'move_cards src=%s dst=%s text=%s' % (src, dst, d.get('text','')[:60]))
            for v in d.values():
                if isinstance(v, (dict, list)):
                    walk(v, depth+1)
        elif isinstance(d, list):
            for item in d:
                walk(item, depth+1)
    walk(eff)
    print()
