"""Cost parsing using a flat pattern registry."""

import re
from typing import Dict, Any, Optional

from test_parser.fields import ExtractedFields, normalize_fullwidth_digits


def parse_cost(text: str) -> Dict[str, Any]:
    """Parse a cost text. Returns structured cost dict."""
    text = normalize_fullwidth_digits(text.strip()).rstrip("。")
    cost: Dict[str, Any] = {"text": text}
    f = ExtractedFields(text)

    # Basic field extraction
    _extract_basic_cost_fields(cost, text, f)

    # Try patterns in priority order
    for _, handler in _COST_HANDLERS:
        result = handler(text, f)
        if result is not None:
            return result

    # Fallback: infer type
    if "type" not in cost:
        if cost.get("source") and cost.get("destination"):
            cost["type"] = "move_cards"
        elif f.state_change:
            cost["type"] = "change_state"
        elif cost.get("source"):
            if cost["source"] == "hand" and (
                "控え室に置く" in text or "控え室に置いて" in text
            ):
                cost["destination"] = "discard"
                cost["type"] = "move_cards"
            elif cost["source"] == "discard" and "手札に加える" in text:
                cost["destination"] = "hand"
                cost["type"] = "move_cards"
            else:
                cost["type"] = "custom"
        else:
            cost["type"] = "custom"

    return cost


def _extract_basic_cost_fields(cost: Dict[str, Any], text: str, f: ExtractedFields):
    """Extract common cost fields."""
    # Source
    if "手札を" in text or "手札の" in text:
        cost["source"] = "hand"
        cost["zone"] = "hand"
    src = f.source
    if src and "source" not in cost:
        cost["source"] = src
        if "zone" not in cost:
            cost["zone"] = src

    # Destination
    dst = f.destination
    if dst:
        cost["destination"] = dst
    if "エネルギーデッキに置く" in text:
        cost["destination"] = "energy_deck"

    # Infer destination from source
    if "source" in cost and "destination" not in cost:
        if cost["source"] == "hand" and (
            "控え室に置く" in text or "控え室に置いて" in text
        ):
            cost["destination"] = "discard"
        elif cost["source"] == "discard" and "手札に加える" in text:
            cost["destination"] = "hand"

    # State change
    if f.state_change:
        cost["state_change"] = f.state_change

    # Count, type, target
    if f.count is not None:
        cost["count"] = f.count
    if f.card_type:
        cost["card_type"] = f.card_type
    if f.target:
        cost["target"] = f.target

    # Group names
    if f.group_names:
        cost["group_names"] = f.group_names

    # Optional
    if f.optional:
        cost["optional"] = True

    # Shuffle
    if f.shuffle:
        cost["shuffle"] = True

    # Movement state
    if "移動している" in text:
        cost["movement_state"] = "has_moved"

    # Baton touch
    if f.baton_touch:
        m = re.search(r"「([^」]+)」からバトンタッチ", text)
        if m:
            cost["baton_touch_source"] = m.group(1)
        m = re.search(r"『([^』]+)』からバトンタッチ", text)
        if m:
            cost["baton_touch_group"] = m.group(1)

    # Cost limit
    if f.cost_limit is not None:
        cost["cost_limit"] = f.cost_limit
        if f.cost_limit_operator:
            cost["cost_limit_operator"] = f.cost_limit_operator

    # Exclude self
    if f.exclude_self:
        cost["exclude_self"] = True

    # Same unit name
    if f.same_unit_name:
        cost["same_unit_name"] = True

    # Self cost (this member as cost)
    if f.self_cost:
        cost["self_cost"] = True

    # Characters from 「」
    include_chars = []
    exclude_chars = []
    for name in f.quoted_text:
        idx = text.find(f"「{name}」")
        if idx >= 0:
            after = text[idx + len(f"「{name}」") : idx + len(f"「{name}」") + 3]
            if after.startswith("以外"):
                exclude_chars.append(name)
            else:
                include_chars.append(name)
    if include_chars:
        cost["characters"] = include_chars
    if exclude_chars:
        cost["exclude_characters"] = exclude_chars

    # Optional
    if f.optional:
        cost["optional"] = True

    # Any number
    if f.any_number:
        cost["any_number"] = True


# ===================== COST HANDLER REGISTRY =====================

_COST_HANDLERS = []


def register(priority: int = 0):
    def wrapper(func):
        _COST_HANDLERS.append((priority, func))
        _COST_HANDLERS.sort(key=lambda x: -x[0])
        return func

    return wrapper


@register(100)
def _try_verb_choice(text, f):
    """Verb-level choice: AかB (e.g. pay energy OR discard)."""
    verb_choice_m = re.search(r"(.*(?:支払う|置く|加える|公開する))か(.+)", text)
    if not verb_choice_m:
        return None
    full_opt1 = text[: text.find("か", text.find(verb_choice_m.group(1)))].strip()
    if not full_opt1:
        full_opt1 = verb_choice_m.group(1).strip()
    opt2 = verb_choice_m.group(2).strip()
    return {
        "text": text,
        "type": "choice_condition",
        "options": [parse_cost(full_opt1), parse_cost(opt2)],
    }


@register(95)
def _try_energy_start(text, f):
    """Energy cost {{E}}{{E}} at start of text."""
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
            if "もよい" in text or "てもよい" in text:
                result["optional"] = True
            return result

    energy_count = text.count("{{icon_energy.png|E}}")
    cost = {
        "text": text,
        "type": "pay_energy",
        "energy": energy_count,
        "zone": "energy_zone",
        "count": energy_count,
    }
    if "もよい" in text or "てもよい" in text:
        cost["optional"] = True
    return cost


@register(90)
def _try_sequential_cost(text, f):
    """Sequential cost: Aし、B or Aて、B"""
    if "、" not in text:
        return None
    parts = text.split("、")
    if len(parts) < 2:
        return None
    first_ends = parts[0].strip()[-1] if parts[0].strip() else ""
    if (
        first_ends not in ("し", "て")
        and not parts[0].strip().endswith("し")
        and not parts[0].strip().endswith("て")
    ):
        return None
    cost_parts = []
    for i, part in enumerate(parts):
        if (
            i == 0
            and not part.strip().endswith("し")
            and not part.strip().endswith("て")
        ):
            part = part.strip() + "し"
        cost_parts.append(parse_cost(part.strip()))
    result = {"text": text, "type": "sequential_cost", "costs": cost_parts}
    if any(cp.get("optional") for cp in cost_parts):
        result["optional"] = True
        for cp in cost_parts:
            cp["optional"] = True
    return result


@register(85)
def _try_reveal_cost(text, f):
    """公開する/公開し — reveal cost."""
    if "公開する" not in text and "公開し" not in text:
        return None
    cost = {"text": text, "type": "reveal"}
    if "手札" in text:
        cost["source"] = "hand"
    cm = re.search(r"(\d+)枚", text)
    if cm:
        cost["count"] = int(cm.group(1))
    if f.card_type:
        cost["card_type"] = f.card_type
    if f.group_names:
        cost["group_names"] = f.group_names
    return cost


@register(80)
def _try_choice_cost(text, f):
    """Choice cost: Aか、B"""
    if "か、" not in text:
        return None
    parts = text.split("か、", 1)
    if len(parts) == 2:
        return {
            "text": text,
            "type": "choice_condition",
            "options": [parse_cost(parts[0].strip()), parse_cost(parts[1].strip())],
        }
    return None


@register(75)
def _try_deck_bottom(text, f):
    """Deck bottom placement cost."""
    deck_bottom_kw = (
        "デッキの一番下に置く",
        "デッキの一番下に置いて",
        "デッキの下に置く",
        "デッキの下に置いて",
        "山札の下に置く",
        "山札の下に置いて",
    )
    if not any(kw in text for kw in deck_bottom_kw):
        return None
    cost = {"text": text, "destination": "deck_bottom", "type": "move_cards"}
    src = f.source
    if src:
        cost["source"] = src
    if f.shuffle:
        cost["shuffle"] = True
    return cost
