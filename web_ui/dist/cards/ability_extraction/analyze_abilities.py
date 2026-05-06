"""
Reads abilities.json and produces a structured breakdown of what the engine needs to support.
Output: triggers → connectors → actions → parameters, with implementation status.

Usage: python analyze_abilities.py
"""

import json
import os
from collections import Counter, defaultdict

# Paths
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ABILITIES_PATH = os.path.join(SCRIPT_DIR, '..', 'abilities.json')
RULES_PATH = os.path.join(SCRIPT_DIR, '..', '..', 'engine', 'rules', 'rules.txt')

with open(ABILITIES_PATH, encoding='utf-8') as f:
    abilities = json.load(f)

abilities_list = abilities.get('unique_abilities', [])

print("=" * 80)
print("ABILITY SYSTEM INVENTORY")
print("=" * 80)

# ====================================================================
# PART 1: TRIGGERS
# ====================================================================
print("\n## 1. TRIGGERS (when abilities activate)")
print("-" * 60)

trigger_counts = Counter()
for ab in abilities_list:
    t = ab.get('triggers', '') or ''
    for trigger in [x.strip() for x in t.split(',')]:
        if trigger:
            trigger_counts[trigger] += 1

for trigger, count in trigger_counts.most_common():
    print(f"  {trigger}: {count} abilities")

# ====================================================================
# PART 2: CONNECTOR PATTERNS (how effects are structured)
# ====================================================================
print("\n## 2. CONNECTOR PATTERNS (effect structure)")
print("-" * 60)

connector_counts = Counter()
for ab in abilities_list:
    eff = ab.get('effect')
    if not eff:
        connector_counts['(no effect)'] += 1
        continue
    action = eff.get('action', 'none')
    if action == 'sequential':
        actions_in_seq = [a.get('action', '?') for a in eff.get('actions', [])]
        label = f"sequential({', '.join(actions_in_seq)})"
    elif action == 'look_and_select':
        look_action = eff.get('look_action', {}).get('action', '?')
        select_action = eff.get('select_action', {}).get('action', '?')
        label = f"look_and_select(look={look_action}, select={select_action})"
    elif action == 'choice':
        opts = eff.get('options', [])
        opt_actions = [o.get('action', '?') for o in opts] if opts else ['?']
        label = f"choice({', '.join(opt_actions)})"
    elif action == 'conditional_alternative':
        label = "conditional_alternative"
    else:
        label = action
    connector_counts[label] += 1

for conn, count in connector_counts.most_common(40):
    print(f"  {conn}: {count}")

# ====================================================================
# PART 3: ATOMIC ACTIONS (the actual effect operations)
# ====================================================================
print("\n## 3. ATOMIC ACTIONS (individual effect operations)")
print("-" * 60)

def collect_actions(effect):
    """Recursively collect all atomic action names from an effect tree."""
    if not effect:
        return []
    result = []
    action = effect.get('action', '')
    if action in ('sequential', 'choice'):
        for sub in effect.get('actions', []) + effect.get('options', []):
            result.extend(collect_actions(sub))
    elif action == 'look_and_select':
        result.extend(collect_actions(effect.get('look_action')))
        result.extend(collect_actions(effect.get('select_action')))
    elif action == 'conditional_alternative':
        result.extend(collect_actions(effect.get('primary_effect')))
        result.extend(collect_actions(effect.get('alternative_effect')))
    elif action:
        result.append(action)
    return result

action_counts = Counter()
for ab in abilities_list:
    eff = ab.get('effect')
    for a in collect_actions(eff):
        action_counts[a] += 1

for action, count in action_counts.most_common():
    print(f"  {action}: {count}")

# ====================================================================
# PART 4: EFFECT PARAMETERS (what each action carries)
# ====================================================================
print("\n## 4. EFFECT PARAMETER DETAILS")
print("-" * 60)

def collect_all_effects(effect):
    """Recursively collect all effect objects."""
    if not effect:
        return []
    result = [effect]
    action = effect.get('action', '')
    if action in ('sequential', 'choice'):
        for sub in effect.get('actions', []) + effect.get('options', []):
            result.extend(collect_all_effects(sub))
    elif action == 'look_and_select':
        result.extend(collect_all_effects(effect.get('look_action')))
        result.extend(collect_all_effects(effect.get('select_action')))
    elif action == 'conditional_alternative':
        result.extend(collect_all_effects(effect.get('primary_effect')))
        result.extend(collect_all_effects(effect.get('alternative_effect')))
    return result

# Group effects by action and collect their parameter sets
action_params = defaultdict(lambda: defaultdict(set))
for ab in abilities_list:
    for eff in collect_all_effects(ab.get('effect')):
        action = eff.get('action', 'none')
        for key in ['source', 'destination', 'card_type', 'target', 'state_change',
                     'resource', 'count', 'cost_limit', 'total_cost_limit',
                     'heart_color', 'blade_type', 'placement_order', 'operation',
                     'value', 'optional', 'max', 'any_number', 'per_unit']:
            val = eff.get(key)
            if val is not None:
                action_params[action][key].add(str(val))

for action in sorted(action_params.keys()):
    params = action_params[action]
    print(f"\n  {action}:")
    for key in ['source', 'destination', 'card_type', 'target', 'state_change',
                 'resource', 'count', 'cost_limit', 'total_cost_limit',
                 'heart_color', 'operation', 'value', 'optional', 'max', 'any_number']:
        vals = params.get(key)
        if vals:
            print(f"    {key}: {', '.join(sorted(vals)[:5])}{'...' if len(vals) > 5 else ''}")

# ====================================================================
# PART 5: SOURCE / DESTINATION MATRIX
# ====================================================================
print("\n\n## 5. MOVE_CARDS SOURCE→DESTINATION MATRIX")
print("-" * 60)

moves = defaultdict(lambda: defaultdict(int))
for ab in abilities_list:
    for eff in collect_all_effects(ab.get('effect')):
        if eff.get('action') == 'move_cards':
            src = eff.get('source', '?')
            dst = eff.get('destination', '?')
            moves[src][dst] += 1

all_sources = sorted(set(k for k in moves.keys() if k is not None))
all_dests = sorted(set(d for v in moves.values() for d in v.keys() if d is not None))

# Print header
print(f"{'':14}", end='')
for d in all_dests:
    dl = str(d) if d else 'None'
    print(f"{dl:14}", end='')
print()

for s in all_sources:
    label = str(s) if s else 'None'
    print(f"{label:14}", end='')
    for d in all_dests:
        count = moves[s].get(d, 0) if s else 0
        print(f"{str(count):14}", end='')
    print()

# ====================================================================
# PART 6: COST TYPES
# ====================================================================
print("\n## 6. COST TYPES")
print("-" * 60)

cost_type_counts = Counter()
cost_params = defaultdict(lambda: defaultdict(set))
for ab in abilities_list:
    cost = ab.get('cost')
    if not cost:
        cost_type_counts['(no cost)'] += 1
        continue
    ct = cost.get('type', 'none')
    cost_type_counts[ct] += 1
    for key in ['source', 'destination', 'card_type', 'target', 'count',
                 'energy', 'state_change', 'optional', 'self_cost', 'cost_limit']:
        val = cost.get(key)
        if val is not None:
            cost_params[ct][key].add(str(val))

for ct, count in cost_type_counts.most_common():
    print(f"\n  {ct}: {count}")
    params = cost_params.get(ct, {})
    for key in ['source', 'destination', 'card_type', 'target', 'count',
                 'energy', 'state_change', 'optional', 'self_cost', 'cost_limit']:
        vals = params.get(key)
        if vals:
            print(f"    {key}: {', '.join(sorted(vals)[:5])}{'...' if len(vals) > 5 else ''}")

# ====================================================================
# PART 7: FILTERS
# ====================================================================
print("\n## 7. FILTERS USED")
print("-" * 60)

filter_counts = Counter()
for ab in abilities_list:
    cost = ab.get('cost')
    if cost:
        if cost.get('card_type'): filter_counts[f"cost.card_type={cost['card_type']}"] += 1
        if cost.get('cost_limit'): filter_counts['cost.cost_limit'] += 1
        if cost.get('characters'): filter_counts['cost.characters'] += 1
    for eff in collect_all_effects(ab.get('effect')):
        if eff.get('card_type'): filter_counts[f"eff.card_type={eff['card_type']}"] += 1
        if eff.get('cost_limit'): filter_counts['eff.cost_limit'] += 1
        if eff.get('total_cost_limit'): filter_counts['eff.total_cost_limit'] += 1
        if eff.get('group'): filter_counts['eff.group'] += 1
        if eff.get('heart_color'): filter_counts['eff.heart_color'] += 1
        if eff.get('characters'): filter_counts['eff.characters'] += 1

for filt, count in filter_counts.most_common():
    print(f"  {filt}: {count}")

# ====================================================================
# PART 8: UNIQUE COMBINATIONS SUMMARY
# ====================================================================
print("\n## 8. UNIQUE TRIGGER+CONNECTOR COMBINATIONS")
print("-" * 60)

combo_counts = Counter()
for ab in abilities_list:
    trigger = ab.get('triggers', 'none') or 'none'
    eff = ab.get('effect')
    if not eff:
        combo_counts[f"{trigger} → (no effect)"] += 1
        continue
    action = eff.get('action', 'none')
    combo_counts[f"{trigger} → {action}"] += 1

for combo, count in combo_counts.most_common(30):
    print(f"  {combo}: {count}")

print("\nDone. Use this output to cross-reference with engine/src/ability/ to find gaps.")
