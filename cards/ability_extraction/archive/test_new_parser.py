"""Test the new parser on the example ability."""
import json
from parser import parse_ability

# Test the example from earlier
test_ability = "自分の成功ライブカード置き場にカードが2枚以上ある場合、自分の控え室からライブカードを1枚手札に加える。"

print("=== Testing New Parser ===")
print(f"Input: {test_ability}\n")

result = parse_ability(test_ability)

print("Parsed result:")
print(json.dumps(result, indent=2, ensure_ascii=False))

# Test another example
test_ability2 = "{{icon_energy.png|E}}{{icon_energy.png|E}}手札を1枚控え室に置く：このカードを控え室からステージに登場させる。"
print("\n\n=== Test 2 ===")
print(f"Input: {test_ability2}\n")

result2 = parse_ability(test_ability2)
print("Parsed result:")
print(json.dumps(result2, indent=2, ensure_ascii=False))

# Test cost only
test_ability3 = "{{icon_energy.png|E}}{{icon_energy.png|E}}手札を1枚控え室に置く"
print("\n\n=== Test 3 (cost only) ===")
print(f"Input: {test_ability3}\n")

result3 = parse_ability(test_ability3)
print("Parsed result:")
print(json.dumps(result3, indent=2, ensure_ascii=False))
