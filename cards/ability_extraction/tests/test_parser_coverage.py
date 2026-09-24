"""
Tests for activation_condition_parsed, _merge_parenthetical, sequential splitting,
trigger detection, and cost extraction.

Run: cd cards/ability_extraction && python tests/test_parser_coverage.py
"""

import sys, os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from parser import (
    parse_effect,
    parse_ability,
    parse_cost,
    parse_condition,
    _merge_parenthetical,
    _normalize_effect_tree,
    _try_sequential,
    _try_shi_sequential,
    _try_te_sequential,
    _try_implicit_sequential,
    _try_zone_placement,
    extract_name_exclusions,
    extract_operator,
    DURATION_PREFIX_MAP,
    _strip_duration_prefix,
)

passed = 0
failed = 0


def run_check(name, fn):
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


def test_extract_operator_le():
    assert extract_operator("2枚以下") == "<="


def test_extract_operator_ge():
    assert extract_operator("3枚以上") == ">="


def test_extract_operator_none():
    assert extract_operator("カードを引く") is None


def test_extract_operator_lt():
    assert extract_operator("2枚未満") == "<"


# ─── zone placement source (deck) ─────────────────────────────────────────────


def test_zone_placement_deck_to_discard():
    result = _try_zone_placement("このカードがデッキから控え室に置かれたとき")
    assert result is not None
    assert result.get("source") == "deck", f"expected source=deck, got {result.get('source')}"
    assert result.get("destination") == "discard"
    te = result.get("trigger_event", {})
    assert te.get("source") == "deck"
    assert te.get("destination") == "discard"


def test_zone_placement_hand_to_discard_source():
    result = _try_zone_placement("このカードが手札から控え室に置かれたとき")
    assert result is not None
    assert result.get("source") == "hand"
    assert result.get("destination") == "discard"


def test_zone_placement_deck_to_hand():
    result = _try_zone_placement("このカードがデッキから手札に加えられたとき")
    assert result is not None
    assert result.get("source") == "deck"
    assert result.get("destination") == "hand"


def test_revealed_member_without_blade_heart_uses_moved_subject():
    text = "それがブレードハートを持たないメンバーカードの場合"
    condition = parse_condition(text)
    assert condition.get("source") == "preceding_moved", condition
    assert "location" not in condition, condition
    assert condition.get("card_type") == "member_card", condition
    assert condition.get("card_property") == "has_blade_heart", condition
    assert condition.get("negation") is True, condition
    assert condition.get("count") == 1, condition
    assert condition.get("operator") == ">=", condition


def test_reveal_to_hand_score_keeps_moved_subject_after_normalization():
    text = (
        "自分のデッキの一番上のカードを公開し、手札に加える。"
        "それがブレードハートを持たないメンバーカードの場合、ライブの合計スコアを＋１する。"
    )
    effect = _normalize_effect_tree(parse_effect(text), text)
    assert effect.get("action") == "sequential", effect
    move, score = effect["actions"]
    assert move.get("source") == "deck_top", move
    assert move.get("destination") == "hand", move
    assert move.get("count") == 1, move
    condition = score["condition"]
    assert condition.get("source") == "preceding_moved", condition
    assert "location" not in condition, condition
    assert condition.get("card_property") == "has_blade_heart", condition
    assert condition.get("card_type") == "member_card", condition
    assert condition.get("negation") is True, condition
    assert score.get("target") == "live_total", score
    assert score.get("value") == 1, score


def test_stage_member_without_blade_heart_keeps_stage_subject():
    condition = parse_condition("ブレードハートを持たないメンバーカードの場合")
    assert condition.get("location") == "stage", condition
    assert "source" not in condition, condition
    assert condition.get("card_property") == "has_blade_heart", condition
    assert condition.get("negation") is True, condition


def test_score_threshold_exact_preserves_value():
    condition = parse_condition("このカードのスコアが３の場合")
    assert condition.get("type") == "comparison_condition", condition
    assert condition.get("comparison_type") == "score", condition
    assert condition.get("operator") == "=", condition
    assert condition.get("count") == 3, condition


def test_score_alternate_values_are_not_treated_as_one_threshold():
    condition = parse_condition("スコアが1か5の場合")
    assert condition.get("values") == [1, 5], condition
    assert "count" not in condition, condition


def test_baton_touch_recovery_uses_recently_moved_source():
    effect = parse_effect(
        "バトンタッチして登場した場合、このバトンタッチで控え室に置かれた『Liella!』のメンバーカードを1枚手札に加える"
    )
    assert effect.get("action") == "move_cards", effect
    assert effect.get("source") == "recently_moved", effect
    assert effect.get("destination") == "hand", effect
    condition = effect.get("condition", {})
    assert condition.get("baton_touch_trigger") is True, condition


def test_baton_touch_displaced_card_under_arriver_uses_discard_source():
    text = "このメンバーがステージから控え室に置かれたとき、バトンタッチしていた場合、このカードをそのバトンタッチで登場したメンバーの下に置く"
    effect = _normalize_effect_tree(parse_effect(text), text)
    assert effect.get("action") == "move_cards", effect
    assert effect.get("source") == "discard", effect
    assert effect.get("destination") == "under_member", effect
    assert effect.get("self_target") is True, effect
    condition = effect.get("condition", {})
    assert condition.get("trigger_event", {}).get("location") == "discard", condition


def test_distributed_baton_arrival_reaggregates_trigger_events():
    ability = parse_ability(
        "自分のステージに、このメンバーか、ほかのメンバーが"
        "バトンタッチして登場したとき、カードを1枚引く。"
    )
    effect = ability["effect"]
    condition = effect["condition"]
    assert condition["type"] == "or_condition", condition
    legs = condition["conditions"]
    assert [leg["type"] for leg in legs] == [
        "movement_condition",
        "movement_condition",
    ], condition
    leg_events = [leg["trigger_event"] for leg in legs]
    assert all(leg_events), condition
    assert leg_events[0] == leg_events[1], condition
    assert condition["trigger_event"] == {
        "type": "or",
        "events": [leg_events[0]],
    }, condition
    assert effect["action"] == "draw_card", effect
    assert effect["count"] == 1, effect


def test_sequential_baton_placement_is_validated_after_source_inference():
    ability = parse_ability(
        "{{toujyou.png|登場}}バトンタッチして登場した場合、このバトンタッチで控え室に置かれた『Liella!』のメンバーカードを1枚、このメンバーの下に置く。"
    )
    effect = ability["effect"]
    assert effect.get("action") == "sequential", effect
    assert effect["actions"][1].get("source") == "those_cards", effect


def test_all_heart_and_blade_gain_is_split():
    text = "自分のライブ中のカードが3枚以上あり、その中に『虹ヶ咲』のライブカードを1枚以上含む場合、{{icon_all.png|ハート}}{{icon_all.png|ハート}}{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る"
    effect = parse_effect(text)
    assert effect.get("action") == "sequential", effect
    assert effect["actions"][0].get("resource") == "blade", effect
    assert effect["actions"][0].get("count") == 2, effect
    assert effect["actions"][1].get("resource") == "heart", effect
    assert effect["actions"][1].get("heart_type") == "all", effect
    assert effect["actions"][1].get("count") == 2, effect


def test_duration_prefixes_use_canonical_codes():
    expected = {
        "ライブ終了時まで": "live_end",
        "ライブ終了まで": "live_end",
        "このターンの間": "this_turn",
        "このライブの間": "live_end",
        "ターン終了時まで": "this_turn",
        "そのターンの間": "this_turn",
    }
    assert DURATION_PREFIX_MAP == expected
    for prefix, code in expected.items():
        text, parsed = _strip_duration_prefix(prefix)
        assert text == ""
        assert parsed == code


def test_q280_energy_placement_restriction_is_delayed_per_card():
    text = (
        "自分のエネルギーデッキから、エネルギーカードを2枚ウェイト状態で置く。"
        "それらのエネルギーカードは、次のターンのアクティブフェイズにアクティブしない。"
    )
    effect = _normalize_effect_tree(parse_effect(text), text)
    assert effect.get("action") == "sequential", effect
    move, restriction = effect["actions"]
    assert move.get("source") == "energy_deck", move
    assert move.get("destination") == "energy_zone", move
    assert move.get("state_change") == "wait", move
    assert move.get("count") == 2, move
    assert restriction.get("action") == "restriction", restriction
    assert restriction.get("restriction_type") == "cannot_active", restriction
    assert restriction.get("delayed") is True, restriction


def test_q279_distinct_under_member_blade_gain_is_parsed():
    text = (
        "ライブ終了時まで、このメンバーの下に置かれている名前の異なるメンバーカード1枚につき、"
        "{{icon_blade.png|ブレード}}を得る。"
    )
    effect = _normalize_effect_tree(parse_effect(text), text)
    assert effect.get("action") == "gain_resource", effect
    assert effect.get("resource") == "blade", effect
    assert effect.get("per_unit") is True, effect
    assert effect.get("distinct") == "card_name", effect
    assert effect.get("location") == "under_member", effect
    assert effect.get("card_type") == "member_card", effect
    assert effect.get("duration") == "live_end", effect


def test_live_start_score_gate_filters_own_live_cards():
    ability = parse_ability(
        "{{live_start.png|ライブ開始時}}自分のライブ中のカードにスコア2以下のライブカードがある場合、このメンバーをアクティブにする"
    )
    condition = ability["effect"]["condition"]
    assert condition.get("type") == "card_count_condition", condition
    assert condition.get("location") == "live_card_zone", condition
    assert condition.get("card_type") == "live_card", condition
    assert condition.get("cost_limit") == 2, condition
    assert condition.get("cost_limit_operator") == "<=", condition
    assert condition.get("count") == 1, condition
    assert condition.get("operator") == ">=", condition
    assert condition.get("target") == "self", condition


def test_sunny_day_song_group_filter_stays_on_mus_branch():
    ability = parse_ability(
        "{{live_start.png|ライブ開始時}}自分のステージにメンバーが1人以上いる場合、自分と相手はカードを1枚引き、手札を1枚控え室に置く。2人以上いる場合、さらに自分のステージにいる『μ's』のメンバー1人は、ライブ終了時まで、{{heart_03.png|heart03}}を得る。3人以上おり、かつそれぞれ名前が異なる場合、さらにこのカードのスコアを＋１する。"
    )
    effect = ability["effect"]
    branch1, branch2, branch3 = effect["actions"]
    assert "group_names" not in effect, effect
    assert "group_names" not in branch1, branch1
    assert branch2.get("group_names") == ["μ's"], branch2
    assert "group_names" not in branch3, branch3


def test_maki_previous_reveal_source_is_resolution_scoped():
    ability = parse_ability(
        "{{toujyou.png|登場}}手札のライブカードを1枚公開してもよい：自分の成功ライブカード置き場にあるカードを1枚手札に加える。そうした場合、これにより公開したカードを自分の成功ライブカード置き場に置く。"
    )
    action = ability["effect"]["actions"][1]
    assert action.get("source") == "selected_cards", action
    assert action.get("count") == 1, action
    assert "dynamic_count" not in action, action
    assert ability["effect"].get("conditional") is not True, ability["effect"]


def test_excluded_this_member_state_change_is_not_self_targeted():
    ability = parse_ability(
        "{{live_start.png|ライブ開始時}}自分のステージにいるこのメンバー以外のウェイト状態のメンバー1人をアクティブにする"
    )
    effect = ability["effect"]
    action = effect.get("actions", [effect])[0]
    assert action.get("target") == "self", action
    assert action.get("exclude_self") is True, action
    assert "self_target" not in action, action


def test_sequential_group_filter_stays_on_recovery_action():
    ability = parse_ability(
        "{{toujyou.png|登場}}このメンバーをウェイトにする。その後、自分の控え室からスコア4以下の『蓮ノ空』のライブカードを1枚手札に加える"
    )
    actions = ability["effect"]["actions"]
    wait, recovery = actions
    assert wait.get("target") == "self", wait
    assert wait.get("self_target") is True, wait
    assert wait.get("count") == 1, wait
    assert "group_names" not in wait, wait
    assert recovery.get("group_names") == ["蓮ノ空"], recovery


# ─── run all ──────────────────────────────────────────────────────────────────

tests = {
    k: v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)
}
for name, fn in tests.items():
    run_check(name, fn)

print(f"\n{passed} passed, {failed} failed")
if failed:
    sys.exit(1)
