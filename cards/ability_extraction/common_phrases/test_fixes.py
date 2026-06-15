import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_effect, parse_condition

ok = True

def check(label, got, expected):
    global ok
    if got != expected:
        print(f'FAIL: {label}: got {got!r}, expected {expected!r}')
        ok = False
    else:
        print(f'  OK: {label}')

# ── 1. same_name in card_count_condition ──
print('\n[1] same_name in card_count_condition')
t = '自分のステージに同じ名前の虹ヶ咲のメンバーが2人以上いる場合'
r = parse_condition(t)
check('same_name flag', r.get('same_name'), True)
check('type', r.get('type'), 'card_count_condition')
check('count', r.get('count'), 2)

# ── 2. same_name in gain_resource action ──
print('\n[2] same_name in gain_resource action')
t = '控え室に置いたカードと同じ名前を持つメンバー1人は、ライブ終了時まで、{{heart_04.png|heart04}}{{icon_blade.png|ブレード}}を得る'
r = parse_effect(t)
check('same_name in action', r.get('same_name'), True)
check('action type', r.get('action'), 'gain_resource')

# ── 3. different_name (カード名の異なる) in card_count_condition ──
print('\n[3] different_name in card_count_condition')
t = '自分の控え室にカード名の異なる虹ヶ咲のライブカードが4枚以上ある場合'
r = parse_condition(t)
check('distinct in condition', r.get('distinct'), 'card_name')
check('type', r.get('type'), 'card_count_condition')
check('count', r.get('count'), 4)

# ── 4. OR-location (zone1かzone2) → locations array ──
print('\n[4] OR-location locations array')
t = '自分の成功ライブカード置き場かライブ中のライブカードの中に、必要ハートに含まれる{{heart_01.png|heart01}}が4の虹ヶ咲のライブカードがある場合'
r = parse_condition(t)
check('locations array', r.get('locations'), ['success_live_card_zone', 'live_card_zone'])

# ── 5. Heart content filter ──
print('\n[5] heart content filter')
check('heart_colors', r.get('heart_colors'), ['heart01'])
check('heart count', r.get('count'), 4)

# ── 6. Heart content with different values ──
print('\n[6] heart content (different count)')
t = '自分の成功ライブカード置き場かライブ中のライブカードの中に、必要ハートに含まれる{{heart_01.png|heart01}}が3の虹ヶ咲のライブカードがある場合'
r = parse_condition(t)
check('heart_colors', r.get('heart_colors'), ['heart01'])
check('heart count', r.get('count'), 3)

# ── Summary ──
print()
if ok:
    print('ALL TESTS PASSED')
else:
    print('SOME TESTS FAILED')
    sys.exit(1)
