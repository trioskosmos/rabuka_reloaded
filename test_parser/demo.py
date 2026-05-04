"""
demo.py — Run the radical parser against all 602 unique abilities and
compare results with the existing parser's output.

Usage:
  python demo.py                    # Full comparison report
  python demo.py --quick            # Quick summary only
  python demo.py --show N           # Show N example comparisons
  python demo.py --failures         # Only show mismatches
"""

import json
import sys
from pathlib import Path
from radical_parser import parse_ability, match_structure, extract_slots, assemble_effect, assemble_cost


def load_abilities() -> dict:
    path = Path(__file__).parent.parent / "cards" / "abilities.json"
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def structural_fingerprint(d: dict, depth=0) -> str:
    """Create a normalized structural fingerprint for comparison.

    Ignores specific values (count numbers, group names, etc.) and
    focuses only on the structure: field names, types, nesting patterns.
    """
    if not isinstance(d, dict):
        return str(type(d).__name__)
    parts = []
    for k in sorted(d.keys()):
        if k in ("text", "_text", "full_text", "triggerless_text",
                 "card_count", "cards", "generated_at", "generated_by",
                 "source_file", "schema", "statistics", "name",
                 "card_no", "triggerless_text", "use_limit", "is_null",
                 "triggers", "value", "count", "per_unit_count",
                 "energy", "cost_limit", "heart_colors", "group_names",
                 "quoted_text", "position", "activation_condition",
                 "parenthetical"):
            continue
        if k == "condition":
            parts.append(f"c:{structural_fingerprint(d[k], depth+1)}")
        elif k == "conditions":
            items = [structural_fingerprint(x, depth+1) for x in d.get(k, [])]
            parts.append(f"conds:[{','.join(items)}]")
        elif k == "actions":
            items = [structural_fingerprint(x, depth+1) for x in d.get(k, [])]
            parts.append(f"acts:[{','.join(items)}]")
        elif k == "options":
            items = [structural_fingerprint(x, depth+1) for x in d.get(k, [])]
            parts.append(f"opts:[{','.join(items)}]")
        elif k == "costs":
            items = [structural_fingerprint(x, depth+1) for x in d.get(k, [])]
            parts.append(f"costs:[{','.join(items)}]")
        elif k == "look_action":
            parts.append(f"look:{structural_fingerprint(d[k], depth+1)}")
        elif k == "select_action":
            parts.append(f"sel:{structural_fingerprint(d[k], depth+1)}")
        elif k == "dynamic_count":
            parts.append(f"dyn:{structural_fingerprint(d[k], depth+1)}")
        elif k == "group":
            parts.append("g:{name}")
        elif isinstance(d[k], (str, int, float, bool, type(None))):
            parts.append(k)
        elif isinstance(d[k], dict):
            inner = structural_fingerprint(d[k], depth+1)
            if inner:
                parts.append(f"{k}:{{{inner}}}")
        elif isinstance(d[k], list):
            parts.append(k)
    return ",".join(parts)


def compare_abilities(original: dict, parsed: dict, verbose: bool = False) -> dict:
    """Compare original vs parsed ability structure."""
    result = {
        "full_text": original.get("full_text", ""),
        "has_cost": "cost" in parsed,
        "has_effect": "effect" in parsed,
        "match": False,
        "issues": [],
    }

    orig_effect = original.get("effect") or {}
    new_effect = parsed.get("effect") or {}

    orig_cost = original.get("cost") or {}
    new_cost = parsed.get("cost") or {}

    if not orig_effect and not new_effect:
        result["match"] = True
        return result

    if not orig_effect and new_effect:
        result["issues"].append("original has no effect, parsed has one")
        return result

    if orig_effect and not new_effect:
        result["issues"].append("original has effect, parsed does not")
        return result

    # Compare structural fingerprints
    orig_fp = structural_fingerprint(orig_effect)
    new_fp = structural_fingerprint(new_effect)

    if orig_fp == new_fp:
        result["match"] = True
    else:
        result["orig_fp"] = orig_fp
        result["new_fp"] = new_fp

    # Compare specific key fields
    for key in ("action", "type"):
        ov = orig_effect.get(key) or orig_cost.get(key)
        nv = new_effect.get(key) or new_cost.get(key)
        if ov and nv and ov != nv:
            result["issues"].append(f"effect.{key}: '{ov}' vs '{nv}'")

    # Check cost
    orig_ct = orig_cost.get("type") if orig_cost else None
    new_ct = new_cost.get("type") if new_cost else None
    if orig_ct and new_ct and orig_ct != new_ct:
        result["issues"].append(f"cost.type: '{orig_ct}' vs '{new_ct}'")

    # Check action
    oa = orig_effect.get("action", "")
    na = new_effect.get("action", "")
    if oa and na and oa != na and oa != "custom":
        result["issues"].append(f"action: '{oa}' vs '{na}'")

    # Check source
    for key in ("source", "destination", "card_type", "target", "state_change"):
        ov = orig_effect.get(key) or orig_cost.get(key)
        nv = new_effect.get(key) or new_cost.get(key)
        if ov and nv and ov != nv:
            result["issues"].append(f"{key}: '{ov}' vs '{nv}'")

    return result


def main():
    args = set(sys.argv[1:])
    quick = "--quick" in args
    show = 0
    for a in args:
        if a.startswith("--show="):
            show = int(a.split("=")[1])
    failures_only = "--failures" in args

    data = load_abilities()
    abilities = data["unique_abilities"]

    total = len(abilities)
    matches = 0
    partials = 0
    failures = []
    action_matches = 0
    cost_matches = 0
    source_matches = 0
    dest_matches = 0

    print(f"\n{'='*60}")
    print(f"  RADICAL PARSER - Comparison with existing parser")
    print(f"  Testing on {total} unique abilities")
    print(f"{'='*60}\n")

    for i, ab in enumerate(abilities):
        triggerless = ab.get("triggerless_text", "")
        if not triggerless:
            continue

        parsed = parse_ability(triggerless)
        result = compare_abilities(ab, parsed)

        if result["match"]:
            matches += 1
        elif not result["issues"]:
            partials += 1
        else:
            failures.append(result)

        # Count field-level matches
        orig_eff = ab.get("effect") or {}
        new_eff = parsed.get("effect") or {}
        orig_cost = ab.get("cost") or {}
        new_cost = parsed.get("cost") or {}

        if orig_eff.get("action") == new_eff.get("action"):
            action_matches += 1
        if orig_cost.get("type") == new_cost.get("type") or (not orig_cost and not new_cost):
            cost_matches += 1
        if orig_eff.get("source") == new_eff.get("source"):
            source_matches += 1
        if orig_eff.get("destination") == new_eff.get("destination"):
            dest_matches += 1

        if show > 0 and i < show:
            print(f"\n--- Example {i+1} ---")
            print(f"  Text: {triggerless[:80]}...")
            print(f"  Original action: {orig_eff.get('action', 'N/A')}")
            print(f"  Parsed action:   {new_eff.get('action', 'N/A')}")
            print(f"  Original cost:   {orig_cost.get('type', 'N/A')}")
            print(f"  Parsed cost:     {new_cost.get('type', 'N/A')}")
            ok = "OK" if result["match"] else ("~" if not result["issues"] else "FAIL")
            print(f"  Status: {ok}  ({len(result['issues'])} issues)")

    # Summary
    print(f"\n{'='*60}")
    print(f"  RESULTS")
    print(f"{'='*60}")
    print(f"  Total abilities:     {total}")
    print(f"  Structural matches:  {matches} ({matches/total*100:.1f}%)")
    print(f"  Partial matches:     {partials} ({partials/total*100:.1f}%)")
    print(f"  Failures:            {len(failures)} ({len(failures)/total*100:.1f}%)")
    print(f"\n  Field-level accuracy:")
    print(f"    Action match:      {action_matches}/{total} ({action_matches/total*100:.1f}%)")
    print(f"    Cost type match:   {cost_matches}/{total} ({cost_matches/total*100:.1f}%)")
    print(f"    Source match:      {source_matches}/{total} ({source_matches/total*100:.1f}%)")
    print(f"    Dest match:        {dest_matches}/{total} ({dest_matches/total*100:.1f}%)")

    if failures_only and failures:
        print(f"\n{'='*60}")
        print(f"  FAILURE DETAILS")
        print(f"{'='*60}")
        for f in failures[:20]:
            print(f"\n  Issues: {f['issues']}")
            print(f"  Text: {f['full_text'][:100]}")
            if "orig_fp" in f:
                print(f"  Orig FP: {f['orig_fp'][:120]}")
                print(f"  New FP:  {f['new_fp'][:120]}")

    return 0 if len(failures) == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
