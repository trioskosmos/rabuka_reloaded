#!/usr/bin/env python3
"""Compare new parser output against existing abilities.json.

Outputs a JSON file with per-ability comparison results.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from test_parser.main import parse_ability

ABILITIES = Path(__file__).parent.parent / 'cards' / 'abilities.json'
OUTPUT = Path(__file__).parent / 'output' / 'comparison.json'

data = json.load(open(ABILITIES, encoding='utf-8'))
entries = data['unique_abilities']

results = []
action_match = 0
action_mismatch = 0
field_match = 0
field_mismatch = 0
total = 0

for i, entry in enumerate(entries):
    t = entry.get('triggerless_text', '')
    if not t:
        continue
    total += 1

    old_effect = entry.get('effect') or {}
    old_cost = entry.get('cost') or {}

    new = parse_ability(t)
    new_effect = new.get('effect') or {}
    new_cost = new.get('cost') or {}

    entry_result = {
        'index': i,
        'triggerless_text': t[:60],
        'old_action': old_effect.get('action', ''),
        'new_action': new_effect.get('action', ''),
        'mismatches': [],
    }

    # Compare action
    oa = old_effect.get('action', '')
    na = new_effect.get('action', '')
    if oa == na:
        action_match += 1
    else:
        entry_result['mismatches'].append(f"action: old={oa} new={na}")
        action_mismatch += 1

    # Compare key fields
    for field in ['source', 'destination', 'count', 'card_type', 'target']:
        ov = old_effect.get(field)
        nv = new_effect.get(field)
        if ov == nv:
            field_match += 1
        else:
            if oa == na:  # only report field mismatches when action matches
                entry_result['mismatches'].append(f"{field}: old={ov} new={nv}")
                field_mismatch += 1

    if entry_result['mismatches']:
        results.append(entry_result)

# Summary
summary = {
    'total': total,
    'action_match': action_match,
    'action_mismatch': action_mismatch,
    'action_accuracy': round(action_match / total * 100, 1) if total else 0,
    'field_match': field_match,
    'field_mismatch': field_mismatch,
    'details': results,
}

OUTPUT.parent.mkdir(parents=True, exist_ok=True)
with open(OUTPUT, 'w', encoding='utf-8') as f:
    json.dump(summary, f, ensure_ascii=False, indent=2)

print(f"Comparison complete: {total} abilities")
print(f"  Action match:     {action_match} ({action_match/total*100:.1f}%)")
print(f"  Action mismatch:  {action_mismatch}")
print(f"  Field accuracy:   {field_match} matches, {field_mismatch} mismatches (on matching actions)")
print(f"  Details saved to: {OUTPUT}")
