import json, sys, traceback
sys.path.insert(0, 'cards/ability_extraction')
import parser

with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

a = data['unique_abilities'][150]
t = a['triggerless_text']

# Monkey-patch parse_action to trace what happens
orig_parse_action = parser.parse_action
def traced_parse_action(text):
    result = orig_parse_action(text)
    if 'スコアを' in text and ('-1' in text or '－１' in text or '\uff0d' in text):
        print('PARSE_ACTION called with:', repr(text[:80]))
        print('  result action:', result.get('action'))
        print('  result op:', result.get('operation'))
        print('  result val:', result.get('value'))
    return result
parser.parse_action = traced_parse_action

# Also trace parse_effect
orig_parse_effect = parser.parse_effect
def traced_parse_effect(text):
    result = orig_parse_effect(text)
    if 'スコアを' in text and ('-1' in text or '－１' in text):
        print('PARSE_EFFECT called with:', repr(text[:80]))
        print('  result action:', result.get('action'))
        if result.get('action') == 'sequential':
            for i, act in enumerate(result.get('actions', [])):
                print('  [%d] %s op=%s val=%s' % (i, act.get('action'), act.get('operation'), act.get('value')))
        else:
            print('  op=%s val=%s' % (result.get('operation'), result.get('value')))
    return result
parser.parse_effect = traced_parse_effect

r = parser.parse_ability(t)
eff = r.get('effect', {})
print('\nFINAL effect trees:')
def show(d, depth=0):
    if isinstance(d, dict):
        a = d.get('action')
        if a:
            print('  ' * depth + 'action=%s op=%s val=%s' % (a, d.get('operation'), d.get('value')))
        for v in d.values():
            if isinstance(v, (dict, list)):
                show(v, depth+1)
    elif isinstance(d, list):
        for item in d:
            show(item, depth+1)
show(eff)
