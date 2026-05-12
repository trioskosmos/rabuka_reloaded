import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_effect, _normalize_effect_tree, normalize, split_cost_effect

with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

# Index 54
a = data['unique_abilities'][54]
tt = a['triggerless_text']

# Step 1: normalize
norm = normalize(tt.strip())

# Step 2: split
cost, effect_text = split_cost_effect(norm)

# Step 3: parse_effect
effect = parse_effect(effect_text)
print('After parse_effect:')
print('  action:', effect.get('action'))
if effect.get('action') == 'sequential':
    for i, ac in enumerate(effect.get('actions', [])):
        print('  [%d] %s text=%s' % (i, ac.get('action'), ac.get('text','')[:50]))
else:
    print('  ability_gain:', effect.get('ability_gain','')[:40])
    print('  parenthetical:', effect.get('parenthetical') is not None)

# Step 4: _normalize_effect_tree
print()
print('After _normalize_effect_tree:')
effect2 = _normalize_effect_tree(effect, norm)
print('  action:', effect2.get('action'))
if effect2.get('action') == 'sequential':
    for i, ac in enumerate(effect2.get('actions', [])):
        print('  [%d] %s text=%s' % (i, ac.get('action'), ac.get('text','')[:50]))
else:
    print('  ability_gain:', effect2.get('ability_gain','')[:40])
    print('  parenthetical:', effect2.get('parenthetical') is not None)
    print('  all keys:', sorted(effect2.keys()))
