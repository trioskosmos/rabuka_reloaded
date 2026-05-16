#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import json

# Load test abilities
with open('test_abilities.json', encoding='utf-8') as f:
    data = json.load(f)

# Extract all remaining unknown texts
unknowns_list = []
for item in data:
    effect = item.get('effect', {})
    if effect and effect.get('action') == 'unknown':
        text = effect.get('text', '')
        if text not in unknowns_list:
            unknowns_list.append(text)

print(f"Total unique unknowns: {len(unknowns_list)}\n")

# Sort by length to see patterns
unknowns_sorted = sorted(unknowns_list, key=len)

for i, text in enumerate(unknowns_sorted[-10:], 1):  # Show last 10 (longest)
    print(f"{i}. ({len(text)} chars): {text[:100]}...")
