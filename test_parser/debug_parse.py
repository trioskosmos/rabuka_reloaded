#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import json
import sys
sys.path.insert(0, '.')
from parser_v2 import parse_ability_v2

# Load the test abilities
with open('test_abilities.json', encoding='utf-8') as f:
    data = json.load(f)

# Debug one specific card
test_card = None
for item in data:
    cards = item.get('cards', [])
    if cards and isinstance(cards[0], str) and '澁谷かのん' in cards[0]:
        if item.get('effect', {}).get('action') == 'unknown':
            test_card = item
            break

if test_card:
    text = test_card.get('triggerless_text', '')
    print(f"Card: {test_card['cards'][0] if test_card.get('cards') else 'Unknown'}")
    print(f"\nOriginal text:")
    print(f"  {text}")
    print(f"\nParsed result:")
    try:
        result = parse_ability_v2(text)
        print(f"  Blocks: {len(result.blocks)}")
        for i, block in enumerate(result.blocks):
            print(f"    Block {i}: effect.action={block.effect.action}, text={block.effect.text[:80]}")
    except Exception as e:
        print(f"  Error: {e}")
        import traceback
        traceback.print_exc()
