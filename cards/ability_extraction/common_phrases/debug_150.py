import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_ability

with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

# Check ability #150
a = data['unique_abilities'][150]
t = a['triggerless_text']
print('TEXT:', t)
print()
r = parse_ability(t)
eff = r.get('effect', {})
print('EFFECT action:', eff.get('action'))

def walk(d, depth=0):
    if isinstance(d, dict):
        if d.get('action') == 'modify_score':
            print('  MS: op=%s val=%s text=%s' % (d.get('operation'), d.get('value'), d.get('text','')[:50]))
        for v in d.values():
            if isinstance(v, (dict, list)):
                walk(v, depth+1)
    elif isinstance(d, list):
        for item in d:
            walk(item, depth+1)

walk(eff)
