"""
Parser utilities for ability extraction.
This module contains pure utility functions for text processing, regex extraction,
pattern lists, and normalization used across the parsing pipeline.
Single owner for shared field/position helpers and the parser's field context.
"""

import re
import inspect
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple, Callable

# Precompiled regex patterns for performance
#
# The counters card text counts with, in priority order: a text carrying both
# "3枚" and "2人" answers 枚, so this order IS the extract_count contract.
# parser_fields.extract_max reads the same list to recognise the "up to N"
# form, which is why the suffix characters are spelled exactly once.
COUNT_SUFFIXES = ("枚", "人", "つ", "個")  # 枚 cards, 人 members, つ counters, 個 items
COUNT_PATTERN, PEOPLE_PATTERN, COUNTER_PATTERN, ITEM_PATTERN = (
    re.compile(rf"(\d+){suffix}") for suffix in COUNT_SUFFIXES
)
GROUP_PATTERN = re.compile(r"『(.+?)』")
QUOTED_NAME_PATTERN = re.compile(r"「(.+?)」")

# Max split count for the clause separators (CHOICE_MARKER, DURATION_MARKER,
# "、相手は、", "その後、", "か、"). Every use splits on a first-occurrence
# clause delimiter, so this is always 1 — it lives here so parser.py and
# cost_parser.py cannot drift into disagreeing about the delimiter contract.
SPLIT_LIMIT = 1

# ======================================================================
# HEART ICON VOCABULARY (one definition, used by every heart scan)
# ======================================================================
# A heart colour reaches the parser in three renderings:
#   {{heart_03.png|heart03}}   card-data form (underscore before the id)
#   {{heart03.png|heart03}}    icon form
#   heart_03                   the bare card-data id
# These patterns are the only place any of that is spelled, so a rendering
# change is a one-line edit rather than a sweep through two dozen ad-hoc
# regexes. They live here rather than in parser.py because the shared
# helpers below need them too.
#
# HEART_ICON       matches a rendered icon, capturing nothing
# HEART_ICON_ID    matches a rendered icon, capturing the colour number
# HEART_ICON_PAIR  matches a rendered icon, capturing both halves of the name
# HEART_REF        the `heart_NN` id on its own, capturing the colour number
# HEART_HAS_REF    the same, capturing nothing (a presence test)
# HEART_LABEL      the `|heartNN}` label half of a rendered icon
HEART_ICON = r"\{\{heart_?\d+\.png\|heart\d+\}\}"
HEART_ICON_ID = r"\{\{heart_?(\d+)\.png\|heart\d+\}\}"
HEART_ICON_PAIR = r"\{\{heart_(\d+)\.png\|heart(\d+)\}\}"
HEART_REF = r"heart_(\d+)"
HEART_HAS_REF = r"heart_\d+"
# The label half of a rendered icon: the `|heart03}` that names the colour.
HEART_LABEL = r"\|(heart\d+)\}"


def heart_id(number: str) -> str:
    """Canonical `heartNN` id for a heart-colour number captured by one of the
    HEART_* patterns above.

    The captured number comes from an icon FILENAME (`heart3.png`) while the
    engine's ids are zero-padded (`heart03`). Two call sites forgot the padding
    and emitted `heart3`, which `HeartColor::from_str` does not recognise — it
    falls through to `Heart00`, so a heart requirement silently became
    colourless. Every site must go through this.
    """
    return f"heart{number.zfill(2)}"


# ======================================================================
# RESOURCE ICON VOCABULARY (one definition, used by every icon scan)
# ======================================================================
# The icons card text writes a cost or a resource with. Whether a phrase names
# or counts a resource is a vocabulary question, so the templates live here
# once rather than being spelled out at every call site.
BLADE_ICON = "{{icon_blade.png|ブレード}}"
ENERGY_ICON = "{{icon_energy.png|E}}"
ALL_HEART_ICON = "{{icon_all.png|ハート}}"
SCORE_ICON = "{{icon_score.png|スコア}}"

# The same icons as match patterns, for the few scans that need a regex rather
# than a substring test.
BLADE_ICON_RE = r"\{\{icon_blade\.png\|ブレード\}\}"
BLADE_ICON_HELD_RE = re.compile(r"{{icon_blade\.png\|ブレード}}[^得]*持つ")

# 「{{icon_score.png|スコア}}を持つ」 — the one fragment these sites test for.
HAS_SCORE_ICON = f"{SCORE_ICON}を持つ"


def count_icons(text, icons):
    """How many times any of `icons` appears in `text`."""
    return sum(text.count(icon) for icon in icons)


def strip_suffix_period(text):
    """Remove trailing period from text."""
    return text.rstrip("。")


def normalize_fullwidth_digits(text):
    """Normalize full-width digits and symbols to half-width (e.g., １ -> 1, ＋ -> +, − -> -, － -> -)."""
    # Handle both U+2212 (minus sign) and U+FF0D (fullwidth hyphen-minus)
    fullwidth = "０１２３４５６７８９＋−－"
    halfwidth = "0123456789+--"
    translation = str.maketrans(fullwidth, halfwidth)
    return text.translate(translation)


def as_int(text):
    """Parse a number that may still be in full-width form (e.g. "３" -> 3).

    Every numeric read in the parser goes through here. The card text is full
    of full-width digits, and 36 of the 38 extraction sites used to call `int()`
    on unnormalized text, so a card printing ３ where another prints 3 silently
    produced no count. Non-numeric input yields 0, which is what those sites
    already fell back to.
    """
    return int(normalize_fullwidth_digits(str(text).strip()) or 0)


# Ordered counters for extract_count: the first that appears wins. "N枚まで"
# ("up to N") is first so it outranks the bare "N枚" it contains, and the bare
# "N以上" is last because it is a fallback for text with no counter at all
# (e.g. "ブレードの合計が10以上").
_COUNT_PATTERNS: Tuple[Any, ...] = (
    re.compile(r"(\d+)枚まで"),
    COUNT_PATTERN,
    PEOPLE_PATTERN,
    COUNTER_PATTERN,
    ITEM_PATTERN,
    re.compile(r"(\d+)以上"),
)
# Ordered cost-limit shapes for extract_cost_limit, most specific first.
_COST_LIMIT_PATTERNS: Tuple[Any, ...] = tuple(
    re.compile(pattern)
    for pattern in (
        r"元々のコスト[がは](\d+)(?:以上|以下|未満|超)",
        r"(\d+)コスト(?:以上|以下|未満|超)",
        r"コスト(\d+)(?:以上|以下|未満|超)",
        r"コスト[がは](\d+)(?:以上|以下|未満|超)",
        r"(\d+)\s*以下",
        r"以下\s*(\d+)",
        r"(\d+)\s*合計",
        r"コスト(\d+)の",
    )
)


def extract_count(text):
    """Extract count from text (e.g., '3枚' -> 3, '2人' -> 2, '3つ' -> 3, '4個' -> 4).
    Prefers count from 'N枚まで' (up to N) over the first bare 'N枚' match.
    """
    for pattern in _COUNT_PATTERNS:
        match = pattern.search(text)
        if match:
            return int(match.group(1))
    return None


def extract_dynamic_count(text):
    """Extract dynamic count references (e.g., score-based, card-based, energy-based).
    Returns a dict with 'type' and 'details' if found, None otherwise.
    """
    if "数まで" in text:
        # Pattern: "Xの数まで" - count based on X
        count_match = re.search(r"(.+?)の数まで", text)
        if count_match:
            source = count_match.group(1).strip()
            return {"type": "dynamic_count", "reference": source, "mode": "max"}

    if "と同じ枚数" in text or "と同じ数" in text:
        # Pattern: "Xと同じ枚数/数" - count equals X
        # e.g. "これにより控え室に置いたカードと同じ枚数" → count = previously moved cards
        result = {
            "type": "dynamic_count",
            "reference": "previous_moved_cards",
            "mode": "equals",
        }
        return result

    if "その枚数に" in text and "を足した枚数" in text:
        # Pattern: "その枚数にNを足した枚数" - count based on the previous moved/discarded cards plus N
        count_match = re.search(r"その枚数に(\d+)を足した(?:枚数|数)", text)
        if count_match:
            return {
                "type": "dynamic_count",
                "reference": "previous_moved_cards",
                "mode": "equals",
                "calculation": "add",
                "calculation_value": int(count_match.group(1)),
            }

    # Pattern: "エネルギーカードの枚数にNを足した枚数" — under-member energy count + N
    if "下にあるエネルギーカードの枚数に" in text and "を足した枚数" in text:
        cm = re.search(r"下にあるエネルギーカードの枚数に(\d+)を足した枚数", text)
        if cm:
            return {
                "type": "dynamic_count",
                "reference": "energy_cards_under_this_member",
                "mode": "equals",
                "calculation": "add",
                "calculation_value": int(cm.group(1)),
            }

    if "に等しい枚数" in text or "に等しい数" in text:
        # Pattern: "Xに等しい枚数" - count equals X
        count_match = re.search(r"(.+?)に等しい(?:枚数|数)", text)
        if count_match:
            source = count_match.group(1).strip()
            if source == "そのカードのスコア":
                source = "selected_card_score"
            result = {"type": "dynamic_count", "reference": source, "mode": "equals"}
            # Check for calculation pattern like "スコアに2を足した数"
            calc_match = re.search(r"(.+?)に(\d+)を足した", source)
            if calc_match:
                calc_base = calc_match.group(1).strip()
                # Only use total_live_score for "合計スコア" patterns (Issue 8 on 穂乃果)
                if "合計スコア" in calc_base:
                    result["reference"] = "total_live_score"
                else:
                    result["reference"] = calc_base
                result["calculation"] = "add"
                result["calculation_value"] = int(calc_match.group(2))
                # Trim action description prefixes from reference (e.g.
                # "自分のデッキの上から、自分のステージにいるメンバーの数" → "自分のステージにいるメンバーの数")
                result["reference"] = _trim_reference_prefix(result["reference"])
            return result

    return None


def _trim_reference_prefix(ref):
    """Remove known action description prefixes from a dynamic count reference string."""
    prefixes = [
        "自分のデッキの上から、",
        "自分のデッキの上から",
        "デッキの上から、",
        "デッキの上から",
    ]
    for prefix in prefixes:
        if ref.startswith(prefix):
            ref = ref[len(prefix) :]
            break
    return ref.strip()


def extract_by_pattern(text: str, patterns: List[Tuple[str, str]]) -> Optional[str]:
    """Match text against a priority-ordered (pattern, value) list. Returns first match."""
    for pattern, value in patterns:
        if pattern in text:
            return value
    return None


def set_if_value(target: dict, field: str, value: Any) -> None:
    """Set a field only when an extractor produced a meaningful value."""
    if value is not None and value != "":
        target[field] = value


def matches_predicate(predicate: Any, text: str) -> bool:
    return predicate(text) if callable(predicate) else predicate in text


def apply_extractors(target: dict, text: str, extractors: List[Tuple[str, Callable]]) -> None:
    """Apply named scalar extractors to a target using one shared contract."""
    for field, extract in extractors:
        set_if_value(target, field, extract(text))


def iter_dict_nodes(value: Any, keys: Optional[Tuple[str, ...]] = None):
    """Yield dictionaries in a parser tree, optionally following selected keys."""
    if isinstance(value, dict):
        yield value
        child_keys = keys or tuple(value.keys())
        for key in child_keys:
            if key in value:
                yield from iter_dict_nodes(value[key], keys)
    elif isinstance(value, list):
        for item in value:
            yield from iter_dict_nodes(item, keys)


_WALK_HOOKS = (
    "enter",
    "leave",
    "child_context",
    "prepare_child",
    "after_child",
    "after_key",
)


def walk_dict_tree(
    value: Any,
    *,
    keys: Tuple[str, ...],
    list_keys: Optional[Tuple[str, ...]] = None,
    context: Any = None,
    **hooks: Any,
) -> None:
    """Walk parser dictionaries in child-key order with optional context hooks.

    `hooks` are the optional callbacks `enter`, `leave`, `child_context`,
    `prepare_child`, `after_child` and `after_key`; each is forwarded verbatim
    to every recursive step, so the walk's plumbing is stated once here instead
    of being re-spelled at each of the three recursion points. Unknown hook
    names still raise TypeError, so a typo cannot silently disable a callback.
    """
    unknown = set(hooks) - set(_WALK_HOOKS)
    if unknown:
        raise TypeError(
            f"walk_dict_tree() got unexpected keyword arguments: "
            f"{', '.join(sorted(unknown))}"
        )
    enter = hooks.get("enter")
    leave = hooks.get("leave")
    child_context = hooks.get("child_context")
    prepare_child = hooks.get("prepare_child")
    after_child = hooks.get("after_child")
    after_key = hooks.get("after_key")

    if isinstance(value, list):
        for item in value:
            walk_dict_tree(
                item, keys=keys, list_keys=list_keys, context=context, **hooks
            )
        return
    if not isinstance(value, dict):
        return

    node_context = enter(value, context) if enter else context
    for key in keys:
        child = value.get(key)
        if isinstance(child, list):
            if list_keys is not None and key not in list_keys:
                continue
            children = child
        elif isinstance(child, dict):
            children = [child]
        else:
            continue
        item_context = (
            child_context(value, node_context, key, child)
            if child_context
            else node_context
        )
        for item in children:
            if not isinstance(item, dict):
                continue
            if prepare_child:
                item = prepare_child(value, node_context, key, item)
            walk_dict_tree(item, keys=keys, context=item_context, **hooks)
            if after_child:
                after_child(value, context, key, item)
        if after_key:
            after_key(value, node_context, key)
    if leave:
        leave(value, context)


def transform_child_lists(
    value: Any,
    transform: Callable[[Any], Any],
    keys: Tuple[str, ...] = ("actions", "options"),
) -> Any:
    """Transform selected child lists bottom-up, then transform their parent."""
    if isinstance(value, dict):
        for key in keys:
            children = value.get(key)
            if isinstance(children, list):
                value[key] = [transform_child_lists(item, transform, keys) for item in children]
        return transform(value)
    if isinstance(value, list):
        return [transform_child_lists(item, transform, keys) for item in value]
    return value


def text_matches(
    text: str,
    *,
    match: str = "",
    match_any: Optional[List[str]] = None,
    match_all: Optional[List[str]] = None,
    exclude: str = "",
    exclude_any: Optional[List[str]] = None,
) -> bool:
    """Evaluate the common declarative text-matcher contract."""
    return (
        (not match or match in text)
        and (not match_any or any(item in text for item in match_any))
        and (not match_all or all(item in text for item in match_all))
        and (not exclude or exclude not in text)
        and (not exclude_any or all(item not in text for item in exclude_any))
    )


def _coerce_capture(value: str) -> Any:
    """Convert numeric regex captures while preserving textual captures."""
    return int(value) if value.isdigit() else value


def extract_operator(text: str) -> Optional[str]:
    """Extract comparison operator from text."""
    return extract_by_pattern(text, OPERATOR_PATTERNS)


def extract_all_groups(text):
    """Extract all group names from text (『...』 and mixed 『...」 patterns).

    Group names are read straight out of the quoted text, so nothing needs a
    hand-maintained list of the known units — that is why there is no such list
    here, and why adding one would only add a place for it to go stale.
    """
    matches = GROUP_PATTERN.findall(text)
    # Also handle mixed brackets: 『name」 (opening 『 but closing 」)
    matches += re.findall(r"『([^』」]+)」", text)
    return matches if matches else []


def extract_all_quoted_names(text):
    """Extract all quoted names from text (「...」 patterns)."""
    matches = QUOTED_NAME_PATTERN.findall(text)
    return matches if matches else []


def annotate_tree(value, text):
    """Attach source text to every parsed dict in a tree."""
    if not text or value is None:
        return value
    if isinstance(value, dict):
        value.setdefault("text", text)
        for item in value.values():
            annotate_tree(item, text)
    elif isinstance(value, list):
        for item in value:
            annotate_tree(item, text)
    return value


# ============== SHARED PATTERN LISTS ==============
# These are used by both parser.py and external tools.
# They live here so they can be imported without loading the full parser.

SOURCE_PATTERNS: List[Tuple[str, str]] = [
    # Hardcoded high-priority patterns from extract_source()
    ("デッキの一番上からカードを", "deck_top"),
    ("デッキの一番上のカードを", "deck_top"),
    ("これにより公開されたほかのすべてのカードを", "revealed_remaining"),
    ("これにより公開したカードを", "selected_cards"),
    ("公開したカードをすべて", "revealed_cards"),
    ("公開したカードを", "revealed_cards"),
    ("それらのカードの中から", "those_cards"),
    # Reference back to the cards THIS effect just placed in the waitroom
    # (「その後、これにより控え室に置いたカードの中から…」) — same
    # preceding-move pool as それらのカード.
    ("これにより控え室に置いたカードの中から", "those_cards"),
    ("そのライブカードを", "those_cards"),
    ("このカードを手札に加えてもよい", "revealed_cards"),
    ("手札にある", "hand"),
    ("エネルギー置き場にある", "energy_zone"),
    # Discard/waitroom patterns before stage patterns, since effect texts
    # may reference both stage (in count specification) and waiting room
    # (as actual card source). The waiting room should take priority.
    ("自分の控え室にある", "discard"),
    ("控え室からライブカード", "discard"),
    ("控え室を", "discard"),
    ("控え室にある", "discard"),
    ("控え室から", "discard"),
    ("相手の控え室にある", "discard"),
    ("相手の控え室から", "discard"),
    ("手札を", "hand"),
    ("手札の", "hand"),
    ("手札から", "hand"),
    ("からライブカード", "discard"),
    ("メンバー1人の下にある", "under_member"),
    ("メンバーの下にある", "under_member"),
    ("自分の成功ライブカード置き場にある", "success_live_zone"),
    ("自分のエネルギーデッキから", "energy_deck"),
    ("エネルギーデッキから", "energy_deck"),
    ("エールにより公開された", "revealed_cards"),
    ("ステージにいる", "stage"),
    ("ステージから", "stage"),
    # Standard patterns (longest-first for correct matching)
    ("デッキの一番下から", "deck_bottom"),
    ("デッキの一番下のカードを", "deck_bottom"),
    ("デッキの下から", "deck_bottom"),
    ("デッキの上から", "deck_top"),
    ("デッキから", "deck"),
    ("山札から", "deck"),
    ("エネルギー置き場から", "energy_zone"),
    ("ライブカード置き場から", "live_card_zone"),
    ("成功ライブカード置き場から", "success_live_zone"),
]


DESTINATION_PATTERNS: List[Tuple[str, str]] = [
    # Hardcoded high-priority patterns from extract_destination()
    ("デッキの一番上に置いてもよい", "deck_top"),
    ("デッキの一番上か一番下に置く", "deck_top_or_bottom"),
    ("デッキの一番上か一番下に置き", "deck_top_or_bottom"),
    ("デッキの一番上か一番下に置いて", "deck_top_or_bottom"),
    ("デッキの一番上から4枚目に置く", "deck_position_4"),
    ("デッキの一番上から4枚目に置き", "deck_position_4"),
    ("デッキの一番上に置く", "deck_top"),
    ("デッキの一番上に置き", "deck_top"),
    ("デッキの一番上に置いて", "deck_top"),
    ("デッキの上に置く", "deck_top"),
    ("デッキの上に置き", "deck_top"),
    ("デッキの上に置いて", "deck_top"),
    ("デッキの一番下に置く", "deck_bottom"),
    ("デッキの一番下に置いて", "deck_bottom"),
    ("デッキの一番下に置き", "deck_bottom"),
    ("デッキの下に置いてもよい", "deck_bottom"),
    ("デッキの下に置く", "deck_bottom"),
    ("デッキの下に置き", "deck_bottom"),
    ("デッキの下に置いて", "deck_bottom"),
    ("山札の上に置く", "deck_top"),
    ("山札の下に置く", "deck_bottom"),
    ("エネルギーデッキに置く", "energy_deck"),
    ("エネルギーデッキに置いてもよい", "energy_deck"),
    ("エネルギーデッキに置いて", "energy_deck"),
    ("エネルギー・デッキに置く", "energy_deck"),
    ("エネルギー・デッキに置いてもよい", "energy_deck"),
    ("エネルギー・デッキに置いて", "energy_deck"),
    ("そのメンバーの下に置く", "under_member"),
    ("メンバーの下に置く", "under_member"),
    ("メンバーの下に置いて", "under_member"),
    ("メンバーの下に置き", "under_member"),
    ("の下に置く", "under_member"),
    ("の下に置いて", "under_member"),
    ("の下に置き", "under_member"),
    ("ライブカード置き場に置いてもよい", "live_card_zone"),
    ("表向きでライブカード置き場に置く", "live_card_zone"),
    ("いたエリアに", "same_area"),
    ("置かれていたエリアに", "same_area"),
    ("控え室に送る", "discard"),
    ("デッキに戻す", "deck"),
    ("デッキに置く", "deck"),
    ("控え室に置く", "discard"),
    ("控え室に置いて", "discard"),
    ("控え室に置き", "discard"),
    ("枚控え室に置く", "discard"),
    ("枚控え室に置いて", "discard"),
    ("手札に加える", "hand"),
    ("手札に加えて", "hand"),
    ("手札に置く", "hand"),
    ("手札に戻す", "hand"),
    ("手札に戻し", "hand"),
    ("デッキの上に戻す", "deck_top"),
    ("デッキの上に戻し", "deck_top"),
    ("エネルギーデッキに戻す", "energy_deck"),
    ("ステージに置く", "stage"),
    ("ステージに登場させる", "stage"),
    ("エネルギー置き場に置く", "energy_zone"),
    ("エネルギーゾーンに置く", "energy_zone"),
    ("成功ライブカード置き場に置く", "success_live_zone"),
    ("ライブカード置き場に置く", "live_card_zone"),
    ("メンバーのいないエリア", "empty_area"),
    ("そのメンバーがいたエリア", "same_area"),
    ("このメンバーの下に置く", "under_member"),
    ("このメンバーの下に置いて", "under_member"),
    ("このメンバーの下に置き", "under_member"),
    ("登場したメンバーの下に置く", "under_member"),
    ("登場したメンバーの下に置いて", "under_member"),
    ("登場したメンバーの下に置き", "under_member"),
]

STATE_CHANGE_PATTERNS: List[Tuple[str, str]] = [
    ("ウェイトにする", "wait"),
    ("ウェイトにしてもよい", "wait"),
    ("ウェイトにし", "wait"),
    ("ウェイト状態で置く", "wait"),
    ("ウェイト状態で登場させる", "wait"),
    ("アクティブにする", "active"),
    ("アクティブにし", "active"),
    ("アクティブにしてもよい", "active"),
]

LOCATION_PATTERNS: List[Tuple[str, str]] = [
    ("成功ライブカード置き場", "success_live_card_zone"),
    ("ライブカード置き場", "live_card_zone"),
    ("控え室", "discard"),
    ("手札", "hand"),
    ("ステージ", "stage"),
    ("デッキ", "deck"),
    ("エネルギーデッキ", "energy_deck"),
    ("エネルギー置き場", "energy_zone"),
]

CARD_TYPE_PATTERNS: List[Tuple[str, str]] = [
    ("メンバーカード", "member_card"),
    ("メンバー", "member_card"),
    ("ライブカード", "live_card"),
    ("エネルギーカード", "energy_card"),
]

OPERATOR_PATTERNS: List[Tuple[str, str]] = [
    ("以上", ">="),
    ("以下", "<="),
    ("より少ない", "<"),
    ("より多い", ">"),
    ("未満", "<"),
    ("超", ">"),
]

# ============== POSITION KEYWORDS ==============
# Single source of truth for position keyword -> canonical id. parser.py owns the
# position-detection functions; this is the only copy of the map.
POSITION_KEYWORDS: Dict[str, str] = {
    "センターエリア": "center",
    "左サイドエリア": "left_side",
    "右サイドエリア": "right_side",
    "センター": "center",
    "左サイド": "left_side",
    "右サイド": "right_side",
    "正面": "front",
}

# ============== CARD TYPE PATTERNS (sorted longest-first) ==============
_CARD_TYPE_LONGEST_FIRST: List[Tuple[str, str]] = sorted(
    CARD_TYPE_PATTERNS, key=lambda x: -len(x[0])
)

# ============== PRE-COMPILED REGEXES ==============
_ALL_KW_RE = re.compile(r"すべての|全ての|全部の|全て|全員|全体|カードをすべて")
_SHUFFLE_RE = re.compile(r"シャッフル")
_OPTIONAL_RE = re.compile(r"もよい|てもよい")


def check_original_value(text):
    """Check if text contains the 'original value' pattern (元々)."""
    return "元々" in text


def detect_require_all_hearts(text: str) -> bool:
    """Detect if heart icons in text are joined by と (AND / all required)."""
    hearts = re.findall(HEART_ICON, text)
    if len(hearts) < 2:
        return False
    for i in range(len(hearts) - 1):
        if not re.search(
            re.escape(hearts[i]) + r"\s*と\s*" + re.escape(hearts[i + 1]), text
        ):
            return False
    return True


def extract_cost_limit(text: str) -> Optional[int]:
    """Extract cost limit value from text."""
    for pattern in _COST_LIMIT_PATTERNS:
        match = pattern.search(text)
        if match:
            return int(match.group(1))
    return None


def extract_cost_limit_with_operator(text: str) -> Optional[Tuple[int, str]]:
    """Extract cost limit value AND operator together.
    Returns (value, operator) e.g. (13, '>=') or None if no match."""
    m = re.search(r"コスト(\d+)(以上|以下|より大きい|より小さい|未満)", text)
    if m:
        value = int(m.group(1))
        op_map = {
            "以上": ">=",
            "以下": "<=",
            "より大きい": ">",
            "より小さい": "<",
            "未満": "<",
        }
        return (value, op_map.get(m.group(2), ">="))
    return None


def extract_cost_values(text: str) -> Optional[List[int]]:
    """Extract a discrete set of allowed cost values joined by か (or),
    e.g. "コストが10か20のメンバーカード" → [10, 20]. Returns None if the text
    does not use the か (or) multi-value form."""
    m = re.search(r"コスト[がは](\d+か\d+)", text)
    if m:
        parts = re.findall(r"\d+", m.group(1))
        if len(parts) >= 2:
            return [int(p) for p in parts]
    return None


def detect_card_property(text: str) -> Optional[Tuple[str, bool]]:
    """Detect card property patterns from text.
    Returns (property_name, is_negated) or None.
    Only matches NEGATED patterns (持たない/がない) — positive patterns are
    handled by the text itself and don't need a card_property filter."""
    if "ブレードハートを持たない" in text or "ブレードハートがない" in text:
        return ("has_blade_heart", True)
    return None


_DECK_POSITION_RE = re.compile(r"デッキの一番上から(\d+)枚目に置(?:いてもよい|く)")
# The を-construction the literal lists miss: 「デッキを(1枚)上から/下から…」.
# Only fires when 上から/下から directly follows a デッキを phrase, so draws
# (デッキを1枚引く) are unaffected.
_DECK_FROM_TOP_RE = re.compile(r"デッキを.{0,6}?上から")
_DECK_FROM_BOTTOM_RE = re.compile(r"デッキを.{0,6}?下から")


def extract_source(text: str) -> Optional[str]:
    """Extract source location (FROM zone).

    Explicit FROM-markers (patterns containing から/からの) take priority over
    located-at descriptors (e.g. メンバーの下にある). A phrase like
    「このメンバーの下にあるエネルギーカードの枚数に1を足した枚数」 is a COUNT
    reference, not the move's origin; the real origin is the earlier
    「エネルギーデッキから」. When both kinds appear, the earliest FROM-marker
    occurrence wins. With no FROM-marker anywhere, fall back to first match.
    """
    best_value: Optional[str] = None
    best_pos = len(text)
    for pattern, value in SOURCE_PATTERNS:
        if "から" not in pattern:
            continue
        pos = text.find(pattern)
        if pos != -1 and pos < best_pos:
            best_value = value
            best_pos = pos
    if best_value is not None:
        return best_value
    if _DECK_FROM_TOP_RE.search(text):
        return "deck_top"
    if _DECK_FROM_BOTTOM_RE.search(text):
        return "deck_bottom"
    return extract_by_pattern(text, SOURCE_PATTERNS)


# Destination fallbacks for shapes DESTINATION_PATTERNS does not list, in
# priority order. Two of them answer energy_zone for different phrasings of
# the same move; they stay as separate rows because the empty-area phrase sits
# between them, and the narrower one has to win first.
_ENERGY_CARD_WAIT = "エネルギーカードを1枚ウェイト状態で置いてもよい"
_EMPTY_AREA = "メンバーのいないエリア"
_ENERGY_CARD_PLACED = "エネルギーカードを"
_WAIT_PLACE = "ウェイト状態で置く"
_PLACE_VERBS = ("置く", "置いてもよい")
_DESTINATION_FALLBACKS: Tuple[Tuple[Callable[[str], bool], str], ...] = (
    (lambda t: bool(_DECK_POSITION_RE.search(t)), "deck"),
    (lambda t: _ENERGY_CARD_WAIT in t, "energy_zone"),
    (lambda t: f"{_EMPTY_AREA}に登場させる" in t or f"{_EMPTY_AREA}にウェイト状態で登場させる" in t, "empty_area"),
    (lambda t: _WAIT_PLACE in t or (_ENERGY_CARD_PLACED in t and any(v in t for v in _PLACE_VERBS)), "energy_zone"),
    (lambda t: "登場させる" in t, "stage"),
)


def extract_destination(text: str) -> Optional[str]:
    """Extract destination location (TO zone)."""
    pattern_result = extract_by_pattern(text, DESTINATION_PATTERNS)
    if pattern_result:
        return pattern_result
    for matches, zone in _DESTINATION_FALLBACKS:
        if matches(text):
            return zone
    return None


# "both" is checked first because a phrase naming both sides also contains
# 自分の and 相手の on their own, so the single-side patterns below would
# otherwise claim it. Within that pair 相手 wins, because these read the
# original text: 「自分のカードの効果」 is stripped above so it cannot form the
# "both" test, but a text carrying it alongside 相手の is still an opponent
# target, not a self one.
_BOTH_TARGET_PHRASES = (
    "自分と相手の",
    "自分と相手は",
    "自分と対戦相手は",
    "自分と対戦相手の",
    "自分と対戦相手",
)
_SINGLE_TARGET_PATTERNS: List[Tuple[str, str]] = [
    ("相手の", "opponent"),
    ("自分の", "self"),
]
_EITHER_TARGET_PHRASE = "自分か相手の"

# 「相手は見ないで」 → the opponent picks from your hand; 「自分は見ないで」 →
# you pick from the opponent's hand.
_PICKER_PATTERNS: List[Tuple[str, str]] = [
    ("相手は見ないで", "opponent"),
    ("自分は見ないで", "self"),
]


def extract_target(text: str) -> Optional[str]:
    """Extract target (self/opponent/both/either)."""
    stripped = text.replace("自分のカードの効果", "")
    names_both_sides = (
        ("自分の" in stripped and "相手の" in stripped)
        or any(phrase in text for phrase in _BOTH_TARGET_PHRASES)
    )
    if names_both_sides:
        return "both"
    if _EITHER_TARGET_PHRASE in text:
        return "either"
    return extract_by_pattern(text, _SINGLE_TARGET_PATTERNS)


def extract_picker(text: str) -> Optional[str]:
    """Extract who performs the blind pick in a reveal effect.

    Returns 'opponent' (相手は見ないで → opponent picks from your hand),
    'self' (自分は見ないで → you pick from the opponent's hand), or None.
    """
    return extract_by_pattern(text, _PICKER_PATTERNS)


def extract_card_type(text: str) -> Optional[str]:
    """Extract card type from text."""
    return extract_by_pattern(text, _CARD_TYPE_LONGEST_FIRST)


@dataclass(frozen=True)
class FieldContext:
    text: str
    source: Optional[str]
    destination: Optional[str]
    target: Optional[str]
    card_type: Optional[str]
    count: Optional[int]
    positions: Tuple[str, ...]

    @property
    def zone(self) -> Optional[str]:
        return self.source

    def value(self, field: str) -> Any:
        if field == "position":
            return self.positions[0] if self.positions else None
        return getattr(self, field)

    def apply(self, target: dict, policy: "FieldPolicy") -> None:
        for field in policy.fields:
            if field == "position":
                if "position" in target and "position" not in policy.overwrite_fields:
                    continue
                position_fields = position_fields_from_matches(self.positions)
                if position_fields:
                    for key, value in position_fields.items():
                        if key not in target or key in policy.overwrite_fields:
                            target[key] = value
                continue
            hook = policy.hooks.get(field)
            value = hook(self) if hook is not None else self.value(field)
            if value is None or value == "":
                continue
            if field not in target or field in policy.overwrite_fields:
                target[field] = value
        if policy.after_apply is not None:
            policy.after_apply(target, self)


@dataclass(frozen=True)
class FieldPolicy:
    fields: Tuple[str, ...]
    overwrite_fields: Tuple[str, ...] = ()
    hooks: Dict[str, Callable[[FieldContext], Any]] = field(default_factory=dict)
    after_apply: Optional[Callable[[dict, FieldContext], None]] = None


def detect_position_matches(text: str) -> List[Tuple[str, str]]:
    seen = set()
    matches = []
    for keyword, position in POSITION_KEYWORDS.items():
        if position not in seen and keyword in text:
            seen.add(position)
            matches.append((keyword, position))
    return matches


def position_fields_from_matches(positions: Tuple[str, ...]) -> Dict[str, str]:
    matched = set(positions)
    if not matched:
        return {}
    if "left_side" in matched and "right_side" in matched:
        return {"position": "left_side", "position_compare": "right_side"}
    if len(matched) == 1:
        return {"position": next(iter(matched))}
    ordered = sorted(matched)
    return {"position": ordered[0], "position_compare": ordered[1]}


def extract_field_context(text: str, **extractors: Callable[[str], Any]) -> FieldContext:
    defaults = {
        "source": extract_source,
        "destination": extract_destination,
        "target": extract_target,
        "card_type": extract_card_type,
        "count": extract_count,
    }
    defaults.update(extractors)
    position_matches = detect_position_matches(text)
    return FieldContext(
        text=text,
        source=defaults["source"](text),
        destination=defaults["destination"](text),
        target=defaults["target"](text),
        card_type=defaults["card_type"](text),
        count=defaults["count"](text),
        positions=tuple(position for _, position in position_matches),
    )


ACTION_SOURCE_POLICY = FieldPolicy(
    fields=("source",),
    overwrite_fields=("source",),
)
ACTION_DESTINATION_POLICY = FieldPolicy(
    fields=("destination",),
    overwrite_fields=("destination",),
)
ACTION_CARD_FIELDS_POLICY = FieldPolicy(
    fields=("count", "card_type", "target"),
    overwrite_fields=("count", "card_type", "target"),
)
ACTION_POSITION_POLICY = FieldPolicy(fields=("position",))
COST_CARD_FIELDS_POLICY = ACTION_CARD_FIELDS_POLICY
COST_REVEAL_FIELDS_POLICY = FieldPolicy(
    fields=("card_type", "target"),
    overwrite_fields=("card_type", "target"),
)


class PriorityRegistry:
    """Priority-sorted handler registry. No fragile ordering — add handlers at any priority."""

    def __init__(self, name: str = "registry"):
        self._handlers: List[Tuple[int, str, Callable]] = []
        self._post_normalizers: Dict[str, Callable] = {}
        self._name = name
        self._sorted = False

    def register(self, priority: int, name: str, handler, post_normalize: Optional[Callable] = None) -> None:
        self._handlers.append((priority, name, handler))
        if post_normalize is not None:
            self._post_normalizers[name] = post_normalize
        self._sorted = False

    def unregister(self, name: str) -> None:
        self._handlers = [(p, n, h) for p, n, h in self._handlers if n != name]
        self._post_normalizers.pop(name, None)
        self._sorted = False

    def get_post_normalize(self, name: str) -> Optional[Callable]:
        """Return the post-normalize callback registered for `name`, or None."""
        return self._post_normalizers.get(name)

    def sorted_handlers(self):
        if not self._sorted:
            self._handlers.sort(key=lambda x: (x[0], x[1]))
            self._sorted = True
        return self._handlers

    def dispatch(
        self, text: str, ctx: Optional[dict] = None, *, default: Any = None
    ) -> Any:
        """Run handlers in priority order. First non-None return wins."""
        if ctx is None:
            ctx = {}
        for _priority, _name, handler in self.sorted_handlers():
            result = handler(text, ctx)
            if result is not None:
                return result
        return default

    def __repr__(self):
        return f"PriorityRegistry({self._name}, {len(self._handlers)} handlers)"


def _accepts_two_positional(f: Callable) -> bool:
    """True when `f` can be called with two positional args (text, action)."""
    try:
        sig = inspect.signature(f)
    except (TypeError, ValueError):
        return True  # builtins / C callables — assume permissive
    for p in sig.parameters.values():
        if p.kind is inspect.Parameter.VAR_POSITIONAL:
            return True
        if p.kind is inspect.Parameter.KEYWORD_ONLY and p.default is inspect.Parameter.empty:
            return False
    n = sum(
        1
        for p in sig.parameters.values()
        if p.kind in (inspect.Parameter.POSITIONAL_ONLY, inspect.Parameter.POSITIONAL_OR_KEYWORD)
    )
    return n >= 2


def _as_two_arg(f: Optional[Callable]) -> Optional[Callable]:
    """Normalize a 1-arg predicate/setter to the (text, action) contract.

    Registered rules historically mixed 1-arg and 2-arg lambdas; dispatch
    papered over the difference with try/except TypeError. Arity is now fixed
    once here so dispatch can call unconditionally.
    """
    if f is None or _accepts_two_positional(f):
        return f

    def wrapped(text, action=None, _f=f):
        return _f(text)

    wrapped.__name__ = getattr(f, "__name__", "wrapped")
    return wrapped


def _apply_rule_fields(rule, text: str, result: Dict[str, Any]) -> None:
    result["action"] = rule.action
    result.update(rule.defaults)
    for field, pattern in rule.extract.items():
        match = re.search(pattern, text)
        if match:
            result[field] = _coerce_capture(match.group(1))


class _TextRule:
    """The shared text-match contract behind the declarative rule dataclasses.

    ActionRule, EffectPattern and ConditionPattern all declare the same five
    matcher fields — `match`, `match_any`, `match_all`, `exclude`,
    `exclude_any` — and each used to inline its own copy of the `text_matches`
    keyword call. Three copies of that call is three places for the contract to
    drift, so it lives here and the subclasses say only what they do with a hit.

    The subclasses keep their own field lists on purpose: ConditionPattern is
    constructed positionally as `ConditionPattern(name, tier, handler)`, so its
    field order is part of its call API and cannot be inherited from a base
    whose defaulted matcher fields would sort ahead of it.
    """

    def matches_text(self, text: str) -> bool:
        """True when `text` satisfies this rule's match/match_*/exclude fields."""
        return text_matches(
            text,
            match=self.match,
            match_any=self.match_any,
            match_all=self.match_all,
            exclude=self.exclude,
            exclude_any=self.exclude_any,
        )


@dataclass
class ActionRule(_TextRule):
    """Declarative action parsing rule: text pattern → action type + field defaults.

    Usage:
        ActionRule(match_any=["シャッフルする", "シャッフルして"], action="shuffle",
                   defaults={"target": "deck"})
        ActionRule(match="カードを1枚引いてもよい", action="draw_card",
                   defaults={"count": 1, "optional": True})
    """

    action: str
    match: str = ""  # simple substring check
    match_any: List[str] = field(default_factory=list)  # any of these substrings
    match_all: List[str] = field(default_factory=list)  # all of these substrings
    exclude: str = ""  # exclude if this substring found
    exclude_any: List[str] = field(
        default_factory=list
    )  # exclude if any of these found
    defaults: Dict[str, Any] = field(default_factory=dict)  # fields to merge
    extract: Dict[str, str] = field(default_factory=dict)  # field → regex
    condition: Optional[Callable] = None  # complex predicate (text, action) → bool
    setter: Optional[Callable] = None  # complex setter (text, action) → None
    extract_optional: bool = False  # auto-detect optional from "もよい"
    name: str = ""

    def __post_init__(self):
        if not self.name:
            self.name = self.action
        self.condition = _as_two_arg(self.condition)
        self.setter = _as_two_arg(self.setter)

    def matches(self, text: str, action: Optional[Dict] = None) -> bool:
        if not self.matches_text(text):
            return False
        if self.condition and action is not None:
            try:
                return bool(self.condition(text, action))
            except Exception:
                return False
        return True

    def apply(self, text: str, action: Dict) -> None:
        _apply_rule_fields(self, text, action)
        if self.setter:
            try:
                self.setter(text, action)
            except Exception as e:
                # Surface real setter bugs instead of silently dropping fields.
                print(f"ActionRule({self.action}) setter raised: {e!r} on {text!r}")
        if self.extract_optional and ("もよい" in text or "してもよい" in text):
            action["optional"] = True


@dataclass
class EffectPattern(_TextRule):
    """Declarative effect parsing pattern: text pattern → effect dict.

    An EffectPattern is callable with (text, ctx) → dict | None, making it
    directly registerable in PriorityRegistry as a handler.

    Usage:
        EffectPattern(match="を失う", action="gain_resource",
                      defaults={"sign": "negative"})
    """

    action: str
    match: str = ""
    match_any: List[str] = field(default_factory=list)
    match_all: List[str] = field(default_factory=list)
    exclude: str = ""
    exclude_any: List[str] = field(default_factory=list)
    defaults: Dict[str, Any] = field(default_factory=dict)
    extract: Dict[str, str] = field(default_factory=dict)
    condition: Optional[Callable] = None
    setter: Optional[Callable] = None
    handler: Optional[Callable] = None

    def matches(self, text: str) -> bool:
        if self.handler is not None:
            return True
        if not self.matches_text(text):
            return False
        if self.condition:
            return bool(self.condition(text))
        return True

    def __call__(self, text: str, ctx: Optional[dict] = None) -> Optional[Dict]:
        if self.handler is not None:
            return self.handler(text)
        if not self.matches(text):
            return None
        result: Dict = {"text": text}
        _apply_rule_fields(self, text, result)
        if self.setter:
            self.setter(text, result)
        return result


@dataclass
class ConditionPattern(_TextRule):
    name: str = ""
    tier: int = 0
    handler: Optional[Callable] = None
    match: str = ""
    match_any: List[str] = field(default_factory=list)
    match_all: List[str] = field(default_factory=list)
    exclude: str = ""
    exclude_any: List[str] = field(default_factory=list)
    condition: Optional[Callable] = None
    setter: Optional[Callable] = None

    def __call__(self, text: str, ctx: Optional[dict] = None) -> Optional[Dict]:
        if self.handler is not None:
            return self.handler(text)
        if not self.matches_text(text):
            return None
        if self.condition and not self.condition(text):
            return None
        result: Dict = {"text": text}
        if self.setter:
            self.setter(text, result)
        return result