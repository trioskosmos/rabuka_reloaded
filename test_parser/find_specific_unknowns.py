#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import json
import sys
sys.path.insert(0, '.')
from parser_v2 import parse_ability_v2

# Load the test abilities
with open('test_abilities.json', encoding='utf-8') as f:
    data = json.load(f)

# Find energy cost sequential cards
target_texts = [
    '{{icon_energy.png|E}}支払ってもよい',
    '{{icon_energy.png|E}}：カードを1枚引き',
    'このメンバーは自分のアクティブフェイズに',
]

for target in target_texts:
    print(f"\n{'='*80}")
    print(f"Looking for: {target[:50]}")
    for item in data:
        triggerless = item.get('triggerless_text', '')
        effect = item.get('effect', {})
        
        if effect and effect.get('action') == 'unknown' and target in effect.get('text', ''):
            print(f"\nCard: {item['cards'][0] if item.get('cards') else 'Unknown'}")
            print(f"Full text: {triggerless}")
            print(f"Unknown text: {effect.get('text', '')[:120]}")
            break
