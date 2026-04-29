import json
import os
import re

# Read existing mappings
with open('web_ui/js/card_image_mapping.json', 'r', encoding='utf-8-sig') as f:
    existing_mappings = json.load(f)

# Get all available image files
image_dir = 'web_ui/img/cards_webp'
image_files = set()
if os.path.exists(image_dir):
    for file in os.listdir(image_dir):
        if file.endswith('.webp'):
            image_files.add(file)

# Handle the remaining 133 cases with more complex transformations
remaining_mappings = []

# Special character and suffix transformations
special_cases = [
    # Full-width plus to P2
    "PL!-bp3-004-P＋", "PL!-bp3-004-R＋", "PL!-bp3-008-P＋", "PL!-bp3-008-R＋",
    "PL!-bp3-009-P＋", "PL!-bp3-009-R＋", "PL!N-bp3-001-P＋", "PL!N-bp3-005-P＋",
    "PL!N-bp3-005-R＋", "PL!S-bp2-005-P＋", "PL!S-bp2-005-R＋", "PL!S-bp2-007-P＋",
    "PL!S-bp2-007-R＋", "PL!S-bp2-008-P＋", "PL!S-bp2-008-R＋",
    
    # SEC suffix cases (try different patterns)
    "PL!-bp3-004-SEC", "PL!-bp3-008-SEC", "PL!-bp3-009-SEC", "PL!N-bp3-001-SEC",
    "PL!N-bp3-005-SEC", "PL!N-bp3-006-SEC", "PL!N-bp3-007-SEC", "PL!N-bp3-008-SEC",
    "PL!N-bp3-009-SEC", "PL!N-bp3-010-SEC", "PL!N-bp3-011-SEC", "PL!N-bp3-012-SEC",
    "PL!N-bp3-013-SEC", "PL!N-bp3-014-SEC", "PL!N-bp3-015-SEC", "PL!N-bp3-016-SEC",
    "PL!N-bp3-017-SEC", "PL!N-bp3-018-SEC", "PL!N-bp3-019-SEC", "PL!N-bp3-020-SEC",
    "PL!N-bp3-021-SEC", "PL!N-bp3-022-SEC", "PL!N-bp3-023-SEC", "PL!N-bp3-024-SEC",
    "PL!N-bp3-025-SEC", "PL!N-bp3-026-SEC", "PL!N-bp3-027-SEC", "PL!N-bp3-028-SEC",
    "PL!N-bp3-029-SEC", "PL!N-bp3-030-SEC", "PL!N-bp3-031-SEC",
    
    # R+ to R2
    "LL-bp1-001-R＋", "LL-bp2-001-R＋", "LL-bp3-001-R＋", "LL-bp4-001-R＋",
    "PL!-bp4-002-R＋", "PL!-bp4-005-R＋", "PL!-bp5-001-R＋", "PL!-bp5-002-R＋",
    "PL!-bp5-005-R＋", "PL!N-bp1-002-R＋", "PL!N-bp1-003-R＋", "PL!N-bp1-006-R＋",
    "PL!N-bp1-012-R＋", "PL!N-bp3-001-R＋", "PL!N-bp4-004-R＋", "PL!N-bp4-007-R＋",
    "PL!N-bp4-010-R＋", "PL!N-bp4-011-R＋", "PL!N-bp5-001-R＋", "PL!N-bp5-005-R＋",
    "PL!N-bp5-007-R＋", "PL!N-bp5-012-R＋", "PL!S-bp5-001-R＋", "PL!S-bp5-002-R＋",
    "PL!S-bp5-005-R＋",
    
    # P+ to P2
    "PL!-bp4-002-P＋", "PL!-bp4-005-P＋", "PL!N-bp1-002-P＋", "PL!N-bp1-003-P＋",
    "PL!N-bp1-006-P＋", "PL!N-bp1-012-P＋", "PL!N-bp3-001-P＋", "PL!N-bp3-005-P＋",
    "PL!N-bp4-004-P＋", "PL!N-bp4-007-P＋", "PL!N-bp4-010-P＋", "PL!N-bp4-011-P＋",
    "PL!S-bp2-005-P＋", "PL!S-bp2-007-P＋", "PL!S-bp2-008-P＋",
]

# Try to find mappings for special cases
new_mappings = {}
for card_id in special_cases:
    if card_id in existing_mappings:
        continue
    
    # Try transformations
    variants = []
    
    # Full-width plus to P2
    if '＋' in card_id:
        variants.append(card_id.replace('＋', 'P2'))
        variants.append(card_id.replace('＋', '+'))
    
    # R+ to R2
    if 'R＋' in card_id:
        variants.append(card_id.replace('R＋', 'R2'))
    
    # P+ to P2
    if 'P＋' in card_id:
        variants.append(card_id.replace('P＋', 'P2'))
    
    # SEC suffix - try different endings
    if card_id.endswith('-SEC'):
        base = card_id[:-4]
        for suffix in ['-R', '-P', '-PR', '-SD', '-LLE']:
            variants.append(base + suffix)
    
    # Try each variant
    for variant in variants:
        variant_file = f"{variant}.webp"
        if variant_file in image_files:
            new_mappings[card_id] = f"img/cards_webp/{variant}"
            print(f"Found: {card_id} -> {variant}")
            break

print(f"\nFound {len(new_mappings)} additional mappings")

# Update the mapping file
if new_mappings:
    existing_mappings.update(new_mappings)
    with open('web_ui/js/card_image_mapping.json', 'w', encoding='utf-8') as f:
        json.dump(existing_mappings, f, ensure_ascii=False, indent=0)
    
    print(f"Updated card_image_mapping.json with {len(new_mappings)} more mappings")
