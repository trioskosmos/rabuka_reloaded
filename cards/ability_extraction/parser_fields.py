import re
from typing import List

from parser_utils import (
    COUNT_SUFFIXES,
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
    """True when the text caps a count with 「まで」 (up to N)."""
    return any(counter + "まで" in text for counter in COUNT_SUFFIXES)


_EXCLUDE_SUFFIX = "以外"


def _quoted_names(text: str) -> List[str]:
    return re.findall(r"「([^」]+)」", text)


def _split_include_exclude_chars(text):
    """Split 「name」 quotes into the ones a filter keeps and the 以外 ones it drops.

    「X」以外 names something the effect skips; every other 「X」 names
    something it applies to. The closing bracket is always present, because the
    name came out of the same text.
    """
    include_chars = []
    exclude_chars = []
    for name in _quoted_names(text):
        tail = text.split(f"「{name}」", 1)[1]
        destination = exclude_chars if tail.startswith(_EXCLUDE_SUFFIX) else include_chars
        destination.append(name)
    return include_chars, exclude_chars


def _has_shuffle(text):
    return _SHUFFLE_RE.search(text) is not None


# 「『X』以外」 excludes a GROUP and 「「X」以外」 excludes a CHARACTER. Neither is about
# the activating card: they are handled by apply_group_exclusions and
# _split_include_exclude_chars respectively. They are blanked out before the self-exclusion
# patterns are tested, so a bare `以外` test cannot turn a group exclusion into a bogus
# exclude_self — which is exactly what one call site did.
_GROUP_OR_CHARACTER_EXCLUSION = re.compile(r"[『「][^『」「」]*[』」]以外")

# The one place 「does this printed clause exclude the activating card?」 is decided.
# Every form any call site ever recognised is listed here, so a clause resolves the same
# way no matter which builder processes it. The previous arrangement spread these eight
# forms across eleven call sites with six different widenings; five of seven sample clauses
# resolved differently depending on the path, and they failed in BOTH directions — the
# broad site invented exclude_self for 「『μ's』以外のメンバー」 and missed 「これを除く
# メンバー」. See docs/PARSER_CONSOLIDATION_SWEEP.md §F1.
_SELF_EXCLUSION_PATTERNS = (
    re.compile(r"この(?:メンバー|カード)以外"),
    re.compile(r"自分以外"),
    re.compile(r"これを除く"),
    re.compile(r"ほかの.*?(?:メンバー|カード)"),
    re.compile(r"他の.*?(?:メンバー|カード)"),
    re.compile(r"ほかの"),
    re.compile(r"他の"),
)


def detect_exclude_self(text: str) -> bool:
    """True when the printed clause excludes the activating card.

    Recognises このメンバー以外 / このカード以外 / 自分以外 / これを除く / ほかの… /
    他の… , in both the 「ほかのメンバー」 and bare 「ほかの」 spellings.

    Deliberately NOT recognised, because they name something other than the activator:
    「『X』以外」 (a group) and 「「X」以外」 (a character).
    """
    candidate = _GROUP_OR_CHARACTER_EXCLUSION.sub("", text)
    return any(p.search(candidate) for p in _SELF_EXCLUSION_PATTERNS)


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
