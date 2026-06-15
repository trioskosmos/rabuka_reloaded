"""
Test Coverage Analyzer
======================
Cross-references tested card IDs from engine/tests/test_modules/*.rs
against abilities.json to identify gaps in action types, conditions,
destinations, resource colors, and sequential patterns.

Usage:
    python cards/ability_extraction/analyze_test_coverage.py

Output:
    stdout — grouped coverage report
    test_coverage_report.json  — full structured data for external processing
"""

import json
import os
import re
import sys
from collections import defaultdict
from pathlib import Path

# ---- paths ----
REPO_ROOT = Path(__file__).resolve().parents[2]
ABILITIES_PATH = REPO_ROOT / "cards" / "abilities.json"
TEST_DIR = REPO_ROOT / "engine" / "tests" / "test_modules"
OUTPUT_PATH = REPO_ROOT / "cards" / "ability_extraction" / "test_coverage_report.json"


# ---- helpers ----
def load_abilities():
    with open(ABILITIES_PATH, encoding="utf-8") as f:
        data = json.load(f)
    return data.get("unique_abilities", [])


def load_tested_card_nos():
    """Scan all test .rs files and extract card_no strings from game.id("...") calls."""
    tested = set()
    if not TEST_DIR.is_dir():
        print(f"[WARN] Test directory not found: {TEST_DIR}", file=sys.stderr)
        return tested
    for fname in sorted(TEST_DIR.iterdir()):
        if not fname.name.endswith(".rs"):
            continue
        try:
            content = fname.read_text(encoding="utf-8", errors="replace")
        except Exception:
            continue
        # Match game.id("PL!...") / id("PL!...") / "PL!..."
        for m in re.finditer(r'id\(\s*"([^"]+)"\s*\)', content):
            tested.add(m.group(1))
        for m in re.finditer(r'"PL!(?:HS|N|S|SP)?[^"]*"', content):
            cn = m.group(0).strip('"')
            if cn[:3] == "PL!" or cn[:2] == "PL":
                tested.add(cn)
    return tested


def classify_effect(eff):
    """Return a structural signature string for an ability effect."""
    if not isinstance(eff, dict):
        return "NONE"
    action = eff.get("action", "")
    if action == "move_cards":
        src = eff.get("source", "")
        dst = eff.get("destination", "")
        sc = eff.get("state_change", "")
        return f"move_cards({src}->{dst}|{sc})"
    elif action == "gain_resource":
        rsrc = eff.get("resource", "")
        colors = eff.get("heart_colors", [])
        return f"gain_resource({rsrc}|colors={colors})"
    elif action == "change_state":
        sc = eff.get("state_change", "")
        return f"change_state({sc})"
    elif action == "sequential":
        acts = eff.get("actions", [])
        sub = [classify_effect(a) for a in acts if isinstance(a, dict)]
        return f"sequential([{', '.join(sub)}])"
    elif action == "choice":
        opts = eff.get("options", [])
        tags = [o.get("action", "") for o in opts if isinstance(o, dict)]
        return f"choice([{', '.join(tags)}])"
    elif action == "conditional_alternative":
        eff2 = eff.get("effect", {})
        alt = eff.get("alternative", {})
        return (
            f"conditional_alternative({classify_effect(eff2)}|{classify_effect(alt)})"
        )
    elif action == "restriction":
        return f"restriction(rt={eff.get('restriction_type', '')})"
    elif action == "draw_card":
        return f"draw_card(opt={eff.get('optional', '')})"
    elif action == "play_baton_touch":
        return f"play_baton_touch(cnt={eff.get('count', '')})"
    elif action == "look_and_select":
        return f"look_and_select(src={eff.get('source', '')})"
    elif action:
        return action
    return "NONE"


def get_action_type(eff):
    """Return the top-level action type string."""
    if isinstance(eff, dict):
        return eff.get("action", "NONE")
    return "NONE"


# ---- main ----
def main():
    abilities = load_abilities()
    tested_card_nos = load_tested_card_nos()

    print(f"Loaded {len(abilities)} unique ability entries")
    print(f"Found {len(tested_card_nos)} distinct card_no strings in tests")

    # Build lookup: card_no → list of (ability_index, action_type, feature_sig, card_name)
    card_abilities = defaultdict(list)
    for ab in abilities:
        for entry in ab.get("cards", []):
            parts = entry.split(" | ")
            if len(parts) < 2:
                continue
            card_no = parts[0]
            ab_label = parts[1] if len(parts) > 1 else "?"
            action = get_action_type(ab.get("effect", {}))
            feature = classify_effect(ab.get("effect", {}))
            card_abilities[card_no].append((ab_label, action, feature))

    # Group by action type
    action_coverage = defaultdict(lambda: {"tested_cards": [], "untested_cards": []})
    for card_no, entries in sorted(card_abilities.items()):
        for ab_label, action, feature in entries:
            if card_no in tested_card_nos:
                action_coverage[action]["tested_cards"].append(
                    (card_no, ab_label, feature)
                )
            else:
                action_coverage[action]["untested_cards"].append(
                    (card_no, ab_label, feature)
                )

    # Group by feature signature
    feature_coverage = defaultdict(lambda: {"tested_cards": [], "untested_cards": []})
    for card_no, entries in sorted(card_abilities.items()):
        for ab_label, action, feature in entries:
            if card_no in tested_card_nos:
                feature_coverage[feature]["tested_cards"].append(card_no)
            else:
                feature_coverage[feature]["untested_cards"].append(card_no)

    # --- print report ---
    print(f"\n{'=' * 70}")
    print("  SECTION 1: ACTION TYPE COVERAGE")
    print(f"{'=' * 70}")
    for action in sorted(action_coverage):
        t = action_coverage[action]["tested_cards"]
        u = action_coverage[action]["untested_cards"]
        tested_set = {c for c, _, _ in t}
        all_set = {c for c, _, _ in t} | {c for c, _, _ in u}
        msg = "OK" if any(cn in tested_card_nos for cn in all_set) else "MISS"
        print(
            f"  [{msg:4s}] {action:40s}  tested={len(tested_set):3d}  untested={len(u):3d}"
        )

    print(f"\n{'=' * 70}")
    print("  SECTION 2: UNTESTED FEATURES (detail)")
    print(f"{'=' * 70}")
    for feature in sorted(feature_coverage):
        t = feature_coverage[feature]["tested_cards"]
        u = feature_coverage[feature]["untested_cards"]
        if not t and u:
            print(f"\n  [NO TEST] {feature}")
            for cn in sorted(u)[:5]:
                print(f"      {cn}")
            if len(u) > 5:
                print(f"      ... and {len(u) - 5} more")

    print(f"\n{'=' * 70}")
    print("  SECTION 3: PARTIALLY TESTED FEATURES (some variants covered)")
    print(f"{'=' * 70}")
    for feature in sorted(feature_coverage):
        t = feature_coverage[feature]["tested_cards"]
        u = feature_coverage[feature]["untested_cards"]
        if t and u:
            print(f"\n  [PARTIAL] {feature}")
            print(f"      tested: {len(t)} cards")
            for cn in sorted(u)[:3]:
                print(f"      MISSING: {cn}")
            if len(u) > 3:
                print(f"      ... and {len(u) - 3} more")

    # --- write JSON report ---
    report = {
        "total_ability_entries": len(abilities),
        "total_tested_card_nos": len(tested_card_nos),
        "action_coverage": {},
        "untested_features": {},
        "partially_tested_features": {},
    }
    for action in sorted(action_coverage):
        t = action_coverage[action]["tested_cards"]
        u = action_coverage[action]["untested_cards"]
        tested_set = {c for c, _, _ in t}
        all_set = tested_set | {c for c, _, _ in u}
        covered = any(cn in tested_card_nos for cn in all_set)
        report["action_coverage"][action] = {
            "covered": covered,
            "tested_count": len(tested_set),
            "untested_count": len(u),
        }
    for feature in sorted(feature_coverage):
        t = feature_coverage[feature]["tested_cards"]
        u = feature_coverage[feature]["untested_cards"]
        if not t and u:
            report["untested_features"][feature] = sorted(u)[:10]
        elif t and u:
            report["partially_tested_features"][feature] = {
                "tested_count": len(t),
                "untested_count": len(u),
                "untested_examples": sorted(u)[:5],
            }

    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        json.dump(report, f, ensure_ascii=False, indent=2)
    print(f"\nFull JSON report written to {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
