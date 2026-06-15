#!/usr/bin/env python3
"""Audit what the parser outputs vs. what the engine handles.
Scans abilities.json for every field and value, cross-references against engine code."""

import json, re, subprocess
from collections import defaultdict


def load_abilities():
    with open("cards/abilities.json", encoding="utf-8") as f:
        return json.load(f)


def get_all_values(entries, key):
    """Recursively collect all values for a given key across all effects."""
    vals = set()

    def walk(obj):
        if isinstance(obj, dict):
            for k, v in obj.items():
                if k == key:
                    if isinstance(v, (list, tuple)):
                        for item in v:
                            if isinstance(item, str):
                                vals.add(item)
                            else:
                                vals.add(str(item))
                    else:
                        vals.add(str(v))
                walk(v)
        elif isinstance(obj, list):
            for item in obj:
                walk(item)

    for entry in entries:
        walk(entry.get("effect", {}))
    return sorted(vals)


def get_all_nested_keys(entries):
    """Get all unique top-level keys across all effects."""
    keys = set()

    def walk(obj, depth=0, max_depth=5):
        if depth > max_depth:
            return
        if isinstance(obj, dict):
            for k in obj.keys():
                keys.add(k)
                walk(obj[k], depth + 1)
        elif isinstance(obj, list):
            for item in obj:
                walk(item, depth + 1)

    for entry in entries:
        walk(entry.get("effect", {}))
    return sorted(keys)


def main():
    data = load_abilities()
    entries = data["unique_abilities"]

    print("=" * 60)
    print("PARSED ACTION TYPES")
    print("=" * 60)
    actions = get_all_values(entries, "action")
    for a in actions:
        print(f"  {a}")

    print()
    print("=" * 60)
    print("PARSED CONDITION TYPES")
    print("=" * 60)
    ctypes = get_all_values(entries, "type")
    for t in ctypes:
        if t == "compound":
            continue
        print(f"  {t}")

    print()
    print("=" * 60)
    print("PARSED DURATION VALUES")
    print("=" * 60)
    durations = get_all_values(entries, "duration")
    for d in durations:
        print(f"  {d}")

    print()
    print("=" * 60)
    print("PARSED SOURCE ZONES")
    print("=" * 60)
    sources = get_all_values(entries, "source")
    for s in sources:
        print(f"  {s}")

    print()
    print("=" * 60)
    print("PARSED DESTINATION ZONES")
    print("=" * 60)
    dests = get_all_values(entries, "destination")
    for d in dests:
        print(f"  {d}")

    print()
    print("=" * 60)
    print("ALL UNIQUE TOP-LEVEL EFFECT KEYS")
    print("=" * 60)
    all_keys = get_all_nested_keys(entries)
    for k in all_keys:
        print(f"  {k}")

    print()
    print("=" * 60)
    print("FIELDS ON PARSED ACTIONS (per action type)")
    print("=" * 60)
    # For each action, list what fields appear
    action_fields = defaultdict(set)

    def collect_action_fields(obj):
        if isinstance(obj, dict) and "action" in obj:
            a = obj["action"]
            for k, v in obj.items():
                if k not in ("action", "text"):
                    action_fields[a].add(k)
        if isinstance(obj, dict):
            for v in obj.values():
                collect_action_fields(v)
        elif isinstance(obj, list):
            for item in obj:
                collect_action_fields(item)

    for entry in entries:
        collect_action_fields(entry.get("effect", {}))

    for a in sorted(action_fields.keys()):
        print(f"  {a}:")
        for f in sorted(action_fields[a]):
            print(f"    {f}")

    print()
    print("=" * 60)
    print("CONDITION FIELDS (per condition type)")
    print("=" * 60)
    cond_fields = defaultdict(set)

    def collect_cond_fields(obj):
        if isinstance(obj, dict) and "type" in obj and obj.get("type") != "compound":
            t = obj["type"]
            for k, v in obj.items():
                if k not in ("type", "text"):
                    cond_fields[t].add(k)
        if isinstance(obj, dict):
            for v in obj.values():
                collect_cond_fields(v)
        elif isinstance(obj, list):
            for item in obj:
                collect_cond_fields(item)

    for entry in entries:
        collect_cond_fields(entry.get("effect", {}))

    for t in sorted(cond_fields.keys()):
        print(f"  {t}:")
        for f in sorted(cond_fields[t]):
            print(f"    {f}")

    print()
    print("=" * 60)
    print("COST TYPES")
    print("=" * 60)
    cost_types = get_all_values(entries, "type")
    for t in cost_types:
        if t in (
            "compound",
            "group_condition",
            "card_count_condition",
            "location_condition",
            "state_change_condition",
            "temporal_condition",
            "comparison_condition",
        ):
            continue
        print(f"  {t}")

    print()
    print("=" * 60)
    print("SAMPLE ABILITIES WITH 'custom' ACTION (unmatched dispatch rules)")
    print("=" * 60)
    custom_count = 0

    def find_custom(obj, path=""):
        global custom_count
        if isinstance(obj, dict) and obj.get("action") == "custom":
            custom_count += 1
            cards = (
                ", ".join([c[:30] for c in path.get("cards", [])])
                if isinstance(path, dict)
                else ""
            )
            text = obj.get("text", "")[:80]
            print(f"  → {text}")
            if custom_count >= 10:
                return
        if isinstance(obj, dict):
            for k, v in obj.items():
                new_path = {}
                if k == "cards":
                    new_path["cards"] = v
                find_custom(v, new_path)
        elif isinstance(obj, list):
            for item in obj:
                find_custom(item, path)

    find_custom(entries)
    if custom_count == 0:
        print("  (none — all actions matched)")

    print()
    print("=" * 60)
    print("ENGINE EffectAction enum (from effects.rs)")
    print("=" * 60)
    try:
        result = subprocess.run(
            r"grep -oP '(?<=EffectAction::)\w+' engine/src/ability/types.rs 2>nul || "
            r"findstr /r \"EffectAction::\" engine\\src\\ability\\types.rs",
            capture_output=True,
            text=True,
            shell=True,
        )
        # Try rust grep instead
        match = re.findall(
            r"EffectAction::(\w+)",
            open(r"engine\src\ability\types.rs", encoding="utf-8").read(),
        )
        for m in sorted(set(match)):
            print(f"  {m}")
    except:
        print("  (could not read engine)")


if __name__ == "__main__":
    main()
