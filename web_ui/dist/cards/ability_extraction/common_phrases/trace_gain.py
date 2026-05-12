import json, sys, traceback
sys.path.insert(0, 'cards/ability_extraction')
from parser import _EFFECT_HANDLERS, parse_effect, parse_action

t = 'これによってウェイト状態になったメンバーは、「{{jyouji.png|常時}}ライブの合計スコアを+1する。」を得る。'

print('Testing parse_action directly:')
r = parse_action(t)
print('  action:', r.get('action'))
print('  keys:', list(r.keys()))
print()

print('Testing parse_effect directly:')
r = parse_effect(t)
print('  action:', r.get('action'))
if r.get('action') == 'sequential':
    for i, a in enumerate(r.get('actions', [])):
        print('  [%d] %s text=%s' % (i, a.get('action'), a.get('text','')[:50]))
print('  parenthetical:', r.get('parenthetical'))
print('  activation_condition_parsed:', r.get('activation_condition_parsed'))
print()

# Now test each handler individually
print('Testing each handler:')
import re
for i, handler in enumerate(_EFFECT_HANDLERS):
    try:
        result = handler(t)
        if result is not None:
            print('  handler[%d] %s MATCHED: action=%s' % (i, handler.__name__, result.get('action')))
            if result.get('action') == 'sequential':
                for j, a in enumerate(result.get('actions', [])):
                    print('    [%d] %s text=%s' % (j, a.get('action'), a.get('text','')[:40]))
    except Exception as e:
        print('  handler[%d] %s ERROR: %s' % (i, handler.__name__, e))
