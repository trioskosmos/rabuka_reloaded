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
                # A sub-condition that names its OWN area must not inherit the
                # compound's zone: stamping it would assert the two at once.
                # PL!-pb2-042-L is the case in point. Its compound is
                #   「…revealed_cards に「矢澤にこ」と…があり、**かつ**
                #    自分のセンターエリアにコスト11以上の『BiBi』のメンバーがいる場合**」
                # so the second sub-condition already carries `position: "center"`
                # and its text names センターエリア. `_propagate_compound_locations`
                # would hand it `location: "revealed_cards"` — the centre area is
                # not the revealed-cards zone, so the inherited value would be
                # false on its face. `_drop_unsupported_child_location` exists
                # precisely to undo that, and its docstring names this card. The
                # net parse is CORRECT; this invariant was checking the
                # intermediate state that the parser deliberately reverses.
                if sub.get("position"):
                    continue
                txt = sub.get("text", "")
                if "エリア" in txt or "置き場" in txt:
                    continue
                bad.append((u.get("cards", [""])[0], sub.get("text", "")))
    assert not bad, (
        "bare sub-conditions of a compound must inherit the clause's location "
        f"(parser._propagate_compound_locations); missing on: {bad[:5]}"
    )


def test_or_condition_has_no_hoisted_qualifiers():
    """A disjunction keeps its qualifiers on its legs, and its parent is never negated.

    Two separate rules, both enforced here because the engine has no support for the
    shapes they forbid.

    1. `card_property` on a parent must be shared by every leg. `detect_card_property`
       used to run over the WHOLE condition text and stamp it onto the parent of a
       compound/or node, which for a plain disjunction is a category error — the
       qualifier belongs to one leg. Four cards were affected, each printing an ordinary
       「AかB」 whose leg already carried the qualifier:

         * PL!S-bp3-019-L MIRACLE WAVE
           「…ブレードハートを持たないカードが0枚の場合か、または余剰ハートを2つ以上持っている場合」
         * PL!N-bp7-006-P 近江彼方
           「…『虹ヶ咲』のライブカードかブレードハートを持たない『虹ヶ咲』のメンバーカードがある場合」
         * PL!SP-pb2-004-PP 平安名すみれ — icon on one leg of an `or`
         * PL!SP-bp5-023-L Shooting Voice!! — icon on one leg of an `and`

    2. A parent `negation` is rejected outright, not merely when legs disagree. The
       engine's Compound path returns from `evaluate_condition` BEFORE the negation
       post-processing, so a parent negation never runs. That is sound only because no
       card can produce one: the house style is 「〜がない」, so 「Aがない、またはBがない」
       parses to an `or` whose LEGS each carry their own negation, and a whole-corpus
       survey found zero 「でない」 and zero disjunctions with two or more negated legs.

       Requiring the absence here — rather than honouring it in the engine — is the
       deliberate choice. An unreachable branch would only ever execute under a
       hand-written Condition, i.e. a test that asserts my own invention. If a future
       card genuinely needs a negated disjunction this test fails, and engine support
       gets added then, against a real card.
    """
    bad_hoisted = []
    bad_negation = []
    for u in json.loads(ABILITIES_JSON.read_text(encoding="utf-8"))["unique_abilities"]:
        for node in walk_nodes(u.get("effect")):
            if node.get("type") not in ("or_condition", "compound"):
                continue
            if not isinstance(node.get("conditions"), list) or len(node["conditions"]) < 2:
                continue
            if node.get("negation"):
                bad_negation.append((u.get("cards", [""])[0], node.get("type")))
            if "card_property" in node:
                shared = all(
                    leg.get("card_property") == node.get("card_property")
                    for leg in node["conditions"]
                )
                if not shared:
                    bad_hoisted.append(
                        (u.get("cards", [""])[0], node.get("card_property"))
                    )
    assert not bad_negation, (
        "a compound/or_condition must not carry a top-level `negation` — the engine's "
        "Compound path does not apply it, so the shape would parse and silently never "
        f"run. If a card truly needs a negated disjunction, add engine support "
        f"deliberately: {bad_negation[:5]}"
    )
    assert not bad_hoisted, (
        "a disjunction's parent must not carry a `card_property` that only some legs have "
        f"— that reads as applying to the whole OR: {bad_hoisted[:5]}"
    )


if __name__ == "__main__":
    from _runner import run_module

    sys.exit(run_module(globals()))
