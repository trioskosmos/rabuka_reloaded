import re
from typing import Any, Dict, List

from parser_utils import (
    COUNT_PATTERN,
    ENERGY_ICON,
    FieldContext,
    COST_CARD_FIELDS_POLICY,
    COST_REVEAL_FIELDS_POLICY,
    extract_all_groups,
    extract_cost_limit,
    extract_cost_values,
    extract_field_context,
    extract_operator,
    matches_predicate,
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
# A cost that puts a card into a state rather than moving it. This is the
# shape `change_state` is classified by, and it is deliberately narrower than
# parser_utils.STATE_CHANGE_PATTERNS: the extra spellings there (〜にしてもよい,
# 〜にし) are cost wording that does not by itself mean a state change here.
_WAIT_ACTIVATE_PHRASES = (
    "ウェイトにする",
    "ウェイト状態で置く",
    "ウェイト状態で登場させる",
    "アクティブにする",
)
# Destinations that mean "energy zone" for classification purposes: a cost
# ending in one of these, with no source of its own, moves a card into it.
_ENERGY_DESTINATIONS = ("energy_deck", "energy_zone")


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


def _fill_cost_source(cost: dict, text: str, context: FieldContext):
    if "手札を" in text or "手札の" in text:
        cost["source"] = "hand"
        cost["zone"] = "hand"
    if context.source and "source" not in cost:
        cost["source"] = context.source
        if "zone" not in cost:
            cost["zone"] = context.source


def _fill_cost_destination(cost: dict, text: str, context: FieldContext):
    if context.destination:
        cost["destination"] = context.destination
    if "エネルギーデッキに置く" in text:
        cost["destination"] = "energy_deck"
        if "source" not in cost and "エネルギー" in text:
            cost["source"] = "energy_zone"


def _mark_self_cost(cost, text):
    if (
        ("このメンバー" in text or "このカード" in text)
        and "このメンバー以外" not in text
        and "このカード以外" not in text
        and not bool(re.search(r"ほかの.*?メンバー", text))
        and re.search(r"(?:このメンバー|このカード)[をが]", text)
    ):
        cost["self_cost"] = True


def _extract_basic_cost_fields(cost, text, context=None):
    if context is None:
        context = extract_field_context(text)
    _fill_cost_source(cost, text, context)
    _fill_cost_destination(cost, text, context)
    _infer_destination_from_source(cost, text)
    _mark_discard_all_hand(cost, text)
    state_change = extract_state_change(text)
    if state_change:
        cost["state_change"] = state_change
    context.apply(cost, COST_CARD_FIELDS_POLICY)
    group_names = extract_all_groups(text)
    if group_names:
        cost["group_names"] = group_names
    for test, field, value in _COST_FLAG_RULES:
        if matches_predicate(test, text):
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
    if not text.strip().startswith(ENERGY_ICON):
        return None
    energy_end = text.find("}}", text.rfind(ENERGY_ICON)) + 2
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
    energy_count = text.count(ENERGY_ICON)
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
    context = extract_field_context(text)
    context.apply(cost, COST_REVEAL_FIELDS_POLICY)
    group_names = extract_all_groups(text)
    if group_names:
        cost["group_names"] = group_names
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
        cost["source"] = extract_field_context(text).source or "energy_zone"
    return "place_energy_under_member"


def _apply_pay_energy_classification(cost, text):
    cost["energy"] = text.count(ENERGY_ICON)
    if extract_optional(text):
        cost["optional"] = True
    return "pay_energy"


def _apply_source_only_classification(cost, text):
    _infer_destination_from_source(cost, text)
    if cost.get("destination"):
        return "move_cards"
    return None


def _is_wait_or_activate(cost, text):
    return any(phrase in text for phrase in _WAIT_ACTIVATE_PHRASES) or bool(
        cost.get("state_change")
    )


def _is_pay_energy(cost, text):
    return ENERGY_ICON in text and ("支払う" in text or "支払って" in text)


def _cost_type(name):
    """A classification row that only names the type, writing no fields.

    Several rows of `_CLASSIFY_COST_RULES` are pure answers — the fields are
    already on `cost` by the time classification runs, and the row's whole job
    is to say which type that shape is. This gives those rows the same
    `(predicate, apply)` shape as the rows that do write fields, so the table
    stays one kind of thing instead of mixing in bare strings.
    """

    def apply(cost, text):
        return name

    return apply


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
        _cost_type("move_cards"),
    ),
    (
        lambda cost, text: cost.get("destination") in _ENERGY_DESTINATIONS
        and not cost.get("source"),
        _cost_type("move_cards"),
    ),
    (_is_wait_or_activate, _cost_type("change_state")),
    (_is_pay_energy, _apply_pay_energy_classification),
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
    context = extract_field_context(text)
    _extract_basic_cost_fields(cost, text, context)
    for handler in _COST_HANDLERS:
        result = handler(text, cost)
        if result is not None:
            return result
    if any(phrase in text for phrase in _DECK_BOTTOM_PHRASES):
        cost["destination"] = "deck_bottom"
        cost["type"] = "move_cards"
        if context.source:
            cost["source"] = context.source
    for phrases, field, value in _TAIL_FLAG_PHRASES:
        if any(phrase in text for phrase in phrases):
            cost[field] = value
    if extract_max(text):
        cost["max"] = True
        cost["any_number"] = True
    if "type" not in cost:
        cost["type"] = _classify_cost(cost, text)
    return cost
