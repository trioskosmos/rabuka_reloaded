"""
Parser utilities for ability extraction.
This module contains pure utility functions for text processing, regex extraction,
pattern lists, and normalization used across the parsing pipeline.
"""

import re
from typing import Dict, List, Tuple

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


def extract_cost(text):
    """Extract cost value from text (e.g., 'コスト3' -> 3)."""
    match = COST_PATTERN.search(text)
    if match:
        return int(match.group(1))
    return None


def extract_heart_types(text):
    """Extract heart types from text (e.g., heart icons)."""
    matches = HEART_PATTERN.findall(text)
    return matches if matches else None


def extract_blade_count(text):
    """Extract blade count from text (number of blade icons)."""
    matches = BLADE_PATTERN.findall(text)
    return len(matches) if matches else 0


def create_fallback(raw_text):
    """Create a fallback result with raw_text."""
    return {"raw_text": raw_text}


def is_fallback(result):
    """Check if a result is a fallback (contains raw_text)."""
    return isinstance(result, dict) and "raw_text" in result


def merge_position_requirement(result, action):
    """Merge position_requirement from action into result if present."""
    if "position_requirement" in action:
        result["position_requirement"] = action["position_requirement"]
        del action["position_requirement"]
    return result


def check_exclude_self(text):
    """Check if text contains 'other' patterns (ほかの/他の) that imply exclude_self."""
    return "ほかの" in text or "他の" in text or "以外" in text


def check_distinct_name(text):
    """Check if text contains 'different name' pattern (名前の異なる)."""
    return "名前の異なる" in text


def check_original_value(text):
    """Check if text contains 'original value' pattern (元々持つ)."""
    return "元々持つ" in text


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
    """Extract all group names from text (『...』 patterns)."""
    matches = GROUP_PATTERN.findall(text)
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
    ("それらのカードの中から", "those_cards"),
    ("そのライブカードを", "those_cards"),
    ("このカードを手札に加えてもよい", "revealed_cards"),
    ("手札にある", "hand"),
    ("ステージにいる", "stage"),
    ("ステージから", "stage"),
    ("自分の成功ライブカード置き場にある", "success_live_zone"),
    ("エールにより公開された", "revealed_cards"),
    ("メンバーの下にある", "under_member"),
    ("メンバー1人の下にある", "under_member"),
    ("自分の控え室にある", "discard"),
    ("控え室からライブカード", "discard"),
    ("控え室を", "discard"),
    ("手札を", "hand"),
    ("手札の", "hand"),
    ("手札から", "hand"),
    # Standard patterns (longest-first for correct matching)
    ("デッキの一番下から", "deck_bottom"),
    ("デッキの上から", "deck_top"),
    ("エネルギーデッキから", "energy_deck"),
    ("デッキから", "deck"),
    ("山札から", "deck"),
    ("エネルギー置き場から", "energy_zone"),
    ("控え室か ら", "discard"),
    ("控え室にある", "discard"),
    ("控え室から", "discard"),
    ("相手の控え室にある", "discard"),
    ("相手の控え室から", "discard"),
    ("からライブカード", "discard"),
    ("手札から", "hand"),
    ("ステージから", "stage"),
    ("ライブカード置き場から", "live_card_zone"),
    ("成功ライブカード置き場から", "success_live_zone"),
]


DESTINATION_PATTERNS: List[Tuple[str, str]] = [
    # Hardcoded high-priority patterns from extract_destination()
    ("デッキの一番上に置いてもよい", "deck_top"),
    ("そのメンバーの下に置く", "under_member"),
    ("デッキの一番上か一番下に置く", "deck_top_or_bottom"),
    ("デッキの一番上か一番下に置き", "deck_top_or_bottom"),
    ("デッキの一番上か一番下に置いて", "deck_top_or_bottom"),
    ("山札の上に置く", "deck_top"),
    ("山札の下に置く", "deck_bottom"),
    ("ライブカード置き場に置いてもよい", "live_card_zone"),
    ("表向きでライブカード置き場に置く", "live_card_zone"),
    ("いたエリアに", "same_area"),
    ("置かれていたエリアに", "same_area"),
    ("控え室に送る", "discard"),
    ("デッキに戻す", "deck"),
    # Standard patterns (longest-first for correct matching)
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
    ("デッキの下に置く", "deck_bottom"),
    ("デッキの下に置き", "deck_bottom"),
    ("デッキの下に置いて", "deck_bottom"),
    ("デッキに置く", "deck"),
    ("控え室に置く", "discard"),
    ("控え室に置いて", "discard"),
    ("控え室に置き", "discard"),
    ("枚控え室に置く", "discard"),
    ("枚控え室に置いて", "discard"),
    ("手札に加える", "hand"),
    ("手札に加えて", "hand"),
    ("手札に置く", "hand"),
    ("ステージに置く", "stage"),
    ("ステージに登場させる", "stage"),
    ("エネルギー置き場に置く", "energy_zone"),
    ("エネルギーゾーンに置く", "energy_zone"),
    ("エネルギー・デッキに置く", "energy_deck"),
    ("エネルギー・デッキに置いてもよい", "energy_deck"),
    ("成功ライブカード置き場に置く", "success_live_zone"),
    ("ライブカード置き場に置く", "live_card_zone"),
    ("メンバーのいないエリア", "empty_area"),
    ("そのメンバーがいたエリア", "same_area"),
    ("このメンバーの下に置く", "under_member"),
    ("このメンバーの下に置いて", "under_member"),
    ("このメンバーの下に置き", "under_member"),
]

STATE_CHANGE_PATTERNS: List[Tuple[str, str]] = [
    ("ウェイトにする", "wait"),
    ("ウェイトにしてもよい", "wait"),
    ("ウェイトにし", "wait"),
    ("ウェイト状態で置く", "wait"),
    ("ウェイト状態で登場させる", "wait"),
    ("アクティブにする", "active"),
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
