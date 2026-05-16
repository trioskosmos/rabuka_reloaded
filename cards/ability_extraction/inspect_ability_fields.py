#V/usr/bin/env python3
"""
Ability Field Inventory — scans abilities.json and the Rust engine source
to find fields that exist in the data but are never read by the engine.
Also generates a minimal list of cards to cover all fields.
"""

import json
import os
import re
from collections import defaultdict

ROOT = os.path.dirname(__file__)
ABILITIES_JSON = os.path.join(ROOT, "..", "abilities.json")
ENGINE_SRC = os.path.join(ROOT, "..", "..", "engine", "src")
OUTPUT_GAPS = os.path.join(ROOT, "field_gaps.txt")
OUTPUT_MINIMAL = os.path.join(ROOT, "minimal_card_coverage.txt")


def load_abilities():
    with open(ABILITIES_JSON, encoding="utf-8") as f:
        return json.load(f)


# ================================================================
# PART 1 — Field inventory: for each type/action, what fields exist?
# ================================================================

class FieldInstance:
    def __init__(self, fields, card_no, text):
        self.fields = set(fields)
        self.card_no = card_no
        self.text = text


def collect_field_inventory(data):
    """Group by condition type / effect action and collect all field instances."""

    conditions = defaultdict(list)  # condition_type → [FieldInstance]
    costs = []  # [FieldInstance]
    effects = defaultdict(list)  # action → [FieldInstance]

    for ability in data.get("unique_abilities", []):
        # Get one card_no as a reference
        card_no = "UNKNOWN"
        if ability.get("cards"):
            card_no = ability["cards"][0].split(" | ")[0]

        # Condition
        eff = ability.get("effect")
        cond = eff.get("condition") if isinstance(eff, dict) else None
        if cond and isinstance(cond, dict):
            ct = cond.get("type", "UNKNOWN")
            if ct == "compound":
                for sub in cond.get("conditions", []):
                    if isinstance(sub, dict):
                        conditions[sub.get("type", "UNKNOWN")].append(
                            FieldInstance(sub.keys(), card_no, sub.get("text", ""))
                        )
            else:
                conditions[ct].append(
                    FieldInstance(cond.keys(), card_no, cond.get("text", ""))
                )

        # Cost
        cost = ability.get("cost")
        if cost and isinstance(cost, dict):
            costs.append(FieldInstance(cost.keys(), card_no, cost.get("text", "")))

        # Effect (and sub-actions for sequential)
        def collect_effects(eff_data):
            if not eff_data or not isinstance(eff_data, dict):
                return
            action = eff_data.get("action", "UNKNOWN")
            effects[action].append(
                FieldInstance(eff_data.keys(), card_no, eff_data.get("text", ""))
            )
            # Recurse into sequential sub-actions
            if action == "sequential":
                for sub in eff_data.get("actions", []):
                    collect_effects(sub)
                for alt in eff_data.get("alternatives", []):
                    collect_effects(alt)
            # Recurse into conditional alternatives
            if eff_data.get("conditional_alternatives"):
                for alt in eff_data.get("conditional_alternatives", []):
                    collect_effects(alt.get("alternative"))

        collect_effects(ability.get("effect"))

    return {
        "conditions": conditions,
        "costs": costs,
        "effects": effects,
    }


def get_minimal_coverage(instances):
    """Greedy set cover to find minimal cards covering all fields."""
    if not instances:
        return [], 0

    all_fields = set()
    for inst in instances:
        all_fields.update(inst.fields)
    
    # We ignore meta-fields that are always there or irrelevant to engine logic
    ignored = {"type", "text", "action", "full_text", "triggerless_text", "is_null"}
    target_fields = all_fields - ignored
    total_to_cover = len(target_fields)
    
    covered = set()
    selected_instances = []
    
    while target_fields - covered:
        # Pick instance that covers the most new fields
        best_inst = None
        best_new = set()
        
        for inst in instances:
            new_covered = (inst.fields & target_fields) - covered
            if len(new_covered) > len(best_new):
                best_new = new_covered
                best_inst = inst
            elif len(new_covered) == len(best_new) and best_inst:
                if len(inst.fields) > len(best_inst.fields):
                   best_inst = inst
        
        if not best_inst:
            break
            
        selected_instances.append(best_inst)
        covered.update(best_new)
        
    return selected_instances, total_to_cover


def union_field_sets(instances):
    if not instances:
        return set()
    return set.union(*(inst.fields for inst in instances))


def intersect_field_sets(instances):
    if not instances:
        return set()
    return set.intersection(*(inst.fields for inst in instances))


def report_inventory(name, inventory, file):
    """Print a structured report to file."""
    file.write(f"\n{'=' * 60}\n")
    file.write(f"  {name}\n")
    file.write(f"{'=' * 60}\n")

    for key in sorted(inventory.keys()):
        instances = inventory[key]
        union = union_field_sets(instances)
        always = intersect_field_sets(instances)
        total = len(instances)

        file.write(f"\n  {key} ({total} instance(s))\n")
        file.write(f"  {'-' * 55}\n")
        file.write(f"  ALL fields present: {sorted(union)}\n")

        optional = union - always
        if optional:
            file.write(f"  Always present:     {sorted(always)}\n")
            file.write(f"  Sometimes present:  {sorted(optional)}\n")
            field_counts = defaultdict(int)
            for inst in instances:
                for f in optional:
                    if f in inst.fields:
                        field_counts[f] += 1
            for f in sorted(optional):
                file.write(f"    {f}: {field_counts[f]}/{total}\n")


def report_minimal_coverage(inventory_dict, file):
    """Report minimal cards to cover all fields."""
    
    # Pre-calculate to get totals
    total_minimal_cards = set()
    total_fields_covered = 0
    results = {}
    
    for section_name, inventory in inventory_dict.items():
        results[section_name] = {}
        if isinstance(inventory, list):
            minimal, field_count = get_minimal_coverage(inventory)
            results[section_name]["_list"] = (minimal, field_count, len(inventory))
            total_minimal_cards.update(inst.card_no for inst in minimal)
            total_fields_covered += field_count
        else:
            for key in sorted(inventory.keys()):
                instances = inventory[key]
                minimal, field_count = get_minimal_coverage(instances)
                results[section_name][key] = (minimal, field_count, len(instances))
                total_minimal_cards.update(inst.card_no for inst in minimal)
                total_fields_covered += field_count

    file.write(f"# MINIMAL CARD COVERAGE REPORT\n")
    file.write(f"# Lists minimal cards needed to demonstrate every field for each type\n")
    file.write(f"# \n")
    file.write(f"# SUMMARY TOTALS:\n")
    file.write(f"#   Total Unique Cards Picked: {len(total_minimal_cards)}\n")
    file.write(f"#   Total Action/Condition Types: {sum(len(v) for v in results.values())}\n")
    file.write(f"# \n\n")

    for section_name, section_results in results.items():
        file.write(f"\n{'=' * 60}\n")
        file.write(f"  {section_name.upper()}\n")
        file.write(f"{'=' * 60}\n")
        
        if "_list" in section_results:
            minimal, field_count, total_inst = section_results["_list"]
            file.write(f"\n  Total Instances: {total_inst}\n")
            file.write(f"  Picked {len(minimal)} cards to cover {field_count} unique fields\n")
            file.write(f"  {'-' * 55}\n")
            for inst in minimal:
                display_fields = sorted(inst.fields - {"type", "action", "text"})
                file.write(f"  [{inst.card_no}] {display_fields}\n")
                file.write(f"    Text: {inst.text}\n")
            continue

        for key in sorted(section_results.keys()):
            minimal, field_count, total_inst = section_results[key]
            file.write(f"\n  {key} ({total_inst} instance(s)):\n")
            file.write(f"    Picked {len(minimal)} cards to cover {field_count} unique fields\n")
            for inst in minimal:
                display_fields = sorted(inst.fields - {"type", "action", "text", "is_null"})
                file.write(f"    - {inst.card_no}: {display_fields}\n")
                file.write(f"      Source: {inst.text}\n")


# ================================================================
# PART 2 — Cross-reference against Rust engine code
# ================================================================

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
    pattern = re.compile(re.escape(prefix) + r"(\w+)")
    return set(pattern.findall(code))


def find_dot_field_refs(code):
    followed_by = re.compile(r"\.(\w+)(?:\(|\.as_deref|\.unwrap|\.clone|\.map)")
    return set(followed_by.findall(code))


def report_coverage(inventory, file):
    file.write(f"\n\n{'#' * 60}\n")
    file.write(f"  ENGINE COVERAGE CHECK\n")
    file.write(f"{'#' * 60}\n")

    conds = inventory["conditions"]
    for ct, instances in sorted(conds.items()):
        if ct in CONDITION_HANDLERS:
            handler = CONDITION_HANDLERS[ct]
            union = union_field_sets(instances)
            file.write(f"\n  --- Condition: {ct} ---\n")
            file.write(f"  Data fields: {sorted(union)}\n")
            cross_reference_inline(handler, union, file)

    effs = inventory["effects"]
    for action, instances in sorted(effs.items()):
        if action in EFFECT_HANDLERS:
            handler = EFFECT_HANDLERS[action]
            union = union_field_sets(instances)
            file.write(f"\n  --- Effect: {action} ---\n")
            file.write(f"  Data fields: {sorted(union)}\n")
            cross_reference_inline(handler, union, file)


def cross_reference_inline(handler_info, data_fields, file):
    filepath, prefix = handler_info
    code = read_engine_code(filepath)
    if not code:
        file.write(f"  (could not read {filepath})\n")
        return
    refs = find_field_refs(code, prefix)
    dot_refs = find_dot_field_refs(code)
    all_refs = refs | dot_refs
    missing = data_fields - all_refs - {"type", "text", "full_text", "triggerless_text"}

    if missing:
        file.write(f"  ** MISSING from engine ({filepath}): {sorted(missing)}\n")
    else:
        file.write(f"  OK - All fields accounted for\n")


def main():
    data = load_abilities()
    inventory = collect_field_inventory(data)

    with open(OUTPUT_GAPS, "w", encoding="utf-8") as f:
        report_inventory("CONDITIONS", inventory["conditions"], f)
        report_inventory("COSTS", {"cost": inventory["costs"]}, f)
        report_inventory("EFFECTS", inventory["effects"], f)
        report_coverage(inventory, f)

    with open(OUTPUT_MINIMAL, "w", encoding="utf-8") as f:
        report_minimal_coverage({
            "conditions": inventory["conditions"],
            "costs": inventory["costs"],
            "effects": inventory["effects"]
        }, f)

    print(f"Reports generated:\n - {OUTPUT_GAPS}\n - {OUTPUT_MINIMAL}")


if __name__ == "__main__":
    main()
