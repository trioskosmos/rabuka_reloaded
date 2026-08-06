"""
Corpus-wide invariant tests over the real parsed output (cards/abilities.json).

These run against the generated corpus so structural rules are enforced across
every card, catching regressions like the BP07 self-appearance card_type leak
without needing a gameplay test.

Run:  cd cards/ability_extraction && python tests/test_ability_invariants.py
"""

import json
import os
import sys
from pathlib import Path

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

ABILITIES_JSON = None
_here = Path(__file__).resolve()
for _parent in _here.parents:
    candidate = _parent / "abilities.json"
    if candidate.exists():
        ABILITIES_JSON = candidate
        break
assert ABILITIES_JSON is not None, f"could not locate cards/abilities.json from {_here}"

_SELF_APPEARANCE_PATTERNS = ("このメンバーが登場", "このカードが登場")

passed = 0
failed = 0


def test(name, fn):
    global passed, failed
    try:
        fn()
        passed += 1
    except Exception as e:
        failed += 1
        print(f"  FAIL: {name}: {e}")


def walk_nodes(obj):
    """Yield every dict node in the tree (conditions and nested structures)."""
    if isinstance(obj, dict):
        yield obj
        for v in obj.values():
            yield from walk_nodes(v)
    elif isinstance(obj, list):
        for item in obj:
            yield from walk_nodes(item)


def load():
    with open(ABILITIES_JSON, encoding="utf-8") as f:
        return json.load(f)


# ─── Invariant 1: self-appearance conditions must NOT carry card_type ───


def test_self_appearance_has_no_card_type():
    data = load()
    bad = []
    for u in data["unique_abilities"]:
        eff = u.get("effect")
        if not isinstance(eff, dict):
            continue
        for node in walk_nodes(eff):
            if node.get("type") != "appearance_condition":
                continue
            text = node.get("text", "")
            if any(p in text for p in _SELF_APPEARANCE_PATTERNS):
                if "card_type" in node:
                    bad.append((u.get("cards", [""])[0], text, node.get("card_type")))
    assert not bad, (
        "self-appearance conditions must have no card_type — engine self-trigger "
        f"guard requires a bare appearance. Violations: {bad[:5]}"
    )


# ─── Invariant 2: every or_condition with child events has a top-level trigger_event ───


def test_or_condition_aggregates_trigger_event():
    data = load()
    bad = []
    for u in data["unique_abilities"]:
        eff = u.get("effect")
        if not isinstance(eff, dict):
            continue
        for node in walk_nodes(eff):
            if node.get("type") != "or_condition":
                continue
            legs = node.get("conditions") or []
            leg_events = [l for l in legs if isinstance(l, dict) and l.get("trigger_event")]
            if not leg_events:
                continue
            top = node.get("trigger_event")
            if not isinstance(top, dict) or top.get("type") != "or":
                bad.append((u.get("cards", [""])[0], node.get("text", "")))
            else:
                top_events = top.get("events") or []
                if len(top_events) != len(leg_events):
                    bad.append((u.get("cards", [""])[0], node.get("text", "")))
    assert not bad, (
        "or_condition must aggregate a top-level trigger_event (type=or) from its "
        f"event-bearing legs so the engine can prefilter. Violations: {bad[:5]}"
    )


# ─── Invariant 3: every appearance_condition has a trigger_event ───


def test_appearance_has_trigger_event():
    data = load()
    bad = []
    for u in data["unique_abilities"]:
        eff = u.get("effect")
        if not isinstance(eff, dict):
            continue
        for node in walk_nodes(eff):
            if node.get("type") == "appearance_condition" and "trigger_event" not in node:
                bad.append((u.get("cards", [""])[0], node.get("text", "")))
    assert not bad, f"appearance_condition missing trigger_event: {bad[:5]}"


if __name__ == "__main__":
    test("self-appearance has no card_type", test_self_appearance_has_no_card_type)
    test("or_condition aggregates trigger_event", test_or_condition_aggregates_trigger_event)
    test("appearance_condition has trigger_event", test_appearance_has_trigger_event)
    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)
