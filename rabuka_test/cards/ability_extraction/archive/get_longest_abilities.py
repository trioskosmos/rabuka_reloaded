"""Get the 20 longest abilities for analysis."""
import json

# Load abilities
with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

# Sort by length
sorted_abilities = sorted(abilities, key=lambda x: len(x.get('triggerless_text', '')), reverse=True)

# Get top 20
longest_abilities = sorted_abilities[:20]

print("=== 20 LONGEST ABILITIES ===\n")
for i, ability in enumerate(longest_abilities, 1):
    text = ability.get('triggerless_text', '')
    print(f"{i}. ({len(text)} chars)")
    print(f"   {text}")
    print()

# Save to file
with open('longest_abilities.json', 'w', encoding='utf-8') as f:
    json.dump(longest_abilities, f, ensure_ascii=False, indent=2)

print(f"\nSaved to longest_abilities.json")
