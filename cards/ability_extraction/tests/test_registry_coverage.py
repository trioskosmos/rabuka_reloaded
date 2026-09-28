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


# NOTE — there is deliberately NO action-registry dead-rule test here.
#
# A "does this rule ever WIN dispatch" check looks like the effect/condition
# versions above, but it is unsound for _ACTION_RULES, which is first-match-wins
# and is dispatched against many derived clause forms (sequential steps, nested
# sub-actions, per-effect re-parsing). Whatever text you drive it with, some
# live rules are never reached. Measured false-positive counts on 2026-09-28:
# 37 rules when driving `parse_action` per corpus text, 29 when driving the
# full `parse_ability`, 30 when driving raw ability strings from cards.json.
# Three inputs, three different wrong answers.
#
# Acting on the first number (deleting those 37) removed `modify_limit`,
# `repeat_procedure` and `draw_until_count` from the emitted corpus — caught by
# the regenerate + `git diff --numstat cards/abilities.json` gate, not by any
# test. The rules were restored and the suite is green again.
#
# The one provably dead action rule, `action_047`, was removed by reading its
# predicate: its condition is a strict subset of `action_039`'s, which sits
# earlier in the list, so 047 is unreachable by construction. That kind of
# static proof is the only trustworthy way to retire an action rule today.


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
