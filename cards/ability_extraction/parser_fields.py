import re
from typing import List

from parser_utils import (
    STATE_CHANGE_PATTERNS,
    _OPTIONAL_RE,
    _SHUFFLE_RE,
    apply_extractors as apply_extracted_fields,
    detect_position_matches,
    extract_by_pattern,
    position_fields_from_matches,
)
ICON_POSITION_TEMPLATES = {
    "{{center.png|センター}}": "center",
    "{{leftside.png|左サイド}}": "left_side",
    "{{rightside.png|右サイド}}": "right_side",
}


def detect_positions(text: str) -> List[str]:
    return [position for _, position in detect_position_matches(text)]


def detect_icon_positions(text: str) -> List[str]:
    return [position for template, position in ICON_POSITION_TEMPLATES.items() if template in text]


# (full form, short form, code) for parenthetical position detection.
# The short form is a substring of the full form ("センター" in
# "センターエリア"), so `short in note` is exactly equivalent to the
# legacy `full in note or short in note` checks. Shared by
# _merge_parenthetical's three internal copies (condition builder,
# single/multi position detection) so the keyword set cannot drift.
NOTE_POSITION_KEYWORDS = (
    ("センターエリア", "センター", "center"),
    ("左サイドエリア", "左サイド", "left_side"),
    ("右サイドエリア", "右サイド", "right_side"),
)


def detect_note_positions(note: str) -> List[str]:
    """Position codes mentioned in a parenthetical note, sorted.

    The legacy call sites iterated center/left/right in an order that
    coincides with sorted order, so the returned list matches them
    element-for-element (including the `[0] if len == 1` single case).
    """
    found = []
    for _full, short, code in NOTE_POSITION_KEYWORDS:
        if short in note:
            found.append(code)
    return sorted(found)


def format_positions(positions: List[str]) -> str:
    return ",".join(positions)


def set_cross_position_fields(target, text):
    if "position" in target:
        return False
    fields = position_fields_from_matches(
        tuple(position for _, position in detect_position_matches(text))
    )
    if not fields:
        return False
    target.update(fields)
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


def apply_character_filters(target, text):
    includes, excludes = _split_include_exclude_chars(text)
    if includes:
        target["characters"] = includes
    if excludes:
        target["exclude_characters"] = excludes


def apply_group_exclusions(target, text):
    """Extract 『X』以外 group exclusions into `exclude_group_names` and
    remove them from an already-extracted `group_names` list.

    ONE home for the pattern previously copy-pasted between the action
    filler (`_fill_exclude_groups`) and the condition extractor
    (`_extract_generic_fields`).
    """
    exc_gns = re.findall(r"『([^』]+)』以外", text)
    if exc_gns:
        target["exclude_group_names"] = exc_gns
        if target.get("group_names"):
            target["group_names"] = [
                g for g in target["group_names"] if g not in exc_gns
            ]
