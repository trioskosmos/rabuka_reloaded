"""
Regression gate for the parser defects in docs/PARSER_DEFECTS.md.

Each test names the card whose printed text and `abilities.json` disagreed, and
asserts the parse now matches the print. These are deliberately card-specific
rather than corpus-wide invariants: the invariant tests in
test_ability_invariants.py police general shape, while a defect here is a rule
ordering or key-propagation slip that only a named card reproduces. A general
invariant written for them would either be too weak to catch the slip or too
strong, and would start failing on unrelated cards.

Run:  python cards/ability_extraction/tests/test_parser_defects.py
"""

import json
import os
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

_here = Path(__file__).resolve()
ABILITIES_JSON = None
for _parent in _here.parents:
    _candidate = _parent / "abilities.json"
    if _candidate.exists():
        ABILITIES_JSON = _candidate
        break
assert ABILITIES_JSON is not None, f"could not locate cards/abilities.json from {_here}"


def walk_nodes(obj):
    """Yield every dict node in the tree."""
    if isinstance(obj, dict):
        yield obj
        for v in obj.values():
            yield from walk_nodes(v)
    elif isinstance(obj, list):
        for item in obj:
            yield from walk_nodes(item)


def load():
    return json.loads(ABILITIES_JSON.read_text(encoding="utf-8"))


def abilities_for(data, card_prefix):
    """Every unique ability referencing a card whose id starts with `card_prefix`.

    Card ids carry a rarity suffix (PL!S-bp7-003-P, -P＋, -R, -R＋) that the
    defect notes write inconsistently, so match on the printed prefix.
    """
    out = []
    for u in data["unique_abilities"]:
        if any(c.split(" ")[0].startswith(card_prefix) for c in (u.get("cards") or [])):
            out.append(u)
    return out


def nodes_for(data, card_prefix):
    for u in abilities_for(data, card_prefix):
        yield from walk_nodes(u.get("effect"))


# ─── D1: a restriction's target must not be read off 「効果によっては」 ───


def test_d1_cannot_wait_by_effect_targets_self():
    """PL!S-bp7-003-P 松浦果南 protects HER OWN Aqours members.

    「…自分のステージにいる…『Aqours』のメンバーは、相手の効果によってはウェイトしない。」
    「相手の効果によっては」 modifies the verb ("because of the opponent's
    effects"); it is not the restriction's target. Reading a target off it
    granted the immunity to the opponent — the exact inverse of the print.
    """
    data = load()
    found = []
    for node in nodes_for(data, "PL!S-bp7-003-P"):
        if node.get("restriction_type") != "cannot_wait_by_effect":
            continue
        found.append(node)
        assert node.get("target") == "self", (
            "cannot_wait_by_effect must target self; the 「相手の効果によっては」 "
            f"phrase is a verb modifier, not a target. Got {node.get('target')!r}"
        )
    assert found, "expected a cannot_wait_by_effect restriction on PL!S-bp7-003-P"


# ─── D2: 「すべて」 must survive onto a 「その後」 follow-up ───


def test_d2_all_quantifier_survives_followup():
    """LL-bp4-001 taps ALL matching members, not 3.

    The clause 「…ブレードが3つ以下のメンバーをすべてウェイトにする」 states a
    blade bound, not a count. The parse used to emit `count: 3` and drop
    `all`, so it tapped exactly three members.
    """
    data = load()
    checked = 0
    for node in nodes_for(data, "LL-bp4-001"):
        if node.get("state_change") != "wait" or node.get("blade_limit") != 3:
            continue
        text = node.get("text", "")
        if "すべて" not in text:
            continue
        checked += 1
        assert node.get("all") is True, (
            "a 「…をすべてウェイトにする」 clause with a blade bound must carry "
            f"all=true, not a count. Got {json.dumps(node, ensure_ascii=False)}"
        )
        assert node.get("count") is None, (
            "the blade bound is blade_limit, not a member count: "
            f"{json.dumps(node, ensure_ascii=False)}"
        )
    assert checked, "expected the all-members blade-bounded wait on LL-bp4-001"


# ─── D4: an effect must not inherit its condition's area and names ───


def test_d4_effect_does_not_inherit_condition_context():
    """PL!-pb2-042-L PSYCHIC FIRE taps any low-heart opponent member.

    The effect clause 「相手のステージにいる…ハートの数が4つ以下のメンバー1人を
    ウェイトにする」 names no position and no characters. The parse leaked
    `position: center` and three quoted character names from the gating
    condition, restricting it to centre-area members of those three.
    """
    data = load()
    checked = 0
    for node in nodes_for(data, "PL!-pb2-042-L"):
        if node.get("state_change") != "wait":
            continue
        checked += 1
        assert not node.get("characters"), (
            "effect inherited character names from its condition: "
            f"{json.dumps(node, ensure_ascii=False)}"
        )
        assert node.get("position") is None, (
            "effect inherited a position from its condition: "
            f"{json.dumps(node, ensure_ascii=False)}"
        )
    assert checked, "expected the wait effect on PL!-pb2-042-L"


# ─── D5: a comparison operator with no bound left is always a parse artefact ───


def test_d5_no_dangling_cost_limit_operator():
    """A `cost_limit_operator` needs a bound to compare against.

    PL!S-pb1-013-N 黒澤ダイヤ leaked a bare `">="` from 「必要ハートにheart04を
    2以上含む」 with no `cost_limit`, which can never be compared.

    A dynamic bound counts: `cost_reference` names another card as the threshold
    (PL!N-bp1-008-P 「これにより控え室に置いたメンバーカードより、コストの低い…」 →
    `cost_reference: previous_moved_card` + `cost_limit_operator: "<"`), and
    `effects/mod.rs` resolves that pair explicitly. So the operator is only
    dangling when neither a numeric bound nor a reference is present.
    """
    data = load()
    bad = []
    for u in data["unique_abilities"]:
        for node in walk_nodes(u.get("effect")):
            if node.get("cost_limit_operator") is None:
                continue
            if node.get("cost_limit") is None and not node.get("cost_reference"):
                bad.append((u.get("cards", [""])[0], node.get("cost_limit_operator")))
    assert not bad, (
        "cost_limit_operator needs a cost_limit or a cost_reference to compare "
        f"against; drop it when neither is present: {bad[:5]}"
    )


# ─── D6: a 「代わりに」 alternative borrows its main condition's card property ───


def test_d6_alternative_condition_inherits_card_property():
    """PL!-pb1-004-P 園田海未 needs a score icon for the +2 tier too.

    「自分の成功ライブカード置き場にスコアを持つ『μ's』のカードが1枚ある場合…
    2枚以上ある場合、代わりに+2」 — 「2枚以上ある場合」 names no property of its
    own, so the alternative must test the same score-icon set. Without it two
    μ's cards with no score icon granted +2.
    """
    data = load()
    checked = 0
    for u in abilities_for(data, "PL!-pb1-004-P"):
        for node in walk_nodes(u.get("effect")):
            main = node.get("condition")
            alt = node.get("alternative_condition")
            if not isinstance(main, dict) or not isinstance(alt, dict):
                continue
            if not main.get("card_property"):
                continue
            checked += 1
            assert alt.get("card_property") == main.get("card_property"), (
                "a shorthand 「代わりに」 alternative condition must borrow the main "
                "condition's card_property; it states none of its own. "
                f"main={json.dumps(main, ensure_ascii=False)} "
                f"alt={json.dumps(alt, ensure_ascii=False)}"
            )
    assert checked, "expected a conditional_alternative with a card_property on PL!-pb1-004-P"


# ─── D7: OR branches of one clause share the clause's zone/owner/type ───


def test_d7_or_branches_share_zone_owner_and_card_type():
    """PL!-pb2-037-L: 「すべて『BiBi』」 is about the revealed member cards.

    「エールにより公開された自分のメンバーカードがすべて『Printemps』か、すべて
    『lily white』か、すべて『BiBi』のいずれかの場合」 — only the first leg parsed
    with a zone, so branches 2/3 degraded to "a BiBi exists anywhere".
    """
    data = load()
    checked = 0
    for node in nodes_for(data, "PL!-pb2-037-L"):
        if node.get("type") not in ("or_condition", "compound"):
            continue
        legs = [c for c in (node.get("conditions") or []) if isinstance(c, dict)]
        if len(legs) < 2 or not any(leg.get("group_names") for leg in legs):
            continue
        for key in ("location", "target", "card_type"):
            donor = next((leg.get(key) for leg in legs if leg.get(key)), None)
            if donor is None:
                continue
            checked += 1
            for leg in legs:
                assert leg.get(key) == donor, (
                    f"OR leg is missing {key}={donor!r} and would read as 'anywhere': "
                    f"{json.dumps(leg, ensure_ascii=False)}"
                )
    assert checked, "expected a grouped or_condition on PL!-pb2-037-L"


# ─── The trigger clause must survive the condition-text strip ───


def test_effect_rule_recognises_its_own_trigger_clause():
    """bp7-022 keeps the state_change trigger that identifies its shape.

    The D4 fix trims a gating condition's clause off every node's `text` before
    the enrichment walk. Done unconditionally it also trimmed the ability's own
    trigger, leaving _try_discard_hand_reactivate_optional with an empty trigger:
    the rule stopped matching, the ability lost its `state_change` trigger, and
    the whole registry-coverage gate failed on a dead rule. A trigger-shaped
    condition (one carrying `trigger_event`) must be left alone.
    """
    data = load()
    checked = 0
    for u in data["unique_abilities"]:
        eff = u.get("effect")
        if not isinstance(eff, dict):
            continue
        if eff.get("action") != "sequential" or not eff.get("conditional"):
            continue
        text = eff.get("text", "")
        if "そのメンバーをアクティブにする" not in text:
            continue
        checked += 1
        cond = eff.get("condition")
        assert isinstance(cond, dict), "the trigger clause was stripped from effect text"
        legs = cond.get("conditions") or [cond]
        assert any(
            isinstance(leg, dict) and leg.get("type") == "state_change_condition"
            for leg in legs
        ), (
            "expected the active→wait state_change trigger to be rebuilt on the "
            f"condition: {json.dumps(cond, ensure_ascii=False)}"
        )
    assert checked, "expected the discard-then-activate ability (Shioriko bp7-022)"


if __name__ == "__main__":
    from _runner import run_module

    sys.exit(run_module(globals()))
