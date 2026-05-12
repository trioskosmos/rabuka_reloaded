import json, sys
sys.path.insert(0, 'cards/ability_extraction')

with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

a = data['unique_abilities'][150]
t = a['triggerless_text']

import parser

# Test what happens when parse_effect processes the post-condition text
txt = 'ライブの合計スコアを-1する。この効果ではライブの合計スコアは0以下にはならない'
r = parser.parse_effect(txt)
print('Effect action:', r.get('action'))
if r.get('action') == 'sequential':
    for i, act in enumerate(r.get('actions', [])):
        print('  [%d] action=%s op=%s val=%s text=%s' % (
            i, act.get('action'), act.get('operation'), act.get('value'), act.get('text','')[:50]))
elif r.get('action') == 'modify_score':
    print('Direct: op=%s val=%s' % (r.get('operation'), r.get('value')))

# Now test just parts[0]
txt0 = 'ライブの合計スコアを-1する'
r0 = parser.parse_effect(txt0)
print()
print('Part 0 action:', r0.get('action'))
print('Part 0 op:', r0.get('operation'))
print('Part 0 val:', r0.get('value'))

# Test parts[1]
txt1 = 'この効果ではライブの合計スコアは0以下にはならない'
r1 = parser.parse_effect(txt1)
print()
print('Part 1 action:', r1.get('action'))
