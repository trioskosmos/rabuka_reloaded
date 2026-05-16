#V/usr/bin/env python3
"""
Ability Field Inventory — scans abilities.json and the Rust engine source
to find fields that exist in the data but are never read by the engine.
"""

import json
import os
import re
from collections import defaultdict

ROOT = os.path.dirname(__file__)
ABILITIES_JSON = os.path.join(ROOT, "..", "abilities.json")
ENGINE_SRC = os.path.join(ROOT, "..", "..", "engine", "src")


def load_abilities():
    with open(ABILITIES_JSON, encoding="utf-8") as f:
        return json.load(f)


# ================================================================
# PART 1 — Field inventory: for each type/action, what fields exist?
# ================================================================


def collect_field_inventory(data):
    """Group by condition type / effect action and collect all field names."""

    conditions = defaultdict(list)  # condition_type → [list of field sets]
    costs = []  # [list of field sets]
    effects = defaultdict(list)  # action → [list of field sets]

    for ability in data.get("unique_abilities", []):
        # Condition
        eff = ability.get("effect")
        cond = eff.get("condition") if isinstance(eff, dict) else None
        if cond and isinstance(cond, dict):
            ct = cond.get("type", "UNKNOWN")
            # Flatten compound conditions into their sub-conditions
            if ct == "compound":
                for sub in cond.get("conditions", []):
                    if isinstance(sub, dict):
                        conditions[sub.get("type", "UNKNOWN")].append(set(sub.keys()))
            else:
                conditions[ct].append(set(cond.keys()))

        # Cost
        cost = ability.get("cost")
        if cost and isinstance(cost, dict):
            costs.append(set(cost.keys()))

        # Effect (and sub-actions for sequential)
        def collect_effects(eff):
            if not eff or not isinstance(eff, dict):
                return
            action = eff.get("action", "UNKNOWN")
            effects[action].append(set(eff.keys()))
            # Recurse into sequential sub-actions
            if action == "sequential":
                for sub in eff.get("actions", []):
                    collect_effects(sub)
                for alt in eff.get("alternatives", []):
                    collect_effects(alt)
            # Recurse into conditional alternatives
            if eff.get("conditional_alternatives"):
                for alt in eff.get("conditional_alternatives", []):
                    collect_effects(alt.get("alternative"))

        collect_effects(ability.get("effect"))

    return {
        "conditions": conditions,
        "costs": costs,
        "effects": effects,
    }


def union_field_sets(field_sets):
    """Union of all field sets."""
    if not field_sets:
        return set()
    return set.union(*field_sets)


def intersect_field_sets(field_sets):
    """Intersection of all field sets (always present)."""
    if not field_sets:
        return set()
    return set.intersection(*field_sets)


def report_inventory(name, inventory):
    """Print a structured report."""
    print(f"\n{'=' * 60}")
    print(f"  {name}")
    print(f"{'=' * 60}")

    for key in sorted(inventory.keys()):
        field_sets = inventory[key]
        union = union_field_sets(field_sets)
        always = intersect_field_sets(field_sets)
        total = len(field_sets)

        print(f"\n  {key} ({total} instance(s))")
        print(f"  {'-' * 55}")
        print(f"  ALL fields present: {sorted(union)}")

        # Show which fields are NOT always present
        optional = union - always
        if optional:
            print(f"  Always present:     {sorted(always)}")
            print(f"  Sometimes present:  {sorted(optional)}")
            # Show frequency for optional fields
            field_counts = defaultdict(int)
            for fs in field_sets:
                for f in optional:
                    if f in fs:
                        field_counts[f] += 1
            for f in sorted(optional):
                print(f"    {f}: {field_counts[f]}/{total}")


# ================================================================
# PART 2 — Cross-reference against Rust engine code
# ================================================================


# Map from condition type to engine handler file + field variable prefixes
CONDITION_HANDLERS = {
    "card_count_condition": ("condition.rs", "condition."),
    "location_condition": ("condition.rs", "condition."),
    "movement_condition": ("condition.rs", "condition."),
    "appearance_condition": ("condition.rs", "condition."),
    "compound": ("condition.rs", "condition."),
    "group_condition": ("condition.rs", "condition."),
    "comparison_condition": ("condition.rs", "condition."),
    "score_threshold_condition": ("condition.rs", "condition."),
    "state_condition": ("condition.rs", "condition."),
    "energy_state_condition": ("condition.rs", "condition."),
    "opponent_choice_condition": ("condition.rs", "condition."),
    "position_condition": ("condition.rs", "condition."),
    "custom": ("condition.rs", "condition."),
}

# Map from effect action to engine handler file
EFFECT_HANDLERS = {
    "gain_resource": ("effects.rs", "effect."),
    "change_state": ("effects.rs", "effect."),
    "move_cards": ("move_cards.rs", "effect."),
    "draw_card": ("effects.rs", "effect."),
    "pay_energy": ("effects.rs", "effect."),
    "modify_score": ("effects.rs", "effect."),
    "gain_ability": ("effects.rs", "effect."),
    "select": ("choice.rs", ""),
    "sequential": ("compound.rs", ""),
    "set_heart_override": ("effects.rs", "effect."),
    "modify_cost": ("effects.rs", "effect."),
    "modify_required_hearts": ("effects.rs", "effect."),
    "custom": ("effects.rs", "effect."),
    "skip": ("effects.rs", "effect."),
}


def read_engine_code(filepath):
    full = os.path.join(ENGINE_SRC, "ability", filepath)
    if not os.path.exists(full):
        return ""
    with open(full, encoding="utf-8") as f:
        return f.read()


def find_field_refs(code, prefix):
    """Find all `prefix.FIELD` access patterns in the code."""
    pattern = re.compile(re.escape(prefix) + r"(\w+)")
    return set(pattern.findall(code))


def find_dot_field_refs(code):
    """Find all `.field_name` method/field accesses that look like
    condition fields or effect fields."""
    # Look for patterns like .as_deref(), .unwrap(), .map()
    followed_by = re.compile(r"\.(\w+)(?:\(|\.as_deref|\.unwrap|\.clone|\.map)")
    return set(followed_by.findall(code))


def cross_reference(data_field_inventory, engine_code, label):
    """Find fields present in data but missing from engine code."""
    print(f"\n  {'=' * 55}")
    print(f"  ENGINE COVERAGE GAPS — {label}")
    print(f"  {'=' * 55}")

    filepath, prefix = data_field_inventory
    code = read_engine_code(filepath)
    if not code:
        print(f"  (could not read {filepath})")
        return

    refs = find_field_refs(code, prefix)
    # Also find direct field access
    dot_refs = find_dot_field_refs(code)
    all_refs = refs | dot_refs

    print(f"  Fields accessed in {filepath}: {sorted(all_refs)}")


def report_coverage(inventory):
    """For each condition type / effect action, cross-reference."""
    print(f"\n\n{'#' * 60}")
    print(f"  ENGINE COVERAGE CHECK")
    print(f"{'#' * 60}")

    conds = inventory["conditions"]
    for ct, field_sets in sorted(conds.items()):
        if ct in CONDITION_HANDLERS:
            handler = CONDITION_HANDLERS[ct]
            union = union_field_sets(field_sets)
            print(f"\n  --- Condition: {ct} ---")
            print(f"  Data fields: {sorted(union)}")
            cross_reference_inline(handler, union)

    effs = inventory["effects"]
    for action, field_sets in sorted(effs.items()):
        if action in EFFECT_HANDLERS:
            handler = EFFECT_HANDLERS[action]
            union = union_field_sets(field_sets)
            print(f"\n  --- Effect: {action} ---")
            print(f"  Data fields: {sorted(union)}")
            cross_reference_inline(handler, union)


def cross_reference_inline(handler_info, data_fields):
    filepath, prefix = handler_info
    code = read_engine_code(filepath)
    if not code:
        print(f"  (could not read {filepath})")
        return
    refs = find_field_refs(code, prefix)
    dot_refs = find_dot_field_refs(code)
    all_refs = refs | dot_refs
    missing = data_fields - all_refs - {"type", "text", "full_text", "triggerless_text"}

    if missing:
        print(f"  ** MISSING from engine ({filepath}): {sorted(missing)}")
    else:
        print(f"  OK - All fields accounted for")


# ================================================================
# MAIN
# ================================================================


def main():
    data = load_abilities()
    inventory = collect_field_inventory(data)

    # -- Part 1: Field inventory --
    report_inventory("CONDITIONS", inventory["conditions"])
    print(f"\n")
    report_inventory("COSTS", defaultdict(list, {"cost": inventory["costs"]}))
    print(f"\n")
    report_inventory("EFFECTS", inventory["effects"])

    # -- Part 2: Coverage check --
    print(f"\n\n{'#' * 70}")
    print(f"  CROSS-REFERENCE: DATA FIELDS vs ENGINE CODE")
    print(f"{'#' * 70}")
    report_coverage(inventory)

    print(f"\n\nDone.")


if __name__ == "__main__":
    main()
