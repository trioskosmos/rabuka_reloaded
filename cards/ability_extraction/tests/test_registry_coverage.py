"""
tests/test_registry_coverage.py

Property-based regression tests for the effect/condition/action rule registries.
Verifies every registered rule is triggered by at least one ability in the real
card corpus — catches dead rules that no longer match anything.

Run: python -m pytest cards/ability_extraction/tests/test_registry_coverage.py -v
  or: python cards/ability_extraction/tests/test_registry_coverage.py
"""
import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from parser import (
    ActionRule,
    parse_ability,
    parse_action,
    _effect_registry,
    _condition_registry,
    _ACTION_RULES,
)

CORPUS_PATH = os.path.join(
    os.path.dirname(__file__), "..", "..", "abilities.json"
)

# Action rules that currently match at least one corpus text but NEVER win
# dispatch, because an earlier rule in the first-match-wins list claims the same
# text first. Recorded rather than deleted: most are deliberate broad fallbacks
# (`公開する` → reveal loses to `加える` → move_cards), which are still correct
# for card text the corpus does not contain. Deleting them would be trading a
# latent fallback for a guaranteed break on any unseen card.
#
# This baseline exists so the set cannot grow unnoticed. It was empty before
# 2026-09-28, when this test was first added — the effect and condition
# registries were always covered and the action registry never was, which is
# how `action_047` (a strict subset of `action_039`, sitting after it) survived
# review unnoticed.
#
# Investigate a rule here before "fixing" it: check which rule shadows it and
# whether the shadowing is intended. Removing the loser is usually wrong.
SHADOWED_ACTION_RULES = {
    "action_001_position_change",
    "action_006_move_cards",
    "action_009_discard_until_count",
    "action_013_draw_card",
    "action_017_change_state",
    "action_019_activate_ability",
    "action_021_restriction",
    "action_022_restriction",
    "action_026_restriction",
    "action_027_restriction",
    "action_028_restriction",
    "action_032_move_cards",
    "action_036_gain_resource",
    "action_041_re_yell",
    "action_043_reveal",
    "action_045_select_number",
    "action_049_activate_ability",
    "action_053_modify_score",
    "action_054_modify_score",
    "action_055_modify_score",
    "action_056_set_blade_type",
    "action_058_specify_heart_color",
    "action_059_gain_resource",
    "action_061_modify_required_hearts",
    "action_063_repeat_procedure",
    "action_064_do_nothing",
    "action_065_do_nothing",
    "action_066_pay_energy",
    "action_068_invalidate_ability",
    "action_071_move_cards",
    "action_074_gain_ability",
    "action_075_reduce_live_card_set_limit",
    "action_076_set_card_identity",
    "action_077_choose_required_hearts",
    "action_078_all_blade_timing",
    "action_081_conditional_alternative",
    "action_082_gain_resource",
}


def load_ability_texts():
    """Load all unique ability triggerless texts from the extracted corpus."""
    if not os.path.exists(CORPUS_PATH):
        return []
    with open(CORPUS_PATH, encoding="utf-8") as f:
        data = json.load(f)
    texts = []
    for ab in data.get("unique_abilities", []):
        if isinstance(ab, dict):
            t = ab.get("triggerless_text")
            if t:
                texts.append(t)
    return texts


def test_action_rules_are_normalized():
    assert _ACTION_RULES
    assert all(isinstance(rule, ActionRule) for rule in _ACTION_RULES)


def _extract_effect_texts(texts):
    """Parse each ability and return the effect-level text strings that the
    effect registry actually dispatches against (after trigger/cost stripping).
    Falls back to the raw text if parsing yields no effect."""
    out = []
    for t in texts:
        try:
            ab = parse_ability(t)
        except Exception:
            ab = {}
        eff = ab.get("effect") if isinstance(ab, dict) else None
        if isinstance(eff, dict) and eff.get("text"):
            out.append(eff["text"])
        else:
            out.append(t)
    return out


def test_all_effect_rules_triggered():
    """Every rule in _effect_registry should match at least one ability.

    Rules are dispatched against the effect text (after trigger/cost stripping
    and parenthetical removal), so we check both the raw triggerless_text and
    the parsed effect text to avoid false positives on handlers that need
    parenthetical or trigger content."""
    texts = load_ability_texts()
    assert texts, "Corpus is empty — cannot run coverage test"

    effect_texts = _extract_effect_texts(texts)
    # Dedupe while preserving order so each rule is tested against every
    # distinct input form it might see in production.
    all_inputs = list(dict.fromkeys(texts + effect_texts))

    triggered = set()
    for t in all_inputs:
        for _priority, name, handler in _effect_registry.sorted_handlers():
            try:
                if handler(t) is not None:
                    triggered.add(name)
            except Exception:
                pass

    all_rules = {name for _, name, _ in _effect_registry.sorted_handlers()}
    dead = all_rules - triggered
    assert not dead, f"Effect rules never triggered by corpus: {sorted(dead)}"


def test_all_condition_rules_triggered():
    """Every rule in _condition_registry should match at least one ability."""
    texts = load_ability_texts()
    assert texts, "Corpus is empty — cannot run coverage test"

    triggered = set()
    for t in texts:
        for _priority, name, handler in _condition_registry.sorted_handlers():
            try:
                if handler(t) is not None:
                    triggered.add(name)
            except Exception:
                pass

    all_rules = {name for _, name, _ in _condition_registry.sorted_handlers()}
    dead = all_rules - triggered
    assert not dead, f"Condition rules never triggered by corpus: {sorted(dead)}"


def _extract_action_texts(texts):
    """Parse each ability and return the action-clause text the action rules
    actually dispatch against (after trigger/cost stripping). Falls back to the
    raw text if parsing yields no action text."""
    out = []
    for t in texts:
        try:
            ab = parse_ability(t)
        except Exception:
            ab = {}
        act = ab.get("action") if isinstance(ab, dict) else None
        if isinstance(act, dict) and act.get("text"):
            out.append(act["text"])
        else:
            out.append(t)
    return out


def test_all_action_rules_win_some_ability():
    """Every rule in _ACTION_RULES should actually WIN at least one ability.

    Unlike the effect/condition registries, _ACTION_RULES is first-match-wins,
    so "does the rule match" is the wrong question: a rule can match text that
    an earlier rule already claimed, and it would then be dead while looking
    alive. That is exactly how `action_047` survived — its condition was a
    strict subset of `action_039`'s, which sits earlier in the list, so 047
    could never be reached.

    This walks the REAL dispatch (via parse_action, so the action dict the
    predicates inspect is the genuine one) and records which rule wins.

    Like the effect/condition versions, this feeds BOTH the raw
    `triggerless_text` and the action clause that `parse_ability` derived from
    it. Action rules dispatch on the clause, so testing only the raw text would
    report a rule as dead merely because the clause it keys off was never
    handed to it in that form.
    """
    texts = load_ability_texts()
    assert texts, "Corpus is empty — cannot run coverage test"

    inputs = list(dict.fromkeys(texts + _extract_action_texts(texts)))

    winners = set()
    original_apply = ActionRule.apply

    def recording_apply(self, text, action):
        winners.add(self.name)
        return original_apply(self, text, action)

    ActionRule.apply = recording_apply
    try:
        for t in inputs:
            try:
                parse_action(t)
            except Exception:
                pass
    finally:
        ActionRule.apply = original_apply

    all_rules = {rule.name for rule in _ACTION_RULES}
    dead = all_rules - winners
    assert dead <= SHADOWED_ACTION_RULES, (
        "New action rules are shadowed (never win dispatch). "
        f"Newly shadowed: {sorted(dead - SHADOWED_ACTION_RULES)}; "
        "either fix the shadowing rule or record the new one in "
        "SHADOWED_ACTION_RULES if it is a deliberate fallback."
    )


def test_parse_ability_no_crash():
    """parse_ability should not crash on any ability in the corpus."""
    texts = load_ability_texts()
    assert texts, "Corpus is empty — cannot run coverage test"

    crashes = []
    for t in texts:
        try:
            parse_ability(t)
        except Exception as e:
            crashes.append((t[:60], str(e)[:100]))
    assert not crashes, f"parse_ability crashed on {len(crashes)} abilities: {crashes[:5]}"


if __name__ == "__main__":
    tests = [
        test_action_rules_are_normalized,
        test_all_effect_rules_triggered,
        test_all_condition_rules_triggered,
        test_all_action_rules_win_some_ability,
        test_parse_ability_no_crash,
    ]
    passed = 0
    failed = 0
    for fn in tests:
        try:
            fn()
            print(f"  PASS: {fn.__name__}")
            passed += 1
        except Exception as e:
            print(f"  FAIL: {fn.__name__}: {e}")
            failed += 1
    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)
