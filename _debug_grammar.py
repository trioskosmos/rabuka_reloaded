"""Debug grammar mismatches."""
import sys, os, json, re
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'test_parser'))
from grammar import parse_action

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

# Debug gain_resource and move_cards mismatches
for category, ref_action in [('gain_resource', 'gain_resource'), ('move_cards', 'move_cards')]:
    print(f'\n=== {ref_action} mismatches ===')
    count = 0
    for a in data['unique_abilities']:
        for leaf in walk(a.get('effect') or {}):
            text = strip_icons(leaf.get('text',''))
            if len(text) < 3: continue
            r = parse_action(text)
            ref = leaf.get('action','?')
            if ref == ref_action and r.get('action') != ref_action:
                count += 1
                got = r.get('action','?')
                verb = r.get('verb','?')
                print(f'  got={got:20s} verb={verb:15s} text={text[:50]}')
                if count >= 15: break
        if count >= 15: break
    print(f'  Total: {count} mismatches')
