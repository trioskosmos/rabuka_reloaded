import sys, os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from parser import parse_ability

t3 = '{{center.png|センター}}メンバー1人をウェイトにする：ライブ終了時まで、これによってウェイト状態になったメンバーは、「{{jyouji.png|常時}}ライブの合計スコアを+1する。」を得る。（この能力はセンターエリアに登場している場合のみ起動できる。）'
print("Input text:", t3)
print()

result = parse_ability(t3)
print("Full result:")
import json
print(json.dumps(result, indent=2, ensure_ascii=False))
