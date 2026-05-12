import sys, os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from parser import parse_ability

# Nico card ability from README
nico_text = '{{toujyou.png|登場}}自分と相手はそれぞれ、自身の控え室からコスト2以下のメンバーカードを1枚、メンバーのいないエリアにウェイト状態で登場させる。（この効果で登場したメンバーのいるエリアには、このターンにメンバーは登場できない。）'
print("Nico card ability:")
print(nico_text)
print()

result = parse_ability(nico_text)
print("Parsed result:")
import json
print(json.dumps(result, indent=2, ensure_ascii=False))

# Check expected fields from README
print("\n=== Expected fields check ===")
effect = result.get('effect', {})
print("action:", effect.get('action'))
print("source:", effect.get('source'))
print("destination:", effect.get('destination'))
print("cost_limit:", effect.get('cost_limit'))
print("cost_limit_operator:", effect.get('cost_limit_operator'))
print("state_change:", effect.get('state_change'))
print("count:", effect.get('count'))
print("card_type:", effect.get('card_type'))
print("target:", effect.get('target'))
print("multiple_targets:", effect.get('multiple_targets'))
