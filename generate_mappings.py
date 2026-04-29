import json
import os
import re

# Read abilities.json to get all card IDs
with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    abilities = json.load(f)

# Read existing mappings
with open('web_ui/js/card_image_mapping.json', 'r', encoding='utf-8-sig') as f:
    existing_mappings = json.load(f)

# Extract all card IDs
card_ids = set()
if 'unique_abilities' in abilities:
    for ability in abilities['unique_abilities']:
        if 'cards' in ability:
            for card_str in ability['cards']:
                if isinstance(card_str, str):
                    card_id = card_str.split(' | ')[0].strip()
                    card_ids.add(card_id)

# Get all available image files
image_dir = 'web_ui/img/cards_webp'
image_files = set()
if os.path.exists(image_dir):
    for file in os.listdir(image_dir):
        if file.endswith('.webp'):
            image_files.add(file)

# Find missing mappings and try to find correct image files
new_mappings = {}
missing_mappings = []

for card_id in sorted(card_ids):
    expected_file = f"{card_id}.webp"
    
    if expected_file in image_files:
        continue  # Already exists
    
    if card_id in existing_mappings:
        continue  # Already mapped
    
    # Try common transformations to find the actual image file
    possible_files = []
    
    # 1. Special character replacements
    variants = [
        card_id.replace('＋', '+'),  # Full-width plus to half-width
        card_id.replace('＋', 'P2'),  # Full-width plus to P2
        card_id.replace('＋', 'P'),   # Full-width plus to P
        card_id.replace('R＋', 'R2'),  # R+ to R2
        card_id.replace('P＋', 'P2'),  # P+ to P2
    ]
    
    # 2. Insert missing patterns (like bp3 -> bp3-L-)
    if '-bp' in card_id and '-L-' not in card_id:
        match = re.search(r'(-bp\d+)-(\d+)-([A-Z])', card_id)
        if match:
            bp_part, num, suffix = match.groups()
            variants.append(card_id.replace(f'{bp_part}-{num}', f'{bp_part}-L-{num}'))
    
    # 3. Try all variants
    for variant in variants:
        variant_file = f"{variant}.webp"
        if variant_file in image_files:
            new_mappings[card_id] = f"img/cards_webp/{variant}"
            print(f"Found mapping: {card_id} -> {variant}")
            break
    else:
        missing_mappings.append(card_id)

print(f"\nFound {len(new_mappings)} new mappings")
print(f"Still missing: {len(missing_mappings)}")

if missing_mappings:
    print("\nStill missing mappings:")
    for card_id in missing_mappings[:20]:
        print(f"  {card_id}")
    if len(missing_mappings) > 20:
        print(f"  ... and {len(missing_mappings) - 20} more")

# Update the mapping file
if new_mappings:
    existing_mappings.update(new_mappings)
    with open('web_ui/js/card_image_mapping.json', 'w', encoding='utf-8') as f:
        json.dump(existing_mappings, f, ensure_ascii=False, indent=0)
    
    print(f"\nUpdated card_image_mapping.json with {len(new_mappings)} new mappings")
