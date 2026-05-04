"""Validate parser_v2 against current parser output for all 602 abilities."""
import json
import sys
sys.path.insert(0, 'test_parser')
import parser_v2

# Load current abilities.json (has the old parser's output)
with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data_old = json.load(f)

# Run new parser
data_new = parser_v2.process_abilities(json.loads(json.dumps(data_old)))

old_entries = data_old['unique_abilities']
new_entries = data_new['unique_abilities']

total = len(old_entries)
exact_match = 0
action_match = 0
effect_action_match = 0
cost_type_match = 0
no_effect_old = 0
no_effect_new = 0

mismatches = []

for i, (old, new) in enumerate(zip(old_entries, new_entries)):
    old_eff = old.get('effect', {})
    new_eff = new.get('effect', {})
    
    if not isinstance(old_eff, dict):
        no_effect_old += 1
        continue
    if not isinstance(new_eff, dict) or not new_eff:
        no_effect_new += 1
        continue
    
    # Compare effect action
    oa = old_eff.get('action', '')
    na = new_eff.get('action', '')
    if oa == na:
        effect_action_match += 1
    
    # Compare cost type
    oc = old.get('cost', {})
    nc = new.get('cost', {})
    if isinstance(oc, dict) and isinstance(nc, dict):
        if oc.get('type') == nc.get('type'):
            cost_type_match += 1
    
    # Exact match of effect dict
    if old_eff == new_eff:
        exact_match += 1
    else:
        if len(mismatches) < 5:
            mismatches.append((i, oa, na, old.get('triggerless_text','')[:50]))

print(f"Total abilities: {total}")
print(f"Exact effect match: {exact_match}/{total} ({100*exact_match/total:.1f}%)")
print(f"Effect action match: {effect_action_match}/{total} ({100*effect_action_match/total:.1f}%)")
print(f"Cost type match: {cost_type_match}/{total} ({100*cost_type_match/total:.1f}%)")
print(f"No effect (old): {no_effect_old}")
print(f"No effect (new): {no_effect_new}")

print(f"\nTop mismatches (old_action vs new_action):")
for i, oa, na, text in mismatches:
    print(f"  #{i}: old={oa} new={na} text={text}")

# Summary by structure type
print("\n=== Comparison by structure type ===")
from collections import Counter
old_types = Counter()
new_types = Counter()
for old, new in zip(old_entries, new_entries):
    oe = old.get('effect', {})
    ne = new.get('effect', {})
    if isinstance(oe, dict): old_types[oe.get('action','?')] += 1
    if isinstance(ne, dict): new_types[ne.get('action','?')] += 1

for k in sorted(set(list(old_types.keys()) + list(new_types.keys()))):
    print(f"  {k:<25} old={old_types.get(k,0):>3} new={new_types.get(k,0):>3}")
