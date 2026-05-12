import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_effect, _normalize_effect_tree, _clean

t = 'ライブ終了時まで、これによってウェイト状態になったメンバーは、「{{jyouji.png|常時}}ライブの合計スコアを+1する。」を得る。（この能力はセンターエリアに登場している場合のみ起動できる。）'

print('=== After parse_effect ===')
r = parse_effect(t)
print('  action:', r.get('action'))
print('  parenthetical:', r.get('parenthetical'))
print('  keys:', sorted(r.keys()))

print()
print('=== After _normalize_effect_tree ===')
r2 = _normalize_effect_tree(r, t)
print('  action:', r2.get('action'))
if r2.get('action') == 'sequential':
    for i, a in enumerate(r2.get('actions', [])):
        print('  [%d] %s text=%s' % (i, a.get('action'), a.get('text','')[:50]))
print('  parenthetical:', r2.get('parenthetical'))

print()
print('=== After _clean ===')
r3 = _clean(r2)
print('  action:', r3.get('action'))
if r3.get('action') == 'sequential':
    for i, a in enumerate(r3.get('actions', [])):
        print('  [%d] %s text=%s' % (i, a.get('action'), a.get('text','')[:50]))
print('  parenthetical:', r3.get('parenthetical'))
