#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import json
import sys

# Load the test abilities
with open('test_abilities.json', encoding='utf-8') as f:
    data = json.load(f)

unknowns = {}
for item in data:
    triggerless = item.get('triggerless_text', '')
    effect = item.get('effect', {})
    
    if effect and effect.get('action') == 'unknown':
        key = effect.get('text', '')
        if key not in unknowns:
            unknowns[key] = 0
        unknowns[key] += 1

# Sort by frequency
sorted_unknowns = sorted(unknowns.items(), key=lambda x: x[1], reverse=True)
print(f'Total unknown actions: {len(sorted_unknowns)}')
print(f'Total occurrences: {sum(v for k, v in sorted_unknowns)}')
print()
for text, count in sorted_unknowns[:30]:
    print(f'{count:3d}x: {text}')
