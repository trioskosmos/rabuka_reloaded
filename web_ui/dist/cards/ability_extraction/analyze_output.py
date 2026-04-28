#!/usr/bin/env python3
"""Analyze the abilities.json output."""
import json
from collections import Counter

with open('C:/Users/trios/OneDrive/Documents/rabuka_reloaded/cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

print(f"Total abilities: {len(data['unique_abilities'])}")
print(f"Total cards with abilities: {data['statistics']['cards_with_abilities']}")
print()

# Count action types
actions = Counter()
for ability in data['unique_abilities']:
    effect = ability.get('effect', {})
    if effect:
        action = effect.get('action', 'null')
        actions[action] += 1

print("Action distribution (top 20):")
for action, count in actions.most_common(20):
    print(f"  {action}: {count}")

# Count trigger types
print("\nTrigger distribution:")
triggers = Counter()
for ability in data['unique_abilities']:
    trigger = ability.get('triggers', 'null')
    triggers[trigger] += 1

for trigger, count in triggers.most_common(10):
    print(f"  {trigger}: {count}")

# Count null abilities
null_count = sum(1 for a in data['unique_abilities'] if a.get('is_null'))
print(f"\nNull abilities (notes): {null_count}")

# Count cost presence
with_cost = sum(1 for a in data['unique_abilities'] if a.get('cost'))
without_cost = len(data['unique_abilities']) - with_cost - null_count
print(f"Abilities with cost: {with_cost}")
print(f"Abilities without cost: {without_cost}")

print("\nAnalysis complete!")
