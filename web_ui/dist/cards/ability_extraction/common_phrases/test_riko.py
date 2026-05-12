import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_ability

t2 = '{{center.png|センター}}自分のステージの右サイドエリアと左サイドエリアにいるメンバーのコストが同じ場合、相手のステージにいる元々持つ{{icon_blade.png|ブレード}}の数が3つ以下のすべてのメンバーをウェイトにする。'
r2 = parse_ability(t2)
e2 = r2.get('effect', {})
print('action:', e2.get('action'))
cond = e2.get('condition', {})
print('condition type:', cond.get('type'))
if cond.get('type') == 'compound':
    for i, c in enumerate(cond.get('conditions', [])):
        print('  [%d] type=%s target=%s' % (i, c.get('type'), c.get('target')))
        print('       text=%s' % c.get('text','')[:80])
else:
    print('condition:', json.dumps(cond, ensure_ascii=False)[:500])
print('all:', e2.get('all'))
print('blade_limit:', e2.get('blade_limit'))
print('blade_limit_operator:', e2.get('blade_limit_operator'))
print('original_value:', e2.get('original_value'))
print('target:', e2.get('target'))
print('activation_position:', e2.get('activation_position'))
