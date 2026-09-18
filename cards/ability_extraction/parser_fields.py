import re
from functools import lru_cache
from typing import List, Tuple

from parser_utils import (
    POSITION_KEYWORDS,
    STATE_CHANGE_PATTERNS,
    _OPTIONAL_RE,
    _SHUFFLE_RE,
    extract_by_pattern,
    extract_count,
    extract_source,
    extract_destination,
    extract_target,
    extract_card_type,
    extract_operator,
    extract_cost_limit,
    extract_cost_limit_with_operator,
    extract_picker,
    detect_require_all_hearts,
    check_original_value,
)


extract_count = lru_cache(maxsize=16384)(extract_count)
extract_source = lru_cache(maxsize=16384)(extract_source)
extract_destination = lru_cache(maxsize=16384)(extract_destination)
extract_target = lru_cache(maxsize=16384)(extract_target)
extract_card_type = lru_cache(maxsize=16384)(extract_card_type)
extract_operator = lru_cache(maxsize=16384)(extract_operator)
extract_cost_limit = lru_cache(maxsize=16384)(extract_cost_limit)
extract_cost_limit_with_operator = lru_cache(maxsize=16384)(extract_cost_limit_with_operator)
extract_picker = lru_cache(maxsize=16384)(extract_picker)
detect_require_all_hearts = lru_cache(maxsize=16384)(detect_require_all_hearts)
check_original_value = lru_cache(maxsize=16384)(check_original_value)


ICON_POSITION_TEMPLATES = {
    "{{center.png|センター}}": "center",
    "{{leftside.png|左サイド}}": "left_side",
    "{{rightside.png|右サイド}}": "right_side",
}


def detect_position_matches(text: str) -> List[Tuple[str, str]]:
    seen = set()
    matches = []
    for keyword, position in POSITION_KEYWORDS.items():
        if position not in seen and keyword in text:
            seen.add(position)
            matches.append((keyword, position))
    return matches


def detect_positions(text: str) -> List[str]:
    return [position for _, position in detect_position_matches(text)]


def detect_icon_positions(text: str) -> List[str]:
    return [position for template, position in ICON_POSITION_TEMPLATES.items() if template in text]


def format_positions(positions: List[str]) -> str:
    return ",".join(positions)


def set_cross_position_fields(target, text):
    if "position" in target:
        return False
    matched = {position for _, position in detect_position_matches(text)}
    if "left_side" in matched and "right_side" in matched:
        target["position"] = "left_side"
        target["position_compare"] = "right_side"
    elif len(matched) == 1:
        target["position"] = next(iter(matched))
    elif len(matched) > 1:
        positions = sorted(matched)
        target["position"] = positions[0]
        target["position_compare"] = positions[1]
    else:
        return False
    return True


def extract_state_change(text):
    return extract_by_pattern(text, STATE_CHANGE_PATTERNS)


def extract_optional(text: str) -> bool:
    return _OPTIONAL_RE.search(text) is not None


def extract_max(text: str) -> bool:
    return any(counter + "まで" in text for counter in ("人", "枚", "つ", "個"))


def _quoted_names(text: str) -> List[str]:
    return re.findall(r"「([^」]+)」", text)


def _split_include_exclude_chars(text):
    include_chars = []
    exclude_chars = []
    for name in _quoted_names(text):
        marker = f"「{name}」"
        idx = text.find(marker)
        if idx >= 0:
            after = text[idx + len(marker) : idx + len(marker) + 3]
            destination = exclude_chars if after.startswith("以外") else include_chars
            destination.append(name)
    return include_chars, exclude_chars


def _has_shuffle(text):
    return _SHUFFLE_RE.search(text) is not None


def detect_exclude_self(text: str) -> bool:
    return "このメンバー以外" in text or re.search(r"ほかの.*?(?:メンバー|カード)", text) is not None


def apply_extracted_fields(target, text, extractors):
    for field, extract in extractors:
        value = extract(text)
        if value:
            target[field] = value


def apply_character_filters(target, text):
    includes, excludes = _split_include_exclude_chars(text)
    if includes:
        target["characters"] = includes
    if excludes:
        target["exclude_characters"] = excludes
