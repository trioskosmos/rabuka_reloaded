import sys, os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from parser import parse_ability, extract_cost_limit, extract_operator

# Card 2: cost 11+ filter
t2 = '自分のデッキの上からカードを3枚見る。その中からコスト11以上のカードを1枚公開して手札に加えてもよい。残りを控え室に置く。'
print("Input text:", t2)
print()

# Test individual extraction functions
print("extract_cost_limit:", extract_cost_limit(t2))
print("extract_operator:", extract_operator(t2))
print()

result = parse_ability(t2)
print("Full result:")
import json
print(json.dumps(result, indent=2, ensure_ascii=False))
