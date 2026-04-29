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

# Get all available image files in both repos
image_dir_our = 'web_ui/img/cards_webp'
image_dir_temp = 'C:/Users/trios/OneDrive/Documents/japanese resume/temp_rabuka/frontend/img/cards_webp'

our_files = set()
if os.path.exists(image_dir_our):
    for file in os.listdir(image_dir_our):
        if file.endswith('.webp'):
            our_files.add(file)

temp_files = set()
if os.path.exists(image_dir_temp):
    for file in os.listdir(image_dir_temp):
        if file.endswith('.webp'):
            temp_files.add(file)

# Read existing mappings
with open('web_ui/js/card_image_mapping.json', 'r', encoding='utf-8-sig') as f:
    existing_mappings = json.load(f)

# Find remaining missing mappings
still_missing = []
found_in_temp = []
need_different_transformations = []

for card_id in sorted(card_ids):
    expected_file = f"{card_id}.webp"
    
    if expected_file in our_files:
        continue  # Already exists
    
    if card_id in existing_mappings:
        continue  # Already mapped
    
    # Check if it exists in temp_rabuka
    if expected_file in temp_files:
        found_in_temp.append(card_id)
        continue
    
    # Try more complex transformations
    base_id = card_id
    
    # 1. Handle different bp patterns
    if '-bp' in base_id and '-L-' not in base_id and not base_id.endswith('-N'):
        # Try various bp transformations
        match = re.search(r'(-bp\d+)-(\d+)-([A-Z])', base_id)
        if match:
            bp_part, num, suffix = match.groups()
            
            # Try different patterns
            variants = [
                base_id.replace(f'{bp_part}-{num}', f'{bp_part}-L-{num}'),  # Add -L-
                base_id.replace(f'{bp_part}-{num}', f'{bp_part}-N-{num}'),  # Change to -N-
                base_id.replace(f'{bp_part}-{num}', f'{bp_part}-P-{num}'),  # Change to -P-
                base_id.replace(f'{bp_part}-{num}', f'{bp_part}-R-{num}'),  # Change to -R-
            ]
            
            for variant in variants:
                variant_file = f"{variant}.webp"
                if variant_file in our_files:
                    need_different_transformations.append((card_id, variant))
                    break
            else:
                still_missing.append(card_id)
        else:
            still_missing.append(card_id)
    else:
        still_missing.append(card_id)

print(f"Found in temp_rabuka but not our repo: {len(found_in_temp)}")
print(f"Need different transformations: {len(need_different_transformations)}")
print(f"Still completely missing: {len(still_missing)}")

if found_in_temp:
    print("\nFound in temp_rabuka:")
    for card_id in found_in_temp[:10]:
        print(f"  {card_id}")
    if len(found_in_temp) > 10:
        print(f"  ... and {len(found_in_temp) - 10} more")

if need_different_transformations:
    print("\nNeed different transformations:")
    for card_id, variant in need_different_transformations[:10]:
        print(f"  {card_id} -> {variant}")
    if len(need_different_transformations) > 10:
        print(f"  ... and {len(need_different_transformations) - 10} more")

if still_missing:
    print("\nStill completely missing:")
    for card_id in still_missing[:10]:
        print(f"  {card_id}")
    if len(still_missing) > 10:
        print(f"  ... and {len(still_missing) - 10} more")
