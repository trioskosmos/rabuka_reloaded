#!/usr/bin/env python3
"""Validate generated abilities.json against the working file (from git HEAD)."""

import json, subprocess, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "ability_extraction"))

r = subprocess.run(["git", "show", "HEAD:cards/abilities.json"], capture_output=True)
working = json.loads(r.stdout)
generated = json.loads(Path("cards/abilities.json").read_text(encoding="utf-8"))

wa = {u["full_text"]: u for u in working["unique_abilities"]}
ga = {u["full_text"]: u for u in generated["unique_abilities"]}

MISSING = "MISSING_FIELD"
FIELD_ORDER = [
    "action",
    "action_by",
    "text",
    "triggerless_text",
    "full_text",
    "cost",
    "effect",
    "condition",
    "source",
    "destination",
    "count",
    "value",
    "optional",
    "duration",
    "target",
    "card_type",
    "timing_condition",
    "exclude_self",
    "position",
    "activation_position",
    "heart_type",
    "per_unit",
    "per_unit_count",
    "state_change",
    "actions",
    "select_action",
]


class b:
    HDR = "\033[95m"
    BLUE = "\033[94m"
    GREEN = "\033[92m"
    WARN = "\033[93m"
    FAIL = "\033[91m"
    END = "\033[0m"
    BOLD = "\033[1m"


def find_diff(w_entry, g_entry, path="", report=None):
    if report is None:
        report = []
    if w_entry == g_entry:
        return report
    if type(w_entry) != type(g_entry):
        report.append((path, str(type(w_entry).__name__), str(type(g_entry).__name__)))
        return report
    if isinstance(w_entry, dict):
        for k in sorted(set(list(w_entry.keys()) + list(g_entry.keys()))):
            nk = f"{path}.{k}" if path else k
            if k not in w_entry:
                report.append((nk, MISSING, "MISSING"))
            elif k not in g_entry:
                report.append((nk, repr(w_entry[k])[:60], MISSING))
            elif w_entry[k] != g_entry[k]:
                if isinstance(w_entry[k], (dict, list)):
                    find_diff(w_entry[k], g_entry[k], nk, report)
                else:
                    report.append((nk, repr(w_entry[k])[:60], repr(g_entry[k])[:60]))
    elif isinstance(w_entry, list):
        for i in range(max(len(w_entry), len(g_entry))):
            nk = f"{path}[{i}]"
            if i >= len(w_entry):
                report.append((nk, MISSING, repr(g_entry[i])[:60]))
            elif i >= len(g_entry):
                report.append((nk, repr(w_entry[i])[:60], MISSING))
            else:
                find_diff(w_entry[i], g_entry[i], nk, report)
    return report


total_diff = 0
cats = {}
for ft, we in wa.items():
    ge = ga.get(ft)
    if ge is None:
        print(f"{b.FAIL}MISSING IN GENERATED: {ft[:50]}{b.END}")
        continue
    we_eff = we.get("effect") or {}
    ge_eff = ge.get("effect") or {}
    we_cost = we.get("cost") or {}
    ge_cost = ge.get("cost") or {}

    diffs = find_diff(we_eff, ge_eff, "effect")
    diffs += find_diff(we_cost, ge_cost, "cost")

    if not diffs:
        continue
    total_diff += 1

    # Categorize
    for d in diffs:
        key = d[0]
        if "cost" in key:
            cat = "cost"
        elif "effect.action" in key:
            cat = "action_diff"
        elif "effect.source" in key:
            cat = "missing_source" if "MISSING" in str(d[2]) else "source"
        elif "effect.optional" in key:
            cat = "missing_optional" if "MISSING" in str(d[2]) else "extra_optional"
        elif "effect.condition" in key:
            cat = "condition_diff"
        elif "effect.activation_position" in key:
            cat = "missing_activation_position"
        elif "effect.position" in key:
            cat = "position"
        elif "effect.exclude_self" in key:
            cat = "exclude_self"
        elif "effect.count" in key:
            cat = "count"
        elif "effect.value" in key:
            cat = "value"
        elif "effect.actions" in key:
            cat = "actions"
        elif "effect.select_action" in key:
            cat = "select_action"
        else:
            cat = "other"
        cats[cat] = cats.get(cat, 0) + 1

print(f"\n{b.BOLD}Entries with differences: {total_diff}{b.END}\n")
for cat, cnt in sorted(cats.items(), key=lambda x: -x[1]):
    print(f"  [{cnt:3d}] {cat}")

print(
    f'\n{b.WARN}To see full diff: python -c "from tools.validate_diff import *"{b.END}'
)
