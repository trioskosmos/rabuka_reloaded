"""Verify both-target fields set correctly."""
import sys, json
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_effect, parse_condition

checks = [
    ('Equality condition should have scope=both',
     '自分と相手の成功ライブカード置き場にあるカードの枚数が同じ場合',
     lambda c: c.get('scope') == 'both' and c.get('operator') == '='),
    ('Combined condition should have scope=both + aggregate=total',
     '自分と相手の成功ライブカード置き場にカードが合計3枚以上ある場合',  
     lambda c: c.get('scope') == 'both' and c.get('aggregate') == 'total'),
    ('Combined energy condition should have scope=both + aggregate=total',
     '自分と相手のエネルギーの合計が15枚以上あるかぎり',
     lambda c: c.get('scope') == 'both' and c.get('aggregate') == 'total'),
    ('Independent action: target=both + independent=true',
     '自分と相手はそれぞれ、自身の控え室からライブカードを1枚手札に加える。',
     lambda e: e.get('target') == 'both' and e.get('independent') == True),
    ('Independent action (energy): move_cards not change_state',
     '自分と相手はそれぞれ、自身のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。',
     lambda e: e.get('action') == 'move_cards' and e.get('target') == 'both' and e.get('independent') == True),
    ('Restriction: target=both',
     'このターン、自分と相手のステージにいるメンバーは、効果によってはアクティブにならない。',
     lambda e: e.get('target') == 'both'),
]

all_pass = True
for name, text, check in checks:
    result = parse_condition(text) if '条件' in locals() else parse_effect(text)
    # Actually use parse_condition for pure conditions, parse_effect for full abilities
    if '場合' in text or 'かぎり' in text:
        result = parse_condition(text)
    else:
        result = parse_effect(text)
    if check(result):
        print('PASS:', name)
    else:
        print('FAIL:', name, json.dumps(result, ensure_ascii=False))
        all_pass = False

print()
if all_pass:
    print('ALL PASSED')
else:
    print('SOME FAILED')
    sys.exit(1)
