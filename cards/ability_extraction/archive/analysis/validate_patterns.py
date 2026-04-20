"""Validate pattern assumptions against actual ability data."""
import json
import re
from collections import defaultdict

# Load abilities
with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']
texts = [a.get('triggerless_text', '') for a in abilities if a.get('triggerless_text')]

print("=== VALIDATING PATTERN ASSUMPTIONS ===\n")

# Check SOURCE_PATTERNS assumptions
print("=== SOURCE PATTERNS VALIDATION ===")
source_patterns = [
    '自分のエネルギーデッキから',
    '自分の控え室から',
    'エールにより公開された自分のカードの中から',
    '自分の控え室にある',
    '自分のエネルギー置き場にある',
    '相手の控え室から',
    '相手の控え室にある',
]

for pattern in source_patterns:
    matches = [t for t in texts if pattern in t]
    print(f"\nPattern: {pattern}")
    print(f"Occurrences: {len(matches)}")
    if matches:
        print("Sample contexts:")
        for i, text in enumerate(matches[:5]):
            # Get surrounding context
            idx = text.index(pattern)
            start = max(0, idx - 20)
            end = min(len(text), idx + len(pattern) + 20)
            print(f"  ...{text[start:end]}...")

# Check LOCATION_PATTERNS assumptions
print("\n\n=== LOCATION PATTERNS VALIDATION ===")
location_patterns = [
    '成功ライブカード置き場',
    'ライブカード置き場',
    '控え室',
    'エネルギー',
    '手札',
    'ステージ',
]

for pattern in location_patterns:
    matches = [t for t in texts if pattern in t]
    print(f"\nPattern: {pattern}")
    print(f"Occurrences: {len(matches)}")
    if matches:
        print("Sample contexts:")
        for i, text in enumerate(matches[:5]):
            idx = text.index(pattern)
            start = max(0, idx - 20)
            end = min(len(text), idx + len(pattern) + 20)
            print(f"  ...{text[start:end]}...")

# Check: "控え室から" vs "控え室に置く"
print("\n\n=== CRITICAL: FROM DISCARD vs TO DISCARD ===")
from_discard = [t for t in texts if '控え室から' in t]
to_discard = [t for t in texts if '控え室に置く' in t]

print(f"FROM discard (控え室から): {len(from_discard)}")
print("Samples:")
for text in from_discard[:5]:
    idx = text.index('控え室から')
    start = max(0, idx - 30)
    end = min(len(text), idx + 30)
    print(f"  ...{text[start:end]}...")

print(f"\nTO discard (控え室に置く): {len(to_discard)}")
print("Samples:")
for text in to_discard[:5]:
    idx = text.index('控え室に置く')
    start = max(0, idx - 30)
    end = min(len(text), idx + 30)
    print(f"  ...{text[start:end]}...")

# Check: "ウェイトにする" vs "控え室に置く"
print("\n\n=== CRITICAL: TO WAIT vs TO DISCARD ===")
to_wait = [t for t in texts if 'ウェイトにする' in t]
to_discard = [t for t in texts if '控え室に置く' in t]

print(f"TO wait (ウェイトにする): {len(to_wait)}")
print("Samples:")
for text in to_wait[:5]:
    idx = text.index('ウェイトにする')
    start = max(0, idx - 30)
    end = min(len(text), idx + 30)
    print(f"  ...{text[start:end]}...")

# Check: destination patterns
print("\n\n=== DESTINATION PATTERNS ===")
dest_patterns = {
    '手札に加える': 'hand',
    '手札に': 'hand',
    '控え室に置く': 'discard',
    'ウェイトにする': 'wait',
    'ステージに置く': 'stage',
    '登場させる': 'stage',
    'デッキの上に置く': 'deck_top',
}

for pattern, assumed_dest in dest_patterns.items():
    matches = [t for t in texts if pattern in t]
    print(f"\nPattern: {pattern} (assumed: {assumed_dest})")
    print(f"Occurrences: {len(matches)}")
    if matches:
        print("Samples:")
        for text in matches[:3]:
            idx = text.index(pattern)
            start = max(0, idx - 30)
            end = min(len(text), idx + 30)
            print(f"  ...{text[start:end]}...")

# Check: source patterns
print("\n\n=== SOURCE PATTERNS ===")
source_patterns = {
    'デッキから': 'deck',
    'デッキの上から': 'deck',
    '手札から': 'hand',
    'ステージから': 'stage',
    '控え室から': 'discard',
    'エネルギーデッキから': 'energy_deck',
}

for pattern, assumed_source in source_patterns.items():
    matches = [t for t in texts if pattern in t]
    print(f"\nPattern: {pattern} (assumed: {assumed_source})")
    print(f"Occurrences: {len(matches)}")
    if matches:
        print("Samples:")
        for text in matches[:3]:
            idx = text.index(pattern)
            start = max(0, idx - 30)
            end = min(len(text), idx + 30)
            print(f"  ...{text[start:end]}...")

# Check: multiple expressions for same concept
print("\n\n=== CODE → MULTIPLE JAPANESE ===")
print("\nConcept: DRAW (deck → hand)")
draw_patterns = [
    'カードを引く',
    'カードをN枚引く',
    '手札に加える',
    '手札にN枚加える',
]
for pattern in draw_patterns:
    count = sum(1 for t in texts if pattern in t)
    print(f"  {pattern}: {count}")

print("\nConcept: DISCARD (any → discard)")
discard_patterns = [
    '控え室に置く',
    'ウェイトにする',
]
for pattern in discard_patterns:
    count = sum(1 for t in texts if pattern in t)
    print(f"  {pattern}: {count}")

print("\nConcept: DEPLOY (any → stage)")
deploy_patterns = [
    '登場させる',
    'ステージに置く',
]
for pattern in deploy_patterns:
    count = sum(1 for t in texts if pattern in t)
    print(f"  {pattern}: {count}")

# Find contradictions
print("\n\n=== POTENTIAL CONTRADICTIONS ===")
# Check for abilities that have both "控え室から" and "控え室に置く"
both_discard = [t for t in texts if '控え室から' in t and '控え室に置く' in t]
print(f"Abilities with both FROM discard and TO discard: {len(both_discard)}")
if both_discard:
    print("Samples:")
    for text in both_discard[:3]:
        print(f"  {text[:100]}...")
