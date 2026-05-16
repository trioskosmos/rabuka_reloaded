#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import json

# Load the test abilities
with open('test_abilities.json', encoding='utf-8') as f:
    data = json.load(f)

# Find unknowns
unknowns = {}
for item in data:
    effect = item.get('effect', {})
    if effect and effect.get('action') == 'unknown':
        key = effect.get('text', '')
        if key not in unknowns:
            unknowns[key] = 0
        unknowns[key] += 1

# Group by pattern
grouped = {
    'position_change': [],
    'icons_gain': [],
    'restrictions': [],
    'energy_cost_sequential': [],
    'complex_conditional': [],
    'other': [],
}

for text, count in unknowns.items():
    if 'バトン' in text or 'ポジション' in text:
        grouped['position_change'].append(text)
    elif '{{' in text and 'を得る' in text:
        grouped['icons_gain'].append(text)
    elif 'アクティブにしない' in text or 'ライブできない' in text or '置くことができない' in text:
        grouped['restrictions'].append(text)
    elif '{{icon_energy.png|E}}' in text and '：' in text:
        grouped['energy_cost_sequential'].append(text)
    elif '場合' in text or 'とき' in text:
        grouped['complex_conditional'].append(text)
    else:
        grouped['other'].append(text)

for category, items in grouped.items():
    if items:
        print(f"\n{category.upper()} ({len(items)} items):")
        for item in items[:3]:
            print(f"  {item[:100]}...")
