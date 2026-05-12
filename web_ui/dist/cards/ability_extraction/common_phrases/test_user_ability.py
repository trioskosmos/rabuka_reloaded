import sys, json
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_ability
t = '自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、かつ名前が異なる場合、「{{jyouji.png|常時}}ライブの合計スコアを+1する。」を得る。'
r = parse_ability(t)
eff = r.get('effect', {})
print('action:', eff.get('action'))
print('ability_gain:', eff.get('ability_gain',''))
ge = eff.get('gained_effect')
if ge:
    print('gained_effect action:', ge.get('action'))
    print('gained_effect op:', ge.get('operation'))
    print('gained_effect val:', ge.get('value'))
if eff.get('action') == 'sequential':
    for i, a in enumerate(eff.get('actions',[])):
        print('  [%d] %s: text=%s' % (i, a.get('action'), a.get('text','')[:40]))
