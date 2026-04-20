"""Verify gain resource pattern: Xを得る"""
import json

with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

# Pattern: "を得る"
pattern_matches = []

for ability in abilities:
    text = ability.get('triggerless_text', '')
    if 'を得る' in text:
        pattern_matches.append(ability)

print(f"=== GAIN RESOURCE PATTERN: を得る ===")
print(f"Total matches: {len(pattern_matches)}\n")

# Show first 30 matches
for i, ability in enumerate(pattern_matches[:30], 1):
    text = ability.get('triggerless_text', '')
    print(f"{i}. {text}")
    print()

print(f"\n=== ANALYSIS ===")
print(f"This pattern appears in {len(pattern_matches)} abilities.")
print(f"Need to manually verify if all these represent gaining something (hearts, blades, abilities).")
