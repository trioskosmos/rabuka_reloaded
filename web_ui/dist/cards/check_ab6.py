import json
with open('abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']
ab6 = abilities[6]
print(f"Ability 6:")
print(f"  Full text: {ab6.get('full_text', 'N/A')[:100]}...")
print(f"  Effect: {json.dumps(ab6.get('effect'), ensure_ascii=False, indent=2)[:2000]}")
