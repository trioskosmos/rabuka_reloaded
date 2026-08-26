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
    parse_ability,
    _effect_registry,
    _condition_registry,
    _ACTION_REGISTRY,
)

CORPUS_PATH = os.path.join(
    os.path.dirname(__file__), "..", "..", "abilities.json"
)


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
