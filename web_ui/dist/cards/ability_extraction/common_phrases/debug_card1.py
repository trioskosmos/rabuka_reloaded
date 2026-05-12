import sys, os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from parser import parse_ability

# Card 1: Liella! group filter
t1 = '{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分のデッキの上からカードを7枚見る。その中から『Liella!』のカードを1枚公開して手札に加えてもよい。残りを控え室に置く。'
print("Input text:", t1)
print()

result = parse_ability(t1)
print("Full result:")
import json
print(json.dumps(result, indent=2, ensure_ascii=False))
