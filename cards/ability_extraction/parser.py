"""Parser for ability extraction - structural approach based on actual data analysis."""

import json
import re
from typing import Dict, Any, Optional, Tuple, List, Union
from parser_utils import (
    extract_count,
    extract_dynamic_count,
    extract_group_name,
    normalize_fullwidth_digits,
    strip_suffix_period,
)

# ============== CONFIGURATION CONSTANTS ==============
MAX_CHARACTER_NAME_LENGTH = 10
SPLIT_LIMIT = 1

# ============== SOURCE PATTERNS (FROM) ==============
SOURCE_PATTERNS = [
    ("デッキの一番下から", "deck_bottom"),  # Must be before デッキから
    ("デッキの上から", "deck_top"),  # Must be before デッキから
    ("デッキから", "deck"),
    ("山札から", "deck"),
    ("エネルギーデッキから", "energy_deck"),
    ("エネルギー置き場から", "energy_zone"),
    ("控え室か ら", "discard"),  # Q226: Handle unusual spacing
    ("控え室にある", "discard"),
    ("控え室から", "discard"),
    ("相手の控え室にある", "discard"),
    ("相手の控え室から", "discard"),
    ("からライブカード", "discard"),  # Q226: Handle "～からライブカード" pattern
    ("手札から", "hand"),
    ("ステージから", "stage"),
    ("ライブカード置き場から", "live_card_zone"),
    ("成功ライブカード置き場から", "success_live_zone"),
]

# ============== DESTINATION PATTERNS (TO) ==============
DESTINATION_PATTERNS = [
    # More specific deck position patterns first
    ("デッキの一番上から4枚目に置く", "deck_position_4"),
    ("デッキの一番上から4枚目に置き", "deck_position_4"),  # Handle continuative form
    ("デッキの一番上に置く", "deck_top"),
    ("デッキの一番上に置き", "deck_top"),  # Handle continuative form
    ("デッキの一番上に置いて", "deck_top"),  # Handle te-form
    ("デッキの上に置く", "deck_top"),
    ("デッキの上に置いて", "deck_top"),  # Handle te-form
    ("デッキの上に置き", "deck_top"),  # Handle continuative form
    ("デッキの一番下に置く", "deck_bottom"),
    ("デッキの一番下に置いて", "deck_bottom"),  # Handle te-form
    ("デッキの一番下に置き", "deck_bottom"),  # Handle continuative form
    ("デッキの下に置く", "deck_bottom"),
    ("デッキの下に置いて", "deck_bottom"),  # Handle te-form
    ("デッキの下に置き", "deck_bottom"),  # Handle continuative form
    ("デッキに置く", "deck"),  # Q226: General deck placement
    ("控え室に置く", "discard"),
    ("控え室に置いて", "discard"),  # Handle te-form
    ("控え室に置き", "discard"),  # Handle continuative form
    ("枚控え室に置く", "discard"),
    ("枚控え室に置いて", "discard"),  # Handle te-form
    ("手札に加える", "hand"),
    ("手札に加えて", "hand"),  # Handle te-form
    ("手札に置く", "hand"),
    ("ステージに置く", "stage"),
    ("ステージに登場させる", "stage"),
    ("エネルギー置き場に置く", "energy_zone"),
    ("エネルギーゾーンに置く", "energy_zone"),
    ("エネルギー・デッキに置く", "energy_deck"),
    ("エネルギー・デッキに置いてもよい", "energy_deck"),
    ("ライブカード置き場に置く", "live_card_zone"),
    ("成功ライブカード置き場に置く", "success_live_zone"),
    ("メンバーのいないエリア", "empty_area"),
    ("そのメンバーがいたエリア", "same_area"),
    ("このメンバーの下に置く", "under_member"),
    ("このメンバーの下に置いて", "under_member"),  # Handle te-form
    ("このメンバーの下に置き", "under_member"),  # Handle continuative form
]

# ============== ACTION PATTERNS ==============
ACTION_PATTERNS = [
    ("シャッフルする", "shuffle"),
    ("シャッフルして", "shuffle"),  # Handle te-form
    ("入れ替える", "swap"),
    ("入れ替えて", "swap"),  # Handle te-form
    ("ポジションチェンジする", "position_change"),
    ("無効にする", "invalidate_ability"),
    ("無効にしてもよい", "invalidate_ability_optional"),
    ("エマパンチする", "emma_punch"),
    ("何もしない", "do_nothing"),
]

# ============== STATE CHANGE PATTERNS ==============
STATE_CHANGE_PATTERNS = [
    ("ウェイトにする", "wait"),
    ("ウェイトにしてもよい", "wait"),
    ("ウェイトにし", "wait"),
    ("ウェイト状態で置く", "wait"),
    ("ウェイト状態で登場させる", "wait"),
    ("アクティブにする", "active"),
]

# ============== LOCATION PATTERNS ==============
LOCATION_PATTERNS = [
    ("成功ライブカード置き場", "success_live_card_zone"),
    ("ライブカード置き場", "live_card_zone"),
    ("控え室", "discard"),
    ("手札", "hand"),
    ("ステージ", "stage"),
    ("デッキ", "deck"),
    ("エネルギーデッキ", "energy_deck"),
    ("エネルギー置き場", "energy_zone"),
]

# ============== POSITION KEYWORDS ==============
POSITION_KEYWORDS = {
    "センターエリア": "center",
    "左サイドエリア": "left_side",
    "右サイドエリア": "right_side",
    "センター": "center",
    "左サイド": "left_side",
    "右サイド": "right_side",
}

# ============== TEMPORAL CONDITION PATTERNS ==============
TEMPORAL_PATTERNS = [
    ("移動していない", "not_moved"),
    ("移動している", "has_moved"),
    ("ライブを成功させていた", "opponent_live_success"),
    ("余剰ハートを持たない", "no_excess_heart"),
]

# ============== COMPARISON TARGETS ==============
COMPARISON_TARGETS = {
    "相手より": "opponent",
    "自分より": "self",
    "このメンバーより": "self",
}

# ============== COMPARISON OPERATORS ==============
COMPARISON_OPERATORS = {
    "高い": ">",
    "低い": "<",
    "少ない": "<",
    "多い": ">",
    "大きい": ">",
    "小さい": "<",
}

# ============== COMPARISON TYPES ==============
COMPARISON_TYPES = {"スコア": "score", "コスト": "cost"}

# ============== CARD TYPE PATTERNS ==============
CARD_TYPE_PATTERNS = [
    ("メンバーカード", "member_card"),
    ("メンバー", "member_card"),
    ("ライブカード", "live_card"),
    ("エネルギーカード", "energy_card"),
]

# ============== OPERATOR PATTERNS ==============
OPERATOR_PATTERNS = [
    ("以上", ">="),
    ("以下", "<="),
    ("より少ない", "<"),
    ("より多い", ">"),
    ("未満", "<"),
    ("超", ">"),
]

# ============== CONDITION MARKERS ==============
CONDITION_MARKERS = ["場合、", "とき、", "なら、"]

# ============== STRUCTURAL MARKERS ==============
SEQUENTIAL_MARKER = "その後、"
CONDITIONAL_SEQUENTIAL_MARKER = "そうした場合"
CHOICE_MARKER = "以下から1つを選ぶ"
DURATION_MARKER = "かぎり"
COMPOUND_OPERATOR = "かつ"
PER_UNIT_MARKER = "につき"
EACH_TIME_MARKER = "たび"
EITHER_CASE_MARKER = "いずれかの場合"
ALTERNATIVE_MARKER = "代わりに"

# ============== DURATION PREFIXES ==============
DURATION_PREFIX_MAP = {
    "ライブ終了時まで": "live_end",
    "ライブ終了まで": "live_end",
    "このターンの間": "this_turn",
    "このライブの間": "this_live",
    "ターン終了時まで": "turn_end",
    "そのターンの間": "turn_end",
}


def _strip_duration_prefix(text):
    """Strip duration prefix from start of text. Returns (rest_text, code_or_None)."""
    for pat, code in DURATION_PREFIX_MAP.items():
        if text.startswith(pat):
            rest = text[len(pat) :].lstrip("、，").strip()
            return rest, code
    return text, None


# ============== COST MODIFICATION PATTERNS ==============
COST_MODIFICATION_PATTERNS = [
    ("元々持つコストより(\d+)低い値に等しくなる", "decrease_by"),
    ("元々持つコストより(\d+)高い値に等しくなる", "increase_by"),
    ("コストが(\d+)以上になった場合", "cost_threshold"),
    ("コストが(\d+)以下になった場合", "cost_threshold_below"),
    ("コストは(\d+)減る", "decrease_by"),
    ("コストは(\d+)減らす", "decrease_by"),
    ("コストは(\d+)増える", "increase_by"),
    ("コストは(\d+)増やす", "increase_by"),
]

# ============== COMPLEX CONDITION PATTERNS ==============
COMPLEX_CONDITION_MARKERS = ["これにより", "その結果"]

# ============== REGEX PATTERNS ==============
REGEX_COUNT_CARDS = r"(\d+)枚"
REGEX_COUNT_PERSONS = r"(\d+)人"
REGEX_COUNT_ITEMS = r"(\d+)つ"
REGEX_COUNT_TIMES = r"(\d+)回"
REGEX_QUOTED_TEXT = r"「([^」]+)」"
REGEX_GROUP_NAME = r"『([^』]+)』"
REGEX_DECK_POSITION = r"(\d+)枚目"

# ============== UTILITY FUNCTIONS ==============


def extract_by_pattern(text: str, patterns: List[Tuple[str, str]]) -> Optional[str]:
    """Generic function to extract value by matching patterns."""
    for pattern, code in patterns:
        if pattern in text:
            return code
    return None


def extract_source(text: str) -> Optional[str]:
    """Extract source location (FROM)."""
    if "デッキの一番上からカードを" in text:
        return "deck_top"
    if "デッキの一番上のカードを" in text:
        return "deck_top"
    if "これにより公開されたほかのすべてのカードを" in text:
        return "revealed_remaining"
    if "これにより公開したカードを" in text or "公開したカードをすべて" in text:
        return "revealed_cards"
    # "それらのカード" is ambiguous — could be revealed cards or trigger-event cards.
    # Use a neutral source flag so the engine can resolve based on trigger context.
    if "それらのカードの中から" in text:
        return "those_cards"
    if "このカードを手札に加えてもよい" in text:
        return "revealed_card"
    if "自分の成功ライブカード置き場にある" in text:
        return "success_live_zone"
    if "エールにより公開された" in text:
        return "revealed_cards"
    if "メンバーの下にある" in text or "メンバー1人の下にある" in text:
        return "under_member"
    if "自分の控え室にある" in text or "控え室からライブカード" in text:
        return "discard"
    if "デッキの一番下から" in text:
        return "deck_bottom"
    if "控え室を" in text:
        return "discard"
    if "エネルギーデッキから" in text:
        return "energy_deck"
    if "デッキの上から" in text:
        return "deck_top"
    if "デッキから" in text or "山札から" in text:
        return "deck"
    if "ステージから" in text:
        return "stage"
    if "ライブカード置き場から" in text:
        return "live_card_zone"
    if "エネルギー置き場から" in text:
        return "energy_zone"
    # Prefer explicit source phrases over broad nouns. "手札に加える" should not
    # be read as a hand source unless the text actually says "手札を/手札から/手札の".
    if "手札を" in text or "手札から" in text or "手札の" in text:
        return "hand"
    return extract_by_pattern(text, SOURCE_PATTERNS)


def extract_destination(text: str) -> Optional[str]:
    """Extract destination location (TO)."""
    if "デッキの一番上に置いてもよい" in text:
        return "deck_top"
    if "エネルギーカードを1枚ウェイト状態で置いてもよい" in text:
        return "energy_zone"
    m = re.search(r"デッキの一番上から(\d+)枚目に置(?:いてもよい|く)", text)
    if m:
        return "deck"
    if "そのメンバーの下に置く" in text:
        return "under_member"
    # Handle "デッキの一番上か一番下" (top OR bottom) — store as deck_top for now
    if "デッキの一番上か一番下に置く" in text or "デッキの一番上か一番下に置き" in text:
        return "deck_top_or_bottom"
    if "デッキの一番上か一番下に置いて" in text:
        return "deck_top_or_bottom"
    if "成功ライブカード置き場に置く" in text:
        return "success_live_zone"
    if (
        "メンバーのいないエリアに登場させる" in text
        or "メンバーのいないエリアにウェイト状態で登場させる" in text
    ):
        return "empty_area"
    if "デッキの一番上に置く" in text or "山札の上に置く" in text:
        return "deck_top"
    if (
        "ライブカード置き場に置いてもよい" in text
        or "表向きでライブカード置き場に置く" in text
    ):
        return "live_card_zone"
    if "ウェイト状態で置く" in text or (
        "エネルギーカードを" in text and "置く" in text
    ):
        return "energy_zone"
    if "いたエリアに" in text or "置かれていたエリアに" in text:
        return "same_area"
    if "登場させる" in text:
        return "stage"
    if "控え室に送る" in text:
        return "discard"
    if (
        "デッキの下に置く" in text
        or "山札の下に置く" in text
        or "デッキの一番下に置く" in text
    ):
        return "deck_bottom"
    if "デッキに戻す" in text:
        return "deck"
    return extract_by_pattern(text, DESTINATION_PATTERNS)


def extract_location(text: str) -> Optional[str]:
    """Extract location (general)."""
    return extract_by_pattern(text, LOCATION_PATTERNS)


def extract_locations(text: str) -> Optional[List[str]]:
    """Extract multiple locations connected by 'と' (e.g. 'ステージと控え室')."""
    locs = []
    for pattern, loc_name in LOCATION_PATTERNS:
        if pattern in text:
            locs.append(loc_name)
    if len(locs) >= 2:
        return locs
    return None


def extract_state_change(text: str) -> Optional[str]:
    """Extract state change (wait/active)."""
    return extract_by_pattern(text, STATE_CHANGE_PATTERNS)


def extract_card_type(text: str) -> Optional[str]:
    """Extract card type."""
    return extract_by_pattern(text, CARD_TYPE_PATTERNS)


def extract_target(text: str) -> Optional[str]:
    """Extract target (self/opponent/both/either).
    'both' means the action applies to or involves both players.
    Used for both action target and condition scope."""
    if (
        ("自分の" in text and "相手の" in text)
        or "自分と相手の" in text
        or "自分と相手は" in text
    ):
        return "both"
    if "自分か相手の" in text:
        return "either"
    if "相手の" in text:
        return "opponent"
    if "自分の" in text:
        return "self"
    return None


def extract_operator(text: str) -> Optional[str]:
    """Extract comparison operator."""
    return extract_by_pattern(text, OPERATOR_PATTERNS)


def extract_cost_limit(text: str) -> Optional[Union[int, List[int]]]:
    """Extract cost limit."""
    for pat in [
        r"(\d+)コスト(?:以上|以下|未満|超)",
        r"コスト(\d+)(?:以上|以下|未満|超)",
        r"(\d+)\s*以下",
        r"以下\s*(\d+)",
        r"(\d+)\s*合計",
        r"コスト(\d+)の",
    ]:  # e.g. "コスト10の" → limit to cost=10
        m = re.search(pat, text)
        if m:
            return int(m.group(1))
    return None


def extract_blade_limit(text: str) -> Optional[Dict[str, Any]]:
    """Extract blade count limit from text like 'ブレードの数が3つ以下' (<=3 blades)."""
    # Normalize: replace {{icon_blade.png|ブレード}} with just ブレード
    normalized = re.sub(r"\{\{icon_blade\.png\|ブレード\}\}", "ブレード", text)
    m = re.search(r"ブレード[の]数[がは](\d+)[つ個](以下|以上|未満|超)", normalized)
    if not m:
        m = re.search(r"ブレード[の]数[がは](\d+)(以下|以上|未満|超)", normalized)
    if m:
        result = {"blade_limit": int(m.group(1))}
        op = m.group(2)
        if op == "以下":
            result["blade_limit_operator"] = "<="
        elif op == "以上":
            result["blade_limit_operator"] = ">="
        elif op == "未満":
            result["blade_limit_operator"] = "<"
        elif op == "超":
            result["blade_limit_operator"] = ">"
        return result
    return None


def extract_deck_position(text: str) -> Optional[int]:
    """Extract deck position from text like '一番上から4枚目' (4th from top)."""
    # Match patterns like "一番上から4枚目" or "上から4枚目"
    match = re.search(r"一番上から(\d+)枚目", text)
    if match:
        return int(match.group(1))
    match = re.search(r"上から(\d+)枚目", text)
    if match:
        return int(match.group(1))
    return None


def extract_deck_position_for_action(text: str) -> Optional[Dict[str, Any]]:
    """Extract deck position for action, returns PositionInfo format."""
    pos = extract_deck_position(text)
    if pos:
        return {"position": {"position": str(pos)}}
    return None


def extract_position(text: str) -> Dict[str, Any]:
    """Extract position requirement with target."""
    result = {}

    # Extract target
    target = extract_target(text)
    if target:
        result["target"] = target

    # Check for both players effect (自分と相手はそれぞれ) - override target
    if "自分と相手はそれぞれ" in text:
        result["target"] = "both"

    # Extract deck position (Q226: 一番上から4枚目)
    deck_pos = extract_deck_position(text)
    if deck_pos:
        result["position"] = {"position": str(deck_pos)}

    # Note: Position field removed to avoid deserialization errors
    # Rust expects PositionInfo struct, not string

    return result if result else None


def extract_optional(text: str) -> bool:
    """Check if action is optional."""
    return "もよい" in text or "てもよい" in text


def extract_group_names(text: str) -> List[str]:
    """Extract all group names within 『』 brackets."""
    return re.findall(r"『([^』]+)』", text)


def extract_quoted_text(text: str) -> List[str]:
    """Extract all text within 「」 quotes."""
    return re.findall(r"「([^」]+)」", text)


def extract_parenthetical(text: str) -> List[str]:
    """Extract all text within （） or () parentheses."""
    results = re.findall(r"（([^）]+)）", text)
    results += re.findall(r"\(([^)]+)\)", text)
    return results


def strip_parenthetical(text: str) -> str:
    """Remove parenthetical notes from text (both full-width （） and ASCII ())."""
    text = re.sub(r"（([^）]+)）", "", text)
    text = re.sub(r"\(([^)]+)\)", "", text)
    return text.strip()


def extract_max(text: str) -> bool:
    """Check if count has 'max' modifier (まで)."""
    return "人まで" in text or "枚まで" in text


def filter_character_names(quoted_text: List[str]) -> List[str]:
    """Filter character names from quoted text, excluding ability names."""
    # Filter out ability names (which typically contain {{ or are longer than 10 chars)
    return [c for c in quoted_text if "{{" not in c and len(c) <= 10]


def categorize_quoted_text(quoted_text: List[str]) -> Dict[str, List[str]]:
    """Categorize quoted text into character names and ability texts."""
    result = {"characters": [], "abilities": []}
    for q in quoted_text:
        if "{{" in q and "}}" in q:
            result["abilities"].append(q)
        else:
            result["characters"].append(q)
    return result


def normalize(text: str) -> str:
    """Canonicalize variant patterns before parsing."""
    text = re.sub(r"'([^']{1,10})'", r"『\1』", text)
    text = text.replace("ライブ終了まで", "ライブ終了時まで")
    return text


def extract_duration(text: str) -> Optional[str]:
    """Extract duration from effect text."""
    DURATION_MAP = {
        "ライブ終了時まで": "live_end",
        "ライブ終了まで": "live_end",
        "このターンの間": "this_turn",
        "このライブの間": "this_live",
    }
    for pattern, code in DURATION_MAP.items():
        if pattern in text:
            return code
    return None


def extract_cost_modification(text: str) -> Optional[Dict[str, Any]]:
    """Extract cost modification patterns from text."""
    result = {}

    # Check for cost modification patterns
    for pattern, mod_type in COST_MODIFICATION_PATTERNS:
        match = re.search(pattern, text)
        if match:
            result["modification_type"] = mod_type
            if match.groups():
                result["value"] = int(match.group(1))
            break

    # Check for cost threshold patterns
    threshold_match = re.search(r"コストが(\d+)以上になった場合", text)
    if threshold_match:
        result["cost_threshold"] = int(threshold_match.group(1))
        result["threshold_operator"] = ">="

    threshold_match_below = re.search(r"コストが(\d+)以下になった場合", text)
    if threshold_match_below:
        result["cost_threshold"] = int(threshold_match_below.group(1))
        result["threshold_operator"] = "<="

    return result if result else None


# ============== STRUCTURAL PARSING ==============


def split_cost_effect(text: str) -> Tuple[str, str]:
    """Split text into cost and effect parts, skipping colons inside parentheses and quotes."""
    if "：" not in text:
        return "", text

    # Find the first colon that's not inside parentheses or quotes
    paren_depth = 0
    quote_depth = 0
    split_index = -1

    for i, char in enumerate(text):
        if char == "（" or char == "(":
            paren_depth += 1
        elif char == "）" or char == ")":
            paren_depth -= 1
        elif char == '"' or char == '"':
            quote_depth += 1 if quote_depth == 0 else -1
        elif char == "：":
            # Only split if not inside parentheses or quotes
            if paren_depth == 0 and quote_depth == 0:
                split_index = i
                break

    if split_index >= 0:
        cost = text[:split_index].strip()
        effect = text[split_index + 1 :].strip()
        return cost, effect
    else:
        # No valid split point found
        return "", text


def split_condition_action(text: str) -> Tuple[str, str]:
    """Split text into condition and action parts.
    The condition keyword (場合/とき/なら) is kept with the condition text."""
    for keyword in ["場合", "とき", "なら"]:
        pattern = keyword + "、"
        if pattern in text:
            keyword_idx = text.find(keyword)
            comma_idx = keyword_idx + len(keyword)
            condition = text[:comma_idx].strip()
            action = text[comma_idx + 1 :].strip()
            return condition, action
    return "", text


def parse_complex_condition(text: str) -> Dict[str, Any]:
    """Parse complex conditions with cause-effect relationships (e.g., これにより)."""
    # Check for complex condition markers
    for marker in COMPLEX_CONDITION_MARKERS:
        if marker in text:
            parts = text.split(marker, 1)
            if len(parts) == 2:
                # Parse the cause part (what triggers the effect)
                cause_text = parts[0].strip()
                # Parse the effect part (what happens as a result)
                effect_text = parts[1].strip()

                # Only treat as complex condition if there's meaningful content before the marker
                # and the marker is not part of a conditional phrase like "これにより～場合"
                if cause_text and not effect_text.startswith("場合"):
                    # Try to parse the effect as an action/effect first
                    # If it looks like an action (contains verbs like 置かれた, 公開された), parse it as a condition
                    # If it looks like a state (contains ない, ある), parse it as a condition
                    effect_condition = parse_condition(effect_text)

                    # If the effect is still custom, try to extract more specific information
                    if effect_condition.get("type") == "custom":
                        # Check for ability invalidation patterns
                        if "無効にした" in effect_text or "無効に" in effect_text:
                            effect_condition["action"] = "invalidate_ability"
                            effect_condition["optional"] = (
                                "もよい" in effect_text or "してもよい" in effect_text
                            )
                        # Check for negation patterns like "～がない"
                        elif "ない" in effect_text and (
                            "カード" in effect_text or "ライブカード" in effect_text
                        ):
                            effect_condition["negation"] = True
                            # Try to extract card type
                            if "ライブカード" in effect_text:
                                effect_condition["card_type"] = "live_card"
                            elif "メンバーカード" in effect_text:
                                effect_condition["card_type"] = "member_card"
                            # Try to extract location
                            if "公開された" in effect_text:
                                effect_condition["location"] = "revealed_cards"

                    return {
                        "type": "complex_condition",
                        "cause": parse_condition(cause_text),
                        "effect": effect_condition,
                        "text": text,
                    }

    # If no complex markers found, return None
    return None


# ============== COMPONENT PARSING ==============

# ============== CONDITION HANDLER CASCADE ==============
#
# Priority order is CRITICAL — each handler checks a text pattern and returns
# the parsed condition dict if it matches. The first match wins. Handlers are
# ordered from most specific (narrow text patterns) to most generic.
#
# Some patterns MUST precede others to prevent false matches:
#   - Compound (かつ/あり、) must come before count patterns like "1枚以上ある"
#     which would match part of a compound text and miss the full structure.
#   - Distinct names (名前が異なる) must come before count conditions since
#     both can appear in the same text ("名前が異なるメンバーが3人以上いる").
#   - Temporal count conditions ("このターン、～3回登場した") must precede
#     plain appearance conditions (登場) to extract time+count info.
#   - OR (か、) must precede movement conditions since "か、" also contains "移動".
#   - Position change must precede position keywords (センター etc) since it's
#     a more specific pattern about the change action, not position queries.
#
# Each _try_* function takes (text) and returns a complete condition dict or None.


def _try_complex(text):
    return parse_complex_condition(text)


def _try_compound(text):
    COMPOUND_OPERATOR_ALT = "あり、"
    if COMPOUND_OPERATOR not in text and COMPOUND_OPERATOR_ALT not in text:
        return None
    op = COMPOUND_OPERATOR if COMPOUND_OPERATOR in text else COMPOUND_OPERATOR_ALT
    parts = [p.strip() for p in text.split(op) if p.strip()]
    if len(parts) < 2:
        return None
    parsed = [parse_condition(p) for p in parts]
    if len(parsed) < 2:
        return None
    result = {"type": "compound", "operator": "and", "conditions": parsed, "text": text}
    tgt = extract_target(text)
    if tgt:
        result["target"] = tgt
    loc = extract_location(text)
    if loc:
        result["location"] = loc
    ct = extract_card_type(text)
    if ct:
        result["card_type"] = ct
    # Post-parse enrichment: convert bare comparison_condition to card_count_condition
    # when sub-condition text contains a people counter ('人')
    for sub in parsed:
        if sub.get("type") == "comparison_condition" and sub.get("count"):
            sub_text = sub.get("text", "")
            if "人" in sub_text:
                sub["type"] = "card_count_condition"
                sub["unit"] = "人"
                sub["card_type"] = "member_card"
    # Propagate distinct flag from sub-conditions to parent
    for sub in parsed:
        if sub.get("distinct"):
            result["distinct"] = True
            break
    # Also check compound text directly for distinct keywords
    for kw in [
        "名前が異なる",
        "カード名が異なる",
        "グループ名が異なる",
        "コストがそれぞれ異なる",
    ]:
        if kw in text:
            result["distinct"] = True
            break
    return result


def _try_distinct(text):
    if (
        "名前が異なる" not in text
        and "名前の異なる" not in text
        and "ユニット名がそれぞれ異なる" not in text
    ):
        return None
    locs = extract_locations(text)
    result = {
        "type": "location_condition",
        "target": "self",
        "distinct": True,
        "text": text,
    }
    if locs:
        result["locations"] = locs
    else:
        result["location"] = "stage"
    if "エリアすべて" in text:
        result["all_areas"] = True
    m = re.search(r"(\d+)(人|枚|つ)以上いる", text)
    if not m:
        m = re.search(r"(\d+)人以上", text)
    if m:
        result["count"] = int(m.group(1))
        result["operator"] = ">="
        result["unit"] = m.group(2) if len(m.groups()) >= 2 and m.group(2) else "人"
    gns = extract_group_names(text)
    if gns:
        result["group_names"] = gns
    return result


def _try_card_count(text):
    for pat, op, unit in [
        (r"(\d+)枚以上ある", ">=", None),
        (r"(\d+)種類以上ある", ">=", "types"),
        (r"(\d+)枚ある", "=", None),
        (r"(\d+)枚以上", ">=", None),
        (r"(\d+)人以上", ">=", "人"),
        (r"(\d+)(人|枚|つ)以上いる", ">=", None),
    ]:
        m = re.search(pat, text)
        if m:
            result = {
                "type": "card_count_condition",
                "count": int(m.group(1)),
                "operator": op,
                "text": text,
            }
            if unit:
                result["unit"] = unit
            elif len(m.groups()) >= 2 and m.group(2):
                result["unit"] = m.group(2)
            # When counting people, it's always member_card
            u = result.get("unit", "")
            if u == "人":
                result["card_type"] = "member_card"
            # Extract exclude_self for "このメンバー以外" pattern
            if (
                "このメンバー以外" in text
                or "このカード以外" in text
                or "ほかのメンバー" in text
            ):
                result["exclude_self"] = True
                result["card_type"] = "member_card"
            # Detect ALL blade property
            if "ALLブレード" in text or "{{icon_b_all.png" in text:
                result["card_property"] = "has_all_blade"
            # Detect revealed cards context (yell)
            if "エールにより公開された" in text:
                result["location"] = "revealed_cards"
            # Extract card_type, location, target
            # Don't infer card_type when it comes from a zone name
            # (e.g. "成功ライブカード置き場にカードが2枚以上" — "ライブカード" is the zone,
            #  not a card type constraint on the counted cards)
            ct = extract_card_type(text)
            if ct:
                zone_keywords = ["置き場", "ゾーン"]
                if not any(kw in text for kw in zone_keywords):
                    result["card_type"] = ct
            loc = extract_location(text)
            if loc:
                result["location"] = loc
            tgt = extract_target(text)
            if tgt:
                result["target"] = tgt
            # Detect live_card_zone from "ライブ中のカード"
            if "ライブ中のカード" in text and not result.get("location"):
                result["location"] = "live_card_zone"
            # Detect energy_zone from energy count context (energy cards in energy zone)
            if (
                "エネルギー" in text
                and not result.get("location")
                and not result.get("resource_type")
            ):
                result["location"] = "energy_zone"
            # Also try _try_either_target for "自分か相手の" patterns
            either_result = _try_either_target(text)
            if either_result:
                if "target" in either_result:
                    result["target"] = either_result["target"]
                if "location" in either_result:
                    result["location"] = either_result["location"]
            # Extract group from 『』 — skip when を含む (includes) is present since
            # the group is a subset qualifier, not a filter on all counted cards
            gns = extract_group_names(text)
            if gns and "を含む" not in text:
                result["group_names"] = gns

            # Extract heart_colors from text for heart icon patterns (e.g. 5種類以上)
            if "{{heart_" in text:
                hm = re.findall(r"{{heart_(\d+)\.png\|heart\d+}}", text)
                if hm:
                    colors = sorted(set(f"heart{m.zfill(2)}" for m in hm))
                    result["heart_colors"] = colors

                # Extract hand count condition (手札がN枚以下の場合 / 手札がN枚以上の場合)
            hand_m = re.search(r"手札が(\d+)枚以下(の|の場)", text)
            if hand_m:
                hand_count = int(hand_m.group(1))
                # Find split at condition marker before "手札" — handles prefix like "自分の"
                split_pos = hand_m.start()
                for marker in ["とき、", "場合、", "なら、"]:
                    pos = text.rfind(marker, 0, hand_m.start())
                    if pos >= 0:
                        split_pos = pos + len(marker)
                        break
                first_text = text[:split_pos].strip()
                second_text = text[split_pos:].rstrip("、。，").strip()
                hand_cond = {
                    "type": "comparison_condition",
                    "resource_type": "hand_count",
                    "location": "hand",
                    "count": hand_count,
                    "operator": "<=",
                    "text": second_text,
                }
                # Re-parse first condition from its own text to avoid field leakage
                full_text = text
                if first_text and first_text != text:
                    first_cond = parse_condition(first_text)
                    if first_cond and first_cond.get("type") not in ("custom", None):
                        result = first_cond
                    else:
                        result["text"] = first_text
                        # Clear contaminated fields then re-extract from first_text only
                        for k in (
                            "card_type",
                            "location",
                            "target",
                            "group_names",
                            "card_property",
                            "exclude_self",
                        ):
                            result.pop(k, None)
                        # Re-extract from first_text using helper
                        _extract_generic_fields(result, first_text)
                # Promote to compound condition with original full text
                result = {
                    "type": "compound",
                    "operator": "and",
                    "conditions": [result, hand_cond],
                    "text": full_text,
                }
            return result
    return None


def _try_both(text):
    if "それらが両方ある" not in text:
        return None
    return {"type": "both_condition", "text": text}


def _try_temporal_this_turn(text):
    if "このターン" not in text:
        return None
    for pattern, cond_type in TEMPORAL_PATTERNS:
        if pattern in text:
            result = {
                "type": "temporal_condition",
                "temporal": "this_turn",
                "condition": {"type": cond_type},
                "text": text,
            }
            ct = extract_card_type(text)
            if ct:
                result["card_type"] = ct
            if (
                "余剰のハートを持たずに" in text
                or "余剰ハートを持たない" in text
                or "余剰ハート" in text
            ):
                result["condition"]["no_excess_heart"] = True
            return result
    return None


def _try_temporal_turn_phase(text):
    if (
        "このゲームの" not in text
        or "ターン目" not in text
        or "ライブフェイズ" not in text
    ):
        return None
    return {"type": "temporal_condition", "phase": "live_phase", "text": text}


def _try_baton_touch(text):
    if (
        "バトンタッチして登場した" not in text
        and "バトンタッチして控え室に置か" not in text
    ):
        return None
    is_to_stage = "バトンタッチして登場した" in text
    result = {
        "type": "movement_condition",
        "movement": "baton_touch",
        "target": "self",
        "baton_touch_trigger": True,
        "text": text,
    }
    if is_to_stage:
        result["location"] = "stage"
    else:
        result["location"] = "discard"
    m = re.search(r"「([^」]+)」からバトンタッチ", text)
    if m:
        result["baton_touch_source"] = m.group(1)
    m = re.search(r"『([^』]+)』からバトンタッチ", text)
    if m:
        result["baton_touch_group"] = m.group(1)
    # Extract cost limit (e.g., "コスト10以上" → cost_limit=10, operator=">=")
    cm = re.search(r"コスト(\d+)(以上|以下|より大きい|より小さい|未満)", text)
    if cm:
        result["cost_limit"] = int(cm.group(1))
        op_map = {
            "以上": ">=",
            "以下": "<=",
            "より大きい": ">",
            "より小さい": "<",
            "未満": "<",
        }
        result["cost_limit_operator"] = op_map.get(cm.group(2), ">=")
    gns = extract_group_names(text)
    if gns:
        result["group_names"] = gns
    if "このメンバー以外" in text or "ほかのメンバー" in text:
        result["exclude_self"] = True
    if "能力を持たない" in text or "能力も持たない" in text:
        result["ability_negation"] = True
    return result


def _try_temporal_count(text):
    if not (
        ("このターン" in text or "ターン目" in text)
        and ("回" in text or "登場" in text)
    ):
        return None
    result = {"type": "temporal_condition", "temporal": "this_turn", "text": text}
    m = re.search(r"(\d+)回", text)
    if m:
        result["count"] = int(m.group(1))
    elif "登場" in text and "回" not in text:
        result["count"] = 1
    if "ライブフェイズ" in text:
        result["phase"] = "live_phase"
    elif "メインフェイズ" in text:
        result["phase"] = "main_phase"
    loc = extract_location(text)
    if loc:
        result["location"] = loc
    ct = extract_card_type(text)
    if ct:
        result["card_type"] = ct
    tgt = extract_target(text)
    if tgt:
        result["target"] = tgt
    if "エリアすべて" in text:
        result["all_areas"] = True
    if "移動している" in text:
        result["movement_state"] = "has_moved"
    return result


def _try_or(text):
    # Only split on "あるか、" (OR pattern: condition A あるか、condition B)
    # NOT on generic "か、" which appears in other grammar patterns
    if "あるか、" not in text:
        return None
    parts = [p.strip() for p in text.split("あるか、") if p.strip()]
    if len(parts) < 2:
        return None
    # Restore "ある" suffix lost during split on "あるか、"
    parts = [p + "ある" if i < len(parts) - 1 else p for i, p in enumerate(parts)]
    # Also add "場合" to last part if present in original
    parsed = [parse_condition(p) for p in parts]
    if len(parsed) < 2:
        return None
    return {"type": "or_condition", "conditions": parsed, "text": text}


def _try_either_target(text):
    if "自分か相手の" not in text:
        return None
    m = re.search(r"自分か相手の(.+?)(?:に|が|にある)", text)
    if not m:
        return None
    loc_text = m.group(1).strip()
    LOC_MAP = {
        "成功ライブカード置き場": "success_live_zone",
        "ライブカード置き場": "live_card_zone",
        "控え室": "discard",
        "手札": "hand",
        "ステージ": "stage",
        "エネルギー置き場": "energy_zone",
    }
    for kw, code in LOC_MAP.items():
        if kw in loc_text:
            result = {
                "type": "location_condition",
                "location": code,
                "target": "either",
                "text": text,
            }
            cl = extract_cost_limit(text)
            if cl:
                result["cost_limit"] = cl
            op = extract_operator(text)
            if op:
                result["operator"] = op
            return result
    return None


def _try_movement(text):
    if "移動した" not in text and "移動している" not in text and "移動する" not in text:
        return None
    result = {"type": "movement_condition", "text": text}
    if "移動する" in text and "移動した" not in text and "移動している" not in text:
        result["movement"] = "moves"
    else:
        result["movement"] = "moved"
        result["movement_state"] = "has_moved"
    if "移動していない" in text:
        result["negation"] = True
    return result


def _try_appearance(text):
    if "登場" not in text:
        return None
    result = {"type": "appearance_condition", "appearance": True, "text": text}
    # Default to stage since abilities almost always check member appearance
    result["location"] = "stage"
    # Extract subject character (the one before が/を登場)
    # Pattern: 「X」が登場 → X is the subject
    # Pattern: 「A」よりコストの(大きい|高い)「B」が登場 → B is the subject
    subject = None
    m = re.search(r"「([^」]+)」[がを]登場", text)
    if m:
        subject = m.group(1)
    else:
        # Try to find the last quoted name before 登場 (the subject)
        quoted = re.findall(r"「([^」]+)」", text[: text.find("登場")])
        if quoted:
            subject = quoted[-1]
    if subject:
        result["characters"] = [subject]
    if "エリアすべて" in text:
        result["all_areas"] = True
    if "バトンタッチ" in text:
        result["baton_touch_trigger"] = True
    # Propagate target from text
    tgt = extract_target(text)
    if tgt:
        result["target"] = tgt
    # Extract position (左サイド/右サイド/センター)
    for kw, pos in POSITION_KEYWORDS.items():
        if kw in text:
            result["position"] = pos
            break
    return result


def _try_energy_state(text):
    has_positive = "エネルギーがある" in text
    has_negative = "エネルギーがない" in text
    if not has_positive and not has_negative:
        return None
    result = {"type": "energy_state_condition", "text": text}
    if has_negative:
        result["negation"] = True
    if "アクティブ状態" in text:
        result["state"] = "active"
    return result


def _try_state(text):
    for patterns, state in [
        (["ウェイト状態である", "ウェイト状態にある", "ウェイト状態の"], "wait"),
        (
            ["アクティブ状態である", "アクティブ状態にある", "アクティブ状態の"],
            "active",
        ),
    ]:
        if any(p in text for p in patterns):
            result = {"type": "state_condition", "state": state, "text": text}
            if state == "active" and "エネルギー" in text:
                result["resource_type"] = "energy"
            if "すべて" in text:
                result["all"] = True
            return result
    return None


def _try_revealed(text):
    if "エールにより公開された自分のカードの中に" not in text:
        return None
    has_negation = "ない" in text
    result = {
        "type": "location_condition",
        "location": "revealed_cards",
        "target": "self",
        "text": text,
    }
    if has_negation:
        result["negation"] = True
    if "ブレードハートを持つ" in text or "ブレードハートを持たない" in text:
        result["card_property"] = "has_blade_heart"
    # "持たないカードが0枚" = cards WITHOUT property = 0.
    # This means NOT(at least 1 card has the property).
    # Set count:1 with operator >= so negation → false only when ≥1 match.
    if "0枚" in text:
        result["count"] = 1
        result["operator"] = ">="
    return result


def _try_opponent_choice(text):
    if "相手は" not in text or "てもよい" not in text or "そうしなかった" not in text:
        return None
    result = {
        "type": "opponent_choice_condition",
        "target": "opponent",
        "optional": True,
        "negation": True,
        "text": text,
    }
    if "手札を1枚控え室に置いてもよい" in text or "控え室に置いてもよい" in text:
        result["action"] = "discard_card"
        result["count"] = 1
        result["source"] = "hand"
        result["destination"] = "discard"
    return result


def _try_unless_pay(text):
    if "支払わないかぎり" not in text:
        return None
    result = {"type": "comparison_condition", "negation": True, "text": text}
    ec = text.count("{{icon_energy.png|E}}")
    if ec > 0:
        result["resource_type"] = "energy"
        result["count"] = ec
        result["operator"] = ">="
    return result


def _try_position_change(text):
    if (
        "ポジションチェンジしてもよい" not in text
        and "ポジションチェンジする" not in text
        and "フォーメーションチェンジ" not in text
    ):
        return None
    result = {
        "type": "position_change_condition",
        "action": "position_change",
        "optional": "してもよい" in text
        or "フォーメーションチェンジしてもよい" in text,
        "text": text,
    }
    if "自分と相手" in text or "相手は" in text:
        result["target"] = "both"
    if "センターエリア以外" in text:
        result["exclude_position"] = "center"
    if "センターにいる" in text:
        result["source_position"] = "center"
    return result


def _handle_position_change_fields(text, action):
    """Set position-related fields for a position_change action.

    Distinguishes three cases:
    1. Exclude:   "センターエリア以外に" → exclude_position='center'
    2. Source:    "センターにいる" → source_position='center'
    3. Destination: default → position='center'

    Clears pre-set 'position' from the keyword loop when setting source
    or exclude, to avoid ambiguity in the engine.
    """
    if "センターエリア以外" in text or "センター以外" in text:
        action["exclude_position"] = "center"
        action.pop("position", None)
        return
    if "センター" in text:
        if "にいる" in text:
            action["source_position"] = "center"
            action.pop("position", None)
        else:
            action["position"] = "center"


def _try_position(text):
    # If the text has condition markers, let _try_conditional or the fall-through handle it
    if (
        any(m in text for m in CONDITION_MARKERS)
        or "場合" in text
        or "とき" in text
        or "なら" in text
    ):
        return None
    for keyword in POSITION_KEYWORDS:
        if keyword in text:
            return {"type": "position_condition", "text": text}
    return None


def _try_ability_negation(text):
    if "能力も持たない" not in text and "能力を持たない" not in text:
        return None
    return {"type": "ability_negation_condition", "text": text}


def _try_state_change(text):
    if "アクティブ状態からウェイト状態になった" not in text and not (
        "アクティブ状態" in text and "ウェイト状態" in text
    ):
        return None
    result = {"type": "state_change_condition", "text": text}

    # Detect direction
    if "ウェイト状態になった" in text or "アクティブ状態から" in text:
        result["from_state"] = "active"
        result["to_state"] = "wait"
    else:
        # Wait → active (e.g. ウェイト状態の...アクティブ状態になった)
        result["from_state"] = "wait"
        result["to_state"] = "active"

    if "メインフェイズの間" in text:
        result["phase"] = "main"

    # Extract count (e.g. "3人以上" or "2枚以上")
    for pat, op, unit in [
        (r"(\d+)人以上", ">=", "人"),
        (r"(\d+)枚以上", ">=", None),
        (r"(\d+)人", "=", "人"),
        (r"(\d+)枚", "=", None),
    ]:
        m = re.search(pat, text)
        if m:
            result["count"] = int(m.group(1))
            result["operator"] = op
            if unit:
                result["unit"] = unit
            break

    return result


def _try_otherwise(text):
    """それ以外の場合 — else/otherwise condition."""
    if "それ以外の場合" not in text:
        return None
    return {"type": "otherwise_condition", "text": text}


def _try_heart_possession(text):
    if not re.search(
        r"{{icon_([^}]+)\.png\|[^}}]+}}(?:[^持]*)(持たない|を持つ)", text
    ) and not ("ハート" in text and ("持たない" in text or "を持つ" in text)):
        return None
    result = {
        "type": "location_condition",
        "card_type": "member_card",
        "location": "stage",
        "text": text,
    }
    if "持たない" in text:
        result["negation"] = True
    if "icon_all" in text:
        result["heart_type"] = "all"
    return result


def _try_live_mid(text):
    if "ライブ中" not in text:
        return None
    result = {"text": text}
    count_match = re.search(r"(\d+)枚以上", text)
    if count_match:
        result["type"] = "card_count_condition"
        result["count"] = int(count_match.group(1))
        result["operator"] = ">="
        result["card_type"] = "live_card"
        result["target"] = "self"
        result["temporal"] = "during_live"
    else:
        result["type"] = "temporal_condition"
        result["temporal"] = "during_live"
    if "手札にある" in text:
        result["location"] = "hand"
    elif "ステージにいる" in text:
        result["location"] = "stage"
    # Also extract generic fields (target, location from full patterns, etc.)
    tgt = extract_target(text)
    if tgt and "target" not in result:
        result["target"] = tgt
    loc = extract_location(text)
    if loc and "location" not in result:
        result["location"] = loc
    return result


def _extract_generic_fields(condition, text):
    """Extract all generic fields from text into condition dict (no early return)."""
    # Character names: 「A」か「B」か「C」がいる (any number of names OR-ed)
    cm = re.search(r"((?:「[^」]+」か? ?)+)がいる", text)
    if cm:
        names = re.findall(r"「([^」]+)」", cm.group(1))
        if names:
            condition["characters"] = names

    # Use positional check for non-contiguous comparison patterns
    # Target
    tgt = extract_target(text)
    if tgt:
        condition["target"] = tgt

    # Location (single or multiple via 'と' conjunction)
    loc = extract_location(text)
    if loc:
        condition["location"] = loc
    locs = extract_locations(text)
    if locs:
        condition["locations"] = locs

    # Heart count
    if "heart" in text and (
        "つ以上持つ" in text or "枚持つ" in text or "つ持つ" in text
    ):
        hc = extract_count(text)
        if hc:
            condition["count"] = hc
            if re.search(r"heart_\d+.*?heart_\d+", text):
                hts = []
                for i in range(1, 7):
                    if f"heart_0{i}" in text:
                        hts.append(f"heart_0{i}")
                if hts:
                    condition["resource_type"] = "heart"
                    condition["heart_types"] = hts
                    tm = re.search(r"合計(\d+)種類以上", text)
                    if tm:
                        condition["types_count"] = int(tm.group(1))
                        condition["operator"] = ">="
            else:
                for pat, rt in [
                    ("heart_01", "heart_01"),
                    ("heart_02", "heart_02"),
                    ("heart_06", "heart_06"),
                ]:
                    if pat in text:
                        condition["resource_type"] = rt
                        break
                else:
                    condition["resource_type"] = "heart"
    # Extract heart_colors from any condition text with {{heart_XX.png}} icons
    # Used by location_condition to check collective presence of specific heart colors
    hm = re.findall(r"{{heart_(\d+)\.png\|heart\d+}}", text)
    if hm:
        colors = sorted(set(f"heart{m.zfill(2)}" for m in hm))
        condition["heart_colors"] = colors

    # Energy count
    if "エネルギー" in text and "枚" in text:
        condition["resource_type"] = "energy"
        ec = extract_count(text)
        if ec:
            condition["count"] = ec

    # Surplus heart
    if "余剰ハート" in text:
        condition["resource_type"] = "surplus_heart"
        sc = extract_count(text)
        if sc:
            condition["count"] = sc

    # Card type, count, operator
    ct = extract_card_type(text)
    if ct:
        condition["card_type"] = ct
    cnt = extract_count(text)
    if cnt:
        condition["count"] = cnt
    op = extract_operator(text)
    if op:
        condition["operator"] = op

    # Comparison targets/operators/types
    # Handle both contiguous "相手より" and non-contiguous "相手が...より"
    for tgt_text, tgt in COMPARISON_TARGETS.items():
        if tgt_text.endswith("より") and len(tgt_text) >= 4:
            noun = tgt_text[:-2]  # Remove 2-char marker 'より'
            if noun in text and "より" in text:
                noun_pos = text.find(noun)
                marker_pos = text.find("より", noun_pos + len(noun))
                if noun_pos >= 0 and marker_pos > noun_pos:
                    condition["comparison_target"] = tgt
                    break
        elif tgt_text in text:
            condition["comparison_target"] = tgt
            break
    for op_text, op in COMPARISON_OPERATORS.items():
        if op_text in text:
            condition["operator"] = op
            break
    for kw, ct in COMPARISON_TYPES.items():
        if kw in text:
            condition["comparison_type"] = ct
            break

    # Aggregate
    if "合計" in text:
        condition["aggregate"] = "total"

    # Exact match
    if "ちょうど" in text or "同じ" in text:
        condition["operator"] = "="
        if "同じ" in text:
            condition["comparison_type"] = "equality"
            condition["type"] = "comparison_condition"

    # Negation (いない)
    if "いない" in text and "メンバーがいない" in text:
        condition["negation"] = True

    # Includes
    if "含む" in text and "その中に" in text:
        condition["includes"] = True
        condition["includes_pattern"] = "nested"

    # Movement
    if "移動した" in text:
        condition["movement"] = "moved"
    elif "移動する" in text:
        condition["movement"] = "moves"
    if "移動している" in text:
        condition["movement_state"] = "has_moved"

    # Temporal scope
    for kw, tmp in [("このターン", "this_turn"), ("このライブ", "this_live")]:
        if kw in text:
            condition["temporal"] = tmp
            condition["temporal_scope"] = tmp
            break

    # Distinct flags
    for kw in [
        "名前が異なる",
        "カード名が異なる",
        "グループ名が異なる",
        "コストがそれぞれ異なる",
    ]:
        if kw in text:
            condition["distinct"] = True
            break

    # All areas
    if "エリアすべて" in text:
        condition["all_areas"] = True

    # Exclude self
    for kw in ["ほかのメンバー", "このメンバー以外", "このメンバー以外の"]:
        if kw in text:
            condition["exclude_self"] = True
            break

    # Any_of values
    if "いずれか" in text:
        vm = re.search(r"(\d+)(?:、(\d+))+(?:のいずれか)", text)
        if vm:
            condition["values"] = [int(v) for v in re.findall(r"\d+", vm.group(0))]

    # Group
    gns = extract_group_names(text)
    if gns:
        condition["group_names"] = gns
    # Detect "のみ" (only/all members must match the group)
    if gns:
        if "のみの場合" in text or (
            "のみ" in text and ("ステージ" in text or "メンバー" in text)
        ):
            condition["all_members"] = True

    # Cost limit
    cl = extract_cost_limit(text)
    if cl:
        condition["cost_limit"] = cl

    # Position
    pos = extract_position(text)
    if pos:
        if isinstance(pos, dict):
            condition.update(pos)
        else:
            condition["position"] = {"position": pos}


def _infer_condition_type(condition, text):
    """Determine condition type from extracted fields (mutates condition in place)."""
    group_names = condition.get("group_names")
    location = condition.get("location")
    card_type = condition.get("card_type")
    count = condition.get("count")
    operator = condition.get("operator")
    position = condition.get("position")

    # comparison_target (directional: self vs opponent) takes priority over location/card_type
    # comparison_type with "equality" should NOT override — the engine handles equality
    # in location_condition via target="both" logic
    if condition.get("comparison_target"):
        condition["type"] = "comparison_condition"
    elif (
        condition.get("comparison_type")
        and condition.get("comparison_type") != "equality"
    ):
        condition["type"] = "comparison_condition"
    elif condition.get("resource_type"):
        condition["type"] = "comparison_condition"
    elif group_names:
        condition["type"] = "group_condition"
        if "コスト" in text and ("低い" in text or "高い" in text):
            condition["comparison_type"] = "cost"
            condition["operator"] = "<" if "低い" in text else ">"
            cm = extract_cost_modification(text)
            if cm:
                condition.update(cm)
    elif location and card_type:
        condition["type"] = "location_condition"
    elif location and position:
        condition["type"] = "position_condition"
    elif condition.get("operator") and condition.get("target"):
        condition["type"] = "comparison_condition"
    elif condition.get("aggregate") == "total":
        if "コスト" in text or "合計が" in text:
            # "コストの合計がN" or "合計がN" → cost comparison, not score threshold
            condition["type"] = "comparison_condition"
            condition["comparison_type"] = "cost"
            condition["operator"] = "="
            # Extract the number from "合計がN" or "合計が、N"
            cm = re.search(r"合計が、?(\d+)", text)
            if cm:
                condition["count"] = int(cm.group(1))
        else:
            condition["type"] = "score_threshold_condition"
    elif location and condition.get("target"):
        condition["type"] = "location_condition"
    elif location and operator:
        condition.setdefault("target", "self")
        condition["type"] = "location_condition"
    elif condition.get("card_type"):
        condition.setdefault("count", 1)
        condition.setdefault("operator", ">=")
        condition["type"] = "card_count_condition"
    elif text.strip() and any(
        k in condition
        for k in (
            "operator",
            "count",
            "location",
            "card_type",
            "target",
            "comparison_type",
            "group_names",
        )
    ):
        condition.setdefault("count", 1)
        condition.setdefault("operator", ">=")
        condition.setdefault("target", "self")
        condition["type"] = "comparison_condition"
    elif text.strip():
        condition["type"] = "custom"
    else:
        return None
    return condition


def parse_condition(text: str) -> Dict[str, Any]:
    """Parse a condition text using priority-ordered handler cascade."""
    text = strip_parenthetical(text)

    # Try early-return handlers (most specific first)
    for handler in [
        _try_complex,
        _try_compound,
        _try_distinct,
        _try_state_change,
        _try_or,
        _try_card_count,
        _try_both,
        _try_temporal_this_turn,
        _try_temporal_turn_phase,
        _try_baton_touch,
        _try_temporal_count,
        _try_or,
        _try_either_target,
        _try_movement,
        _try_appearance,
        _try_energy_state,
        _try_state,
        _try_revealed,
        _try_opponent_choice,
        _try_unless_pay,
        _try_position_change,
        _try_position,
        _try_ability_negation,
        _try_otherwise,
        _try_heart_possession,
        _try_live_mid,
    ]:
        result = handler(text)
        if result is not None:
            # Add scope/aggregate fields for "自分と相手" conditions
            if "scope" not in result and "自分と相手" in text:
                result["scope"] = "both"
            if "aggregate" not in result and "合計" in text:
                result["aggregate"] = "total"
            # Add position for ALL condition types (not just appearance_condition)
            if "position" not in result:
                for kw, pos in POSITION_KEYWORDS.items():
                    if kw in text:
                        result["position"] = pos
                        break
            return result

    # Fall-through: generic field extraction + type inference
    condition = {"text": text}
    _extract_generic_fields(condition, text)
    # Add scope for conditions that span both players
    if "scope" not in condition and "自分と相手" in text:
        condition["scope"] = "both"
    return _infer_condition_type(condition, text)


# ============== CONSOLIDATED NORMALIZATION ==============


def normalize_action(obj, original_text=None):
    """DEPRECATED: No-op stub. Normalization is now inline in parse_action."""
    return obj


def _infer_card_type(text, action=None):
    """Infer card_type from text context."""
    # Check energy BEFORE broad メンバー match (エネルギーの下にメンバー may contain メンバー)
    if "エネルギーカード" in text:
        return "energy_card"
    # Ambiguous: text contains BOTH メンバーカード and ライブカード (e.g. "メンバーカードかライブカード")
    if "メンバーカード" in text and "ライブカード" in text:
        return "card"
    if "メンバーカード" in text or ("メンバー" in text and "エネルギー" not in text):
        return "member_card"
    if "ライブカード" in text:
        return "live_card"
    if "エネルギー" in text:
        return "energy_card"
    if "カード" in text:
        return "card"
    # Infer from source
    if action:
        src = action.get("source", "")
        if src == "stage":
            return "member_card"
        if src in (
            "deck",
            "deck_top",
            "hand",
            "discard",
            "revealed_cards",
            "revealed_remaining",
        ):
            return "card"
    return "card"


def _count_resource_icons(text):
    """Count resource icons in text (heart_XX, blade, energy)."""
    heart_count = len(re.findall(r"{{heart_\d+\.png\|heart\d+}}", text))
    blade_count = text.count("{{icon_blade.png|ブレード}}")
    energy_count = text.count("{{icon_energy.png|E}}")
    all_heart_count = text.count("{{icon_all.png|ハート}}")
    total = heart_count + blade_count + energy_count + all_heart_count
    return total


def infer_resource(d, text):
    """Infer resource type for gain_resource actions."""
    # Icon-based inference (most specific)
    if "{{icon_blade.png|ブレード}}" in text:
        d["resource"] = "blade"
    elif "{{icon_energy.png|E}}" in text:
        d["resource"] = "energy"
    elif "{{heart_03.png|heart03}}" in text:
        d["resource"] = "heart03"
    elif "{{heart_02.png|heart02}}" in text:
        d["resource"] = "heart02"
    elif "{{heart_01.png|heart01}}" in text:
        d["resource"] = "heart01"
    # Text-based inference
    elif "ブレード" in text:
        d["resource"] = "blade"
    elif "ハート" in text:
        d["resource"] = "heart"
    else:
        d["resource"] = "generic"


def infer_count_from_icons(d, text):
    """Infer count for gain_resource from icon occurrences."""
    blade_count = text.count("{{icon_blade.png|ブレード}}")
    if blade_count > 0:
        d["count"] = blade_count
        return
    heart_count = len(re.findall(r"{{heart_\d+\.png\|heart\d+}}", text))
    if heart_count > 0:
        d["count"] = heart_count
        return
    # Try numeric extract
    count_match = re.search(r"(\d+)つ", text)
    if count_match:
        d["count"] = int(count_match.group(1))


def _fill_defaults(action, text):
    """Consolidated post-dispatch normalization. Fills defaults every action needs."""
    a = action.get("action")
    if a == "draw":
        action["action"] = "draw_card"
        a = "draw_card"
    if a == "draw_card":
        action.setdefault("source", "deck")
        action.setdefault("destination", "hand")
    # Shuffle is always combined with a move action (shuffle then place).
    # If dispatch matched shuffle but text also has a destination pattern, emit move_cards with shuffle flag.
    if a == "shuffle":
        dest = extract_destination(text)
        if dest:
            action["action"] = "move_cards"
            action["shuffle"] = True
            action["destination"] = dest
            if "source" not in action:
                s = extract_source(text)
                if s:
                    action["source"] = s
            if "card_type" not in action:
                ct = _infer_card_type(text, action)
                if ct:
                    action["card_type"] = ct
            a = "move_cards"

    if action.get("source") == "selected_cards":
        action.setdefault("count", 1)
    if a == "gain_resource" and "resource" not in action:
        infer_resource(action, text)
    if a == "gain_resource":
        if action.get("count") is None:
            infer_count_from_icons(action, text)
        if action.get("count") is None:
            action["count"] = 1
        if (
            action.get("resource") in ("heart", None)
            and "heart_color" not in action
            and "heart_colors" not in action
        ):
            hm = re.search(r"{{heart_(\d+)\.png\|heart\d+}}", text)
            if hm:
                action["heart_color"] = f"heart{hm.group(1).zfill(2)}"
    if a == "gain_resource" and "duration" not in action:
        dur = extract_duration(text)
        if dur:
            action["duration"] = dur
    if a == "modify_score" and "value" not in action:
        vm = re.search(r"[+＋](\d+)", text)
        if vm:
            action["value"] = int(vm.group(1))
    if a == "modify_required_hearts" and "heart_color" not in action:
        hm = re.search(r"{{heart_(\d+)\.png\|heart\d+}}", text)
        if hm:
            action["heart_color"] = f"heart{hm.group(1).zfill(2)}"
    if "original_value" not in action and ("元々持つ" in text or "元々" in text):
        action["original_value"] = True
    if "このカード" in text:
        action["self_target"] = True
    # Position from text (for ALL action types, not just appearance_condition)
    if (
        "position" not in action
        and "exclude_position" not in action
        and "source_position" not in action
    ):
        for kw, pos in POSITION_KEYWORDS.items():
            if kw in text:
                action["position"] = pos
                break
    if (
        a == "move_cards"
        and action.get("destination") == "hand"
        and "source" not in action
    ):
        action["source"] = "discard"
    if a == "move_cards":
        if "source" not in action:
            s = extract_source(text)
            if s:
                action["source"] = s
        if "source" not in action:
            # Fallback: infer source from destination common patterns
            dest = action.get("destination", "")
            if dest in ("deck_top", "deck_bottom", "deck"):
                if "メンバー" not in text:
                    action["source"] = "hand"
            elif dest in ("discard",):
                if "このカード" in text:
                    action["source"] = "deck_top"
                elif "エネルギー" not in text:
                    action["source"] = "hand"
        if "destination" not in action:
            d = extract_destination(text)
            if d:
                action["destination"] = d
        # Relative cost search: "そのメンバーのコストに2を足した数に等しいコスト"
        # This references the card moved by the previous sub-action.
        if (
            action.get("source") == "discard"
            and action.get("destination") == "same_area"
            and "そのメンバーのコストに" in text
            and "足した数に等しいコスト" in text
        ):
            m = re.search(r"コストに(\d+)を足した数に等しいコスト", text)
            if m:
                action["cost_reference"] = "previous_moved_card"
                action["cost_offset"] = int(m.group(1))
                action.setdefault("cost_limit_operator", "=")
        if "card_type" not in action:
            ct = _infer_card_type(text, action)
            if ct:
                action["card_type"] = ct
        if "state_change" not in action and "ウェイト状態" in text:
            action["state_change"] = "wait"
        # If after inference source and destination are both missing/None,
        # or destination is a zone-only reference without source,
        # this isn't really a move_cards — demote to custom
        has_source = action.get("source") is not None
        has_dest = action.get("destination") is not None
        dest_val = action.get("destination", "")
        zone_only_dest = (
            dest_val in ("live_card_zone", "success_live_zone", "stage")
            and not has_source
        )
        if (not has_source and not has_dest) or zone_only_dest:
            action["action"] = "custom"
            a = "custom"
        card_type_kws = [
            ("live_card", "ライブカード"),
            ("member_card", "メンバーカード"),
            ("energy_card", "エネルギーカード"),
        ]
        if action.get("card_type") and re.search(
            r"(ライブカード|メンバーカード|エネルギーカード).*か.*(ライブカード|メンバーカード|エネルギーカード)",
            text,
        ):
            or_types = [t for t, kw in card_type_kws if kw in text]
            if len(or_types) >= 2:
                action["or_card_types"] = or_types
                action.pop("card_type", None)
    # OR card types for ALL action types (not just move_cards/select)
    if a not in ("move_cards", "select") and "or_card_types" not in action:
        card_type_kws = [
            ("live_card", "ライブカード"),
            ("member_card", "メンバーカード"),
            ("energy_card", "エネルギーカード"),
        ]
        if re.search(
            r"(ライブカード|メンバーカード|エネルギーカード).*か.*(ライブカード|メンバーカード|エネルギーカード)",
            text,
        ):
            or_types = [t for t, kw in card_type_kws if kw in text]
            if len(or_types) >= 2:
                action["or_card_types"] = or_types
    if action.get("source") == "under_member" and a != "place_energy_under_member":
        action["action"] = "place_energy_under_member"
        a = "place_energy_under_member"
        action.setdefault("energy_count", 1)
        action.setdefault("target_member", "this_member")
    if (
        a == "move_cards"
        and action.get("source") in ("revealed_remaining", "revealed_cards")
        and "dynamic_count" not in action
    ):
        action["dynamic_count"] = {
            "type": "revealed_cards",
            "reference": "previous_reveal",
        }
    if (
        a == "move_cards"
        and action.get("source") == "revealed_card"
        and action.get("count") is None
    ):
        action["count"] = 1
    if not action.get("all") and re.search(
        r"すべての|全ての|全部の|全て|全員|全体", text
    ):
        action["all"] = True
    if action.get("all"):
        action.pop("count", None)
    if "それぞれ" in text or "ずつ" in text:
        action["multiple_targets"] = True
    if (
        action.get("count") is None
        and "dynamic_count" not in action
        and not action.get("any_number")
        and not action.get("all")
    ):
        extracted = extract_count(text)
        if extracted is not None:
            action["count"] = extracted
        else:
            icon_count = _count_resource_icons(text)
            if icon_count > 0:
                action["count"] = icon_count
            elif a in (
                "move_cards",
                "draw_card",
                "gain_resource",
                "reveal",
                "look_at",
                "change_state",
                "restriction",
            ):
                if a == "change_state" and action.get("group_names"):
                    pass
                elif a == "draw_card" and (
                    "置いた枚数分" in text or "置いた枚数" in text
                ):
                    action["count"] = 0
                else:
                    action["count"] = 1
    # Fix remaining custom actions that have enough parsed info
    if action.get("action") == "custom":
        if action.get("ability_gain"):
            action["action"] = "gain_ability"
        elif re.search(r"枚数.*\d*枚増やす", text) or re.search(
            r"枚数.*\d*枚増え", text
        ):
            action["action"] = "modify_limit"
            action.setdefault("operation", "increase")
            cnt = extract_count(text)
            if cnt:
                action["count"] = cnt
        elif re.search(r"枚数.*\d*枚減らす", text) or re.search(
            r"枚数.*\d*枚減る", text
        ):
            action["action"] = "modify_limit"
            action.setdefault("operation", "decrease")
            cnt = extract_count(text)
            if cnt:
                action["count"] = cnt
        elif re.search(r"スコアを[+＋]\d+する", text):
            action["action"] = "modify_score"
            action.setdefault("operation", "add")
            vm = re.search(r"([+＋])(\d+)", text)
            if vm:
                action["value"] = int(vm.group(2))

    if "optional" not in action and extract_optional(text):
        action["optional"] = True
    if "max" not in action and extract_max(text):
        action["max"] = True
    if "好きな枚数" in text or "好きな枚数まで" in text or "任意の枚数" in text:
        action["any_number"] = True
        action.pop("count", None)
    # Extract blade count limit (e.g. "ブレードの数が3つ以下" → blade_limit=3, operator=<=")
    if "blade_limit" not in action and "ブレード" in text:
        bl = extract_blade_limit(text)
        if bl:
            action.update(bl)
    if action.get("original_value") and "元々の" in text:
        cnt = extract_count(text)
        if cnt is not None:
            action["original_count"] = cnt
        op = extract_operator(text)
        if op:
            action["original_operator"] = op
    if a == "select" and "のどちらか" in text:
        or_types = []
        for t, kw in [
            ("live_card", "ライブカード"),
            ("member_card", "メンバーカード"),
            ("energy_card", "エネルギーカード"),
        ]:
            if kw in text:
                or_types.append(t)
                if t == "member_card":
                    cl = extract_cost_limit(text)
                    if cl:
                        action["cost_limit"] = cl
        if len(or_types) >= 2:
            action["or_card_types"] = or_types
            action.pop("card_type", None)
    if action.get("per_unit") and "dynamic_count" not in action:
        action["dynamic_count"] = {"type": "per_unit", "reference": "unit_count"}
        if "count" in action and action["count"] is None:
            del action["count"]

    # Parse gained ability text into structured effect
    # Note: NOT done here — done in parse_ability to avoid recursion


def parse_action(text: str) -> Dict[str, Any]:
    """Parse an action text."""
    # Check for optional draw action "カードを1枚引いてもよい" - CHECK THIS FIRST
    if "カードを1枚引いてもよい" in text:
        return {"text": text, "action": "draw_card", "count": 1, "optional": True}

    # Strip parenthetical notes first (before any other processing)
    text = strip_parenthetical(text)

    # Check for per-unit scaling (e.g., "メンバー1人につき") - CHECK THIS FIRST before any text splitting
    if PER_UNIT_MARKER in text:
        # Extract the per-unit pattern
        per_unit_match = re.search(r"(.*?)につき", text)
        if per_unit_match:
            per_unit_text = per_unit_match.group(1).strip()
            # Extract the count if present (e.g., "メンバー1人")
            count_match = re.search(r"(\d+)(?:人|枚|つ)", per_unit_text)
            if count_match:
                per_unit_count = int(count_match.group(1))
            else:
                per_unit_count = 1
            # Extract the unit type (e.g., "メンバー")
            per_unit_type = None
            if (
                "ライブ中のカード" in per_unit_text
                or "ライブカード置き場" in per_unit_text
            ):
                per_unit_type = "live_card_zone"
            elif "メンバー" in per_unit_text:
                per_unit_type = "member"
            elif "カード" in per_unit_text:
                per_unit_type = "card"
            # Store per_unit info to be set later
            per_unit_info = {
                "per_unit": True,
                "per_unit_count": per_unit_count,
            }
            if per_unit_type:
                per_unit_info["per_unit_type"] = per_unit_type
            # Infer action from text
            if "ブレードを得る" in text or "選んだブレード" in text:
                per_unit_info["action"] = "gain_resource"
                per_unit_info["resource"] = "blade"
                # Extract resource icon count
                icon_count = text.count("{{icon_blade.png|ブレード}}")
                if icon_count > 0:
                    per_unit_info["count"] = icon_count
                # Set duration if present
                if "ライブ終了時まで" in text:
                    per_unit_info["duration"] = "live_end"
            elif "ハートを得る" in text or "選んだハート" in text:
                per_unit_info["action"] = "gain_resource"
                per_unit_info["resource"] = "heart"
                # Set duration if present
                if "ライブ終了時まで" in text:
                    per_unit_info["duration"] = "live_end"
            elif "引く" in text:
                per_unit_info["action"] = "draw_card"
                # Set duration if present
                if "ライブ終了時まで" in text:
                    per_unit_info["duration"] = "live_end"
            # Strip the per-unit pattern from the text
            text = text.replace(per_unit_match.group(0), "").strip()

    # Strip duration prefixes
    text, dur_code = _strip_duration_prefix(text)
    action = {"text": text}
    if dur_code:
        action["duration"] = dur_code
    # Also check for duration keywords embedded in text
    if "duration" not in action:
        if "ライブ終了時まで" in text:
            action["duration"] = "live_end"
        elif "このターンの間" in text:
            action["duration"] = "this_turn"

    # Strip parenthetical notes for the rest of processing
    text = strip_parenthetical(text)

    # Extract count, card_type, target, state_change for dispatch rules
    count = extract_count(text)
    target = extract_target(text)
    card_type = extract_card_type(text)
    state_change = extract_state_change(text)

    if "per_unit_info" in locals():
        action.update(per_unit_info)

    # Extract effect constraints (最小/最大/未満にはならない/以上にはならない)
    constraint_patterns = {
        "最小": ("min", r"最小(\d+)"),
        "最大": ("max", r"最大(\d+)"),
        "未満にはならない": ("minimum_value", r"(\d+)未満にはならない"),
        "以上にはならない": ("maximum_value", r"(\d+)以上にはならない"),
    }
    for keyword, (constraint_type, pattern) in constraint_patterns.items():
        if keyword in text:
            constraint_match = re.search(pattern, text)
            if constraint_match:
                action["effect_constraint"] = (
                    f"{constraint_type}:{constraint_match.group(1)}"
                )
            break

    # Extract source - handle "手札を" pattern for discard
    if "手札を" in text and "控え室に置く" in text:
        action["source"] = "hand"
    # Handle source description patterns: "この[X]で控え室に置かれた[Y]"
    # Extract the description of which cards from the source are being targeted
    source_desc_match = re.search(r"この([^で]+)で控え室に置かれた", text)
    if source_desc_match:
        action["source"] = "discard"
    # Check for under_member source (e.g., "下に置かれているエネルギーカード")
    if "下に置かれているエネルギーカード" in text:
        action["source"] = "under_member"
        action["card_type"] = "energy_card"

    # Extract source
    source = extract_source(text)
    if source:
        action["source"] = source
        # Special case: if source is deck_top and no count was extracted, default to 1
        if source == "deck_top" and "count" not in action:
            action["count"] = 1
        # Special case: if source is revealed_remaining, use dynamic_count
        elif source == "revealed_remaining" and "count" not in action:
            action["dynamic_count"] = {
                "type": "revealed_cards",
                "reference": "previous_reveal",
            }
        # Special case: if source is revealed_cards and no count was extracted, use dynamic_count
        elif source == "revealed_cards" and "count" not in action:
            action["dynamic_count"] = {
                "type": "revealed_cards",
                "reference": "previous_reveal",
            }
        # Special case: if source is revealed_card and no count was extracted, set to 1
        elif source == "revealed_card" and "count" not in action:
            action["count"] = 1

    # Extract destination
    destination = extract_destination(text)
    if destination:
        action["destination"] = destination
    # Check for "好きな順番で" (in any order) placement
    if "好きな順番で" in text:
        action["placement_order"] = "any_order"
    # Extract deck position (Q226: 一番上から4枚目)
    deck_position = extract_deck_position_for_action(text)
    if deck_position:
        action.update(deck_position)

    # Extract cost limit specifically for move_cards actions
    cost_limit = extract_cost_limit(text)
    if cost_limit:
        action["cost_limit"] = cost_limit
        # Extract cost_limit_operator: 以下(<=), 以上(>=), exact(=), 未満(<), 超(>)
        if "以下" in text:
            action["cost_limit_operator"] = "<="
        elif "以上" in text:
            action["cost_limit_operator"] = ">="
        elif "未満" in text:
            action["cost_limit_operator"] = "<"
        elif "超" in text:
            action["cost_limit_operator"] = ">"
        else:
            action["cost_limit_operator"] = "="  # bare number → exact match

    # Check for card name matching constraints (Q236/Q237 - 日野下花帆 pattern)
    # Pattern: "これにより公開したカードのカード名がすべて含まれる"
    if "カード名がすべて含まれる" in text or "カード名が含まれる" in text:
        action["name_constraint"] = "contains_all"
        action["name_constraint_source"] = "revealed_card"

    # Check for distinct card name constraints (Q118)
    if "カード名の異なる" in text:
        action["distinct"] = "card_name"

    if state_change:
        action["state_change"] = state_change

    if count:
        action["count"] = count
    elif "これにより引いた枚数と同じ枚数を" in text:
        action["dynamic_count"] = {"type": "drawn_cards", "reference": "previous_draw"}
    else:
        dynamic_count = extract_dynamic_count(text)
        if dynamic_count:
            action["dynamic_count"] = dynamic_count

    if card_type:
        action["card_type"] = card_type

    if target:
        action["target"] = target

    # Extract position restrictions (e.g., "センター", "センターエリア")
    for keyword, position in POSITION_KEYWORDS.items():
        if keyword in text:
            action["position"] = position
            break

    # Extract exclude_self for actions (e.g., "このメンバー以外の" or "「character name」以外")
    if "このメンバー以外" in text or "ほかのメンバー" in text:
        action["exclude_self"] = True
    # Also check for specific character name exclusions like "「鬼塚冬毬」以外"
    if re.search(r"「.+」以外", text):
        action["exclude_self"] = True
        # Extract the quoted character names being excluded
        quoted_exclusions = extract_quoted_text(text)
        if quoted_exclusions:
            categorized = categorize_quoted_text(quoted_exclusions)

    # Extract group names from 『』 brackets
    group_names = extract_group_names(text)
    if group_names:
        action["group_names"] = group_names

    # Check for ability gain pattern - MUST BE CHECKED BEFORE general quoted text extraction
    # Pattern 1: Explicit "能力を得る" (gain ability) with quoted text
    # Pattern 2: "～を得る" where quoted text contains icon syntax (indicates ability text)
    quoted_text = extract_quoted_text(text)
    is_ability_gain = False
    if "を得る" in text and "能力" in text and quoted_text:
        is_ability_gain = True
    elif "を得る" in text and quoted_text:
        categorized = categorize_quoted_text(quoted_text)
        if categorized["abilities"]:
            is_ability_gain = True

    if is_ability_gain:
        action["action"] = "gain_ability"
        # Populate ability_gain with the actual ability text
        if quoted_text:
            categorized = categorize_quoted_text(quoted_text)
            if categorized["abilities"]:
                action["ability_gain"] = (
                    re.sub(r"\{\{[^}]+\}\}", "", categorized["abilities"][0])
                    .replace("「", "")
                    .replace("」", "")
                    .strip()
                )
            elif categorized["characters"]:
                # These are likely character names or card names
                # Convert to QuotedText struct format (text, quoted_type)
                # Only set if single character - Rust expects single QuotedText, not array
                if len(categorized["characters"]) == 1:
                    action["quoted_text"] = {
                        "text": categorized["characters"][0],
                        "quoted_type": "character",
                    }
                # For multiple characters, don't set quoted_text to avoid deserialization errors
                # action['gained_ability'] = {'text': ability_text}
        # Early return — must come before the dispatch table (which resets action to 'custom')
        _fill_defaults(action, text)
        return action
    else:
        # Extract quoted text from 「」 for other contexts
        quoted_text = extract_quoted_text(text)
        if quoted_text:
            categorized = categorize_quoted_text(quoted_text)
            if categorized["characters"]:
                # These are likely character names or card names
                # Only set quoted_text for single character - Rust expects QuotedText struct, not array
                if len(categorized["characters"]) == 1:
                    action["quoted_text"] = {
                        "text": categorized["characters"][0],
                        "quoted_type": "character",
                    }
                # For multiple characters, don't set quoted_text to avoid deserialization errors

    # Extract position
    position = extract_position(text)
    if position:
        if isinstance(position, dict):
            action.update(position)
        else:
            # Output as PositionInfo struct format
            action["position"] = {"position": position}

    # Extract optional flag
    if extract_optional(text):
        action["optional"] = True

    # Check for multiple targets pattern (ずつ or それぞれ)
    if "ずつ" in text or "それぞれ" in text:
        action["multiple_targets"] = True

    # Extract max flag
    if extract_max(text):
        action["max"] = True

    # ======================== DISPATCH TABLE ========================
    # Replaces the ~730-line if/elif chain with data-driven rules.
    # Each rule: (condition_text_or_fn, action_type, field_setter_fn_or_None)
    # Order matches original if/elif priority.

    def _ic(t, tag):
        return t.count(tag) or None

    def _handle_dynamic_count(text, action):
        """Handle dynamic count patterns in look_at actions."""
        # Check for dynamic count patterns (e.g., "スコアに2を足した数に等しい枚数")
        dynamic_count = extract_dynamic_count(text)
        if dynamic_count:
            action["dynamic_count"] = dynamic_count
        return action

    def _handle_cost_modification(text, action):
        """Handle cost modification patterns."""
        if "減る" in text or "減らす" in text or "マイナス" in text:
            action["operation"] = "subtract"
        elif (
            "増える" in text
            or "増やす" in text
            or "プラス" in text
            or "コストを+" in text
        ):
            action["operation"] = "add"
        # Set location for hand-based cost reductions (手札にある/手札から)
        if "手札" in text:
            action["location"] = "hand"
        # Extract cost limit (e.g. "コスト10の" → limit to cards with cost 10)
        cl = extract_cost_limit(text)
        if cl is not None:
            action["cost_limit"] = cl
            if "以下" in text:
                action["cost_limit_operator"] = "<="
            elif "以上" in text:
                action["cost_limit_operator"] = ">="
            elif "未満" in text:
                action["cost_limit_operator"] = "<"
            elif "超" in text:
                action["cost_limit_operator"] = ">"
            else:
                action["cost_limit_operator"] = "="
        # Extract numeric value from patterns like "コストは2減る" or "コストを+1する"
        value_match = re.search(r"コスト[はがを](\d+)(減る|減らす|増える|増やす)", text)
        if value_match:
            action["value"] = int(value_match.group(1))
        else:
            vm2 = re.search(r"コスト[をがは][+＋](\d+)", text)
            if vm2:
                action["value"] = int(vm2.group(1))
        # Extract energy icon count
        icon_count = text.count("{{icon_energy.png|E}}")
        if icon_count > 0:
            action["count"] = icon_count
        return action

    def _set_score_op(t, a):
        """Set operation and value for modify_score from text patterns."""
        sm = re.search(r"([+\-])(\d+)", t)
        if sm:
            a["value"] = int(sm.group(2))
            a["operation"] = "remove" if sm.group(1) == "-" else "add"
            return
        cnt = extract_count(t)
        if cnt:
            a["value"] = cnt
            if "マイナス" in t or "減らす" in t or "減る" in t:
                a["operation"] = "remove"
            else:
                a["operation"] = "add"
            return
        if "プラス" in t or "増やす" in t or "増える" in t:
            a["operation"] = "add"
        elif "マイナス" in t or "減らす" in t or "減る" in t:
            a["operation"] = "remove"

    _R = []

    def R(cond, act, setter=None):
        _R.append((cond, act, setter))

    R(
        lambda t: "シャッフルする" in t
        or "シャッフルして" in t
        or ("シャッフルし" in t and "、" in t),
        "shuffle",
        lambda t, a: a.update({"target": "deck" if "デッキ" in t else "energy_deck"}),
    )
    R(lambda t: "入れ替える" in t or "入れ替えて" in t, "position_change", None)
    R(
        lambda t: "フォーメーションチェンジ" in t,
        "formation_change",
        lambda t, a: a.update(
            {"optional": extract_optional(t), "multiple_targets": True}
        ),
    )
    R(
        lambda t: "{{icon_energy.png|E}}" in t
        and ("支払う" in t or "支払って" in t)
        and "選び" not in t,
        "pay_energy",
        lambda t, a: a.update(
            {
                "energy": t.count("{{icon_energy.png|E}}"),
                "optional": "もよい" in t or "してもよい" in t,
            }
        )
        or None,
    )
    R(
        lambda t, a: destination == "under_member"
        and ("エネルギー" in t or "energy_card" in t),
        "place_energy_under_member",
        lambda t, a: a.update({"energy_count": count or 1}),
    )
    R(
        lambda t: "枚になるまで" in t and "引く" in t,
        "draw_until_count",
        lambda t, a: a.update(
            {
                "source": "deck",
                "destination": "hand",
                "target_count": int(re.search(r"(\d+)枚になるまで", t).group(1)),
            }
        ),
    )
    R(
        lambda t: "枚になるまで" in t and ("控え室に置く" in t or "控え室に置き" in t),
        "discard_until_count",
        lambda t, a: a.update(
            {"target_count": int(re.search(r"(\d+)枚になるまで", t).group(1))}
        ),
    )
    R(
        "カードを1枚引いてもよい",
        "draw_card",
        lambda t, a: a.update(
            {"count": 1, "optional": True, "source": "deck", "destination": "hand"}
        ),
    )
    R(
        lambda t: "引く" in t or "引き" in t or "引い" in t,
        "draw_card",
        lambda t, a: a.update({"source": "deck", "destination": "hand"}),
    )
    R(
        lambda t: "引いてもよい" in t,
        "draw_card",
        lambda t, a: a.update(
            {"source": "deck", "destination": "hand", "optional": True}
        ),
    )
    # Check for cost modification BEFORE general move_cards (which also matches source+dest)
    R(
        lambda t: re.search(r"コスト[はが](\d+)(減る|減らす|増える|増やす)", t)
        or re.search(r"ためのコストは(\d+)減る", t),
        "modify_cost",
        lambda t, a: _handle_cost_modification(t, a),
    )
    # move_cards with known source+destination beats change_state when both movement and state are specified
    R(
        lambda t, a: "source" in a
        and a.get("source")
        and "destination" in a
        and a.get("destination"),
        "move_cards",
        None,
    )
    R(
        lambda t, a: state_change and state_change != "",
        "change_state",
        lambda t, a: (
            a.update({"target": extract_target(t)}) if extract_target(t) else None,
            a.update({"card_type": "energy_card"})
            if "エネルギー" in t and "メンバー" not in t
            else None,
            a.update({"card_type": "member_card"})
            if "このメンバー" in t
            or (
                "メンバー" in t
                and ("ウェイト" in t or "レスト" in t or "アクティブ" in t)
            )
            else None,
        )[-1],
    )
    R(
        lambda t: "アクティブにしてもよい" in t or "アクティブにする" in t,
        "change_state",
        lambda t, a: a.update(
            {
                "state_change": "active",
                "card_type": "energy_card" if "エネルギー" in t else "member_card",
            }
        )
        or (a.update({"optional": True}) if "してもよい" in t else None),
    )
    R(
        lambda t: "のみ起動できる" in t or "のみ発動する" in t,
        "activation_restriction",
        lambda t, a: a.update({"restriction_type": "only"}),
    )
    R(
        "支払って発動させる",
        "activate_ability",
        lambda t, a: a.update({"activation_type": "pay_to_activate"}),
    )
    R(
        "ライブできない",
        "restriction",
        lambda t, a: a.update({"restriction_type": "cannot_live"}),
    )
    R(
        "アクティブにしない",
        "restriction",
        lambda t, a: a.update({"restriction_type": "cannot_activate"}),
    )
    R(
        "バトンタッチで控え室に置けない",
        "restriction",
        lambda t, a: a.update({"restriction_type": "cannot_baton_touch"}),
    )
    R(
        "置くことができない",
        "restriction",
        lambda t, a: a.update({"restriction_type": "cannot_place"}),
    )
    R(
        "置けない",
        "restriction",
        lambda t, a: a.update({"restriction_type": "cannot_place"}),
    )
    R(
        "登場できない",
        "restriction",
        lambda t, a: a.update({"restriction_type": "cannot_appear"}),
    )
    R(
        "移動できない",
        "restriction",
        lambda t, a: a.update({"restriction_type": "cannot_move"}),
    )
    R(
        lambda t: "加える" in t or "加え" in t,
        "move_cards",
        lambda t, a: a.update({"destination": "hand"}),
    )
    R(
        "ポジションチェンジ",
        "position_change",
        lambda t, a: (
            a.update({"target": extract_target(t)}),
            _handle_position_change_fields(t, a),
        )[-1],
    )
    R(lambda t: "移動させ" in t and "エリア" in t, "position_change", None)
    R(lambda t: "移動させ" in t and "エリア" not in t, "move_cards", None)
    R(
        lambda t: ("置く" in t or "置いて" in t) or ("置き" in t and "置き場" not in t),
        "move_cards",
        lambda t, a: a.update({"destination": extract_destination(t)})
        if "destination" not in a
        else None,
    )
    R(
        lambda t: "ブレードを得る" in t or "選んだブレード" in t,
        "gain_resource",
        lambda t, a: a.update(
            {"resource": "blade", "count": _ic(t, "{{icon_blade.png|ブレード}}")}
        ),
    )
    R(
        lambda t: "{{icon_blade.png|ブレード}}" in t and "得る" in t,
        "gain_resource",
        lambda t, a: a.update(
            {
                "resource": "blade",
                "count": t.count("{{icon_blade.png|ブレード}}") or None,
            }
        ),
    )
    R(
        lambda t: ("{{heart" in t and "得る" in t)
        or "ハートを得る" in t
        or "選んだハート" in t,
        "gain_resource",
        lambda t, a: a.update(
            {
                "resource": "heart",
                "count": len(re.findall(r"{{heart_\d+\.png\|heart\d+}}", t)) or None,
            }
        ),
    )
    R(
        lambda t: "もう一度エール" in t or "もう1度エール" in t,
        "re_yell",
        lambda t, a: a.update({"lose_blade_hearts": True})
        if "できない" not in t
        else None,
    )
    R(
        lambda t: ("見る" in t or "見て" in t or t.endswith("見")),
        "look_at",
        lambda t, a: (
            a.update({"source": "deck_top"}) if "デッキの上" in t else None,
            _handle_dynamic_count(t, a),
            a.update({"action": "look_at"}),
        ),
    )
    R(
        lambda t: "公開する" in t or "公開して" in t,
        "reveal",
        lambda t, a: a.update({"source": source or "hand"}),
    )
    R(
        lambda t: "1枚ずつ公開" in t or "枚ずつ公開" in t,
        "reveal",
        lambda t, a: (
            a.update({"per_unit": True, "per_unit_count": 1, "multiple_targets": True}),
            None,
        ),
    )
    R(
        lambda t: "選ぶ" in t or "選ん" in t or "選び" in t,
        "select",
        lambda t, a: a.update(
            {"heart_colors": [m.group(1) for m in re.finditer(r"\|(heart\d{2})}", t)]}
        )
        if not a.get("source") and not a.get("card_type") and "{{heart_" in t
        else None,
    )
    R(
        lambda t: "ブレードを得る" in t or "選んだブレード" in t,
        "gain_resource",
        lambda t, a: None,
    )  # already matched above, this is fallback
    R(lambda t: "ハートを得る" in t or "選んだハート" in t, "gain_resource", None)
    R(lambda t: "もう一度エール" in t or "もう1度エール" in t, "re_yell", None)
    R(lambda t: "登場させ" in t, "appear", None)
    R(lambda t: "起動でき" in t or "起動して" in t, "activate_ability", None)
    R(lambda t: "無効に" in t, "invalidate_ability", None)
    R(
        lambda t: "必要ハート" in t or "ハートを増やす" in t or "ハートを減らす" in t,
        "modify_required_hearts",
        None,
    )
    R(
        lambda t: "追加" in t,
        "modify_score",
        lambda t, a: a.update({"operation": "add"}),
    )
    R(
        lambda t: "スコアを1プラス" in t or "スコアをプラス" in t,
        "modify_score",
        lambda t, a: a.update({"operation": "add", "value": 1}),
    )
    R(
        "スコアを1マイナス",
        "modify_score",
        lambda t, a: a.update({"operation": "remove", "value": 1}),
    )
    R("以下から1つを選ぶ", "choice", None)
    R("ブレードの色を", "set_blade_type", None)
    R(
        lambda t: "ハートの色を" in t or ("ハートを" in t and "にする" in t),
        "gain_resource",
        lambda t, a: a.update({"resource": "heart", "heart_selection": True}),
    )
    # If "コスト" text contains heart icons, it's about required hearts (not energy cost)
    R(
        lambda t: ("コストを" in t or "コストが" in t or "コストは" in t)
        and "{{heart_" in t,
        "set_required_hearts",
        lambda t, a: a.update(
            {"heart_colors": [m.group(1) for m in re.finditer(r"\|(heart\d{2})}", t)]}
        )
        or a.update(
            {
                "count": len(
                    [m.group() for m in re.finditer(r"{{heart_\d+\.png\|heart\d+}}", t)]
                )
            }
        )
        or a.update({"text": t}),
    )
    R(
        lambda t: "コストを" in t or "コストが" in t or "コストは" in t,
        "modify_cost",
        lambda t, a: _handle_cost_modification(t, a),
    )
    R(
        lambda t: "繰り返してもよい" in t,
        "repeat_procedure",
        lambda t, a: (
            a.update({"max_repeats": int(re.search(r"(\d+)回", t).group(1))})
            if re.search(r"(\d+)回", t)
            else None
        ),
    )
    R("何もしない", "do_nothing", None)
    R(lambda t: t.strip() == "", "do_nothing", None)
    R(lambda t: "{{icon_energy.png|E}}" in t and "エネルギー" in t, "pay_energy", None)
    R(
        lambda t: "バトンタッチ" in t or "baton touch" in t.lower(),
        "play_baton_touch",
        None,
    )
    R("無効にできない", "invalidate_ability", lambda t, a: a.update({"optional": True}))
    R(
        lambda t: ("スコアは" in t or "スコアが" in t)
        and ("になる" in t or "なった" in t or "なっている" in t),
        "set_score",
        lambda t, a: a.update(
            {"value": int(re.search(r"(\d+).*(になる|なった|なっている)", t).group(1))}
        )
        if re.search(r"(\d+).*(になる|なった|なっている)", t)
        else None,
    )
    R(
        lambda t: "スコアを" in t,
        "modify_score",
        lambda t, a: (_set_score_op(t, a), a)[-1],
    )
    R(
        lambda t: "デッキの上に置き" in t or "デッキの上に置く" in t,
        "move_cards",
        lambda t, a: a.update(
            {"destination": "deck_top", "placement_order": "any_order"}
            if "好きな順番で" in t
            else {"destination": "deck_top"}
        ),
    )
    R(
        lambda t: ("エール" in t and ("枚数" in t or "数" in t)),
        "modify_yell_count",
        None,
    )
    R(
        "得る",
        "gain_ability",
        lambda t, a: a.update(
            {
                "ability_gain": t.replace("を失う", "")
                .replace("を得る", "")
                .replace("をえる", "")
                .replace("「", "")
                .replace("」", "")
                .strip()
            }
        )
        if a.get("ability_gain") is None
        else None,
    )
    R(
        lambda t: ("セット" in t or "設定" in t) and "コスト" not in t,
        "set_card_identity",
        None,
    )
    R("必要ハートを選ぶ", "choose_required_hearts", None)
    R(
        "好きな順番で",
        "move_cards",
        lambda t, a: a.update({"placement_order": "any_order"}),
    )
    R(
        lambda t: "必要ハートを確認する時" in t
        and "ALLブレード" in t
        and "任意の色のハートとして扱う" in t,
        "all_blade_timing",
        lambda t, a: a.update(
            {"timing": "check_required_hearts", "treat_as": "any_heart_color"}
        ),
    )
    R(
        lambda t: "すべての領域にあるこのカードは" in t and "として扱う" in t,
        "set_card_identity",
        lambda t, a: a.update(
            {"identities": re.findall(r"『([^』]+)』", t) or None, "all_regions": True}
        ),
    )

    # Run dispatch
    action["action"] = "custom"
    for cond, act, setter in _R:
        try:
            if callable(cond):
                try:
                    match = cond(text, action)
                except TypeError:
                    match = cond(text)
            else:
                match = cond in text
        except:
            match = False
        if match:
            action["action"] = act
            if setter:
                try:
                    setter(text, action)
                except:
                    pass
            break

    _fill_defaults(action, text)
    return action


def _extract_basic_cost_fields(cost, text):
    """Extract common fields for cost dict (source, dest, count, card_type, etc.)."""
    # Source
    if "手札を" in text or "手札の" in text:
        cost["source"] = "hand"
        cost["zone"] = "hand"  # Add zone for choice creation
    src = extract_source(text)
    if src and "source" not in cost:
        cost["source"] = src
        # Set zone based on source if not already set
        if "zone" not in cost:
            cost["zone"] = src
    # Destination
    dst = extract_destination(text)
    if dst:
        cost["destination"] = dst
    if "エネルギーデッキに置く" in text:
        cost["destination"] = "energy_deck"
    # Infer destination from source if missing
    if "source" in cost and "destination" not in cost:
        if cost["source"] == "hand" and (
            "控え室に置く" in text or "控え室に置いて" in text
        ):
            cost["destination"] = "discard"
        elif cost["source"] == "discard" and "手札に加える" in text:
            cost["destination"] = "hand"
    # State change
    sc = extract_state_change(text)
    if sc:
        cost["state_change"] = sc
    # Count, type, target
    cnt = extract_count(text)
    if cnt:
        cost["count"] = cnt
    ct = extract_card_type(text)
    if ct:
        cost["card_type"] = ct
    tgt = extract_target(text)
    if tgt:
        cost["target"] = tgt
    # Groups, names, flags
    gns = extract_group_names(text)
    if gns:
        cost["group_names"] = gns
    if extract_optional(text):
        cost["optional"] = True
    if "シャッフルする" in text or "シャッフルして" in text:
        cost["shuffle"] = True
    if "移動している" in text:
        cost["movement_state"] = "has_moved"
    # Baton touch
    if "バトンタッチ" in text:
        qm = re.search(r"「([^」]+)」からバトンタッチ", text)
        if qm:
            cost["baton_touch_source"] = qm.group(1)
        gm = re.search(r"『([^』]+)』からバトンタッチ", text)
        if gm:
            cost["baton_touch_group"] = gm.group(1)
    # Cost limit
    cl = extract_cost_limit(text)
    if cl:
        cost["cost_limit"] = cl
        for kw, op in [("以下", "<="), ("以上", ">="), ("未満", "<"), ("超", ">")]:
            if kw in text:
                cost["cost_limit_operator"] = op
                break
    # Exclude self / self cost
    if "このメンバー以外" in text or "ほかのメンバー" in text:
        cost["exclude_self"] = True
    # Same unit name
    if "同じユニット名" in text:
        cost["same_unit_name"] = True
    if re.search(r"「.+」以外", text):
        cost["exclude_self"] = True
    if (
        "このメンバー" in text
        and "このメンバー以外" not in text
        and "ほかのメンバー" not in text
    ):
        if re.search(r"このメンバー[をが]", text):
            cost["self_cost"] = True
    # Card names from 「」 — detect exclusion patterns (「name」以外)
    name_matches = re.findall(r"「([^」]+)」", text)
    include_chars = []
    exclude_chars = []
    for name in name_matches:
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


def parse_cost(text: str) -> Dict[str, Any]:
    """Parse a cost text."""
    cost = {"text": text}

    # Extract basic fields first for all cost types
    _extract_basic_cost_fields(cost, text)

    # Energy cost: count energy icons at start + distinct action (more specific)
    import re

    # Only match if text starts with energy icons, not contains them anywhere
    if text.strip().startswith("{{icon_energy.png|E}}"):
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
        # Always set energy fields for energy costs (whether simple or with other text)
        energy_count = text.count("{{icon_energy.png|E}}")
        cost["type"] = "pay_energy"
        cost["energy"] = energy_count
        cost["zone"] = "energy_zone"
        cost["count"] = energy_count
        if "もよい" in text or "てもよい" in text:
            cost["optional"] = True
        return cost

    # Sequential cost (～し、～ or ～て、～)
    if "、" in text:
        parts = text.split("、")
        first_ends_with = parts[0].strip()[-1] if parts[0].strip() else ""
        if len(parts) >= 2 and (
            first_ends_with in ("し", "て")
            or parts[0].strip().endswith("し")
            or parts[0].strip().endswith("て")
        ):
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
            # Propagate optional to the outer sequential if any sub-cost is optional
            if any(cp.get("optional") for cp in cost_parts):
                result["optional"] = True
                # Also make each sub-cost optional so they don't create their own choices
                for cp in cost_parts:
                    cp["optional"] = True
            return result

    # Reveal cost (公開する/公開し)
    if "公開する" in text or "公開し" in text:
        cost["type"] = "reveal"
        if "手札" in text:
            cost["source"] = "hand"
        cm = re.search(REGEX_COUNT_CARDS, text)
        if cm:
            cost["count"] = int(cm.group(1))
        ct = extract_card_type(text)
        if ct:
            cost["card_type"] = ct
        gns = extract_group_names(text)
        if gns:
            cost["group_names"] = gns
        return cost

    # Choice cost (～か、～)
    if "か、" in text:
        parts = text.split("か、", SPLIT_LIMIT)
        if len(parts) == 2:
            return {
                "text": text,
                "type": "choice_condition",
                "options": [parse_cost(parts[0].strip()), parse_cost(parts[1].strip())],
            }

    # Deck bottom placement (early return to avoid custom fallback)
    deck_bottom_kw = (
        "デッキの一番下に置く",
        "デッキの一番下に置いて",
        "デッキの下に置く",
        "デッキの下に置いて",
        "山札の下に置く",
        "山札の下に置いて",
    )
    if any(kw in text for kw in deck_bottom_kw):
        cost["destination"] = "deck_bottom"
        cost["type"] = "move_cards"
        # Extract common fields that were previously extracted before this check
        src = extract_source(text)
        if src:
            cost["source"] = src
    if "シャッフルする" in text or "シャッフルして" in text or "シャッフルし" in text:
        cost["shuffle"] = True
    names = re.findall(r"「([^」]+)」", text)
    include_chars = []
    exclude_chars = []
    for name in names:
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
    if "もよい" in text or "てもよい" in text:
        cost["optional"] = True
    gns = extract_group_names(text)
    if gns:
        cost["group_names"] = gns
    cnt = extract_count(text)
    if cnt:
        cost["count"] = cnt
    ct = extract_card_type(text)
    if ct:
        cost["card_type"] = ct
    tgt = extract_target(text)
    if tgt:
        cost["target"] = tgt
    if "好きな順番で" in text:
        cost["placement_order"] = "any_order"
    # If cost not yet typed, classify it now
    if "type" not in cost:
        if cost.get("source") and cost.get("destination"):
            cost["type"] = "move_cards"
        elif (
            "ウェイトにする" in text
            or "ウェイト状態で置く" in text
            or "ウェイト状態で登場させる" in text
            or "アクティブにする" in text
        ):
            cost["type"] = "change_state"
        elif cost.get("state_change"):
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


# ============== EFFECT HANDLER CASCADE ==============
#
# Priority order is CRITICAL — each handler checks a text pattern and returns
# the parsed effect dict if it matches. The first match wins.
#
# Key ordering constraints:
#   - Per-unit (につき) must be first because it restructures the entire text
#     into a condition+action pattern that would confuse other handlers.
#   - Cost modification patterns must come before plain per-unit matching
#     since they also contain "につき" but with different semantics.
#   - "これにより～の場合" must precede "その中から" because the former's
#     pattern would otherwise be consumed by the latter's regex.
#   - Conditional sequential (そうした場合) must precede implicit sequential
#     (comma-separated) because "そうした場合" contains commas but has
#     special structure.
#   - "さらに" must precede other sequential patterns to correctly handle
#     multi-level "さらに" expansions.
#   - Choice marker (以下から1つを選ぶ) must precede implicit sequential
#     since choices contain bullet points, not comma-separated actions.
#   - The main conditional (場合、/とき、/なら、) must come AFTER specific
#     conditional patterns (これにより～の場合, そうした場合) that have
#     their own structure.
#
# Each _try_effect_* takes the fully prepared text (normalized, parenthetical-stripped)
# and returns a complete effect dict or None.


def _try_per_unit(text):
    """Check for per-unit scaling (Xにつき) effects."""
    excludes = (
        "各グループ名につき",
        "グループ名につき",
        "グループ名",
        "グループ名1種類につき",
    )
    if not ("につき" in text or "ごとに" in text):
        return None
    if any(e in text for e in excludes):
        return None
    if "この能力を起動するためのコストは" in text:
        return None
    if "コストは" in text and "減る" in text:
        return None

    m = re.search(r"(.+?)(につき|ごとに)", text)
    if not m:
        return None
    per_text = m.group(1).strip()
    # If per_text contains a sentence boundary (。), the structure is likely
    # "choice/action。per_unit_effect" — defer to sequential/choice handlers
    if "。" in per_text:
        return None
    result = {"text": text, "per_unit": True}

    # Extract condition from per_text if present
    # Pattern: "条件場合、per_unit_reference" e.g.
    # "自分のセンターエリアに『μ's』のメンバーがいる場合、そのメンバーが持つheart03 2つ"
    cond_part, remaining = split_condition_action(per_text)
    if cond_part and remaining:
        parsed_cond = parse_condition(cond_part)
        if parsed_cond and parsed_cond.get("type") != "custom":
            result["condition"] = parsed_cond
            per_text = remaining  # Use remaining for per-unit extraction
    # Also check for とき、/時、pattern not inside ライブ終了時まで
    if "condition" not in result:
        for mark in ("とき、", "時、"):
            t_pos = per_text.find(mark)
            if t_pos > 0:
                before = per_text[: t_pos + 2]
                if "ライブ終了時まで" not in before:
                    cond_text = per_text[: t_pos + 2].rstrip("、").strip()
                    remaining = per_text[t_pos + len(mark) :].strip()
                    if cond_text and remaining:
                        cond = parse_condition(cond_text)
                        if cond and cond.get("type") != "custom":
                            result["condition"] = cond
                        per_text = remaining
                    break
    # Also check for 時、pattern (any form, kanji) at a position not inside ライブ終了時まで
    if "condition" not in result:
        t_pos = per_text.find("時、")
        if t_pos > 0 and "ライブ終了時まで" not in per_text[: t_pos + 2]:
            cond_text = per_text[: t_pos + 1].strip()  # Include 時
            remaining = per_text[t_pos + 2 :].strip()
            if cond_text and remaining:
                cond = parse_condition(cond_text)
                if cond and cond.get("type") != "custom":
                    result["condition"] = cond
                per_text = remaining

    # Extract duration from per_text (e.g., "ライブ終了時まで、カード1枚につき")
    for prefix, code in [
        ("ライブ終了時まで", "live_end"),
        ("このターンの間", "turn_end"),
        ("ターン終了時まで", "turn_end"),
    ]:
        if per_text.startswith(prefix):
            result["duration"] = code
            per_text = per_text[len(prefix) :].lstrip("、").strip()
            break

    pm = re.search(r"(\d+)(人|枚|つ)(につき|ごとに)", text)
    if pm:
        result["per_unit_count"] = int(pm.group(1))
        result["per_unit_type"] = pm.group(2)
        if "ライブ中のカード" in text:
            import sys

            print(
                f"[DEBUG per_unit] OVERRIDE: 枚 → live_card_zone for text={text[:80]}",
                file=sys.stderr,
            )
            result["per_unit_type"] = "live_card_zone"
    else:
        for kw, t in [
            ("メンバー", "member"),
            ("人", "member"),
            ("カード", "card"),
            ("枚", "card"),
            ("ブレード", "blade"),
            ("ハート", "heart"),
            ("スコア", "score"),
            ("コスト", "cost"),
        ]:
            if kw in per_text:
                result["per_unit_type"] = t
                break

    # "これにより控え室に置いた" = placed in waitroom by this cost → count in discard
    if "控え室に置いた" in per_text:
        result["per_unit_type"] = "discard"

    gm = re.search(r"『([^』]+)』", per_text)
    if gm:
        result["group_names"] = [gm.group(1)]
    if "名前の異なる" in per_text or "カード名の異なる" in per_text:
        result["distinct"] = "card_name"

    # Extract cost_limit from per-text (e.g., "コスト4以上")
    cl = extract_cost_limit(per_text)
    if cl:
        result["cost_limit"] = cl
        if "以下" in per_text:
            result["cost_limit_operator"] = "<="
        elif "以上" in per_text:
            result["cost_limit_operator"] = ">="
        elif "未満" in per_text:
            result["cost_limit_operator"] = "<"
        elif "超" in per_text:
            result["cost_limit_operator"] = ">"

    if "このターン中に登場" in per_text and "エリアを移動した" in per_text:
        result["timing_condition"] = "appeared_or_moved_this_turn"
    elif "このターン中に登場" in per_text:
        result["timing_condition"] = "appeared_this_turn"
    elif "エリアを移動した" in per_text:
        result["timing_condition"] = "moved_areas_this_turn"

    if "ウェイト状態" in per_text:
        result["state"] = "wait"
    elif "アクティブ状態" in per_text:
        result["state"] = "active"

    # Extract target from per_text
    tgt = extract_target(per_text)
    if tgt:
        result["target"] = tgt

    for kw, loc in [
        ("成功ライブカード置き場にある", "success_live_zone"),
        ("ステージにいる", "stage"),
        ("控え室にある", "discard"),
        ("ライブカード置き場にある", "live_card_zone"),
        ("手札にある", "hand"),
        ("デッキにある", "deck"),
    ]:
        if kw in per_text:
            result["location"] = loc
            break

    action_text = text.split("につき", 1)[1].strip().lstrip("、")

    # Sequential pattern in action (Aし、B)
    if "、" in action_text and "し" in action_text:
        parts = [p.strip().rstrip("、") for p in action_text.split("、")]
        if len(parts) >= 2 and "し" in parts[0]:
            actions = []
            for part in parts:
                pa = parse_action(part)
                if pa.get("action") != "custom":
                    _propagate(result, pa)
                    actions.append(pa)
            if len(actions) >= 2:
                return {"text": text, "action": "sequential", "actions": actions}

    action = parse_action(action_text)
    _propagate(result, action)

    # Detect cost reduction per unit patterns (コストが～につき～少なくなる/減る)
    if (
        action.get("action") == "custom"
        and result.get("location") == "hand"
        and result.get("per_unit_type") in ("member", "人", "枚")
    ):
        if "少なくなる" in action_text or "減る" in action_text:
            action["action"] = "modify_cost"
            action["operation"] = "subtract"

    # Sequential after per-unit (その後)
    if "その後" in action_text:
        parts = action_text.split("その後", 1)
        if len(parts) == 2:
            fa_text = parts[0].strip()
            # When fa_text contains "。" + another per-unit（につき),
            # split on "。" to handle compound sub-effects (e.g. reveal + per-unit score)
            if "。" in fa_text:
                sub_texts = [t.strip() for t in fa_text.split("。") if t.strip()]
                sub_actions = []
                for st in sub_texts:
                    spa = parse_effect(st)
                    if spa.get("action") != "custom" or spa.get("actions"):
                        _propagate_if_missing(result, spa)
                        sub_actions.append(spa)
                if len(sub_actions) >= 2:
                    fa = {"action": "sequential", "actions": sub_actions}
                    _propagate_if_missing(result, fa)
                else:
                    fa = parse_action(fa_text)
                    _propagate(result, fa)
            else:
                fa = parse_action(fa_text)
                _propagate(result, fa)
            sa = parse_action(parts[1].strip())
            return {"text": text, "action": "sequential", "actions": [fa, sa]}

    action["text"] = text
    return action


def _propagate(src, dst):
    """Copy common per-unit fields from src to dst (overwrites existing)."""
    for k in (
        "per_unit",
        "per_unit_count",
        "per_unit_type",
        "group_names",
        "distinct",
        "timing_condition",
        "state",
        "location",
        "cost_limit",
        "cost_limit_operator",
        "duration",
        "condition",
        "target",
    ):
        if k in src:
            dst[k] = src[k]


def _propagate_if_missing(src, dst):
    """Copy common per-unit fields from src to dst only if not already present."""
    for k in (
        "per_unit",
        "per_unit_count",
        "per_unit_type",
        "group_names",
        "distinct",
        "timing_condition",
        "state",
        "location",
        "cost_limit",
        "cost_limit_operator",
        "duration",
        "condition",
        "target",
    ):
        if k in src and k not in dst:
            dst[k] = src[k]


def _try_conditional_alternative(text):
    """代わりに — conditional alternative effects."""
    if ALTERNATIVE_MARKER not in text:
        return None
    # If choice marker is present before the alternative marker,
    # the choice handler should handle this text instead
    if CHOICE_MARKER in text and text.find(CHOICE_MARKER) < text.find(
        ALTERNATIVE_MARKER
    ):
        return None
    parts = text.split(ALTERNATIVE_MARKER, 1)
    if len(parts) != 2:
        return None
    pa = parse_action(parts[0].strip())
    aa = parse_action(parts[1].strip())
    return {
        "text": text,
        "action": "conditional_alternative",
        "primary_effect": pa,
        "alternative_effect": aa,
    }


def _try_character_specific(text):
    """「X」N人はYを、「Z」N人はWを得る — character-specific effects."""
    m = re.search(r"「([^」]+)」\d+人は(.+?)を、「([^」]+)」\d+人は(.+?)を得る", text)
    if not m:
        return None
    effects = []
    for part in text.split("、"):
        pm = re.search(r"「([^」]+)」(\d+)人は(.+?)を得る", part) or re.search(
            r"「([^」]+)」(\d+)人は(.+?)を", part
        )
        if pm:
            effects.append(
                {
                    "character": pm.group(1),
                    "count": int(pm.group(2)),
                    "resources": pm.group(3),
                }
            )
    if effects:
        # Expand character effects into individual gain_resource actions
        actions = []
        for eff in effects:
            resources_text = eff["resources"]
            # Detect heart and blade resources from icons
            heart_m = re.search(r"heart_(\d+)", resources_text)
            blade_count = resources_text.count("icon_blade.png")
            heart_color = f"heart{heart_m.group(1)}" if heart_m else None
            if blade_count > 0:
                actions.append(
                    {
                        "action": "gain_resource",
                        "resource": "blade",
                        "count": blade_count,
                        "characters": [eff["character"]],
                        "target": "self",
                        "card_type": "member_card",
                    }
                )
            if heart_color:
                actions.append(
                    {
                        "action": "gain_resource",
                        "resource": "heart",
                        "heart_color": heart_color,
                        "count": 1,
                        "characters": [eff["character"]],
                        "target": "self",
                        "card_type": "member_card",
                    }
                )
        if len(actions) == 1:
            result = actions[0]
        else:
            result = {"action": "sequential", "actions": actions}
        result["text"] = text
        result["character_effects"] = effects
        return result
    return None


def _try_activation_suffix(text):
    """この能力は～場合のみ起動できる/発動する — activation condition at text end."""
    m = re.search(r"この能力は、(.+?)場合のみ(?:起動できる|発動する)", text)
    if not m:
        return None
    suffix = m.group(0).split("場合のみ")[-1]
    cond_text = "この能力は、" + m.group(1).strip() + "場合のみ" + suffix
    action_text = text.replace(cond_text, "").strip().rstrip("。")
    action = parse_action(action_text)
    result = {"text": text}
    result.update(action)
    cond_parsed = parse_condition(m.group(1).strip() + "場合")
    if cond_parsed.get("type") != "custom":
        result["activation_condition_parsed"] = cond_parsed
    return result


def _make_cost_mod_action(text_part, operation="decrease"):
    """Build a modify_cost action dict from text."""
    a = parse_action(text_part)
    a["action"] = "modify_cost"
    a["operation"] = operation
    ic = text_part.count("{{icon_energy.png|E}}")
    if ic > 0:
        a["count"] = ic
    if "グループ名" in text_part and "につき" in text_part:
        a["per_unit"] = True
        a["per_unit_type"] = "group_name"
        cm = re.search(r"(\d+)種類", text_part)
        if cm:
            a["per_unit_count"] = int(cm.group(1))
    return a


def _try_cost_modification(text):
    """コストは～につき～減る — cost modification with per-unit scaling.
    Handles: "action。コストは～につき～減る" (sequential) and
    "この能力を起動するためのコストは～につき～減る" (flat modify_cost)."""
    energy_count = text.count("{{icon_energy.png|E}}")
    cost_prefixes = ("コストは", "この能力を起動するためのコストは")
    if not any(p in text for p in cost_prefixes):
        return None
    if "につき" not in text or "減る" not in text:
        return None
    if "。" in text:
        parts = text.split("。", 1)
        if len(parts) == 2:
            first, second = parts[0].strip(), parts[1].strip()
            if any(p in second for p in cost_prefixes):
                op = "subtract"
                # Return ONLY the modify_cost action — the cost reduction is a passive modifier,
                # not a sibling action that runs alongside the main effect.
                # The main effect (first part) will be handled by subsequent handlers or parse_action fallback.
                return _make_cost_mod_action(second, op)
    return {
        "action": "modify_cost",
        "operation": "subtract",
        "text": text,
        "count": energy_count,
    }


def _try_duration_prefix(text):
    """ライブ終了時まで / ターン終了時まで / そのターンの間 — strip prefix and mark duration.
    Only matches if the pattern is at the very start of the text,
    not embedded within sub-effects/options."""
    rest, code = _strip_duration_prefix(text)
    if code:
        return {"text": rest, "duration": code, "_rest": rest}
    return None


def _try_answer_choice(text):
    """回答が — answer-based choice effects.
    Structure: 相手に何が好き？と聞く。回答がXかYの場合、action。回答がZの場合、action。回答がそれ以外の場合、action"""
    if "回答が" not in text:
        return None
    result = {"text": text, "action": "choice", "choice_type": "answer_based"}
    qm = re.search(r"(.+?)(?=回答が)", text, re.DOTALL)
    if qm:
        qt = re.sub(r"[\n。]+$", "", qm.group(1).strip())
        if qt:
            result["question"] = qt
    options = []
    segments = re.split(r"(?=回答が)", text)
    for seg in segments:
        seg = seg.strip()
        if not seg.startswith("回答が"):
            continue
        idx = seg.find("場合、")
        if idx == -1:
            continue
        answers_text = seg[len("回答が") : idx].strip()
        action_text = seg[idx + len("場合、") :].strip().rstrip("。")
        if not answers_text or not action_text:
            continue
        answers = [
            a.strip().rstrip("の") for a in answers_text.split("か") if a.strip()
        ]
        pa = parse_action(action_text)
        pa["answers"] = answers
        options.append(pa)
    if options:
        result["options"] = options
        result["choice_maker"] = "opponent"
        return result
    return None


def _try_each_time(text):
    """たび — each-time triggers."""
    if EACH_TIME_MARKER not in text:
        return None
    tm = re.search(r"([^たび]+)たび", text)
    if not tm:
        return None
    trigger_text = tm.group(1).strip()
    rest = text[tm.end() :].strip().lstrip("、，")
    sub = parse_effect(rest)
    sub["trigger_type"] = "each_time"
    sub["text"] = text
    # Parse the trigger condition text
    if "か、" in trigger_text:
        or_cond = _try_or(trigger_text)
        if or_cond:
            sub["trigger_condition"] = or_cond
    else:
        trigger_cond = parse_condition(trigger_text)
        if trigger_cond and trigger_cond.get("type") != "custom":
            sub["trigger_condition"] = trigger_cond
    return sub


def _try_opponent_action(text):
    """相手は — opponent action patterns (with or without comma).

    When there is follow-up text (e.g. "これにより...手札に加える"), returns a
    sequential so the opponent's choice and the follow-up are naturally chained
    by the engine's sequence handler — no need for complex merged-dict hacks.
    """
    if not text.startswith("相手は"):
        return None
    om = re.match(r"相手は[、]?(.+?)。", text)
    if not om:
        return None
    oa_text = om.group(0)
    rest = text[len(oa_text) :].strip()
    oa = parse_action(om.group(1).strip())
    opp_action = {
        "text": oa_text,
        "action": "opponent_action",
        "action_by": "opponent",
        "opponent_action": oa,
    }
    if rest:
        re_eff = parse_effect(rest)
        return {"text": text, "action": "sequential", "actions": [opp_action, re_eff]}
    return opp_action


def _try_choose_self_opponent(text):
    """自分か相手を選ぶ。— choose self or opponent."""
    if not text.startswith("自分か相手を選ぶ。"):
        return None
    rest = text[len("自分か相手を選ぶ。") :].strip()
    result = {"text": text, "action": "sequential"}
    if rest:
        re_eff = parse_effect(rest)
        if "target" not in re_eff or not re_eff["target"]:
            re_eff["target"] = "self"
        result["actions"] = [re_eff]
        if "そうした場合" in rest:
            result["conditional"] = True
    else:
        result["actions"] = []
    return result


def _try_opponent_after_conditional(text):
    """、相手は、 — opponent action after conditional marker."""
    if "、相手は" not in text:
        return None
    parts = text.split("、相手は、", SPLIT_LIMIT)
    if len(parts) != 2:
        return None
    first = parts[0].strip()
    opp = "相手は、" + parts[1]
    om = re.match(r"相手は、(.+?)。", opp)
    if not om:
        return None
    fa = parse_action(first.replace("そうした場合、", "").strip())
    rest = opp[len(om.group(0)) :].strip()
    oa = parse_action(om.group(1).strip())
    result = {
        "text": text,
        "action": "sequential",
        "actions": [fa, oa],
        "conditional": True,
    }
    if rest:
        result["actions"].append(parse_action(rest))
    return result if result["actions"] else None


def _try_kore_niyori_case(text):
    """これにより～の場合、～。～以外の場合、～ — conditional alternative with card type.
    MUST precede その中から since the regex for "これにより～の場合" would
    be consumed by the "その中から" text split logic."""
    if "これにより" not in text or "の場合" not in text or "以外の場合" not in text:
        return None
    parts = text.split("以外の場合", 1)
    if len(parts) != 2:
        return None
    first, second = parts[0].strip(), parts[1].strip()
    if "場合、" not in first:
        return None
    cp, ap = first.split("場合、", 1)
    cond_text = "これにより" + cp.replace("これにより", "").strip() + "場合"
    action_text = re.sub(r"『.+』のカード$", "", ap.strip()).strip()
    fe = parse_effect(action_text)
    se = parse_effect(second.lstrip("、。").strip())
    return {
        "text": text,
        "action": "conditional_alternative",
        "condition": parse_condition(cond_text),
        "primary_effect": fe,
        "alternative_effect": se,
    }


def _build_reveal_add_discard(fp, sa_text, select_text):
    """Build select_cards for 'reveal → add → discard' pattern."""
    result = {
        "action": "select_cards",
        "destination": "hand",
        "discard_remaining": True,
        "reveal": True,
    }
    cnt = extract_count(select_text)
    if cnt:
        result["count"] = cnt
    ct = extract_card_type(select_text)
    if ct:
        result["card_type"] = ct
    hc = list(
        dict.fromkeys(
            f"heart{m.zfill(2)}" for m in re.findall(r"heart_(\d+)", select_text)
        )
    )
    if hc:
        result["heart_colors"] = hc
    if extract_max(select_text):
        result["max"] = True
    if extract_optional(select_text):
        result["optional"] = True
    gns = extract_group_names(select_text)
    if gns:
        result["group_names"] = gns
    cl = extract_cost_limit(select_text)
    if cl:
        result["cost_limit"] = cl
    op = extract_operator(select_text)
    if op:
        result["cost_limit_operator"] = op
    return result


def _enrich_from_text(d, text):
    """Add common fields (count, max, card_type, heart_colors, optional, group_names, cost_limit) from text."""
    c = extract_count(text)
    if c:
        d["count"] = c
    if extract_max(text):
        d["max"] = True
    ct = extract_card_type(text)
    if ct:
        d["card_type"] = ct
    hc = list(
        dict.fromkeys(f"heart{m.zfill(2)}" for m in re.findall(r"heart_(\d+)", text))
    )
    if hc:
        d["heart_colors"] = hc
    if extract_optional(text):
        d["optional"] = True
    gns = extract_group_names(text)
    if gns:
        d["group_names"] = gns
    cl = extract_cost_limit(text)
    if cl:
        d["cost_limit"] = cl
    op = extract_operator(text)
    if op:
        d["cost_limit_operator"] = op


def _build_look_select_actions(select_text):
    """Build the select_action for その中から patterns."""
    result = {"action": "select_cards", "discard_remaining": True}

    # Pattern: reveal → add → discard
    if "手札に加え" in select_text and "残りを控え室に置く" in select_text:
        parts = re.split(r"[、。]", select_text)
        if len(parts) >= 2:
            fp = parts[0].strip()
            if "公開して" in fp:
                act = _build_reveal_add_discard(fp, parts[1].strip(), select_text)
                if act:
                    return act
            if "公開して" not in fp:
                result["destination"] = "hand"
                cnt = extract_count(select_text)
                if cnt:
                    result["count"] = cnt
                hc = list(
                    dict.fromkeys(
                        f"heart{m.zfill(2)}"
                        for m in re.findall(r"heart_(\d+)", select_text)
                    )
                )
                if hc:
                    result["heart_colors"] = hc
                ct = extract_card_type(select_text)
                if ct:
                    result["card_type"] = ct
                if extract_optional(select_text):
                    result["optional"] = True
                return result

    # Pattern: any number → deck_top → discard remaining
    if (
        "好きな枚数を好きな順番でデッキの上に置き" in select_text
        and "残りを控え室に置く" in select_text
    ):
        result["destination"] = "deck_top"
        result["placement_order"] = "any_order"
        result["any_number"] = True
        result["reveal"] = False
        return result

    # Default: detect destination from text
    result["reveal"] = False
    if "手札に加える" in select_text or "手札に加え" in select_text:
        result["destination"] = "hand"
    elif "控え室に置く" in select_text:
        result["destination"] = "discard"
    elif "デッキの上に置く" in select_text or "デッキの上に" in select_text:
        result["destination"] = "deck_top"

    # Propagate selection criteria
    _enrich_from_text(result, select_text)

    # Handle heart-color filter in default case
    if result.get("destination") is None and (
        "{{heart_" in select_text or "ハートに" in select_text
    ):
        result["destination"] = "hand"

    return result


def _try_look_and_select(text):
    """その中から — look_at + select + action."""
    if "その中から" not in text:
        return None
    result = {"text": text, "action": "look_and_select"}
    lm = re.search(r"(.+?)その中から", text)
    if lm:
        look_text = lm.group(1).strip()
        # Strip condition prefix from look action text to prevent condition
        # keywords (e.g. 成功ライブカード置き場) from bleeding into source extraction
        ct, at = split_condition_action(look_text)
        if at:
            look_text = at
        result["look_action"] = parse_action(look_text)
    am = re.search(r"その中から(.+)", text)
    if am:
        result["select_action"] = _build_look_select_actions(am.group(1).strip())
    return result


def _try_reveal_until_chosen_card(text):
    """ライブカードか...メンバーカードのどちらか1つを選ぶ — type choice + reveal until match."""
    # Pattern: "ライブカードか" + optional "コストN以上の" + "メンバーカードのどちらか1つを選ぶ"
    if (
        "ライブカードか" in text
        and "メンバーカードのどちらか" in text
        and "選んだカードが公開されるまで" in text
    ):
        cost_limit = None
        m = re.search(r"コスト(\d+)以上", text)
        if m:
            cost_limit = int(m.group(1))
        action = {
            "text": text,
            "action": "sequential",
            "actions": [
                {
                    "action": "select",
                    "or_card_types": ["live_card", "member_card"],
                    "count": 1,
                    "all": False,
                },
                {
                    "action": "reveal",
                    "source": "deck_top",
                    "count": 1,
                    "multiple_targets": True,
                    "all": False,
                },
                {
                    "action": "move_cards",
                    "source": "looked_at",
                    "destination": "hand",
                    "count": 1,
                    "all": False,
                },
                {
                    "action": "move_cards",
                    "source": "looked_at_remaining",
                    "destination": "discard",
                    "all": True,
                },
            ],
        }
        if cost_limit is not None:
            action["actions"][0]["cost_limit"] = cost_limit
            action["actions"][0]["cost_limit_operator"] = ">="
        return action
    return None


def _try_reveal_until_live(text):
    """ライブカードが公開されるまで — reveal deck until live card found."""
    if "ライブカードが公開されるまで" not in text:
        return None
    return {
        "text": text,
        "action": "sequential",
        "actions": [
            {
                "action": "reveal_until_live_card",
                "source": "deck_top",
                "target": "self",
            },
            {
                "action": "move_cards",
                "source": "looked_at",
                "destination": "hand",
                "card_type": "live_card",
                "count": 1,
                "text": "そのライブカードを手札に加え",
            },
            {
                "action": "move_cards",
                "source": "looked_at_remaining",
                "destination": "discard",
                "all": True,
                "text": "これにより公開されたほかのすべてのカードを控え室に置く",
            },
        ],
    }


def _try_furthermore(text):
    """さらに — sequential conditional effects with "furthermore"."""
    if "さらに" not in text:
        return None
    parts = text.split("。")
    if len(parts) < 2:
        return None
    if not any("さらに" in p for p in parts[1:]):
        return None
    actions = []
    for p in parts:
        pt = p.strip()
        if not pt:
            continue
        if "さらに" in pt:
            pt = pt.replace("さらに", "", 1).strip()
        actions.append(parse_effect(pt))
    if actions and any(a.get("action") or a.get("actions") for a in actions):
        return {"text": text, "action": "sequential", "actions": actions}
    return None


def _try_sequential_duration(text):
    """その後、～かぎり、 — sequential with duration condition."""
    if "その後、" not in text or "かぎり、" not in text:
        return None
    parts = text.split("その後、", SPLIT_LIMIT)
    if len(parts) != 2:
        return None
    fa = parse_action(parts[0].strip())
    second = parts[1].strip()
    if "かぎり、" not in second:
        return None
    cp = second.split("かぎり、", 1)
    cond = parse_condition(cp[0].strip())
    sa = parse_action(cp[1].strip())
    sa["condition"] = cond
    sa["duration"] = "unless"
    return {"text": text, "action": "sequential", "actions": [fa, sa]}


def _try_compound_select(text):
    """Aのうちのメンバー1人と、これにより選んだメンバー以外のBのメンバー1人は — compound selection."""
    if "のうちの" not in text or "これにより選んだメンバー以外" not in text:
        return None
    # Extract character names from the first group 「」
    char_names = re.findall(r"「([^」]+)」", text)
    if len(char_names) < 2:
        return None
    # Extract second group from 『』
    group_names = re.findall(r"『([^』]+)』", text)
    # Determine duration
    dur = "live_end" if "ライブ終了まで" in text else None
    # Build actions
    actions = []
    # First select: from named characters
    actions.append(
        {
            "action": "select",
            "count": 1,
            "card_type": "member_card",
            "characters": char_names,
        }
    )
    # Second select: from group excluding first selection
    sel2 = {
        "action": "select",
        "count": 1,
        "card_type": "member_card",
        "exclude_selected": True,
    }
    if group_names:
        sel2["group_names"] = group_names
    actions.append(sel2)
    # Check if the action is gain_resource
    action_text = text.split("は、")[-1].strip() if "は、" in text else text
    gain = parse_action(action_text)
    if gain.get("action") != "custom":
        actions.append(gain)
    result = {"text": text, "action": "sequential", "actions": actions}
    if dur:
        result["duration"] = dur
    return result


def _try_implicit_sequential(text):
    """、— comma-separated actions (implicit sequential).
    Also handles 。(period-separated) patterns.
    Checked AFTER そうした場合 and conditional patterns to prevent
    mis-parsing actions that happen to contain commas."""
    if "、" not in text and "。" not in text:
        return None
    if any(m in text for m in CONDITION_MARKERS):
        return None
    if CHOICE_MARKER in text:
        return None
    # Prefer 。as separator when present (sentence boundaries)
    if "。" in text:
        # Filter out fully parenthetical segments before splitting
        # to prevent notes like （対戦相手のカードの効果でも発動する） from becoming do_nothing
        clean_for_split = re.sub(r"（[^）]*）", "", text)
        clean_for_split = re.sub(r"\([^)]*\)", "", clean_for_split)
        # Protect 「」 content from internal splitting (ability text with periods)
        clean_for_split = re.sub(
            r"「[^」]*」", lambda m: m.group(0).replace("。", "\x00"), clean_for_split
        )
        parts = [p.strip() for p in clean_for_split.split("。") if p.strip()]
        # Restore protected periods
        parts = [p.replace("\x00", "。") for p in parts]
        # Also filter duration prefix fragments that are just "ライブ終了時まで、"
        filtered = []
        for p in parts:
            if re.match(r"^ライブ終了時まで[、，]?$", p) or re.match(
                r"^ライブ終了まで[、，]?$", p
            ):
                continue
            filtered.append(p)
        parts = filtered
    else:
        parts = [p for p in text.split("、") if p.strip()]
    if len(parts) < 2:
        return None
    actions = []
    for p in parts:
        cp = p.strip().lstrip("、")
        if cp.endswith("その後"):
            cp = cp[: -len("その後")].strip()
        elif cp.endswith("その後。"):
            cp = cp[: -len("その後。")].strip()
        a = parse_effect(cp)
        if (
            a
            and a.get("action", "custom") != "custom"
            and a.get("action") != "do_nothing"
        ):
            actions.append(a)
    if len(actions) >= 2:
        return {"text": text, "action": "sequential", "actions": actions}
    return None


def _try_conditional_sequential(text):
    """そうした場合 — conditional sequential actions."""
    if CONDITIONAL_SEQUENTIAL_MARKER not in text:
        return None
    parts = text.split(CONDITIONAL_SEQUENTIAL_MARKER, SPLIT_LIMIT)
    fp = parts[0].strip()
    sp = parts[1].strip()

    # Check for condition in first part
    fc, fat = split_condition_action(fp)
    if fc and fat:
        fa = parse_action(fat)
        fa["text"] = fat
        cond = parse_condition(fc)
    else:
        fa = parse_action(fp)
        cond = None

    # Process second part — use parse_effect to handle sequential sub-actions
    clean = sp.replace(CONDITIONAL_SEQUENTIAL_MARKER, "").strip().lstrip("、")
    sa = parse_effect(clean)
    # selected_cards reference from select action
    if fa.get("action") == "select":
        if isinstance(sa, dict) and "actions" in sa:
            for sub in sa.get("actions", []):
                if sub.get("action") == "move_cards":
                    sub["source"] = "selected_cards"
                if sub.get("action_by") == "opponent":
                    oa = sub.setdefault("opponent_action", {})
                    if "source" not in oa:
                        oa["source"] = "selected_cards"
        elif isinstance(sa, dict):
            # For merged opponent_action format, set source on the opponent_action
            if sa.get("action_by") == "opponent":
                oa = sa.setdefault("opponent_action", {})
                if "source" not in oa:
                    oa["source"] = "selected_cards"
            else:
                sa["source"] = "selected_cards"

    result = {
        "text": text,
        "action": "sequential",
        "actions": [fa, sa],
        "conditional": True,
    }
    if cond:
        result["condition"] = cond
    return result


def _try_sequential(text):
    """その後、 — sequential marker. Must be checked BEFORE _try_conditional
    so that 条件→行動。その後、条件→行動 patterns are split correctly
    (moved from position 17 to position 12 in _EFFECT_HANDLERS)."""
    if SEQUENTIAL_MARKER not in text:
        return None
    parts = text.split(SEQUENTIAL_MARKER, 1)
    fa = parse_effect(parts[0].strip())
    sp = parts[1].strip().lstrip("、")
    if sp.startswith("その後"):
        sp = sp[len("その後") :].strip()
    sa = parse_effect(sp)
    return {"text": text, "action": "sequential", "actions": [fa, sa]}


def _try_choice(text):
    """以下から1つを選ぶ — choice effects."""
    if CHOICE_MARKER not in text:
        return None
    parts = text.split(CHOICE_MARKER, SPLIT_LIMIT)
    if len(parts) <= 1:
        return None

    # Parse bullet options and optional condition modifier
    lines = [l.strip() for l in parts[1].strip().split("\n") if l.strip()]
    opts, cond_mod, in_opts = [], None, False
    for line in lines:
        if line.startswith("・"):
            in_opts = True
            opts.append(line[1:].strip())
        elif in_opts:
            opts[-1] += " " + line
        elif not cond_mod:
            cond_mod = line

    result = {"text": text, "action": "choice"}
    if cond_mod and cond_mod not in ("。", "."):
        result["choice_modifier"] = cond_mod
        cond = parse_condition(cond_mod)
        if cond.get("type") != "custom":
            result["choice_condition"] = cond

    options = []
    for ot in opts:
        oc, oa = split_condition_action(ot)
        po = parse_effect(oa) if oc and oa else parse_action(ot)
        if oc and oa:
            po["condition"] = parse_condition(oc)
        po["text"] = ot
        options.append(po)
    if not options:
        return None

    # Conditional alternative in choice modifier: "代わりに" → conditional_alternative
    if cond_mod and ALTERNATIVE_MARKER in cond_mod:
        alt_parts = cond_mod.split(ALTERNATIVE_MARKER, 1)
        if len(alt_parts) == 2:
            before = alt_parts[0].strip().rstrip("、。")
            after = alt_parts[1].strip().rstrip("。")
            alt_effect = {"action": "choice", "options": options}
            if "以上" in after:
                alt_effect["any_number"] = True
            else:
                ac = extract_count(after)
                if ac:
                    alt_effect["count"] = ac
            return {
                "text": text,
                "action": "conditional_alternative",
                "condition": parse_condition(before),
                "primary_effect": {"action": "choice", "count": 1, "options": options},
                "alternative_effect": alt_effect,
            }

    result["options"] = options
    return result


def _try_kore_niyori_cascade(text):
    """これにより cascading: [actions]。[actions]。これにより[cond]場合、[result]."""
    m = re.search(r"^(.*?)。これにより(.+?)場合、(.+)$", text, re.DOTALL)
    if not m:
        return None
    action_text, cond_text, result_text = (
        m.group(1).strip(),
        m.group(2).strip(),
        m.group(3).strip(),
    )
    acts = [parse_action(p) for p in action_text.split("。") if p.strip()]
    if not acts:
        return None
    cp = parse_condition(cond_text + "場合")
    rp = parse_effect(result_text)
    follow = {"condition": cp}
    follow.update(rp)
    acts.append(follow)
    return {"text": text, "action": "sequential", "actions": acts}


def _try_period_conditional(text):
    """。場合、 — period-then-conditional patterns (chainable).
    Handles "<uncond_action>。<cond1>、<action1>。<cond2>、<action2>..."
    Splits on periods and processes each condition=action pair."""
    if "。" not in text or "場合" not in text:
        return None
    # Don't handle patterns with "これにより" (complex condition markers) or
    # "この能力は" (activation condition suffixes) — those have their own handlers.
    if "これにより" in text or "この能力は" in text:
        return None
    parts = [p.strip() for p in text.split("。") if p.strip()]
    if len(parts) < 2:
        return None
    # Find where conditional segments start (first part containing '場合')
    cond_start = None
    for i, p in enumerate(parts):
        if "場合" in p:
            cond_start = i
            break
    if cond_start is None:
        return None
    # Unconditional leading action(s)
    actions = []
    for p in parts[:cond_start]:
        fa = parse_effect(p)
        if fa.get("action", "custom") != "custom":
            actions.append(fa)
    # Each conditional segment: "条件、action"
    for p in parts[cond_start:]:
        # Split on the first occurrence of 場合、
        idx = p.find("場合、")
        if idx >= 0:
            cond_part = p[: idx + 2]  # includes "場合"
            action_part = p[idx + 3 :].strip()  # after "場合、"
            # Parse the action with its condition
            full = cond_part + "、" + action_part
            ce = _try_conditional(full)
            if ce is not None:
                ca = ce.get("actions", [])
                if ca:
                    actions.extend(ca)
                else:
                    actions.append(ce)
    if len(actions) >= 2:
        return {"text": text, "action": "sequential", "actions": actions}
    return None


def _try_conditional(text):
    """場合、 / とき、 / なら、 — conditional effects (generic).
    Also handles た時、 (past tense + kanji 時) e.g., "エネルギーを選んだ時、"
    Checked LAST among conditional handlers so that specific conditional
    patterns (これにより～の場合, そうした場合) get their own structure."""
    ct, at = split_condition_action(text)
    if not ct or not at:
        # Check for 時、pattern (any form, kanji) at a position not inside ライブ終了時まで
        t_pos = text.find("時、")
        if t_pos > 0 and "ライブ終了時まで" not in text[: t_pos + 2]:
            ct = text[: t_pos + 1].strip()
            at = text[t_pos + 2 :].strip()
            cond = parse_condition(ct)
            result = {"text": text, "condition": cond}
            at = at.lstrip("、")
            at, dur = _strip_duration_prefix(at)
            at = strip_suffix_period(at)
            action = parse_effect(at)
            if dur:
                action["duration"] = dur
            result["action"] = action.get("action", "custom")
            if action.get("action") == "sequential":
                result["actions"] = action.get("actions", [])
            else:
                result.update(action)
            return result
        return None
    # If choice marker appears before the first condition marker,
    # let _try_choice handle this (the condition is part of a choice modifier)
    if CHOICE_MARKER in text:
        choice_pos = text.find(CHOICE_MARKER)
        for marker in CONDITION_MARKERS:
            cm_pos = text.find(marker)
            if cm_pos >= 0 and choice_pos < cm_pos:
                return None
    cond = parse_condition(ct)
    at = at.lstrip("、")
    at, dur = _strip_duration_prefix(at)
    at = strip_suffix_period(at)

    # Special: yell count modification
    if "エールによって公開される自分のカードの枚数が" in at:
        cm = re.search(REGEX_COUNT_CARDS, at)
        cnt = int(cm.group(1)) if cm else None
        result = {
            "text": text,
            "condition": cond,
            "action": "modify_yell_count",
            "operation": "subtract" if ("減る" in at or "減らす" in at) else "add",
        }
        if cnt:
            result["count"] = cnt
        if dur:
            result["duration"] = dur
        return result

    action = parse_effect(at)
    if dur:
        action["duration"] = dur
    result = {"text": text, "condition": cond}

    # Handle "条件Aの場合、または条件Bの場合、行動" — merge into OR condition
    if action.get("condition") and at.lstrip().startswith("または"):
        cond = {
            "type": "or_condition",
            "conditions": [cond, action.pop("condition")],
            "text": text,
        }
        result["condition"] = cond

    if action.get("action") == "sequential":
        result["action"] = "sequential"
        result["actions"] = action.get("actions", [])
        if "text" in action:
            result["text"] = action["text"]
    else:
        result.update(action)
    return result if (result.get("action") or result.get("actions")) else None


def _try_ability_activation(text):
    """能力を発動させる — ability activation effects.
    Handles both simple patterns ("...能力を発動させる") and sequential
    patterns ("select card. activate its ability")."""
    # Check for compound patterns: "select card. activate its ability"
    if "。" in text:
        # Protect 「」 content from internal splitting (ability text with periods)
        clean = re.sub(r"「[^」]*」", lambda m: m.group(0).replace("。", "\x00"), text)
        parts = [p.strip() for p in clean.split("。") if p.strip()]
        # Restore protected periods
        parts = [p.replace("\x00", "。") for p in parts]
        if len(parts) >= 2:
            actions = []
            for p in parts:
                result = _try_ability_activation(p)
                if result and result.get("action") == "activate_ability":
                    actions.append(result)
                else:
                    pa = parse_action(p)
                    if pa.get("action") != "custom":
                        actions.append(pa)
            if len(actions) >= 2:
                return {"text": text, "action": "sequential", "actions": actions}
    # Simple pattern: "...能力を発動させる" or "...能力を発動させて" (te-form)
    m = re.search(r"(.+?)能力.*?を発動させ", text)
    if not m:
        return None
    target_raw = m.group(1).strip() + "能力"
    result = {"text": text, "action": "activate_ability"}
    # Detect "これにより" pattern — references the card from the cost payment
    if "これにより" in target_raw:
        result["source_card"] = "cost_card"
    result["target"] = target_raw
    tm = re.search(r"\{\{(.+?)\}\}", target_raw)
    if tm:
        trigger_raw = tm.group(1)
        if "|" in trigger_raw:
            trigger_raw = trigger_raw.split("|")[1]
        result["target_trigger"] = trigger_raw
        result["ability_text"] = "%s_ability" % trigger_raw
    # Extract count (e.g., "1つ" in "能力1つを発動させる")
    cnt = extract_count(text)
    if cnt:
        result["count"] = cnt
    return result


def _try_baton_touch_effect(text):
    """バトンタッチ + 場合 — baton touch specific condition."""
    if "バトンタッチ" not in text or "場合" not in text:
        return None
    m = re.search(r"([^場合]+)場合", text)
    if not m:
        return None
    cond_text = m.group(0)
    action_text = text.replace(cond_text, "").strip()
    cond = parse_condition(cond_text)
    cond["type"] = "baton_touch"
    action = parse_action(action_text)
    result = {"text": text, "condition": cond}
    result.update(action)
    return result


def _try_kore_niyori_result(text):
    """これにより～した場合/とき — conditional on result (invalidation follow-up, discard follow-up, etc.)."""
    if "これにより" not in text:
        return None
    # Support both 場合 and とき as condition markers
    cond_marker = None
    for marker in ["場合", "とき"]:
        m = re.search(r"これにより(.+?)" + marker, text)
        if m:
            cond_marker = marker
            break
    if cond_marker is None:
        return None
    m = re.search(r"これにより(.+?)" + cond_marker, text)
    if not m:
        return None
    parts = text.split("これにより", 1)
    sp = "これにより" + parts[1].strip()
    if cond_marker not in sp:
        return None
    cp, fp = sp.split(cond_marker, 1)
    cond = parse_condition(cp.strip() + cond_marker)
    # "custom" type conditions can't be evaluated by the engine — skip them
    if cond.get("type") == "custom":
        cond = None
    primary_text = parts[0].strip()
    # Skip empty/trivial primary text (e.g. cost text already consumed, or just bracket fragments)
    if not primary_text or re.match(r"^[\s）」\)』」、。]*$", primary_text):
        return None
    return {
        "text": text,
        "action": "conditional_on_result",
        "primary_effect": parse_action(primary_text),
        "result_condition": cond,
        "followup_action": parse_action(fp.strip()),
    }


def _try_sou_shinakatta(text):
    """そうしなかった場合 — conditional on optional action NOT taken."""
    if "そうしなかった場合" not in text:
        return None
    parts = text.split("そうしなかった場合", SPLIT_LIMIT)
    opt_text = parts[0].strip()
    fa = parse_action(opt_text)
    # If optional action starts with "相手は" (opponent does X), set target to opponent
    if opt_text.startswith("相手は"):
        fa["target"] = "opponent"
    aa_text = parts[1].strip().lstrip("、")
    # The alternative action text may include duration prefixes
    aa = parse_effect(aa_text)
    result = {
        "text": text,
        "action": "conditional_on_optional",
        "optional_action": fa,
        "conditional_action": aa,
        "conditional_negation": True,
    }
    return result


def _try_same_action(text):
    """そうした場合 — conditional on optional action."""
    if "そうした場合" not in text:
        return None
    parts = text.split("そうした場合", SPLIT_LIMIT)
    fa = parse_action(parts[0].strip())
    aa = parse_action(parts[1].strip())
    if "それぞれ" in parts[1] or "ずつ" in parts[1]:
        aa["multiple_targets"] = True
    if fa.get("action") == "select":
        if isinstance(aa, dict):
            aa["source"] = "selected_cards"
    result = {
        "text": text,
        "action": "conditional_on_optional",
        "optional_action": fa,
        "conditional_action": aa,
    }
    return result


def _try_shi_sequential(text):
    """Aし、B — multiple actions joined by te-form."""
    if "、" not in text:
        return None
    # If choice marker is present, let _try_choice handle the text
    # (te-form connectors inside bullet options should not split the choice)
    if CHOICE_MARKER in text:
        return None
    # If condition markers are present, let _try_conditional handle the text
    # to prevent splitting "Aし、B、Cした場合、D" into fragments
    if any(m in text for m in CONDITION_MARKERS):
        return None
    parts = [p.strip().rstrip("、") for p in text.split("、")]
    if len(parts) < 2 or not parts[0].strip().endswith("し"):
        return None
    actions = [
        parse_action(p)
        for p in parts
        if parse_action(p).get("action") not in ("custom", "do_nothing")
    ]
    if len(actions) >= 2:
        return {"text": text, "action": "sequential", "actions": actions}
    return None


def _try_te_sequential(text):
    """Xを得て、Yを得る — te-form sequential for resource gains (e.g. blade + heart on different targets).
    Splits 'member A gets resource X, different member B gets resource Y' into sequential actions."""
    if "を得て、" not in text or "を得る" not in text:
        return None
    parts = text.split("を得て、", 1)
    if len(parts) != 2:
        return None
    left = parts[0].strip() + "を得る"
    right = parts[1].strip()
    fa = parse_action(left)
    sa = parse_action(right)
    if fa.get("action") != "custom" and sa.get("action") != "custom":
        return {"text": text, "action": "sequential", "actions": [fa, sa]}
    return None


def _try_global_modifier(text):
    """～は、～ — global state change modifier."""
    m = re.search(r".+は、.+", text)
    if not m or "ある場合" in text:
        return None
    if "必要ハート" in text and ("多くなる" in text or "少なくなる" in text):
        result = {
            "text": text,
            "action": "restriction",
            "restriction_type": "modify_required_hearts_global",
            "operation": "increase" if "多くなる" in text else "decrease",
        }
        tm = re.search(r"([^は]+)は", text)
        if tm:
            raw_target = tm.group(1).strip()
            if "相手の" in raw_target:
                result["target"] = "opponent"
            else:
                result["target"] = raw_target
        if "すべて" in text:
            result["all"] = True
        return result
    return None


def _try_play_baton_touch(text):
    """プレイに際し、バトンタッチしてもよい — play baton touch."""
    if "プレイに際し" not in text or "バトンタッチ" not in text:
        return None
    result = {"text": text, "action": "play_baton_touch"}
    m = re.search(r"(\d+)人のメンバーとバトンタッチ", text)
    if m:
        result["count"] = int(m.group(1))
    return result


def _try_lose_resource(text):
    """を失う — lose/gain-negative resource effects (e.g. ブレードを失う)."""
    if "を失う" not in text:
        return None
    result = {"text": text, "action": "gain_resource", "sign": "negative"}
    # Detect what's being lost
    if "ブレード" in text:
        result["resource"] = "blade"
    elif "ハート" in text:
        result["resource"] = "heart"
    # Count icons in the lost resource
    blade_count = len(re.findall(r"\{\{icon_blade\.png\|ブレード\}\}", text))
    if blade_count > 0:
        result["count"] = blade_count
    heart_count = len(re.findall(r"\{\{heart_\d+\.png\|heart\d+\}\}", text))
    if heart_count > 0:
        result["count"] = heart_count
        result["heart_colors"] = extract_heart_types(text)
    # Duration
    if "ライブ終了時まで" in text:
        result["duration"] = "live_end"
    return result


def _try_duration_effect(text):
    """かぎり — duration effects.
    If the condition is an "unless pay N energy" pattern (negation + energy resource),
    convert to an optional cost instead of a conditional effect (Q92: player chooses)."""
    if DURATION_MARKER not in text:
        return None
    parts = text.split(DURATION_MARKER, SPLIT_LIMIT)
    ct = parts[0].strip() + DURATION_MARKER
    at = parts[1].strip().lstrip("、")
    cond = parse_condition(ct)
    # Check for lose_resource in the action part before falling back to parse_action
    action = _try_lose_resource(at)
    if action is None:
        action = parse_action(at)

    # Detect "unless pay N energy": negated condition with energy resource type
    if (
        cond.get("negation")
        and cond.get("resource_type") == "energy"
        and cond.get("count")
        and cond.get("operator") == ">="
    ):
        energy_count = cond["count"]
        # Restructure as optional cost + unconditional effect
        result = {"text": text, "action": action.get("action", "custom")}
        result["cost"] = {
            "type": "pay_energy",
            "energy": energy_count,
            "optional": True,
        }
        if action.get("action") == "sequential":
            result["actions"] = action.get("actions", [])
        else:
            result.update(action)
        return result

    result = {"text": text, "condition": cond, "duration": "as_long_as"}
    result.update(action)
    return result


def _try_restriction_effect(text):
    """アクティブにならない — restriction effect preventing activation.
    Matches both "効果によってはアクティブにならない" and plain "アクティブフェイズにアクティブにならない"."""
    if "アクティブにならない" not in text:
        return None
    result = {
        "text": text,
        "action": "restriction",
        "restriction_type": "cannot_activate",
    }
    if "アクティブフェイズ" in text:
        result["phase"] = "active_phase"
    if "効果によっては" in text:
        result["restriction_type"] = "cannot_activate_by_effect"
    if "自分と相手の" in text:
        result["target"] = "both"
    elif "自分の" in text:
        result["target"] = "self"
    elif "相手の" in text:
        result["target"] = "opponent"
    if "このターン" in text:
        result["duration"] = "this_turn"
    if "メンバー" in text:
        result["card_type"] = "member_card"
    if "エネルギー" in text:
        result["card_type"] = "energy_card"
    return result


def _try_blade_actions(text):
    """Blade-related actions: gain_equal, same_thing, set_blade_count, blade_conversion."""
    # blade conversion: すべて[色]になる
    if "すべて[" in text and "]になる" in text:
        m = re.search(r"すべて\[([^\]]+)\]", text)
        if m:
            result = {
                "text": text,
                "action": "set_blade_type",
                "blade_type": m.group(1),
            }
            if "ライブ終了時まで" in text:
                result["duration"] = "live_end"
            return result

    # gain_resource with equality condition: コストが同じ + を得る
    if "を得る" in text and "コストが同じ" in text:
        result = {"text": text, "action": "gain_resource", "resource": "blade"}
        ic = text.count("{{icon_blade.png|ブレード}}")
        if ic > 0:
            result["count"] = ic
        return result

    # same_thing: 同じことを行う
    if "同じことを行う" in text:
        result = {"text": text, "action": "gain_resource", "resource": "blade"}
        ic = text.count("{{icon_blade.png|ブレード}}")
        result["count"] = ic if ic > 0 else 1
        if "ライブ終了時まで" in text:
            result["duration"] = "live_end"
        return result

    # set_blade_count: ブレードの数はXつになる
    if "ブレードの数は" in text and ("つになる" in text or "になる" in text):
        result = {"text": text, "action": "set_blade_count"}
        m = re.search(r"(\d+)つになる", text) or re.search(r"(\d+)になる", text)
        if m:
            result["count"] = int(m.group(1))
        return result

    return None


def _try_both_discard_until(text):
    """自分と相手はそれぞれ...枚になるまで手札を控え室に置き, その後..."
    Both players each discard until condition, then both draw."""
    if (
        "自分と相手はそれぞれ" not in text
        or "枚になるまで" not in text
        or ("控え室に置き" not in text and "控え室に置く" not in text)
    ):
        return None
    result = {
        "text": text,
        "action": "sequential",
        "target": "both",
        "multiple_targets": True,
    }
    # Split on その後
    parts = re.split(r"その後[、。]?", text, maxsplit=1)
    if len(parts) == 2:
        fa_text = parts[0].strip()
        sa_text = parts[1].strip()
        fa = {
            "text": fa_text,
            "action": "discard_until_count",
            "target": "both",
            "multiple_targets": True,
        }
        m = re.search(r"(\d+)枚になるまで", fa_text)
        if m:
            fa["target_count"] = int(m.group(1))
        sa = parse_effect(sa_text)
        result["actions"] = [fa, sa]
    return result if result.get("actions") else None


def _try_re_yell(text):
    """もう一度エールを行う + ブレードハートを失い — re-yell."""
    if "もう一度エールを行う" not in text or "ブレードハートを失い" not in text:
        return None
    return {"text": text, "action": "re_yell", "lose_blade_hearts": True}


def _try_energy_under_member(text):
    """under_member source — energy placement."""
    if "下に置かれているエネルギーカード" not in text:
        return None
    return {
        "text": text,
        "action": "place_energy_under_member",
        "source": "under_member",
        "card_type": "energy_card",
        "energy_count": 1,
        "target_member": "this_member",
    }


# ============== FALLTHROUGH PATTERN MATCHERS ==============

_EFFECT_HANDLERS = [
    _try_per_unit,
    _try_conditional_alternative,
    _try_character_specific,
    _try_activation_suffix,
    _try_cost_modification,
    _try_kore_niyori_case,
    _try_look_and_select,
    _try_answer_choice,
    _try_each_time,
    _try_opponent_action,
    _try_choose_self_opponent,
    _try_opponent_after_conditional,
    _try_reveal_until_chosen_card,
    _try_reveal_until_live,
    _try_furthermore,
    _try_kore_niyori_result,
    _try_sequential_duration,
    _try_conditional_sequential,
    _try_same_action,
    _try_sequential,
    _try_duration_effect,
    _try_sou_shinakatta,
    _try_period_conditional,
    _try_compound_select,
    _try_shi_sequential,
    _try_te_sequential,
    _try_implicit_sequential,
    _try_conditional,
    _try_ability_activation,
    _try_choice,
    _try_kore_niyori_cascade,
    _try_baton_touch_effect,
    _try_global_modifier,
    _try_play_baton_touch,
    _try_energy_under_member,
    _try_blade_actions,
    _try_both_discard_until,
    _try_lose_resource,
    _try_re_yell,
    _try_restriction_effect,
]


def _merge_parenthetical(target, parenthetical):
    """Merge extracted parenthetical notes into target dict (handles activation conditions)."""
    if not parenthetical or "parenthetical" in target:
        return
    target["parenthetical"] = parenthetical
    for note in parenthetical:
        if "起動できる" in note or "発動する" in note:
            # Only parse positional conditions from parenthetical notes
            # (e.g. "センターエリアにいる場合のみ発動できる").
            # Informational notes like "対戦相手のカードの効果でも発動する"
            # are stored in text, not parsed.
            if "センター" in note or "サイド" in note or "エリアにいる場合" in note:
                cond_parsed = parse_condition(note)
                if cond_parsed and cond_parsed.get("type") != "custom":
                    target["activation_condition_parsed"] = cond_parsed
            if "センターエリア" in note:
                target["activation_position"] = "center"
            elif "左サイドエリア" in note or "左サイド" in note:
                target["activation_position"] = "left_side"
            elif "右サイドエリア" in note or "右サイド" in note:
                target["activation_position"] = "right_side"
            break


def parse_effect(text: str) -> Dict[str, Any]:
    """Parse an effect text. Tries handlers in priority order, then falls back to single action."""
    text = normalize_fullwidth_digits(text).strip()
    text = strip_suffix_period(text)

    # Handle duration prefix — strip and mark
    dur_result = _try_duration_prefix(text)
    had_duration = dur_result is not None
    if had_duration:
        text = dur_result["_rest"]
        effect = dur_result
    else:
        effect = {"text": text}

    # Extract parenthetical notes
    parenthetical = extract_parenthetical(text)
    text = strip_parenthetical(text)

    # Also check the full original text for activation condition patterns (e.g.
    # "（この能力はセンターエリアに登場している場合のみ起動できる。）") that
    # may have been in parenthetical notes. Extract them early so they can be
    # propagated to the effect even if _merge_parenthetical fails.
    extra_activation_cond = None
    extra_activation_pos = None
    if parenthetical:
        import sys

        print(
            f"[PAREN] {len(parenthetical)} note(s), first={parenthetical[0][:40]}",
            file=sys.stderr,
        )
    for note in parenthetical:
        if "起動できる" in note or "発動する" in note:
            if "センター" in note or "サイド" in note or "エリアにいる場合" in note:
                cond_parsed = parse_condition(note)
                if cond_parsed and cond_parsed.get("type") != "custom":
                    extra_activation_cond = cond_parsed
            if "センターエリア" in note:
                extra_activation_pos = "center"
            elif "左サイドエリア" in note or "左サイド" in note:
                extra_activation_pos = "left_side"
            elif "右サイドエリア" in note or "右サイド" in note:
                extra_activation_pos = "right_side"

    # Try all handlers in priority order
    for handler in _EFFECT_HANDLERS:
        result = handler(text)
        if result is not None:
            _merge_parenthetical(result, parenthetical)
            # Apply duration prefix info
            if "duration" in effect and "duration" not in result:
                result["duration"] = effect["duration"]
            # Propagate duration to sub-actions in sequential/choice/conditional_alternative effects
            dur = result.get("duration")
            if dur:
                if result.get("action") in ("sequential", "conditional_alternative"):
                    for sub in result.get("actions", []):
                        if "duration" not in sub and sub.get("action") in (
                            "gain_resource",
                            "modify_score",
                            "change_state",
                            "set_blade_count",
                        ):
                            sub["duration"] = dur
                    for key in ("primary_effect", "alternative_effect"):
                        sub = result.get(key)
                        if (
                            sub
                            and "duration" not in sub
                            and sub.get("action")
                            in (
                                "gain_resource",
                                "modify_score",
                                "change_state",
                                "set_blade_count",
                            )
                        ):
                            sub["duration"] = dur
            # Handle choice and conditional_on_result results with early return
            dur = result.get("duration")
            if result.get("action") == "choice":
                for opt in result.get("options", []):
                    if (
                        dur
                        and "duration" not in opt
                        and opt.get("action")
                        in (
                            "gain_resource",
                            "modify_score",
                            "change_state",
                            "set_blade_count",
                        )
                    ):
                        opt["duration"] = dur
                return result
            if result.get("action") == "conditional_on_result":
                for key in ("primary_effect", "followup_action"):
                    sub = result.get(key)
                    if sub and dur and "duration" not in sub:
                        sub["duration"] = dur
                return result
            # For all other handlers, use the result as the effect directly
            # (do NOT run parse_action on the full text — that would leak
            #  card_type/target/etc from the condition into the effect)
            effect = result
            _merge_parenthetical(effect, parenthetical)
            if extra_activation_cond and "activation_condition_parsed" not in effect:
                import sys

                print(
                    f"[ACTIVATION] setting activation_condition_parsed on effect",
                    file=sys.stderr,
                )
                effect["activation_condition_parsed"] = extra_activation_cond
            if extra_activation_pos and "activation_position" not in effect:
                effect["activation_position"] = extra_activation_pos
            return effect

    # No handler matched: fallback to parse_action
    effect.pop("_rest", None)
    fallback_text = text

    action = parse_action(fallback_text)
    effect.update(action)

    _merge_parenthetical(effect, parenthetical)
    if extra_activation_cond and "activation_condition_parsed" not in effect:
        effect["activation_condition_parsed"] = extra_activation_cond
    if extra_activation_pos and "activation_position" not in effect:
        effect["activation_position"] = extra_activation_pos

    # Post-fallback exact text matches
    normalized = re.sub(r"\s+", "", fallback_text)
    pattern_text = re.sub(r"\{\{[^|]+\|([^}]+)\}\}", r"\1", normalized)

    extra_checks = [
        (
            lambda: "ブレードの数は3つになる" in pattern_text,
            lambda: effect.update({"action": "set_blade_count", "count": 3})
            or (
                effect.update({"duration": "live_end"})
                if "duration" not in effect and "ライブ終了時まで" in pattern_text
                else None
            ),
        ),
        (
            lambda: "何もしない" in pattern_text,
            lambda: effect.update({"action": "choice", "choice_type": "emma_punch"}),
        ),
        (
            lambda: "元々の" in pattern_text
            and "ブレード" in pattern_text
            and "同じ場合についても同じことを行う" in pattern_text,
            lambda: effect.update(
                {
                    "action": "gain_resource",
                    "resource": "blade",
                    "count": effect.get("count", 1),
                }
            )
            or (
                effect.update({"duration": "live_end"})
                if "duration" not in effect and "ライブ終了時まで" in pattern_text
                else None
            ),
        ),
        (
            lambda: "カードを1枚引いてもよい" in pattern_text,
            lambda: effect.update(
                {"action": "draw_card", "count": 1, "optional": True}
            ),
        ),
        (
            lambda: effect.get("per_unit")
            and "コスト" in pattern_text
            and "少なくなる" in pattern_text
            and effect.get("location") == "hand",
            lambda: effect.update({"action": "modify_cost", "operation": "subtract"}),
        ),
        (
            lambda: effect.get("character_effects")
            and "ハート" in pattern_text
            and "ブレード" in pattern_text,
            lambda: effect.update({"action": "gain_resource"}),
        ),
    ]
    for check, apply in extra_checks:
        if check():
            apply()
            break

    # Ensure action field
    if "action" not in effect and "actions" not in effect:
        effect["action"] = "custom"
    if "actions" in effect and not effect["actions"]:
        del effect["actions"]

    # Fallback per_unit action inference
    if effect.get("per_unit") and "action" not in effect:
        if "ブレードを得る" in fallback_text or "選んだブレード" in fallback_text:
            effect["action"] = "gain_resource"
            effect["resource"] = "blade"
            ic = fallback_text.count("{{icon_blade.png|ブレード}}")
            if ic > 0:
                effect["count"] = ic
        elif "ハートを得る" in fallback_text or "選んだハート" in fallback_text:
            effect["action"] = "gain_resource"
            effect["resource"] = "heart"
        elif "引く" in fallback_text:
            effect["action"] = "draw_card"

    # Apply duration prefix
    if "duration" in effect and "duration" not in locals().get("dur_effect", {}):
        pass  # Already handled inline

    return effect


# ============== POST-PROCESSING NORMALIZER ==============


def _normalize_effect_tree(effect, original_text=None):
    """Post-processing pass to fix common parser artifacts:
    - Remove do_nothing actions between real actions
    - Propagate fields from parent to sub-actions
    - Extract activation position from parenthetical text & main text
    - Propagate exclude_self, all, position from text to sub-actions
    """
    if not effect or not isinstance(effect, dict):
        return effect

    # Scan the full text once for field hints
    _full_text = effect.get("text") or original_text or ""

    def _has_position_keywords(text):
        for keyword, position in POSITION_KEYWORDS.items():
            if keyword in text:
                return position
        if "center" in text.lower():
            return "center"
        return None

    def _has_original_modifier(text):
        """Check if text contains 元々持つ (original/natural value)."""
        return "元々持つ" in text or "元々" in text

    def _clean_action_list(actions, parent_effect=None, parent_text=""):
        if not actions:
            return actions
        # Remove do_nothing actions
        cleaned = []
        for a in actions:
            if a.get("action") == "do_nothing":
                continue
            cleaned.append(a)
        if not cleaned:
            return actions[-1:] if actions else []
        # Propagate fields from parent to each sub-action
        if parent_effect:
            for f in (
                "exclude_self",
                "all",
                "target",
                "position",
                "activation_position",
                "source_position",
                "exclude_position",
            ):
                if f in parent_effect:
                    for sub in cleaned:
                        if f not in sub:
                            sub[f] = parent_effect[f]
            # Propagate card_type from parent to sub-actions that don't have it
            pt = parent_effect.get("card_type")
            if pt:
                for sub in cleaned:
                    if "card_type" not in sub:
                        sub["card_type"] = pt
            # Propagate cost_limit from parent to sub-actions
            cl = parent_effect.get("cost_limit")
            if cl:
                for sub in cleaned:
                    if "cost_limit" not in sub:
                        sub["cost_limit"] = cl
        return cleaned

    def _walk(d, ctx_text=None):
        if not isinstance(d, dict):
            return d
        d_ctx = d.get("text") or ctx_text or _full_text
        d_text = d.get("text") or ""

        # Check parenthetical for activation position if not set
        if "activation_position" not in d:
            parenthetical = d.get("parenthetical", [])
            for note in (
                parenthetical if isinstance(parenthetical, list) else [parenthetical]
            ):
                if "起動できる" in note or "発動する" in note:
                    pos = _has_position_keywords(note)
                    if pos:
                        d["activation_position"] = pos
                        break

        # Propagate exclude_self from text context to sub-actions
        if (
            "exclude_self" not in d
            and d_ctx
            and ("このメンバー以外" in d_ctx or "ほかの" in d_ctx or "他の" in d_ctx)
        ):
            d["exclude_self"] = True

        # Propagate distinct from text context — use string form for serde compat
        if (
            "distinct" not in d
            and d_ctx
            and ("名前の異なる" in d_ctx or "異なる名前" in d_ctx)
        ):
            d["distinct"] = "card_name"

        # Propagate original_value from text context (元々持つ for blade/heart comparisons)
        if "original_value" not in d and d_ctx and _has_original_modifier(d_ctx):
            d["original_value"] = True

        # Propagate all from text context
        if "all" not in d and d_ctx:
            if (
                "すべての" in d_ctx
                or "全ての" in d_ctx
                or "全部の" in d_ctx
                or "カードをすべて" in d_ctx
            ):
                d["all"] = True

        # Propagate multiple_targets from parent text to sub-actions
        if d_ctx and "それぞれ" in d_ctx:
            for sub_key in ("actions", "options"):
                if sub_key in d:
                    for sub in d.get(sub_key, []):
                        if "multiple_targets" not in sub:
                            if (
                                "それぞれ" in sub.get("text", "")
                                or sub.get("target") == "both"
                            ):
                                sub["multiple_targets"] = True

        # Propagate count/source from move_cards to subsequent conditions
        # "すべて"/"全部": "If ALL moved cards match" → count=N, operator="="
        # "それらの中に": "If any among moved cards match" → source="preceding_moved"
        if d.get("action") == "sequential" and "actions" in d:
            acts = d["actions"]
            prev_count = None
            for act in acts:
                if act.get("action") == "move_cards" and act.get("count"):
                    prev_count = act["count"]
                cond = act.get("condition", {})
                cond_text = cond.get("text", "")
                if (
                    "すべて" in cond_text or "全部" in cond_text
                ) and prev_count is not None:
                    if cond.get("type") == "card_count_condition":
                        if (
                            cond.get("count", 1) == 1
                            and cond.get("operator", ">=") == ">="
                        ):
                            cond["count"] = prev_count
                            cond["operator"] = "="
                            cond["source"] = "preceding_moved"
                if "それらの中に" in cond_text and prev_count is not None:
                    if (
                        cond.get("type") == "card_count_condition"
                        and cond.get("source") is None
                    ):
                        cond["source"] = "preceding_moved"

        # Collapse single-action sequential wrappers (preserve condition + trigger_type + text)
        if (
            d.get("action") == "sequential"
            and d.get("actions")
            and len(d["actions"]) == 1
        ):
            inner = d["actions"][0]
            if not d.get("condition") and not d.get("conditional"):
                outer_fields = {}
                for k in ("condition", "trigger_type", "text"):
                    if k in d:
                        outer_fields[k] = d[k]
                d.clear()
                d.update(inner)
                for k, v in outer_fields.items():
                    if k not in d:
                        d[k] = v

        # Default target to "self" for location_conditions if missing
        if d.get("type") == "location_condition" and "target" not in d:
            d["target"] = "self"

        # Infer operator for comparison conditions when counts are present
        ct = d.get("condition_type") or d.get("type")
        if (
            ct in ("comparison_condition", "card_count_condition")
            and "operator" not in d
        ):
            if d.get("values"):
                d["operator"] = "in"
            elif d.get("comparison_target") and not d.get("operator"):
                text = d.get("text", "")
                if "高い" in text or "多い" in text or "大きい" in text:
                    d["operator"] = ">"
                elif "低い" in text or "少ない" in text or "小さい" in text:
                    d["operator"] = "<"
            elif (
                d.get("count")
                and not d.get("operator")
                and not d.get("comparison_target")
            ):
                d["operator"] = "="

        # Default per_unit_count to 1 when missing
        if d.get("per_unit") and "per_unit_count" not in d:
            d["per_unit_count"] = 1

        # Propagate position from text context (for condition+action splits)
        # Don't set position if source_position or exclude_position already set
        if (
            "position" not in d
            and "source_position" not in d
            and "exclude_position" not in d
            and d_ctx
        ):
            pos = _has_position_keywords(d_ctx)
            if pos:
                d["position"] = pos

        # Strip {{center.png|センター}} etc from text when extracted as position
        if d.get("position") and d_text:
            d["text"] = re.sub(
                r"\{\{.+?\.png\|(?:センター|左サイド|右サイド)\}\}", "", d_text
            ).strip()

        # Mark original_value flag for 元々持つ patterns
        if "original_value" not in d and d_text and _has_original_modifier(d_text):
            d["original_value"] = True

        # Mark group_reference for non-bracket group name patterns (safe string field)
        if "group_reference" not in d and d_ctx:
            if "同じグループ名" in d_ctx:
                d["group_reference"] = "same_group_name"
            elif "グループ名が異なる" in d_ctx or "グループ名がそれぞれ異なる" in d_ctx:
                d["group_reference"] = "different_group_names"

        # Set same_unit_name for cost text containing '同じユニット名'
        if "same_unit_name" not in d and "同じユニット名" in (d.get("text", "") or ""):
            d["same_unit_name"] = True

        # Propagate heart_colors from effect into condition for collective heart checks
        # Supports or_condition (propagate into sub-conditions) and location_condition
        if "heart_colors" in d and "condition" in d:
            cond = d["condition"]
            if isinstance(cond, dict) and "heart_colors" not in cond:
                if cond.get("type") == "or_condition":
                    for sub in cond.get("conditions", []):
                        if isinstance(sub, dict) and "heart_colors" not in sub:
                            sub["heart_colors"] = d["heart_colors"]
                elif cond.get("type") in ("location_condition", "card_count_condition"):
                    cond["heart_colors"] = d["heart_colors"]

        # Strip leading comma from text artifacts (e.g. "、{{icon_energy.png|E}}支払ってもよい")
        if d_text and (d_text.startswith("、") or d_text.startswith("，")):
            d["text"] = d_text.lstrip("、，").strip()

        # Recurse into sub-actions
        for sub_key in (
            "actions",
            "options",
            "primary_effect",
            "alternative_effect",
            "select_action",
            "look_action",
            "opponent_action",
            "followup_action",
            "optional_action",
            "conditional_action",
        ):
            sub = d.get(sub_key)
            if isinstance(sub, list):
                # Filter out do_nothing from action lists
                if sub_key == "actions":
                    d[sub_key] = _clean_action_list(sub, d, d_ctx)
                    sub = d[sub_key]
                for item in sub:
                    _walk(item, d_ctx)
            elif isinstance(sub, dict):
                _walk(sub, d_ctx)

        # Clean up empty actions
        if d.get("action") == "sequential" and not d.get("actions"):
            d.pop("action", None)

        return d

    return _walk(effect, original_text)


def _strip_parenthetical(text):
    """Remove parenthetical notes (rule clarifications) before parsing."""
    text = re.sub(r"（[^）]*）", "", text)
    text = re.sub(r"\([^)]*\)", "", text)
    return text.strip()


def _clean(obj):
    """Recursively remove null/false/0/empty fields from dicts/lists."""
    if isinstance(obj, dict):
        return {
            k: _clean(v)
            for k, v in obj.items()
            if v is not None
            and v is not False
            and v != 0
            and v != []
            and v != {}
            and v != ""
        }
    if isinstance(obj, list):
        cleaned = [_clean(item) for item in obj]
        return [x for x in cleaned if x is not None and x != {}]
    return obj


# Required field validators per action type
_VALIDATORS = {
    "gain_resource": {"required": ["resource", "count"]},
    "move_cards": {"required": ["source", "destination"]},
    "draw_card": {"required": ["count"]},
    "modify_score": {"required": ["operation", "value"]},
    "modify_required_hearts": {"required": ["heart_color", "count"]},
    "change_state": {"required": ["state_change"]},
}


def _validate_effect(eff, context=""):
    """Check required fields for each action type. Non-fatal warnings."""
    if not isinstance(eff, dict):
        return
    action = eff.get("action", eff.get("type", ""))
    rules = _VALIDATORS.get(action)
    if rules:
        for field in rules["required"]:
            if field not in eff or eff[field] is None:
                import sys

                print(
                    f"[VALIDATION] {context} {action} missing required field: {field}",
                    file=sys.stderr,
                )
    for sub_key in (
        "actions",
        "options",
        "primary_effect",
        "followup_action",
        "optional_action",
        "conditional_action",
        "look_action",
        "select_action",
        "opponent_action",
    ):
        sub = eff.get(sub_key)
        if isinstance(sub, dict):
            _validate_effect(sub, context)
        elif isinstance(sub, list):
            for item in sub:
                _validate_effect(item, context)


def parse_ability(triggerless_text: str) -> Dict[str, Any]:
    """Parse a complete ability text."""
    triggerless_text = normalize(triggerless_text.strip())

    ability = {
        "triggerless_text": triggerless_text,
    }

    # Split cost and effect (no need to pre-strip parenthetical —
    # the activation conditions in （...） are needed for later processing)
    cost_text, effect_text = split_cost_effect(triggerless_text)

    # Parse cost
    if cost_text:
        ability["cost"] = parse_cost(cost_text)

    # Parse effect
    if effect_text:
        effect = parse_effect(effect_text)
        if isinstance(effect, dict) and "cost" in effect:
            ability["cost"] = effect.pop("cost")
        effect = _normalize_effect_tree(effect, triggerless_text)

        # Enrich gain_ability nodes with parsed gained_effect (one level deep only)
        def _collect_gain(d, nodes):
            if isinstance(d, dict):
                if d.get("action") == "gain_ability" and d.get("ability_gain"):
                    nodes.append(d)
                for v in d.values():
                    if isinstance(v, dict):
                        _collect_gain(v, nodes)
                    elif isinstance(v, list):
                        for item in v:
                            _collect_gain(item, nodes)

        gain_nodes = []
        _collect_gain(effect, gain_nodes)
        for node in gain_nodes:
            if "gained_effect" not in node:
                gained = parse_effect(node["ability_gain"])
                if gained and gained.get("action") and gained.get("action") != "custom":
                    node["gained_effect"] = gained
                    # Pure gain_ability: the gained constant ability provides the effect.
                    # Do NOT add a separate direct action — the constant ability handles it.
                    # Just preserve the gain_ability action as-is.
        effect = _clean(effect)
        _validate_effect(effect, triggerless_text[:40])
        ability["effect"] = effect

    # Clean cost too
    if "cost" in ability:
        ability["cost"] = _clean(ability["cost"])

    return ability


# ============== PROCESSING ==============


def process_abilities(data: Dict[str, Any]) -> Dict[str, Any]:
    """Re-parse all abilities from triggerless_text using the current parser."""
    for ability in data["unique_abilities"]:
        triggerless = ability.get("triggerless_text", "")
        if triggerless:
            parsed = parse_ability(triggerless)
            if "effect" in parsed:
                ability["effect"] = parsed["effect"]
            if "cost" in parsed:
                ability["cost"] = parsed["cost"]
    return data


def _validate_semantic(abilities):
    """Quick semantic validation: checks parsed JSON against text patterns."""
    issues = []
    for i, entry in enumerate(abilities):
        t = entry.get("triggerless_text", "")
        eff = entry.get("effect") or {}
        if not t:
            continue
        # change_state: energy needs card_type=energy_card
        if (
            eff.get("action") == "change_state"
            and "エネルギー" in t
            and "メンバー" not in t
        ):
            if eff.get("card_type") != "energy_card":
                issues.append(
                    f"  #{i}: energy activation without card_type=energy_card"
                )
        # move_cards: cost_limit in text but not in effect
        if eff.get("action") == "move_cards" and re.search(r"コスト\d+", t):
            if eff.get("cost_limit") is None:
                issues.append(f"  #{i}: cost_limit in text but not in effect")
        # look_and_select: heart_colors on select parent but not on reveal sub-action
        if eff.get("action") == "look_and_select":
            sa = eff.get("select_action")
            if sa and isinstance(sa, dict):
                for act in sa.get("actions", []):
                    if (
                        isinstance(act, dict)
                        and act.get("action") == "reveal"
                        and not act.get("heart_colors")
                    ):
                        if sa.get("heart_colors"):
                            issues.append(
                                f"  #{i}: heart_colors on select parent but not on reveal sub-action"
                            )
    if issues:
        print(f"[Semantic] {len(issues)} issues:")
        for issue in issues[:15]:
            print(issue)
        if len(issues) > 15:
            print(f"  ... and {len(issues) - 15} more")


if __name__ == "__main__":
    import json
    from pathlib import Path

    abilities_file = Path(__file__).parent.parent / "abilities.json"

    with open(abilities_file, "r", encoding="utf-8") as f:
        data = json.load(f)

    result = process_abilities(data)

    with open(abilities_file, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)

    print("Normalized abilities.json with parser.py")

    # Run semantic validation on the output
    _validate_semantic(data["unique_abilities"])
