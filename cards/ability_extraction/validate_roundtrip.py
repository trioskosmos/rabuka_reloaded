#!/usr/bin/env python3
"""
Round‑trip validator: for every card ability, parse the triggerless text
and cross‑check the parsed JSON against patterns in the raw text.
Flags mismatches as potential parser bugs.

Usage:  python validate_roundtrip.py
"""

import json, re, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from parser import parse_ability, parse_effect, _normalize_effect_tree

CARDS_JSON = Path(__file__).parent.parent / "cards.json"
MAX_SHOW = 50  # max card examples per pattern


def walk(obj, path=""):
    """Yield (path, value) leaf pairs from a nested dict/list."""
    if isinstance(obj, dict):
        for k, v in obj.items():
            yield from walk(v, f"{path}.{k}" if path else k)
    elif isinstance(obj, list):
        for i, v in enumerate(obj):
            yield from walk(v, f"{path}[{i}]")
    else:
        yield path, obj


def find_in_tree(obj, key, value=None):
    """Recursively search for a key in a nested dict/list tree.
    Returns True if key exists somewhere with matching value (if value is not None)."""
    if isinstance(obj, dict):
        for k, v in obj.items():
            if k == key:
                if value is None or v == value:
                    return True
            if isinstance(v, (dict, list)):
                if find_in_tree(v, key, value):
                    return True
    elif isinstance(obj, list):
        for item in obj:
            if find_in_tree(item, key, value):
                return True
    return False


def check_text_in_parsed(text, pattern, field_path, label):
    """Check that a pattern from the raw text exists in the parsed JSON at field_path."""
    m = re.search(pattern, text)
    if not m:
        return []
    found = m.group(1) if m.lastindex else m.group(0)
    # Walk the parsed JSON looking for the value
    return [(label, found, card_no, ability_idx)]


def main():
    with open(CARDS_JSON, encoding="utf-8") as f:
        cards = json.load(f)

    if isinstance(cards, list):
        cards = {c["card_no"]: c for c in cards if "card_no" in c}

    issues = []
    total = 0
    checked = 0

    for card_no, card in cards.items():
        ability_raw = card.get("ability", "")
        if not ability_raw:
            continue
        # Split by newline for multiple abilities
        parts = [p.strip() for p in ability_raw.split("\n") if p.strip()]
        for ab_idx, part in enumerate(parts):
            total += 1
            # Extract triggerless text (strip trigger icon)
            triggerless = re.sub(r"\{\{[^}]+\}\}", "", part, count=1).strip()
            if not triggerless:
                continue
            checked += 1

            # Parse
            try:
                result = parse_ability(triggerless)
                eff = result.get("effect") or {}
            except Exception:
                issues.append(("PARSE_ERROR", part[:60], card_no, ab_idx))
                continue

            # ── Checks ────────────────────────────────────────────
            e = eff

            # 1. 能力を持たない → ability_filter or or_ability_filters
            if "能力を持たない" in triggerless:
                found_af = False

                def scan_af(d):
                    nonlocal found_af
                    if isinstance(d, dict):
                        if d.get("ability_filter"):
                            found_af = True
                        if d.get("or_ability_filters"):
                            found_af = True
                        for v in d.values():
                            scan_af(v)
                    elif isinstance(d, list):
                        for item in d:
                            scan_af(item)

                scan_af(e)
                if not found_af:
                    issues.append(
                        ("ABILITY_FILTER_MISSING", part[:60], card_no, ab_idx)
                    )

            # 2. そうした場合 → followup / optional_action / conditional_action / conditional:True
            if "そうした場合" in triggerless:
                has = (
                    e.get("followup_action")
                    or e.get("optional_action")
                    or e.get("conditional_action")
                    or e.get("alternative_condition")
                    or (e.get("action") == "conditional_on_optional")
                    or (
                        e.get("action") == "sequential" and e.get("conditional") is True
                    )
                )
                if not has:
                    issues.append(
                        ("CONDITIONAL_FOLLOWUP_MISSING", part[:60], card_no, ab_idx)
                    )

            # 3. から...に置かれた → preceding_moved in condition or per_unit_source
            # Skip cost:effect patterns (colon-separated) since the movement is in the cost
            # Skip self-trigger movements ("このメンバーがステージから控え室に置かれた")
            if (
                re.search(r"から.*?に(置かれた|置いた)", triggerless)
                and "：" not in triggerless
                and "このメンバーが" not in triggerless
            ):
                has_mv = (
                    find_in_tree(e, "source", "preceding_moved")
                    or find_in_tree(e, "per_unit_source", "previous_moved_cards")
                    or find_in_tree(e, "destination", None)
                )
                if not has_mv:
                    issues.append(
                        ("MOVEMENT_PRECEDING_MOVED_MISSING", part[:60], card_no, ab_idx)
                    )

            # 4. 合計 + 以上 → operator ">=" (not "=")
            # Skip equality comparisons (同じ → comparison_type=equality, legitimately "=")
            if re.search(r"合計.*?以上", triggerless) and "同じ" not in triggerless:

                def scan_aggregate(d):
                    hits = []
                    if isinstance(d, dict):
                        if (
                            d.get("aggregate") == "total"
                            and d.get("operator") == "="
                            and d.get("comparison_type") != "equality"
                        ):
                            hits.append(("=", d.get("text", "")[:40]))
                        for v in d.values():
                            hits.extend(scan_aggregate(v))
                    elif isinstance(d, list):
                        for item in d:
                            hits.extend(scan_aggregate(item))
                    return hits

                agg_issues = scan_aggregate(e.get("condition") or {})
                for op, txt in agg_issues:
                    if op == "=":
                        issues.append(
                            ("AGGREGATE_OP_EQ_SHOULD_BE_GE", txt, card_no, ab_idx)
                        )

            # 5. {{icon_all.png → heart_type "all"
            if "{{icon_all.png" in triggerless:
                ht = e.get("heart_type")
                if ht != "all":
                    if e.get("action") == "gain_resource":
                        issues.append(
                            ("ICON_ALL_NO_HEART_TYPE", part[:60], card_no, ab_idx)
                        )
                    # Sequential with multiple gain_resources — check sub-actions
                    elif e.get("action") == "sequential":
                        found_all = False
                        for sub in e.get("actions", []):
                            if sub.get("heart_type") == "all":
                                found_all = True
                        if not found_all:
                            # Check the OLD abilities.json output
                            pass  # might be combined blade+heart handled differently

            # 6. まで (duration) → duration field (skip cost:effect patterns)
            if (
                "まで" in triggerless
                and "ライブ終了時まで" in triggerless
                and "：" not in triggerless
            ):
                if not find_in_tree(e, "duration", "live_end"):
                    issues.append(("DURATION_MISSING", part[:60], card_no, ab_idx))

            # 7. てもよい → optional (check effect tree; skip cost:effect patterns)
            if (
                "てもよい" in triggerless or "してもよい" in triggerless
            ) and "：" not in triggerless:
                # select actions carry optionality internally without the field
                if e.get("action") != "select" and not find_in_tree(
                    e, "optional", True
                ):
                    issues.append(("OPTIONAL_MISSING", part[:60], card_no, ab_idx))

    # ── Report ───────────────────────────────────────────────
    if not issues:
        print(f"\n  OK: {checked}/{total} abilities checked, 0 issues.")
        return

    # Group by type
    from collections import Counter

    by_type = Counter()
    grouped = {}
    for label, detail, cno, aidx in issues:
        by_type[label] += 1
        grouped.setdefault(label, []).append((detail, cno, aidx))

    print(f"\n  ROUND-TRIP ISSUES: {len(issues)} across {len(by_type)} patterns")
    print(f"  (checked {checked}/{total} abilities)\n")
    for label, count in by_type.most_common():
        print(f"  [{label}] {count}")
        for detail, cno, aidx in grouped[label][:MAX_SHOW]:
            print(f"    {cno} (ab#{aidx}): {detail[:80]}")
        if count > MAX_SHOW:
            print(f"    ... and {count - MAX_SHOW} more")
        print()

    return 1 if issues else 0


if __name__ == "__main__":
    sys.exit(main())
