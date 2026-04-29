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

# The 50 transformations we found
transformations = [
    ("PL!-bp3-001-P", "PL!-bp3-P-001-P"),
    ("PL!-bp3-001-R", "PL!-bp3-R-001-R"),
    ("PL!-bp3-002-P", "PL!-bp3-P-002-P"),
    ("PL!-bp3-002-R", "PL!-bp3-R-002-R"),
    ("PL!-bp3-003-P", "PL!-bp3-P-003-P"),
    ("PL!-bp3-003-R", "PL!-bp3-R-003-R"),
    ("PL!-bp3-004-P", "PL!-bp3-P-004-P"),
    ("PL!-bp3-005-P", "PL!-bp3-P-005-P"),
    ("PL!-bp3-005-R", "PL!-bp3-R-005-R"),
    ("PL!-bp3-006-P", "PL!-bp3-P-006-P"),
    ("PL!-bp3-006-R", "PL!-bp3-R-006-R"),
    ("PL!-bp3-007-P", "PL!-bp3-P-007-P"),
    ("PL!-bp3-007-R", "PL!-bp3-R-007-R"),
    ("PL!-bp3-008-P", "PL!-bp3-P-008-P"),
    ("PL!-bp3-008-R", "PL!-bp3-R-008-R"),
    ("PL!-bp3-009-P", "PL!-bp3-P-009-P"),
    ("PL!-bp3-009-R", "PL!-bp3-R-009-R"),
    ("PL!-bp3-010-N", "PL!-bp3-N-010-N"),
    ("PL!-bp3-011-N", "PL!-bp3-N-011-N"),
    ("PL!-bp3-012-N", "PL!-bp3-N-012-N"),
    ("PL!-bp3-013-N", "PL!-bp3-N-013-N"),
    ("PL!-bp3-014-N", "PL!-bp3-N-014-N"),
    ("PL!-bp3-014-PR", "PL!-bp3-PR-014-PR"),
    ("PL!-bp3-017-N", "PL!-bp3-N-017-N"),
    ("PL!-bp3-018-N", "PL!-bp3-N-018-N"),
    ("PL!-bp3-019-L", "PL!-bp3-L-019-L"),
    ("PL!-bp3-020-L", "PL!-bp3-L-020-L"),
    ("PL!-bp3-021-L", "PL!-bp3-L-021-L"),
    ("PL!-bp3-022-L", "PL!-bp3-L-022-L"),
    ("PL!-bp3-023-L", "PL!-bp3-L-023-L"),
    ("PL!-bp3-024-L", "PL!-bp3-L-024-L"),
    ("PL!-bp3-025-L", "PL!-bp3-L-025-L"),
    ("PL!-bp3-026-L", "PL!-bp3-L-026-L"),
    ("LL-bp1-001-R＋", "LL-bp1-001-R2"),
    ("LL-bp2-001-R＋", "LL-bp2-001-R2"),
    ("LL-bp3-001-R＋", "LL-bp3-001-R2"),
    ("LL-bp4-001-R＋", "LL-bp4-001-R2"),
    ("PL!-bp4-002-P＋", "PL!-bp4-002-P2"),
    ("PL!-bp4-002-R＋", "PL!-bp4-002-R2"),
    ("PL!-bp4-005-P＋", "PL!-bp4-005-P2"),
    ("PL!-bp4-005-R＋", "PL!-bp4-005-R2"),
    ("PL!-bp5-001-R＋", "PL!-bp5-001-R2"),
    ("PL!-bp5-002-R＋", "PL!-bp5-002-R2"),
    ("PL!-bp5-005-R＋", "PL!-bp5-005-R2"),
    ("PL!S-bp3-016-N", "PL!S-bp3-N-016-N"),
    ("PL!S-bp3-019-L", "PL!S-bp3-L-019-L"),
    ("PL!S-bp3-020-L", "PL!S-bp3-L-020-L"),
    ("PL!S-bp3-021-L", "PL!S-bp3-L-021-L"),
    ("PL!S-bp3-022-L", "PL!S-bp3-L-022-L"),
    ("PL!S-bp3-023-L", "PL!S-bp3-L-023-L"),
    ("PL!S-bp3-024-L", "PL!S-bp3-L-024-L"),
    ("PL!S-bp3-025-L", "PL!S-bp3-L-025-L"),
    ("PL!S-bp3-026-L", "PL!S-bp3-L-026-L"),
]

# Add the transformations
new_mappings = {}
for card_id, variant in transformations:
    variant_file = f"{variant}.webp"
    if variant_file in image_files and card_id not in existing_mappings:
        new_mappings[card_id] = f"img/cards_webp/{variant}"
        print(f"Added: {card_id} -> {variant}")

print(f"\nAdded {len(new_mappings)} more mappings")

# Update the mapping file
if new_mappings:
    existing_mappings.update(new_mappings)
    with open('web_ui/js/card_image_mapping.json', 'w', encoding='utf-8') as f:
        json.dump(existing_mappings, f, ensure_ascii=False, indent=0)
    
    print(f"Updated card_image_mapping.json with {len(new_mappings)} additional mappings")
