"""
Parser utilities for ability extraction.
This module contains pure utility functions for text processing, regex extraction,
pattern lists, and normalization used across the parsing pipeline.
"""

import re
import inspect
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple, Callable

# Precompiled regex patterns for performance
DIGIT_PATTERN = re.compile(r"(\d+)")
COUNT_PATTERN = re.compile(r"(\d+)枚")
PEOPLE_PATTERN = re.compile(r"(\d+)人")
COUNTER_PATTERN = re.compile(r"(\d+)つ")  # Generic counter (e.g., "3つ")
ITEM_PATTERN = re.compile(r"(\d+)個")  # Item counter (e.g., "4個")
GROUP_PATTERN = re.compile(r"『(.+?)』")
QUOTED_NAME_PATTERN = re.compile(r"「(.+?)」")
COST_PATTERN = re.compile(r"コスト(\d+)")
HEART_PATTERN = re.compile(r"{{heart_(\d+)\.png\|heart\d+}}")
BLADE_PATTERN = re.compile(r"{{icon_blade\.png\|ブレード}}")
ALL_ICON_PATTERN = re.compile(r"\{\{icon_all\.png\|ハート\}\}")


def extract_int(pattern, text, default=None):
    """Extract an integer from text using a pattern or regex."""
    if isinstance(pattern, str):
        match = re.search(pattern, text)
    else:
        match = pattern.search(text)
    if match:
        return int(match.group(1))
    return default


def extract_group_name(text):
    """Extract group name from text (e.g., 『虹ヶ咲』 -> 虹ヶ咲)."""
    match = GROUP_PATTERN.search(text)
    if match:
        return match.group(1)
    return None


def extract_quoted_name(text):
    """Extract quoted name from text (e.g., 「上原歩夢」 -> 上原歩夢)."""
    match = QUOTED_NAME_PATTERN.search(text)
    if match:
        return match.group(1)
    return None


def has_any(text, phrases):
    """Check if text contains any of the given phrases."""
    return any(phrase in text for phrase in phrases)


def strip_suffix_period(text):
    """Remove trailing period from text."""
    return text.rstrip("。")


def strip_prefix_period(text):
    """Remove leading period from text."""
    return text.lstrip("。")


def parse_optional_flag(text, phrases):
    """Check if text contains optional phrases and return boolean."""
    return any(phrase in text for phrase in phrases)


def normalize_whitespace(text):
    """Normalize whitespace in text - collapse multiple spaces to single space."""
    return re.sub(r"\s+", " ", text).strip()


def normalize_fullwidth_digits(text):
    """Normalize full-width digits and symbols to half-width (e.g., １ -> 1, ＋ -> +, − -> -, － -> -)."""
    # Handle both U+2212 (minus sign) and U+FF0D (fullwidth hyphen-minus)
    fullwidth = "０１２３４５６７８９＋−－"
    halfwidth = "0123456789+--"
    translation = str.maketrans(fullwidth, halfwidth)
    return text.translate(translation)


def normalize_text(text):
    """Apply all normalization steps to text."""
    text = normalize_whitespace(text)
    text = normalize_fullwidth_digits(text)
    text = strip_suffix_period(text)
    return text


def extract_count(text):
    """Extract count from text (e.g., '3枚' -> 3, '2人' -> 2, '3つ' -> 3, '4個' -> 4).
    Prefers count from 'N枚まで' (up to N) over the first bare 'N枚' match.
    """
    # Prefer count from "X枚まで" (e.g., "3枚まで") over the first bare \d+枚
    max_match = re.search(r"(\d+)枚まで", text)
    if max_match:
        return int(max_match.group(1))
    match = COUNT_PATTERN.search(text)
    if match:
        return int(match.group(1))
    match = PEOPLE_PATTERN.search(text)
    if match:
        return int(match.group(1))
    match = COUNTER_PATTERN.search(text)
    if match:
        return int(match.group(1))
    match = ITEM_PATTERN.search(text)
    if match:
        return int(match.group(1))
    # Fallback: bare number before 以上 (e.g. "10以上" in "ブレードの合計が10以上")
    bare = re.search(r"(\d+)以上", text)
    if bare:
        return int(bare.group(1))
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


def extract_operator(text: str) -> Optional[str]:
    """Extract comparison operator from text."""
    return extract_by_pattern(text, OPERATOR_PATTERNS)


def extract_heart_types(text):
    """Extract heart types from text (e.g., heart icons)."""
    matches = HEART_PATTERN.findall(text)
    return matches if matches else None


def extract_blade_count(text):
    """Extract blade count from text (number of blade icons)."""
    matches = BLADE_PATTERN.findall(text)
    return len(matches) if matches else 0


def _check_exclude_self_broad(text):
    """Check if text contains 'other' patterns (ほかの/他の) that imply exclude_self."""
    return "ほかの" in text or "他の" in text or "以外" in text


def check_distinct_name(text):
    """Check if text contains 'different name' pattern (名前の異なる)."""
    return "名前の異なる" in text


def check_original_value(text):
    """Check if text contains 'original value' pattern (元々持つ or bare 元々)."""
    return "元々持つ" in text or "元々" in text


def split_commas_smartly(text):
    """Split text by commas, but preserve structural commas."""
    parts = []
    current = ""
    i = 0
    while i < len(text):
        if text[i] == "、":
            if i >= 1:
                prev_char = text[i - 1]
                if prev_char == "は":
                    current += "、"
                    i += 1
                    continue
                if i >= 7 and text[i - 7 : i] == "ライブ終了時まで":
                    current += "、"
                    i += 1
                    continue
                if i >= 2 and text[i - 2 : i] == "場合":
                    current += "、"
                    i += 1
                    continue
            if i >= 3 and text[i - 3 : i] == "その後":
                parts.append(current)
                current = ""
                i += 1
                continue
            parts.append(current)
            current = ""
            i += 1
        else:
            current += text[i]
            i += 1
    if current:
        parts.append(current)
    return parts


# Main groups (large idol groups) - from rules v1.06 Appendix A
MAIN_GROUPS = {
    "μ's",
    "Aqours",
    "Saint Snow",
    "虹ヶ咲",
    "Liella!",
    "Nijigaku",
    "Liella",
    "SaintSnow",
    "Muse",
    "蓮ノ空",  # Hasunosora
    "A-RISE",
    "Sunny Passion",
}

# Subunits (smaller groups within main groups) - from rules v1.06 Appendix A
SUBUNITS = {
    "CYaRon!",
    "AZALEA",
    "Guilty Kiss",
    "Dance",
    "Qu4rtz",
    "R3BIRTH",
    "CatChu!",
    "5yncri5e!",
    "BiBi",
    "Printemps",
    "lily white",
    "DOLLCHESTRA",
    "スリーズブーケ",
    "みらくらぱーく！",
    "MIRAPARK",
    "EdelNote",
    "Edel Note",
    "KALEIDOSCORE",
    "A・ZU・NA",
    "DiverDiva",
    "AiScReam",
}

# Combined known units (both main groups and subunits)
KNOWN_UNITS = MAIN_GROUPS | SUBUNITS


def detect_group_type(group_name):
    """Detect whether a group name is a unit or character.
    Returns 'unit' if it's a known unit name, 'character' otherwise."""
    # Normalize the group name for comparison
    normalized = group_name.strip()

    # Check against known units
    if normalized in KNOWN_UNITS:
        return "unit"

    # Check for common unit patterns
    # Units often have special characters like !, ', or are in English
    if "!" in normalized or "'" in normalized:
        return "unit"

    # Japanese katakana/hiragana names are typically characters
    # Units are usually in English or have special formatting
    # This is a heuristic - may need refinement
    if any(
        c in normalized
        for c in "アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲン"
    ):
        # If it's mostly katakana and not a known unit, it's likely a character
        return "character"

    # Default to character if unknown
    return "character"


def extract_all_groups(text):
    """Extract all group names from text (『...』 and mixed 『...」 patterns)."""
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
    ("これにより公開したカードを", "revealed_cards"),
    ("公開したカードをすべて", "revealed_cards"),
    ("公開したカードを", "revealed_cards"),
    ("それらのカードの中から", "those_cards"),
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
_DISTINCT_NAME_RE = re.compile(r"名前[がの]異なる|カード名が異なる")
_DISTINCT_COST_RE = re.compile(r"コストがそれぞれ異なる")
_ORIGINAL_VALUE_RE = re.compile(r"元々持つ|元々")
_SHUFFLE_RE = re.compile(r"シャッフル")
_SAME_GROUP_RE = re.compile(r"同じグループ名")
_DIFF_GROUP_RE = re.compile(r"グループ名[がの]異なる|異なるグループ名")
_SAME_UNIT_RE = re.compile(r"同じユニット名")
_NON_STACKABLE_RE = re.compile(r"この効果は重複しない")
_OPTIONAL_RE = re.compile(r"もよい|てもよい")
_MULTIPLE_TARGETS_RE = re.compile(r"それぞれ|ずつ")
_STATE_CHANGE_WAIT = re.compile(r"ウェイト(状態)?に(す|で)(る|き)")
_STATE_CHANGE_ACTIVE = re.compile(r"アクティブにする")
_ABILITY_FILTER_HAS = re.compile(r"能力を持つ")
_ABILITY_FILTER_NO = re.compile(r"能力を持たない|能力も持たない")
_CARD_PROPERTY_BLADE = re.compile(r"ブレードハートを持たない|ブレードハートがない")
_CARD_PROPERTY_BLADE_POS = re.compile(r"ブレードハートを持つ")
_CARD_PROPERTY_SCORE = re.compile(r"\{\{icon_score\.png\|スコア\}\}を持つ")
_NEGATION_RE = re.compile(r"(がない|がなく|が\d*ない|いない|を持たない)")
_SELF_TARGET_RE = re.compile(r"この(メンバー|カード)[がは]")


def detect_require_all_hearts(text: str) -> bool:
    """Detect if heart icons in text are joined by と (AND / all required)."""
    heart_block = r"\{\{heart_\d+\.png\|heart\d+\}\}"
    hearts = re.findall(heart_block, text)
    if len(hearts) < 2:
        return False
    for i in range(len(hearts) - 1):
        if not re.search(
            re.escape(hearts[i]) + r"\s*と\s*" + re.escape(hearts[i + 1]), text
        ):
            return False
    return True


def extract_position(text: str) -> Optional[str]:
    """Extract position (center/left_side/right_side) from text."""
    return extract_by_pattern(text, list(POSITION_KEYWORDS.items()))


def extract_cost_limit(text: str) -> Optional[int]:
    """Extract cost limit value from text."""
    for pat in [
        r"元々のコスト[がは](\d+)(?:以上|以下|未満|超)",
        r"(\d+)コスト(?:以上|以下|未満|超)",
        r"コスト(\d+)(?:以上|以下|未満|超)",
        r"コスト[がは](\d+)(?:以上|以下|未満|超)",
        r"(\d+)\s*以下",
        r"以下\s*(\d+)",
        r"(\d+)\s*合計",
        r"コスト(\d+)の",
    ]:
        m = re.search(pat, text)
        if m:
            return int(m.group(1))
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
    # を-construction the literal list misses: 「デッキを(1枚)上から/下から…」
    # (e.g. pb1-014-R's mill clause). Only fires when 上から/下から directly
    # follows a デッキを phrase, so draws (デッキを1枚引く) are unaffected.
    if re.search(r"デッキを.{0,6}?上から", text):
        return "deck_top"
    if re.search(r"デッキを.{0,6}?下から", text):
        return "deck_bottom"
    return extract_by_pattern(text, SOURCE_PATTERNS)


def extract_destination(text: str) -> Optional[str]:
    """Extract destination location (TO zone)."""
    pattern_result = extract_by_pattern(text, DESTINATION_PATTERNS)
    if pattern_result:
        return pattern_result
    m = re.search(r"デッキの一番上から(\d+)枚目に置(?:いてもよい|く)", text)
    if m:
        return "deck"
    if "エネルギーカードを1枚ウェイト状態で置いてもよい" in text:
        return "energy_zone"
    if (
        "メンバーのいないエリアに登場させる" in text
        or "メンバーのいないエリアにウェイト状態で登場させる" in text
    ):
        return "empty_area"
    if "ウェイト状態で置く" in text or (
        "エネルギーカードを" in text and ("置く" in text or "置いてもよい" in text)
    ):
        return "energy_zone"
    if "登場させる" in text:
        return "stage"
    return None


def extract_target(text: str) -> Optional[str]:
    """Extract target (self/opponent/both/either)."""
    t = text.replace("自分のカードの効果", "")
    if (
        ("自分の" in t and "相手の" in t)
        or "自分と相手の" in text
        or "自分と相手は" in text
        or "自分と対戦相手は" in text
        or "自分と対戦相手の" in text
        or "自分と対戦相手" in text
    ):
        return "both"
    if "自分か相手の" in text:
        return "either"
    if "相手の" in text:
        return "opponent"
    if "自分の" in text:
        return "self"
    return None


def extract_picker(text: str) -> Optional[str]:
    """Extract who performs the blind pick in a reveal effect.

    Patterns:
      '相手は見ないで' → opponent picks from your hand
      '自分は見ないで' → you pick from opponent's hand
    Returns 'opponent', 'self', or None.
    """
    if "相手は見ないで" in text:
        return "opponent"
    if "自分は見ないで" in text:
        return "self"
    return None


def extract_card_type(text: str) -> Optional[str]:
    """Extract card type from text."""
    return extract_by_pattern(text, _CARD_TYPE_LONGEST_FIRST)



class PriorityRegistry:
    """Priority-sorted handler registry. No fragile ordering — add handlers at any priority."""

    def __init__(self, name: str = "registry"):
        self._handlers: List[Tuple[int, str, Callable]] = []
        self._name = name
        self._sorted = False

    def register(self, priority: int, name: str, handler) -> None:
        self._handlers.append((priority, name, handler))
        self._sorted = False

    def unregister(self, name: str) -> None:
        self._handlers = [(p, n, h) for p, n, h in self._handlers if n != name]
        self._sorted = False

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


def action_rule(registry: PriorityRegistry, priority: int, name: str = ""):
    """Decorator to register a handler in a PriorityRegistry."""

    def decorator(func):
        func_name = name or func.__name__
        registry.register(priority, func_name, func)
        return func

    return decorator


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


@dataclass
class ActionRule:
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

    def __post_init__(self):
        self.condition = _as_two_arg(self.condition)
        self.setter = _as_two_arg(self.setter)

    def matches(self, text: str, action: Optional[Dict] = None) -> bool:
        if self.match and self.match not in text:
            return False
        if self.match_any and not any(m in text for m in self.match_any):
            return False
        if self.match_all and not all(m in text for m in self.match_all):
            return False
        if self.exclude and self.exclude in text:
            return False
        if self.exclude_any and any(e in text for e in self.exclude_any):
            return False
        if self.condition and action is not None:
            # Arity normalized in __post_init__ — call directly. A raised
            # exception is a real predicate bug; log loudly, treat as no-match
            # (same outcome as the old silent swallow) so behavior is stable.
            try:
                if not self.condition(text, action):
                    return False
            except Exception:
                return False
        return True

    def apply(self, text: str, action: Dict) -> None:
        action["action"] = self.action
        action.update(self.defaults)
        for field, pattern in self.extract.items():
            m = re.search(pattern, text)
            if m:
                val = m.group(1)
                action[field] = int(val) if val.isdigit() else val
        if self.setter:
            try:
                self.setter(text, action)
            except Exception as e:
                # Surface real setter bugs instead of silently dropping fields.
                print(f"ActionRule({self.action}) setter raised: {e!r} on {text!r}")
        if self.extract_optional and ("もよい" in text or "してもよい" in text):
            action["optional"] = True


@dataclass
class EffectPattern:
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

    def matches(self, text: str) -> bool:
        if self.match and self.match not in text:
            return False
        if self.match_any and not any(m in text for m in self.match_any):
            return False
        if self.match_all and not all(m in text for m in self.match_all):
            return False
        if self.exclude and self.exclude in text:
            return False
        if self.exclude_any and any(e in text for e in self.exclude_any):
            return False
        if self.condition:
            try:
                return bool(self.condition(text))
            except Exception:
                return False
        return True

    def __call__(self, text: str, ctx: Optional[dict] = None) -> Optional[Dict]:
        if not self.matches(text):
            return None
        result: Dict = {"text": text, "action": self.action}
        result.update(self.defaults)
        for field, pattern in self.extract.items():
            m = re.search(pattern, text)
            if m:
                val = m.group(1)
                result[field] = int(val) if val.isdigit() else val
        if self.setter:
            try:
                self.setter(text, result)
            except Exception:
                pass
        return result
