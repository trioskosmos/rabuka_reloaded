#!/usr/bin/env python3
"""Analyze custom actions in abilities.json to find patterns for conversion."""
import json
from pathlib import Path
from collections import Counter

abilities_path = Path(__file__).parent.parent / 'abilities.json'
with open(abilities_path, 'r', encoding='utf-8') as f:
    data = json.load(f)

customs = [a for a in data['unique_abilities'] if a.get('effect', {}).get('action') == 'custom' and a.get('effect', {}).get('text')]

print(f"Total custom actions with text: {len(customs)}\n")

# Group by common patterns
patterns = {}
for ability in customs:
    text = ability['effect']['text']
    # Categorize by key phrases
    if '指定する' in text:
        pattern = 'choose/specify'
    elif '選ぶ' in text or '選択' in text:
        pattern = 'choose/select'
    elif 'なる' in text and ('スコア' in text or 'コスト' in text or 'ハート' in text):
        pattern = 'become (state change)'
    elif '置き換える' in text or '入れ替える' in text:
        pattern = 'replace/swap'
    elif '得る' in text:
        pattern = 'gain (resource/ability)'
    elif '見る' in text:
        pattern = 'look_at'
    elif '移動させる' in text or '移動する' in text:
        pattern = 'move'
    elif '無効にする' in text:
        pattern = 'invalidate'
    elif '追加する' in text:
        pattern = 'add'
    elif '減らす' in text or '減る' in text:
        pattern = 'decrease'
    elif '増やす' in text or '増える' in text:
        pattern = 'increase'
    elif '枚数' in text:
        pattern = 'count modification'
    else:
        pattern = 'other'
    
    if pattern not in patterns:
        patterns[pattern] = []
    patterns[pattern].append(text)

print("Pattern distribution:")
for pattern, texts in patterns.items():
    print(f"  {pattern}: {len(texts)}")

print("\n" + "="*80)
print("Full texts by pattern:")
print("="*80)
for pattern, texts in patterns.items():
    print(f"\n{pattern} ({len(texts)} examples):")
    print("-" * 80)
    for i, text in enumerate(texts, 1):
        print(f"  {i}. {text}")
        print()
