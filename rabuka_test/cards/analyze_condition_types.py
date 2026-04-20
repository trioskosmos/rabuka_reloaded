import json
from collections import Counter

# Load abilities.json
with open('abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

# Collect all condition types with samples
condition_types = []
condition_samples = {}

# Handle nested conditions in compound conditions
def extract_conditions(obj, full_text=None):
    if isinstance(obj, dict):
        if 'type' in obj:
            cond_type = obj['type']
            condition_types.append(cond_type)
            # Store sample for rare types
            if cond_type not in condition_samples:
                condition_samples[cond_type] = {
                    'text': obj.get('text', 'N/A'),
                    'full_text': full_text
                }
        if 'conditions' in obj and isinstance(obj['conditions'], list):
            for cond in obj['conditions']:
                extract_conditions(cond, full_text)
        if 'condition' in obj and isinstance(obj['condition'], dict):
            extract_conditions(obj['condition'], full_text)
        # Also check other nested fields that might contain conditions
        for key, value in obj.items():
            if isinstance(value, dict):
                extract_conditions(value, full_text)
            elif isinstance(value, list):
                for item in value:
                    extract_conditions(item, full_text)

# Extract from unique_abilities array
for ability in data.get('unique_abilities', []):
    full_text = ability.get('full_text', 'N/A')
    # Extract from cost
    if 'cost' in ability:
        extract_conditions(ability['cost'], full_text)
    # Extract from condition
    if 'condition' in ability:
        extract_conditions(ability['condition'], full_text)
    # Extract from effect
    if 'effect' in ability:
        extract_conditions(ability['effect'], full_text)

# Count frequencies
counter = Counter(condition_types)

# Print sorted by frequency
print("Condition Type Frequency:")
print("=" * 50)
for cond_type, count in counter.most_common():
    print(f"{cond_type}: {count}")

print(f"\nTotal unique condition types: {len(counter)}")
print(f"Total condition instances: {sum(counter.values())}")

# Show samples of rare types (frequency <= 3)
print("\n" + "=" * 80)
print("SAMPLES OF RARE CONDITION TYPES (frequency <= 3):")
print("=" * 80)
for cond_type, count in counter.most_common():
    if count <= 3:
        sample = condition_samples.get(cond_type, {})
        print(f"\n{cond_type} (count: {count}):")
        print(f"  Text: {sample.get('text', 'N/A')}")
        print(f"  Full: {sample.get('full_text', 'N/A')[:100]}...")
