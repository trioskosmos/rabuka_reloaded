import json
with open('abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']
ab = abilities[13]
print(f"Ability 13:")
print(f"  Full text: {ab.get('full_text', 'N/A')[:100]}...")
print(f"  Effect: {json.dumps(ab.get('effect'), ensure_ascii=False, indent=2)[:2000]}")
