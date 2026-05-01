import json
with open('abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']
ab63 = abilities[63]
print(f'Ability 63:')
print(f'  Text: {ab63.get("full_text", "N/A")[:60]}...')
print(f'  Cost: {json.dumps(ab63.get("cost"), ensure_ascii=False, indent=2)}')
