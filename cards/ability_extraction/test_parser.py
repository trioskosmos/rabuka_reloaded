"""Test parser on SUNNY DAY SONG branch 2 text."""
import sys
import json
sys.path.insert(0, '.')
from parser import parse_effect, parse_ability

# Branch 2 action (after condition is stripped)
text = "自分のステージにいる'μ's'のメンバー1人は、ライブ終了まで、{{heart_03.png|heart03}}を得る"
result = parse_effect(text)
print('=== Branch 2 action (no condition) ===')
print(json.dumps(result, indent=2, ensure_ascii=False))

# Branch 2 with condition
text2 = '2人以上いる場合、' + text
result2 = parse_effect(text2)
print()
print('=== Branch 2 with condition ===')
print(json.dumps(result2, indent=2, ensure_ascii=False))

# Full triggerless text
full = "自分のステージにメンバーが1人以上いる場合、自分と相手はカードを1枚引き、手札から1枚を控え室に置く。2人以上いる場合、さらに自分のステージにいる'μ's'のメンバー1人は、ライブ終了まで、{{heart_03.png|heart03}}を得る。3人以上いて、それぞれ名前が異なる場合、さらにこのカードのスコアを＋1する。"
result3 = parse_ability(full)
print()
print('=== Full ability ===')
print(json.dumps(result3.get('effect', {}), indent=2, ensure_ascii=False))
