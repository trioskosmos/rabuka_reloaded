import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_effect, _normalize_effect_tree, _clean, normalize, split_cost_effect

with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

a = data['unique_abilities'][54]
tt = a['triggerless_text']

norm = normalize(tt.strip())
cost, effect_text = split_cost_effect(norm)
effect = parse_effect(effect_text)
effect = _normalize_effect_tree(effect, norm)

# Add _enrich_gain manually
def enrich_gain(d):
    from parser import parse_effect as pe
    gain_nodes = []
    def collect(n, nodes):
        if isinstance(n, dict):
            if n.get('action') == 'gain_ability' and n.get('ability_gain'):
                nodes.append(n)
            for v in n.values():
                if isinstance(v, (dict, list)):
                    collect(v, nodes)
        elif isinstance(n, list):
            for item in n:
                collect(item, nodes)
    collect(d, gain_nodes)
    for node in gain_nodes:
        if 'gained_effect' not in node:
            gained = pe(node['ability_gain'])
            if gained and gained.get('action') and gained.get('action') != 'custom':
                node['gained_effect'] = gained

enrich_gain(effect)

# Now check
print('After enrich_gain:')
print('  action:', effect.get('action'))
if effect.get('action') == 'sequential':
    for i, ac in enumerate(effect.get('actions', [])):
        print('  [%d] %s text=%s' % (i, ac.get('action'), ac.get('text','')[:50]))
else:
    print('  ability_gain:', effect.get('ability_gain','')[:40])
    print('  parenthetical:', effect.get('parenthetical') is not None)
    print('  gained_effect:', effect.get('gained_effect') is not None)
    if effect.get('gained_effect'):
        print('  gained_effect action:', effect['gained_effect'].get('action'))

effect = _clean(effect)
print()
print('After _clean:')
print('  action:', effect.get('action'))
if effect.get('action') == 'sequential':
    for i, ac in enumerate(effect.get('actions', [])):
        print('  [%d] %s text=%s' % (i, ac.get('action'), ac.get('text','')[:50]))
