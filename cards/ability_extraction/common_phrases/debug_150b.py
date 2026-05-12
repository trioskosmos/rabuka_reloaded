import json, sys
sys.path.insert(0, 'cards/ability_extraction')

with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

a = data['unique_abilities'][150]
t = a['triggerless_text']

# Try parsing the sub-text directly
import parser
txt = 'ライブの合計スコアを-1する。この効果ではライブの合計スコアは0以下にはならない'
print('Sub-text:', repr(txt))
r = parser.parse_effect(txt)
print('Effect action:', r.get('action'))
print('Operation:', r.get('operation'))
print('Value:', r.get('value'))
if r.get('action') == 'sequential':
    for i, act in enumerate(r.get('actions', [])):
        print('  [%d] %s op=%s val=%s' % (i, act.get('action'), act.get('operation'), act.get('value')))
elif r.get('action') == 'modify_score':
    print('Direct modify_score: op=%s val=%s' % (r.get('operation'), r.get('value')))

# Also test just the parse_action on the raw text
print()
print('parse_action raw:')
r2 = parser.parse_action(txt)
print('Action:', r2.get('action'))
print('Operation:', r2.get('operation'))
print('Value:', r2.get('value'))
