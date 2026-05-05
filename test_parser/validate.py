"""Validate the new parser structure against ALL 602 real abilities from abilities.json.

This loads the previously-parsed abilities.json and re-parses each ability's
effect text using the new prototype parser. It then compares:
- Action type distribution (should match the old parser's distribution)
- Unmatched text count (NONE should be "custom" for the common patterns)
- Priority resolution correctness (modify_cost beats move_cards, etc.)

Usage: python test_parser/validate.py
"""

import json
import os
import sys
import re
import logging
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
sys.path.insert(0, os.path.dirname(__file__))

logging.basicConfig(level=logging.WARNING, format='%(levelname)s: %(message)s')

from dispatcher import Rule
from full_actions import parse_action, pre_extract, DISPATCH
from full_effects import parse_effect
from conditions import parse_condition

# Load reference data
ABILITIES_PATH = os.path.join(os.path.dirname(__file__), '..', 'cards', 'abilities.json')
with open(ABILITIES_PATH, encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']
print(f"Loaded {len(abilities)} unique abilities from abilities.json")

# ------------------------------------------------------------------
# Phase 1: Compare action type distributions
# ------------------------------------------------------------------

def extract_text_for_parsing(ab: dict) -> str:
    """Get the text that represents the core action of this ability."""
    # Use triggerless_text if available; strip cost marker
    t = ab.get('triggerless_text') or ab.get('full_text', '')
    # Remove trigger icons
    t = re.sub(r'\{\{[^}]+?\}\}', '', t).strip()
    # Split on ： to get just the effect part
    if '：' in t:
        t = t.split('：', 1)[1].strip()
    # Strip trailing period
    t = t.rstrip('。')
    return t

def get_reference_action(ab: dict) -> str:
    """Get the action type from the reference abilities.json."""
    eff = ab.get('effect')
    if eff:
        return eff.get('action', '(none)')
    return '(no effect)'

# Parse all abilities with the new prototype
print("\n--- Phase 1: Action type distribution ---")
new_stats = {}
old_stats = {}
custom_count = 0
unmatched = []

for i, ab in enumerate(abilities):
    # Old parser's action
    old_action = get_reference_action(ab)
    old_stats[old_action] = old_stats.get(old_action, 0) + 1

    # New parser's action
    text = extract_text_for_parsing(ab)
    try:
        state = parse_effect(text)
        new_action = state.get('action', 'custom')
        if new_action == 'custom' and 'actions' in state:
            new_action = 'sequential'
    except Exception as e:
        new_action = 'ERROR'
        print(f"  ERROR parsing [{i}]: {e}")
    
    new_stats[new_action] = new_stats.get(new_action, 0) + 1
    if new_action == 'custom':
        custom_count += 1
        unmatched.append((i, text[:50]))

# Report
all_actions = sorted(set(list(old_stats.keys()) + list(new_stats.keys())))
print(f"\n{'Action':30s} {'Old':>6s} {'New':>6s}  Match")
print("-" * 55)
match_count = 0
total = len(abilities)
for a in all_actions:
    o = old_stats.get(a, 0)
    n = new_stats.get(a, 0)
    m = "OK" if o == n else f"Δ ({o - n:+d})"
    if o == n:
        match_count += 1
    print(f"{a:30s} {o:6d} {n:6d}  {m}")

print(f"\nMatched actions: {match_count}/{len(all_actions)}")
print(f"Unmatched (custom): {custom_count} / {total}")

if unmatched:
    print(f"\nFirst 10 unmatched texts:")
    for idx, txt in unmatched[:10]:
        print(f"  [{idx}] {txt}")

# ------------------------------------------------------------------
# Phase 2: Condition parsing coverage
# ------------------------------------------------------------------

print("\n\n--- Phase 2: Condition parsing coverage ---")

# Collect all condition texts from abilities.json
condition_texts = []
for ab in abilities:
    eff = ab.get('effect') or {}
    def collect_conditions(d, path):
        if isinstance(d, dict):
            if d.get('text') and d.get('condition_type'):
                condition_texts.append((d['text'][:80], d.get('condition_type', '?')))
            if 'condition' in d and isinstance(d['condition'], dict):
                collect_conditions(d['condition'], path + '.condition')
            if 'conditions' in d and isinstance(d['conditions'], list):
                for ci, c in enumerate(d['conditions']):
                    collect_conditions(c, path + f'.conditions[{ci}]')
            if 'cause' in d and isinstance(d['cause'], dict):
                collect_conditions(d['cause'], path + '.cause')
    collect_conditions(eff, 'effect')

# De-duplicate by first 40 chars
seen = set()
unique_conditions = []
for txt, ctype in condition_texts:
    key = txt[:40]
    if key not in seen:
        seen.add(key)
        unique_conditions.append((txt, ctype))

print(f"Total condition instances: {len(condition_texts)}")
print(f"Unique condition texts: {len(unique_conditions)}")

# Parse each with new prototype and check type match
matched_conditions = 0
mismatched = []
for txt, expected_type in unique_conditions[:100]:  # sample first 100
    result = parse_condition(txt)
    if result and result.get('type') == expected_type:
        matched_conditions += 1
    else:
        got = result.get('type', 'None') if result else 'None'
        mismatched.append((txt[:40], expected_type, got))

print(f"Condition type match: {matched_conditions}/{min(100, len(unique_conditions))}")
if mismatched:
    print(f"First 5 mismatches:")
    for txt, exp, got in mismatched[:5]:
        print(f"  expected={exp} got={got}  text={txt}")

# ------------------------------------------------------------------
# Phase 3: Priority resolution correctness
# ------------------------------------------------------------------

print("\n\n--- Phase 3: Priority edge cases ---")

priority_tests = [
    # (test_name, text, expected_action, check_fields)
    ("modify_cost beats move", 
     "能力を持たないメンバーカードを自分の手札から登場させるためのコストは1減る",
     "modify_cost", {'operation': 'decrease'}),
    ("modify_cost vs move with source+dest",
     "手札を1枚控え室に置く",
     "move_cards", {'source': 'hand', 'destination': 'discard'}),
    ("draw_card with 枚",
     "カードを3枚引く",
     "draw_card", {'count': 3}),
    ("gain_resource blade",
     "{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る",
     "gain_resource", {'resource': 'blade', 'count': 2}),
    ("change_state wait",
     "このメンバーをウェイトにする",
     "change_state", {'state_change': 'wait'}),
    ("change_state active",
     "エネルギーを2枚アクティブにする",
     "change_state", {'state_change': 'active'}),
    ("pay_energy optional",
     "{{icon_energy.png|E}}支払ってもよい",
     "pay_energy", {'energy': 1, 'optional': True}),
]

passed = 0
failed = 0
for name, text, expected_action, fields in priority_tests:
    state = parse_action(text)
    actual = state.get('action')
    if actual != expected_action:
        print(f"  FAIL {name}: expected {expected_action}, got {actual}")
        failed += 1
        continue
    all_ok = True
    for k, v in fields.items():
        if state.get(k) != v:
            print(f"  FAIL {name}: field {k}={state.get(k)!r}, expected {v!r}")
            all_ok = False
    if all_ok:
        passed += 1

print(f"Priority edge cases: {passed}/{passed+failed} passed")

# ------------------------------------------------------------------
# Phase 4: Performance comparison
# ------------------------------------------------------------------

import time

print("\n\n--- Phase 4: Performance (first 500 abilities) ---")

sample = abilities[:500]

# Warm up
for ab in sample[:10]:
    parse_action(extract_text_for_parsing(ab))

# Timed run
start = time.perf_counter()
for ab in sample:
    parse_action(extract_text_for_parsing(ab))
elapsed = time.perf_counter() - start
print(f"New parser: {len(sample)} texts in {elapsed*1000:.1f}ms ({elapsed/len(sample)*1000:.3f}ms per text)")

# ------------------------------------------------------------------
# Summary
# ------------------------------------------------------------------

print(f"\n\n{'='*60}")
print(f"VALIDATION SUMMARY")
print(f"{'='*60}")
print(f"Total unique abilities:        {len(abilities)}")
print(f"Action distribution matches:   {match_count}/{len(all_actions)}")
print(f"Unmatched (custom) rate:       {custom_count}/{total} ({100*custom_count/total:.1f}%)")
print(f"Condition type match rate:     {matched_conditions}/{min(100, len(unique_conditions))}")
print(f"Priority edge cases passed:    {passed}/{passed+failed}")
print(f"Parse time:                    {elapsed*1000:.1f}ms for {len(sample)} texts")

if custom_count == 0 and failed == 0:
    print(f"\nRESULT: The new parser structure matches or exceeds the old parser's coverage.")
else:
    print(f"\nRESULT: {custom_count} unmatched texts, {failed} edge case failures.")
    print(f"These indicate patterns the prototype doesn't handle yet.")
