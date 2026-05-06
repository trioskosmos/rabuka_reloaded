"""Run parser directly on specific texts to see current output."""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parser import parse_effect

# Test 1: "のみ発動する" in main text (no parentheses)
text1 = 'ライブの合計スコアが相手より高い場合、このカードを手札に加えてもよい。この能力は、このカードが自分のエールによって公開されている場合のみ発動する。'
print("=== TEST 1: のみ発動する in main text ===")
print(f"Input: {text1[:80]}...")
result1 = parse_effect(text1)
print(f"action: {result1.get('action')}")
print(f"activation_condition: {result1.get('activation_condition', 'MISSING')}")
print(f"parenthetical: {result1.get('parenthetical', 'MISSING')}")
print()

# Test 2: parenthetical with side area
text2 = '自分のデッキの上からカードを7枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。（この能力は左サイドエリアか右サイドエリアに登場した場合のみ発動する。）'
print("=== TEST 2: Side area parenthetical ===")
result2 = parse_effect(text2)
print(f"action: {result2.get('action')}")
print(f"activation_condition: {result2.get('activation_condition', 'MISSING')}")
print(f"parenthetical: {result2.get('parenthetical', 'MISSING')}")
print(f"'parenthetical' in result2: {'parenthetical' in result2}")
print()

# Test 3: Center area parenthetical (known working)
text3 = 'メンバー1人をウェイトにする：ライブ終了時まで、これによってウェイト状態になったメンバーは、「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。（この能力はセンターエリアに登場している場合のみ起動できる。）'
print("=== TEST 3: Center area parenthetical ===")
result3 = parse_effect(text3)
print(f"action: {result3.get('action')}")
print(f"activation_condition: {result3.get('activation_condition', 'MISSING')}")
print(f"parenthetical: {result3.get('parenthetical', 'MISSING')}")
print()

# Test 4: Simple side area case with draw+discard
text4 = 'カードを2枚引き、手札を2枚控え室に置く。（この能力は左サイドエリアか右サイドエリアに登場した場合のみ発動する。）'
print("=== TEST 4: Simple draw+discard with side area ===")
result4 = parse_effect(text4)
print(f"action: {result4.get('action')}")
print(f"activation_condition: {result4.get('activation_condition', 'MISSING')}")
print(f"parenthetical: {result4.get('parenthetical', 'MISSING')}")
print(f"keys: {list(result4.keys())}")
print()

# Test 5: Check the dispatch table for "のみ発動する"
print("=== TEST 5: parse_action for のみ発動する ===")
from parser import parse_action
text5 = 'この能力は、このカードが自分のエールによって公開されている場合のみ発動する。'
result5 = parse_action(text5)
print(f"action: {result5.get('action')}")
print(f"restriction_type: {result5.get('restriction_type', 'MISSING')}")
print()

print("DONE")
