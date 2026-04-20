"""Show all abilities with 登場させる."""
import json

# Load abilities
with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

# Find all abilities with 登場させる
toujou_abilities = []
for ability in abilities:
    text = ability.get('triggerless_text', '')
    if '登場させる' in text:
        toujou_abilities.append(ability)

print(f"Total abilities with 登場させる: {len(toujou_abilities)}\n")

for i, ability in enumerate(toujou_abilities, 1):
    print(f"{i}. {ability['triggerless_text'][:150]}...")
    print(f"   Cards: {ability['cards'][0] if ability['cards'] else 'N/A'}")
    print()
