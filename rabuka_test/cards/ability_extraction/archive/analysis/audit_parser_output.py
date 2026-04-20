"""Audit parser output to identify unparsed or poorly parsed abilities."""
import json

# Load abilities
with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

print("=== PARSER OUTPUT AUDIT ===\n")

# Count raw_text effects (unparsed)
raw_effects = 0
for a in abilities:
    effect = a.get('effect')
    if effect:
        if isinstance(effect, dict) and 'raw_text' in effect:
            raw_effects += 1
        elif isinstance(effect, dict) and 'actions' in effect:
            for action in effect['actions']:
                if isinstance(action, dict) and 'raw_text' in action:
                    raw_effects += 1

print(f"Unparsed effects (raw_text): {raw_effects}")

# Count costs with only text (no structured data)
text_only_costs = 0
for a in abilities:
    cost = a.get('cost')
    if cost and isinstance(cost, dict):
        if 'text' in cost and len(cost.keys()) == 1:
            text_only_costs += 1

print(f"Costs with only text (no structure): {text_only_costs}")

# Find abilities with complex patterns that might not be parsed
complex_patterns = {
    'newline': 0,
    'choice': 0,
    'sequential': 0,
    'compound': 0,
}

for a in abilities:
    text = a.get('triggerless_text', '')
    if '\n' in text:
        complex_patterns['newline'] += 1
    if '以下から1つを選ぶ' in text:
        complex_patterns['choice'] += 1
    if 'その後' in text:
        complex_patterns['sequential'] += 1
    if 'かつ' in text:
        complex_patterns['compound'] += 1

print("\nComplex pattern occurrences:")
for k, v in complex_patterns.items():
    print(f"  {k}: {v}")

# Find specific examples of unparsed complex abilities
print("\n=== UNPARSED COMPLEX ABILITIES ===\n")

unparsed_examples = []
for a in abilities:
    text = a.get('triggerless_text', '')
    effect = a.get('effect')
    
    # Check if complex pattern exists but effect is raw_text or missing structure
    is_complex = any(pattern in text for pattern in ['\n', '以下から1つを選ぶ', 'その後', 'かつ'])
    is_unparsed = (effect and isinstance(effect, dict) and 'raw_text' in effect) or not effect
    
    if is_complex and is_unparsed:
        unparsed_examples.append({
            'text': text,
            'effect': effect
        })

print(f"Complex abilities that are unparsed: {len(unparsed_examples)}")
for i, ex in enumerate(unparsed_examples[:5]):
    print(f"\n{i+1}. {ex['text'][:100]}...")
    print(f"   Effect: {ex['effect']}")

# Check for choice parsing
print("\n=== CHOICE EFFECTS ===\n")
choice_effects = 0
for a in abilities:
    effect = a.get('effect')
    if effect and isinstance(effect, dict):
        if effect.get('action') == 'choice':
            choice_effects += 1
            print(f"Choice effect found: {a.get('triggerless_text', '')[:80]}...")

print(f"\nTotal choice effects parsed: {choice_effects}")

# Check for sequential effects
print("\n=== SEQUENTIAL EFFECTS ===\n")
sequential_effects = 0
for a in abilities:
    effect = a.get('effect')
    if effect and isinstance(effect, dict):
        if 'actions' in effect and len(effect.get('actions', [])) > 1:
            sequential_effects += 1
            print(f"Sequential effect found: {a.get('triggerless_text', '')[:80]}...")

print(f"\nTotal sequential effects parsed: {sequential_effects}")

# Check for compound conditions
print("\n=== COMPOUND CONDITIONS ===\n")
compound_conditions = 0
for a in abilities:
    effect = a.get('effect')
    if effect and isinstance(effect, dict):
        def check_compound(node):
            if isinstance(node, dict):
                if node.get('type') == 'compound':
                    return True
                if 'condition' in node and isinstance(node['condition'], dict):
                    if node['condition'].get('type') == 'compound':
                        return True
                if 'actions' in node:
                    for action in node['actions']:
                        if check_compound(action):
                            return True
            return False
        
        if check_compound(effect):
            compound_conditions += 1
            print(f"Compound condition found: {a.get('triggerless_text', '')[:80]}...")

print(f"\nTotal compound conditions parsed: {compound_conditions}")

# Summary
print("\n=== SUMMARY ===\n")
print(f"Total abilities: {len(abilities)}")
print(f"Unparsed effects: {raw_effects}")
print(f"Text-only costs: {text_only_costs}")
print(f"Complex abilities unparsed: {len(unparsed_examples)}")
print(f"Choice effects parsed: {choice_effects}")
print(f"Sequential effects parsed: {sequential_effects}")
print(f"Compound conditions parsed: {compound_conditions}")
