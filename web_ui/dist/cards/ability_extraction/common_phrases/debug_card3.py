import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_effect, _normalize_effect_tree, _clean, normalize, split_cost_effect

t = '{{center.png|センター}}メンバー1人をウェイトにする：ライブ終了時まで、これによってウェイト状態になったメンバーは、「{{jyouji.png|常時}}ライブの合計スコアを+1する。」を得る。（この能力はセンターエリアに登場している場合のみ起動できる。）'

norm = normalize(t.strip())
cost, effect_text = split_cost_effect(norm)
print('effect_text:', effect_text[:80])

effect = parse_effect(effect_text)
print('After parse_effect:')
print('  action:', effect.get('action'))
if effect.get('action') == 'gain_ability':
    print('  ability_gain:', effect.get('ability_gain','')[:40])
else:
    print('  actions:', [a.get('action') for a in effect.get('actions',[])])

effect2 = _normalize_effect_tree(effect, norm)
print('After _normalize_effect_tree:')
print('  action:', effect2.get('action'))
if effect2.get('action') == 'gain_ability':
    print('  ability_gain:', effect2.get('ability_gain','')[:40])
else:
    print('  actions:', [a.get('action') for a in effect2.get('actions',[])])

effect3 = _clean(effect2)
print('After _clean:')
print('  action:', effect3.get('action'))
if effect3.get('action') == 'gain_ability':
    print('  ability_gain:', effect3.get('ability_gain','')[:40])
else:
    print('  actions:', [a.get('action') for a in effect3.get('actions',[])])
