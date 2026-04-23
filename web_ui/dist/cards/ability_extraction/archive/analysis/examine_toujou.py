"""Examine 登場させる patterns and baton pass relationship."""
import json
import re

# Load abilities
with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']
texts = [a.get('triggerless_text', '') for a in abilities if a.get('triggerless_text')]

print("=== EXAMINING 登場させる (DEPLOY) PATTERNS ===\n")

# Find all abilities with 登場させる
toujou_abilities = [t for t in texts if '登場させる' in t]
print(f"Total abilities with 登場させる: {len(toujou_abilities)}")

# Check for baton touch (バトンタッチ) relationship
baton_touch_with_toujou = [t for t in toujou_abilities if 'バトンタッチ' in t]
print(f"登場させる with バトンタッチ: {len(baton_touch_with_toujou)}")

# Check for without baton touch
toujou_without_baton = [t for t in toujou_abilities if 'バトンタッチ' not in t]
print(f"登場させる without バトンタッチ: {len(toujou_without_baton)}")

print("\n=== SAMPLES: 登場させる WITH バトンタッチ ===")
for text in baton_touch_with_toujou[:10]:
    print(f"- {text[:120]}...")

print("\n=== SAMPLES: 登場させる WITHOUT バトンタッチ ===")
for text in toujou_without_baton[:10]:
    print(f"- {text[:120]}...")

# Analyze sources for 登場させる
print("\n=== SOURCE PATTERNS FOR 登場させる ===")
sources = {
    '控え室から': 0,
    '手札から': 0,
    'デッキから': 0,
    'ステージから': 0,
}

for text in toujou_abilities:
    if '控え室から' in text:
        sources['控え室から'] += 1
    if '手札から' in text:
        sources['手札から'] += 1
    if 'デッキから' in text:
        sources['デッキから'] += 1
    if 'ステージから' in text:
        sources['ステージから'] += 1

for source, count in sources.items():
    print(f"{source}: {count}")

# Check for cost modifiers with 登場させる
print("\n=== COST MODIFIERS WITH 登場させる ===")
cost_modifiers = {
    'コスト': 0,
    'コスト減る': 0,
    'コスト増える': 0,
}

for text in toujou_abilities:
    if 'コスト' in text and '減る' in text:
        cost_modifiers['コスト減る'] += 1
    if 'コスト' in text and '増える' in text:
        cost_modifiers['コスト増える'] += 1
    if 'コスト' in text:
        cost_modifiers['コスト'] += 1

for modifier, count in cost_modifiers.items():
    print(f"{modifier}: {count}")

# Check for position requirements
print("\n=== POSITION REQUIREMENTS WITH 登場させる ===")
positions = {
    'センターエリア': 0,
    '左サイド': 0,
    '右サイド': 0,
    'メンバーのいないエリア': 0,
}

for text in toujou_abilities:
    if 'センターエリア' in text:
        positions['センターエリア'] += 1
    if '左サイド' in text:
        positions['左サイド'] += 1
    if '右サイド' in text:
        positions['右サイド'] += 1
    if 'メンバーのいないエリア' in text:
        positions['メンバーのいないエリア'] += 1

for position, count in positions.items():
    print(f"{position}: {count}")

# Analyze the structure more carefully
print("\n=== STRUCTURAL ANALYSIS ===")
for text in toujou_abilities[:5]:
    print(f"\nText: {text[:150]}...")
    
    # Extract source
    if '控え室から' in text:
        print(f"  Source: discard (控え室から)")
    elif '手札から' in text:
        print(f"  Source: hand (手札から)")
    elif 'デッキから' in text:
        print(f"  Source: deck (デッキから)")
    
    # Check for baton touch
    if 'バトンタッチ' in text:
        print(f"  Baton touch: YES")
    
    # Check for destination specifics
    if 'メンバーのいないエリア' in text:
        print(f"  Destination: empty area")
    elif 'そのメンバーがいたエリア' in text:
        print(f"  Destination: same area as previous member")
