import json
import sys
from collections import Counter, defaultdict

with open(r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

entries = data['unique_abilities']
print(f"Total entries: {len(entries)}")

# === 1. Structure Type Classification ===
def classify_structure(entry):
    """Classify the structure type of an ability entry."""
    if entry.get("is_null") or entry.get("effect") is None:
        return "null_effect"
    
    effect = entry["effect"]
    action = effect.get("action", "")
    
    # Check if effect has a condition
    has_condition = "condition" in effect and effect["condition"] is not None
    
    # Check if effect has cost (entry-level cost)
    has_cost = entry.get("cost") is not None
    
    if action == "sequential":
        # Check if this is a known pattern like "draw + discard" which is just sequential
        return "sequential"
    elif action == "look_and_select":
        return "look_and_select"
    elif has_condition:
        # Check if effect also has sequential nested inside
        sub_action = effect.get("action", "")
        if sub_action == "sequential":
            return "conditional_sequential"
        return "conditional"
    elif action == "gain_resource" and effect.get("per_unit"):
        return "per_unit"
    elif action == "choice":
        return "choice"
    elif action in ("move_cards", "draw_card", "gain_resource", "change_state", "gain_ability", "look_at", "modify_score"):
        # Simple single-action effect
        return "simple"
    else:
        return f"other:{action}"

def get_cost_type(entry):
    cost = entry.get("cost")
    if cost is None:
        return "no_cost"
    ctype = cost.get("type", "unknown")
    return ctype

def get_effect_action(entry):
    if entry.get("is_null") or entry.get("effect") is None:
        return "null"
    effect = entry["effect"]
    action = effect.get("action", "unknown")
    if action == "sequential":
        # Get sub-actions
        subs = [a.get("action", "unknown") for a in effect.get("actions", [])]
        return f"sequential({'+'.join(subs)})"
    elif action == "look_and_select":
        look = effect.get("look_action", {}).get("action", "unknown")
        sel = effect.get("select_action", {}).get("action", "unknown")
        if sel == "sequential":
            sel_subs = [a.get("action", "unknown") for a in effect.get("select_action", {}).get("actions", [])]
            return f"look_and_select({look}+sequential({'+'.join(sel_subs)}))"
        return f"look_and_select({look}+{sel})"
    return action

def get_condition_type(entry):
    if entry.get("is_null") or entry.get("effect") is None:
        return None
    effect = entry["effect"]
    cond = effect.get("condition")
    if cond is None:
        return None
    return cond.get("type", "unknown")

def has_nested_pattern(entry):
    """Check if effect has nested actions (sequential, look_and_select inside something else)."""
    if entry.get("is_null") or entry.get("effect") is None:
        return False
    effect = entry["effect"]
    action = effect.get("action", "")
    if action == "sequential":
        return True
    if action == "look_and_select":
        return True
    # Check condition effect having nested
    if "condition" in effect and effect.get("condition"):
        # Check if the effect action itself is sequential/look_and_select
        if action in ("sequential", "look_and_select"):
            return True
    return False

# Stats
structure_counts = Counter()
cost_type_counts = Counter()
effect_action_counts = Counter()
condition_type_counts = Counter()
cost_effect_pairs = Counter()
condition_effect_pairs = Counter()

# Per-cost-type breakdown of effect types
cost_effect_cross = defaultdict(Counter)

for entry in entries:
    struct = classify_structure(entry)
    structure_counts[struct] += 1
    
    ct = get_cost_type(entry)
    cost_type_counts[ct] += 1
    
    ea = get_effect_action(entry)
    effect_action_counts[ea] += 1
    
    condt = get_condition_type(entry)
    if condt:
        condition_type_counts[condt] += 1
    
    pair = f"cost:{ct} + effect:{ea}"
    cost_effect_pairs[pair] += 1
    
    if condt:
        cond_eff_pair = f"cond:{condt} + effect:{ea}"
        condition_effect_pairs[cond_eff_pair] += 1
    
    cost_effect_cross[ct][ea] += 1

# === Print Summary ===
total = len(entries)
print("\n" + "="*70)
print("STRUCTURAL ANALYSIS OF 602 UNIQUE ABILITY ENTRIES")
print("="*70)

print("\n## 1. STRUCTURE TYPE DISTRIBUTION")
print(f"{'Structure Type':<25} {'Count':>6} {'% of total':>10}")
print("-"*45)
for struct, count in sorted(structure_counts.most_common(), key=lambda x: -x[1]):
    pct = count / total * 100
    print(f"{struct:<25} {count:>6} {pct:>9.1f}%")

print("\n## 2. COST TYPE DISTRIBUTION")
print(f"{'Cost Type':<25} {'Count':>6} {'% of total':>10}")
print("-"*45)
for ct, count in sorted(cost_type_counts.most_common(), key=lambda x: -x[1]):
    pct = count / total * 100
    print(f"{ct:<25} {count:>6} {pct:>9.1f}%")

print("\n## 3. EFFECT ACTION TYPE DISTRIBUTION")
print(f"{'Effect Action':<50} {'Count':>6} {'% of total':>10}")
print("-"*70)
for ea, count in sorted(effect_action_counts.most_common(), key=lambda x: -x[1]):
    pct = count / total * 100
    print(f"{ea:<50} {count:>6} {pct:>9.1f}%")

print("\n## 4. CONDITION TYPE DISTRIBUTION (entries with conditions)")
total_with_cond = sum(condition_type_counts.values())
print(f"Total entries with condition field: {total_with_cond}")
print(f"{'Condition Type':<30} {'Count':>6} {'% of conditioned':>16}")
print("-"*55)
for condt, count in sorted(condition_type_counts.most_common(), key=lambda x: -x[1]):
    pct = count / total_with_cond * 100 if total_with_cond > 0 else 0
    print(f"{condt:<30} {count:>6} {pct:>15.1f}%")

print("\n## 5. TOP COST + EFFECT PATTERNS (min 2 occurrences)")
print(f"{'Pattern':<65} {'Count':>6} {'%':>6}")
print("-"*80)
for pair, count in sorted(cost_effect_pairs.most_common(), key=lambda x: -x[1]):
    if count < 2:
        continue
    pct = count / total * 100
    print(f"{pair:<65} {count:>6} {pct:>5.1f}%")

print("\n## 6. CONDITION + EFFECT PATTERNS")
print(f"{'Pattern':<65} {'Count':>6}")
print("-"*75)
for pair, count in sorted(condition_effect_pairs.most_common(), key=lambda x: -x[1]):
    if count < 2:
        continue
    print(f"{pair:<65} {count:>6}")

print("\n## 7. COST x EFFECT CROSS-TABULATION")
print(f"{'Cost Type':<20} {'Effect Action':<45} {'Count':>6}")
print("-"*75)
for ct in sorted(cost_effect_cross.keys()):
    for ea, count in sorted(cost_effect_cross[ct].most_common(), key=lambda x: -x[1]):
        print(f"{ct:<20} {ea:<45} {count:>6}")

print("\n## 8. CONDITION SUBTYPES DETAIL")
# Detailed breakdown of condition structure
cond_subtypes = defaultdict(Counter)
for entry in entries:
    if entry.get("is_null") or entry.get("effect") is None:
        continue
    effect = entry["effect"]
    cond = effect.get("condition")
    if cond is None:
        continue
    ctype = cond.get("type", "unknown")
    if ctype == "compound":
        op = cond.get("operator", "unknown")
        sub_types = [c.get("type", "unknown") for c in cond.get("conditions", [])]
        cond_subtypes["compound_"+op].update(sub_types)
    elif ctype == "temporal_condition":
        temporal = cond.get("temporal", "unknown")
        cond_subtypes["temporal"][temporal] += 1
    elif ctype == "comparison_condition":
        comp_type = cond.get("comparison_type", cond.get("operator", "unknown"))
        cond_subtypes["comparison"][comp_type] += 1
    elif ctype == "appearance_condition":
        cond_subtypes["appearance"]["yes"] += 1
    elif ctype == "location_condition":
        loc = cond.get("location", "unknown")
        cond_subtypes["location"][loc] += 1
    elif ctype == "card_count_condition":
        cond_subtypes["card_count"][cond.get("operator", ">=")] += 1
    elif ctype == "group_condition":
        cond_subtypes["group"]["yes"] += 1

for cond_cat, sub_counts in sorted(cond_subtypes.items()):
    print(f"\n  {cond_cat}:")
    for sub, cnt in sorted(sub_counts.most_common(), key=lambda x: -x[1]):
        print(f"    {sub:<30} {cnt:>4}")
