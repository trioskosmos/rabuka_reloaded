import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_ability

with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

a = data['unique_abilities'][54]
tt = a['triggerless_text']
r = parse_ability(tt)
eff = r.get('effect', {})
print('action:', eff.get('action'))
print('ability_gain:', eff.get('ability_gain','')[:50])
print('parenthetical:', eff.get('parenthetical'))
acp = eff.get('activation_condition_parsed')
if acp:
    print('activation_position:', acp.get('position'))
ge = eff.get('gained_effect')
if ge:
    print('gained_effect action:', ge.get('action'))
    print('gained_effect op:', ge.get('operation'))
    print('gained_effect val:', ge.get('value'))
else:
    print('NO gained_effect!')
