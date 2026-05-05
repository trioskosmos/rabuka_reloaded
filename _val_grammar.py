"""Validate grammar-based parser against all 602 abilities."""
import sys, os, json, re
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'test_parser'))
from grammar import parse_action, parse_ability_text

with open(os.path.join(os.path.dirname(__file__), 'cards', 'abilities.json'), encoding='utf-8') as f:
    data = json.load(f)

def strip_icons(t):
    return re.sub(r'\{\{[^}]+\}\}', '', t).strip()

def walk(eff):
    if not isinstance(eff, dict): return
    a = eff.get('action','')
    if a and a not in ('sequential','conditional_alternative','look_and_select'): yield eff
    for k in ('actions','options','primary_effect','alternative_effect','select_action','look_action','condition'):
        if k in eff and isinstance(eff.get(k), dict): yield from walk(eff[k])
        elif k in eff and isinstance(eff.get(k), list):
            for sub in eff[k]: yield from walk(sub)

total = 0; matched = 0; by_action = {}

for a in data['unique_abilities']:
    for leaf in walk(a.get('effect') or {}):
        total += 1
        text = strip_icons(leaf.get('text',''))
        if len(text) < 3: continue
        result = parse_action(text)
        ref_act = leaf.get('action', '?')
        got_act = result.get('action', '?')
        
        if got_act not in by_action:
            by_action[got_act] = {'match': 0, 'total': 0}
        by_action[got_act]['total'] += 1
        
        if ref_act == got_act:
            matched += 1
            by_action[got_act]['match'] += 1

print(f'Grammar-based parser results: {matched}/{total} ({100*matched/total:.1f}%)')
print()
print('By action type:')
for act in sorted(by_action.keys()):
    m = by_action[act]['match']
    t = by_action[act]['total']
    pct = 100*m/t if t > 0 else 0
    print(f'  {act:25s} {m:4d}/{t:<4d} ({pct:5.1f}%)')
