"""Effect parsing using a flat pattern registry."""

import re
from typing import Dict, Any, Optional, List, Tuple

from test_parser.fields import (
    ExtractedFields,
    normalize_fullwidth_digits,
    strip_parenthetical as sp,
)
from test_parser.structure import split_condition_action, strip_duration_prefix
from test_parser.condition import parse_condition
from test_parser.schema import (
    POSITION_KEYWORDS,
    DESTINATION_PATTERNS,
    SOURCE_PATTERNS,
)


_EFFECT_HANDLERS: List[Tuple[int, Any]] = []


def register(priority: int = 0):
    def wrapper(func):
        _EFFECT_HANDLERS.append((priority, func))
        _EFFECT_HANDLERS.sort(key=lambda x: -x[0])
        return func

    return wrapper


def parse_effect(text: str) -> Dict[str, Any]:
    text = normalize_fullwidth_digits(text.strip())
    parens_raw = re.findall(r"（[^）]*）", text)
    text = sp(text)
    text = text.rstrip("。")
    f = ExtractedFields(text)
    effect: Dict[str, Any] = {"text": text}
    for _, handler in _EFFECT_HANDLERS:
        result = handler(text, f)
        if result is not None:
            effect = result
            if f.heart_colors and "heart_colors" not in effect:
                effect["heart_colors"] = f.heart_colors
            break
    else:
        action = _parse_simple_action(text, f)
        effect.update(action)
    if parens_raw:
        effect["parenthetical"] = [p.strip("（）").strip() for p in parens_raw]
    # Extract activation_position from text
    if "activation_position" not in effect:
        if "{{center.png|センター}}" in text:
            effect["activation_position"] = "center"
        elif "{{left.png|左サイド}}" in text:
            effect["activation_position"] = "left_side"
        elif "{{right.png|右サイド}}" in text:
            effect["activation_position"] = "right_side"
    _post_process(effect, text)
    return effect


def _parse_simple_action(
    text: str, f: Optional[ExtractedFields] = None
) -> Dict[str, Any]:
    if f is None:
        f = ExtractedFields(text)
    action = _dispatch_action(text, f)
    _fill_common_action_fields(action, text)
    return action


def _infer_card_type(text: str, action: Dict[str, Any] = None) -> Optional[str]:
    if "エネルギーカード" in text:
        return "energy_card"
    if "メンバーカード" in text and "ライブカード" in text:
        return "card"
    if "メンバーカード" in text or (
        "メンバー" in text and "エネルギー" not in text and "置き場" not in text
    ):
        return "member_card"
    if "ライブカード" in text:
        return "live_card"
    if "エネルギー" in text:
        return "energy_card"
    if "カード" in text:
        return "card"
    if action and action.get("source") == "stage":
        return "member_card"
    if action and action.get("source") in (
        "deck",
        "deck_top",
        "hand",
        "discard",
        "revealed_cards",
        "revealed_remaining",
    ):
        if action.get("action") == "move_cards":
            return "card"
    return None


def _fill_common_action_fields(action: Dict[str, Any], text: str):
    f = ExtractedFields(text)
    a = action.get("action", "")

    if f.count is not None and "count" not in action and "dynamic_count" not in action:
        action["count"] = f.count
    if f.target and "target" not in action:
        action["target"] = f.target
    if f.optional and "optional" not in action:
        action["optional"] = True
    if f.heart_colors and "heart_colors" not in action:
        action["heart_colors"] = f.heart_colors
    # source: only set for types that have source
    if (
        f.source
        and "source" not in action
        and a
        in ("move_cards", "look_at", "reveal", "select", "draw_card", "change_state")
    ):
        action["source"] = f.source
    # destination
    if (
        f.destination
        and "destination" not in action
        and a in ("move_cards", "draw_card")
    ):
        action["destination"] = f.destination
    # card_type: only for action types that have card_type
    if (
        f.card_type
        and "card_type" not in action
        and a
        in (
            "move_cards",
            "change_state",
            "gain_resource",
            "select",
            "reveal",
            "discard_until_count",
        )
    ):
        action["card_type"] = f.card_type
    elif "card_type" not in action and a in ("move_cards",) and not f.card_type:
        src = action.get("source", "")
        has_explicit_type = any(
            ct in text
            for ct in ["メンバー", "ライブ", "エネルギー", "スタンド", "ウェイト"]
        )
        if "カード" in text and not has_explicit_type:
            action["card_type"] = "card"
        elif not has_explicit_type and src in (
            "hand",
            "deck",
            "deck_top",
            "deck_bottom",
            "discard",
            "revealed_cards",
            "under_member",
        ):
            action["card_type"] = "card"
    # group_names
    if f.group_names and "group_names" not in action:
        action["group_names"] = f.group_names
    self_target_types = (
        "move_cards",
        "gain_resource",
        "change_state",
        "select",
        "modify_score",
        "modify_required_hearts",
        "set_heart_type",
        "gain_ability",
    )
    if f.self_target and "self_target" not in action and a in self_target_types:
        action["self_target"] = True
    if f.exclude_self and "exclude_self" not in action:
        action["exclude_self"] = True
    if (
        f.position
        and "position" not in action
        and "source_position" not in action
        and "exclude_position" not in action
    ):
        action["position"] = f.position
    # cost_limit
    if f.cost_limit is not None and "cost_limit" not in action:
        action["cost_limit"] = f.cost_limit
        if f.cost_limit_operator:
            action["cost_limit_operator"] = f.cost_limit_operator
    # duration: only for action types that should have duration
    duration_types = (
        "gain_resource",
        "modify_score",
        "change_state",
        "set_blade_count",
        "modify_required_hearts",
    )
    if "duration" not in action and a in duration_types:
        for kw, code in [
            ("ライブ終了時まで", "live_end"),
            ("ライブ終了まで", "live_end"),
            ("このターンの間", "this_turn"),
            ("このライブの間", "this_live"),
            ("ターン終了時まで", "turn_end"),
            ("そのターンの間", "turn_end"),
        ]:
            if kw in text:
                action["duration"] = code
                break
    # default count for draw_card
    if a == "draw_card" and "count" not in action:
        action["count"] = 1
    # all
    if f.all and "all" not in action:
        action["all"] = True
    # multiple_targets
    if f.multiple_targets and "multiple_targets" not in action:
        action["multiple_targets"] = True
    # any_number
    if f.any_number and "any_number" not in action:
        action["any_number"] = True
        action.pop("count", None)
    # max
    if f.max and "max" not in action:
        action["max"] = True
    # non_stackable
    if f.non_stackable and "non_stackable" not in action:
        action["non_stackable"] = True
    # placement_order
    if f.placement_order and "placement_order" not in action:
        action["placement_order"] = f.placement_order
    # state_change
    if f.state_change and "state_change" not in action:
        action["state_change"] = f.state_change
    # shuffle
    if f.shuffle and "shuffle" not in action:
        action["shuffle"] = True
    # original_value
    if f.original_value and "original_value" not in action:
        action["original_value"] = True
    # blade_limit
    if f.blade_limit is not None and "blade_limit" not in action:
        action["blade_limit"] = f.blade_limit
        if f.blade_limit_operator:
            action["blade_limit_operator"] = f.blade_limit_operator


def _dispatch_action(text: str, f: ExtractedFields) -> Dict[str, Any]:
    action: Dict[str, Any] = {"text": text}
    f2 = ExtractedFields(text)

    def has(t: str) -> bool:
        return t in text

    if has("カードを1枚引いてもよい"):
        action["action"] = "draw_card"
        action["count"] = 1
        action["optional"] = True
        action["source"] = "deck"
        action["destination"] = "hand"
        return action

    if (
        has("シャッフルする")
        or has("シャッフルして")
        or (has("シャッフルし") and has("、"))
    ):
        action["action"] = "shuffle"
        action["target"] = "deck" if "デッキ" in text else "energy_deck"
        return action

    if has("入れ替える") or has("入れ替えて"):
        action["action"] = "position_change"
        return action
    if has("フォーメーションチェンジ"):
        action["action"] = "position_change"
        action["optional"] = f2.optional
        action["multiple_targets"] = True
        return action
    if has("ポジションチェンジ"):
        action["action"] = "position_change"
        if f2.target:
            action["target"] = f2.target
        if "正面" in text:
            action["destination"] = "front"
        if "メンバー" in text and ("1人" in text or "N人" in text):
            action["target_member"] = "select"
        return action
    if has("移動させ") and "エリア" in text:
        action["action"] = "position_change"
        return action
    if has("移動させ") and "エリア" not in text:
        action["action"] = "move_cards"
        return action
    if has("移動する") or has("移動し"):
        action["action"] = "position_change"
        return action

    if (
        has("{{icon_energy.png|E}}")
        and (has("支払う") or has("支払って"))
        and "選び" not in text
    ):
        action["action"] = "pay_energy"
        action["energy"] = text.count("{{icon_energy.png|E}}")
        action["optional"] = "もよい" in text or "してもよい" in text
        return action

    if has("引く") or has("引き") or has("引い"):
        action["action"] = "draw_card"
        action["source"] = "deck"
        action["destination"] = "hand"
        return action
    if has("引いてもよい"):
        action["action"] = "draw_card"
        action["source"] = "deck"
        action["destination"] = "hand"
        action["optional"] = True
        return action

    if has("枚になるまで") and has("引く"):
        m = re.search(r"(\d+)枚になるまで", text)
        if m:
            action["action"] = "draw_until_count"
            action["source"] = "deck"
            action["destination"] = "hand"
            action["target_count"] = int(m.group(1))
            return action
    if has("枚になるまで") and (has("控え室に置く") or has("控え室に置き")):
        m = re.search(r"(\d+)枚になるまで", text)
        if m:
            action["action"] = "discard_until_count"
            action["target_count"] = int(m.group(1))
            return action

    if re.search(r"コスト[はが](\d+)(減る|減らす|増える|増やす)", text):
        action["action"] = "modify_cost"
        _handle_cost_mod(action, text, f2)
        return action

    if f2.source and f2.destination:
        action["action"] = "move_cards"
        action["source"] = f2.source
        action["destination"] = f2.destination
        _fill_move_card_fields(action, text, f2)
        return action

    if f2.state_change:
        action["action"] = "change_state"
        action["state_change"] = f2.state_change
        if f2.target:
            action["target"] = f2.target
        if f2.source:
            action["source"] = f2.source
        if "source" not in action:
            action["source"] = "stage"
        if "エネルギー" in text and "メンバー" not in text:
            action["card_type"] = "energy_card"
        if "このメンバー" in text or (
            "メンバー" in text and ("ウェイト" in text or "アクティブ" in text)
        ):
            action["card_type"] = "member_card"
        return action

    if has("のみ起動できる") or has("のみ発動する"):
        action["action"] = "activation_restriction"
        action["restriction_type"] = "only"
        return action

    if has("置くことができない"):
        action["action"] = "restriction"
        action["restriction_type"] = "cannot_place"
        action["destination"] = _extract_place_dest(text)
        return action
    if has("置けない") and "置くことができない" not in text:
        action["action"] = "restriction"
        action["restriction_type"] = "cannot_place"
        action["destination"] = _extract_place_dest(text)
        return action
    if has("登場できない"):
        action["action"] = "restriction"
        action["restriction_type"] = "cannot_appear"
        return action
    if has("移動できない"):
        action["action"] = "restriction"
        action["restriction_type"] = "cannot_move"
        return action
    if has("ライブできない"):
        action["action"] = "restriction"
        action["restriction_type"] = "cannot_live"
        return action
    if has("アクティブにしない"):
        action["action"] = "restriction"
        action["restriction_type"] = "cannot_activate"
        return action

    if has("{{icon_blade.png|ブレード}}") and has("{{heart") and has("得る"):
        blade_count = text.count("{{icon_blade.png|ブレード}}")
        heart_matches = re.findall(r"heart_(\d+)", text)
        actions = []
        if blade_count:
            actions.append(
                {"action": "gain_resource", "resource": "blade", "count": blade_count}
            )
        if heart_matches:
            colors = sorted(set(f"heart{m.zfill(2)}" for m in heart_matches))
            actions.append(
                {
                    "action": "gain_resource",
                    "resource": "heart",
                    "heart_colors": colors,
                    "count": len(heart_matches),
                }
            )
        if actions:
            action["action"] = "sequential"
            action["actions"] = actions
            return action

    if has("ブレードを得る") or has("選んだブレード"):
        action["action"] = "gain_resource"
        action["resource"] = "blade"
        bc = text.count("{{icon_blade.png|ブレード}}")
        if bc:
            action["count"] = bc
        _fill_gain_resource_fields(action, text, f2)
        return action
    if has("{{icon_blade.png|ブレード}}") and has("得る"):
        action["action"] = "gain_resource"
        action["resource"] = "blade"
        action["count"] = text.count("{{icon_blade.png|ブレード}}") or None
        return action

    if (
        (has("{{heart") and has("得る"))
        or bool(re.search(r"ハート.*得る", text))
        or (has("選んだハート") and "になる" not in text)
    ):
        action["action"] = "gain_resource"
        action["resource"] = "heart"
        _fill_gain_resource_fields(action, text, f2)
        return action

    if has("{{icon_all.png|ハート}}") and has("得る"):
        action["action"] = "gain_resource"
        action["resource"] = "heart"
        action["heart_type"] = "all"
        action["count"] = text.count("{{icon_all.png|ハート}}") or None
        return action

    if has("を失う") or has("をすべて失う"):
        action["action"] = "gain_resource"
        action["sign"] = "negative"
        action["resource"] = "surplus_heart" if "余剰ハート" in text else "heart"
        action["all"] = "すべて" in text or None
        return action

    if has("加える") or has("加え"):
        action["action"] = "move_cards"
        action["destination"] = "hand"
        _fill_move_card_fields(action, text, f2)
        return action

    if (has("置く") or has("置いて")) or (has("置き") and "置き場" not in text):
        action["action"] = "move_cards"
        if "destination" not in action:
            d = f2.destination
            if d:
                action["destination"] = d
        _fill_move_card_fields(action, text, f2)
        return action

    if has("もう一度エール") or has("もう1度エール"):
        action["action"] = "re_yell"
        if "できない" not in text:
            action["lose_blade_hearts"] = True
        return action

    if has("見る") or has("見て") or text.endswith("見"):
        action["action"] = "look_at"
        if "デッキの上" in text:
            action["source"] = "deck_top"
        if f2.count is not None:
            action["count"] = f2.count
        if f2.target:
            action["target"] = f2.target
        return action

    if has("公開する") or has("公開して"):
        action["action"] = "reveal"
        action["source"] = f2.source or "hand"
        if "見ないで" in text:
            action["blind"] = True
        return action

    if has("選ぶ") or has("選び") or bool(re.search(r"選ん(?!だ)", text)):
        action["action"] = "select"
        return action

    if has("指定する") or has("指定し") or has("指定して"):
        if has("ハート"):
            action["action"] = "gain_resource"
            action["resource"] = "heart"
            action["heart_selection"] = True
            return action
        action["action"] = "select"
        return action

    if has("登場させ"):
        action["action"] = "move_cards"
        action["destination"] = "stage"
        _fill_move_card_fields(action, text, f2)
        return action

    if has("起動でき") or has("起動して"):
        action["action"] = "activate_ability"
        return action
    if has("無効に"):
        action["action"] = "invalidate_ability"
        return action

    if has("必要ハート") or has("ハートを増やす") or has("ハートを減らす"):
        action["action"] = "modify_required_hearts"
        return action

    if has("追加"):
        action["action"] = "modify_score"
        action["operation"] = "add"
        return action
    if has("スコアを1プラス") or has("スコアをプラス"):
        action["action"] = "modify_score"
        action["operation"] = "add"
        action["value"] = 1
        return action
    if has("スコアを1マイナス"):
        action["action"] = "modify_score"
        action["operation"] = "remove"
        action["value"] = 1
        return action
    if has("スコアを"):
        action["action"] = "modify_score"
        _set_score_op(action, text)
        return action

    if has("以下から1つを選ぶ"):
        action["action"] = "choice"
        return action

    if has("バトンタッチ") or "baton touch" in text.lower():
        action["action"] = "play_baton_touch"
        return action

    if has("何もしない") or text.strip() == "":
        action["action"] = "do_nothing"
        return action

    if has("コストを") or has("コストが") or has("コストは"):
        action["action"] = "modify_cost"
        _handle_cost_mod(action, text, f2)
        return action

    if has("繰り返してもよい"):
        action["action"] = "repeat_procedure"
        m = re.search(r"(\d+)回", text)
        if m:
            action["max_repeats"] = int(m.group(1))
        return action

    if has("支払って発動させる"):
        action["action"] = "activate_ability"
        action["activation_type"] = "pay_to_activate"
        return action

    action["action"] = "custom"
    _fill_defaults(action, text, f2)
    return action


def _fill_move_card_fields(action: Dict[str, Any], text: str, f: ExtractedFields):
    if "source" not in action:
        src = f.source
        if src:
            action["source"] = src
        elif "手札を" in text and "控え室に置く" in text:
            action["source"] = "hand"
        elif "下に置かれているエネルギーカード" in text:
            action["source"] = "under_member"
            action["card_type"] = "energy_card"
    if "destination" not in action:
        dst = f.destination
        if dst:
            action["destination"] = dst
    if "card_type" not in action and f.card_type:
        action["card_type"] = f.card_type
    if "count" not in action and f.count is not None:
        action["count"] = f.count
    elif "count" not in action and "dynamic_count" not in action:
        action["count"] = 1
    if f.target and "target" not in action:
        action["target"] = f.target
    if f.optional and "optional" not in action:
        action["optional"] = True
    if f.state_change and "state_change" not in action:
        action["state_change"] = f.state_change
    if f.multiple_targets and "multiple_targets" not in action:
        action["multiple_targets"] = True
    if f.exclude_self and "exclude_self" not in action:
        action["exclude_self"] = True
    if f.group_names and "group_names" not in action:
        action["group_names"] = f.group_names
    if f.placement_order and "placement_order" not in action:
        action["placement_order"] = f.placement_order
    if f.max and "max" not in action:
        action["max"] = True
    if f.any_number and "any_number" not in action:
        action["any_number"] = True
        action.pop("count", None)
    if f.position and "position" not in action:
        action["position"] = f.position
    if f.self_target and "self_target" not in action:
        action["self_target"] = True
    if f.all and "all" not in action:
        action["all"] = True
        action.pop("count", None)
    if "好きな順番で" in text:
        action["placement_order"] = "any_order"


def _fill_gain_resource_fields(action: Dict[str, Any], text: str, f: ExtractedFields):
    if action.get("resource") == "blade":
        bc = text.count("{{icon_blade.png|ブレード}}")
        if bc and "count" not in action:
            action["count"] = bc
    if f.heart_colors and "heart_colors" not in action:
        action["heart_colors"] = f.heart_colors
    if "count" not in action:
        hc = text.count("{{icon_blade.png|ブレード}}")
        if hc == 0:
            hc = len(re.findall(r"heart_(\d+)", text))
        if hc > 0:
            action["count"] = hc
        elif f.count is not None:
            action["count"] = f.count
        else:
            action["count"] = 1
    if f.target and "target" not in action:
        action["target"] = f.target
    if f.optional and "optional" not in action:
        action["optional"] = True
    if f.multiple_targets and "multiple_targets" not in action:
        action["multiple_targets"] = True
    if f.group_names and "group_names" not in action:
        action["group_names"] = f.group_names
    if f.position and "position" not in action:
        action["position"] = f.position


def _handle_cost_mod(action: Dict[str, Any], text: str, f: ExtractedFields):
    if "減る" in text or "減らす" in text or "マイナス" in text:
        action["operation"] = "subtract"
    elif (
        "増える" in text or "増やす" in text or "プラス" in text or "コストを+" in text
    ):
        action["operation"] = "add"
    if "手札" in text:
        action["location"] = "hand"
    if f.cost_limit is not None:
        action["cost_limit"] = f.cost_limit
        if f.cost_limit_operator:
            action["cost_limit_operator"] = f.cost_limit_operator
    vm = re.search(r"コスト[はがを](\d+)(減る|減らす|増える|増やす)", text)
    if vm:
        action["value"] = int(vm.group(1))
    else:
        vm2 = re.search(r"コスト[をがは][+＋](\d+)", text)
        if vm2:
            action["value"] = int(vm2.group(1))
    if f.energy_count:
        action["count"] = f.energy_count


def _set_score_op(action: Dict[str, Any], text: str):
    sm = re.search(r"([+\-])(\d+)", text)
    if sm:
        action["value"] = int(sm.group(2))
        action["operation"] = "remove" if sm.group(1) == "-" else "add"
        return
    cnt_match = re.search(r"(\d+)", text)
    if cnt_match:
        action["value"] = int(cnt_match.group(1))
        if "マイナス" in text or "減らす" in text or "減る" in text:
            action["operation"] = "remove"
        else:
            action["operation"] = "add"
        return
    if "プラス" in text or "増やす" in text or "増える" in text:
        action["operation"] = "add"
    elif "マイナス" in text or "減らす" in text or "減る" in text:
        action["operation"] = "remove"


def _extract_place_dest(text: str) -> Optional[str]:
    if "成功ライブカード置き場" in text:
        return "success_live_zone"
    if "ライブカード置き場" in text:
        return "live_card_zone"
    if "控え室" in text:
        return "discard"
    if "手札" in text:
        return "hand"
    if "エネルギー置き場" in text:
        return "energy_zone"
    if "ステージ" in text:
        return "stage"
    return None


def _fill_defaults(action: Dict[str, Any], text: str, f: ExtractedFields):
    if f.source and "source" not in action:
        action["source"] = f.source
    if f.destination and "destination" not in action:
        action["destination"] = f.destination
    if f.card_type and "card_type" not in action:
        action["card_type"] = f.card_type
    if f.count is not None and "count" not in action and "dynamic_count" not in action:
        if action.get("action") in (
            "move_cards",
            "draw_card",
            "gain_resource",
            "reveal",
            "look_at",
            "change_state",
        ):
            action["count"] = f.count
    if f.target and "target" not in action:
        action["target"] = f.target
    if f.optional and "optional" not in action:
        action["optional"] = True
    if f.self_target and "self_target" not in action:
        a2 = action.get("action", "")
        if a2 in (
            "move_cards",
            "gain_resource",
            "change_state",
            "select",
            "modify_score",
            "modify_required_hearts",
            "set_heart_type",
            "gain_ability",
        ):
            action["self_target"] = True
    if f.exclude_self and "exclude_self" not in action:
        action["exclude_self"] = True
    if f.group_names and "group_names" not in action:
        action["group_names"] = f.group_names
    if f.placement_order and "placement_order" not in action:
        action["placement_order"] = f.placement_order
    if f.max and "max" not in action:
        action["max"] = True
    if f.any_number and "any_number" not in action:
        action["any_number"] = True


def _post_process(effect: Dict[str, Any], original_text: str):
    pass


# ===================== COMPLEX EFFECT HANDLERS =====================


@register(100)
def _try_per_unit(text, f):
    if "につき" not in text and "ごとに" not in text:
        return None
    excludes = [
        "各グループ名につき",
        "グループ名につき",
        "グループ名",
        "この能力を起動するためのコストは",
    ]
    if any(e in text for e in excludes):
        return None
    if "コストは" in text and ("減る" in text or "少なくなる" in text):
        return None
    m = re.search(r"(.+?)(につき|ごとに)", text)
    if not m:
        return None
    per_text = m.group(1).strip()
    if "。" in per_text:
        return None
    result: Dict[str, Any] = {"text": text, "per_unit": True}
    cond_part, action_part = split_condition_action(per_text)
    if cond_part and action_part:
        cond = parse_condition(cond_part)
        if cond and cond.get("type") != "custom":
            result["condition"] = cond
            per_text = action_part
    for prefix, code in [
        ("ライブ終了時まで", "live_end"),
        ("このターンの間", "turn_end"),
        ("ターン終了時まで", "turn_end"),
    ]:
        if per_text.startswith(prefix):
            result["duration"] = code
            per_text = per_text[len(prefix) :].lstrip("、").strip()
            break
    pm = re.search(r"(\d+)(人|枚|つ)(につき|ごとに)", text)
    if pm:
        result["per_unit_count"] = int(pm.group(1))
        result["per_unit_type"] = pm.group(2)
    else:
        for kw, t in [
            ("メンバー", "member"),
            ("人", "member"),
            ("カード", "card"),
            ("枚", "card"),
            ("ブレード", "blade"),
            ("ハート", "heart"),
            ("スコア", "score"),
            ("コスト", "cost"),
        ]:
            if kw in per_text:
                result["per_unit_type"] = t
                break
    if "控え室に置いた" in per_text:
        result["per_unit_type"] = "discard"
    gm = re.search(r"『([^』]+)』", per_text)
    if gm:
        result["group_names"] = [gm.group(1)]
    if "名前の異なる" in per_text or "カード名の異なる" in per_text:
        result["distinct"] = "card_name"
    if f.cost_limit is not None:
        result["cost_limit"] = f.cost_limit
        if f.cost_limit_operator:
            result["cost_limit_operator"] = f.cost_limit_operator
    for kw, loc in [
        ("成功ライブカード置き場にある", "success_live_zone"),
        ("メンバーの下にある", "under_member"),
        ("ステージにいる", "stage"),
        ("控え室にある", "discard"),
        ("ライブカード置き場にある", "live_card_zone"),
        ("手札にある", "hand"),
        ("デッキにある", "deck"),
    ]:
        if kw in per_text:
            result["location"] = loc
            break
    if f.target:
        result["target"] = f.target
    if f.card_type:
        result["card_type"] = f.card_type
    action_text = text.split("につき", 1)[1].strip().lstrip("、")
    # Handle sequential in action (Aし、B)
    if "、" in action_text and "し" in action_text:
        parts = [p.strip().rstrip("、") for p in action_text.split("、")]
        if len(parts) >= 2 and "し" in parts[0]:
            actions = []
            for part in parts:
                pa = parse_effect(part)
                if pa.get("action") and pa.get("action") != "custom":
                    _propagate(result, pa)
                    actions.append(pa)
            if len(actions) >= 2:
                return {"text": text, "action": "sequential", "actions": actions}
    # Use parse_effect for chained actions, _parse_simple_action for simple
    if any(m in action_text for m in ["その後、", "し、", "。"]):
        parsed = parse_effect(action_text)
        if parsed.get("action") == "sequential":
            # Apply per_unit to the first sub-action, not the sequential wrapper
            first = parsed["actions"][0] if parsed["actions"] else parsed
            _propagate(result, first)
            first["text"] = text
            return parsed
    action = _parse_simple_action(action_text)
    _propagate(result, action)
    action["text"] = text
    return action


def _propagate(src, dst):
    for k in (
        "per_unit",
        "per_unit_count",
        "per_unit_type",
        "card_type",
        "group_names",
        "distinct",
        "timing_condition",
        "state",
        "location",
        "cost_limit",
        "cost_limit_operator",
        "duration",
        "condition",
        "target",
    ):
        if k in src and k not in dst:
            dst[k] = src[k]


@register(95)
def _try_conditional_alternative(text, f):
    if "代わりに" not in text:
        return None
    if "以下から1つを選ぶ" in text and text.find("以下から1つを選ぶ") < text.find(
        "代わりに"
    ):
        return None
    parts = text.split("代わりに", 1)
    if len(parts) != 2:
        return None
    primary_text = parts[0].strip()
    ct, at = split_condition_action(primary_text)
    cond = parse_condition(ct) if ct else None
    primary_action = parse_effect(at) if at else _parse_simple_action(primary_text)
    alt_action = _parse_simple_action(parts[1].strip())
    result: Dict[str, Any] = {
        "text": text,
        "action": "conditional_alternative",
        "primary_effect": primary_action,
        "alternative_effect": alt_action,
    }
    if cond and cond.get("type") != "custom":
        result["condition"] = cond
    return result


@register(93)
def _try_each_time(text, f):
    if "たび" not in text:
        return None
    tm = re.search(r"([^たび]+)たび", text)
    if not tm:
        return None
    trigger_text = tm.group(1).strip()
    rest = text[tm.end() :].strip().lstrip("、，")
    sub = parse_effect(rest)
    sub["trigger_type"] = "each_time"
    sub["text"] = text
    tc = parse_condition(trigger_text)
    if tc and tc.get("type") != "custom":
        sub["trigger_condition"] = tc
    return sub


@register(91)
def _try_opponent_action(text, f):
    if not text.startswith("相手は"):
        return None
    om = re.match(r"相手は[、]?(.+?)(?:。|$)", text)
    if not om:
        return None
    oa_text = om.group(0)
    rest = text[len(oa_text) :].strip()
    oa = _parse_simple_action(om.group(1).strip())
    opp = {
        "text": oa_text,
        "action": "opponent_action",
        "action_by": "opponent",
        "opponent_action": oa,
    }
    if rest:
        re_eff = parse_effect(rest)
        return {"text": text, "action": "sequential", "actions": [opp, re_eff]}
    return opp


@register(92)
def _try_unless_effect(text, f):
    kw = "しないかぎり" if "しないかぎり" in text else None
    if not kw:
        kw = "ないかぎり" if "ないかぎり" in text else None
    if not kw:
        return None
    parts = text.split(kw + "、", 1)
    if len(parts) < 2:
        return None
    unless_text = parts[0].strip()
    eff_text = parts[1].strip()
    if "{{icon_energy.png|E}}" not in unless_text:
        return None
    ec = unless_text.count("{{icon_energy.png|E}}")
    fa = {"action": "pay_energy", "energy": ec, "count": ec, "target": "self"}
    aa = parse_effect(eff_text)
    return {
        "text": text,
        "action": "conditional_on_optional",
        "optional_action": fa,
        "conditional_action": aa,
        "conditional_negation": True,
    }


@register(85)
def _try_sou_shinakatta(text, f):
    if "そうしなかった場合" not in text:
        return None
    parts = text.split("そうしなかった場合", 1)
    opt_text = parts[0].strip()
    fa = _parse_simple_action(opt_text)
    if opt_text.startswith("相手は"):
        fa["target"] = "opponent"
    aa = parse_effect(parts[1].strip().lstrip("、"))
    return {
        "text": text,
        "action": "conditional_on_optional",
        "optional_action": fa,
        "conditional_action": aa,
        "conditional_negation": True,
    }


@register(88)
def _try_conditional_sequential(text, f):
    if "そうした場合" not in text:
        return None
    parts = text.split("そうした場合", 1)
    fp = parts[0].strip()
    sp = parts[1].strip()
    fc, fat = split_condition_action(fp)
    cond = parse_condition(fc) if fc and fat else None
    fa = _parse_simple_action(fat if fat else fp)
    sa = parse_effect(sp.lstrip("、"))
    if fa.get("action") == "select":
        if isinstance(sa, dict):
            if "actions" in sa:
                for sub in sa.get("actions", []):
                    if isinstance(sub, dict) and sub.get("action") == "move_cards":
                        sub["source"] = "selected_cards"
            else:
                sa["source"] = "selected_cards"
    result: Dict[str, Any] = {
        "text": text,
        "action": "sequential",
        "actions": [fa, sa],
        "conditional": True,
    }
    if cond:
        result["condition"] = cond
    return result


@register(87)
def _try_sequential(text, f):
    if "その後、" not in text:
        return None
    parts = text.split("その後、", 1)
    fa = parse_effect(parts[0].strip())
    sa = parse_effect(parts[1].strip().lstrip("、"))
    return {"text": text, "action": "sequential", "actions": [fa, sa]}


@register(89)
def _try_furthermore(text, f):
    if "さらに" not in text:
        return None
    clean = re.sub(r"「[^」]*」", lambda m: m.group(0).replace("。", "\x00"), text)
    parts = [p.strip().replace("\x00", "。") for p in clean.split("。") if p.strip()]
    if len(parts) < 2 or not any("さらに" in p for p in parts[1:]):
        return None
    actions = []
    for p in parts:
        if "さらに" in p:
            p = p.replace("さらに", "", 1).strip()
        actions.append(parse_effect(p))
    if actions and any(a.get("action") or a.get("actions") for a in actions):
        return {"text": text, "action": "sequential", "actions": actions}
    return None


@register(90)
def _try_kore_niyori_result(text, f):
    if "これにより" not in text:
        return None
    for marker in ["場合", "とき"]:
        m = re.search(r"これにより(.+?)" + marker, text)
        if m:
            cond_marker = marker
            break
    else:
        return None
    parts = text.split("これにより", 1)
    sp = "これにより" + parts[1].strip()
    if cond_marker not in sp:
        return None
    cp, fp = sp.split(cond_marker, 1)
    cond = parse_condition(cp.strip() + cond_marker)
    if cond and cond.get("type") == "custom":
        cond = None
    primary_text = parts[0].strip()
    if not primary_text or re.match(r"^[\s）」》』」、。]*$", primary_text):
        return None
    return {
        "text": text,
        "action": "conditional_on_result",
        "primary_effect": parse_effect(primary_text),
        "result_condition": cond,
        "followup_action": parse_effect(fp.strip()),
    }


@register(80)
def _try_choice(text, f):
    if "以下から1つを選ぶ" not in text:
        return None
    parts = text.split("以下から1つを選ぶ", 1)
    if len(parts) <= 1:
        return None
    lines = [l.strip() for l in parts[1].strip().split("\n") if l.strip()]
    opts, cond_mod = [], None
    for line in lines:
        if line.startswith("・"):
            opts.append(line[1:].strip())
        elif not cond_mod:
            cond_mod = line
    result: Dict[str, Any] = {"text": text, "action": "choice"}
    if cond_mod and cond_mod not in ("。", "."):
        result["choice_modifier"] = cond_mod
        cond = parse_condition(cond_mod)
        if cond and cond.get("type") != "custom":
            result["choice_condition"] = cond
    options = []
    for ot in opts:
        oc, oa = split_condition_action(ot)
        po = parse_effect(oa) if oc and oa else _parse_simple_action(ot)
        if oc and oa:
            po["condition"] = parse_condition(oc)
        po["text"] = ot
        options.append(po)
    result["options"] = options
    return result


@register(86)
def _try_duration_effect(text, f):
    if "かぎり" not in text:
        return None
    parts = text.split("かぎり", 1)
    ct = parts[0].strip() + "かぎり"
    at = parts[1].strip().lstrip("、")
    cond = parse_condition(ct)
    action = _parse_simple_action(at)
    result: Dict[str, Any] = {"text": text, "condition": cond, "duration": "as_long_as"}
    if action.get("action") == "sequential":
        result["conditional"] = True
    result.update(action)
    return result


@register(82)
def _try_implicit_sequential(text, f):
    if "、" not in text and "。" not in text:
        return None
    # Skip if condition markers present but no 。separator
    cond_markers = ["場合、", "とき、", "なら、", "うち、", "において、"]
    has_cond = any(m in text for m in cond_markers)
    if has_cond and "。" not in text:
        return None
    if "以下から1つを選ぶ" in text:
        return None
    if "。" in text:
        clean_for_split = re.sub(r"（[^）]*）", "", text)
        clean_for_split = re.sub(r"\([^)]*\)", "", clean_for_split)
        clean_for_split = re.sub(
            r"「[^」]*」", lambda m: m.group(0).replace("。", "\x00"), clean_for_split
        )
        parts = [
            p.strip().replace("\x00", "。")
            for p in clean_for_split.split("。")
            if p.strip()
        ]
    else:
        parts = [p for p in text.split("、") if p.strip()]
    if len(parts) < 2:
        return None
    actions = []
    for p in parts:
        cp = p.strip().lstrip("、")
        a = parse_effect(cp)
        if a and a.get("action", "custom") not in ("custom", "do_nothing"):
            actions.append(a)
    if len(actions) >= 2:
        return {"text": text, "action": "sequential", "actions": actions}
    return None


@register(79)
def _try_conditional(text, f):
    ct, at = split_condition_action(text)
    if not ct or not at:
        return None
    cond = parse_condition(ct)
    at = at.lstrip("、")
    at, dur = strip_duration_prefix(at)
    action = parse_effect(at)
    result: Dict[str, Any] = {"text": text, "condition": cond}
    if dur:
        action["duration"] = dur
    if action.get("action") == "sequential":
        result["action"] = "sequential"
        result["actions"] = action.get("actions", [])
        result["conditional"] = True
    else:
        result.update(action)
    return result


@register(94)
def _try_look_and_select(text, f):
    if "その中から" not in text:
        return None
    result: Dict[str, Any] = {"text": text, "action": "look_and_select"}
    lm = re.search(r"(.+?)その中から", text)
    if lm:
        look_text = lm.group(1).strip()
        ct, at = split_condition_action(look_text)
        if ct:
            cond = parse_condition(ct)
            if cond and cond.get("type") != "custom":
                result["condition"] = cond
                cond_loc = cond.get("location")
                if cond_loc and cond_loc not in ("stage", "hand") and at:
                    la = _parse_simple_action(at)
                    if la.get("action") != "custom":
                        la.setdefault("source", cond_loc)
                        result["look_action"] = la
        if "look_action" not in result and at:
            la = _parse_simple_action(at)
            if la.get("action") != "custom":
                result["look_action"] = la
    am = re.search(r"その中から(.+)", text)
    if am:
        select_text = am.group(1).strip()
        sf = ExtractedFields(select_text)
        sa: Dict[str, Any] = {"action": "select_cards", "discard_remaining": True}
        if (
            "デッキの上に置く" in select_text
            or "デッキの上に" in select_text
            or "デッキの一番上に" in select_text
        ):
            sa["destination"] = "deck_top"
        elif "手札に加える" in select_text or "手札に加え" in select_text:
            sa["destination"] = "hand"
        elif "控え室に置く" in select_text:
            sa["destination"] = "discard"
        if sf.count is not None:
            sa["count"] = sf.count
        if sf.card_type:
            sa["card_type"] = sf.card_type
        if sf.group_names:
            sa["group_names"] = sf.group_names
        if sf.heart_colors:
            sa["heart_colors"] = sf.heart_colors
        if sf.max:
            sa["max"] = True
        if sf.optional:
            sa["optional"] = True
        if sf.any_number:
            sa["any_number"] = True
        if sf.cost_limit is not None:
            sa["cost_limit"] = sf.cost_limit
            if sf.cost_limit_operator:
                sa["cost_limit_operator"] = sf.cost_limit_operator
        if sf.placement_order:
            sa["placement_order"] = sf.placement_order
        result["select_action"] = sa
    # Propagate optional, duration from text to look_and_select result
    if "optional" not in result and ("もよい" in text or "てもよい" in text):
        result["optional"] = True
    if "duration" not in result:
        for kw, code in [
            ("ライブ終了時まで", "live_end"),
            ("ライブ終了まで", "live_end"),
            ("このターンの間", "this_turn"),
            ("このライブの間", "this_live"),
            ("ターン終了時まで", "turn_end"),
            ("そのターンの間", "turn_end"),
        ]:
            if kw in text:
                result["duration"] = code
                break
    return result


@register(84)
def _try_shi_sequential(text, f):
    if "し、" not in text:
        return None
    if "以下から1つを選ぶ" in text:
        return None
    if any(m in text for m in ["場合、", "とき、", "なら、"]):
        return None
    idx = text.find("し、")
    if idx < 0:
        return None
    first = text[: idx + 1]
    rest = text[idx + 2 :].strip().lstrip("、")
    fa = parse_effect(first)
    if fa.get("action", "custom") in ("custom", "do_nothing"):
        return None
    sa = parse_effect(rest)
    if sa.get("action", "custom") in ("custom", "do_nothing"):
        return None
    return {"text": text, "action": "sequential", "actions": [fa, sa]}


@register(75)
def _try_blade_actions(text, f):
    if "同じことを行う" in text:
        result: Dict[str, Any] = {
            "text": text,
            "action": "gain_resource",
            "resource": "blade",
        }
        bc = text.count("{{icon_blade.png|ブレード}}")
        result["count"] = bc if bc > 0 else 1
        if "ライブ終了時まで" in text:
            result["duration"] = "live_end"
        return result
    if "ブレードの数は" in text and ("つになる" in text or "になる" in text):
        result = {"text": text, "action": "set_blade_count"}
        m = re.search(r"(\d+)つになる", text) or re.search(r"(\d+)になる", text)
        if m:
            result["count"] = int(m.group(1))
        return result
    return None


@register(60)
def _try_restriction_effect(text, f):
    if "アクティブにならない" not in text:
        return None
    result: Dict[str, Any] = {
        "text": text,
        "action": "restriction",
        "restriction_type": "cannot_activate",
    }
    if "アクティブフェイズ" in text:
        result["phase"] = "active_phase"
    if "効果によっては" in text:
        result["restriction_type"] = "cannot_activate_by_effect"
    if "自分と相手の" in text:
        result["target"] = "both"
    elif "自分の" in text:
        result["target"] = "self"
    elif "相手の" in text:
        result["target"] = "opponent"
    if "このターン" in text:
        result["duration"] = "this_turn"
    if "メンバー" in text:
        result["card_type"] = "member_card"
    if "エネルギー" in text:
        result["card_type"] = "energy_card"
    return result


@register(65)
def _try_lose_resource(text, f):
    if "を失う" not in text:
        return None
    result: Dict[str, Any] = {
        "text": text,
        "action": "gain_resource",
        "sign": "negative",
    }
    if "ブレード" in text:
        result["resource"] = "blade"
    elif "ハート" in text:
        result["resource"] = "heart"
    bc = len(re.findall(r"\{\{icon_blade\.png\|ブレード\}\}", text))
    if bc > 0:
        result["count"] = bc
    hc = len(re.findall(r"\{\{heart_\d+\.png\|heart\d+\}\}", text))
    if hc > 0:
        result["count"] = hc
    if "ライブ終了時まで" in text:
        result["duration"] = "live_end"
    return result


@register(55)
def _try_global_modifier(text, f):
    if "必要ハート" in text and ("多くなる" in text or "少なくなる" in text):
        result: Dict[str, Any] = {
            "text": text,
            "action": "modify_required_hearts_global",
            "operation": "increase" if "多くなる" in text else "decrease",
        }
        tm = re.search(r"([^は]+)は", text)
        if tm:
            raw_target = tm.group(1).strip()
            result["target"] = "opponent" if "相手の" in raw_target else raw_target
        if "すべて" in text:
            result["all"] = True
        hm = re.search(r"\{\{heart_(\d+)\.png\|heart\d+\}\}", text)
        if hm:
            result["heart_colors"] = [f"heart{hm.group(1).zfill(2)}"]
        vm = re.search(r"(\d+)つ多", text)
        result["value"] = int(vm.group(1)) if vm else 1
        return result
    return None


@register(50)
def _try_both_discard(text, f):
    if "自分と相手はそれぞれ" not in text or "枚になるまで" not in text:
        return None
    if "控え室に置き" not in text and "控え室に置く" not in text:
        return None
    result: Dict[str, Any] = {
        "text": text,
        "action": "sequential",
        "target": "both",
        "multiple_targets": True,
    }
    parts = re.split(r"その後[、。]?", text, maxsplit=1)
    if len(parts) == 2:
        fa_text = parts[0].strip()
        sa_text = parts[1].strip()
        fa: Dict[str, Any] = {
            "text": fa_text,
            "action": "discard_until_count",
            "target": "both",
            "multiple_targets": True,
        }
        m = re.search(r"(\d+)枚になるまで", fa_text)
        if m:
            fa["target_count"] = int(m.group(1))
        sa = parse_effect(sa_text)
        result["actions"] = [fa, sa]
        return result
    return None
