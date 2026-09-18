import re
from typing import Any, Dict, List

from parser_utils import (
    COUNT_PATTERN,
    extract_all_groups,
    extract_card_type,
    extract_cost_limit,
    extract_cost_values,
    extract_count,
    extract_destination,
    extract_operator,
    extract_source,
    extract_target,
)
from parser_fields import (
    _has_shuffle,
    apply_character_filters,
    apply_extracted_fields,
    detect_exclude_self,
    extract_max,
    extract_optional,
    extract_state_change,
    set_cross_position_fields,
)


SPLIT_LIMIT = 1
_COST_HANDLERS: List[Any] = []
_COST_FLAG_RULES = [
    ("同じグループ名", "group_reference", "same_group_name"),
    (extract_optional, "optional", True),
    (_has_shuffle, "shuffle", True),
    (detect_exclude_self, "exclude_self", True),
    ("同じユニット名", "same_unit_name", True),
]
_COST_BATON_TOUCH_PATTERNS = [
    (r"「([^」]+)」からバトンタッチ", "baton_touch_source"),
    (r"『([^』]+)』からバトンタッチ", "baton_touch_group"),
]
_COST_CARD_FIELDS = (
    ("count", extract_count),
    ("card_type", extract_card_type),
    ("target", extract_target),
    ("group_names", extract_all_groups),
)


def _mark_discard_all_hand(cost, text):
    if (
        cost.get("source") == "hand"
        and cost.get("destination") == "discard"
        and re.search(r"手札を\s*(すべて|全て|全部)|手札の\s*(すべて|全て|全部)", text)
    ):
        cost["all"] = True
        cost.pop("count", None)


def _infer_destination_from_source(cost, text):
    if "source" in cost and "destination" not in cost:
        if cost["source"] == "hand" and (
            "控え室に置く" in text or "控え室に置いて" in text
        ):
            cost["destination"] = "discard"
        elif cost["source"] == "discard" and "手札に加える" in text:
            cost["destination"] = "hand"


def _fill_cost_source(cost, text):
    if "手札を" in text or "手札の" in text:
        cost["source"] = "hand"
        cost["zone"] = "hand"
    src = extract_source(text)
    if src and "source" not in cost:
        cost["source"] = src
        if "zone" not in cost:
            cost["zone"] = src


def _fill_cost_destination(cost, text):
    dst = extract_destination(text)
    if dst:
        cost["destination"] = dst
    if "エネルギーデッキに置く" in text:
        cost["destination"] = "energy_deck"
        if "source" not in cost and "エネルギー" in text:
            cost["source"] = "energy_zone"


def _mark_self_cost(cost, text):
    if (
        "このメンバー" in text
        and "このメンバー以外" not in text
        and not bool(re.search(r"ほかの.*?メンバー", text))
        and re.search(r"このメンバー[をが]", text)
    ):
        cost["self_cost"] = True


def _extract_basic_cost_fields(cost, text):
    _fill_cost_source(cost, text)
    _fill_cost_destination(cost, text)
    _infer_destination_from_source(cost, text)
    _mark_discard_all_hand(cost, text)
    apply_extracted_fields(cost, text, (
        ("state_change", extract_state_change),
        *_COST_CARD_FIELDS,
    ))
    for test, field, value in _COST_FLAG_RULES:
        if test(text) if callable(test) else test in text:
            cost[field] = value
    if "バトンタッチ" in text:
        for pattern, field in _COST_BATON_TOUCH_PATTERNS:
            match = re.search(pattern, text)
            if match:
                cost[field] = match.group(1)
    limit = extract_cost_limit(text)
    if limit:
        cost["cost_limit"] = limit
        apply_extracted_fields(cost, text, (("cost_limit_operator", extract_operator),))
    values = extract_cost_values(text)
    if values:
        cost["cost_values"] = values
        cost.pop("cost_limit", None)
    _mark_self_cost(cost, text)
    apply_character_filters(cost, text)
    if "position" not in cost:
        set_cross_position_fields(cost, re.sub(r"\{\{[^}]+?\}\}", "", text))


def _register_cost(handler):
    _COST_HANDLERS.append(handler)
    return handler


def _set_optional_cost(result):
    result["optional"] = True
    for cost in result["costs"]:
        cost["optional"] = True


def _choice_cost(text, parts):
    return {
        "text": text,
        "type": "choice_condition",
        "options": [parse_cost(part.strip()) for part in parts],
    }


@_register_cost
def _cost_verb_choice(text, cost):
    match = re.search(r"(.*(?:支払う|置く|加える|公開する))か(.+)", text)
    if not match:
        return None
    first = text[: text.find("か", text.find(match.group(1)))].strip()
    if not first:
        first = match.group(1).strip()
    return _choice_cost(text, (first, match.group(2)))


@_register_cost
def _cost_energy(text, cost):
    if not text.strip().startswith("{{icon_energy.png|E}}"):
        return None
    energy_end = text.find("}}", text.rfind("{{icon_energy.png|E}}")) + 2
    energy_text = text[:energy_end].strip()
    other_text = text[energy_end:].strip()
    if energy_text and other_text:
        other_cost = parse_cost(other_text)
        if other_cost.get("type") not in (None, "custom"):
            result = {
                "text": text,
                "type": "sequential_cost",
                "costs": [parse_cost(energy_text), other_cost],
            }
            if extract_optional(text):
                _set_optional_cost(result)
            return result
    energy_count = text.count("{{icon_energy.png|E}}")
    cost["type"] = "pay_energy"
    cost["energy"] = energy_count
    cost["zone"] = "energy_zone"
    cost["count"] = energy_count
    if extract_optional(text):
        cost["optional"] = True
    if "好きな数" in text or "任意の数" in text:
        cost["any_number"] = True
    if extract_max(text):
        cost["max"] = True
    return cost


@_register_cost
def _cost_sequential(text, cost):
    if "、" not in text:
        return None
    parts = text.split("、")
    if len(parts) < 2 or not parts[0].strip().endswith(("し", "て")):
        return None
    cost_parts = [parse_cost(part.strip()) for part in parts]
    result = {"text": text, "type": "sequential_cost", "costs": cost_parts}
    if "position" in cost:
        result["position"] = cost["position"]
        if "position_compare" in cost:
            result["position_compare"] = cost["position_compare"]
    if any(part.get("optional") for part in cost_parts):
        _set_optional_cost(result)
    return result


@_register_cost
def _cost_reveal(text, cost):
    if "公開する" not in text and "公開し" not in text:
        return None
    cost["type"] = "reveal"
    if "手札" in text:
        cost["source"] = "hand"
    match = re.search(COUNT_PATTERN, text)
    if match:
        cost["count"] = int(match.group(1))
    apply_extracted_fields(cost, text, _COST_CARD_FIELDS[1:2] + _COST_CARD_FIELDS[3:])
    return cost


@_register_cost
def _cost_choice_comma(text, cost):
    if "か、" not in text:
        return None
    parts = text.split("か、", SPLIT_LIMIT)
    if len(parts) != 2:
        return None
    return _choice_cost(text, parts)


def _apply_under_member_classification(cost, text):
    if re.search(r"エネルギー\s*\d*\s*枚", text) or "エネルギーカード" in text:
        cost["card_type"] = "energy_card"
    elif "このカード" in text or "メンバーカード" in text:
        cost["card_type"] = "member_card"
    if not cost.get("source"):
        cost["source"] = extract_source(text) or "energy_zone"
    return "place_energy_under_member"


def _apply_pay_energy_classification(cost, text):
    cost["energy"] = text.count("{{icon_energy.png|E}}")
    if extract_optional(text):
        cost["optional"] = True
    return "pay_energy"


def _apply_source_only_classification(cost, text):
    _infer_destination_from_source(cost, text)
    if cost.get("destination"):
        return "move_cards"
    return None


# Ordered classification table for `_classify_cost`: first match wins.
# Each row is (predicate, apply). The predicates run in the same order as
# the legacy if-chain; `apply` performs the branch's field writes and
# returns the cost type. `_infer_destination_from_source` inside the
# source-only row can promote it to move_cards, hence the Optional return.
_CLASSIFY_COST_RULES = [
    (
        lambda cost, text: cost.get("destination") == "under_member",
        _apply_under_member_classification,
    ),
    (
        lambda cost, text: bool(cost.get("source") and cost.get("destination")),
        lambda cost, text: "move_cards",
    ),
    (
        lambda cost, text: cost.get("destination") in ("energy_deck", "energy_zone")
        and not cost.get("source"),
        lambda cost, text: "move_cards",
    ),
    (
        lambda cost, text: any(
            phrase in text
            for phrase in (
                "ウェイトにする",
                "ウェイト状態で置く",
                "ウェイト状態で登場させる",
                "アクティブにする",
            )
        )
        or cost.get("state_change"),
        lambda cost, text: "change_state",
    ),
    (
        lambda cost, text: "{{icon_energy.png|E}}" in text
        and ("支払う" in text or "支払って" in text),
        _apply_pay_energy_classification,
    ),
    (
        lambda cost, text: bool(cost.get("source")),
        _apply_source_only_classification,
    ),
]


def _classify_cost(cost, text):
    for check, apply in _CLASSIFY_COST_RULES:
        if check(cost, text):
            result = apply(cost, text)
            if result is not None:
                return result
    return "custom"


# Post-handler tail flags for `parse_cost`: (trigger phrases, field, value).
# One table replaces the copy-pasted `any(phrase in text)` blocks. The
# deck-bottom case keeps its bespoke shape (it also backfills source and
# the move_cards type in field-insertion order) and stays ahead of the
# table so regenerated JSON stays byte-identical.
_DECK_BOTTOM_PHRASES = (
    "デッキの一番下に置く",
    "デッキの一番下に置いて",
    "デッキの下に置く",
    "デッキの下に置いて",
    "山札の下に置く",
    "山札の下に置いて",
)
_TAIL_FLAG_PHRASES = (
    (
        ("好きな枚数", "好きな枚数まで", "任意の枚数", "好きな組み合わせ"),
        "any_number",
        True,
    ),
    (("好きな順番で",), "placement_order", "any_order"),
)


def parse_cost(text: str) -> Dict[str, Any]:
    cost: Dict[str, Any] = {"text": text}
    _extract_basic_cost_fields(cost, text)
    for handler in _COST_HANDLERS:
        result = handler(text, cost)
        if result is not None:
            return result
    if any(phrase in text for phrase in _DECK_BOTTOM_PHRASES):
        cost["destination"] = "deck_bottom"
        cost["type"] = "move_cards"
        apply_extracted_fields(cost, text, (("source", extract_source),))
    for phrases, field, value in _TAIL_FLAG_PHRASES:
        if any(phrase in text for phrase in phrases):
            cost[field] = value
    if extract_max(text):
        cost["max"] = True
        cost["any_number"] = True
    if "type" not in cost:
        cost["type"] = _classify_cost(cost, text)
    return cost
