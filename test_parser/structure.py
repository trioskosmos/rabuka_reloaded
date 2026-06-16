"""Text topology analysis — determine how ability text is organized."""

import re
from typing import Dict, Any, Optional, Tuple


def split_cost_effect(text: str) -> Tuple[str, str]:
    """Split text into cost and effect parts at the first '：' not inside parens/quotes."""
    if "：" not in text:
        return "", text
    paren_depth = 0
    quote_depth = 0
    split_index = -1
    for i, char in enumerate(text):
        if char in ("（", "("):
            paren_depth += 1
        elif char in ("）", ")"):
            paren_depth -= 1
        elif char in ('"', '"'):
            quote_depth += 1 if quote_depth == 0 else -1
        elif char == "：":
            if paren_depth == 0 and quote_depth == 0:
                split_index = i
                break
    if split_index >= 0:
        return text[:split_index].strip(), text[split_index + 1 :].strip()
    return "", text


def split_condition_action(text: str) -> Tuple[str, str]:
    """Split text at the first condition marker (場合、/とき、/なら、/時、).
    Returns (condition_part, action_part)."""
    for marker in ["場合、", "とき、", "なら、"]:
        idx = text.find(marker)
        if idx >= 0:
            return text[: idx + 2].strip(), text[idx + 2 :].strip()
    # Also check 「時、」at a non-duration position
    t_pos = text.find("時、")
    if t_pos > 0 and "ライブ終了時まで" not in text[: t_pos + 2]:
        return text[: t_pos + 1].strip(), text[t_pos + 2 :].strip()
    return "", text


def strip_duration_prefix(text: str) -> Tuple[str, Optional[str]]:
    DURATION_PREFIX_MAP = {
        "ライブ終了時まで": "live_end",
        "ライブ終了まで": "live_end",
        "このターンの間": "this_turn",
        "このライブの間": "this_live",
        "ターン終了時まで": "turn_end",
        "そのターンの間": "turn_end",
    }
    for pat, code in DURATION_PREFIX_MAP.items():
        if text.startswith(pat):
            rest = text[len(pat) :].lstrip("、，").strip()
            return rest, code
    return text, None


def analyze_topology(text: str) -> Dict[str, Any]:
    """Analyze the text topology. Returns a structure dict describing
    how the text is organized."""
    result = {
        "text": text,
        "has_cost_effect_split": "：" in text,
        "cost_text": "",
        "effect_text": text,
        "duration_code": None,
        "condition_text": "",
        "action_text": text,
        "has_condition": False,
        "is_per_unit": False,
        "is_choice": False,
        "is_sequential": False,
        "is_kore_niyori": False,
        "is_sou_shita": False,
        "is_furthermore": False,
        "is_each_time": False,
        "is_unless": False,
        "is_baton_touch": False,
        "is_alternative": False,
        "has_opponent_action": False,
    }

    # 1. Cost:effect split
    if result["has_cost_effect_split"]:
        result["cost_text"], result["effect_text"] = split_cost_effect(text)
    else:
        result["effect_text"] = text

    eff = result["effect_text"]

    # 2. Duration prefix
    rest, dur = strip_duration_prefix(eff)
    if dur:
        result["duration_code"] = dur
        eff = rest

    # 3. Condition split (場合、/とき、/なら、)
    cond_part, action_part = split_condition_action(eff)
    if cond_part and action_part:
        result["has_condition"] = True
        result["condition_text"] = cond_part
        result["action_text"] = action_part

    # 4. Structural markers in the action text
    at = result["action_text"]

    # Per-unit
    if "につき" in at or "ごとに" in at:
        # Check for exclusion patterns (group names, cost modification)
        excludes = [
            "各グループ名につき",
            "グループ名につき",
            "この能力を起動するためのコストは",
        ]
        if not any(e in at for e in excludes):
            if not ("コストは" in at and ("減る" in at or "少なくなる" in at)):
                result["is_per_unit"] = True

    # Choice
    if "以下から1つを選ぶ" in at:
        result["is_choice"] = True

    # Sequential markers
    if "その後、" in at:
        result["is_sequential"] = True

    # これにより
    if "これにより" in at:
        result["is_kore_niyori"] = True

    # そうした場合
    if "そうした場合" in at:
        result["is_sou_shita"] = True

    # さらに
    if "さらに" in at:
        result["is_furthermore"] = True

    # たび (each time)
    if "たび" in at:
        result["is_each_time"] = True

    # しないかぎり/ないかぎり (unless pay)
    if "しないかぎり" in at or "ないかぎり" in at:
        result["is_unless"] = True

    # バトンタッチ
    if "バトンタッチ" in at:
        result["is_baton_touch"] = True

    # 代わりに (alternative)
    if "代わりに" in at:
        result["is_alternative"] = True

    # 相手は (opponent action) - at start
    if at.startswith("相手は"):
        result["has_opponent_action"] = True

    # Activation suffix
    if "この能力は" in at and (
        "場合のみ" in at or "起動できる" in at or "発動する" in at
    ):
        result["has_activation_suffix"] = True

    # Implicit sequential (comma separated)
    if "、" in at and not result["has_condition"] and not result["is_choice"]:
        if not any(m in at for m in ["場合、", "とき、", "なら、"]):
            result["has_implicit_sequential"] = True

    return result
