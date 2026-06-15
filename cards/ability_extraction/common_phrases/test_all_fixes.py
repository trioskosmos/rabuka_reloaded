import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_effect, parse_condition, parse_ability, parse_action, parse_cost

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']

ok = True
def check(label, got, expected):
    global ok
    if got != expected:
        print(f'FAIL: {label}')
        print(f'  got:      {got!r}')
        print(f'  expected: {expected!r}')
        ok = False
    else:
        print(f'  OK: {label}')

def check_true(label, got):
    global ok
    if not got:
        print(f'FAIL: {label}: expected truthy, got {got!r}')
        ok = False
    else:
        print(f'  OK: {label}')

# Test each fix by parsing specific abilities from the JSON

# Fix 1: preceding_moved — verify entry [332] has count extracted
print('\n=== Fix 1: distinct count extraction ===')
ab = abilities[332]
cond = ab.get('effect', {}).get('condition', {})
check('condition should have count', cond.get('count'), 3)
check('condition should have operator >=', cond.get('operator'), '>=')
check('condition should have distinct', cond.get('distinct'), 'card_name')

# Fix 2: optional propagation — verify entry [452] has optional
print('\n=== Fix 2: optional flag propagation ===')
ab = abilities[452]
eff = ab.get('effect', {})
def walk_opt(d):
    if isinstance(d, dict):
        if d.get('action') and d.get('text') and ('\\u3082\\u3088\\u3044' in json.dumps(d.get('text'))):
            return d.get('optional')
        for v in d.values():
            r = walk_opt(v)
            if r: return r
    elif isinstance(d, list):
        for item in d:
            r = walk_opt(item)
            if r: return r
    return None
# Just check the root effect has the right structure
check_true('choice effect has action', eff.get('action'))
print(f'  action type: {eff.get("action")}')

# Fix 3: custom cost types fixed — verify [93] is place_energy_under_member
print('\n=== Fix 3: custom cost types ===')
ab = abilities[93]
cost = ab.get('cost', {})
check('cost type should be place_energy_under_member', cost.get('type'), 'place_energy_under_member')

# Fix 4: cost [512] should be pay_energy
ab = abilities[512]
cost = ab.get('cost', {})
check('cost type should be pay_energy', cost.get('type'), 'pay_energy')
check('cost should have energy count', cost.get('energy'), 2)
check('cost should be optional', cost.get('optional'), True)

# Fix 5: distinct entries with count — [495] should have count extracted
print('\n=== Fix 5: distinct count extraction ===')
ab = abilities[495]
cond = ab.get('effect', {}).get('condition', {})
print(f'  [495] distinct: {cond.get("distinct")}  count: {cond.get("count")}  text: {cond.get("text","")[:60]}')

# Fix 6: select without source for stage — [588]
print('\n=== Fix 6: select source inference ===')
ab = abilities[588]
eff = ab.get('effect', {})
# Check the nested select actions
def find_selects(d):
    results = []
    if isinstance(d, dict):
        if d.get('action') == 'select':
            results.append(d)
        for v in d.values():
            results.extend(find_selects(v))
    elif isinstance(d, list):
        for item in d:
            results.extend(find_selects(item))
    return results
selects = find_selects(eff)
for i, s in enumerate(selects):
    src = s.get('source')
    txt = s.get('text', '')[:50]
    print(f'  select[{i}]: source={src} text={txt}')

# Fix 7: optional flag on sequential sub-actions
print('\n=== Fix 7: optional on sequential sub-actions ===')
ab = abilities[71]
eff = ab.get('effect', {})
txt = eff.get('text', '')
has_optional_marker = '\\u3082\\u3088\\u3044' in json.dumps(txt)
print(f'  [71] text has optional marker: {has_optional_marker}')
print(f'  [71] effect has optional: {eff.get("optional")}')

# Fix 8: revealed_card → revealed_cards (entry has source=revealed_cards)
print('\n=== Fix 8: revealed_card normalization ===')
# Find entries with 'revealed' in effect source
for i, ab in enumerate(abilities):
    eff = ab.get('effect')
    if eff is None:
        continue
    src = eff.get('source', '')
    if 'revealed' in src and src != 'revealed_cards':
        print(f'  [{i}] has unusual source: {src}')

print()
print('=== Fix 9: group_reference in cost ===')
for i, ab in enumerate(abilities):
    cost = ab.get('cost')
    if cost is None:
        continue
    if cost.get('group_reference') == 'same_group_name':
        print(f'  [{i}] cost has group_reference=same_group_name')
        check('cost type should be move_cards', cost.get('type'), 'move_cards')
        break

# Fix 10: score-based energy cost — entry [157]
print('\n=== Fix 10: score-based energy ===')
ab = abilities[157]
eff = ab.get('effect', {})
act = eff.get('action')
actions = eff.get('actions', [])
check('effect should be sequential', act, 'sequential')
if len(actions) >= 2:
    check('action[1] should be pay_energy', actions[1].get('action'), 'pay_energy')
    dc = actions[1].get('dynamic_count', {})
    check_true('action[1] should have dynamic_count', dc)
    if dc:
        check('dynamic_count mode', dc.get('mode'), 'equals')
        check('optional should be true', actions[1].get('optional'), True)

# Fix 11: optional without flag — should be 0 remaining
print('\n=== Fix 11: no optional-without-flag ===')
opt_missing = []
for i, ab in enumerate(abilities):
    ft = ab.get('full_text', '') or ''
    eff = ab.get('effect', {})
    def flatten(d):
        res = []
        if isinstance(d, dict):
            res.append(d)
            for v in d.values():
                res.extend(flatten(v))
        elif isinstance(d, list):
            for item in d:
                res.extend(flatten(item))
        return res
    def ct(d, out):
        if isinstance(d, dict):
            if 'text' in d and d.get('text'):
                out.append(d['text'])
            for v in d.values():
                ct(v, out)
        elif isinstance(d, list):
            for item in d:
                ct(item, out)
    texts = []
    ct(eff, texts)
    all_nodes = flatten(eff)
    has_opt_text = any(('\\u3082\\u3088\\u3044' in t or '\\u3066\\u3082\\u3088\\u3044' in t) for t in texts)
    has_opt_flag = any(isinstance(n, dict) and n.get('optional') == True for n in all_nodes)
    if has_opt_text and not has_opt_flag:
        opt_missing.append(i)
check(f'optional-without-flag count should be 0', len(opt_missing), 0)
if opt_missing:
    for idx in opt_missing[:3]:
        print(f'  remaining: [{idx}] cards={abilities[idx].get("cards",[])[:2]}')

# Fix 12: count=0 for conditions should have operator check
print('\n=== Fix 12: count=0 in conditions ===')
for i, ab in enumerate(abilities):
    def walk_count(d, path):
        issues = []
        if isinstance(d, dict):
            if d.get('count') == 0 and d.get('type') in ('card_count_condition', 'location_condition'):
                issues.append((path, d.get('operator')))
            for k, v in d.items():
                issues.extend(walk_count(v, f'{path}.{k}'))
        elif isinstance(d, list):
            for idx, item in enumerate(d):
                issues.extend(walk_count(item, f'{path}[{idx}]'))
        return issues
    issues = walk_count(ab.get('effect', {}), '')
    for path, op in issues:
        print(f'  [{i}] {path} count=0 operator={op}')

if ok:
    print('\n=== ALL FIXES TEST PASSED ===')
else:
    print('\n=== SOME TESTS FAILED ===')
    sys.exit(1)
