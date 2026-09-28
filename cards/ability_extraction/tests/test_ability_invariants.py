"""
Corpus-wide invariant tests over the real parsed output (cards/abilities.json).

These run against the generated corpus so structural rules are enforced across
every card, catching regressions like the BP07 self-appearance card_type leak
without needing a gameplay test.

Run:  python cards/ability_extraction/tests/test_ability_invariants.py
"""

import json
import os
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(__file__))
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
_CANONICAL_DURATIONS = {"this_turn", "live_end", "as_long_as", "unless", "permanent"}


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


def test_duration_codes_are_canonical():
    data = load()
    used = {
        node["duration"]
        for ability in data["unique_abilities"]
        for root in (ability.get("cost"), ability.get("effect"))
        if isinstance(root, dict)
        for node in walk_nodes(root)
        if isinstance(node.get("duration"), str)
    }
    assert used <= _CANONICAL_DURATIONS
    assert "until_end_of_live" not in used


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
            leg_events = [
                leg["trigger_event"]
                for leg in legs
                if isinstance(leg, dict) and leg.get("trigger_event")
            ]
            if not leg_events:
                continue
            top = node.get("trigger_event")
            if not isinstance(top, dict) or top.get("type") != "or":
                bad.append((u.get("cards", [""])[0], node.get("text", "")))
            else:
                unique_leg_events = []
                for event in leg_events:
                    if event not in unique_leg_events:
                        unique_leg_events.append(event)
                if top.get("events") != unique_leg_events:
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


# ─── Invariant 4: compound sub-conditions inherit the clause's zone ───


def test_compound_subconditions_inherit_location():
    """A 「AかつB」 clause is read in one zone throughout.

    Whichever sub-condition named the zone names it for the whole clause, so a
    bare sibling must carry it too. Drop it and a cost comparison silently
    widens from "a member on stage's cost" to "any card's cost" — the condition
    still type-checks, still validates, and is simply true far more often.

    The exclusions each mark a sub-condition that already names its own scope,
    and are the same set parser._propagate_compound_locations honours, plus
    `source`:
      - `temporal` — scoped to a turn or a live, not a zone
      - `resource_type` — the resource reads, not the zone
      - `comparison_type` of "score" — a score is never zone-scoped
      - `source` — the card set is already named ("the cards this effect just
        moved"). Adding a zone would silently narrow it to "moved AND in this
        zone", which is a different and usually wrong claim.
    """
    data = load()
    bad = []
    for u in data["unique_abilities"]:
        eff = u.get("effect")
        if not isinstance(eff, dict):
            continue
        for node in walk_nodes(eff):
            if node.get("type") != "compound":
                continue
            subs = node.get("conditions") or []
            inherited = next(
                (s["location"] for s in subs if isinstance(s, dict) and s.get("location")),
                None,
            )
            if not inherited:
                continue
            for sub in subs:
                if not isinstance(sub, dict) or sub.get("location"):
                    continue
                if sub.get("temporal") or sub.get("resource_type") or sub.get("source"):
                    continue
                if sub.get("comparison_type") == "score":
                    continue
                bad.append((u.get("cards", [""])[0], sub.get("text", "")))
    assert not bad, (
        "bare sub-conditions of a compound must inherit the clause's location "
        f"(parser._propagate_compound_locations); missing on: {bad[:5]}"
    )


if __name__ == "__main__":
    from _runner import run_module

    sys.exit(run_module(globals()))
