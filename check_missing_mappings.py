import json
import os
import re

# Read abilities.json to get all card IDs
with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    abilities = json.load(f)

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

# Read current mappings
with open('web_ui/js/card_image_mapping.json', 'r', encoding='utf-8-sig') as f:
    current_mappings = json.load(f)

# Find missing mappings
missing_mappings = []
for card_id in sorted(card_ids):
    expected_file = f"{card_id}.webp"
    
    # Check if direct file exists
    if expected_file in image_files:
        continue
    
    # Check if mapping exists
    if card_id in current_mappings:
        continue
    
    missing_mappings.append(card_id)

print(f"Total card IDs found: {len(card_ids)}")
print(f"Total image files found: {len(image_files)}")
print(f"Missing mappings: {len(missing_mappings)}")
print("\nMissing card IDs:")
for card_id in missing_mappings[:50]:  # Show first 50
    print(f"  {card_id}")

if len(missing_mappings) > 50:
    print(f"  ... and {len(missing_mappings) - 50} more")
