"""Test process_abilities vs parse_ability."""
import sys
import json
sys.path.insert(0, '.')
from parser import parse_ability, process_abilities

# Load abilities.json
with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

# Get the sunny day song entry
ua = data.get('unique_abilities', [])
entry = ua[523]
triggerless = entry.get('triggerless_text', '')
print(f"Triggerless text: {triggerless[:60]}...")

# Parse directly
parsed = parse_ability(triggerless)
print("\n=== Direct parse_ability result ===")
print(json.dumps(parsed.get('effect', {}), indent=2, ensure_ascii=False)[:2000])

# Run process_abilities
data2 = process_abilities(data)
entry2 = data2['unique_abilities'][523]
print("\n=== process_abilities result ===")
print(json.dumps(entry2.get('effect', {}), indent=2, ensure_ascii=False)[:2000])

print("\n=== Are they the same? ===")
print(parsed.get('effect') == entry2.get('effect'))
