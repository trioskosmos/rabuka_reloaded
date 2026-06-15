import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_effect, parse_condition, parse_ability, parse_action

ok = True

def check(label, got, expected):
    global ok
    if got != expected:
        print(f'FAIL: {label}: got {got!r}, expected {expected!r}')
        ok = False
    else:
        print(f'  OK: {label}')

def check_in(label, got, container):
    global ok
    if got not in container:
        print(f'FAIL: {label}: {got!r} not in {container!r}')
        ok = False
    else:
        print(f'  OK: {label}')

# ── Fix 13: revealed_card → revealed_cards normalization ──
print('\n[Fix 13] revealed_card (singular) normalized to revealed_cards (plural)')
t = 'ライブの合計スコアが相手より高い場合、このカードを手札に加えてもよい'
r = parse_ability(t)
eff = r.get('effect', {})
src = eff.get('source')
check('source should be revealed_cards not revealed_card', src, 'revealed_cards')

# ── Fix 11 (9c): Score-based energy cost ──
print('\n[Fix 11/9c] Score-based energy cost with dynamic_count')
t = '自分の控え室にあるライブカードを1枚選び、そのカードのスコアに等しい数の{{icon_energy.png|E}}を支払ってもよい。そうした場合、そのライブカードを手札に加える。'
r = parse_effect(t)
# This should be a sequential with 3 actions: select + pay_energy(dynamic) + move_cards
check('should be sequential', r.get('action'), 'sequential')
acts = r.get('actions', [])
check('should have 3 actions', len(acts), 3)
if len(acts) >= 1:
    check('action[0] should be select', acts[0].get('action'), 'select')
    check('action[0] source should be discard', acts[0].get('source'), 'discard')
if len(acts) >= 2:
    check('action[1] should be pay_energy', acts[1].get('action'), 'pay_energy')
    dc = acts[1].get('dynamic_count', {})
    check('action[1] should have dynamic_count', dc.get('type'), 'dynamic_count')
    check('action[1] dynamic reference', dc.get('reference'), 'そのカードのスコア')
    check('action[1] dynamic mode', dc.get('mode'), 'equals')
    check('action[1] should be optional', acts[1].get('optional'), True)
if len(acts) >= 3:
    check('action[2] should be move_cards', acts[2].get('action'), 'move_cards')
    check('action[2] source', acts[2].get('source'), 'selected_cards')
    check('action[2] destination', acts[2].get('destination'), 'hand')

# ── Fix 12: Missing source in move_cards for "手札にある" ──
print('\n[Fix 12] Source inference for "手札にある" pattern')
t = '手札にあるコスト2以下のμ\'sのメンバーカードを1枚公開し、このメンバーの下に置いてもよい'
r = parse_action(t)
check('source should be hand for 手札にある', r.get('source'), 'hand')
check('destination should be under_member', r.get('destination'), 'under_member')
check('action should be move_cards', r.get('action'), 'move_cards')

# ── Fix 14: Missing destination inference ──
print('\n[Fix 14] Destination inference for bare "置いてもよい"')
# This pattern appears in conditional_alternative contexts
t = '自分の控え室にあるμ\'sのライブカードを1枚置いてもよい'
r = parse_action(t)
check('action should be move_cards', r.get('action'), 'move_cards')
check('source should be discard', r.get('source'), 'discard')

# ── Fix 15: count=0 handling ──
print('\n[Fix 15] count=0 handling for dynamic draw')
t = 'これにより置いた枚数分カードを引く'
r = parse_action(t)
check('action should be draw_card', r.get('action'), 'draw_card')
dc = r.get('dynamic_count', {})
# Should have dynamic_count instead of count=0
has_dc = 'dynamic_count' in r
check('should have dynamic_count when count=0', has_dc, True)
if has_dc:
    check('drawn_cards type', dc.get('type'), 'drawn_cards')
    check('drawn_cards reference', dc.get('reference'), 'previous_draw')

# ── Fix 17: select actions with source/destination ──
print('\n[Fix 17] select actions should have source/destination')
t = '自分の控え室にあるコスト4以下の虹ヶ咲のメンバーカードを1枚選ぶ'
r = parse_action(t)
check('action should be select', r.get('action'), 'select')
check('select should have source=discard', r.get('source'), 'discard')
check('select should have card_type=member_card', r.get('card_type'), 'member_card')

t2 = '自分のステージにいるAqoursのメンバー1人を選ぶ'
r2 = parse_action(t2)
check('action[2] should be select', r2.get('action'), 'select')
check('select[2] should have source=stage', r2.get('source'), 'stage')

# ── Fix 16: condition without text ──
print('\n[Fix 16] condition always has text field')
# Trigger text with movement condition
t = 'このターン、このメンバーがエリアを移動している場合、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る'
r = parse_ability(t)
cond = r.get('effect', {}).get('condition', {})
text = cond.get('text')
check('condition should have text field', bool(text), True)
check('condition type should not be custom', cond.get('type') != 'custom', True)

if ok:
    print('\n=== ALL PHASE 2 TESTS PASSED ===')
else:
    print('\n=== SOME TESTS FAILED ===')
    sys.exit(1)
