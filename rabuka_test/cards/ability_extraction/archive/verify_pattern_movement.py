"""Verify card movement pattern: XをYからZに置く"""
import json

with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

# Pattern: "を" + "から" + "に" + "置く"
pattern_matches = []

for ability in abilities:
    text = ability.get('triggerless_text', '')
    if 'を' in text and 'から' in text and 'に' in text and '置く' in text:
        pattern_matches.append(ability)

print(f"=== CARD MOVEMENT PATTERN: を...から...に...置く ===")
print(f"Total matches: {len(pattern_matches)}\n")

# Show first 20 matches
for i, ability in enumerate(pattern_matches[:20], 1):
    text = ability.get('triggerless_text', '')
    print(f"{i}. {text}")
    print()

print(f"\n=== ANALYSIS ===")
print(f"This pattern appears in {len(pattern_matches)} abilities.")
print(f"Need to manually verify if all these represent card movement from source to destination.")
