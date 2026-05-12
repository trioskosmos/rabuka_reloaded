import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_effect as pe
from parser import _normalize_effect_tree, _clean, normalize, split_cost_effect

t = '{{center.png|センター}}メンバー1人をウェイトにする：ライブ終了時まで、これによってウェイト状態になったメンバーは、「{{jyouji.png|常時}}ライブの合計スコアを+1する。」を得る。（この能力はセンターエリアに登場している場合のみ起動できる。）'

norm = normalize(t.strip())
cost, effect_text = split_cost_effect(norm)
effect = pe(effect_text)
effect = _normalize_effect_tree(effect, norm)

# Replicate _enrich_gain
def enrich(d):
    nodes = []
    def collect(n, lst):
        if isinstance(n, dict):
            if n.get('action') == 'gain_ability' and n.get('ability_gain'):
                lst.append(n)
            for v in n.values():
                if isinstance(v, (dict, list)):
                    collect(v, lst)
        elif isinstance(n, list):
            for item in n:
                collect(item, lst)
    collect(d, nodes)
    for node in nodes:
        if 'gained_effect' not in node:
            gained = pe(node['ability_gain'])
            if gained and gained.get('action') and gained.get('action') != 'custom':
                node['gained_effect'] = gained

enrich(effect)
print('After enrich:')
print('  action:', effect.get('action'))
if effect.get('action') == 'gain_ability':
    print('  ability_gain:', effect.get('ability_gain','')[:40])
    print('  gained_effect:', effect.get('gained_effect',{}).get('action'))
else:
    print('  actions:', [a.get('action') for a in effect.get('actions',[])])

effect = _clean(effect)
print('After clean:')
print('  action:', effect.get('action'))
