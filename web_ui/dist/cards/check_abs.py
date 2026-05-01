import json
with open('abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']

for idx in [13, 31, 99, 242]:
    ab = abilities[idx]
    print(f"\n=== Ability {idx} ===")
    print(f"Text: {ab.get('full_text', 'N/A')[:80]}...")
    effect = ab.get('effect', {})
    print(f"Effect: {json.dumps(effect, ensure_ascii=False, indent=2)[:1500]}")
