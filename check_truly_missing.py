import json

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

# Check the specific examples I found
# lose_blade_hearts at unique_abilities[213].effect
print("=== lose_blade_hearts ===")
print(json.dumps(data['unique_abilities'][213]['effect'], indent=2, ensure_ascii=False))

# conditional at unique_abilities[49].effect
print("\n=== conditional ===")
print(json.dumps(data['unique_abilities'][49]['effect'], indent=2, ensure_ascii=False))

# choice_type at unique_abilities[360].effect
print("\n=== choice_type ===")
print(json.dumps(data['unique_abilities'][360]['effect'], indent=2, ensure_ascii=False))

# heart_type at unique_abilities[565].effect
print("\n=== heart_type ===")
print(json.dumps(data['unique_abilities'][565]['effect'], indent=2, ensure_ascii=False))

# values at unique_abilities[23].effect.condition
print("\n=== values ===")
print(json.dumps(data['unique_abilities'][23]['effect']['condition'], indent=2, ensure_ascii=False))
