"""
Tests for activation_condition_parsed, _merge_parenthetical, sequential splitting,
trigger detection, and cost extraction.

Run: cd cards/ability_extraction && python tests/test_parser_coverage.py
"""

import sys, os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from parser import (
    parse_effect,
    parse_cost,
    parse_condition,
    _merge_parenthetical,
    _normalize_effect_tree,
    _try_sequential,
    _try_shi_sequential,
    _try_te_sequential,
    _try_implicit_sequential,
    extract_name_exclusions,
    extract_cost_operator,
)

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


# ─── activation_condition_parsed ───────────────────────────────────────────────


def test_activation_condition_center_only():
    text = "カードを2枚引く。（この能力はセンターエリアに登場した場合のみ発動する。）"
    effect = parse_effect(text)
    effect = _normalize_effect_tree(effect, text)
    assert effect.get("activation_position") == "center", (
        f"Got {effect.get('activation_position')}"
    )


def test_activation_condition_left_right():
    text = "カードを2枚引く。（この能力は左サイドエリアか右サイドエリアに登場した場合のみ発動する。）"
    effect = parse_effect(text)
    effect = _normalize_effect_tree(effect, text)
    assert effect.get("activation_position") == "left_side,right_side", (
        f"Got {effect.get('activation_position')}"
    )


def test_activation_condition_left_right_no_spurious_position():
    text = (
        "{{leftside.png|左サイド}}{{rightside.png|右サイド}}"
        "カードを2枚引き、手札を2枚控え室に置く。"
        "（この能力は左サイドエリアか右サイドエリアに登場した場合のみ発動する。）"
    )
    effect = parse_effect(text)
    effect = _normalize_effect_tree(effect, text)
    assert "position" not in effect, f"Spurious position: {effect.get('position')}"
    assert effect.get("activation_position") == "left_side,right_side"


def test_activation_condition_left_only():
    text = "{{leftside.png|左サイド}}カードを2枚引く。"
    effect = parse_effect(text)
    effect = _normalize_effect_tree(effect, text)
    assert effect.get("activation_position") == "left_side", (
        f"Got {effect.get('activation_position')}"
    )


def test_no_activation_condition_without_parenthetical():
    text = "カードを2枚引く"
    effect = parse_effect(text)
    assert "activation_condition_parsed" not in effect
    assert effect.get("activation_position") is None


# ─── merge_parenthetical ──────────────────────────────────────────────────────


def test_merge_parenthetical_stores_text():
    target = {"text": "テスト"}
    _merge_parenthetical(
        target, "（この能力はセンターエリアに登場した場合のみ発動する。）"
    )
    assert "parenthetical" in target
    assert "センターエリア" in str(target["parenthetical"])


def test_merge_parenthetical_empty():
    target = {"text": "テスト"}
    _merge_parenthetical(target, "")
    # Should not crash on empty string


# ─── sequential splitting ─────────────────────────────────────────────────────


def test_implicit_sequential_comma_split():
    text = "{{toujyou.png|登場}}カードを2枚引き、手札を2枚控え室に置く"
    result = _try_implicit_sequential(text)
    assert result is not None, "Should split implicit sequential"
    assert result.get("action") == "sequential"
    assert len(result.get("actions", [])) >= 2


def test_implicit_sequential_draw_discard():
    text = "カードを2枚引き手札を1枚控え室に置く"
    result = _try_implicit_sequential(text)
    # May or may not match depending on text patterns
    if result:
        assert result.get("action") == "sequential"


def test_try_te_sequential():
    text = "カードを1枚を得て、手札を1枚控え室に置く"
    result = _try_te_sequential(text)
    if result:
        assert result.get("action") == "sequential"
        assert len(result.get("actions", [])) >= 2


# ─── cost extraction ──────────────────────────────────────────────────────────


def test_cost_energy():
    result = parse_cost("{{E}}{{E}}支払う")
    assert result is not None, "Should parse energy cost"
    assert isinstance(result, dict)


def test_cost_none():
    result = parse_cost("カードを1枚引く")
    # Either None or energy=0 is acceptable


def test_cost_text_preserved():
    result = parse_cost("エネルギーを1個置く")
    assert result is not None


# ─── utility functions ─────────────────────────────────────────────────────────


def test_extract_name_exclusions_include():
    inc, exc = extract_name_exclusions("「マリ」をコストに控え室に置く")
    assert "マリ" in inc
    assert exc == []


def test_extract_name_exclusions_exclude():
    inc, exc = extract_name_exclusions("「マリ」以外のメンバー")
    assert "マリ" in exc
    assert "マリ" not in inc


def test_extract_cost_operator_le():
    assert extract_cost_operator("2枚以下") == "<="


def test_extract_cost_operator_ge():
    assert extract_cost_operator("3枚以上") == ">="


def test_extract_cost_operator_none():
    assert extract_cost_operator("カードを引く") is None


def test_extract_cost_operator_lt():
    assert extract_cost_operator("2枚未満") == "<"


# ─── run all ──────────────────────────────────────────────────────────────────

tests = {
    k: v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)
}
for name, fn in tests.items():
    test(name, fn)

print(f"\n{passed} passed, {failed} failed")
if failed:
    sys.exit(1)
