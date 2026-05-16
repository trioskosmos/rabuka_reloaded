from parser_v2 import parse_ability_v2
import json

unknown_texts = [
    "その中から{{heart_02.png|heart02}}か{{heart_04.png|heart04}}か{{heart_05.png|heart05}}を持つメンバーカードを3枚まで公開して手札に加えてもよい",
    "その中からハートに{{heart_04.png|heart04}}を2つ以上持つメンバーカードを1枚公開して手札に加えてもよい",
    "その中からハートに{{heart_02.png|heart02}}を2個以上持つメンバーカードか",
    "必要ハートに{{heart_04.png|heart04}}を2以上含むライブカードを1枚公開して手札に加えてもよい",
]

for text in unknown_texts:
    try:
        result = parse_ability_v2(text)
        print(f"SUCCESS: {text}")
        print(
            json.dumps(
                result.model_dump(exclude_none=True), ensure_ascii=False, indent=2
            )
        )
    except Exception as e:
        print(f"FAILED: {text}")
        print(f"Error: {e}")
    print("-" * 20)
