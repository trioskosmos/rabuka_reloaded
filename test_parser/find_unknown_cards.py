#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import json

# Load the test abilities
with open('test_abilities.json', encoding='utf-8') as f:
    data = json.load(f)

# Find cards with unknown effects
for item in data:
    triggerless = item.get('triggerless_text', '')
    effect = item.get('effect', {})
    
    if effect and effect.get('action') == 'unknown':
        print(f"\n{'='*80}")
        print(f"Card: {item.get('cards', ['Unknown'])[0]}")
        print(f"Full text: {triggerless[:100]}...")
        print(f"Unknown text: {effect.get('text', '')[:120]}...")
