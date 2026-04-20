"""Show unparsed custom actions and costs to identify patterns."""
import json
from pathlib import Path

# Load abilities
abilities_path = Path(__file__).parent.parent / 'abilities.json'
with open(abilities_path, 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

print("=== UNPARSED CUSTOM ACTIONS ===\n")
custom_actions = [a for a in abilities if a.get('effect', {}).get('action') == 'custom']
for i, a in enumerate(custom_actions[:20], 1):
    print(f"{i}. {a['triggerless_text']}")

print("\n=== UNPARSED CUSTOM COSTS ===\n")
custom_costs = [a for a in abilities if a.get('cost') and a.get('cost', {}).get('type') == 'custom']
for i, a in enumerate(custom_costs[:20], 1):
    print(f"{i}. {a['triggerless_text']}")

print("\n=== COMPOUND CONDITIONS NOT PARSED ===\n")
compound_abilities = [a for a in abilities if 'かつ' in a['triggerless_text']]
compound_not_parsed = [a for a in compound_abilities if not (a.get('effect') and a.get('effect', {}).get('condition', {}).get('type') == 'compound')]
for i, a in enumerate(compound_not_parsed, 1):
    print(f"{i}. {a['triggerless_text']}")

print("\n=== WAIT NOT PARSED ===\n")
wait_abilities = [a for a in abilities if 'ウェイト' in a['triggerless_text']]
wait_not_parsed = [a for a in wait_abilities if not ((a.get('effect') and a.get('effect', {}).get('state_change')) or (a.get('cost') and a.get('cost', {}).get('state_change')))]
for i, a in enumerate(wait_not_parsed[:20], 1):
    print(f"{i}. {a['triggerless_text']}")

print("\n=== POSITION NOT PARSED ===\n")
position_abilities = [a for a in abilities if any(x in a['triggerless_text'] for x in ['センターエリア', '左サイド', '右サイド'])]
position_not_parsed = [a for a in position_abilities if not ((a.get('effect') and a.get('effect', {}).get('condition', {}).get('position')) or (a.get('cost') and a.get('cost', {}).get('position')))]
for i, a in enumerate(position_not_parsed, 1):
    print(f"{i}. {a['triggerless_text']}")
