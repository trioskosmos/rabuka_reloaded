"""Analyze all 609 abilities to extract structural patterns and rules."""
import json
import re
from collections import defaultdict, Counter

# Load abilities
with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

print(f"Analyzing {len(abilities)} abilities...\n")

# Extract all triggerless texts
texts = [a.get('triggerless_text', '') for a in abilities if a.get('triggerless_text')]

# Pattern extraction
patterns = {
    'cost_effect_separator': 0,
    'sequential_comma': 0,
    'sequential_sono_go': 0,
    'choice': 0,
    'compound_katsu': 0,
    'duration_kagiri': 0,
    'duration_live_end': 0,
    'per_unit_tsuki': 0,
    'conditional_bai': 0,
    'conditional_toki': 0,
    'optional_may': 0,
    'bullet_points': 0,
    'parenthetical_notes': 0,
    'newlines': 0,
}

for text in texts:
    if '：' in text:
        patterns['cost_effect_separator'] += 1
    if '、' in text and '引く' in text:
        patterns['sequential_comma'] += 1
    if 'その後' in text:
        patterns['sequential_sono_go'] += 1
    if '以下から1つを選ぶ' in text:
        patterns['choice'] += 1
    if 'かつ' in text:
        patterns['compound_katsu'] += 1
    if 'かぎり' in text:
        patterns['duration_kagiri'] += 1
    if 'ライブ終了時まで' in text:
        patterns['duration_live_end'] += 1
    if 'につき' in text:
        patterns['per_unit_tsuki'] += 1
    if '場合' in text:
        patterns['conditional_bai'] += 1
    if 'とき' in text:
        patterns['conditional_toki'] += 1
    if 'もよい' in text or 'てもよい' in text:
        patterns['optional_may'] += 1
    if '・' in text:
        patterns['bullet_points'] += 1
    if '（' in text or '(' in text:
        patterns['parenthetical_notes'] += 1
    if '\n' in text:
        patterns['newlines'] += 1

print("=== PATTERN FREQUENCIES ===")
for k, v in sorted(patterns.items(), key=lambda x: -x[1]):
    print(f"{k}: {v}")

# Extract action verbs
action_verbs = []
for text in texts:
    verbs = re.findall(r'(引く|置く|選ぶ|発動させる|得る|加える|登場させる|ウェイトにする|アクティブにする|見る|公開する|戻す|リフレッシュ)', text)
    action_verbs.extend(verbs)

verb_counts = Counter(action_verbs)
print("\n=== ACTION VERBS ===")
for k, v in verb_counts.most_common():
    print(f"{k}: {v}")

# Extract locations
locations = []
for text in texts:
    locs = re.findall(r'(控え室|手札|ステージ|デッキ|エネルギーデッキ|エネルギー置き場|ライブカード置き場|成功ライブカード置き場)', text)
    locations.extend(locs)

location_counts = Counter(locations)
print("\n=== LOCATIONS ===")
for k, v in location_counts.most_common():
    print(f"{k}: {v}")

# Extract card types
card_types = []
for text in texts:
    types = re.findall(r'(メンバーカード|ライブカード|エネルギーカード|ドルチェストラ)', text)
    card_types.extend(types)

card_type_counts = Counter(card_types)
print("\n=== CARD TYPES ===")
for k, v in card_type_counts.most_common():
    print(f"{k}: {v}")

# Extract operators
operators = []
for text in texts:
    ops = re.findall(r'(以上|以下|より少ない|より多い|未満|超)', text)
    operators.extend(ops)

operator_counts = Counter(operators)
print("\n=== OPERATORS ===")
for k, v in operator_counts.most_common():
    print(f"{k}: {v}")

# Extract properties
properties = []
for text in texts:
    props = re.findall(r'(コスト|スコア|ブレード|ハート)', text)
    properties.extend(props)

property_counts = Counter(properties)
print("\n=== PROPERTIES ===")
for k, v in property_counts.most_common():
    print(f"{k}: {v}")

# Extract groups
groups = []
for text in texts:
    grps = re.findall(r'『([^』]+)』', text)
    groups.extend(grps)

group_counts = Counter(groups)
print("\n=== TOP GROUPS ===")
for k, v in group_counts.most_common(20):
    print(f"{k}: {v}")

# Extract numbers and their contexts
number_contexts = []
for text in texts:
    matches = re.finditer(r'(\d+)(?:枚|人)', text)
    for m in matches:
        # Get surrounding context
        start = max(0, m.start() - 10)
        end = min(len(text), m.end() + 10)
        context = text[start:end]
        number_contexts.append((m.group(1), context))

print("\n=== NUMBER CONTEXTS (sample) ===")
for num, ctx in number_contexts[:20]:
    print(f"{num}: ...{ctx}...")

# Identify unique structural templates
templates = []
for text in texts[:100]:  # Sample first 100
    # Normalize to identify structure
    template = text
    template = re.sub(r'\d+', 'N', template)
    template = re.sub(r'『[^』]+』', 'GROUP', template)
    template = re.sub(r'\{\{[^}]+\}\}', 'ICON', template)
    template = re.sub(r'「[^」]+」', 'QUOTE', template)
    templates.append(template)

template_counts = Counter(templates)
print("\n=== UNIQUE STRUCTURAL TEMPLATES (top 20) ===")
for k, v in template_counts.most_common(20):
    print(f"Count {v}: {k[:100]}...")

# Find complex abilities (multiple patterns)
complex_abilities = []
for i, text in enumerate(texts):
    pattern_count = 0
    if 'かつ' in text: pattern_count += 1
    if 'その後' in text: pattern_count += 1
    if '以下から1つを選ぶ' in text: pattern_count += 1
    if 'かぎり' in text: pattern_count += 1
    if '\n' in text: pattern_count += 1
    if pattern_count >= 2:
        complex_abilities.append((i, text, pattern_count))

complex_abilities.sort(key=lambda x: x[2], reverse=True)
print(f"\n=== COMPLEX ABILITIES (multiple patterns) ===")
for i, text, count in complex_abilities[:20]:
    print(f"Complexity {count}: {text[:80]}...")
