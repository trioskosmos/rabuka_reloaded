"""Analyze all abilities with を (object marker) to understand actual usage."""
import json

# Load abilities
with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

print("=== ANALYZING を (OBJECT MARKER) USAGE ===\n")

# Find all abilities with を
wo_abilities = []
for ability in abilities:
    text = ability.get('triggerless_text', '')
    if 'を' in text:
        wo_abilities.append(ability)

print(f"Total abilities with を: {len(wo_abilities)}\n")

# Show first 20 examples with context
for i, ability in enumerate(wo_abilities[:30], 1):
    text = ability.get('triggerless_text', '')
    # Find all occurrences of を and show context
    occurrences = []
    for j, char in enumerate(text):
        if char == 'を':
            context_start = max(0, j-10)
            context_end = min(len(text), j+11)
            context = text[context_start:context_end]
            occurrences.append(context)
    
    print(f"--- Ability {i} ({len(text)} chars) ---")
    print(f"Text: {text}")
    print(f"を contexts ({len(occurrences)}):")
    for occ in occurrences:
        print(f"  ...{occ}...")
    print()

print(f"\n=== SUMMARY ===")
print(f"Total abilities with を: {len(wo_abilities)}")
