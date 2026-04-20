"""Audit all abilities to identify patterns, punctuation, and variations."""
import json
from collections import Counter
import re

# Load abilities
with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']
texts = [a.get('triggerless_text', '') for a in abilities]

# Basic stats
print(f"Total abilities: {len(abilities)}")
print(f"Max length: {max(len(t) for t in texts)}")
print(f"Min length: {min(len(t) for t in texts)}")
print(f"Avg length: {sum(len(t) for t in texts) / len(texts):.1f}")

# Long abilities
long_texts = [t for t in texts if len(t) > 150]
print(f"\nLong abilities (>150 chars): {len(long_texts)}")
print("Sample long texts:")
for t in sorted(long_texts, key=len, reverse=True)[:5]:
    print(f"- {t[:100]}...")

# Punctuation
punct_chars = ['：', '。', '、', '？', '！', '「', '」', '『', '』', '（', '）', '\n']
punct_counts = {p: sum(t.count(p) for t in texts) for p in punct_chars}
print("\nPunctuation usage:")
for k, v in sorted(punct_counts.items(), key=lambda x: -x[1]):
    print(f"  {k}: {v}")

# Triggers
triggers = Counter()
for a in abilities:
    trig = a.get('triggers')
    if trig:
        if isinstance(trig, str):
            triggers[trig] += 1
        elif isinstance(trig, list):
            for t in trig:
                triggers[t] += 1

print("\nTrigger types:")
for k, v in triggers.most_common():
    print(f"  {k}: {v}")

# Cost types
cost_types = Counter()
for a in abilities:
    cost = a.get('cost')
    if cost and isinstance(cost, dict):
        ctype = cost.get('type', 'none')
        cost_types[ctype] += 1

print("\nCost types:")
for k, v in cost_types.most_common():
    print(f"  {k}: {v}")

# Effect action types
action_types = Counter()
for a in abilities:
    effect = a.get('effect')
    if effect and isinstance(effect, dict):
        action = effect.get('action')
        if action:
            action_types[action] += 1

print("\nEffect action types:")
for k, v in action_types.most_common():
    print(f"  {k}: {v}")

# Keywords analysis
keywords = ['場合', 'とき', 'かぎり', 'その後', 'また', 'または', 'かつ', '以下から1つを選ぶ', '好きな']
keyword_counts = {k: sum(1 for t in texts if k in t) for k in keywords}
print("\nKeywords:")
for k, v in sorted(keyword_counts.items(), key=lambda x: -x[1]):
    print(f"  {k}: {v}")

# Resource types
resources = ['ハート', 'ブレード', 'スコア', 'エネルギー', 'コスト']
resource_counts = {r: sum(1 for t in texts if r in t) for r in resources}
print("\nResource mentions:")
for k, v in sorted(resource_counts.items(), key=lambda x: -x[1]):
    print(f"  {k}: {v}")

# Card movement keywords
movement = ['引く', '手札に加える', '控え室に置く', 'ウェイトにする', '登場させる', 'ステージに置く', 'デッキの上に置く']
movement_counts = {m: sum(1 for t in texts if m in t) for m in movement}
print("\nCard movement keywords:")
for k, v in sorted(movement_counts.items(), key=lambda x: -x[1]):
    print(f"  {k}: {v}")

# Complex patterns
complex_patterns = {
    'newline': '\n',
    'choice': '以下から1つを選ぶ',
    'conditional': '場合',
    'timing': 'とき',
    'sequential': 'その後',
    'compound': 'かつ',
    'alternative': 'または',
}
complex_counts = {k: sum(1 for t in texts if v in t) for k, v in complex_patterns.items()}
print("\nComplex patterns:")
for k, v in sorted(complex_counts.items(), key=lambda x: -x[1]):
    print(f"  {k}: {v}")
