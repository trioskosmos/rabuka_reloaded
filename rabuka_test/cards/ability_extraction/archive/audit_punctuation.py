"""Audit punctuation extraction results."""
import json

# Load abilities
with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

print("=== PUNCTUATION EXTRACTION AUDIT ===\n")

# Count group_names extraction
with_group_names = [a for a in abilities if (a.get('effect') and a.get('effect', {}).get('condition', {}).get('group_names')) or (a.get('cost') and a.get('cost', {}).get('group_names'))]
print(f"Abilities with group_names extracted: {len(with_group_names)}")
if with_group_names:
    print("Sample:")
    for i, a in enumerate(with_group_names[:5], 1):
        cond_group = a.get('effect', {}).get('condition', {}).get('group_names') if a.get('effect') else None
        cost_group = a.get('cost', {}).get('group_names') if a.get('cost') else None
        print(f"  {i}. Condition: {cond_group}, Cost: {cost_group}")

# Count quoted_text extraction
with_quoted_text = [a for a in abilities if (a.get('effect') and a.get('effect', {}).get('quoted_text')) or (a.get('cost') and a.get('cost', {}).get('quoted_text'))]
print(f"\nAbilities with quoted_text extracted: {len(with_quoted_text)}")
if with_quoted_text:
    print("Sample:")
    for i, a in enumerate(with_quoted_text[:5], 1):
        effect_quote = a.get('effect', {}).get('quoted_text') if a.get('effect') else None
        cost_quote = a.get('cost', {}).get('quoted_text') if a.get('cost') else None
        print(f"  {i}. Effect: {effect_quote}, Cost: {cost_quote}")

# Count parenthetical extraction
with_parenthetical = [a for a in abilities if a.get('effect') and a.get('effect', {}).get('parenthetical')]
print(f"\nAbilities with parenthetical extracted: {len(with_parenthetical)}")
if with_parenthetical:
    print("Sample:")
    for i, a in enumerate(with_parenthetical[:5], 1):
        paren = a.get('effect', {}).get('parenthetical')
        print(f"  {i}. {paren}")

# Count except_quoted
with_except_quoted = [a for a in abilities if a.get('effect') and a.get('effect', {}).get('condition', {}).get('except_quoted')]
print(f"\nAbilities with except_quoted: {len(with_except_quoted)}")
if with_except_quoted:
    print("Sample:")
    for i, a in enumerate(with_except_quoted[:5], 1):
        except_q = a.get('effect', {}).get('condition', {}).get('except_quoted')
        print(f"  {i}. {except_q}")

print("\n=== SUMMARY ===")
print(f"group_names extracted: {len(with_group_names)}")
print(f"quoted_text extracted: {len(with_quoted_text)}")
print(f"parenthetical extracted: {len(with_parenthetical)}")
print(f"except_quoted extracted: {len(with_except_quoted)}")
