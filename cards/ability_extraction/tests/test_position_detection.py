"""
tests/test_position_detection.py

Tests for position detection utilities: detect_positions(), detect_icon_positions(),
set_cross_position_fields(), and activation_position output from parse_effect().

Catches:
- Missing positions in detection
- Comma-separated activation_position (multi-position OR)
- Spurious "position" field on effects with multi-position activation
- Icon template detection (leftside/rightside/center)

Run: python -m pytest cards/ability_extraction/tests/test_position_detection.py -v
"""

import sys, os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from parser import (
    detect_positions,
    detect_icon_positions,
    set_cross_position_fields,
    parse_effect,
    _normalize_effect_tree,
)


class TestDetectPositions:
    def test_center_only(self):
        assert detect_positions("センターエリアにいるメンバー") == ["center"]

    def test_left_only(self):
        assert detect_positions("左サイドエリアに移動") == ["left_side"]

    def test_right_only(self):
        assert detect_positions("右サイドエリアに登場") == ["right_side"]

    def test_left_and_right(self):
        result = detect_positions(
            "左サイドエリアか右サイドエリアに登場した場合のみ発動する"
        )
        assert "left_side" in result
        assert "right_side" in result

    def test_all_three(self):
        result = detect_positions("センターエリア、左サイドエリア、右サイドエリア")
        assert "center" in result
        assert "left_side" in result
        assert "right_side" in result

    def test_no_positions(self):
        assert detect_positions("カードを2枚引く") == []

    def test_short_keywords(self):
        result = detect_positions("センターにいるメンバー")
        assert "center" in result

    def test_front_keyword(self):
        result = detect_positions("正面のメンバー")
        assert "front" in result

    def test_deduplication(self):
        result = detect_positions("左サイドエリアの左サイドメンバー")
        assert result.count("left_side") == 1


class TestDetectIconPositions:
    def test_center_icon(self):
        assert detect_icon_positions("{{center.png|センター}}カードを得る") == [
            "center"
        ]

    def test_leftside_icon(self):
        assert detect_icon_positions("{{leftside.png|左サイド}}カードを引く") == [
            "left_side"
        ]

    def test_rightside_icon(self):
        assert detect_icon_positions("{{rightside.png|右サイド}}ハートを得る") == [
            "right_side"
        ]

    def test_left_and_right_icons(self):
        result = detect_icon_positions(
            "{{leftside.png|左サイド}}{{rightside.png|右サイド}}カードを2枚引く"
        )
        assert "left_side" in result
        assert "right_side" in result

    def test_no_icons(self):
        assert detect_icon_positions("カードを2枚引く") == []

    def test_mixed_with_text(self):
        result = detect_icon_positions(
            "{{toujyou.png|登場}}{{leftside.png|左サイド}}{{rightside.png|右サイド}}効果"
        )
        assert "left_side" in result
        assert "right_side" in result
        assert len(result) == 2


class TestSetCrossPositionFields:
    def test_single_position(self):
        target = {}
        set_cross_position_fields(target, "センターエリアにいるメンバー")
        assert target["position"] == "center"
        assert "position_compare" not in target

    def test_left_and_right(self):
        target = {}
        set_cross_position_fields(target, "左サイドエリアと右サイドエリアにいる")
        assert target["position"] == "left_side"
        assert target["position_compare"] == "right_side"

    def test_no_positions(self):
        target = {}
        result = set_cross_position_fields(target, "カードを2枚引く")
        assert result is False
        assert "position" not in target

    def test_does_not_overwrite_existing(self):
        target = {"position": "center"}
        set_cross_position_fields(target, "左サイドエリアと右サイドエリアにいる")
        assert target["position"] == "center"


class TestParseEffectActivationPosition:
    """Verify that parse_effect produces correct activation_position and
    no spurious position field on multi-position effects."""

    def test_left_right_activation(self):
        text = (
            "{{leftside.png|左サイド}}{{rightside.png|右サイド}}"
            "カードを2枚引き、手札を2枚控え室に置く。"
            "（この能力は左サイドエリアか右サイドエリアに登場した場合のみ発動する。）"
        )
        effect = parse_effect(text)
        effect = _normalize_effect_tree(effect, text)
        assert effect.get("activation_position") == "left_side,right_side"

    def test_left_right_no_spurious_position(self):
        """Multi-position should NOT set 'position' field — only activation_position."""
        text = (
            "{{leftside.png|左サイド}}{{rightside.png|右サイド}}"
            "カードを2枚引き、手札を2枚控え室に置く。"
            "（この能力は左サイドエリアか右サイドエリアに登場した場合のみ発動する。）"
        )
        effect = parse_effect(text)
        effect = _normalize_effect_tree(effect, text)
        assert "position" not in effect, (
            f"effect should not have 'position' field for multi-position activation, "
            f"got position={effect.get('position')}"
        )

    def test_center_only_activation(self):
        text = "{{center.png|センター}}{{icon_blade.png|ブレード}}を得る。"
        effect = parse_effect(text)
        effect = _normalize_effect_tree(effect, text)
        assert effect.get("activation_position") == "center"

    def test_left_only_activation(self):
        text = "{{leftside.png|左サイド}}カードを2枚引く。"
        effect = parse_effect(text)
        effect = _normalize_effect_tree(effect, text)
        assert effect.get("activation_position") == "left_side"

    def test_sub_action_activation_position(self):
        """Sub-actions of a multi-position sequential effect should have activation_position."""
        text = (
            "{{leftside.png|左サイド}}{{rightside.png|右サイド}}"
            "カードを2枚引き、手札を2枚控え室に置く。"
            "（この能力は左サイドエリアか右サイドエリアに登場した場合のみ発動する。）"
        )
        effect = parse_effect(text)
        effect = _normalize_effect_tree(effect, text)
        for action in effect.get("actions", []):
            assert action.get("activation_position") == "left_side,right_side", (
                f"Sub-action '{action.get('text')}' should have activation_position "
                f"'left_side,right_side', got {action.get('activation_position')}"
            )


if __name__ == "__main__":
    import pytest

    sys.exit(pytest.main([__file__, "-v"]))
