"""
Parser for ability extraction -- converts raw Japanese card ability text into
structured JSON consumed by the Rust engine and analysis tooling.

Architecture overview:
  cards.json (raw text)
    → extract_card_abilities.py (splits by newline, extracts trigger icons)
      → parser.py (this file):
          1. parse_cost()         - cost before colon (：)
          2. parse_effect()       - effect after colon, dispatched through handler cascade
          3. parse_condition()    - trigger conditions (場合/とき/たび)
          4. parse_action()       - individual actions (move, gain, etc.)
      → abilities.json (structured output consumed by Rust engine)

=== FILE STRUCTURE (line ranges) ===
  45- 62   Imports (parser_utils)
  64- 103  Shared configuration constants
 105- 169  Text extraction utilities
 171- 204  Regex patterns
 206- 413  Utility functions (extract_location, etc.)
 414- 459  split_cost_effect, split_condition_action
 460- 632  parse_complex_condition, _extract_basic_cost_fields
 633- 714  _try_duration_prefix, extract_phase_gate
 715- 840  parse_ability
 841-1058  parse_cost
1059-1296  parse_effect
1297-1430  parse_condition
1431-2785  parse_action + dispatch table (_R)
2786-4277  Condition handler functions (_try_*)
4278-4424  CONDITION_PATTERNS + _condition_registry
4427-4766  _extract_generic_fields (fallthrough extraction)
4767-4868  _infer_condition_type
4869-4932  _enrich_* helpers
4933-5048  Action utility helpers (_infer_card_type, infer_resource, etc.)
5049-5550  _fill_defaults
5551-8691  Effect handler functions (_try_effect_*) + _EFFECT_HANDLERS
8692-9180  _walk (post-processing tree walker)
9181-9720  _normalize_effect_tree
9721-10615 process_abilities (post-hoc fixes)
10616-10698 _propagate_optional, _merge_parenthetical
10699-10765 Validation helpers (_VALIDATORS, _validate_effect, _enrich_effect_type)
10766-11348 Semantic validation (_json_has*, _validate_semantic)
11348-11351 __main__ entry point

Condition types produced (type field in condition dict):
  complex                - nested compound conditions (AかつBかつC)
  compound               - "AかつB" / "Aあり、B" → {operator: "and", conditions: [...]}
  compound (or)          - "Aか、Bか" → {operator: "or", conditions: [...]}
  location_condition     - location-based count with optional distinct/card_type filters
  card_count_condition   - "N枚以上" / "N人以上" → {count, operator, card_type}
  card_blade_condition   - "ブレードがNつ以上" → {count, operator}
  comparison_condition   - cost/resource comparisons → {comparison_target, ...}
  highest_cost_on_stage_condition - "センターにいるメンバーが最も大きいコストを持つ"
  both_condition         - "それらが両方ある"
  temporal_condition     - "このターン" / live during / phase-gate → {temporal, phase}
  movement_condition     - "エリアを移動" / "このターン中にエリアを移動した"
  appearance             - "登場した" / "場に出た" → {type: "appearance"}
  zone_placement         - zone-to-zone movement (discard → waitroom, etc.)
  state_change_condition - "アクティブ状態からウェイト状態になった"
  energy_state           - energy <= N / >= N → {resource_type: "energy", count, operator}
  member_state           - "アクティブ状態" / "ウェイト状態" → {member_state}
  revealed_condition     - revealed card matching
  opponent_choice        - opponent-driven selection condition
  position_condition     - position check (センター/左サイド/右サイド)
  position_change_condition - "ポジションチェンジ"
  ability_filter_condition - "能力を持たない" / "能力も持たない"
  otherwise_condition    - "それ以外の場合"
  temporal_during_live   - "ライブ中" (without count) → {temporal: "during_live"}
  custom                 - fallback when no handler matches

On every run, abilities_debug.txt is written next to abilities.json
with a trace of every condition and effect handler call.
"""

import json
import re
import copy
from typing import Dict, Any, Optional, Tuple, List, Union

# Debug log accumulator: every parse_condition and parse_effect call appends
# a trace entry. Written to abilities_debug.txt at the end of the pipeline.
_DEBUG_LOG: List[str] = []
_DEBUG_CURRENT_SECTION: str = ""


def set_debug_section(section: str) -> None:
    """Emit a section header when starting to parse a new ability."""
    global _DEBUG_CURRENT_SECTION
    if section != _DEBUG_CURRENT_SECTION:
        _DEBUG_LOG.append(f"\n==== {section} ====")
        _DEBUG_CURRENT_SECTION = section


from parser_utils import (
    extract_count,
    extract_dynamic_count,
    extract_group_name,
    normalize_fullwidth_digits,
    strip_suffix_period,
    extract_by_pattern,
    extract_source,
    extract_destination,
    extract_target,
    extract_card_type,
    extract_operator,
    extract_cost_limit,
    extract_cost_limit_with_operator,
    extract_picker,
    detect_card_property,
    detect_require_all_hearts,
    SOURCE_PATTERNS,
    DESTINATION_PATTERNS,
    STATE_CHANGE_PATTERNS,
    LOCATION_PATTERNS,
    CARD_TYPE_PATTERNS,
    OPERATOR_PATTERNS,
    PriorityRegistry,
    ActionRule,
    EffectPattern,
)

# ============== CONFIGURATION CONSTANTS ==============
MAX_CHARACTER_NAME_LENGTH = 10
SPLIT_LIMIT = 1


# ============== POSITION FIELD SEMANTICS ==============
# The parser outputs three different position-related fields. They have different
# meanings and the Rust engine interprets them differently:
#
# "position" — Target card filter (single value only).
#   Used by: conditions (where must the target be?), actions (which cards to target?).
#   Engine check: compares against a specific stage slot. Expects ONE position
#   like "left_side" or "center". A comma-separated string here WILL BREAK
#   the engine's appearance condition cross-comparison check.
#
# "position_compare" — Second position for cross-comparison (single value only).
#   Used by: conditions that compare values at two positions
#   (e.g. "compare left vs right blade count"). Only set alongside "position".
#
# "activation_position" — Where the activating card must be (comma-separated OR).
#   Used by: effects to gate ability activation. "left_side,right_side" means
#   the ability works on EITHER side. The engine splits on comma and checks
#   if the card is at any listed position. This is the ONLY field that should
#   contain comma-separated multi-position values.
#
# RULE: When multi-position detection finds both left and right, set
#   activation_position = "left_side,right_side"
# Do NOT set position to "left_side,right_side" — that will break the engine.

# ============== POSITION KEYWORDS ==============
POSITION_KEYWORDS = {
    "センターエリア": "center",
    "左サイドエリア": "left_side",
    "右サイドエリア": "right_side",
    "センター": "center",
    "左サイド": "left_side",
    "右サイド": "right_side",
    "正面": "front",
}


def detect_positions(text: str) -> list:
    """Return all position values found in text via POSITION_KEYWORDS.

    Returns a list of position strings (e.g. ["left_side", "right_side"]).
    Deduplicates by position value so e.g. "左サイドエリア" and "左サイド"
    both contribute only one "left_side".
    """
    positions = []
    for _keyword, pos in POSITION_KEYWORDS.items():
        if pos in positions:
            continue
        if _keyword in text:
            positions.append(pos)
    return positions


def detect_icon_positions(text: str) -> list:
    """Return all positions from {{center.png|...}}, {{leftside.png|...}}, {{rightside.png|...}} icon templates."""
    positions = []
    if "{{center.png|センター}}" in text:
        positions.append("center")
    if "{{leftside.png|左サイド}}" in text:
        positions.append("left_side")
    if "{{rightside.png|右サイド}}" in text:
        positions.append("right_side")
    return positions


def format_positions(positions: list) -> str:
    """Join position list into comma-separated string, or return empty string."""
    return ",".join(positions)


def set_cross_position_fields(target, text):
    """Detect positions in text and set position/position_compare on target.

    For cross-comparison patterns like "left vs right" where the engine
    needs to compare values at two positions. Sets target["position"] to
    the first detected position and target["position_compare"] to the second.
    Returns True if any position was set. Does not overwrite existing position.
    """
    if "position" in target:
        return False
    matched = set(detect_positions(text))
    if "left_side" in matched and "right_side" in matched:
        target["position"] = "left_side"
        target["position_compare"] = "right_side"
    elif len(matched) == 1:
        target["position"] = next(iter(matched))
    elif len(matched) > 1:
        positions_list = sorted(matched)
        target["position"] = positions_list[0]
        target["position_compare"] = positions_list[1]
    else:
        return False
    return True


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
    # Continuative forms (used before かつ/、 in compound conditions)
    "高く": ">",
    "低く": "<",
    "多く": ">",
    "少なく": "<",
    "大きく": ">",
    "小さく": "<",
    # Plain/dictionary forms
    "高い": ">",
    "低い": "<",
    "少ない": "<",
    "多い": ">",
    "大きい": ">",
    "小さい": "<",
}

# ============== COMPARISON TYPES ==============
COMPARISON_TYPES = {"スコア": "score", "コスト": "cost"}


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

# ======================================================================
# TEXT EXTRACTION & UTILITY HELPERS
# ======================================================================


def _strip_duration_prefix(text):
    """Strip duration prefix from start of text. Returns (rest_text, code_or_None)."""
    for pat, code in DURATION_PREFIX_MAP.items():
        if text.startswith(pat):
            rest = text[len(pat) :].lstrip("、，").strip()
            return rest, code
    return text, None


# ============== COST MODIFICATION PATTERNS ==============
COST_MODIFICATION_PATTERNS = [
    (r"元々持つコストより(\d+)低い値に等しくなる", "decrease_by"),
    (r"元々持つコストより(\d+)高い値に等しくなる", "increase_by"),
    (r"コストが(\d+)以上になった場合", "cost_threshold"),
    (r"コストが(\d+)以下になった場合", "cost_threshold_below"),
    (r"コストは(\d+)減る", "decrease_by"),
    (r"コストは(\d+)減らす", "decrease_by"),
    (r"コストは(\d+)増える", "increase_by"),
    (r"コストは(\d+)増やす", "increase_by"),
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

# ====================================================================
# UTILITY FUNCTIONS
# ====================================================================
# Zone/type/target extraction helpers used throughout the parser.
# These extract single fields from raw Japanese text via pattern matching.
# ====================================================================


def extract_location(text: str) -> Optional[str]:
    """Extract location (general)."""
    return extract_by_pattern(text, LOCATION_PATTERNS)


def extract_locations(text: str) -> Optional[List[str]]:
    """Extract multiple locations connected by 'と' (e.g. 'ステージと控え室')."""
    locs = []
    for pattern, loc_name in LOCATION_PATTERNS:
        if pattern in text:
            locs.append(loc_name)
    if "success_live_card_zone" in locs and "live_card_zone" in locs:
        locs = [l for l in locs if l != "live_card_zone"]
    if len(locs) >= 2:
        return locs
    return None


def extract_state_change(text: str) -> Optional[str]:
    """Extract state change (wait/active)."""
    return extract_by_pattern(text, STATE_CHANGE_PATTERNS)


def extract_cost_range(text: str) -> Optional[Dict[str, int]]:
    """Extract cost range pattern like 'コスト4以上9以下' (cost >=4 AND <=9).
    Returns {"min": min_val, "max": max_val} or None."""
    m = re.search(r"コスト(\d+)以上(\d+)以下", text)
    if m:
        return {"min": int(m.group(1)), "max": int(m.group(2))}
    return None


def extract_blade_limit(text: str) -> Optional[Dict[str, Any]]:
    """Extract blade count limit from text like 'ブレードの数が3つ以下' (<=3 blades)."""
    normalized = re.sub(r"\{\{icon_blade\.png\|ブレード\}\}", "ブレード", text)
    m = re.search(r"ブレード[の]数[がは](\d+)[つ個](以下|以上|未満|超)", normalized)
    if not m:
        m = re.search(r"ブレード[の]数[がは](\d+)(以下|以上|未満|超)", normalized)
    if not m:
        m = re.search(r"ブレード[の]数[がは]ちょうど(\d+)[つ個]", normalized)
    if not m:
        m = re.search(r"ブレード[の]数[がは](\d+)[つ個](?!になる)", normalized)
    if m:
        result: Dict[str, Any] = {"blade_limit": int(m.group(1))}
        if len(m.groups()) >= 2 and m.group(2) and m.group(2) != "ちょうど":
            op = m.group(2)
            if op == "以下":
                result["blade_limit_operator"] = "<="
            elif op == "以上":
                result["blade_limit_operator"] = ">="
            elif op == "未満":
                result["blade_limit_operator"] = "<"
            elif op == "超":
                result["blade_limit_operator"] = ">"
        else:
            result["blade_limit_operator"] = "=="
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


def extract_position(text: str) -> Optional[Dict[str, Any]]:
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
    """Extract all group names within 『』 or mixed 『」 brackets."""
    names = re.findall(r"『([^』」]+)』", text)
    # Also handle mixed brackets: 『虹ヶ咲」 (opening 『, closing 」)
    names += re.findall(r"『([^』」]+)」", text)
    return names


def extract_heart_types(text: str) -> List[str]:
    """Extract heart type identifiers (e.g. heart02, heart01) from icon markup."""
    return re.findall(r"heart_(\d+)\.png\|heart(\d+)", text)  # type: ignore[return-value]


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
    return "人まで" in text or "枚まで" in text or "つまで" in text or "個まで" in text


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


def extract_name_exclusions(text):
    """Extract name inclusion/exclusion from 「X」以外 patterns.
    Returns (include_names, exclude_names) as lists.
    """
    includes = re.findall(r"「([^」]+)」", text)
    excludes = re.findall(r"「([^」]+)」以外", text)
    # Remove excluded from included
    includes = [n for n in includes if n not in excludes]
    return includes, excludes


def extract_cost_operator(text):
    """Extract cost comparison operator from Japanese text.
    Returns operator string like '<=', '>=', '<', '>', '=' or None.
    """
    if "以下" in text:
        return "<="
    elif "以上" in text:
        return ">="
    elif "未満" in text:
        return "<"
    elif "超" in text:
        return ">"
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


# ====================================================================
# STRUCTURAL PARSING
# ====================================================================
# split_cost_effect, split_condition_action, and related helpers that
# break raw ability text into structural segments before parsing.
# ====================================================================


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


def parse_complex_condition(text: str) -> Optional[Dict[str, Any]]:
    """Parse complex conditions with cause-effect relationships (e.g., これにより)."""
    # "かつこれにより" is an AND compound, not a complex cause-effect
    if "かつこれにより" in text:
        return None
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
    return None  # type: ignore[return-value]


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
    if "同じグループ名" in text:
        cost["group_reference"] = "same_group_name"
    if extract_optional(text):
        cost["optional"] = True
    if "シャッフルする" in text or "シャッフルして" in text:
        cost["shuffle"] = True
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
    if "このメンバー以外" in text or bool(re.search(r"ほかの.*?メンバー", text)):
        cost["exclude_self"] = True
    # Same unit name
    if "同じユニット名" in text:
        cost["same_unit_name"] = True
    if re.search(r"「.+」以外", text):
        cost["exclude_self"] = True
    if (
        "このメンバー" in text
        and "このメンバー以外" not in text
        and not bool(re.search(r"ほかの.*?メンバー", text))
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

    # Extract position restrictions (e.g., "センター", "左サイド").
    # Skip keywords that only appear inside icon templates {{...}} — those
    # are activation-requirement positions (e.g. {{center.png|センター}}),
    # already captured as activation_position on the effect. Only set cost
    # position from bare (non-icon) keywords which indicate target filtering.
    if "position" not in cost:
        clean_text = re.sub(r"\{\{[^}]+?\}\}", "", text)
        set_cross_position_fields(cost, clean_text)


def _try_duration_prefix(text):
    """ライブ終了時まで / ターン終了時まで / そのターンの間 — strip prefix and mark duration.
    Only matches if the pattern is at the very start of the text,
    not embedded within sub-effects/options."""
    rest, code = _strip_duration_prefix(text)
    if code:
        return {"text": rest, "duration": code, "_rest": rest}
    return None


# ======================================================================
# CORE PARSING ENTRY POINTS
# ======================================================================


_PHASE_GATE_RE = re.compile(
    r"(?:このゲームの\d+ターン目の)?"
    r"(?:自分の|相手の)?"
    r"(?:メイン|ライブ|セット|エール|アクティブ)フェイズ"
    r"(?:の場合|の間|に)"
)

_PHASE_MAP = {
    "メイン": "main",
    "ライブ": "live_phase",
    "セット": "set_phase",
    "エール": "yell_phase",
    "アクティブ": "active_phase",
}


def extract_phase_gate(text: str) -> Tuple[Optional[Dict[str, Any]], str]:
    """Extract a phase gate from ability text.

    Returns (gate_condition, remaining_text) or (None, original_text) if no
    phase gate is found or the match is a consequence (next-phase reference).
    """
    m = _PHASE_GATE_RE.search(text)
    if not m:
        return None, text

    # Skip consequence references (次のターンのXフェイズに)
    before = text[: m.start()]
    if "次の" in before or "来" in before:
        return None, text

    gate_phrase = m.group(0)
    remaining = text[: m.start()] + text[m.end() :]
    # Clean leading/trailing commas and orphaned particles
    remaining = re.sub(r"^[、，]\s*", "", remaining)
    remaining = re.sub(r"\s+$", "", remaining)

    # Determine phase
    phase_name = re.search(r"(メイン|ライブ|セット|エール|アクティブ)", gate_phrase)
    phase = _PHASE_MAP.get(phase_name.group(1)) if phase_name else None
    if not phase:
        return None, text

    gate: Dict[str, Any] = {
        "type": "temporal_condition",
        "phase": phase,
        "text": gate_phrase,
        "trigger_event": {"type": "temporal", "phase": phase},
    }

    # Phase target
    if gate_phrase.startswith("自分の"):
        gate["phase_target"] = "self"
        gate["trigger_event"]["phase_target"] = "self"
    elif gate_phrase.startswith("相手の"):
        gate["phase_target"] = "opponent"
        gate["trigger_event"]["phase_target"] = "opponent"

    # Turn number (e.g. このゲームの1ターン目のライブフェイズの場合)
    turn_m = re.search(r"(\d+)ターン目", gate_phrase)
    if turn_m:
        gate["turn_number"] = int(turn_m.group(1))
        gate["trigger_event"]["turn_number"] = int(turn_m.group(1))

    return gate, remaining


def parse_ability(triggerless_text: str) -> Dict[str, Any]:
    """Parse a complete ability text."""
    triggerless_text = normalize(triggerless_text.strip())

    ability: Dict[str, Any] = {
        "triggerless_text": triggerless_text,
    }

    # Extract phase gate as its own condition BEFORE splitting cost/effect.
    # The gate is a pure pre-check: "only during X phase" — no point
    # evaluating anything else if the phase is wrong.
    phase_gate, remaining_text = extract_phase_gate(triggerless_text)

    # Split cost and effect on the text with the phase prefix removed
    cost_text, effect_text = split_cost_effect(remaining_text)

    # Parse cost
    if cost_text:
        ability["cost"] = parse_cost(cost_text)

    # Extract activation_position from cost text (e.g. {{center.png|センター}})
    # This must be set before parse_effect so the effect gets the position.
    extra_pos_from_cost = None
    if "{{center.png|センター}}" in triggerless_text:
        extra_pos_from_cost = "center"
    elif "{{leftside.png|左サイド}}" in triggerless_text:
        extra_pos_from_cost = "left_side"
    elif "{{rightside.png|右サイド}}" in triggerless_text:
        extra_pos_from_cost = "right_side"

    # Parse effect
    if effect_text:
        effect = parse_effect(effect_text)
        if isinstance(effect, dict) and "cost" in effect:
            ability["cost"] = effect.pop("cost")
        effect = _normalize_effect_tree(effect, triggerless_text)
        if not isinstance(effect, dict):
            effect = {}

        # Fill missing condition from text — unified single field
        trigger_condition = None
        txt = remaining_text
        for sep in ["とき、", "場合、", "たび、", "なら、"]:
            idx = txt.find(sep)
            if idx >= 0:
                cond_text = txt[
                    : idx
                    + len(
                        "とき"
                        if sep == "とき、"
                        else "場合"
                        if sep == "場合、"
                        else "たび"
                        if sep == "たび、"
                        else "なら"
                    )
                ]
                if ")" in cond_text or "）" in cond_text:
                    continue
                tc = parse_condition(cond_text)
                if tc and tc.get("type") not in (None, "custom"):
                    trigger_condition = tc
                    break
        # Merge trigger condition with any condition already on the effect
        # (e.g. promoted from a sub-action by _try_re_yell).
        existing = effect.get("condition")
        if existing and isinstance(existing, dict) and trigger_condition:
            effect["condition"] = {
                "type": "compound",
                "operator": "and",
                "conditions": [trigger_condition, existing],
            }
        elif not existing and trigger_condition:
            effect["condition"] = trigger_condition
        elif not existing:
            for key in ("actions", "primary_effect", "conditional_action"):
                sub = effect.get(key)
                if isinstance(sub, dict):
                    sv = sub.get("condition")
                    if sv and isinstance(sv, dict):
                        effect["condition"] = copy.deepcopy(sv)
                        break
                elif isinstance(sub, list):
                    for item in sub:
                        if isinstance(item, dict):
                            sv = item.get("condition")
                            if sv and isinstance(sv, dict):
                                effect["condition"] = copy.deepcopy(sv)
                                break
                    if effect.get("condition"):
                        break

        # Apply activation_position from cost text to the effect
        if extra_pos_from_cost and "activation_position" not in effect:
            effect["activation_position"] = extra_pos_from_cost

        effect = _clean(effect)
        _validate_effect(effect, triggerless_text[:40])
        ability["effect"] = effect

    # Merge phase gate into effect["condition"] (not ability["condition"])
    # so the Rust Ability struct picks it up via AbilityEffect.condition.
    if phase_gate and ability.get("effect") and isinstance(ability["effect"], dict):
        existing_cond = ability["effect"].get("condition")
        if existing_cond and isinstance(existing_cond, dict):
            ability["effect"]["condition"] = {
                "type": "compound",
                "operator": "and",
                "conditions": [phase_gate, existing_cond],
            }
        else:
            ability["effect"]["condition"] = phase_gate

    # Clean cost too
    if "cost" in ability:
        ability["cost"] = _clean(ability["cost"])

    return ability


def parse_cost(text: str) -> Dict[str, Any]:
    """Parse a cost text."""
    cost: Dict[str, Any] = {"text": text}

    # Extract basic fields first for all cost types
    _extract_basic_cost_fields(cost, text)

    # Choice cost with "か" (OR marker) without trailing comma — BEFORE energy handler
    # e.g. "{{E}}{{E}}支払うか手札を2枚控え室に置いてもよい" → choice: [pay 2E, discard 2]
    verb_choice_m = re.search(r"(.*(?:支払う|置く|加える|公開する))か(.+)", text)
    if verb_choice_m:
        full_opt1 = text[: text.find("か", text.find(verb_choice_m.group(1)))].strip()
        if not full_opt1:
            full_opt1 = verb_choice_m.group(1).strip()
        opt2 = verb_choice_m.group(2).strip()
        return {
            "text": text,
            "type": "choice_condition",
            "options": [parse_cost(full_opt1), parse_cost(opt2)],
        }

    # Energy cost: count energy icons at start + distinct action (more specific)
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
                    # Propagate optional to sub-costs so they're all optional
                    for cp in result["costs"]:
                        cp["optional"] = True
                return result
        # Always set energy fields for energy costs (whether simple or with other text)
        energy_count = text.count("{{icon_energy.png|E}}")
        cost["type"] = "pay_energy"
        cost["energy"] = energy_count
        cost["zone"] = "energy_zone"
        cost["count"] = energy_count
        if "もよい" in text or "てもよい" in text:
            cost["optional"] = True
        if "好きな数" in text or "任意の数" in text:
            cost["any_number"] = True
        if extract_max(text):
            cost["max"] = True
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
            # Propagate position from parent cost extraction
            if "position" in cost:
                result["position"] = cost["position"]
                if "position_compare" in cost:
                    result["position_compare"] = cost["position_compare"]
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
    if extract_max(text):
        cost["max"] = True
        cost["any_number"] = True
    if (
        "好きな枚数" in text
        or "好きな枚数まで" in text
        or "任意の枚数" in text
        or "好きな組み合わせ" in text
    ):
        cost["any_number"] = True
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
        elif cost.get("destination") == "under_member":
            cost["type"] = "place_energy_under_member"
        elif cost.get("destination") in ("energy_deck", "energy_zone") and not cost.get(
            "source"
        ):
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
        elif "{{icon_energy.png|E}}" in text and (
            "支払う" in text or "支払って" in text
        ):
            cost["type"] = "pay_energy"
            cost["energy"] = text.count("{{icon_energy.png|E}}")
            if "もよい" in text or "てもよい" in text:
                cost["optional"] = True
        elif cost.get("source"):
            if cost["source"] == "hand" and (
                "控え室に置く" in text or "控え室に置いて" in text
            ):
                cost["destination"] = "discard"
                cost["type"] = "move_cards"
            elif cost["source"] == "discard" and "手札に加える" in text:
                cost["destination"] = "hand"
                cost["type"] = "move_cards"
            elif cost.get("destination"):
                cost["type"] = "move_cards"
            else:
                cost["type"] = "custom"
        else:
            cost["type"] = "custom"
    return cost


def parse_effect(text: str) -> Dict[str, Any]:
    """Parse an effect text. Tries handlers in priority order, then falls back to single action."""
    text = normalize_fullwidth_digits(text).strip()

    # Handle duration prefix — strip and mark
    dur_result = _try_duration_prefix(text)
    had_duration = dur_result is not None
    if had_duration:
        text = dur_result["_rest"]
        effect: Dict[str, Any] = dur_result
    else:
        effect: Dict[str, Any] = {"text": text}

    # Extract parenthetical notes (e.g. "（この能力は...）")
    parenthetical = extract_parenthetical(text)
    text = strip_parenthetical(text)
    # Strip trailing period AFTER removing parenthetical, so main-clause
    # "。" at the end is properly removed (fixes _try_implicit_sequential splitting).
    text = strip_suffix_period(text)

    # Also check the full original text for activation condition patterns (e.g.
    # "（この能力はセンターエリアに登場している場合のみ起動できる。）") that
    # may have been in parenthetical notes. Extract them early so they can be
    # propagated to the effect even if _merge_parenthetical fails.
    extra_activation_cond = None
    extra_activation_pos = None
    if parenthetical:
        for note in parenthetical:
            if "起動できる" in note or "発動する" in note:
                if "センター" in note or "サイド" in note or "エリアにいる場合" in note:
                    cond_parsed = parse_condition(note)
                    if cond_parsed and cond_parsed.get("type") != "custom":
                        extra_activation_cond = cond_parsed
                positions = []
                if "センターエリア" in note:
                    positions.append("center")
                if "左サイドエリア" in note or "左サイド" in note:
                    positions.append("left_side")
                if "右サイドエリア" in note or "右サイド" in note:
                    positions.append("right_side")
                if positions:
                    extra_activation_pos = ",".join(positions)

    # Also check the full_text/cost_text for position icon patterns
    # (e.g. {{center.png|センター}} in the cost/effect text)
    if extra_activation_pos is None:
        _icon_positions = detect_icon_positions(text)
        if _icon_positions:
            extra_activation_pos = ",".join(_icon_positions)

    # Try all handlers in priority order
    for _priority, hn, handler in _effect_registry.sorted_handlers():
        result = handler(text)
        if result is not None:
            _DEBUG_LOG.append(
                f"parse_effect({text!r})\n"
                f"  [effect] {hn}: MATCH → action={result.get('action')}\n"
                f"  output: {json.dumps(result, ensure_ascii=False)}"
            )
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
                _propagate_optional(result)
                return result
            if result.get("action") == "conditional_on_result":
                for key in ("primary_effect", "followup_action"):
                    sub = result.get(key)
                    if sub and dur and "duration" not in sub:
                        sub["duration"] = dur
                _propagate_optional(result)
                return result
            # For all other handlers, use the result as the effect directly
            effect = result
            _merge_parenthetical(effect, parenthetical)
            if extra_activation_cond and "activation_condition_parsed" not in effect:
                effect["activation_condition_parsed"] = extra_activation_cond
            if extra_activation_pos and "activation_position" not in effect:
                effect["activation_position"] = extra_activation_pos
            _propagate_optional(effect)
            return effect

    # No handler matched: fallback to parse_action
    effect.pop("_rest", None)
    fallback_text = text

    action = parse_action(fallback_text)
    effect.update(action)
    _DEBUG_LOG.append(
        f"parse_effect({text!r})\n"
        f"  → no handler matched, parse_action fallback\n"
        f"  output: {json.dumps(effect, ensure_ascii=False)}"
    )

    _merge_parenthetical(effect, parenthetical)
    if extra_activation_cond and "activation_condition_parsed" not in effect:
        effect["activation_condition_parsed"] = extra_activation_cond
    if extra_activation_pos and "activation_position" not in effect:
        effect["activation_position"] = extra_activation_pos
    _propagate_optional(effect)

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


def parse_condition(text: str) -> Dict[str, Any]:
    """Parse a condition text using priority-ordered handler cascade.

    Priority tiers:
      Tier 1 - Complex/compound patterns (most specific):
        complex, compound, distinct, dual_distinct
      Tier 2 - State changes & OR triggers (event-based):
        state_change, or, blade_count
      Tier 3 - Card counts & cost comparisons (broad numeric patterns):
        card_count, cost_override_condition, both
      Tier 4 - Temporal constraints:
        temporal_this_turn, temporal_turn_phase, baton_touch, temporal_count
      Tier 5 - Target/location/zone patterns:
        either_target, appear_or_move, movement, appearance, zone_placement
      Tier 6 - State/resource checks:
        energy_state, state, revealed, opponent_choice, unless_pay
      Tier 7 - Position, ability, and other filters:
        position_change, position, ability_filter, otherwise,
        heart_possession, live_mid
    """
    text = strip_parenthetical(text)
    # Try early-return handlers (most specific first).
    for _priority, name, handler in _condition_registry.sorted_handlers():
        result = handler(text)
        if result is not None:
            _DEBUG_LOG.append(
                f"parse_condition({text!r})\n"
                f"  [{name}]: MATCH → type={result['type']}\n"
                f"  output: {json.dumps(result, ensure_ascii=False)}"
            )
            # Extract cost_limit and card_property from text for handlers
            # that don't set these fields themselves (e.g. _try_heart_possession).
            # Only extract when the key is truly absent (not just None) to avoid
            # interfering with handlers that intentionally omit these fields.
            if "cost_limit" not in result and "cost_limit_operator" not in result:
                cl_op = extract_cost_limit_with_operator(text)
                if cl_op:
                    result["cost_limit"] = cl_op[0]
                    result["cost_limit_operator"] = cl_op[1]
            if "card_property" not in result:
                cp = detect_card_property(text)
                if cp:
                    result["card_property"] = cp[0]
                    if cp[1]:
                        result["negation"] = True
            # Add scope/aggregate fields for "自分と相手" conditions
            if "scope" not in result and "自分と相手" in text:
                result["scope"] = "both"
            if "aggregate" not in result and "合計" in text:
                result["aggregate"] = "total"
            # Add position only when exactly one keyword matches (avoids overriding
            # _try_appearance's intentional multi-position detection).
            # Skip when positions_characters already encodes per-position mappings.
            if "position" not in result and "positions_characters" not in result:
                set_cross_position_fields(result, text)
            elif (
                "position_compare" not in result
                and result.get("comparison_type") == "equality"
            ):
                # Handler already set position — check for cross-position equality
                matched = {pos for kw, pos in POSITION_KEYWORDS.items() if kw in text}
                matched.discard(result["position"])
                if matched:
                    result["position_compare"] = sorted(matched)[0]
            # require_position_cards: when text says members "are in" multiple
            # positions (e.g. "右サイドエリアと左サイドエリアにいる"), both
            # positions must have cards.  Empty slot should not count as cost 0.
            if result.get("position") and result.get("position_compare"):
                if re.search(
                    r"(?:センターエリア|左サイドエリア|右サイドエリア|センター|左サイド|右サイド).*にいる",
                    text,
                ):
                    result["require_position_cards"] = True
            # OR-location: zone1かzone2 pattern → add locations array
            _enrich_or_location(result, text)
            # Heart-content filter: 必要ハートに含まれるheartXXがN → add heart_colors
            _enrich_heart_content(result, text)
            return result

    # Fall-through: generic field extraction + type inference
    condition: Dict[str, Any] = {"text": text}
    _extract_generic_fields(condition, text)
    # Add scope for conditions that span both players
    if "scope" not in condition and "自分と相手" in text:
        condition["scope"] = "both"
    # Extract position from POSITION_KEYWORDS for fall-through conditions
    # Skip when positions_characters already encodes per-position mappings.
    if "position" not in condition and "positions_characters" not in condition:
        set_cross_position_fields(condition, text)
    elif (
        "position_compare" not in condition
        and condition.get("comparison_type") == "equality"
    ):
        matched = {pos for kw, pos in POSITION_KEYWORDS.items() if kw in text}
        matched.discard(condition.get("position"))
        if matched:
            condition["position_compare"] = sorted(matched)[0]
    # require_position_cards: same detection as handler path above
    if condition.get("position") and condition.get("position_compare"):
        if re.search(
            r"(?:センターエリア|左サイドエリア|右サイドエリア|センター|左サイド|右サイド).*にいる",
            text,
        ):
            condition["require_position_cards"] = True
    # OR-location and heart-content enrichment for fallthrough path
    _enrich_or_location(condition, text)
    _enrich_heart_content(condition, text)
    _DEBUG_LOG.append(
        f"parse_condition({text!r})\n"
        f"  → no handler matched, fallthrough\n"
        f"  output: {json.dumps(condition, ensure_ascii=False)}"
    )
    return _infer_condition_type(condition, text)

    # Parse gained ability text into structured effect
    # Note: NOT done here — done in parse_ability to avoid recursion


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
        "増える" in text or "増やす" in text or "プラス" in text or "コストを+" in text
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
    # Replace コスト with energy texticon in the action text so the
    # frontend can render an icon instead of plain kanji for the bonus.
    # Also set count from value when no explicit icon count was found
    # (the +N value represents N energy units of cost bonus).
    if "value" in action and "コスト" in action.get("text", text):
        if "count" not in action:
            action["count"] = action["value"]
        action["text"] = action.get("text", text).replace(
            "コスト", "{{icon_energy.png|E}}", 1
        )
    # "このメンバーのコストを+Nする" → self_target (modifies only the activating card)
    if "このメンバー" in text:
        action["self_target"] = True
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


def _handle_required_hearts(t, a):
    raw = [m.group(1) for m in re.finditer(r"\|(heart\d{2})}", t)]
    seen = {}
    for c in raw:
        seen[c] = seen.get(c, 0) + 1
    # Ensure equal count across all colors (sanity check; report if not)
    counts = list(seen.values())
    if counts and len(set(counts)) == 1:
        per_color = counts[0]
    else:
        per_color = counts[0] if counts else 1
    colors = list(dict.fromkeys(raw))
    a.update(
        {
            "operation": "set",
            "heart_colors": colors,
            "count": per_color,
            "value": per_color,
            "text": t,
        }
    )
    if "このカード" in t:
        a["self_target"] = True
    # Add replace_all flag to signal that unspecified colors should be cleared
    # (semantic: "cost becomes exactly these values")
    a["replace_all"] = True


def parse_action(text: str) -> Dict[str, Any]:
    """Parse an action text."""
    # Check for optional draw action "カードを1枚引いてもよい" - CHECK THIS FIRST
    if "カードを1枚引いてもよい" in text:
        return {"text": text, "action": "draw_card", "count": 1, "optional": True}

    # Strip parenthetical notes first (before any other processing)
    text = strip_parenthetical(text)

    # If text has "。" before the first "につき", the per-unit reference
    # belongs to a later sentence. Split into sequential first so the
    # earlier sentence (e.g. draw effect) isn't consumed by the per-unit match.
    if "。" in text and PER_UNIT_MARKER in text:
        first_period = text.find("。")
        first_perunit = text.find("につき")
        if 0 <= first_period < first_perunit:
            parts = [t.strip().rstrip("。") for t in text.split("。") if t.strip()]
            actions = [parse_action(p) for p in parts]
            # When the first action counts from discard (e.g. "replaced by baton touch
            # → placed in waitroom"), and a subsequent action has a per-unit type that
            # came from generic keyword matching ("メンバー" → "member"), inherit the
            # discard type. Both sentences refer to the same set of cards.
            if actions and actions[0].get("per_unit_type") in ("discard",):
                proto_type = actions[0]["per_unit_type"]
                for sub in actions[1:]:
                    if sub.get("per_unit") and sub.get("per_unit_type") in (
                        "member",
                        "枚",
                    ):
                        sub["per_unit_type"] = proto_type
            return {"text": text, "action": "sequential", "actions": actions}

    per_unit_info = None
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
                or "ライブ中のライブカード" in per_unit_text
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
            # Check for under_member location in per_unit source text
            if "メンバーの下" in per_unit_text:
                per_unit_info["location"] = "under_member"
            # When the per-unit count targets stage members (「ステージにいる...メンバー」)
            # but the effect's location is already set to "hand" (手札にある), store a
            # separate per_unit_location so the engine counts from the right zone.
            if "ステージ" in per_unit_text:
                per_unit_info["per_unit_location"] = "stage"
            # Extract card_type from per_unit source text
            if "エネルギーカード" in per_unit_text:
                per_unit_info["card_type"] = "energy_card"
                # When per-unit counts energy cards placed by this effect
                # ("これによって置いたエネルギーカード"), the count should
                # reference recently_moved cards (energy_deck destination).
                if "エネルギーデッキ" in text or "これによって置いた" in per_unit_text:
                    per_unit_info["per_unit_type"] = "energy_deck"
            elif "メンバーカード" in per_unit_text:
                per_unit_info["card_type"] = "member_card"
            # Extract card_property from per_unit source text
            if "ブレードハートを持たない" in per_unit_text:
                per_unit_info["card_property"] = "has_blade_heart"
                per_unit_info["negation"] = True
            elif "ブレードハートを持つ" in per_unit_text:
                per_unit_info["card_property"] = "has_blade_heart"
            # Infer action from text
            if "ブレードを得る" in text or "選んだブレード" in text:
                per_unit_info["action"] = "gain_resource"
                per_unit_info["resource"] = "blade"
                # Extract resource icon count
                icon_count = text.count("{{icon_blade.png|ブレード}}")
                if icon_count > 0:
                    per_unit_info["count"] = icon_count
                    per_unit_info["resource_icon_count"] = icon_count
                # Set duration if present
                if "ライブ終了時まで" in text:
                    per_unit_info["duration"] = "live_end"
            elif bool(re.search(r"ハート.*得る", text)) or "選んだハート" in text:
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
    action: Dict[str, Any] = {"text": text}
    if dur_code:
        action["duration"] = dur_code
    # Also check for duration keywords embedded in text
    if "duration" not in action:
        if "ライブ終了時まで" in text:
            action["duration"] = "live_end"
        elif "このターンの間" in text:
            action["duration"] = "this_turn"

    # Extract count, card_type, target, state_change for dispatch rules
    count = extract_count(text)
    target = extract_target(text)
    card_type = extract_card_type(text)
    state_change = extract_state_change(text)

    if per_unit_info is not None:
        action.update(per_unit_info)

    # Extract effect constraints (最小/最大/N未満にはならない/N以上にはならない)
    constraint_text = normalize_fullwidth_digits(text)
    constraint_patterns = {
        "最小": ("min", r"最小(\d+)"),
        "最大": ("max", r"最大(\d+)"),
        "未満にはならない": ("min", r"(\d+)未満にはならない"),
        "以上にはならない": ("max", r"(\d+)以上にはならない"),
    }
    for keyword, (constraint_type, pattern) in constraint_patterns.items():
        if keyword in constraint_text:
            constraint_match = re.search(pattern, constraint_text)
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
        # Special case: if source is revealed_card(s) and no count was extracted, set to 1
        elif source in ("revealed_card", "revealed_cards") and "count" not in action:
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
    cost_range = extract_cost_range(text)
    if cost_range:
        action["cost_limit_min"] = cost_range["min"]
        action["cost_limit_max"] = cost_range["max"]
    else:
        cost_limit = extract_cost_limit(text)
        if cost_limit:
            # 合計 = total/sum → use cost_total instead of cost_limit
            is_total = "合計" in text
            op = "cost_total_operator" if is_total else "cost_limit_operator"
            key = "cost_total" if is_total else "cost_limit"
            action[key] = cost_limit
            # Extract operator: 以下(<=), 以上(>=), exact(=), 未満(<), 超(>)
            if "以下" in text:
                action[op] = "<="
            elif "以上" in text:
                action[op] = ">="
            elif "未満" in text:
                action[op] = "<"
            elif "超" in text:
                action[op] = ">"
            else:
                action[op] = "="  # bare number → exact match

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
        # When putting a member into wait, check if the text requires the member
        # to currently be in active state (アクティブ状態のメンバーをウェイトにする).
        # Emit state: "active" so the engine only targets currently-active members.
        # Without this, already-waited members become candidates and the effect
        # can softlock when the opponent has no active members to choose.
        if state_change == "wait" and "アクティブ状態" in text:
            action["state"] = "active"

    if count:
        action["count"] = count
    elif "これにより引いた枚数と同じ枚数を" in text:
        action["dynamic_count"] = {"type": "drawn_cards", "reference": "previous_draw"}
    else:
        dynamic_count = extract_dynamic_count(text)
        if dynamic_count:
            action["dynamic_count"] = dynamic_count
            # If dynamic_count set for place_energy_under_member, remove fixed energy_count
            if (
                action.get("action") == "place_energy_under_member"
                and "energy_count" in action
            ):
                del action["energy_count"]

    if card_type:
        action["card_type"] = card_type

    if target:
        action["target"] = target

    # Extract position restrictions (e.g., "センター", "センターエリア")
    # Also detect cross-position patterns (e.g. "右サイドエリアと左サイドエリア")
    set_cross_position_fields(action, text)

    # Extract exclude_self for actions (e.g., "このメンバー以外の" or "「character name」以外")
    # Only for filtering actions, NOT for gain_resource/select (self-buffs)
    if (
        "このメンバー以外" in text or bool(re.search(r"ほかの.*?メンバー", text))
    ) and action.get("action") not in ("gain_resource", "select", "heart_selection"):
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
        # Re-apply exclude_group_names if already set (line 2143 may have set it
        # but group_names extraction above may re-add the excluded groups)
        exc_gns = action.get("exclude_group_names")
        if exc_gns:
            action["group_names"] = [
                g for g in action["group_names"] if g not in exc_gns
            ]

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
        elif any(
            kw in text
            for kw in (
                "常時",
                "登場",
                "起動",
                "ライブ成功時",
                "ライブ開始時",
                "ライブ中",
            )
        ):
            # Quoted text contains ability keywords even without trigger icons
            is_ability_gain = True

    if is_ability_gain:
        action["action"] = "gain_ability"
        # Populate ability_gain with the actual ability text
        if quoted_text:
            categorized = categorize_quoted_text(quoted_text)
            if categorized["abilities"]:
                trigger_match = re.search(
                    r"\{\{[^|]+\|([^}]+)\}\}", categorized["abilities"][0]
                )
                if trigger_match:
                    action["ability_gain_trigger"] = trigger_match.group(1)
                action["ability_gain"] = (
                    re.sub(
                        r"\{\{([^|]+)\|([^}]+)\}\}",
                        r"【\2】",
                        categorized["abilities"][0],
                    )
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
    # Extract quoted text from 「」 for other contexts
    quoted_text = extract_quoted_text(text)
    if quoted_text:
        categorized = categorize_quoted_text(quoted_text)
        if categorized["characters"]:
            # Set characters list for filtering (used by engine for targeting)
            action["characters"] = categorized["characters"]
            # Also set quoted_text for single character (Rust expects QuotedText struct)
            if len(categorized["characters"]) == 1:
                action["quoted_text"] = {
                    "text": categorized["characters"][0],
                    "quoted_type": "character",
                }

    # Extract target count (e.g., "1人は" → target_count=1)
    tc_match = re.search(r"(\d+)(人|枚)は", text)
    if tc_match:
        action["target_count"] = int(tc_match.group(1))

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

    _R = []

    def R(cond, act=None, setter=None):
        if isinstance(cond, ActionRule):
            _R.append(cond)
        else:
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
        "position_change",
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
                "target_count": int(re.search(r"(\d+)枚になるまで", t).group(1)),  # type: ignore
            }
        ),
    )
    R(
        lambda t: "枚になるまで" in t and ("控え室に置く" in t or "控え室に置き" in t),
        "discard_until_count",
        lambda t, a: a.update(
            {"target_count": int(re.search(r"(\d+)枚になるまで", t).group(1))}  # type: ignore
        ),
    )
    R(
        "カードを1枚引いてもよい",
        "draw_card",
        lambda t, a: a.update(
            {"count": 1, "optional": True, "source": "deck", "destination": "hand"}
        ),
    )
    # "手札から控え室に置く" + "引いた枚数" → move_cards from hand to discard
    # (NOT a draw — "引いた" refers to the number of cards drawn previously)
    R(
        lambda t: "引いた枚数" in t and "手札から" in t and "控え室に置く" in t,
        "move_cards",
        lambda t, a: a.update({"source": "hand", "destination": "discard"}),
    )
    R(
        lambda t: ("引く" in t or "引き" in t or "引い" in t)
        and "手札から控え室に置く" not in t,
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
        and a.get("destination")
        and "選ぶ" not in t
        and "選び" not in t,
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
        lambda t: "アクティブにしてもよい" in t
        or "アクティブにする" in t
        or ("アクティブにし" in t and "しない" not in t),
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
        ActionRule(
            match="ライブできない",
            action="restriction",
            setter=lambda t, a: a.update(
                {
                    "restriction_type": "cannot_live",
                    "target": "self"
                    if "自分" in t
                    else "both"
                    if "自分と相手" in t or "お互い" in t
                    else "opponent"
                    if "相手" in t
                    else None,
                }
            ),
        )
    )
    R(
        ActionRule(
            match="アクティブにしない",
            action="restriction",
            setter=lambda t, a: a.update(
                {
                    "restriction_type": "cannot_activate",
                    "target": "opponent"
                    if "相手" in t
                    else "both"
                    if "自分と相手" in t or "お互い" in t
                    else "self"
                    if "自分" in t
                    else None,
                    "phase": "active_phase" if "アクティブフェイズ" in t else None,
                }
            ),
        )
    )
    R(
        ActionRule(
            condition=lambda t: "アクティブしない" in t
            and "アクティブにしない" not in t,
            action="restriction",
            defaults={"restriction_type": "cannot_active", "delayed": True},
        )
    )
    R(
        ActionRule(
            match="バトンタッチで控え室に置けない",
            action="restriction",
            defaults={"restriction_type": "cannot_baton_touch"},
        )
    )
    R(
        ActionRule(
            match="置くことができない",
            action="restriction",
            setter=lambda t, a: a.update(
                {
                    "restriction_type": "cannot_place",
                    "destination": _extract_place_restriction_destination(t),
                }
            ),
        )
    )
    R(
        ActionRule(
            match="置けない",
            action="restriction",
            setter=lambda t, a: a.update(
                {
                    "restriction_type": "cannot_place",
                    "destination": _extract_place_restriction_destination(t),
                }
            ),
        )
    )
    R(
        ActionRule(
            match="登場できない",
            action="restriction",
            defaults={"restriction_type": "cannot_appear"},
        )
    )
    R(
        ActionRule(
            match="移動できない",
            action="restriction",
            defaults={"restriction_type": "cannot_move"},
        )
    )
    R(
        ActionRule(
            match_any=["加える", "加え"],
            exclude_any=["選ぶ", "選び"],
            action="move_cards",
            defaults={"destination": "hand"},
        )
    )
    R(
        ActionRule(
            match="ポジションチェンジ",
            action="position_change",
            setter=lambda t, a: (
                a.update({"target": extract_target(t)}),
                _handle_position_change_fields(t, a),
                a.update({"destination": "front"}) if "正面" in t else None,
                a.update({"target_member": "select"})
                if "メンバー" in t and ("1人" in t or "N人" in t)
                else None,
            )[-1],
        )
    )
    R(ActionRule(match_all=["移動させ", "エリア"], action="position_change"))
    R(ActionRule(match="移動させ", exclude="エリア", action="move_cards"))
    R(
        ActionRule(
            match_all=["エリアを移動", "ブレード"],
            action="gain_resource",
            setter=lambda t, a: a.update(
                {
                    "resource": "blade",
                    "count": t.count("{{icon_blade.png|ブレード}}") or 1,
                    "timing_condition": "moved_this_turn",
                }
            ),
        )
    )
    R(ActionRule(match_any=["移動する", "移動し"], action="position_change"))
    R(
        lambda t: (
            ("置く" in t or "置いて" in t) or ("置き" in t and "置き場" not in t)
        )
        and "選ぶ" not in t
        and "選び" not in t,
        "move_cards",
        lambda t, a: a.update({"destination": extract_destination(t)})
        if "destination" not in a
        else None,
    )
    R(
        lambda t: "ブレードを得る" in t or "選んだブレード" in t,
        "gain_resource",
        lambda t, a: a.update(
            {
                "resource": "blade",
                "count": _ic(t, "{{icon_blade.png|ブレード}}") or 1,
            }
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
    # {{icon_all.png}} must come BEFORE {{heart}}+得る to correctly set heart_type:all
    R(
        ActionRule(
            match="得る",
            condition=lambda t: "{{icon_all.png" in t,
            action="gain_resource",
            defaults={"resource": "heart", "heart_type": "all"},
            setter=lambda t, a: a.update(
                {"count": t.count("{{icon_all.png|ハート}}") or None}
            ),
        )
    )
    R(
        lambda t: ("{{heart" in t and "得る" in t)
        or bool(re.search(r"ハート.*得る", t))
        or ("選んだハート" in t and "になる" not in t),
        "gain_resource",
        lambda t, a: a.update(
            {
                "resource": "heart",
            }
        ),
    )
    R(
        lambda t: bool(re.search(r"を(すべて)?失[うい]", t))
        and "もう一度エール" not in t
        and "もう1度エール" not in t,
        "gain_resource",
        lambda t, a: a.update(
            {
                "sign": "negative",
                "resource": "surplus_heart"
                if "余剰ハート" in t or "余分ハート" in t or "それら" in t
                else "heart",
                "all": "すべて" in t or None,
            }
        ),
    )
    R(
        lambda t: "もう一度エール" in t or "もう1度エール" in t,
        "re_yell",
        lambda t, a: None
        if "できない" in t
        else (
            a.pop("lose_blade_hearts", None),
            a.pop("location", None),
            a.update(
                {
                    "action": "sequential",
                    "actions": [
                        {
                            "text": "ブレードハートを失い",
                            "action": "re_yell",
                            "lose_blade_hearts": True,
                            "target": "self",
                        },
                        {
                            "text": "もう一度エールを行う",
                            "action": "perform_yell",
                            "count": 1,
                            "target": "self",
                        },
                    ],
                }
            ),
        )
        if "ブレードハートを失い" in t
        else a.update({"lose_blade_hearts": True}),
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
        lambda t, a: a.update(
            {
                "source": source or "hand",
                **({"blind": True} if "見ないで" in t else {}),
                **({"picker": extract_picker(t)} if extract_picker(t) else {}),
            }
        ),
    )
    R(
        lambda t: "もう一度エール" in t or "もう1度エール" in t,
        "re_yell",
        lambda t, a: None
        if "できない" in t
        else (
            a.update({"lose_blade_hearts": True}),
            a.update(
                {
                    "action": "sequential",
                    "actions": [
                        {
                            "text": t.split("、")[0] if "、" in t else t,
                            "action": "re_yell",
                            "lose_blade_hearts": True,
                            "target": "self",
                        },
                        {
                            "text": t.split("、")[-1] if "、" in t else t,
                            "action": "perform_yell",
                            "count": 1,
                            "target": "self",
                        },
                    ],
                }
            )
            if "ブレードハートを失い" in t
            else (a.update({"lose_blade_hearts": True}),),
        ),
    )
    R(ActionRule(match="以下から1つを選ぶ", action="choice"))
    R(
        ActionRule(
            condition=lambda t: bool(re.search(r"数\d*つを選ぶ", t)),
            action="select_number",
        )
    )
    R(
        ActionRule(
            condition=lambda t: "選ぶ" in t
            or "選び" in t
            or bool(re.search(r"選ん(?!だ)", t)),
            action="select",
            setter=lambda t, a: a.update(
                {
                    "heart_colors": [
                        m.group(1) for m in re.finditer(r"\|(heart\d{2})}", t)
                    ]
                }
            )
            if not a.get("source") and not a.get("card_type") and "{{heart_" in t
            else None,
        )
    )
    R(
        ActionRule(
            condition=lambda t: bool(re.search(r"ハート.*得る", t))
            or ("選んだハート" in t and "になる" not in t),
            action="gain_resource",
        )
    )
    R(
        ActionRule(
            match="登場させ", action="move_cards", defaults={"destination": "stage"}
        )
    )
    R(ActionRule(match_any=["起動でき", "起動して"], action="activate_ability"))
    R(ActionRule(match="無効に", exclude="無効にできない", action="invalidate_ability"))
    R(
        ActionRule(
            match="能力は発動しない",
            action="suppress_ability_trigger",
            setter=lambda t, a: a.update(
                {
                    "suppressed_trigger": (
                        m.group(1)
                        if (m := re.search(r"\{\{(\w+)\.png\|", t[: t.find("能力")]))
                        else None
                    ),
                }
            ),
        )
    )
    R(
        ActionRule(
            match_any=["必要ハート", "ハートを増やす", "ハートを減らす"],
            action="modify_required_hearts",
        )
    )
    R(
        ActionRule(
            match="追加",
            exclude="エール",
            action="modify_score",
            defaults={"operation": "add"},
        )
    )
    R(
        ActionRule(
            match_any=["スコアを1プラス", "スコアをプラス"],
            action="modify_score",
            defaults={"operation": "add", "value": 1},
        )
    )
    R(
        ActionRule(
            match="スコアを1マイナス",
            action="modify_score",
            defaults={"operation": "remove", "value": 1},
        )
    )
    R(ActionRule(match="ブレードの色を", action="set_blade_type"))
    # "ハートをすべてheartXXにする" → set all hearts to specific color (not player choice)
    R(
        lambda t: (
            "ハートを" in t
            and "すべて" in t
            and "{{heart_" in t
            and "にする" in t
            and not "ハートの色を" in t
        ),
        "set_heart_type",
        lambda t, a: a.update(
            {
                "heart_type": (
                    f"heart{m.group(1)}"
                    if (m := re.search(r"{{heart_(\d+)\.png\|heart\d+}}", t))
                    else None
                ),
                "original_value": "元々" in t,
                "self_target": "このメンバー" in t or "このカード" in t,
                "card_type": "member_card" if "メンバー" in t else None,
            }
        ),
    )
    # "ハートの色を指定する" → specify_heart_color (separate from the actual gain)
    R(
        lambda t: "ハートの色を" in t,
        "specify_heart_color",
        lambda t, a: a.update({"choice": True, "target": "self"}),
    )
    # "ハートをXXにする" → gain_resource with heart_selection
    R(
        lambda t: "ハートを" in t
        and "にする" in t
        and not ("すべて" in t and "{{heart_" in t)
        and not "ハートの色を" in t,
        "gain_resource",
        lambda t, a: a.update({"resource": "heart", "heart_selection": True}),
    )
    # "ハートはすべてheartXXになる" / "ハートは選んだハートになる" → set_heart_type
    R(
        lambda t: "ハートは" in t and "になる" in t and not "ハートを" in t,
        "set_heart_type",
        lambda t, a: a.update(
            {
                "heart_type": (
                    f"heart{m.group(1)}"
                    if (m := re.search(r"{{heart_(\d+)\.png\|heart\d+}}", t))
                    else "selected"
                    if "選んだハート" in t
                    else None
                ),
                "original_value": "元々" in t,
                "self_target": "このメンバー" in t or "このカード" in t,
                "card_type": "member_card" if "メンバー" in t else None,
            }
        ),
    )

    # If "コスト" text contains heart icons, it's about required hearts (not energy cost)

    R(
        lambda t: ("コストを" in t or "コストが" in t or "コストは" in t)
        and "{{heart_" in t,
        "modify_required_hearts",
        _handle_required_hearts,
    )
    R(
        lambda t: "コストを" in t or "コストが" in t or "コストは" in t,
        "modify_cost",
        lambda t, a: _handle_cost_modification(t, a),
    )
    R(
        ActionRule(
            match="繰り返してもよい",
            action="repeat_procedure",
            setter=lambda t, a: a.update(
                {"max_repeats": int(re.search(r"(\d+)回", t).group(1))}
            )
            if re.search(r"(\d+)回", t)
            else None,
        )
    )
    R(ActionRule(match="何もしない", action="do_nothing"))
    R(ActionRule(condition=lambda t: t.strip() == "", action="do_nothing"))
    R(
        ActionRule(
            match_all=["{{icon_energy.png|E}}", "エネルギー"], action="pay_energy"
        )
    )
    R(ActionRule(match_any=["バトンタッチ", "baton touch"], action="play_baton_touch"))
    R(
        ActionRule(
            match="無効にできない",
            action="invalidate_ability",
            defaults={"optional": True},
        )
    )
    R(
        ActionRule(
            condition=lambda t: ("スコアは" in t or "スコアが" in t)
            and ("になる" in t or "なった" in t or "なっている" in t),
            action="modify_score",
            setter=lambda t, a: (
                a.update({"operation": "set"}),
                a.update({"value": int(m.group(1))})
                if (m := re.search(r"(\d+).*(になる|なった|なっている)", t))
                else None,
            )[-1],
        )
    )
    R(
        ActionRule(
            match="スコアを",
            action="modify_score",
            setter=lambda t, a: (_set_score_op(t, a), a)[-1],
        )
    )
    R(
        ActionRule(
            match_any=["デッキの上に置き", "デッキの上に置く"],
            action="move_cards",
            setter=lambda t, a: a.update(
                {"destination": "deck_top", "placement_order": "any_order"}
                if "好きな順番で" in t
                else {"destination": "deck_top"}
            ),
        )
    )
    R(
        ActionRule(
            match_all=["エール"], match_any=["枚数", "数"], action="modify_yell_count"
        )
    )
    R(
        lambda t: "持つ" in t and "能力" in t and "得る" in t and "すべて" in t,
        "gain_ability_from_source",
        lambda t, a: a.update(
            {
                "source_location": "under_member",
                "trigger_filter": [
                    m.group(1).split("|")[-1]
                    for m in [
                        re.search(
                            r"\{\{([^}]+)\}\}", t.split("持つ")[1].split("能力")[0]
                        )
                    ]
                    if m
                ],
                "all": True,
            }
        ),
    )
    R(
        lambda t: "得る" in t
        and any(
            kw in t
            for kw in ("能力", "常時", "ライブ成功時", "ライブ開始時", "登場", "起動")
        ),
        "gain_ability",
        lambda t, a: a.update(
            {
                "ability_gain": re.sub(r"\{\{([^|]+)\|([^}]+)\}\}", r"【\2】", t)
                .replace("を失う", "")
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
        ActionRule(
            match_all=["ライブカードセットフェイズ", "減る"],
            match_any=["上限", "枚数"],
            action="reduce_live_card_set_limit",
        )
    )
    R(
        ActionRule(
            match_any=["セット", "設定"], exclude="コスト", action="set_card_identity"
        )
    )
    R(ActionRule(match="必要ハートを選ぶ", action="choose_required_hearts"))
    R(
        ActionRule(
            match="好きな順番で",
            action="move_cards",
            defaults={"placement_order": "any_order"},
        )
    )
    R(
        ActionRule(
            match_all=[
                "必要ハートを確認する時",
                "ALLブレード",
                "任意の色のハートとして扱う",
            ],
            action="all_blade_timing",
            defaults={"timing": "check_required_hearts", "treat_as": "any_heart_color"},
        )
    )
    R(
        ActionRule(
            match_all=["すべての領域にあるこのカードは", "として扱う"],
            action="set_card_identity",
            setter=lambda t, a: a.update(
                {
                    "identities": re.findall(r"『([^』]+)』", t) or None,
                    "all_regions": True,
                }
            ),
        )
    )
    R(
        ActionRule(
            condition=lambda t: bool(re.search(r"追加で.*エール.*行", t)),
            action="perform_yell",
            setter=lambda t, a: a.update({"count": extract_count(t) or 1}),
        )
    )
    R(
        ActionRule(
            match_all=["代わりに", "置く", "場合"],
            action="conditional_alternative",
            setter=lambda t, a: a.update({"condition_text": t}),
        )
    )
    R(
        ActionRule(
            condition=lambda t: bool(re.search(r"［[^］]+ハート］", t)),
            action="gain_resource",
            setter=lambda t, a: a.update(
                {
                    "resource": "heart",
                    "heart_selection": True,
                    "heart_colors": [
                        {
                            "緑": "heart01",
                            "赤": "heart02",
                            "青": "heart03",
                            "黄": "heart04",
                            "紫": "heart05",
                            "白": "heart06",
                        }.get(re.search(r"［([^］]+)ハート］", t).group(1), "heart00")
                    ]
                    if re.search(r"［([^］]+)ハート］", t)
                    else ["heart00"],
                }
            ),
        )
    )

    # Run dispatch
    # NEW: heart + blade concurrent grant → sequential
    # Order-invariant: detects both icon types anywhere in text.
    # Excludes character-specific patterns handled earlier by _try_character_specific.
    if (
        "{{icon_blade.png|ブレード}}" in text
        and "{{heart" in text
        and "得る" in text
        and "N人が" not in text
    ):
        blade_count = text.count("{{icon_blade.png|ブレード}}")
        heart_matches = re.findall(r"\{\{heart_(\d+)\.png\|heart\d+\}\}", text)
        actions = []
        if blade_count:
            actions.append(
                {
                    "action": "gain_resource",
                    "resource": "blade",
                    "count": blade_count,
                }
            )
        if heart_matches:
            colors = sorted(set(f"heart{m.zfill(2)}" for m in heart_matches))
            actions.append(
                {
                    "action": "gain_resource",
                    "resource": "heart",
                    "heart_colors": colors,
                    "count": len(heart_matches),
                }
            )
        if actions:
            # Propagate target_count, same_name, card_type to each sub-action
            # so the engine applies effects to the correct targets.
            tc_match = re.search(r"(\d+)人", text)
            target_count = int(tc_match.group(1)) if tc_match else None
            is_same_name = "と同じ名前" in text or (
                "同じ名前" in text and "持つ" in text
            )
            card_type = _infer_card_type(text, action)
            for sub in actions:
                if target_count is not None:
                    sub["target_count"] = target_count
                if is_same_name:
                    sub["same_name"] = True
                if card_type:
                    sub["card_type"] = card_type
                _fill_defaults(sub, text)
            result = {"text": text, "action": "sequential", "actions": actions}
            _fill_defaults(result, text)
            return result
    action["action"] = "custom"
    for entry in _R:
        if isinstance(entry, ActionRule):
            if entry.matches(text, action):
                entry.apply(text, action)
                break
        else:
            cond, act, setter = entry
            try:
                if callable(cond):
                    try:
                        match = cond(text, action)
                    except TypeError:
                        match = cond(text)
                else:
                    match = cond in text
            except Exception:
                match = False  # Expected: lambda arity mismatch or type mismatch
            if match:
                action["action"] = act
                if setter:
                    try:
                        setter(text, action)
                    except Exception:
                        pass  # Expected: lambda arity mismatch or type mismatch
                break

    _fill_defaults(action, text, _cached_source=source, _cached_dest=destination)
    return action


# ======================================================================
# CONDITION MATCHING HANDLERS & HELPERS
# ======================================================================


# ====================================================================
# COMPONENT PARSING: CONDITION HANDLER CASCADE
# ====================================================================
# The condition parser (parse_condition) dispatches through the
# CONDITION_PATTERNS registry in priority order. Each handler tries to
# match a specific Japanese pattern (e.g. "N枚以上" → card_count_condition,
# "エリアを移動" → movement_condition). The first match wins. If no handler
# matches, generic field extraction runs and _infer_condition_type assigns
# the most likely type.
#
# Priority ordering is CRITICAL — more specific patterns must come first
# to prevent generic matchers from catching them incorrectly.
# ====================================================================


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
            result["distinct"] = sub.get("distinct")
            break
    # Also check compound text directly for distinct keywords
    if "コストがそれぞれ異なる" in text:
        result["distinct"] = "cost"
    elif any(kw in text for kw in ["名前が異なる", "名前の異なる", "カード名が異なる"]):
        result["distinct"] = "card_name"
    elif "グループ名が異なる" in text:
        result["distinct"] = "group_name"
    # Propagate location from first sub-condition to subsequent ones that
    # reference "その中に" (among those) — e.g. "ライブ中のカードが3枚以上あり、
    # その中に『虹ヶ咲』のライブカードを1枚以上含む" → both sub-conditions
    # should use live_card_zone.
    first_loc = None
    for sub in parsed:
        if not first_loc:
            first_loc = sub.get("location")
        elif (
            first_loc
            and not sub.get("location")
            and "その中に" in (sub.get("text") or "")
        ):
            sub["location"] = first_loc
    return result


def _try_dual_distinct(text):
    """Detect dual-distinct condition 'AとBが両方ともそれぞれ異なる' with count.

    e.g., "名前とコストが両方ともそれぞれ異なるメンバーが3人以上いる場合"
    Creates a compound (AND) condition with two card_count_condition sub-conditions,
    one for each attribute with the appropriate distinct flag.
    """
    ATTR_PAIRS = [
        ("名前", "コスト"),
        ("コスト", "名前"),
        ("名前", "グループ名"),
        ("グループ名", "名前"),
    ]
    ATTR_MAP = {
        "名前": "card_name",
        "コスト": "cost",
        "グループ名": "group_name",
        "ユニット名": "group_name",
    }
    for attr1, attr2 in ATTR_PAIRS:
        if f"{attr1}と{attr2}が両方ともそれぞれ異なる" not in text:
            continue
        cm = re.search(r"(\d+)人以上", text)
        if not cm:
            continue
        count = int(cm.group(1))
        dist1 = ATTR_MAP.get(attr1)
        dist2 = ATTR_MAP.get(attr2)
        if not dist1 or not dist2:
            continue

        def mk_sub(d):
            return {
                "type": "card_count_condition",
                "count": count,
                "operator": ">=",
                "unit": "人",
                "card_type": "member_card",
                "location": "stage",
                "target": "self",
                "distinct": d,
                "text": text,
            }

        result = {
            "type": "compound",
            "operator": "and",
            "conditions": [mk_sub(dist1), mk_sub(dist2)],
            "text": text,
        }
        tgt = extract_target(text)
        if tgt:
            result["target"] = tgt
        loc = extract_location(text)
        if loc:
            result["location"] = loc
        return result
    return None


def _try_distinct(text):
    if (
        "名前が異なる" not in text
        and "名前の異なる" not in text
        and "ユニット名がそれぞれ異なる" not in text
        and "グループ名がそれぞれ異なる" not in text
    ):
        return None
    locs = extract_locations(text)
    dist_val = "card_name"
    if "ユニット名" in text or "グループ名" in text:
        dist_val = "group_name"
    result = {
        "type": "location_condition",
        "target": "self",
        "distinct": dist_val,
        "text": text,
    }
    if locs:
        result["locations"] = locs
    else:
        result["location"] = "stage"
    # Override to revealed_cards for yell/reveal context
    if "エールにより公開された" in text or "これにより公開された" in text:
        result["location"] = "revealed_cards"
    if "エリアすべて" in text:
        result["all_areas"] = True
    m = re.search(r"(\d+)(人|枚|つ)以上(?:いる|ある)", text)
    if not m:
        m = re.search(r"(\d+)(人|枚|つ)以上", text)
    if not m:
        m = re.search(r"(\d+)人以上", text)
    if m:
        result["count"] = int(m.group(1))
        result["operator"] = ">="
        result["unit"] = m.group(2) if len(m.groups()) >= 2 and m.group(2) else "人"
    gns = extract_group_names(text)
    if gns:
        result["group_names"] = gns
    # Extract 「』-bracketed character names from のうち patterns
    cm = re.search(r"((?:「[^」]+」[とか、]? ?)+)(?:」?がいる|」?のうち)", text)
    if cm:
        names = re.findall(r"「([^」]+)」", cm.group(1))
        if names:
            result["characters"] = names
    return result


def _try_blade_count(text):
    # Strip template markers like {{icon_blade.png|ブレード}} → ブレード
    clean = re.sub(r"\{\{.*?\|([^}]+)\}\}", r"\1", text)
    for pat, op in [
        (r"ブレードが(\d+)つ以上", ">="),
        (r"ブレードの数が(\d+)以上", ">="),
        (r"ブレードが(\d+)より多い", ">"),
        (r"ブレードが(\d+)つ", ">="),
        (r"ブレードの数が(\d+)つ以上", ">="),
    ]:
        m = re.search(pat, clean)
        if m:
            return {
                "type": "card_blade_condition",
                "count": int(m.group(1)),
                "operator": op,
                "text": text,
                "source": "selected_cards",
            }
    return None


def _infer_heart_source(cond, text):
    """Set heart_source on a card_count_condition based on text mentioning
    ブレードハート in a heart-color-checking context (not 持たない/がない).
    """
    if cond.get("heart_colors") and "ブレードハート" in text:
        if "持たない" not in text and "がない" not in text:
            cond["heart_source"] = "blade"


def _infer_baton_touch(obj, text):
    """Detect バトンタッチして登場した in condition or action text and
    set baton_touch_trigger and optionally min_baton_touch_count.
    Does NOT set baton_touch_source — that field is only set by the
    existing _try_baton_touch handler which correctly distinguishes
    「X」から (character name source) vs 『X』から (group name).
    Returns True if baton_touch_trigger was added.
    """
    if "バトンタッチして登場" not in text:
        return False
    if not isinstance(obj, dict):
        return False
    # Skip compound action types — baton_touch belongs on leaf nodes only
    if obj.get("action") in ("choice", "sequential", "compound"):
        return False
    obj["baton_touch_trigger"] = True
    # Only set min_baton_touch_count on condition nodes (where count = threshold),
    # not on action nodes (where count = how many to select/target).
    if obj.get("type") in ("card_count_condition", "movement_condition", "appearance"):
        count = obj.get("count")
        if count and count > 0:
            obj["min_baton_touch_count"] = count
    return True


def _enrich_card_count_condition(result, text):
    """Post-match enrichment for card_count_condition: extracts all filter fields
    from the raw text (exclude_self, negation, character names, cost limits,
    heart colors, location, target, etc.).
    """
    # Unit → card_type inference
    u = result.get("unit", "")
    if u == "人":
        result["card_type"] = "member_card"
    # Extract exclude_self for "このメンバー以外" pattern
    if (
        "このメンバー以外" in text
        or "このカード以外" in text
        or bool(re.search(r"ほかの.*?メンバー", text))
    ):
        result["exclude_self"] = True
        result["card_type"] = "member_card"
    # Detect negation
    if "ない" in text or "いない" in text or re.search(r"がなく", text):
        result["negation"] = True
    # Detect distinct card name constraint
    if "カード名の異なる" in text or "カード名が異なる" in text:
        result["distinct"] = "card_name"
    # Extract character names
    char_m = re.search(r"「([^」]+)」の(?:メンバーカード|ライブカード)", text)
    if char_m:
        result["characters"] = [char_m.group(1)]
    elif "characters" not in result:
        bracketed = re.findall(r"「([^」]+)」", text)
        if bracketed:
            result["characters"] = bracketed
    # Same name constraint
    if "同じ名前" in text:
        result["same_name"] = True
    # Distinct cost constraint
    if "コストがそれぞれ異なる" in text:
        result["distinct"] = "cost"
    # Surplus heart → comparison_condition
    if "余剰ハート" in text:
        result["type"] = "comparison_condition"
        result["resource_type"] = "surplus_heart"
        if "相手" in text:
            result["target"] = "opponent"
        if "失っている" in text and "これにより" in text:
            result["delta"] = True
    # ALL blade property
    if "ALLブレード" in text or "{{icon_b_all.png" in text:
        result["card_property"] = "has_all_blade"
    # Revealed cards context
    if "エールにより" in text and "公開" in text:
        result["location"] = "revealed_cards"
    elif "これにより公開" in text:
        result["location"] = "revealed_cards"
    # Generic card_type/location/target (don't re-add if or_card_types is already set)
    ct = extract_card_type(text)
    if ct and "or_card_types" not in result:
        zone_keywords = ["置き場", "ゾーン"]
        if not any(kw in text for kw in zone_keywords):
            result["card_type"] = ct
    loc = extract_location(text)
    if loc:
        result["location"] = loc
    tgt = extract_target(text)
    if tgt:
        result["target"] = tgt
    # Comparison targets
    for cmp_text, cmp_tgt in COMPARISON_TARGETS.items():
        if cmp_text in text:
            result["comparison_target"] = cmp_tgt
            break
    if "comparison_target" not in result:
        for cmp_text, cmp_tgt in COMPARISON_TARGETS.items():
            if cmp_text.endswith("より") and len(cmp_text) >= 4:
                noun = cmp_text[:-2]
                if noun in text and "より" in text:
                    noun_pos = text.find(noun)
                    marker_pos = text.find("より", noun_pos + len(noun))
                    if noun_pos >= 0 and marker_pos > noun_pos:
                        result["comparison_target"] = cmp_tgt
                        break
    # Fix: when target=both AND comparison_target is set, the comparison_target
    # already handles the opponent side, so target should be self
    if result.get("target") == "both" and result.get("comparison_target"):
        result["target"] = "self"
    # Live card zone
    if "ライブ中の" in text and not result.get("location"):
        result["location"] = "live_card_zone"
    # Under member takes priority when both "下" and "エネルギー" appear
    if "メンバーの下" in text or "このメンバーの下" in text:
        if not result.get("location") and not result.get("resource_type"):
            result["location"] = "under_member"
    # Energy zone
    if (
        "エネルギー" in text
        and not result.get("location")
        and not result.get("resource_type")
    ):
        result["location"] = "energy_zone"
    # Either target
    either_result = _try_either_target(text)
    if either_result:
        if "target" in either_result:
            result["target"] = either_result["target"]
        if "location" in either_result:
            result["location"] = either_result["location"]
    # Group names
    gns = extract_group_names(text)
    if gns and "を含む" not in text:
        result["group_names"] = gns
    # Cost limit
    cl_op = extract_cost_limit_with_operator(text)
    if cl_op:
        result["cost_limit"] = cl_op[0]
        result["cost_limit_operator"] = cl_op[1]
    # Heart colors
    if "{{heart_" in text:
        hm = re.findall(r"{{heart_(\d+)\.png\|heart\d+}}", text)
        if hm:
            colors = sorted(set(f"heart{m.zfill(2)}" for m in hm))
            if not result.get("check_self"):
                result["heart_colors"] = colors
    elif "heart_colors" in result:
        del result["heart_colors"]
    # Heart source: blade (ブレードハート) vs base (default)
    _infer_heart_source(result, text)
    # Baton touch trigger (ターンにバトンタッチして登場した)
    _infer_baton_touch(result, text)
    # State (wait/active)
    if "ウェイト状態" in text:
        result["state"] = "wait"
    elif "アクティブ状態" in text:
        result["state"] = "active"
    # Phase restriction: 自分のメインフェイズ / 相手のメインフェイズ
    if "メインフェイズ" in text:
        result["phase"] = "main"
        if "自分の" in text:
            result["phase_target"] = "self"
        elif "相手の" in text:
            result["phase_target"] = "opponent"


def _try_hand_count_compound(result, text):
    """Detect 手札がN枚以下 (hand ≤ N cards) pattern within a card_count_condition
    and promote to a compound AND condition: [original_condition, hand_count_condition].
    Returns the compound condition, or None if no hand count pattern found.
    """
    hand_m = re.search(r"手札が(\d+)枚以下(の|の場)", text)
    if not hand_m:
        return None
    hand_count = int(hand_m.group(1))
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
    full_text = text
    if first_text and first_text != text:
        first_cond = parse_condition(first_text)
        if first_cond and first_cond.get("type") not in ("custom", None):
            result = first_cond
        else:
            result["text"] = first_text
            for k in (
                "card_type",
                "location",
                "target",
                "group_names",
                "card_property",
                "exclude_self",
            ):
                result.pop(k, None)
            _extract_generic_fields(result, first_text)
    return {
        "type": "compound",
        "operator": "and",
        "conditions": [result, hand_cond],
        "text": full_text,
    }


def _try_card_count(text):
    """Card count condition: N枚以上, N人以上, etc.
    Matches numeric count patterns, then enriches with filter fields.
    Also detects hand-count sub-conditions and promotes to compound.
    """
    for pat, op, unit in [
        (r"(\d+)枚以下の場合", "<=", None),
        (r"(\d+)枚以下の", "<=", None),
        (r"(\d+)枚以下", "<=", None),
        (r"(\d+)つ以上ある", ">=", None),
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
            _enrich_card_count_condition(result, text)
            compound = _try_hand_count_compound(result, text)
            return compound or result
    return None


def _try_cost_override_condition(text):
    # Pattern: "相手のXにいるすべてのメンバーのそれぞれの[属性]より[属性]が[高い/低い]メンバーが自分のYにいる"
    # Semantics: exists self member such that self[attr] > every opponent[attr]
    # (universal comparison over opponent individuals, existential over self)
    m = re.search(
        r"相手の(.+?)にいる(?:すべての|全ての)?(?:メンバー|カード)のそれぞれの(.+?)より\2が(高い|大きい|低い|小さい)(?:メンバー|カード)が自分の(.+?)にいる",
        text,
    )
    if not m:
        return None
    opp_location = m.group(1)
    comp_type_raw = m.group(2)
    self_location = m.group(4)
    op_text = m.group(3)

    comp_type_map = {
        "コスト": "cost",
        "レベル": "level",
        "スコア": "score",
    }
    comparison_type = comp_type_map.get(comp_type_raw, comp_type_raw)

    operator = ">" if op_text in ("高い", "大きい") else "<"

    return {
        "type": "all_cost_comparison_condition",
        "comparison_source": "opponent",
        "comparison_target": "self",
        "comparison_type": comparison_type,
        "operator": operator,
        "location": self_location,
        "card_type": "member_card",
        "text": text,
    }


def _try_highest_cost_on_stage(text):
    """Detect 'positionにいるメンバーが最も大きいコストを持つ' (member at position has the highest cost among stage members)."""
    m = re.search(
        r"(センターエリア|センター|左サイドエリア|左サイド|右サイドエリア|右サイド)(?:エリア)?にいる(?:メンバー|カード)が最も大きいコストを持つ",
        text,
    )
    if not m:
        return None
    pos_raw = m.group(1)
    pos_map = {
        "センターエリア": "center",
        "センター": "center",
        "左サイドエリア": "left_side",
        "左サイド": "left_side",
        "右サイドエリア": "right_side",
        "右サイド": "right_side",
    }
    position = pos_map.get(pos_raw.strip())
    if not position:
        return None
    return {
        "type": "highest_cost_on_stage_condition",
        "position": position,
        "operator": ">",
        "location": "stage",
        "target": "self",
        "card_type": "member_card",
        "text": text,
    }


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
                "trigger_event": {"type": "temporal", "event_type": cond_type},
            }
            ct = extract_card_type(text)
            if ct:
                result["card_type"] = ct
            gns = extract_group_names(text)
            if gns:
                result["group_names"] = gns
            if (
                "余剰のハートを持たずに" in text
                or "余剰ハートを持たない" in text
                or "余剰ハート" in text
            ):
                result["condition"]["no_excess_heart"] = True
            return result
    return None


def _try_phase_gate(text):
    """Simple phase gate: Xフェイズの場合/の間/に — restricts activation to a specific phase."""
    m = re.search(
        r"(?:このゲームの\d+ターン目の)?(?:自分の|相手の)?(メイン|ライブ|セット|エール|アクティブ)フェイズ(?:の場合|の間|に)",
        text,
    )
    if not m:
        return None
    phase_name = m.group(1)
    phase_map = {
        "メイン": "main",
        "ライブ": "live_phase",
        "セット": "set_phase",
        "エール": "yell_phase",
        "アクティブ": "active_phase",
    }
    phase = phase_map.get(phase_name)
    if not phase:
        return None
    result = {
        "type": "temporal_condition",
        "phase": phase,
        "text": text,
        "trigger_event": {"type": "temporal", "phase": phase},
    }
    if text.startswith("自分の"):
        result["phase_target"] = "self"
        result["trigger_event"]["phase_target"] = "self"
    elif text.startswith("相手の"):
        result["phase_target"] = "opponent"
        result["trigger_event"]["phase_target"] = "opponent"
    turn_m = re.search(r"(\d+)ターン目", text)
    if turn_m:
        result["turn_number"] = int(turn_m.group(1))
        result["trigger_event"]["turn_number"] = int(turn_m.group(1))
    return result


def _try_temporal_turn_phase(text):
    if (
        "このゲームの" not in text
        or "ターン目" not in text
        or "ライブフェイズ" not in text
    ):
        return None
    result = {
        "type": "temporal_condition",
        "phase": "live_phase",
        "text": text,
        "trigger_event": {"type": "temporal", "scope": "game", "phase": "live_phase"},
    }
    tm = re.search(r"(\d+)ターン目", text)
    if tm:
        result["turn_number"] = int(tm.group(1))
        result["trigger_event"]["turn_number"] = int(tm.group(1))
    return result


def _try_baton_touch(text):
    is_teita = "とバトンタッチしていた" in text or "とバトンタッチしていて" in text
    if not (
        "バトンタッチして登場した" in text
        or "バトンタッチして登場しており" in text
        or "バトンタッチして控え室に置か" in text
        or is_teita
    ):
        return None
    is_to_stage = "バトンタッチして登場した" in text
    te_data: Dict[str, Any] = {"type": "baton_touch", "tense": "past"}
    result = {
        "type": "movement_condition",
        "movement": "baton_touch",
        "target": "self",
        "baton_touch_trigger": True,
        "text": text,
        "trigger_event": te_data,
    }
    if is_to_stage:
        te_data["location"] = "stage"
    else:
        te_data["location"] = "discard"
    # から-form: used with "baton touch FROM" (arriving member's perspective)
    # と-form: used with "baton touch WITH" (departing member's perspective)
    sep = "と" if is_teita else "から"
    m = re.search(rf"「([^」]+)」{sep}バトンタッチ", text)
    if m:
        result["characters"] = [m.group(1)]
        te_data["source_character"] = m.group(1)
    # 「名」以外の...から/とバトンタッチ — exclude this character
    m = re.search(rf"「([^」]+)」以外の.*{sep}バトンタッチ", text)
    if m:
        result["exclude_characters"] = [m.group(1)]
        te_data["exclude_characters"] = [m.group(1)]
    m = re.search(rf"『([^』」]+)』{sep}バトンタッチ", text)
    if m:
        te_data["source_group"] = m.group(1)
        # Also add to characters if not yet set
        if "characters" not in result:
            result["characters"] = [m.group(1)]
    # と-form: also try 『X』のメンバーとバトンタッチ (group name before の)
    if is_teita:
        gns = extract_group_names(text)
        if gns:
            result["group_names"] = gns
            if "characters" not in result:
                result["characters"] = gns[:1]
    count_m = re.search(rf"(\d+)人{sep}バトンタッチ", text)
    if count_m:
        te_data["min_count"] = int(count_m.group(1))
    # Extract cost limit (e.g., "コスト10以上" → cost_limit=10, cost_limit_operator=">=")
    cl_op = extract_cost_limit_with_operator(text)
    if cl_op:
        result["cost_limit"] = cl_op[0]
        result["cost_limit_operator"] = cl_op[1]
    # Extract group_names (for から-form, this is handled below)
    gns = extract_group_names(text)
    if gns:
        result["group_names"] = gns
    # Extract card_property
    cp = detect_card_property(text)
    if cp:
        result["card_property"] = cp[0]
        if cp[1]:
            result["negation"] = True
    # Extract cost comparison (e.g. "コストが低い" → comparison_type=cost, operator=<)
    if "コスト" in text and ("低い" in text or "高い" in text):
        te_data["cost_comparison"] = {
            "operator": "<" if "低い" in text else ">",
            "relative_to": "activating",
        }
    if "このメンバー以外" in text or bool(re.search(r"ほかの.*?メンバー", text)):
        result["exclude_self"] = True
    if "能力を持たない" in text or "能力も持たない" in text:
        result["ability_filter"] = "no_ability"
    return result


def _try_temporal_count(text):
    if not (
        ("このターン" in text or "ターン目" in text)
        and ("回" in text or "登場" in text)
    ):
        return None
    result = {
        "type": "temporal_condition",
        "temporal": "this_turn",
        "text": text,
        "trigger_event": {"type": "temporal_count"},
    }
    m = re.search(r"(\d+)回", text)
    if m:
        result["count"] = int(m.group(1))
        result["trigger_event"]["count"] = int(m.group(1))
    elif "登場" in text and "回" not in text:
        result["count"] = 1
    if "ライブフェイズ" in text:
        result["phase"] = "live_phase"
    elif "メインフェイズ" in text:
        result["phase"] = "main_phase"
        result["trigger_event"]["phase"] = "main_phase"
        if "自分の" in text:
            result["phase_target"] = "self"
            result["trigger_event"]["phase_target"] = "self"
        elif "相手の" in text:
            result["phase_target"] = "opponent"
            result["trigger_event"]["phase_target"] = "opponent"
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
    gns = extract_group_names(text)
    if gns:
        result["group_names"] = gns
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


def _extract_place_restriction_destination(text):
    """Extract the destination zone for a "cannot_place" restriction.

    Looks for patterns like:
      "成功ライブカード置き場に置くことができない" -> "success_live_zone"
      "ライブカード置き場に置くことができない" -> "live_card_zone"
      "控え室に置くことができない" -> "discard"
      "手札に置くことができない" -> "hand"
      "ステージに置くことができない" -> "stage"
      "エネルギー置き場に置くことができない" -> "energy_zone"
    """
    if "成功ライブカード置き場" in text:
        return "success_live_zone"
    if "ライブカード置き場" in text:
        return "live_card_zone"
    if "控え室" in text:
        return "discard"
    if "手札" in text:
        return "hand"
    if "エネルギー置き場" in text:
        return "energy_zone"
    if "ステージ" in text:
        return "stage"
    return None


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
                result["cost_limit_operator"] = op
            return result
    return None


def _try_movement(text):
    """Area movement event: member moved between areas on stage.
    E.g. このメンバーがエリアを移動したとき (this member moved areas).
    """
    if "移動した" not in text and "移動している" not in text and "移動する" not in text:
        return None
    tense = "past" if ("移動した" in text or "移動している" in text) else "nonpast"
    te_data: Dict[str, Any] = {"type": "area_move", "tense": tense}
    if "たび" in text:
        te_data["recurrence"] = "each_time"
    is_future = (
        "移動する" in text and "移動した" not in text and "移動している" not in text
    )
    result = {
        "type": "movement_condition",
        "text": text,
        "trigger_event": te_data,
        "movement": "moves"
        if is_future
        else None,  # placeholder, set after generic fields
    }
    _extract_generic_fields(result, text)
    # Override movement: _extract_generic_fields may set "moved" from "置かれた"
    # patterns, but _try_movement handles area moves, not zone placements.
    result["movement"] = "moves" if is_future else "position_change"
    # Extract position (センター/左サイド/右サイド) and direction.
    # "センターエリアにいるメンバーがエリアを移動" → position=center, area_direction=from
    # "メンバーがセンターエリアに移動"               → position=center, area_direction=to
    if "position" not in result:
        for kw, pos in POSITION_KEYWORDS.items():
            if kw in text:
                result["position"] = pos
                if f"{kw}にいる" in text or f"{kw}にいて" in text:
                    result["area_direction"] = "from"
                break
        matched = {pos for kw, pos in POSITION_KEYWORDS.items() if kw in text}
        if len(matched) > 1:
            positions_list = sorted(matched)
            result["position_compare"] = (
                positions_list[1]
                if positions_list[0] == result["position"]
                else positions_list[0]
            )
    if "position" in result:
        te_data["position"] = result["position"]
    if "移動していない" in text:
        result["negation"] = True
    # Extract "自分のカードの効果" (own card effect) constraint
    if "自分のカードの効果" in text:
        te_data["self_effect_only"] = True
    # Extract "エネルギーが置かれ" (energy placed) trigger
    if "エネルギーが置かれ" in text:
        te_data["energy_placed"] = True
    # Phase restriction: 自分のメインフェイズ / 相手のメインフェイズ
    if "メインフェイズ" in text:
        te_data["phase"] = "main"
        if "自分の" in text:
            te_data["phase_target"] = "self"
        elif "相手の" in text:
            te_data["phase_target"] = "opponent"
    return result


def _try_zone_placement(text):
    """Zone change event: card moves from one zone to another.
    E.g. このカードが控え室から手札に加えられた (this card added from discard to hand).

    Sets flat `source`/`destination` fields on the condition for the engine's
    `evaluate_card_count_condition` routing, plus keeps `trigger_event` for
    documentary use by `movement_condition` and phase-gate evaluation.
    """
    if (
        not re.search(r"から.*?に(?:置かれ|加えられ|加わる|移され|送られ)", text)
        or "バトンタッチ" in text
    ):
        return None
    result = {
        "type": "card_count_condition",
        "count": 1,
        "operator": ">=",
        "text": text,
        "trigger_event": {
            "type": "zone_change",
        },
    }
    _extract_generic_fields(result, text)
    # Extract source zone (before から) into `source` (flat + trigger_event)
    m_from = re.search(r"から", text)
    if m_from:
        source_text = text[: m_from.start()]
        if "控え室" in source_text:
            src = "discard"
        elif "ライブカード置き場" in source_text:
            src = "live_card_zone"
        elif "エネルギー置き場" in source_text:
            src = "energy_zone"
        elif "手札" in source_text or "手元" in source_text:
            src = "hand"
        elif "ステージ" in source_text:
            src = "stage"
        else:
            src = None
        if src:
            result["source"] = src
            result["trigger_event"]["source"] = src
    # Extract destination zone (between から and the verb) into `destination` (flat + trigger_event)
    dest_match = re.search(r"から(.+?)に(?:置かれ|加えられ|加わる|移され|送られ)", text)
    if dest_match:
        dest_text = dest_match.group(1)
        if "控え室" in dest_text:
            dest = "discard"
        elif "手札" in dest_text or "手元" in dest_text:
            dest = "hand"
        elif "ステージ" in dest_text:
            dest = "stage"
        elif "デッキ" in dest_text:
            dest = "deck"
        else:
            dest = None
        if dest:
            result["destination"] = dest
            result["trigger_event"]["destination"] = dest
    # Phase restriction: 自分のメインフェイズに / 相手のメインフェイズに
    if "メインフェイズ" in text:
        result["trigger_event"]["phase"] = "main"
        if "自分の" in text:
            result["trigger_event"]["phase_target"] = "self"
        elif "相手の" in text:
            result["trigger_event"]["phase_target"] = "opponent"
    # Remove location/locations — source+destination carry the zone info
    result.pop("location", None)
    result.pop("locations", None)
    return result


def _try_live_success_or_move(text):
    if "ライブが成功するか、" not in text or "エリアを移動" not in text:
        return None
    tense = "past" if "移動した" in text else "nonpast"
    return {
        "type": "or_condition",
        "conditions": [
            {
                "type": "movement_condition",
                "movement": "live_success",
                "text": text,
                "trigger_event": {"type": "live_success", "tense": "past"},
                "self_target": True,
            },
            {
                "type": "movement_condition",
                "movement": "position_change",
                "text": text,
                "trigger_event": {"type": "area_move", "tense": tense},
                "self_target": True,
            },
        ],
        "text": text,
        "trigger_event": {
            "type": "or",
            "events": [
                {"type": "live_success", "tense": "past"},
                {"type": "area_move", "tense": tense},
            ],
        },
    }


def _try_appear_or_move(text):
    if "登場か、エリアを移動" not in text:
        return None
    tense = "past" if "移動した" in text else "nonpast"
    return {
        "type": "or_condition",
        "conditions": [
            {
                "type": "appearance_condition",
                "appearance": True,
                "location": "stage",
                "text": text,
                "trigger_event": {"type": "appearance"},
            },
            {
                "type": "movement_condition",
                "movement": "moves",
                "text": text,
                "trigger_event": {"type": "area_move", "tense": tense},
            },
        ],
        "text": text,
        "trigger_event": {
            "type": "or",
            "events": [
                {"type": "appearance"},
                {"type": "area_move", "tense": tense},
            ],
        },
    }


def _try_appearance(text):
    if "登場" not in text:
        return None
    result = {
        "type": "appearance_condition",
        "appearance": True,
        "text": text,
        "trigger_event": {"type": "appearance", "location": "stage"},
    }
    # Default to stage since abilities almost always check member appearance
    result["location"] = "stage"
    # Detect "appeared from waiting room" (控え室から登場)
    if "控え室から" in text:
        result["appearance_source"] = "discard"
        result["trigger_event"]["appearance_source"] = "discard"
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
    # Detect cost comparison: 「A」よりコストの(大きい|高い)「B」
    # Extract reference character A that B's cost is compared against.
    cost_ref_m = re.search(r"「([^」]+)」よりコストの(大きい|高い)", text)
    if cost_ref_m and subject:
        result["cost_reference_character"] = cost_ref_m.group(1)
        result["cost_reference_operator"] = (
            ">" if "大きい" in cost_ref_m.group(2) else ">"
        )
        result["cost_reference_type"] = "cost"
    if subject:
        # Detect position-specific character assignments (e.g.
        # "右サイドエリアに「大沢瑠璃乃」が、左サイドエリアに「安養寺姫芽」が")
        pos_char_pairs = re.findall(
            r"(右サイドエリア|左サイドエリア|センターエリア)に「([^」]+)」", text
        )
        if pos_char_pairs:
            result["positions_characters"] = [
                {"position": POSITION_KEYWORDS[pos], "character": char}
                for pos, char in pos_char_pairs
            ]
        all_quoted = re.findall(r"「([^」]+)」", text)
        if all_quoted:
            result["characters"] = list(dict.fromkeys(all_quoted))
        else:
            result["characters"] = [subject]
        # Remove cost reference character from characters — it's not the one appearing
        if "cost_reference_character" in result:
            ref = result["cost_reference_character"]
            result["characters"] = [c for c in result["characters"] if c != ref]
    if "エリアすべて" in text:
        result["all_areas"] = True
    # Exclude self for "このメンバー以外" / "このカード以外" patterns
    if (
        "このメンバー以外" in text
        or "このカード以外" in text
        or re.search(r"ほかの.*?メンバー", text)
    ):
        result["exclude_self"] = True
    gns = extract_group_names(text)
    if gns:
        result["group_names"] = gns
    if "バトンタッチ" in text:
        result["baton_touch_trigger"] = True
        result["trigger_event"]["baton_touch"] = True
        bts = re.search(r"「([^」]+)」からバトンタッチ", text)
        if bts:
            result["baton_touch_source"] = bts.group(1)
            result["trigger_event"]["source_character"] = bts.group(1)
        # 「名」以外の...からバトンタッチ — exclude this character
        bte = re.search(r"「([^」]+)」以外の.*からバトンタッチ", text)
        if bte:
            result["exclude_characters"] = [bte.group(1)]
        count_m = re.search(r"(\d+)人からバトンタッチ", text)
        if count_m:
            result["min_baton_touch_count"] = int(count_m.group(1))
    # Propagate target from text
    tgt = extract_target(text)
    if tgt:
        result["target"] = tgt
    # Extract cost limit (e.g. コスト10の → cost_limit=10)
    cl = extract_cost_limit(text)
    if cl is not None:
        result["cost_limit"] = cl
    # Extract card type (e.g. メンバー → member_card)
    ct = extract_card_type(text)
    if ct:
        result["card_type"] = ct
    # Extract position (左サイド/右サイド/センター) — skip when
    # positions_characters already encodes per-position character mappings.
    if "positions_characters" not in result:
        set_cross_position_fields(result, text)
    if "position" in result:
        result["trigger_event"]["position"] = result["position"]
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
    result = None
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
    if result is None:
        return None
    gns = extract_group_names(text)
    if gns:
        result["group_names"] = gns
    return result


def _try_revealed(text):
    if "エールにより公開された" not in text and "エールした" not in text:
        return None
    has_negation = "ない" in text
    result = {
        "type": "location_condition",
        "location": "revealed_cards",
        "target": "self",
        "text": text,
    }
    # "エールしたとき" (when you yell) vs "エールにより公開された" (when cards revealed by yell).
    # "エールしたとき" requires that a yell actually occurred (total_blade > 0 after modifiers).
    # If no yell was performed (e.g. all members wait), the ability should not trigger (Q264).
    if "エールしたとき" in text and "エールにより公開された" not in text:
        result["yell_trigger"] = True
    if has_negation:
        result["negation"] = True
    if "ブレードハートを持つ" in text or "ブレードハートを持たない" in text:
        result["card_property"] = "has_blade_heart"
    if "{{icon_score.png|スコア}}を持つ" in text:
        result["card_property"] = "has_score_icon"
    # "持たないカードが0枚" = cards WITHOUT property = 0.
    # This means NOT(at least 1 card has the property).
    # Set count:1 with operator >= so negation → false only when ≥1 match.
    if "0枚" in text:
        result["count"] = 1
        result["operator"] = ">="
    ct = extract_card_type(text)
    if ct:
        zone_keywords = ["置き場", "ゾーン"]
        if not any(kw in text for kw in zone_keywords):
            result["card_type"] = ct
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
        and "ポジションチェンジさせてもよい" not in text
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
    # "メンバー1人をポジションチェンジ" = pick a member, don't auto-target this_member
    if "メンバー" in text and ("1人を" in text or "1人" in text):
        # Any count-based member selection implies the player chooses the target
        result["target_member"] = "select"
    return result


def _handle_position_change_fields(text, action):
    """Set position-related fields for a position_change action.

    Distinguishes three cases:
    1. Exclude:   "センターエリア以外に" → exclude_position='center'
    2. Source:    "センターにいる" → source_position='center'
    3. Destination: default → position='center'

    Clears pre-set 'position' from the keyword loop when setting source
    or exclude, to avoid ambiguity in the engine.

    Also sets exclude_self for single-target position changes per Rule 11.10.1:
    "ポジションチェンジするとは、そのメンバーを今いるエリア以外のエリアに移動させることである。"
    """
    if "センターエリア以外" in text or "センター以外" in text:
        action["exclude_position"] = "center"
        action.pop("position", None)
    elif "センター" in text:
        if "にいる" in text:
            action["source_position"] = "center"
            action.pop("position", None)
        else:
            action["position"] = "center"

    # Rule 11.10.1: Position change MUST move to a different area.
    # Exclude the member's current position for single-target moves.
    # NOT for "メンバー1人を" (where the player selects a target first)
    # and NOT for formation change ("それぞれ" / multiple_targets).
    if "それぞれ" not in text and not (
        "メンバー" in text and ("1人" in text or "N人" in text)
    ):
        action["exclude_self"] = True


def _try_position(text):
    # If the text has condition markers or duration markers, let the fall-through handle it
    if (
        any(m in text for m in CONDITION_MARKERS)
        or "場合" in text
        or "とき" in text
        or "なら" in text
        or DURATION_MARKER in text
    ):
        return None
    for keyword in POSITION_KEYWORDS:
        if keyword in text:
            return {"type": "position_condition", "text": text}
    return None


def _try_ability_filter(text):
    if "能力も持たない" not in text and "能力を持たない" not in text:
        return None
    result = {
        "type": "ability_filter_condition",
        "text": text,
        "ability_filter": "no_ability",
    }
    # Check for specific trigger type exclusions: "能力も...能力も持たない"
    # e.g., {{live_start.png|ライブ開始時}}能力も{{live_success.png|ライブ成功時}}能力も持たない
    if "能力も" in text:
        result["ability_filter"] = "no_ability_type"
        triggers = re.findall(r"\{\{(\w+)\.png\|[^}]+\}\}能力も", text)
        if triggers:
            result["ability_filter_triggers"] = triggers
    return result


def _try_state_change(text):
    """State change event: member transitions between active/wait.
    E.g. アクティブ状態からウェイト状態になった (member went from active to wait).
    """
    if "アクティブ状態からウェイト状態になった" not in text and not (
        "アクティブ状態" in text and "ウェイト状態" in text
    ):
        return None
    result = {
        "type": "state_change_condition",
        "text": text,
        "trigger_event": {"type": "state_change"},
    }

    # Detect direction
    if "ウェイト状態になった" in text or "アクティブ状態から" in text:
        result["trigger_event"]["from_state"] = "active"
        result["trigger_event"]["to_state"] = "wait"
    else:
        # Wait → active (e.g. ウェイト状態の...アクティブ状態になった)
        result["trigger_event"]["from_state"] = "wait"
        result["trigger_event"]["to_state"] = "active"

    if "メインフェイズ" in text:
        result["trigger_event"]["phase"] = "main"
        if "自分の" in text:
            result["trigger_event"]["phase_target"] = "self"
        elif "相手の" in text:
            result["trigger_event"]["phase_target"] = "opponent"

    # Extract target (self/opponent/both)
    tgt = extract_target(text)
    if tgt:
        result["target"] = tgt

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

    # Extract generic fields (cost_limit, position, card_type, blade_limit, etc.)
    _extract_generic_fields(result, text)

    return result


def _try_otherwise(text):
    """それ以外の場合 — else/otherwise condition."""
    if "それ以外の場合" not in text:
        return None
    return {"type": "otherwise_condition", "text": text}


def _try_heart_possession(text):
    if "余剰ハート" in text:
        return None
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
    # "元々持つハートの数より多い" → compare current hearts > base hearts per member
    if "元々" in text and "ハート" in text and "より多い" in text:
        result["original_value"] = True
        result["operator"] = ">"
        result["count"] = 1
    # Exclude self for "他のメンバー" / "ほかのメンバー" / "このメンバー以外" patterns
    if (
        "このメンバー以外" in text
        or "このカード以外" in text
        or re.search(r"ほかの.*?メンバー", text)
    ):
        result["exclude_self"] = True
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
        if "ライブ中の" in text:
            result["location"] = "live_card_zone"
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


# ====================================================================
# CONDITION HANDLER REGISTRY (priority-ordered)
# ====================================================================
# Each entry: (name, priority_tier, handler_function).
# Priority tiers match the order in parse_condition's docstring.
# Handlers earlier in the list take precedence over later ones.
# Annotations show representative Japanese text → JSON type produced.
CONDITION_PATTERNS = [
    # Tier 1: Complex/compound patterns (most specific)
    ("complex", 1, _try_complex),  # nested "AかつB、CかつD" → {type: "complex"}
    (
        "compound",
        1,
        _try_compound,
    ),  # "AかつB" / "Aあり、B" → {type: "compound", operator: "and"}
    (
        "distinct",
        1,
        _try_distinct,
    ),  # "名前が異なる" → {type: "location_condition", distinct: "card_name"}
    (
        "dual_distinct",
        1,
        _try_dual_distinct,
    ),  # "名前とコストが両方ともそれぞれ異なる" → {type: "compound"}
    # Tier 2: State changes & OR triggers
    (
        "state_change",
        2,
        _try_state_change,
    ),  # "アクティブ状態→ウェイト状態" → {type: "state_change_condition"}
    ("or", 2, _try_or),  # "Aか、B" → {type: "compound", operator: "or"}
    (
        "blade_count",
        2,
        _try_blade_count,
    ),  # "ブレードがNつ以上" → {type: "card_blade_condition", count: N}
    # Tier 3: Card counts & cost comparisons
    (
        "card_count",
        3,
        _try_card_count,
    ),  # "N枚以上" / "N人以上" → {type: "card_count_condition", count: N}
    (
        "highest_cost_on_stage",
        3,
        _try_highest_cost_on_stage,
    ),  # "位置にいるメンバーが最も大きいコスト" → {type: "highest_cost_on_stage_condition"}
    (
        "cost_override_condition",
        3,
        _try_cost_override_condition,
    ),  # cost override comparison → {type: "comparison_condition"}
    ("both", 3, _try_both),  # "それらが両方ある" → {type: "both_condition"}
    # Tier 4: Temporal constraints
    (
        "temporal_this_turn",
        4,
        _try_temporal_this_turn,
    ),  # "このターン" + sub-pattern → {type: "temporal_condition", temporal: "this_turn"}
    (
        "phase_gate",
        4,
        _try_phase_gate,
    ),  # "メインフェイズの場合" → {type: "temporal_condition", phase: "main"}
    (
        "temporal_turn_phase",
        4,
        _try_temporal_turn_phase,
    ),  # turn+phase combo → {type: "temporal_condition"}
    (
        "baton_touch",
        4,
        _try_baton_touch,
    ),  # "バトンタッチして登場" → baton touch conditions
    (
        "temporal_count",
        4,
        _try_temporal_count,
    ),  # "このターンN回登場" → {type: "temporal_condition", count: N}
    # Tier 5: Target/location/zone patterns
    ("either_target", 5, _try_either_target),  # "自分か相手" → target selection
    (
        "live_success_or_move",
        5,
        _try_live_success_or_move,
    ),  # "ライブ成功時かエリアを移動" → compound OR
    (
        "appear_or_move",
        5,
        _try_appear_or_move,
    ),  # "登場するかエリアを移動" → compound OR
    ("movement", 5, _try_movement),  # "エリアを移動" → {type: "movement_condition"}
    ("appearance", 5, _try_appearance),  # "登場した" → {type: "appearance"}
    (
        "zone_placement",
        5,
        _try_zone_placement,
    ),  # "控え室に置かれた" → {type: "zone_placement_condition"}
    # Tier 6: State/resource checks
    (
        "energy_state",
        6,
        _try_energy_state,
    ),  # "エネルギーがN以下" → {type: "comparison_condition", resource_type: "energy"}
    (
        "state",
        6,
        _try_state,
    ),  # "アクティブ状態" / "ウェイト状態" → {type: "member_state"}
    ("revealed", 6, _try_revealed),  # "公開されたカード" → {type: "revealed_condition"}
    (
        "opponent_choice",
        6,
        _try_opponent_choice,
    ),  # "相手は" → opponent choice condition
    (
        "unless_pay",
        6,
        _try_unless_pay,
    ),  # "支払わないかぎり" → {type: "comparison_condition", negation: True}
    # Tier 7: Position, ability, other filters
    (
        "position_change",
        7,
        _try_position_change,
    ),  # "ポジションチェンジ" → {type: "position_change_condition"}
    ("position", 7, _try_position),  # POSITION_KEYWORDS → {type: "position_condition"}
    (
        "ability_filter",
        7,
        _try_ability_filter,
    ),  # "能力を持たない" → {type: "ability_filter_condition"}
    (
        "otherwise",
        7,
        _try_otherwise,
    ),  # "それ以外の場合" → {type: "otherwise_condition"}
    (
        "heart_possession",
        7,
        _try_heart_possession,
    ),  # "ハートを持つ/持たない" → {type: "location_condition"}
    (
        "live_mid",
        7,
        _try_live_mid,
    ),  # "ライブ中" → {type: "card_count_condition"|"temporal_condition", temporal: "during_live"}
]

# PriorityRegistry wrapping CONDITION_PATTERNS for named, sorted dispatch.
_condition_registry = PriorityRegistry("condition_patterns")
for _ci, (_cn, _ct, _ch) in enumerate(CONDITION_PATTERNS):
    _condition_registry.register(_ct * 100 + _ci, _cn, _ch)


def _extract_generic_fields(condition, text):
    """Extract all generic fields from text into condition dict (no early return)."""
    # Character names: 「A」か「B」か「C」がいる, 「A」と「B」がいる,
    # 「A」、「B」、「C」のうち (any number of names with か/と/、 separators)
    cm = re.search(r"((?:「[^」]+」[とか、]? ?)+)(?:」?がいる|」?のうち)", text)
    if cm:
        names = re.findall(r"「([^」]+)」", cm.group(1))
        if names:
            condition["characters"] = names

    # Card name filter: カード名に「DreamBelievers」を含む
    cn = re.search(r"カード名に「([^」]+)」を含む", text)
    if cn:
        condition["card_names"] = [cn.group(1)]

    # Card name exact: カード名が「EMOTION」のカード
    cn_exact = re.search(r"カード名が「([^」]+)」のカード", text)
    if cn_exact:
        condition["card_names"] = [cn_exact.group(1)]

    # Group/unit names: 『虹ヶ咲』 etc.
    gns = extract_group_names(text)
    if gns:
        condition["group_names"] = gns

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

    # If location mentions "公開" (revealed cards), prefer revealed_cards
    # over default "stage" that some handlers set
    if "公開した" in text or "公開された" in text or "公開する" in text:
        condition["location"] = "revealed_cards"

    # Ability filter: has ability / no ability
    if "能力を持つ" in text and "能力を持たない" not in text:
        condition["ability_filter"] = "has_ability"
        trig_match = re.search(r"\{\{(\w+)\.png\|([^}]+?)\}\}", text)
        if trig_match:
            condition["ability_filter_triggers"] = [trig_match.group(2)]
    elif "能力を持たない" in text or "能力も持たない" in text:
        trig_match = re.search(r"\{\{([^}]+?)\.png\|([^}]+?)\}\}能力", text)
        if trig_match:
            condition["ability_filter"] = "no_ability_type"
            condition["ability_filter_triggers"] = [trig_match.group(2)]
        elif "能力も" in text:
            condition["ability_filter"] = "no_ability_type"
            triggers = re.findall(r"\{\{([^}]+?)\.png\|([^}]+?)\}\}能力も", text)
            if triggers:
                condition["ability_filter_triggers"] = [t[1] for t in triggers]
        else:
            condition["ability_filter"] = "no_ability"

    # Delta tracking for surplus heart loss ("これにより失っている場合")
    if "失っている" in text and "これにより" in text:
        condition["delta"] = True

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
    if "エネルギー" in text:
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
    if cnt is not None:
        condition["count"] = cnt
    op = extract_operator(text)
    if op:
        condition["operator"] = op

    # Comparison targets/operators/types
    # First check contiguous matches (e.g. "自分より" as one substring),
    # then fall back to non-contiguous "Noun...より" patterns.
    # This prevents "相手の...自分より" from matching "相手" with the wrong "より".
    contiguous_found = False
    for tgt_text, tgt in COMPARISON_TARGETS.items():
        if tgt_text in text:
            condition["comparison_target"] = tgt
            contiguous_found = True
            break
    if not contiguous_found:
        for tgt_text, tgt in COMPARISON_TARGETS.items():
            if tgt_text.endswith("より") and len(tgt_text) >= 4:
                noun = tgt_text[:-2]
                if noun in text and "より" in text:
                    noun_pos = text.find(noun)
                    marker_pos = text.find("より", noun_pos + len(noun))
                    if noun_pos >= 0 and marker_pos > noun_pos:
                        condition["comparison_target"] = tgt
                        break
    # Fix: when target=both AND comparison_target is set, the comparison_target
    # already handles the opponent side, so target should be self
    if condition.get("target") == "both" and condition.get("comparison_target"):
        condition["target"] = "self"
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

    # Two-sided "self vs opponent total X" pattern (e.g. メビウスループ:
    # "自分と相手のライブの合計スコアが同じ場合"). Provide enough fields for
    # the engine to compute both sides generically.
    if "自分と相手の" in text and "合計" in text and "同じ" in text:
        condition["target"] = "self"
        condition["comparison_target"] = "opponent"
        if "成功ライブカード" in text:
            condition["location"] = "success_live_zone"
        elif "ライブカード" in text or "ライブ" in text:
            condition["location"] = "live_card_zone"
        if "スコア" in text:
            # Use "score" comparison_type so the engine sums live card
            # scores on each side generically.
            condition["comparison_type"] = "score"
            condition["resource_type"] = "score"
        elif "コスト" in text:
            condition["resource_type"] = "cost"
        # Keep operator "=" from the 同じ check below for the equality test.

    # Exact match — set operator and type. The "equality" comparison_type
    # is the default for "Xが同じ" patterns; specific two-sided patterns
    # (like メビウスループ's 合計スコアが同じ) override this in the block above.
    if "ちょうど" in text or "同じ" in text:
        condition["operator"] = "="
        if "同じ" in text and condition.get("comparison_type") != "score":
            condition["comparison_type"] = "equality"
            condition["type"] = "comparison_condition"

    # Negation (〜がない / 〜がなく / 〜が〜ない / 〜いない / 〜を持たない)
    if (
        re.search(r"がない", text)
        or re.search(r"がなく", text)
        or re.search(r"が\d*ない", text)
        or "いない" in text
        or "を持たない" in text
    ):
        condition["negation"] = True

    # Self-location check: "このカードが...にある" — condition checks if THIS SPECIFIC
    # CARD is in the zone, not just "any card". Set check_self = True to distinguish
    # from generic presence checks.
    if "このカードが" in text and re.search(r"に(ある|いる)", text):
        condition["check_self"] = True
    # check_self conditions check a specific card's location — heart_colors on
    # the condition is effect metadata leaked by the parser. Strip it here.
    if condition.get("check_self") and "heart_colors" in condition:
        del condition["heart_colors"]

    # Self-target: "このメンバーが" / "このメンバーは" / "このカードが" / "このカードは"
    # (without "以外") means the condition refers to this specific card.
    # Uses self_target (distinct from target="self" which can come from
    # extract_target's "自分の" zone-reference match).
    if re.search(r"この(メンバー|カード)[がは]", text) and "以外" not in text:
        condition["self_target"] = True

    # Includes
    if "含む" in text and "その中に" in text:
        condition["includes"] = True
        condition["includes_pattern"] = "nested"

    # Movement
    if "移動した" in text:
        condition["movement"] = "moved"
    elif "移動する" in text:
        condition["movement"] = "moves"
    # "置かれた" (was placed) — single-location self_target triggers
    # (multi-location "置かれた" like discard-from-stage use the engine's
    # 2-locations mechanism and should NOT get movement:"moved")
    if "置かれた" in text and "locations" not in condition:
        condition["movement"] = "moved"

    # Cost limit (e.g., "コスト10以上" → cost_limit=10, cost_limit_operator=">=")
    cl_op = extract_cost_limit_with_operator(text)
    if cl_op:
        condition["cost_limit"] = cl_op[0]
        condition["cost_limit_operator"] = cl_op[1]

    # Card property
    cp = detect_card_property(text)
    if cp:
        condition["card_property"] = cp[0]
        if cp[1]:
            condition["negation"] = True

    # Temporal scope
    for kw, tmp in [("このターン", "this_turn"), ("このライブ", "this_live")]:
        if kw in text:
            condition["temporal"] = tmp
            condition["temporal_scope"] = tmp
            break

    # Distinct flags
    if "コストがそれぞれ異なる" in text:
        condition["distinct"] = "cost"
    elif any(kw in text for kw in ["名前が異なる", "名前の異なる", "カード名が異なる"]):
        condition["distinct"] = "card_name"
    elif "グループ名が異なる" in text or "グループ名がそれぞれ異なる" in text:
        condition["distinct"] = "group_name"

    # All areas
    if "エリアすべて" in text:
        condition["all_areas"] = True

    # Exclude self
    has_exclude_self_kw = any(
        kw in text
        for kw in [
            "このメンバー以外",
            "このメンバー以外の",
            "このカード以外",
            "このカード以外の",
        ]
    )
    if not has_exclude_self_kw:
        has_exclude_self_kw = bool(re.search(r"ほかの.*?メンバー", text))
    if has_exclude_self_kw:
        condition["exclude_self"] = True

    # Exclude specific card names (e.g. 「MY舞☆TONIGHT」以外)
    quoted_exclusions = re.findall(r"「([^」]+)」以外", text)
    if quoted_exclusions:
        condition["exclude_characters"] = quoted_exclusions

    # Any_of values
    if "いずれか" in text:
        vm = re.search(r"(\d+)(?:、(\d+))+(?:のいずれか)", text)
        if vm:
            condition["values"] = [int(v) for v in re.findall(r"\d+", vm.group(0))]
    # Also handle "1か5" pattern (score is 1 or 5)
    vm = re.search(r"(\d+)[か](\d+)", text)
    if vm:
        condition["values"] = [int(v) for v in re.findall(r"\d+", vm.group(0))]

    # Group
    gns = extract_group_names(text)
    if gns:
        condition["group_names"] = gns
    # Exclude group (以外)
    exc_gns = re.findall(r"『([^』]+)』以外", text)
    if exc_gns:
        condition["exclude_group_names"] = exc_gns
        # Remove excluded groups from regular group_names
        if "group_names" in condition:
            condition["group_names"] = [
                g for g in condition["group_names"] if g not in exc_gns
            ]
    # Detect "のみ" (only/all members must match the group)
    if gns:
        if "のみの場合" in text or (
            "のみ" in text
            and ("ステージ" in text or "メンバー" in text or "カードが" in text)
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

    # Blade count limit for condition nodes
    if "blade_limit" not in condition and "ブレード" in text:
        bl = extract_blade_limit(text)
        if bl:
            condition.update(bl)


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
        # Only set cost_total for sum-total cost comparisons (合計), not per-card checks
        if condition.get("comparison_type") == "cost" and condition.get("count"):
            if "合計" in text:
                condition["cost_total"] = condition["count"]
        # Issue 2: Extract count and operator from "合計が、N" patterns when
        # comparison_type is "cost" and aggregate is "total". This branch fires
        # BEFORE the aggregate-only branch below, so we must extract here too.
        if (
            condition.get("comparison_type") == "cost"
            and condition.get("aggregate") == "total"
        ):
            condition.setdefault("operator", "=")
            cm = re.search(r"合計が、?(\d+)", text)
            if cm:
                condition["count"] = int(cm.group(1))
                condition["cost_total"] = int(cm.group(1))
    elif condition.get("resource_type"):
        if (
            condition.get("resource_type") == "blade"
            and condition.get("aggregate") == "total"
        ):
            condition["type"] = "resource_condition"
        else:
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
            # Also set cost_total for easier downstream access
            if "コスト" in text and condition.get("count"):
                condition["cost_total"] = condition["count"]
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


def _enrich_or_location(cond, text):
    """Detect zone1かzone2 pattern and add locations array."""
    or_m = re.search(
        r"((?:成功)?ライブカード置き場|エネルギー置き場)(?:か(?!ら)|又は).{0,30}?(?:ライブ中|エネルギー置き場)",
        text,
    )
    if or_m and cond.get("location") and "locations" not in cond:
        zone1 = or_m.group(1)
        # Extract both zone names
        full = or_m.group(0)
        parts = re.split(r"[か又は]", full)
        zones_seen = []
        for p in parts:
            p = p.strip()
            if "成功" in p:
                zones_seen.append("success_live_card_zone")
            elif "ライブカード置き場" in p:
                zones_seen.append("live_card_zone")
            elif "エネルギー置き場" in p:
                zones_seen.append("energy_zone")
            elif "ライブ中" in p:
                zones_seen.append("live_card_zone")
        if len(zones_seen) >= 2:
            cond["locations"] = zones_seen


def _enrich_heart_content(cond, text):
    """Detect 必要ハートに含まれるheartXXがN and add heart_colors + count + group_names."""
    hc_m = re.search(
        r"必要ハートに含まれる\{\{heart_(\d+)\.png\|heart\d+\}\}が(\d+)", text
    )
    if hc_m:
        heart_color = f"heart{hc_m.group(1).zfill(2)}"
        heart_count = int(hc_m.group(2))
        # Add heart_colors to the condition if not already present
        if "heart_colors" not in cond:
            cond["heart_colors"] = [heart_color]
        elif heart_color not in cond["heart_colors"]:
            cond["heart_colors"].append(heart_color)
        # Add count if not already present and operator is "="
        # (heart content = N is an exact match, not >=)
        if "count" not in cond:
            cond["count"] = heart_count
        # Extract group name from 『X』 pattern (e.g. 『虹ヶ咲』のライブカード)
        gn_m = re.search(r"『([^』]+)』", text)
        if gn_m and "group_names" not in cond:
            cond["group_names"] = [gn_m.group(1)]


# ====================================================================
# CONSOLIDATED NORMALIZATION
# ====================================================================
# parse_action() and _fill_defaults() — the main action parser.
# This is the fallback when no effect handler matches. It extracts
# source, destination, card_type, count and other fields from text,
# then _fill_defaults normalizes/validates the result.
# ====================================================================
# This section handles post-dispatch normalization of parsed actions,
# including card type inference, resource inference, and default filling.
#


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
    # Use only the effect portion (last segment after comma / duration)
    effect_text = text
    for sep in ("、", "まで", "は、"):
        if sep in text:
            parts = text.rsplit(sep, 1)
            if len(parts) == 2 and parts[1].strip():
                effect_text = parts[1].strip().lstrip("、")
                break
    # Issue 9: Prefer explicit numeric count (e.g. "2つ得る") over icon counting
    # so that "ハートを2つ得る" with a single heart icon correctly gets count=2
    count_match = re.search(r"(\d+)つ", effect_text)
    if count_match:
        d["count"] = int(count_match.group(1))
        return
    count_match = re.search(r"(\d+)つ", text)
    if count_match:
        d["count"] = int(count_match.group(1))
        return
    blade_count = effect_text.count("{{icon_blade.png|ブレード}}")
    if blade_count > 0:
        d["count"] = blade_count
        return
    all_heart_count = effect_text.count("{{icon_all.png|ハート}}")
    if all_heart_count > 0:
        d["count"] = all_heart_count
        return
    heart_count = len(re.findall(r"{{heart_\d+\.png\|heart\d+}}", effect_text))
    if heart_count > 0:
        # Check for consecutive heart icons (e.g. 4 heart06 in a row = gain 4)
        # This correctly handles "{{heart_06.png|heart06}}{{heart_06.png|heart06}}... = gain N"
        # vs "{{heart_06.png|heart06}}を持つ" (has heart06, used as condition not count)
        consecutive = re.findall(r"(?:{{heart_\d+\.png\|heart\d+}}){2,}", effect_text)
        if consecutive:
            # Use the longest consecutive run as the actual gain count
            max_run = max(
                len(re.findall(r"{{heart_\d+\.png\|heart\d+}}", run))
                for run in consecutive
            )
            d["count"] = max_run
        else:
            d["count"] = heart_count
        return


def _fill_defaults(action, text, _cached_source=None, _cached_dest=None):
    """Consolidated post-dispatch normalization. Fills defaults every action needs."""
    # Use the action's own text field if available — it may be trimmed of condition/duration
    action_text = action.get("text", text) or text
    a = action.get("action")
    # Normalize "revealed_card" (singular) to "revealed_cards" (plural) for consistency
    if action.get("source") == "revealed_card":
        action["source"] = "revealed_cards"
    if a == "draw":
        action["action"] = "draw_card"
        a = "draw_card"
    if a == "draw_card":
        action.setdefault("source", "deck")
        action.setdefault("destination", "hand")
    # Shuffle is always combined with a move action (shuffle then place).
    # If dispatch matched shuffle but text also has a destination pattern, emit move_cards with shuffle flag.
    if a == "shuffle":
        dest = _cached_dest if _cached_dest is not None else extract_destination(text)
        if dest:
            action["action"] = "move_cards"
            action["shuffle"] = True
            action["destination"] = dest
            if "source" not in action:
                s = (
                    _cached_source
                    if _cached_source is not None
                    else extract_source(text)
                )
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
        infer_count_from_icons(action, action_text)
        if action.get("count") is None:
            action["count"] = 1
        # Extract target_count from "N人" (e.g., "メンバー1人" → target_count=1)
        tc_match = re.search(r"(\d+)人", text)
        if tc_match:
            action["target_count"] = int(tc_match.group(1))
        # Extract distinct_card_name from "名前の異なる" (different name constraint)
        if "名前の異なる" in text:
            action["distinct"] = "card_name"
        # Extract same_name from "と同じ名前" / "同じ名前" (same name constraint)
        if "と同じ名前" in text or ("同じ名前" in text and "持つ" in text):
            action["same_name"] = True
    # Extract heart_colors for ALL action types, not just gain_resource
    if "heart_colors" not in action:
        hm = re.findall(r"\{\{heart_(\d+)\.png\|heart\d+\}\}", text)
        if hm:
            colors = sorted(set(f"heart{m.zfill(2)}" for m in hm))
            action["heart_colors"] = colors
            if detect_require_all_hearts(text):
                action["require_all_heart_colors"] = True
    if a == "modify_required_hearts" and "operation" not in action:
        if "減らす" in text or "減る" in text:
            action["operation"] = "decrease"
        elif "増やす" in text or "増える" in text:
            action["operation"] = "increase"
        elif "になる" in text or "にする" in text:
            action["operation"] = "set"
        else:
            action["operation"] = "decrease"
    # Exclude group names: detect 「以外」 pattern
    if "以外" in text:
        exc_gns = re.findall(r"『([^』]+)』以外", text)
        if exc_gns:
            action["exclude_group_names"] = exc_gns
            # Remove excluded groups from regular group_names if set
            if action.get("group_names"):
                action["group_names"] = [
                    g for g in action["group_names"] if g not in exc_gns
                ]
    # per_unit_type: detect heart colors vs member counts
    if a in (
        "modify_score",
        "gain_resource",
        "modify_cost",
        "perform_yell",
        "modify_required_hearts",
    ):
        if action.get("per_unit"):
            if "色につき" in text or "色に付き" in text:
                action["per_unit_type"] = "heart_colors"
            elif "コスト" in text and "につき" in text:
                action["per_unit_type"] = "cost"
                cm = re.search(r"コスト(\d+)につき", text)
                if cm:
                    action["per_unit_count"] = int(cm.group(1))
            # Issue 15: per_unit_source from "これにより控え室に置いたカード" pattern
            if "これにより" in text and ("置いた" in text or "置かれた" in text):
                action["per_unit_source"] = "previous_moved_cards"
        # Issue 15: max_repeats from "N枚までしか" / "N回までしか" patterns
        max_m = re.search(r"(\d+)(?:枚|回|つ)までしか", text)
        if not max_m:
            max_m = re.search(r"(\d+)までしか", text)
        if max_m:
            action["max_repeats"] = int(max_m.group(1))
        # Issue 6: Detect timing constraint for gain_resource
        if "このターンに登場" in text and a == "gain_resource":
            action["timing_condition"] = "appeared_this_turn"
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
            s = _cached_source if _cached_source is not None else extract_source(text)
            if s:
                action["source"] = s
        # Handle explicit "控え室から" even when extract_source missed due to
        # the action body being a sub-expression after "加える".
        if action.get("source") is None and "控え室から" in text:
            action["source"] = "discard"
        if "source" not in action:
            # Fallback: infer source from destination common patterns
            dest = action.get("destination", "")
            # "それらのカード" refers to cards from a preceding reveal/yell
            if "それらのカード" in text:
                action["source"] = "revealed_cards"
            elif "それら" in text:
                action["source"] = "selected_cards"
            elif dest in ("deck_top", "deck_bottom", "deck"):
                if "メンバー" not in text and "選ぶ" not in text and "選び" not in text:
                    action["source"] = "hand"
            elif dest in ("discard",):
                if "このカード" in text:
                    action["source"] = "deck_top"
                elif "そのカード" in text:
                    action["source"] = "looked_at"
                elif "エネルギー" not in text:
                    action["source"] = "hand"
        if "destination" not in action:
            d = _cached_dest if _cached_dest is not None else extract_destination(text)
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
        if "card_type" not in action and "or_card_types" not in action:
            ct = _infer_card_type(text, action)
            if ct:
                action["card_type"] = ct
        if "card_property" not in action:
            if "ブレードハートを持たない" in text:
                action["card_property"] = "has_blade_heart"
                action["negation"] = True
            elif "ブレードハートを持つ" in text:
                action["card_property"] = "has_blade_heart"
            elif "{{icon_score.png|スコア}}を持つ" in text:
                action["card_property"] = "has_score_icon"
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
        # AND card types: "ライブカードとメンバーカード" → split into sequential sub-actions
        if action.get("card_type") and re.search(
            r"(ライブカード|メンバーカード|エネルギーカード).*と.*(ライブカード|メンバーカード|エネルギーカード)",
            text,
        ):
            and_types = [
                t
                for kw, t in sorted(
                    [(kw, t) for t, kw in card_type_kws if kw in text],
                    key=lambda x: text.index(x[0]),
                )
            ]
            if (
                len(and_types) >= 2
                and action.get("source")
                and action.get("destination")
            ):
                sub_actions = []
                for ct in and_types:
                    sub = {
                        "text": action.get("text", ""),
                        "action": "move_cards",
                        "source": action["source"],
                        "destination": action["destination"],
                        "card_type": ct,
                        "count": action.get("count", 1),
                        "max": True,
                        "target": action.get("target", "self"),
                    }
                    if action.get("optional") is not None:
                        sub["optional"] = action["optional"]
                    sub_actions.append(sub)
                action["action"] = "sequential"
                action["actions"] = sub_actions
                action.pop("card_type", None)
                action.pop("multiple_targets", None)
        if (
            "cost_limit" not in action
            and "cost_total" not in action
            and "cost_limit_min" not in action
        ):
            cl = extract_cost_limit(text)
            if cl:
                action["cost_limit"] = cl
                if "cost_limit_operator" not in action:
                    if "以下" in text:
                        action["cost_limit_operator"] = "<="
                    elif "以上" in text:
                        action["cost_limit_operator"] = ">="
                    elif "未満" in text:
                        action["cost_limit_operator"] = "<"
                    elif "より大きい" in text:
                        action["cost_limit_operator"] = ">"
                    else:
                        action["cost_limit_operator"] = "="
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
        # Cards under a member going to energy_deck are energy cards.
        # Cards going elsewhere (e.g. empty_area for member deployment)
        # keep whatever card_type the text parsing determined.
        if action.get("destination") == "energy_deck":
            action["card_type"] = "energy_card"
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
        and action.get("source") in ("revealed_card", "revealed_cards")
        and action.get("count") is None
    ):
        action["count"] = 1
    if "non_stackable" not in action and "この効果は重複しない" in text:
        action["non_stackable"] = True
    if not action.get("all") and re.search(
        r"すべての|全ての|全部の|全て|全員|全体|カードをすべて", text
    ):
        action["all"] = True
    if (
        action.get("all")
        and action.get("action") == "invalidate_ability"
        and action.get("count") == 1
    ):
        action["all"] = False
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
            if a == "modify_required_hearts":
                # Per-color count, not total (e.g. 3 each, not 12 total)
                target_colors = action.get("heart_colors", [])
                color_counts = {}
                for m in re.finditer(r"\|(heart\d+)}", action_text):
                    h = m.group(1)
                    if not target_colors or h in target_colors:
                        color_counts[h] = color_counts.get(h, 0) + 1
                if color_counts:
                    counts = list(color_counts.values())
                    action["count"] = (
                        counts[0] if len(set(counts)) == 1 else min(counts)
                    )
            else:
                icon_count = _count_resource_icons(action_text)
                if icon_count > 0:
                    action["count"] = icon_count
            if action.get("count") is None and a in (
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
                    "置いた枚数分" in text
                    or "置いた枚数" in text
                    or bool(re.search(r"置いた.*枚数分", text))
                ):
                    action["dynamic_count"] = {
                        "type": "drawn_cards",
                        "reference": "previous_draw",
                    }
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
    # Infer source for select actions from common location patterns.
    # When multiple zones appear in text (e.g. count reference mentions stage
    # but actual selection is from waiting room), prefer the selection source.
    if a == "select" and "source" not in action:
        if "控え室にある" in text or "控え室の" in text:
            action["source"] = "discard"
        elif "手札の" in text or "手札にある" in text:
            action["source"] = "hand"
        elif "ステージにいる" in text or "ステージの" in text:
            action["source"] = "stage"
        elif "ライブ中の" in text or "ライブカード置き場" in text:
            action["source"] = "live_card_zone"
        elif "それらの中から" in text:
            action["source"] = "revealed_cards"
    # Fix target for select actions: when both "自分の" and "相手の" appear in text
    # (e.g., count reference mentions opponent but selection is from your waiting room),
    # extract_target may incorrectly return "both". Override to the correct player
    # based on the source zone's owner.
    if a == "select" and action.get("target") == "both":
        src = action.get("source", "")
        if src == "discard":
            if "自分の控え室" in text and "相手の控え室" not in text:
                action["target"] = "self"
            elif "相手の控え室" in text and "自分の控え室" not in text:
                action["target"] = "opponent"
        elif src == "stage":
            if "自分のステージ" in text and "相手のステージ" not in text:
                action["target"] = "self"
            elif "相手のステージ" in text and "自分のステージ" not in text:
                action["target"] = "opponent"
        elif src == "hand":
            if "自分の手札" in text and "相手の手札" not in text:
                action["target"] = "self"
            elif "相手の手札" in text and "自分の手札" not in text:
                action["target"] = "opponent"
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
    # If count was incorrectly extracted from blade_limit text (e.g. "3つ以下のメンバー"),
    # remove the spurious count so change_state doesn't treat it as a selection limit.
    if action.get("blade_limit") and action.get("count") == action["blade_limit"]:
        if "ブレードの数が" in text and "枚" not in text:
            action.pop("count", None)
    # Dynamic cost from revealed card (e.g. "公開したカードのコスト以下")
    if "公開したカードのコスト" in text:
        action["cost_from_revealed"] = True
        if "以下" in text and "cost_limit_operator" not in action:
            action["cost_limit_operator"] = "<="
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

    # Parse need_heart constraints like "{{heart_06.png|heart06}}を3以上含むライブカード"
    nh = re.search(r"(?:heart\d{2}|heart\d{2}[^」]*?})を(\d+)以上含む", text)
    if nh:
        # Extract the heart color from the raw text (either bare "heart06" or icon "{{heart_06.png|heart06}}")
        color_match = re.search(r"heart(\d{2})", text[: nh.end()])
        if color_match:
            action["need_heart_color"] = f"heart{int(color_match.group(1)):02d}"
            action["need_heart_total"] = int(nh.group(1))
            action["need_heart_operator"] = ">="

    # Also parse plain-text patterns like "ハートを4つ以上持つ" (without heart icon)
    nh2 = re.search(r"ハートを(\d+)つ以上持つ", text)
    if nh2:
        # Total heart icon count threshold (any colors)
        action["need_heart_total"] = int(nh2.group(1))
        action["need_heart_operator"] = ">="


# ======================================================================
# EFFECT MATCHING HANDLERS & HELPERS
# ======================================================================


# ====================================================================
# EFFECT HANDLER CASCADE
# ====================================================================
# Each handler checks a text pattern and returns a parsed effect dict.
# The first match wins. Priority ordering is CRITICAL — specific/compound
# patterns must come before generic ones.
#
# See the _EFFECT_HANDLERS list at the end of this section for the
# full priority-ordered cascade with tier grouping.
# ====================================================================
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
    if "コストは" in text and ("減る" in text or "少なくなる" in text):
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
        if "ライブ中のカード" in text or "ライブ中のライブカード" in text:
            result["per_unit_type"] = "live_card_zone"
    else:
        # Handle "コストNにつき" (cost-based scaling without explicit counter unit)
        cm = re.search(r"コスト(\d+)(につき|ごとに)", text)
        if cm:
            result["per_unit_count"] = int(cm.group(1))
            result["per_unit_type"] = "cost"
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

    # "控え室に置いた" (active) / "控え室に置かれた" (passive) = placed in waitroom → count in discard
    if "控え室に置" in per_text:
        result["per_unit_type"] = "discard"

    # "これによって置いたエネルギーカード" = energy cards placed by this effect
    # → count from recently_moved_cards (energy_deck destination)
    if "これによって置いた" in per_text and "エネルギーカード" in per_text:
        result["per_unit_type"] = "energy_deck"

    # Check for excluded groups: 『group』以外
    exc_gns = re.findall(r"『([^』]+)』以外", per_text)
    if exc_gns:
        result["exclude_group_names"] = exc_gns
    # Extract exclude_heart_colors from per_text
    # Pattern: "{{heart_01.png|heart01}}と{{heart_06.png|heart06}}以外の色のハートを持つ"
    if "以外" in per_text:
        before_igai = per_text.split("以外")[0]
        hc_ids = re.findall(r"heart_(\d+)", before_igai)
        if hc_ids:
            result["exclude_heart_colors"] = [
                f"heart{m.zfill(2)}" for m in dict.fromkeys(hc_ids)
            ]
    # Extract included groups only (groups NOT followed by 以外)
    remaining_per_text = re.sub(r"『[^』]+』以外", "", per_text)
    gm = re.search(r"『([^』]+)』", remaining_per_text)
    if gm:
        result["group_names"] = [gm.group(1)]
    # Extract heart colors from per_text for per-unit counting (e.g. heart03 from "そのメンバーが持つ{{heart_03.png|heart03}}2つ")
    # Exclude heart colors in exclusion patterns (e.g. "heart01とheart06以外の色" — those are excluded colors, not counted)
    per_heart_matches = re.findall(r"\{\{heart_(\d+)\.png\|heart(\d+)\}\}", per_text)
    if per_heart_matches:
        colors = sorted(set(f"heart{m.zfill(2)}" for _, m in per_heart_matches))
        # Remove colors that appear in "以外" exclusion context
        igai_before = per_text.split("以外")[0] if "以外" in per_text else ""
        excluded_colors = set()
        if igai_before:
            exc_matches = re.findall(
                r"\{\{heart_(\d+)\.png\|heart(\d+)\}\}", igai_before
            )
            excluded_colors = set(f"heart{m.zfill(2)}" for _, m in exc_matches)
        counted_colors = [c for c in colors if c not in excluded_colors]
        if counted_colors:
            result["per_unit_heart_colors"] = counted_colors

    if "名前の異なる" in per_text or "カード名の異なる" in per_text:
        result["distinct"] = "card_name"

    # Extract card_property from per_text (e.g. "ブレードハートを持たない")
    if "ブレードハートを持たない" in per_text:
        result["card_property"] = "has_blade_heart"
        result["negation"] = True
    elif "ブレードハートを持つ" in per_text:
        result["card_property"] = "has_blade_heart"

    # Extract exclude_self from per_text (self-referential "other" patterns)
    if (
        "このメンバー以外" in per_text
        or "このカード以外" in per_text
        or "自分以外" in per_text
        or re.search(r"ほかの.*?(?:メンバー|カード)", per_text)
        or re.search(r"他の.*?(?:メンバー|カード)", per_text)
        or "これを除く" in per_text
    ):
        result["exclude_self"] = True

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
        result["timing_condition"] = "moved_this_turn"

    if "ウェイト状態" in per_text:
        result["state"] = "wait"
    elif "アクティブ状態" in per_text:
        result["state"] = "active"

    # Extract target from per_text
    tgt = extract_target(per_text)
    if tgt:
        result["target"] = tgt

    # Extract card_type from per_text
    if "エネルギーカード" in per_text:
        result["card_type"] = "energy_card"
    elif "メンバーカード" in per_text:
        result["card_type"] = "member_card"
    # "そのメンバー" (that member) targets a stage member, not energy cards.
    # Override card_type to member_card for targeting — per_unit counting
    # uses per_unit_type (e.g. "energy_deck") independently.
    if per_text.startswith("そのメンバー"):
        result["card_type"] = "member_card"

    for kw, loc in [
        ("成功ライブカード置き場にある", "success_live_zone"),
        ("メンバーの下にある", "under_member"),
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

    # Sequential pattern in action (Aし、B) — comma-separated
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

    # Sequential pattern in action: Aし(て)B — te-form without comma
    # (e.g. コストを+4してheart05を得る)
    if "して" in action_text:
        idx = action_text.find("して")
        left = action_text[:idx].rstrip()
        right = action_text[idx + 2 :].strip().lstrip("、")
        if left and right:
            fa = parse_action(left)
            sa = parse_action(right)
            if fa.get("action", "custom") not in ("custom", "do_nothing") and sa.get(
                "action", "custom"
            ) not in ("custom", "do_nothing"):
                _propagate(result, fa)
                _propagate(result, sa)
                return {"text": text, "action": "sequential", "actions": [fa, sa]}

    action = parse_action(action_text)
    _propagate(result, action)
    # Propagate resource_icon_count from the parsed action back to result,
    # so it reaches sub-actions via the sequential propagation below.
    if (
        "resource_icon_count" not in result
        and action.get("count")
        and action.get("action") in ("gain_resource", "gain_heart")
    ):
        result["resource_icon_count"] = action["count"]
    # When action is a sequential, propagate per-unit config into each sub-action
    # so the engine can resolve per-unit counts for each sub-action individually.
    # Exclude condition — it should remain on the sequential wrapper only.
    if action.get("action") == "sequential":
        first_put = None
        for sub in action.get("actions", []):
            for k in (
                "per_unit",
                "per_unit_count",
                "per_unit_type",
                "per_unit_heart_colors",
                "card_type",
                "group_names",
                "exclude_group_names",
                "exclude_heart_colors",
                "distinct",
                "timing_condition",
                "state",
                "location",
                "cost_limit",
                "cost_limit_operator",
                "duration",
                "target",
                "exclude_self",
                "card_property",
                "negation",
                "resource_icon_count",
            ):
                if k in result and k not in sub:
                    sub[k] = result[k]
            if first_put is None and sub.get("per_unit_type"):
                first_put = sub
        # When the first per-unit sub-action counts from discard (e.g. replaced by
        # baton touch → placed in waitroom), propagate to subsequent per-unit
        # sub-actions that only have a generic ("member"/"枚") per_unit_type.
        # Both sub-effects refer to the same set of cards.
        if first_put and first_put.get("per_unit_type") in ("discard",):
            proto_type = first_put["per_unit_type"]
            for sub in action.get("actions", []):
                if (
                    sub is not first_put
                    and sub.get("per_unit")
                    and sub.get("per_unit_type") in ("member", "枚")
                ):
                    sub["per_unit_type"] = proto_type

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
                        _propagate(result, spa, skip_existing=True)
                        sub_actions.append(spa)
                if len(sub_actions) >= 2:
                    fa = {"action": "sequential", "actions": sub_actions}
                    _propagate(result, fa, skip_existing=True)
                else:
                    fa = parse_action(fa_text)
                    _propagate(result, fa)
            else:
                fa = parse_action(fa_text)
                _propagate(result, fa)
            sa = parse_action(parts[1].strip())
            return {"text": text, "action": "sequential", "actions": [fa, sa]}

    # Issue 15: Extract per_unit_source from "これにより控え室に置いた" patterns
    if "これにより" in text and ("置いた" in text or "置かれた" in text):
        action["per_unit_source"] = "previous_moved_cards"
    # Issue 15: Extract max_repeats from "N枚/回/つまでしか" patterns
    max_m = re.search(r"(\d+)(?:枚|回|つ)までしか", text)
    if not max_m:
        max_m = re.search(r"(\d+)までしか", text)
    if max_m:
        action["max_repeats"] = int(max_m.group(1))

    action["text"] = text
    return action


_PROPAGATE_FIELDS = (
    "per_unit",
    "per_unit_count",
    "per_unit_type",
    "per_unit_heart_colors",
    "card_type",
    "group_names",
    "exclude_group_names",
    "exclude_heart_colors",
    "distinct",
    "timing_condition",
    "state",
    "location",
    "cost_limit",
    "cost_limit_operator",
    "duration",
    "condition",
    "target",
    "exclude_self",
    "card_property",
    "negation",
    "resource_icon_count",
)


def _propagate(src, dst, skip_existing=False):
    """Copy common per-unit fields from src to dst.

    If skip_existing is True, only copy fields not already present in dst.
    """
    for k in _PROPAGATE_FIELDS:
        if k in src and (not skip_existing or k not in dst):
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
    # If 代わりに is inside quoted text that ends with 」を得る (gain_ability
    # pattern), skip — the inline gain_ability handler in parse_action will
    # process it and the inner effect will be parsed from the quoted text.
    alt_pos = text.find(ALTERNATIVE_MARKER)
    qs = text.rfind("「", 0, alt_pos)
    qe = text.find("」", qs) if qs >= 0 else -1
    if qs >= 0 and qe >= 0 and alt_pos > qs and alt_pos < qe:
        after = text[qe + 1 :].strip()
        if after.startswith("を得る") or after.startswith("を得る。"):
            return None
    parts = text.split(ALTERNATIVE_MARKER, 1)
    if len(parts) != 2:
        return None
    primary_text = parts[0].strip()
    result = {
        "text": text,
        "action": "conditional_alternative",
        "alternative_effect": parse_action(parts[1].strip()),
    }
    ct, at = split_condition_action(primary_text)
    if ct:
        if "成功ライブカード置き場に置く" in ct:
            result["condition"] = {
                "type": "location_condition",
                "location": "success_live_card_zone",
                "card_type": "live_card",
                "target_event": "placing_in_success_zone",
                "text": ct,
            }
        else:
            cond = parse_condition(ct)
            if cond and cond.get("type") != "custom":
                result["condition"] = cond
    if at:
        # Check for secondary condition in action text (e.g. "2枚以上いる場合")
        # This handles the "代わりに" (instead of) pattern where a stricter
        # tiered condition qualifies the alternative effect.
        # Also handle "それらが両方ある場合" (both exist) pattern.
        if "両方" in at:
            sec_text = "それらが両方ある場合"
            alt_cond = parse_condition(sec_text)
            if alt_cond and alt_cond.get("type") != "custom":
                if "condition" in result:
                    for key in (
                        "location",
                        "group_names",
                        "target",
                        "position",
                        "values",
                    ):
                        if key in result["condition"] and key not in alt_cond:
                            alt_cond[key] = result["condition"][key]
                result["alternative_condition"] = alt_cond
            at = at.replace(sec_text, "").strip().strip("、").strip()
        secondary_m = re.search(r"([^、。]*\d+)枚以上[いあ]る場合", at)
        if secondary_m:
            sec_text = secondary_m.group(0)
            alt_cond = parse_condition(sec_text)
            if alt_cond and alt_cond.get("type") != "custom":
                # Inherit location/group/position context from the main condition
                # since the secondary text ("2枚以上ある場合") omits the zone/group
                if "condition" in result:
                    for key in ("location", "group_names", "target", "position"):
                        if key in result["condition"] and key not in alt_cond:
                            alt_cond[key] = result["condition"][key]
                result["alternative_condition"] = alt_cond
            at = at.replace(sec_text, "").strip().strip("、").strip()
        result["primary_effect"] = parse_action(at)
    # When an alternative_condition exists, the base "N枚" in the primary
    # condition means "at least N" (not exactly N), since a stricter tier
    # overrides it.  Change operator from "=" to ">=".
    if "alternative_condition" in result and "condition" in result:
        cond = result["condition"]
        if cond.get("operator") == "=" and cond.get("type") == "card_count_condition":
            cond["operator"] = ">="
    return result


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
        # Build gain_resource actions per character, then group by character
        per_char_actions = []
        for eff in effects:
            char_acts = []
            resources_text = eff["resources"]
            heart_m = re.search(r"heart_(\d+)", resources_text)
            blade_count = resources_text.count("icon_blade.png")
            heart_color = f"heart{heart_m.group(1)}" if heart_m else None
            if blade_count > 0:
                char_acts.append(
                    {
                        "action": "gain_resource",
                        "resource": "blade",
                        "count": blade_count,
                        "characters": [eff["character"]],
                        "card_type": "member_card",
                        "target": "self",
                        "target_count": eff["count"],
                    }
                )
            if heart_color:
                char_acts.append(
                    {
                        "action": "gain_resource",
                        "resource": "heart",
                        "heart_color": heart_color,
                        "count": 1,
                        "characters": [eff["character"]],
                        "card_type": "member_card",
                        "target": "self",
                        "target_count": eff["count"],
                    }
                )
            if len(char_acts) == 1:
                per_char_actions.append(char_acts[0])
            else:
                per_char_actions.extend(char_acts)
        if len(per_char_actions) == 1:
            result = per_char_actions[0]
        else:
            result = {
                "action": "sequential",
                "actions": per_char_actions,
            }
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
    action = parse_effect(action_text)
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
    if "につき" not in text or ("減る" not in text and "少なくなる" not in text):
        return None
    if "。" in text:
        parts = text.split("。", 1)
        if len(parts) == 2:
            first, second = parts[0].strip(), parts[1].strip()
            if any(p in second for p in cost_prefixes):
                op = "subtract"
                # Parse the first part as the main effect and the second as a
                # modify_cost modifier, then combine them as sequential so both
                # are preserved in the output.
                main_effect = parse_effect(first)
                cost_mod = _make_cost_mod_action(second, op)
                if main_effect and main_effect.get("action", "custom") not in (
                    "custom",
                    "do_nothing",
                ):
                    return {
                        "text": text,
                        "action": "sequential",
                        "actions": [main_effect, cost_mod],
                    }
                # Fallback: if main effect didn't parse, return just cost mod
                return _make_cost_mod_action(second, op)
    value_match = re.search(r"(\d+)(少なくなる|減る|増える|増やす)", text)
    value = int(value_match.group(1)) if value_match else energy_count
    result = {
        "action": "modify_cost",
        "operation": "subtract",
        "text": text,
        "value": value,
    }
    # Location: "手札にある" → hand, and emit a condition so the engine
    # only activates this effect when the card is actually in hand.
    if "手札" in text:
        result["location"] = "hand"
        if "手札にある" in text or "手札にいる" in text:
            result["condition"] = {
                "type": "location_condition",
                "location": "hand",
                "card_type": "member_card",
                "target": "self",
                "text": "手札にある",
            }
    # Exclude self: "このカード以外" or "ほかの"
    if "以外" in text or "ほかの" in text or "他の" in text:
        result["exclude_self"] = True
    # Per-unit scaling: "X枚につき" or "X人につき"
    unit_match = re.search(r"(\d+)(枚|人)につき", text)
    if unit_match:
        result["per_unit"] = True
        result["per_unit_count"] = int(unit_match.group(1))
        result["per_unit_type"] = unit_match.group(2)
        # Detect when per-unit count targets stage members
        # (「ステージにいる...メンバー」) vs the effect's location
        # (e.g. "手札にある" → hand).
        if "ステージ" in text:
            result["per_unit_location"] = "stage"
    return result


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
    trigger_cond = None
    if "か、" in trigger_text:
        or_cond = _try_or(trigger_text)
        if or_cond:
            trigger_cond = or_cond
    if trigger_cond is None:
        trigger_cond = parse_condition(trigger_text)
    if trigger_cond and trigger_cond.get("type") != "custom":
        if (
            trigger_cond.get("type") == "card_count_condition"
            and trigger_cond.get("location") in ("discard",)
            and "source" not in trigger_cond
        ):
            trigger_cond["source"] = "preceding_moved"
        # Merge into condition (previously trigger_condition — unified).
        # If condition already exists (from the action part), combine via compound AND.
        existing = sub.get("condition")
        if existing and isinstance(existing, dict):
            sub["condition"] = {
                "type": "compound",
                "operator": "and",
                "conditions": [existing, trigger_cond],
                "text": existing.get("text", "")
                + "、かつ、"
                + trigger_cond.get("text", ""),
            }
        else:
            sub["condition"] = trigger_cond
    return sub


def _try_opponent_action(text):
    """相手は — opponent action patterns (with or without comma).

    Flattens the opponent_action wrapper — the inner action gets target="opponent"
    and action_by="opponent" so the engine handles it directly via ActionType
    dispatch instead of the legacy inline handler in effects/mod.rs.

    Also strips condition-related fields (condition, group_names) from the
    flattened action because those are trigger-level metadata, not effect filters.
    """
    if not text.startswith("相手は"):
        return None
    om = re.match(r"相手は[、]?(.+?)(?:。|$)", text)
    if not om:
        return None
    oa_text = om.group(0)
    rest = text[len(oa_text) :].strip()
    oa = parse_action(om.group(1).strip())
    oa["text"] = oa_text
    oa["target"] = "opponent"
    oa["action_by"] = "opponent"
    # Strip trigger-level fields that don't belong on the effect
    oa.pop("condition", None)
    oa.pop("group_names", None)
    if rest:
        re_eff = parse_effect(rest)
        return {"text": text, "action": "sequential", "actions": [oa, re_eff]}
    return oa


def _try_choose_self_opponent(text):
    """自分か相手を選ぶ。— choose self or opponent, then execute effect on chosen player."""
    if not text.startswith("自分か相手を選ぶ。"):
        return None
    rest = text[len("自分か相手を選ぶ。") :].strip()
    inner = parse_effect(rest)
    return {
        "text": text,
        "action": "choose_target_player",
        "choice_options": ["自分", "相手"],
        "effect_steps": [inner],
    }


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
    condition = parse_condition(cond_text)
    # The "これにより" refers to the card just moved by the preceding cost
    # (discard from hand). Check it via preceding_moved, not the discard pile.
    if condition and condition.get("location") == "discard":
        condition["source"] = "preceding_moved"
        condition.pop("location", None)
        # Branch logic: TRUE → alternative_effect, FALSE → primary_effect.
        # The primary_effect is the named/gated case (μ's), so negate so
        # that μ's → FALSE → primary, non-μ's → TRUE → alternative.
        condition["negation"] = True
    return {
        "text": text,
        "action": "conditional_alternative",
        "condition": condition,
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
        if detect_require_all_hearts(select_text):
            result["require_all_heart_colors"] = True
    if extract_max(select_text):
        result["max"] = True
    if extract_optional(select_text):
        result["optional"] = True
    gns = extract_group_names(select_text)
    if gns:
        result["group_names"] = gns
    char_names = re.findall(r"「([^」]+)」", select_text)
    if char_names:
        result["characters"] = list(dict.fromkeys(char_names))
    cl = extract_cost_limit(select_text)
    if cl:
        result["cost_limit"] = cl
    op = extract_operator(select_text)
    if op:
        result["cost_limit_operator"] = op
    pg = re.search(r"各グループ名につき(\d+)枚ずつ", select_text)
    if pg:
        result["per_group"] = True
        result["per_group_count"] = int(pg.group(1))
    _add_or_card_types_if_needed(result, select_text)
    return result


def _add_or_card_types_if_needed(d, text):
    """If text describes an OR between card types (e.g. メンバーカードか...ライブカード),
    add or_card_types to the dict and remove the single card_type."""
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
            d["or_card_types"] = or_types
            d.pop("card_type", None)


def _add_heart_color_threshold(d, text):
    """Extract heart color count threshold from patterns like 'heart05を2個以上' or 'heart05を2以上'.
    Stores as heart_color_count on the dict."""
    if "heart_color_count" in d or "heart_colors" not in d:
        return
    m = re.search(
        r"\{\{heart_(\d+)\.png\|heart\d+\}\}を(\d+)(?:個)?以上",
        text,
    )
    if m:
        count = int(m.group(2))
        if count > 0:
            d["heart_color_count"] = count


def _enrich_from_text(d, text):
    """Add common fields (count, max, card_type, heart_colors, optional, group_names, cost_limit) from text."""
    c = extract_count(text)
    if c:
        d["count"] = c
    if extract_max(text):
        d["max"] = True
    ct = extract_card_type(text)
    if ct and "or_card_types" not in d:
        d["card_type"] = ct
    hc = list(
        dict.fromkeys(f"heart{m.zfill(2)}" for m in re.findall(r"heart_(\d+)", text))
    )
    if hc:
        d["heart_colors"] = hc
        _add_heart_color_threshold(d, text)
        if detect_require_all_hearts(text):
            d["require_all_heart_colors"] = True
    if extract_optional(text):
        d["optional"] = True
    gns = extract_group_names(text)
    if gns:
        d["group_names"] = gns
    # Extract 「」-bracketed character names (e.g. 「朝香果林」のメンバーカード)
    char_names = re.findall(r"「([^」]+)」", text)
    if char_names:
        d["characters"] = list(dict.fromkeys(char_names))
    cl = extract_cost_limit(text)
    if cl:
        d["cost_limit"] = cl
    op = extract_operator(text)
    if op:
        d["cost_limit_operator"] = op
    # Dynamic cost from revealed card (e.g. "公開したカードのコスト以下")
    if "公開したカードのコスト" in text:
        d["cost_from_revealed"] = True
        if "以下" in text and "cost_limit_operator" not in d:
            d["cost_limit_operator"] = "<="

    # Extract ability filters (e.g. "{{live_start.png|ライブ開始時}}能力を持つ")
    if "能力を持つ" in text and "能力を持たない" not in text:
        d["ability_filter"] = "has_ability"
        trig_match = re.search(r"\{\{(\w+)\.png\|", text)
        if trig_match:
            d["ability_filter_triggers"] = [trig_match.group(1)]
    elif "能力を持たない" in text or "能力も持たない" in text:
        trig_match = re.search(r"\{\{([^}]+?)\.png\|([^}]+?)\}\}能力", text)
        if trig_match:
            d["ability_filter"] = "no_ability_type"
            d["ability_filter_triggers"] = [trig_match.group(2)]
        elif "能力も" in text:
            d["ability_filter"] = "no_ability_type"
            triggers = re.findall(r"\{\{([^}]+?)\.png\|([^}]+?)\}\}能力も", text)
            if triggers:
                d["ability_filter_triggers"] = [t[1] for t in triggers]
        else:
            d["ability_filter"] = "no_ability"


def _apply_or_select_criteria(result, select_text):
    m = re.search(
        r"([^か。：\n]+)か([^か。：\n]+)(?=を(?:1枚|\d+枚|好きな枚数|すべて|公開|手札|デッキ|控え室|戻す))",
        select_text,
    )
    if not m:
        return
    part1, part2 = m.group(1).strip(), m.group(2).strip()

    # Clean up part1 and part2 (remove leading context fragments)
    for prefix in (
        "その中から",
        "自分のステージにいる",
        "自分の手札にある",
        "控え室にある",
    ):
        if part1.startswith(prefix):
            part1 = part1[len(prefix) :].strip()

    parts = [part1, part2]
    parsed_parts = []
    has_ability_filter = False

    for p in parts:
        p_dict = {}
        gns = extract_group_names(p)
        if gns:
            p_dict["group_names"] = gns
        ct = extract_card_type(p)
        if ct:
            p_dict["card_type"] = ct
        if "ブレードハートを持つ" in p:
            p_dict["card_property"] = "has_blade_heart"
        elif "ブレードハートを持たない" in p:
            p_dict["card_property"] = "has_blade_heart"
            p_dict["negation"] = True

        if "能力を持たない" in p or "能力も持たない" in p:
            p_dict["ability_filter"] = "no_ability"
            has_ability_filter = True
        elif "能力を持つ" in p:
            p_dict["ability_filter"] = "has_ability_type"
            has_ability_filter = True
            trig_list = re.findall(r"【([^】]+)】", p)
            if not trig_list:
                trig_list = re.findall(r"\{\{[^|]+\|([^}]+)\}\}", p)
            if trig_list:
                p_dict["ability_filter_triggers"] = trig_list

        parsed_parts.append(p_dict)

    if has_ability_filter:
        or_filters = []
        for p_dict in parsed_parts:
            f = {}
            if "ability_filter" in p_dict:
                f["ability_filter"] = p_dict["ability_filter"]
            if "ability_filter_triggers" in p_dict:
                f["ability_filter_triggers"] = p_dict["ability_filter_triggers"]
            or_filters.append(f)
        result["or_ability_filters"] = or_filters
    else:
        # Only add options when parts have meaningful discriminating criteria
        # (group_names or card_property). Differences only in card_type are already
        # captured in or_card_types and don't need a separate options list.
        meaningful = any(
            "group_names" in p or "card_property" in p for p in parsed_parts
        )
        if meaningful:
            result["options"] = parsed_parts


def _build_or_destination_followup(select_text):
    """Generate a followup_action for 'debut to stage OR add to hand' pattern.

    The select_action handles the default (destination=hand). The followup
    presents an optional choice to debut the selected card to an empty stage
    area instead, using a move from hand → stage.
    """
    result = {
        "action": "choice",
        "optional": True,
        "text": select_text,
        "options": [
            {
                "action": "move_cards",
                "source": "hand",
                "destination": "stage",
                "card_type": "member_card",
                "count": 1,
                "text": "自分のステージのメンバーのいないエリアに登場させる",
            }
        ],
    }
    return result


def _build_look_select_actions(select_text):
    res = _build_look_select_actions_inner(select_text)
    if isinstance(res, dict):
        _apply_or_select_criteria(res, select_text)
    return res


def _build_look_select_with_followup(select_text, effect_result):
    """Build select_action and promote followup_action to the effect level."""
    sa = _build_look_select_actions(select_text) or {}
    effect_result["select_action"] = sa
    if isinstance(sa, dict) and "followup_action" in sa:
        effect_result["followup_action"] = sa.pop("followup_action")


def _build_look_select_actions_inner(select_text):
    """Build the select_action for その中から patterns."""
    result = {"action": "select_cards", "discard_remaining": True}
    result["text"] = select_text

    # Pattern: reveal → add → discard
    if "手札に加え" in select_text and "残りを控え室に置く" in select_text:
        parts = re.split(r"[、。]", select_text)
        if len(parts) >= 2:
            fp = parts[0].strip()
            if "公開し" in fp:
                act = _build_reveal_add_discard(fp, parts[1].strip(), select_text)
                if act:
                    return act
            if "公開し" not in fp:
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
                _add_heart_color_threshold(result, select_text)
                ct = extract_card_type(select_text)
                if ct:
                    result["card_type"] = ct
                _add_or_card_types_if_needed(result, select_text)
                _enrich_from_text(result, select_text)
                if extract_optional(select_text):
                    result["optional"] = True
                # Check for "か" (OR) between 'debut to stage' and 'add to hand'
                # e.g. "カードを...エリアに登場させるか、手札に加える"
                if "登場させる" in select_text and "手札に加え" in select_text:
                    followup = _build_or_destination_followup(select_text)
                    if followup:
                        result["followup_action"] = followup
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

    # Issue 12: Pattern: hand + deck_top remainder (e.g. "1枚を手札に加え、残りをデッキの上に戻す")
    if "手札に加え" in select_text and "残りをデッキの上" in select_text:
        result["destination"] = "hand"
        result["reveal"] = False
        result.pop("discard_remaining", None)
        result["remainder_destination"] = "deck_top"
        cnt = extract_count(select_text)
        if cnt:
            result["count"] = cnt
        ct = extract_card_type(select_text)
        if ct:
            result["card_type"] = ct
        _enrich_from_text(result, select_text)
        _add_or_card_types_if_needed(result, select_text)
        return result

    # Default: detect destination from text
    # NOTE: Check deck_top BEFORE discard, since select_text often contains
    # both "デッキの上に置く" (selected card goes to deck top) AND
    # "残りを控え室に置く" (remaining cards go to discard).
    result["reveal"] = False
    if (
        "デッキの上に置く" in select_text
        or "デッキの上に" in select_text
        or "デッキの一番上に" in select_text
    ):
        result["destination"] = "deck_top"
    elif "手札に加える" in select_text or "手札に加え" in select_text:
        result["destination"] = "hand"
    elif "控え室に置く" in select_text:
        result["destination"] = "discard"

    # Propagate selection criteria
    _enrich_from_text(result, select_text)

    # Handle heart-color filter in default case
    if result.get("destination") is None and (
        "{{heart_" in select_text or "ハートに" in select_text
    ):
        result["destination"] = "hand"

    return result


def _try_heart_select_reveal(text):
    """好きなハートの色を指定 + 公開 + その中から — choose heart, reveal, select from revealed.

    Handles the Maki bp6 pattern:
      "好きなハートの色を1つ指定する。その後、デッキの上からカードをN枚公開する。
       公開されたカードの中に...条件...合計N枚含まれる場合、その中から..."
    → sequential [specify_heart_color, conditional_on_result(reveal→cond→followup), cleanup]
    """
    if "好きなハートの色" not in text or "公開" not in text or "その中から" not in text:
        return None
    # Split on "その中から"
    parts = text.split("その中から", 1)
    before = parts[0].strip()
    after = parts[1].strip() if len(parts) > 1 else ""
    # Extract heart color selection and reveal from before-text
    # "好きなハートの色を1つ指定する。その後、..."
    if "その後、" in before:
        _, rest = before.split("その後、", 1)
    else:
        rest = before
    # Extract reveal instruction: "自分のデッキの上からカードを5枚公開する"
    reveal_m = re.search(r"デッキの上からカードを(\d+)枚公開", rest)
    if not reveal_m:
        return None
    reveal_count = int(reveal_m.group(1))
    # Parse the "合計N枚含まれる場合" condition from the before-text
    # "公開されたカードの中に...合計N枚含まれる場合"
    heart_match_condition = None
    cond_m = re.search(
        r"公開されたカードの中に.+?合計(\d+)枚含まれる場合",
        before,
    )
    if cond_m:
        cond_count = int(cond_m.group(1))
        heart_match_condition = {
            "type": "all_revealed_match_heart_color",
            "count": cond_count,
            "operator": ">=",
            "cache": True,
            "text": f"公開されたカードの中に指定した色に合致するカードが合計{cond_count}枚含まれる場合",
        }
    # Parse the after-text for the followup actions
    # after = "『μ's』のカードを1枚手札に加え、...公開した残りのカードを控え室に置く"
    select_actions = _build_look_select_actions(after) or {}
    # Check if gain_resource is already nested in select_actions or needs standalone handling
    has_blade = "{{icon_blade.png|ブレード}}" in after or "ブレード" in after
    blade_count = after.count("{{icon_blade.png|ブレード}}")
    if blade_count == 0 and has_blade:
        m = re.search(r"ブレード", after)
        blade_count = len(re.findall(r"ブレード", after))
    seq = []
    # Step 1: specify heart color
    seq.append(
        {
            "action": "specify_heart_color",
            "choice": True,
            "target": "self",
            "text": "好きなハートの色を1つ指定する",
        }
    )
    # Step 2: reveal + conditional_on_result wrapping the followup actions
    # The condition lives ONLY on the conditional_on_result wrapper —
    # select_cards and gain_resource inside have NO conditions so they
    # never get re-evaluated against stale state (revealed_cards was
    # modified by select_cards filtering).
    cond_on_result = {
        "action": "conditional_on_result",
        "text": f"自分のデッキの上からカードを{reveal_count}枚公開する",
        "primary_effect": {
            "action": "reveal",
            "source": "deck_top",
            "count": reveal_count,
            "target": "self",
            "text": f"自分のデッキの上からカードを{reveal_count}枚公開する",
        },
    }
    if heart_match_condition:
        cond_on_result["result_condition"] = heart_match_condition
    followup_actions = []
    if select_actions.get("action") == "select_cards":
        sel = {
            "action": "select_cards",
            "source": "revealed_cards",
            "count": select_actions.get("count", 1),
            "destination": select_actions.get("destination", "hand"),
        }
        if select_actions.get("group_names"):
            sel["group_names"] = select_actions["group_names"]
        if select_actions.get("card_type"):
            sel["card_type"] = select_actions["card_type"]
        followup_actions.append(sel)
    if blade_count > 0:
        gr = {
            "action": "gain_resource",
            "resource": "blade",
            "count": blade_count,
            "duration": "live_end",
            "text": "ブレードを得る",
        }
        followup_actions.append(gr)
    if followup_actions:
        cond_on_result["followup_action"] = {
            "action": "sequential",
            "actions": followup_actions,
        }
    seq.append(cond_on_result)
    # Step 3: discard remaining revealed cards (unconditional cleanup)
    seq.append(
        {
            "action": "move_cards",
            "source": "revealed_cards",
            "destination": "discard",
            "count": 0,
            "all": True,
            "text": "公開した残りのカードを控え室に置く",
        }
    )
    return {"text": text, "action": "sequential", "actions": seq}


def _try_look_and_select(text):
    """その中から — look_at + select + action."""
    if "その中から" not in text:
        return None
    # When "その中から" follows a zone+count condition (e.g. "ライブカード置き場に
    # カードが2枚以上ある場合、その中から..."), the selection operates directly
    # on that zone's cards — not on a previously looked-at set. Skip this handler
    # so the text falls through to _try_conditional_sequential which correctly
    # extracts the condition and produces a non-look-based sequential output.
    if re.search(r"にカードが\d+枚以上ある場合", text.split("その中から")[0]):
        return None
    result = {"text": text, "action": "look_and_select"}
    lm = re.search(r"(.+?)その中から", text)
    if lm:
        look_text = lm.group(1).strip()
        # Extract condition prefix from look action text
        ct, at = split_condition_action(look_text)
        if ct:
            cond = parse_condition(ct)
            if cond and cond.get("type") != "custom":
                result["condition"] = cond
                # When condition mentions a specific zone, propagate as look_action source
                # so the engine knows where to look (e.g. "ライブカード置き場にカードが2枚以上
                # ある場合、その中から..." → look at live_card_zone, not deck_top)
                cond_location = cond.get("location")
                if cond_location and cond_location not in ("stage", "hand"):
                    if at:
                        la = parse_action(at)
                        if la.get("action") != "custom":
                            la.setdefault("source", cond_location)
                            result["look_action"] = la
                    else:
                        result["look_action"] = {
                            "action": "look_at",
                            "source": cond_location,
                            "target": "self",
                        }
                        if cond_location == "live_card_zone":
                            result["look_action"]["all"] = True
        if "look_action" not in result and at:
            look_text = at
            la = parse_action(look_text)
            if la.get("action") != "custom":
                result["look_action"] = la
    am = re.search(r"その中から(.+)", text)
    if am:
        select_text = am.group(1).strip()
        # Issue 12: Split on trailing period-separated conditionals like
        # "...戻す。N以上の場合、さらに..."  — the conditional after the period
        # becomes a separate followup action (not part of the select filter).
        cond_followup = None
        cond_split = re.search(
            r"[。](?:\s*)(?:(\d+以上の場合、)|(そうした場合、))", select_text
        )
        if cond_split:
            cond_start = cond_split.start()
            cond_followup = select_text[cond_start + 1 :].strip()  # skip period
            select_text = select_text[:cond_start].strip()
            if cond_followup.startswith("そうした場合、"):
                cond_followup = cond_followup[len("そうした場合、") :].strip()
        # Split on その後 — the clause after その後 becomes a followup action
        # executed after the look_and_select completes.
        sonogo_parts = re.split(r"[。、]?\s*その後[、。]?\s*", select_text, maxsplit=1)
        if len(sonogo_parts) > 1:
            _build_look_select_with_followup(sonogo_parts[0].strip(), result)
            followup_text = sonogo_parts[1].strip()
            if followup_text:
                parsed = parse_effect(followup_text)
                if parsed:
                    result["followup_action"] = parsed
        else:
            _build_look_select_with_followup(select_text, result)
        if cond_followup:
            parsed_cond = parse_effect(cond_followup)
            if parsed_cond and parsed_cond.get("action") != "custom":
                # Inherit context from parent condition for shorthand followup
                # conditions like "30以上の場合" (which lack comparison_type,
                # group_names, location, card_type, aggregate).
                parent_cond = result.get("condition")
                if parent_cond and isinstance(parent_cond, dict):
                    follow_cond = parsed_cond.get("condition") or {}
                    if (
                        follow_cond
                        and follow_cond.get("type") == "comparison_condition"
                    ):
                        for inherit_key in (
                            "comparison_type",
                            "group_names",
                            "location",
                            "card_type",
                            "aggregate",
                        ):
                            if (
                                inherit_key in parent_cond
                                and inherit_key not in follow_cond
                            ):
                                follow_cond[inherit_key] = parent_cond[inherit_key]
                        # cost_total is special: inherit from the followup's own
                        # count value (the threshold in "N以上"), NOT from the
                        # parent condition's cost_total.
                        if (
                            "cost_total" in parent_cond
                            and "cost_total" not in follow_cond
                        ):
                            follow_cond["cost_total"] = follow_cond.get(
                                "count", parent_cond["cost_total"]
                            )
                # If there's already a followup from その後, nest as sequential
                if result.get("followup_action"):
                    existing = result["followup_action"]
                    result["followup_action"] = {
                        "action": "sequential",
                        "actions": [existing, parsed_cond],
                    }
                else:
                    result["followup_action"] = parsed_cond
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


def _try_self_and_other(text):
    """このメンバーと...ほかの...メンバーN人 — sequential self + other targeting.
    Detects patterns like "このメンバーと自分のステージにいるほかの『Liella!』のメンバー1人は..."
    and splits into sequential actions: self-target first, other-target second."""
    if "このメンバーと" not in text or "ほかの" not in text:
        return None
    # Ensure there's actually a member/group reference after ほかの
    if not re.search(r"ほかの.+?メンバー", text):
        return None
    # Extract condition prefix (e.g. "自分のエネルギーが7枚以上ある場合、") that
    # gates the entire self+other effect.
    cond_text, action_text = split_condition_action(text)
    condition = None
    if cond_text:
        condition = parse_condition(cond_text)
        if condition and condition.get("type") == "custom":
            condition = None
    # Extract the resource/effect portion (after the target description, typically before "は")
    # The text after "このメンバーと...メンバーN人は" contains the actual effect
    m = re.search(r"このメンバーと(.+?)(?:は|が)", action_text)
    if not m:
        return None
    other_part = m.group(1)
    effect_part = action_text[m.end() :].strip()
    # Determine count of other targets
    tc_match = re.search(r"(\d+)人", other_part)
    other_count = int(tc_match.group(1)) if tc_match else 1
    # Extract group names from the other-target part
    other_groups = extract_group_names(other_part)
    # Extract duration prefix
    effect_clean, duration = _strip_duration_prefix(effect_part)
    # Parse the effect action
    action = parse_action(effect_clean)
    if action.get("action") == "custom":
        return None
    # Build self action (targets this card/member)
    self_action = {"target": "self", "count": action.get("count", 1)}
    if "resource" in action:
        self_action["resource"] = action["resource"]
    if "duration" in action:
        self_action["duration"] = action["duration"]
    if "heart_colors" in action:
        self_action["heart_colors"] = action["heart_colors"]
    # Propagate action type and resource fields
    for k in ("action", "operation", "value", "card_type", "self_target"):
        if k in action:
            self_action[k] = action[k]
    # Build other action (targets other members)
    other_action = dict(action)
    other_action["exclude_self"] = True
    other_action["target_count"] = other_count
    other_action["card_type"] = "member_card"
    if other_groups:
        other_action["group_names"] = other_groups
    if duration:
        self_action["duration"] = duration
        other_action["duration"] = duration
    result = {
        "text": text,
        "action": "sequential",
        "actions": [self_action, other_action],
    }
    if condition:
        result["condition"] = condition
    return result


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
    # Protect 「」 content from internal splitting (ability text with periods)
    clean = re.sub(r"「[^」]*」", lambda m: m.group(0).replace("。", "\x00"), text)
    parts = clean.split("。")
    if len(parts) < 2:
        return None
    if not any("さらに" in p for p in parts[1:]):
        return None
    actions = []
    for p in parts:
        pt = p.strip().replace("\x00", "。")
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
        # but remember the duration to apply to the next action.
        filt = []
        stash = None
        for p in parts:
            dm = re.match(
                r"^(ライブ終了時まで|ライブ終了まで|このターンの間|このライブの間)[、，]?$",
                p,
            )
            if dm:
                stash = dm.group(1)  # remember, will prepend to next action
                continue
            if stash:
                p = stash + p
                stash = None
            filt.append(p)
        parts = filt

    else:
        pending_duration = None
        parts = [p for p in text.split("、") if p.strip()]
        filt = []
        stash = None
        for p in parts:
            dm = re.match(
                r"^(ライブ終了時まで|ライブ終了まで|このターンの間|このライブの間)[、，]?$",
                p,
            )
            if dm:
                stash = dm.group(1)
                continue
            if stash:
                p = stash + p
                stash = None
            filt.append(p)
        parts = filt
    # Merge fragments ending with conjunction particle "と" with the next fragment
    # e.g. "これによりアクティブにしたメンバーと" + "このメンバーは" → single fragment
    # Replace the comma with a space to prevent re-splitting by parse_effect.
    merged = []
    for p in parts:
        if (
            merged
            and merged[-1].rstrip("、").endswith("と")
            and not p.startswith("それぞれ")
        ):
            merged[-1] = merged[-1].rstrip("、") + " " + p
        else:
            merged.append(p)
    parts = merged
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
        # Propagate the condition's zone to the action's source when the
        # action text no longer mentions the zone explicitly (e.g. the
        # condition says "ライブカード置き場" but the action only says
        # "その中から" — the zone is implied by the condition).
        if cond and cond.get("location") and "source" not in fa:
            fa["source"] = cond["location"]
    else:
        fa = parse_action(fp)
        cond = None

    # Fix 9c: Handle "select + energy payment" pattern where both appear
    # before the conditional follow-up. E.g.:
    # "ライブカードを1枚選び、そのカードのスコアに等しい数のEを支払ってもよい。そうした場合、..."
    # Split into select + pay_energy(dynamic) + conditional_move
    middle_pay = None
    if (
        fa.get("action") == "select"
        and "{{icon_energy.png|E}}" in fp
        and ("支払う" in fp or "支払って" in fp)
    ):
        # Split the first part on "、" to separate select from energy payment
        fp_segments = fp.split("、")
        if (
            len(fp_segments) >= 2
            and "選び" in fp_segments[0]
            and "{{icon_energy.png|E}}" in fp_segments[1]
        ):
            select_text = fp_segments[0] + "、"
            pay_text = fp_segments[1]
            # Re-parse select part (trimmed to exclude energy payment)
            if fc and fat:
                fa = parse_action(select_text)
                fa["text"] = select_text
            else:
                fa = parse_action(select_text)
            # Parse energy payment as a separate action
            middle_pay = parse_action(pay_text)
            if middle_pay.get("action") != "pay_energy":
                middle_pay = None  # fallback: don't split

    # Process second part — use parse_effect to handle sequential sub-actions
    clean = sp.replace(CONDITIONAL_SEQUENTIAL_MARKER, "").strip().lstrip("、")
    sa = parse_effect(clean)
    # selected_cards reference from select action
    # opponent-targeted sub-actions use source="selected_cards" directly
    # (no opponent_action wrapper needed — target+action_by fields suffice).
    if fa.get("action") == "select":
        if isinstance(sa, dict) and "actions" in sa:
            for sub in sa.get("actions", []):
                if sub.get("action") == "move_cards":
                    sub["source"] = "selected_cards"
                if sub.get("action_by") == "opponent":
                    sub.setdefault("source", "selected_cards")
        elif isinstance(sa, dict):
            if sa.get("action_by") == "opponent":
                sa.setdefault("source", "selected_cards")
            else:
                sa["source"] = "selected_cards"

    # For select actions with an explicit destination in the first-part text
    # (e.g. "選び、デッキの一番上に置いてもよい"), insert a move_cards step
    # between the select and the followup so the selected card actually moves.
    move_step = None
    if fa.get("action") == "select" and ("置く" in fp or "置いて" in fp):
        if "デッキの一番上" in fp:
            fa["destination"] = "deck_top"
        if fa.get("destination"):
            move_step = {
                "action": "move_cards",
                "source": "selected_cards",
                "destination": fa.pop("destination"),
                "count": 0,
                "all": True,
            }

    # NOTE: Returning `conditional_on_optional` would be semantically cleaner
    # (optional_action = do X, conditional_action = if done, do Y), but the
    # engine's `execute_conditional_on_optional` handler treats optional_action
    # as a COST (presenting "Skip/Pay" choice, then running conditional_action
    # on Pay). Action-based optionals (like DIVE! where the optional IS the
    # effect) must remain as `sequential` + `conditional: true` for the engine
    # to handle them correctly via the sequential pipeline.
    actions = [fa, middle_pay, sa] if middle_pay else [fa, sa]
    if move_step:
        actions.insert(1, move_step)
    result = {
        "text": text,
        "action": "sequential",
        "actions": actions,
        "conditional": True,
    }
    if cond:
        result["condition"] = cond
    return result


def _try_sequential(text):
    """此后、 — sequential marker. Must be checked BEFORE _try_conditional
    so that 条件→行動。此后、条件→行動 patterns are split correctly
    (moved from position 17 to position 12 in _EFFECT_HANDLERS)."""
    if SEQUENTIAL_MARKER not in text:
        return None
    parts = text.split(SEQUENTIAL_MARKER, 1)
    fa = parse_effect(parts[0].strip())
    sp = parts[1].strip().lstrip("、")
    if sp.startswith("此后"):
        sp = sp[len("此后") :].strip()
    sa = parse_effect(sp)
    # Reduce unnecessary nesting: if sa is a sequential wrapping a single
    # conditional action (場合、action), flatten it by pulling the condition
    # onto the action directly instead of double-wrapping.
    if (
        sa.get("action") == "sequential"
        and sa.get("condition")
        and not sa.get("actions")
    ):
        # sa is {action: sequential, condition: X, ...action_fields}
        # This means _try_conditional produced a conditional sequential.
        # Keep as-is since the condition gate is meaningful.
        pass
    elif sa.get("action") == "sequential" and len(sa.get("actions", [])) == 1:
        inner = sa["actions"][0]
        if inner.get("condition") and not inner.get("actions"):
            # Inner is a single conditional action — flatten
            sa = inner
    return {"text": text, "action": "sequential", "actions": [fa, sa]}


def _resolve_preceding_moved_condition(cond, src_text):
    """Post-process a choice condition to detect preceding_moved references.

    When the condition text contains "これにより" + "置い" (referring to a
    card placed/discarded by the preceding cost action), unwrap any
    complex_condition wrapping and set source: "preceding_moved" so the
    engine checks the recently-moved card rather than a static zone.
    """
    if "これにより" in src_text and ("置いた" in src_text or "置かれ" in src_text):
        if cond.get("type") == "complex_condition":
            effect = cond.get("effect", {})
            if effect:
                effect["source"] = "preceding_moved"
                effect.pop("location", None)
                if effect.get("type") in ("location_condition",):
                    effect["type"] = "card_count_condition"
                return effect
        elif cond:
            cond["source"] = "preceding_moved"
            cond.pop("location", None)
            if cond.get("type") in ("location_condition",):
                cond["type"] = "card_count_condition"
    return cond


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
        cond = _resolve_preceding_moved_condition(cond, cond_mod)
        if cond.get("type") != "custom":
            result["choice_condition"] = cond

    options = []
    for ot in opts:
        oc, oa = split_condition_action(ot)
        po = parse_effect(oa) if oc and oa else parse_action(ot)
        if oc and oa:
            po["condition"] = parse_condition(oc)
        # Check for compound option: text with multiple actions split by "、"
        if po.get("action") and po.get("action") != "sequential":
            sub_texts = [
                s.strip().rstrip("。、") for s in re.split(r"[。、]", ot) if s.strip()
            ]
            if len(sub_texts) >= 2:
                sub_actions = [parse_action(t) for t in sub_texts]
                sub_actions = [
                    a
                    for a in sub_actions
                    if a.get("action")
                    and a.get("action") not in ("custom", "do_nothing")
                ]
                if len(sub_actions) >= 2:
                    po = {"action": "sequential", "actions": sub_actions, "text": ot}
        po["text"] = ot
        options.append(po)
    if not options:
        return None

    # Conditional alternative in choice modifier: "代わりに"
    # Emit a single choice with alternative_condition so the engine can
    # pick count=1 vs any_number based on the condition, without duplicating
    # the entire options list.
    if cond_mod and ALTERNATIVE_MARKER in cond_mod:
        alt_parts = cond_mod.split(ALTERNATIVE_MARKER, 1)
        if len(alt_parts) == 2:
            before = alt_parts[0].strip().rstrip("、。")
            after = alt_parts[1].strip().rstrip("。")
            # Extract the actual condition part from the before text.
            # The before text includes stuff like "1つを選ぶ。" prefix.
            alt_cond = parse_condition(before)
            alt_cond = _resolve_preceding_moved_condition(alt_cond, before)
            if alt_cond.get("type") != "custom":
                result["alternative_condition"] = alt_cond
            # Count becomes 1 by default (pick exactly one).
            # If the alternative is "1つ以上" (one or more), use any_number.
            if "以上" in after:
                result["alternative_count_type"] = "any_number"
            else:
                ac = extract_count(after)
                if ac:
                    result["alternative_count"] = ac

    result["options"] = options
    result["count"] = 1
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
    # Let _try_choice handle "以下からNつを選ぶ" patterns (bullet-pointed choice)
    if CHOICE_MARKER in text:
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
                cond = ce.get("condition")
                if ca:
                    # Propagate condition to each sub-action so conditional
                    # gating is preserved even when the action is compound.
                    for sub in ca:
                        if cond and "condition" not in sub:
                            sub["condition"] = cond
                    actions.extend(ca)
                else:
                    # Issue 7: reference_card binding for select→conditional chain
                    if (
                        actions
                        and actions[-1].get("action") == "select"
                        and ("同じカード名" in full or "それと同じ" in full)
                    ):
                        c = ce.get("condition")
                        if c and c.get("comparison_type") == "equality":
                            c["reference_card"] = "previous_selected"
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
        t_pos = text.find("時、")
        if t_pos > 0 and "ライブ終了時まで" not in text[: t_pos + 2]:
            before_toki = text[:t_pos].strip()
            # Skip timing phrases (ライブ開始時、ライブ成功時、ターン開始時 etc.)
            # These are NOT conditions — they are temporal triggers.
            # Real conditions have action verbs like 選んだ、置いた、した.
            timing_keywords = ("開始", "成功", "終了", "勝利", "敗北")
            if any(kw in before_toki for kw in timing_keywords):
                return None
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

    # Baton touch "this baton touch placed" — use recently_moved source
    # Only override if the action text explicitly says "by this baton touch"
    # (e.g. "このバトンタッチで控え室に置かれた"), not a generic discard search.
    if (
        cond.get("baton_touch_trigger")
        and action.get("action") == "move_cards"
        and action.get("source") == "discard"
        and ("このバトンタッチで" in at or "このバトンタッチにより" in at)
    ):
        action["source"] = "recently_moved"

    # Handle "条件Aの場合、または条件Bの場合、行動" — merge into OR condition
    if action.get("condition") and at.lstrip().startswith("または"):
        cond = {
            "type": "or_condition",
            "conditions": [cond, action.pop("condition")],
            "text": text,
        }
        result["condition"] = cond

    # Save condition before result.update(action) since action may carry its own
    # phantom condition from timing phrases (e.g. "相手のライブ開始時") that should
    # not overwrite the real conditional gate.
    saved_condition = result.get("condition")
    saved_text = result.get("text")

    if action.get("action") == "sequential":
        result["action"] = "sequential"
        result["actions"] = action.get("actions", [])
        if "text" in action:
            result["text"] = action["text"]
    else:
        result.update(action)
    # Only strip exclude_self when it duplicates condition-level exclude_self.
    # Per-unit gain_resource effects need exclude_self on the action for
    # filter_subset() to correctly exclude self from counting.
    if result.get("action") in ("gain_resource", "heart_selection", "set_heart_type"):
        cond = result.get("condition", {})
        if isinstance(cond, dict) and cond.get("exclude_self"):
            result.pop("exclude_self", None)

    # Restore the outer condition — it must NOT be overwritten by timing phrases
    # or phantom conditions from the recursive parse_effect call.
    if saved_condition is not None:
        result["condition"] = saved_condition
    if saved_text is not None:
        result["text"] = saved_text

    return result if (result.get("action") or result.get("actions")) else None


def _try_ability_activation(text):
    """能力を発動させる — ability activation effects.
    Handles both simple patterns ("...能力を発動させる") and sequential
    patterns ("select card. activate its ability")."""
    # Check for compound patterns: "select card. activate its ability"
    if "。" in text:
        # Let _try_choice handle "以下からNつを選ぶ" (bullet-pointed choice)
        if CHOICE_MARKER in text:
            return None
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
    # Baton touch "this baton touch placed" — override source to recently_moved
    # Only override if the action text explicitly says "by this baton touch"
    # (e.g. "このバトンタッチで控え室に置かれた"), not a generic discard search.
    if (
        cond.get("baton_touch_trigger")
        and action.get("source") == "discard"
        and (
            "このバトンタッチで" in action_text
            or "このバトンタッチにより" in action_text
        )
    ):
        action["source"] = "recently_moved"
    result = {"text": text, "condition": cond}
    result.update(action)
    return result


def _try_kore_niyori_result(text):
    """これにより～した場合/とき — conditional on result (invalidation follow-up, discard follow-up, etc.)."""
    if "これにより" not in text:
        return None
    # If the text contains a choice marker (以下から1つを選ぶ), the choice handler
    # should process this instead — this is a "choose from options with conditional
    # count upgrade" pattern, not a "do X, then if result condition Y, do Z" pattern.
    if CHOICE_MARKER in text:
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
    cond_raw = cp.strip() + cond_marker
    cond = parse_condition(cond_raw)
    # Infer location from context: "公開された" means revealed_cards
    if cond and "公開された" in cp and not cond.get("location"):
        cond["location"] = "revealed_cards"
    # Issue 11: If the condition describes cards placed somewhere ("デッキの下に置いた"),
    # it references the cards moved by the preceding action, not the entire deck.
    if cond and ("置いた" in cp or "置かれ" in cp):
        cond["source"] = "preceding_moved"
        cond.pop("location", None)
        if cond.get("type") in ("location_condition",):
            cond["type"] = "card_count_condition"
    # Extract character names from 「X」のメンバーカード/ライブカード patterns
    if cond and isinstance(cond, dict) and not cond.get("characters"):
        char_m = re.search(r"「([^」]+)」の(?:メンバーカード|ライブカード)", cond_raw)
        if char_m:
            cond["characters"] = [char_m.group(1)]
    # "custom" type conditions can't be evaluated by the engine — skip them.
    # However, detect known action result patterns and convert them.
    if cond and cond.get("type") == "custom":
        # Issue 1: "無効にした場合" → action_success_condition for invalidation
        if "無効にし" in cp:
            cond = {
                "type": "action_success_condition",
                "text": cond_raw,
                "action_reference": "invalidate_ability",
            }
        else:
            cond = None
    primary_text = parts[0].strip()
    # Skip empty/trivial primary text (e.g. cost text already consumed, or just bracket fragments)
    if not primary_text or re.match(r"^[\s）」\)』」、。]*$", primary_text):
        return None
    primary = parse_action(primary_text)
    return {
        "text": text,
        "action": "conditional_on_result",
        "primary_effect": primary,
        "result_condition": cond,
        "followup_action": parse_effect(fp.strip()),
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


def _try_unless_effect(text):
    """しないかぎり/ないかぎり — unless-pay effect pattern.
    E.g. "{{E}}{{E}}支払わないかぎり、自分の手札を2枚控え室に置く。"
    → optional_action: pay 2 energy to AVOID the effect
    → conditional_action: discard 2 from hand (fires when cost NOT paid)

    Also handles non-energy unless effects like:
    "手札を1枚控え室に置かないかぎり、自分のエネルギー1枚をエネルギーデッキに置く。"
    → optional_action: discard 1 from hand
    → conditional_action: place 1 energy to energy deck
    """

    if "しないかぎり" not in text and "ないかぎり" not in text:
        return None
    kw = "しないかぎり" if "しないかぎり" in text else "ないかぎり"
    parts = text.split(kw + "、", 1)
    if len(parts) < 2:
        return None
    unless_text = parts[0].strip()
    eff_text = parts[1].strip()

    # Energy-based unless (existing): pay energy to avoid effect
    if "{{icon_energy.png|E}}" in unless_text:
        ec = unless_text.count("{{icon_energy.png|E}}")
        fa = {"action": "pay_energy", "energy": ec, "count": ec, "target": "self"}
        aa = parse_effect(eff_text)
        return {
            "text": text,
            "action": "conditional_on_optional",
            "optional_action": fa,
            "conditional_action": aa,
            "conditional_negation": True,
        }

    # Non-energy unless: detect "手札をN枚控え室に置かないかぎり" pattern
    # (unless you discard N from hand to waitroom). The split leaves the
    # imperfective stem (e.g. "置か" from 置かない), so we reconstruct
    # the action manually.
    m = re.search(r"手札を(\d+)枚控え室に置", unless_text)
    if m:
        count = int(m.group(1))
        fa = {
            "action": "move_cards",
            "source": "hand",
            "destination": "discard",
            "count": count,
            "target": "self",
        }
        aa = parse_effect(eff_text)
        # When the conditional action is "place energy to energy deck", fix both
        # the source (energy_zone, not hand) and the destination (energy_deck, not deck).
        if aa.get("card_type") == "energy_card" and aa.get("destination") == "deck":
            aa["source"] = "energy_zone"
            aa["destination"] = "energy_deck"
        if aa.get("action") != "custom":
            return {
                "text": text,
                "action": "conditional_on_optional",
                "optional_action": fa,
                "conditional_action": aa,
                "conditional_negation": True,
            }

    return None


def _try_shi_sequential(text):
    """Aし、B — multiple actions joined by te-form."""
    if "し、" not in text:
        return None
    # If choice marker is present, let _try_choice handle the text
    if CHOICE_MARKER in text:
        return None
    # If condition markers are present, let _try_conditional handle the text
    if any(m in text for m in CONDITION_MARKERS):
        return None
    # Split only on "し、" (te-form action boundary), NOT on every comma.
    # Commas within the second clause (e.g. topic-comment "は、" separators)
    # should remain intact so parse_effect sees the complete action text.
    idx = text.find("し、")
    if idx < 0:
        return None
    first = text[: idx + 1]  # include the "し"
    rest = text[idx + 2 :].strip().lstrip("、")  # everything after "し、"
    first_a = parse_effect(first)
    if first_a.get("action", "custom") in ("custom", "do_nothing"):
        return None
    second_a = parse_effect(rest)
    if second_a.get("action", "custom") in ("custom", "do_nothing"):
        return None
    return {"text": text, "action": "sequential", "actions": [first_a, second_a]}


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
    """～は、～ — global required hearts modifier (必要ハートが多くなる/少なくなる)."""
    m = re.search(r".+は、.+", text)
    if not m or "ある場合" in text:
        return None
    if "必要ハート" in text and ("多くなる" in text or "少なくなる" in text):
        result = {
            "text": text,
            "action": "modify_required_hearts_global",
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
        # Extract heart_colors from the effect text (the heart icon being modified)
        hm = re.search(r"\{\{heart_(\d+)\.png\|heart\d+\}\}", text)
        if hm:
            hc = f"heart{hm.group(1).zfill(2)}"
            result["heart_colors"] = [hc]
        # Extract value: "2つ多くなる" → value=2, bare "多くなる" → value=1
        vm = re.search(r"(\d+)つ多", text)
        if vm:
            result["value"] = int(vm.group(1))
        else:
            result["value"] = 1
        return result
    return None


def _set_lose_resource_fields(text, result):
    """Setter for lose_resource EffectPattern: detect resource, count, duration."""
    if "ブレード" in text:
        result["resource"] = "blade"
    elif "ハート" in text:
        result["resource"] = "heart"
    blade_count = len(re.findall(r"\{\{icon_blade\.png\|ブレード\}\}", text))
    if blade_count > 0:
        result["count"] = blade_count
    heart_count = len(re.findall(r"\{\{heart_\d+\.png\|heart\d+\}\}", text))
    if heart_count > 0:
        result["count"] = heart_count
        result["heart_colors"] = extract_heart_types(text)
    if "ライブ終了時まで" in text:
        result["duration"] = "live_end"


_try_lose_resource = EffectPattern(
    match="を失う",
    exclude_any=["もう一度エールを行う", "もう1度エールを行う"],
    action="gain_resource",
    defaults={"sign": "negative"},
    setter=_set_lose_resource_fields,
)

_try_play_baton_touch = EffectPattern(
    match_all=["プレイに際し", "バトンタッチ"],
    action="play_baton_touch",
    extract={"count": r"(\d+)人のメンバーとバトンタッチ"},
)


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
    if cond:
        result["conditional"] = True
    result.update(action)
    # Restore outer condition — must not be overwritten by action's own condition
    if cond:
        result["condition"] = cond
    return result


def _try_restriction_effect(text):
    """アクティブにならない/にしない — restriction effect preventing activation.
    Matches both "効果によってはアクティブにならない" and plain "アクティブフェイズにアクティブにならない".
    Also matches "アクティブにしない" (causative negative, e.g. 近江彼方)."""
    if "アクティブにならない" not in text and "アクティブにしない" not in text:
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
    """もう一度エールを行う + ブレードハートを失い — re-yell with perform_yell.
    Handles preceding actions before the re-yell sentence (e.g. optional discard)
    when the text has a 。 separator.

    The preceding action's condition (e.g. ≤2 blade_heart) gates the ENTIRE
    sequential — not just the first action. It is promoted to the outer level."""
    if "もう一度エールを行う" not in text or "ブレードハートを失い" not in text:
        return None
    promoted_condition = None
    actions = []
    # If there are preceding sentences before the re-yell instruction, parse them
    if "。" in text:
        parts = text.rsplit("。", 1)
        preceding = parts[0].strip()
        if preceding:
            parsed = parse_effect(preceding)
            if isinstance(parsed, dict):
                if parsed.get("action") == "sequential":
                    actions.extend(parsed.get("actions", []))
                else:
                    actions.append(parsed)
        text = parts[1].strip().lstrip("、")
    # Promote condition from the first action so it gates the entire re-yell sequence
    if actions:
        first = actions[0]
        if "condition" in first and isinstance(first.get("condition"), dict):
            promoted_condition = first.pop("condition")
    actions.append(
        {
            "text": "ブレードハートを失い",
            "action": "re_yell",
            "lose_blade_hearts": True,
            "target": "self",
        }
    )
    actions.append(
        {
            "text": "もう一度エールを行う",
            "action": "perform_yell",
            "count": 1,
            "target": "self",
        }
    )
    result = {"text": text, "action": "sequential", "actions": actions}
    if promoted_condition:
        result["condition"] = promoted_condition
    return result


_try_energy_under_member = EffectPattern(
    match="下に置かれているエネルギーカード",
    action="place_energy_under_member",
    defaults={
        "source": "under_member",
        "card_type": "energy_card",
        "energy_count": 1,
        "target_member": "this_member",
    },
)


def _try_heart_choice(text):
    """XXのうち、選んだYつ — choice between heart requirement options.
    e.g. "必要ハートは、heart01...か、heart04...か、heart05...のうち、選んだ1つにしてもよい"
    Each option is modify_required_hearts with the specific heart pattern."""
    if "のうち、選んだ" not in text and "のうち選んだ" not in text:
        return None
    at = text
    cond = None
    for marker in ("場合、", "場合"):
        if marker in text:
            parts = text.split(marker, 1)
            if len(parts) == 2:
                cond_text = parts[0].strip() + marker.rstrip("、")
                at = parts[1].strip().lstrip("、")
                cond = parse_condition(cond_text)
                break
    marker = "のうち、選んだ" if "のうち、選んだ" in at else "のうち選んだ"
    idx = at.find(marker)
    if idx < 0:
        return None
    options_text = at[:idx].strip()
    count = 1
    cm = re.search(r"選んだ(\d+)つ", at[idx:])
    if cm:
        count = int(cm.group(1))
    optional = "してもよい" in at or "てもよい" in at
    operation = "set"
    if "減らす" in at or "減る" in at:
        operation = "decrease"
    elif "増やす" in at or "増える" in at:
        operation = "increase"
    raw_options = re.split(r"か[、，]?", options_text)
    options = []
    icon_pat = r"\{\{heart_(\d+)\.png\|heart\d+\}\}"
    for ro in raw_options:
        ro = ro.strip().rstrip("、，").strip()
        if not ro:
            continue
        hm = re.search(icon_pat, ro)
        if hm:
            ro = ro[hm.start() :].strip()
        if not ro:
            continue
        per_color = {}
        per_color_text = {}
        for m in re.finditer(icon_pat, ro):
            full = m.group(0)
            key = f"heart{m.group(1).zfill(2)}"
            per_color[key] = per_color.get(key, 0) + 1
            per_color_text.setdefault(key, "")
            per_color_text[key] += full
        sub_actions = []
        for color_str, color_count in per_color.items():
            icon_text = per_color_text.get(color_str, "")
            sub = {
                "text": icon_text if icon_text else f"{color_str}×{color_count}",
                "action": "modify_required_hearts",
                "heart_colors": [color_str],
                "operation": operation,
                "self_target": True,
                "count": color_count,
            }
            sub_actions.append(sub)
        if len(sub_actions) == 1:
            opt = sub_actions[0]
            opt["text"] = ro
        else:
            opt = {"text": ro, "action": "sequential", "actions": sub_actions}
        options.append(opt)
    if not options:
        return None
    result = {"text": text, "action": "choice", "options": options}
    if cond and cond.get("type") != "custom":
        result["condition"] = cond
    result["count"] = count
    if optional:
        result["optional"] = True
    return result


# ====================================================================
# EFFECT HANDLER DEFINITIONS & DISPATCH
# ====================================================================
# The _EFFECT_HANDLERS list defines the priority-ordered cascade.
# Each function takes raw effect text and returns a parsed dict or None.
# The first match wins — ordering is CRITICAL.
#
# Key constraints:
#   - Per-unit (につき) must be checked early because it restructures text
#   - Conditional shapes (場合/そうした場合) must be checked before single actions
#   - Sequential patterns (その後) must be checked before _try_conditional
# ====================================================================


def _try_timing_condition_gain(text):
    """このターン中にエリアを移動した(全ての)メンバーはYを得る — gain with timing_condition."""
    m = re.search(
        r"このターン中にエリアを移動した(?:(?:全て|すべて)の)?(?:(.+?)の)?メンバー[は、]+(.+?)を得る",
        text,
    )
    if not m:
        return None
    has_all = bool(re.search(r"このターン中にエリアを移動した(?:全て|すべて)の", text))
    group_name = m.group(1)
    resource_text = m.group(2)
    blade_count = resource_text.count("{{icon_blade.png|ブレード}}")
    full_resource_text = resource_text + "を得る"
    if blade_count == 0:
        return None
    result = {
        "text": full_resource_text,
        "action": "gain_resource",
        "resource": "blade",
        "count": blade_count,
        "card_type": "member_card",
        "timing_condition": "moved_this_turn",
        "target": "self",
    }
    if has_all:
        result["all"] = True
    if group_name:
        gn = group_name.strip("｢「『　 ").rstrip("｣」』　 ")
        result["group_names"] = [gn]
    return result


_EFFECT_HANDLERS = [
    # Tier 1: Very specific patterns that must be checked first.
    # These would be misparsed by any generic handler.
    _try_timing_condition_gain,  # このターン中にエリアを移動した全てのX...
    _try_self_and_other,  # XとYを得る (two different targets)
    _try_per_unit,  # XにつきY (per-unit gain — restructures text)
    _try_conditional_alternative,  # X場合Y、そうでない場合Z (if/else)
    _try_character_specific,  # 「X」のキャラ specific effect
    _try_activation_suffix,  # （この能力は...） activation conditions
    _try_cost_modification,  # Cost modification patterns
    _try_kore_niyori_case,  # これによりX場合、Y (conditional on prior result)
    _try_heart_select_reveal,  # Heart selection + reveal combo
    _try_choose_self_opponent,  # 自分か相手を選ぶ (choose self or opponent)
    _try_look_and_select,  # Look at N cards, select some
    _try_answer_choice,  # 回答がXの場合 (answer-based choice)
    _try_each_time,  # Xたび (each-time trigger + effect)
    # Tier 2: Conditional/optional effect shapes
    _try_unless_effect,  # しないかぎり (unless-pay)
    _try_opponent_action,  # 相手は... (opponent action)
    _try_opponent_after_conditional,  # 相手はX場合、Y (opponent conditional)
    _try_reveal_until_chosen_card,  # Reveal until condition met
    _try_reveal_until_live,  # Reveal until live card found
    _try_furthermore,  # さらに (furthermore/additional effect)
    _try_kore_niyori_result,  # これによりX場合 (result-based conditional)
    _try_sequential_duration,  # Duration + sequential
    _try_conditional_sequential,  # そうした場合X場合 (nested conditional-on-optional)
    # Tier 3: Sequential and compound patterns
    _try_sequential,  # その後、 (then)
    _try_duration_effect,  # Duration-bounded effect
    _try_sou_shinakatta,  # そうしなかった場合 (if you don't)
    _try_period_conditional,  # 。場合 (period-separated conditional)
    _try_compound_select,  # Compound select + action
    _try_shi_sequential,  # Aし、B (te-form sequential)
    _try_te_sequential,  # Xを得て、Yを得る (sequential gains)
    _try_re_yell,  # Re-yell — must be before implicit sequential to prevent splitting
    _try_implicit_sequential,  # Implicit sequential (two actions fused)
    # Tier 4: Single-action patterns (most general)
    _try_conditional,  # X場合、Y (generic conditional)
    _try_ability_activation,  # 能力を発動させる (activate ability)
    _try_heart_choice,  # ハートをXつ選ぶ (heart choice)
    _try_choice,  # 以下から1つを選ぶ (generic choice)
    _try_kore_niyori_cascade,  # により cascade
    _try_baton_touch_effect,  # バトンタッチ (baton touch)
    _try_global_modifier,  # Global effect modifiers
    _try_play_baton_touch,  # Play + baton touch combo
    _try_energy_under_member,  # Energy under member
    _try_blade_actions,  # Blade-specific actions
    _try_both_discard_until,  # Both players discard until
    _try_lose_resource,  # Lose resource
    _try_restriction_effect,  # Restriction effects
]

# PriorityRegistry wrapping _EFFECT_HANDLERS for named, sorted dispatch.
# Priority = index in the list preserves existing ordering.
_effect_registry = PriorityRegistry("effect_handlers")
for _i, _h in enumerate(_EFFECT_HANDLERS):
    _hn = getattr(_h, "__name__", f"handler_{_i}")
    _effect_registry.register(_i, _hn, _h)

# ======================================================================
# POST-PROCESSING NORMALIZERS & CHAINING
# ======================================================================


def _has_position_keywords(text):
    positions = []
    for keyword, position in POSITION_KEYWORDS.items():
        if keyword in text and position not in positions:
            positions.append(position)
    if not positions and "center" in text.lower():
        return "center"
    return ",".join(positions) if positions else None


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
            "exclude_by_name_source",
            "all",
            "target",
            "position",
            "activation_position",
            "source_position",
            "exclude_position",
            "group_names",
            "exclude_group_names",
            "heart_colors",
            "shuffle",
            "optional",
            "duration",
            "count",
        ):
            if f in parent_effect:
                for sub in cleaned:
                    if f not in sub:
                        # Don't propagate exclude_self to self-targeting
                        # sub-actions — it's contradictory.
                        if f == "exclude_self" and (
                            sub.get("target") == "self"
                            or sub.get("action")
                            in (
                                "gain_resource",
                                "set_heart_type",
                                "heart_selection",
                                "modify_score",
                            )
                        ):
                            continue
                        # Don't propagate group_names to energy change_state actions
                        if (
                            f == "group_names"
                            and sub.get("action") == "change_state"
                            and sub.get("card_type") == "energy_card"
                        ):
                            continue
                        # Don't propagate group_names to move_cards sub-actions
                        # unless the group name appears in the sub-action's own text.
                        # This prevents parent-context groups (e.g. "μ's", "Liella!")
                        # from leaking into generic cost payments like
                        # "手札を1枚控え室に置く" (discard any card).
                        if f == "group_names" and sub.get("action") == "move_cards":
                            sub_text = sub.get("text", "")
                            if not any(g in sub_text for g in parent_effect[f]):
                                continue
                        # Don't propagate group_names to gain_resource sub-actions.
                        # group_names should only appear on gain_resource when the
                        # ability text explicitly says "Groupのメンバーにブレードを与える".
                        # Leaked group_names cause the engine to distribute resources
                        # to ALL matching group members instead of the activating card.
                        if f == "group_names" and sub.get("action") == "gain_resource":
                            continue
                        # Don't propagate group_names to specify_heart_color or
                        # reveal sub-actions — group filtering doesn't apply to
                        # color selection or deck revelation (only to selection).
                        if f == "group_names" and sub.get("action") in (
                            "specify_heart_color",
                            "reveal",
                        ):
                            continue
                        # Don't propagate group_names to modify_cost sub-actions
                        # UNLESS it's a per-unit cost modifier (which needs the
                        # group filter to count the right cards on stage).
                        # Leaked group_names on non-per-unit modify_cost would
                        # cause the engine to apply cost changes to ALL matching
                        # group members instead of just this card.
                        if (
                            f == "group_names"
                            and sub.get("action") == "modify_cost"
                            and not sub.get("per_unit")
                        ):
                            continue
                        # Don't propagate heart_colors to gain_resource actions
                        # (the heart color was selected by a previous select action)
                        if f == "heart_colors" and sub.get("action") == "gain_resource":
                            continue
                        sub[f] = parent_effect[f]
        # Propagate card_type from parent to sub-actions that don't have it
        pt = parent_effect.get("card_type")
        if pt:
            for sub in cleaned:
                if "card_type" not in sub:
                    # Don't propagate card_type to gain_resource sub-actions.
                    # card_type should only appear on gain_resource when the ability
                    # text explicitly says "メンバーカードにブレードを与える".
                    if sub.get("action") == "gain_resource":
                        continue
                    sub["card_type"] = pt
        # Propagate cost_limit from parent to sub-actions
        cl = parent_effect.get("cost_limit")
        if cl:
            for sub in cleaned:
                if "cost_limit" not in sub:
                    sub["cost_limit"] = cl
        # Also propagate cost_limit_operator
        clo = parent_effect.get("cost_limit_operator")
        if clo and cl:
            for sub in cleaned:
                if "cost_limit_operator" not in sub:
                    sub["cost_limit_operator"] = clo
    return cleaned


def _walk_ensure_text(d, ctx_text, full_text):
    # Ensure every condition/effect dict has a text field
    if "text" not in d and ("type" in d or "action" in d):
        if ctx_text:
            d["text"] = ctx_text
        elif full_text:
            d["text"] = full_text


def _walk_propagate_activation_position(d, original_text, full_text):
    # Check activation position if not set — first from parenthetical notes,
    # then from position icons ({{center.png}}, {{leftside.png}}, {{rightside.png}})
    # in the original text (e.g. {{leftside.png|左サイド}} after a trigger).
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

    if "activation_position" not in d:
        raw = original_text or full_text
        positions = detect_icon_positions(raw)
        if positions:
            d["activation_position"] = ",".join(positions)


def _walk_propagate_text_context_fields(d, d_ctx, ctx_text):
    # Propagate exclude_self from text context to sub-actions.
    # "このメンバー以外" = "other than this member" → always exclude the activating card.
    # per_unit gain_resource/heart effects should receive exclude_self
    # (e.g. "ほかのメンバー1人につき" needs self excluded from the count).
    # Non-per_unit self-buffs (plain "gain_resource" without "per_unit")
    # are excluded since target="self" + exclude_self is contradictory,
    # UNLESS the text explicitly says "このメンバー以外".
    if (
        "exclude_self" not in d
        and "exclude_by_name_source" not in d
        and d_ctx
        and ("このメンバー以外" in d_ctx or "ほかの" in d_ctx or "他の" in d_ctx)
    ):
        is_per_unit = d.get("per_unit", False)
        is_self_buff = d.get("action") in (
            "set_heart_type",
            "heart_selection",
            "modify_score",
            "change_state",
        ) or (d.get("action") in ("gain_resource",) and not is_per_unit)
        if (
            "このメンバー以外" in d_ctx
            or d.get("action") == "position_change"
            or d.get("target") != "self"
        ) and not is_self_buff:
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

    # Propagate group_names from text context (including parent context) to any dict node
    if "group_names" not in d:
        gms = re.findall(r"『([^』]+)』", d_ctx or "")
        from_parent = not gms and ctx_text
        if from_parent:
            gms = re.findall(r"『([^』]+)』", ctx_text or "")
        if gms and (
            d.get("action") or d.get("type") or d.get("condition") or "text" in d
        ):
            # Deduplicate group_names (same group may appear multiple times in text)
            gms = list(dict.fromkeys(gms))
            # Skip group_names for energy change_state actions entirely.
            # Energy cards are generic and should not be filtered by group.
            if not (
                d.get("action") == "change_state"
                and d.get("card_type") == "energy_card"
            ):
                # Also skip when group_names came from parent context (not this
                # node's own text) and this node is a pure action (no type or
                # condition of its own).  This prevents parent-context groups
                # like "『みらくらぱーく！』" from leaking into primary_effect /
                # followup_action sub-actions of conditional_on_result structures
                # where the group applies only to the result_condition.
                if from_parent:
                    # When the group came from parent context (not the node's own
                    # text), check if it's only in an attached condition's text.
                    # If so, skip — the group belongs to the condition, not the
                    # action (e.g. "『Liella!』のメンバーからバトンタッチ" in the
                    # condition should not make the action filter by Liella!).
                    # Only propagate to sub-conditions with `distinct` (name
                    # distinctness checks) since they need the group context to
                    # know which cards to compare. Avoid blind propagation to
                    # all sub-conditions (e.g. card_count_condition) which would
                    # incorrectly filter counts by group.
                    own_has_group = any(g in (d.get("text", "") or "") for g in gms)
                    needs_group = own_has_group or d.get("distinct")
                    if (
                        needs_group
                        and d.get("action") != "gain_resource"
                        and d.get("type") != "card_count_condition"
                        and (d.get("action") != "modify_cost" or d.get("per_unit"))
                    ):
                        d["group_names"] = gms
                else:
                    # Don't propagate group_names to gain_resource actions.
                    # Leaked group_names cause the engine to distribute resources
                    # to ALL matching group members instead of the activating card.
                    # Also skip card_count_condition — the parser already handles
                    # pure group-filtered counts; _walk would incorrectly add
                    # group_names to inclusion-pattern counts ("1人を含む").
                    # Only propagate if this action depends on the condition's
                    # context (source="those_cards") or the group name actually
                    # appears in the action's own text (not just the condition).
                    # Otherwise the group name may only belong to the condition
                    # (e.g. "『スリーズブーケ』のメンバーがいる場合") and should not
                    # filter the action.
                    node_text = d.get("text", "") or ""
                    cond_text = d.get("condition", {}).get("text", "")
                    if cond_text:
                        for sep in ("、", "場合、", "とき、", "なら、"):
                            combined = cond_text + sep
                            if combined in node_text:
                                node_text = node_text.split(combined, 1)[-1]
                                break
                    if d.get("source") == "those_cards" or any(
                        g in node_text for g in gms
                    ):
                        if (
                            d.get("action") != "gain_resource"
                            and d.get("type") != "card_count_condition"
                            and (d.get("action") != "modify_cost" or d.get("per_unit"))
                        ):
                            d["group_names"] = gms

    # Propagate shuffle from text context
    if "shuffle" not in d and d_ctx and "シャッフル" in d_ctx:
        d["shuffle"] = True


def _walk_extract_heart_colors(d, d_text, ctx_text):
    # Extract heart_colors from action text for gain_resource / modify_required_hearts
    if "heart_colors" not in d and d.get("action") in (
        "gain_resource",
        "modify_required_hearts",
        "move_cards",
    ):
        # gain_resource uses the heart color selected by a
        # preceding select action (stored in conditional_choice) —
        # never inherit heart_colors from parent context.
        if d.get("action") == "gain_resource":
            search_text = d_text or ""
        else:
            # Check own text first, then parent context
            search_text = d_text or ""
            # Actions with explicit heart_color (e.g. from character_effects)
            # should not inherit aggregate heart_colors from parent context
            if not re.search(r"heart_\d+", search_text) and ctx_text:
                if not d.get("heart_color"):
                    search_text = ctx_text
        hc = list(
            dict.fromkeys(
                f"heart{m.zfill(2)}" for m in re.findall(r"heart_(\d+)", search_text)
            )
        )
        if hc or "heart_00" in search_text:
            if not hc:
                hc = ["heart00"]
        if hc:
            d["heart_colors"] = hc
    # For modify_required_hearts, set value = per-color count (not total).
    # The icon sequence {heart02×3, heart03×3, ...} means 3 per color, not 12 total.
    # When all colors have the same count, value = that per-color count.
    if d.get("action") == "modify_required_hearts" and "value" not in d:
        search_val = d_text or ""
        if not re.search(r"heart_\d+", search_val) and ctx_text:
            search_val = ctx_text
        target_colors = d.get("heart_colors", [])
        color_counts = {}
        for m in re.finditer(r"heart_(\d+)", search_val):
            h = f"heart{m.group(1).zfill(2)}"
            if not target_colors or h in target_colors:
                color_counts[h] = color_counts.get(h, 0) + 1
        if color_counts:
            counts = list(color_counts.values())
            if len(set(counts)) == 1:
                d["value"] = counts[0]
            else:
                d["value"] = min(counts)

    # Detect possession pattern (を持つ) in gain_resource heart effects:
    # when the text says "member POSSESSING heartXX", the heart_colors
    # should act as a TARGET FILTER, not just the resource to grant.
    # Only set the filter when a heart icon appears BEFORE "を持つ"
    # (e.g. "{{heart_01.png|heart01}}を持つ"), not when the heart is
    # the thing being gained after "を持つ" (e.g. "グループ名を持つ...heart01を得る").
    if (
        d.get("action") == "gain_resource"
        and d.get("resource") in ("heart", "ハート")
        and d.get("heart_colors")
    ):
        d_text_local = d.get("text", "") or ""
        possess_pos = d_text_local.find("を持つ")
        if possess_pos >= 0:
            before = d_text_local[:possess_pos]
            if re.search(r"\{\{heart_\d+\.png\|heart\d+\}\}", before):
                d["filter_targets_by_heart_colors"] = True

    # Extract heart_colors from the node's own text for look_and_select select_actions.
    # Uses d.get("text") instead of ctx_text to avoid inheriting heart colors
    # from unrelated parts of the parent effect (e.g. a followup condition like
    # "30以上の場合、さらにこのカードの必要ハートを{{heart_00.png|heart0}}減らす").
    node_text = d.get("text") or ""
    if "heart_colors" not in d and d.get("action") == "select_cards" and node_text:
        hc = list(
            dict.fromkeys(
                f"heart{m.zfill(2)}" for m in re.findall(r"heart_(\d+)", node_text)
            )
        )
        if hc:
            d["heart_colors"] = hc
            if detect_require_all_hearts(node_text):
                d["require_all_heart_colors"] = True


def _walk_propagate_all_and_targets(d, d_ctx):
    # Propagate all from text context (must match _fill_defaults patterns)
    if (
        "all" not in d
        and d_ctx
        and re.search(r"すべての|全ての|全部の|全て|全員|全体|カードをすべて", d_ctx)
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


def _walk_propagate_sequential_links(d):
    # Propagate count/source from move_cards to subsequent conditions
    # "すべて"/"全部": "If ALL moved cards match" → count=N, operator="="
    # "それらの中に": "If any among moved cards match" → source="preceding_moved"
    if d.get("action") == "sequential" and "actions" in d:
        acts = d["actions"]
        prev_cost_limit = None
        prev_cost_op = None
        prev_count = None
        prev_pm_cond = None
        for act in acts:
            # Propagate cost_limit from select to subsequent reveal
            if act.get("action") == "select":
                prev_cost_limit = act.get("cost_limit")
                prev_cost_op = act.get("cost_limit_operator")
            elif act.get("action") == "reveal" and prev_cost_limit is not None:
                if "cost_limit" not in act:
                    act["cost_limit"] = prev_cost_limit
                if "cost_limit_operator" not in act and prev_cost_op is not None:
                    act["cost_limit_operator"] = prev_cost_op
            if act.get("action") == "move_cards" and act.get("count"):
                prev_count = act["count"]
            # Also look inside a nested sequential (e.g. draw+discard grouped together)
            elif act.get("action") == "sequential" and "actions" in act:
                for _sub in act["actions"]:
                    if _sub.get("action") == "move_cards" and _sub.get("count"):
                        prev_count = _sub["count"]
            cond = act.get("condition", {})
            cond_text = cond.get("text", "")
            if (
                "すべて" in cond_text or "全部" in cond_text
            ) and prev_count is not None:
                if cond.get("type") == "card_count_condition":
                    if cond.get("count", 1) == 1 and cond.get("operator", ">=") == ">=":
                        cond["count"] = prev_count
                        cond["operator"] = "="
                        cond["source"] = "preceding_moved"
                elif cond.get("type") == "group_condition":
                    cond["type"] = "card_count_condition"
                    cond["count"] = prev_count
                    cond["operator"] = "="
                    cond["source"] = "preceding_moved"
            if (
                "それらの中に" in cond_text or "これにより" in cond_text
            ) and prev_count is not None:
                if cond.get("type") in (
                    "card_count_condition",
                    "location_condition",
                    "group_condition",
                ) and cond.get("source") in (None, "card", "discard"):
                    cond["type"] = "card_count_condition"
                    cond["source"] = "preceding_moved"
                    cond.pop("location", None)
            # Track the last preceding_moved condition so bare follow-up count
            # conditions ("2枚ある場合") can inherit its filter fields.
            if cond.get("source") == "preceding_moved":
                prev_pm_cond = cond
            elif (
                prev_pm_cond is not None
                and cond.get("type") == "card_count_condition"
                and cond.get("source") is None
                and "location" not in cond
                and "card_type" not in cond
                and re.search(r"^\d+枚ある場合$", cond_text.strip())
            ):
                # Bare count escalation — inherit the preceding_moved filter
                cond["source"] = "preceding_moved"
                for _key in ("card_type", "negation", "card_property"):
                    if _key in prev_pm_cond and _key not in cond:
                        cond[_key] = prev_pm_cond[_key]


def _walk_collapse_sequential(d):
    # Collapse single-action sequential wrappers (preserve condition + trigger_type + text)
    if d.get("action") == "sequential" and d.get("actions") and len(d["actions"]) == 1:
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


def _walk_set_defaults(d, d_text, ct):
    # Default target to "self" for location_conditions if missing
    if d.get("type") == "location_condition" and "target" not in d:
        d["target"] = "self"

    # Infer operator for comparison conditions when counts are present
    if ct in ("comparison_condition", "card_count_condition", "location_condition"):
        # Always override for "以上"/"以下" even if operator was pre-set.
        # Check "以下" BEFORE "以上" so that compound texts like
        # "1枚以上公開...2枚以下の場合" pick ≤2 (the real condition),
        # not ≥1 (the trigger clause).
        _text = d.get("text", "")
        if d.get("count") is not None and not d.get("comparison_target"):
            if "以下" in _text:
                d["operator"] = "<="
            elif "以上" in _text:
                d["operator"] = ">="
            elif "operator" not in d:
                d["operator"] = "="
        if "operator" not in d:
            if d.get("values"):
                d["operator"] = "in"
            elif d.get("comparison_target"):
                if "高い" in _text or "多い" in _text or "大きい" in _text:
                    d["operator"] = ">"
                elif "低い" in _text or "少ない" in _text or "小さい" in _text:
                    d["operator"] = "<"

    # Infer count from cost_limit for comparison_conditions (non-cost comparisons)
    if (
        ct == "comparison_condition"
        and "count" not in d
        and d.get("cost_limit") is not None
        and d.get("comparison_type") != "cost"
    ):
        d["count"] = d["cost_limit"]

    # Default per_unit_count to 1 when missing
    if d.get("per_unit") and "per_unit_count" not in d:
        d["per_unit_count"] = 1


def _walk_propagate_position(d, d_ctx, d_text):
    # Propagate position from text context (for condition+action splits)
    # Don't set position if source_position or exclude_position already set,
    # or if positions_characters already encodes per-position mappings.
    # Also don't set position when multiple positions are detected (comma-separated):
    # "left_side,right_side" belongs in activation_position, not position.
    # The position field is a cross-comparison filter expecting a single value.
    if (
        "position" not in d
        and "source_position" not in d
        and "exclude_position" not in d
        and "positions_characters" not in d
        and d_ctx
    ):
        pos = _has_position_keywords(d_ctx)
        if pos and "," not in pos:
            d["position"] = pos
        elif pos:
            # Multi-position: set activation_position instead
            if "activation_position" not in d:
                d["activation_position"] = pos

    # Strip {{center.png|センター}} etc from text when extracted as position
    if d.get("position") and d_text:
        d["text"] = re.sub(
            r"\{\{.+?\.png\|(?:センター|左サイド|右サイド)\}\}", "", d_text
        ).strip()

    # Mark original_value flag for 元々持つ patterns
    if "original_value" not in d and d_text and _has_original_modifier(d_text):
        d["original_value"] = True


def _walk_propagate_flags(d, d_ctx):
    # Mark group_reference for non-bracket group name patterns (safe string field)
    if "group_reference" not in d and d_ctx:
        if "同じグループ名" in d_ctx:
            d["group_reference"] = "same_group_name"
        elif (
            "グループ名が異なる" in d_ctx
            or "グループ名がそれぞれ異なる" in d_ctx
            or "異なるグループ名" in d_ctx
        ):
            d["group_reference"] = "different_group_names"

    # Set same_unit_name for cost text containing '同じユニット名'
    if "same_unit_name" not in d and "同じユニット名" in (d.get("text", "") or ""):
        d["same_unit_name"] = True


def _walk_propagate_heart_colors_to_conditions(d):
    # Propagate heart_colors from effect into condition for collective heart checks.
    # Only propagate when the condition's location is a zone that CAN have heart colors
    # (stage, hand — NOT energy_zone, discard, energy_deck, success_live_zone which store colorless game pieces).
    # Also skip card_count_condition (pure count check — heart colors don't apply).
    # Skip check_self conditions (they check a specific card's location, not collective
    # heart presence — heart_colors on the condition is effect metadata leakage).
    # Q148: Skip blade-aggregate conditions — "ブレードの合計がN以上" is a blade total
    # check, not a heart filter. The heart_colors in the effect text is the modification
    # target (what gets decreased), not a condition filter (what triggers the effect).
    if "heart_colors" in d and "condition" in d:
        cond = d["condition"]
        if isinstance(cond, dict) and "heart_colors" not in cond:
            cond_type = cond.get("type", "")
            loc = cond.get("location", "")
            cond_text = cond.get("text", "")
            if (
                cond_type == "card_count_condition"
                and cond.get("source") != "preceding_moved"
            ):
                pass
            elif cond.get("check_self"):
                pass
            elif "ブレード" in cond_text and cond.get("aggregate") == "total":
                pass
            elif loc in ("stage", "hand", "live_card_zone", ""):
                if cond_type == "or_condition":
                    for sub in cond.get("conditions", []):
                        if isinstance(sub, dict) and "heart_colors" not in sub:
                            sub["heart_colors"] = d["heart_colors"]
                elif cond_type in ("location_condition",):
                    cond["heart_colors"] = d["heart_colors"]


def _walk_cleanup_text(d, d_text):
    # Strip leading comma from text artifacts (e.g. "、{{icon_energy.png|E}}支払ってもよい")
    if d_text and (d_text.startswith("、") or d_text.startswith("，")):
        d["text"] = d_text.lstrip("、，").strip()


def _walk(d, full_text, original_text, ctx_text=None):
    if not isinstance(d, dict):
        return d
    d_ctx = d.get("text") or ctx_text or full_text
    d_text = d.get("text") or ""
    ct = d.get("condition_type") or d.get("type")

    _walk_ensure_text(d, ctx_text, full_text)
    _walk_propagate_activation_position(d, original_text, full_text)
    _walk_propagate_text_context_fields(d, d_ctx, ctx_text)
    _walk_extract_heart_colors(d, d_text, ctx_text)
    _walk_propagate_all_and_targets(d, d_ctx)
    _walk_propagate_sequential_links(d)
    _walk_collapse_sequential(d)
    _walk_set_defaults(d, d_text, ct)
    _walk_propagate_position(d, d_ctx, d_text)
    _walk_propagate_flags(d, d_ctx)
    _walk_propagate_heart_colors_to_conditions(d)
    _walk_cleanup_text(d, d_text)

    # Recurse into sub-actions
    for sub_key in (
        "actions",
        "options",
        "conditions",
        "condition",
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
            if sub_key == "actions":
                d[sub_key] = _clean_action_list(sub, d, d_ctx)
                sub = d[sub_key]
            for item in sub:
                _walk(item, full_text, original_text, d_ctx)
        elif isinstance(sub, dict):
            _walk(sub, full_text, original_text, d_ctx)

    # Clean up empty actions
    if d.get("action") == "sequential" and not d.get("actions"):
        d.pop("action", None)

    return d


def _normalize_effect_tree(effect, original_text=None):
    if not effect or not isinstance(effect, dict):
        return effect
    _full_text = effect.get("text") or original_text or ""
    effect = _walk(effect, _full_text, original_text, original_text)
    effect = _collapse_position_changes(effect)
    _enrich_gain_abilities(effect)
    _enrich_characters(effect)
    _clean_gain_resource(effect)
    return effect


def _collapse_position_changes(node):
    if isinstance(node, dict):
        for v in node.values():
            _collapse_position_changes(v)
        if node.get("action") == "sequential":
            acts = node.get("actions", [])
            collapsed = []
            skip_next = False
            for i, act in enumerate(acts):
                if skip_next:
                    skip_next = False
                    continue
                if isinstance(act, dict) and act.get("action") == "position_change":
                    if (
                        i + 1 < len(acts)
                        and isinstance(acts[i + 1], dict)
                        and acts[i + 1].get("action") == "gain_resource"
                    ):
                        gr = dict(acts[i + 1])
                        gr["timing_condition"] = "moved_this_turn"
                        for f in ("card_type", "all", "target"):
                            if f == "target" and not gr.get("target"):
                                gr["target"] = "self"
                            elif act.get(f) and not gr.get(f):
                                gr[f] = act[f]
                        collapsed.append(gr)
                        skip_next = True
                        continue
                if isinstance(act, dict) and act.get("action") == "sequential":
                    sub_acts = act.get("actions", [])
                    if len(sub_acts) == 1:
                        item = dict(sub_acts[0])
                        for f in ("duration", "all", "card_type", "target"):
                            if act.get(f) and not item.get(f):
                                item[f] = act[f]
                        collapsed.append(item)
                        continue
                collapsed.append(act)
            node["actions"] = collapsed
    elif isinstance(node, list):
        for item in node:
            _collapse_position_changes(item)
    return node


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


def _enrich_gain_abilities(effect):
    gain_nodes = []
    _collect_gain(effect, gain_nodes)
    for node in gain_nodes:
        if "gained_effect" not in node:
            clean_gain = re.sub(r"【[^】]+】", "", node["ability_gain"]).strip()
            gained = parse_effect(clean_gain)
            if gained and gained.get("action") and gained.get("action") != "custom":
                node["gained_effect"] = gained


def _enrich_characters(d):
    if isinstance(d, dict):
        if d.get("action") in (
            "sequential",
            "conditional_on_optional",
            "conditional_on_result",
            "conditional_alternative",
            "look_and_select",
        ):
            pass
        elif "text" in d:
            text = d["text"]
            if not d.get("characters"):
                cm = re.search(
                    r"((?:「[^」]+」[か、]? ?)+)の(?:メンバーカード|ライブカード)", text
                )
                if cm:
                    names = re.findall(r"「([^」]+)」", cm.group(1))
                    if names:
                        d["characters"] = names
            if not d.get("card_names"):
                cn = re.search(r"カード名(?:に|が)「([^」]+)」", text)
                if cn:
                    d["card_names"] = [cn.group(1)]
        for v in d.values():
            if isinstance(v, (dict, list)):
                _enrich_characters(v)
    elif isinstance(d, list):
        for item in d:
            if isinstance(item, (dict, list)):
                _enrich_characters(item)


# ====================================================================
# ====================================================================
# PROCESSING: process_abilities() & parse_ability()
# ====================================================================
# Top-level orchestration: loads abilities.json, runs post-processing
# fixes, semantic validation, and targeted patches for known parser gaps.
# Also handles gain_ability text re-parsing and the main entry point.
# ====================================================================


def _clean_gain_resource(node):
    """Recursively remove inappropriate fields from gain_resource action nodes."""
    if isinstance(node, dict):
        if node.get("action") == "gain_resource":
            res = node.get("resource")
            if res in ("blade", "ブレード"):
                node.pop("heart_colors", None)
            node.pop("source", None)
            # Remove position from gain_resource when the condition has the same
            # position. Position on gain_resource tells the engine which card
            # position must hold the ability card (pos_ok gate in recalculate_constants).
            # That's wrong when position describes something else:
            # - Trigger conditions: position = WHERE the event happened (not target)
            # - Comparison with comparison_type: position = which member to compare
            #   (e.g. "center has highest cost" — position is the subject, not target)
            # Keep position when condition has resource_type (e.g. heart_02 >= 3 at
            # left_side) because position IS the effect target.
            cond = node.get("condition", {})
            if node.get("position") and cond.get("position") == node.get("position"):
                trigger_types = (
                    "movement_condition",
                    "appearance_condition",
                    "baton_touch",
                    "state_change_condition",
                )
                is_trigger = cond.get("type") in trigger_types
                # comparison_type (e.g. "cost") means position is the comparison
                # subject, not the effect target — strip it.
                is_comparison_subject = cond.get("comparison_type") is not None
                # highest_cost_on_stage: position describes WHICH member to check,
                # not where the ability card must be.
                is_highest_cost = cond.get("type") == "highest_cost_on_stage_condition"
                if is_trigger or is_comparison_subject or is_highest_cost:
                    node.pop("position", None)
        for v in node.values():
            _clean_gain_resource(v)
    elif isinstance(node, list):
        for item in node:
            _clean_gain_resource(item)


def _clean_per_unit_source(d):
    """Recursively remove redundant fields on perform_yell with per_unit_source.

    When per_unit_source is 'previous_moved_cards', the engine sums costs from
    self.moved_cards directly — per_unit_type and count are unused for yell.
    """
    if isinstance(d, dict):
        if (
            d.get("per_unit")
            and d.get("action") == "perform_yell"
            and d.get("per_unit_source") == "previous_moved_cards"
        ):
            d.pop("per_unit_type", None)
            d.pop("count", None)
        for v in d.values():
            _clean_per_unit_source(v)
    elif isinstance(d, list):
        for item in d:
            _clean_per_unit_source(item)


def _propagate_context(node, ctx=None, *, t="", eff_root=None):
    if not isinstance(node, dict):
        return
    if ctx is None:
        ctx = {}

    action = node.get("action")
    ct = node.get("condition_type") or node.get("type")

    # Apply operator inference for condition-type nodes
    if ct in (
        "comparison_condition",
        "card_count_condition",
        "location_condition",
    ):
        _text = node.get("text", "")
        if node.get("count") is not None and not node.get("comparison_target"):
            if "以下" in _text:
                node["operator"] = "<="
            elif "以上" in _text:
                node["operator"] = ">="
            elif "operator" not in node:
                node["operator"] = "="
        if "operator" not in node:
            if node.get("values"):
                node["operator"] = "in"
            elif node.get("comparison_target"):
                if "高い" in _text or "多い" in _text or "大きい" in _text:
                    node["operator"] = ">"
                elif "低い" in _text or "少ない" in _text or "小さい" in _text:
                    node["operator"] = "<"
        # Infer count from cost_limit for score-based comparisons
        if (
            ct == "comparison_condition"
            and "count" not in node
            and node.get("cost_limit") is not None
            and node.get("comparison_type") != "cost"
        ):
            node["count"] = node["cost_limit"]

    # Inherit location into conditions
    if isinstance(node.get("condition"), dict):
        nc = node["condition"]
        if nc.get("type") in ("comparison_condition", "card_count_condition"):
            if not nc.get("location") and ctx.get("location"):
                nc["location"] = ctx["location"]
            if not nc.get("target") and ctx.get("target"):
                nc["target"] = ctx["target"]
            if not nc.get("card_type") and ctx.get("card_type"):
                nc["card_type"] = ctx["card_type"]

    # Inherit duration and target into action-type dicts
    if action in (
        "gain_resource",
        "change_state",
        "move_cards",
        "select",
    ):
        if not node.get("duration") and ctx.get("duration"):
            node["duration"] = ctx["duration"]
        if not node.get("target") and ctx.get("target"):
            node["target"] = ctx["target"]
    # draw_card: only inherit duration if context has it and action type
    # is not a setup step (drawing itself is instantaneous)
    if action == "draw_card" and not node.get("duration") and ctx.get("duration"):
        act_text = node.get("text", "") or ""
        if "得る" in act_text or "を得る" in act_text or "得られる" in act_text:
            node["duration"] = ctx["duration"]
        if not node.get("target") and ctx.get("target"):
            node["target"] = ctx["target"]
        if not node.get("all") and ctx.get("all"):
            ap = node.get("activation_position") or ctx.get("activation_position")
            if ap not in ("center", "left_side"):
                if action == "move_cards" and ctx.get("source") == node.get("source"):
                    pass
                elif node.get("count"):
                    pass
                else:
                    node["all"] = ctx.get("all")

    # Inherit timing_condition into gain_resource actions
    if action == "gain_resource":
        if not node.get("timing_condition") and ctx.get("timing_condition"):
            node["timing_condition"] = ctx["timing_condition"]

    # Build context for children
    new_ctx = dict(ctx)
    for f in (
        "location",
        "target",
        "card_type",
        "duration",
        "timing_condition",
        "all",
    ):
        if f in node:
            new_ctx[f] = node[f]

    # Inherit location into compound sub-conditions (from compound's own context)
    if node.get("type") == "compound" and "conditions" in node:
        # Also inherit from first sub-condition to later ones (e.g. "ライブ中のカードが
        # 3枚以上あり、その中に『虹ヶ咲』のライブカードを1枚以上" → both should use
        # live_card_zone from the first sub-condition).
        first_loc = None
        for sub in node["conditions"]:
            if isinstance(sub, dict) and sub.get("location"):
                first_loc = sub["location"]
                break
        for sub in node["conditions"]:
            if isinstance(sub, dict):
                loc_source = new_ctx.get("location") or first_loc
                if loc_source and not sub.get("location"):
                    # Don't propagate location to score comparison conditions
                    # — they need live_card_zone / success_live_zone, not stage
                    if (
                        not sub.get("temporal")
                        and not sub.get("resource_type")
                        and sub.get("comparison_type") != "score"
                    ):
                        sub["location"] = loc_source
                _propagate_context(sub, new_ctx, t=t, eff_root=eff_root)

    # Restore duration for sequential sub-actions when the parser lost it
    # during sequential creation (e.g., "と" split targets).
    if action == "sequential" and not new_ctx.get("duration") and t:
        if "ライブ終了時まで" in t:
            for act in node.get("actions", []):
                if isinstance(act, dict) and act.get("action") in (
                    "gain_resource",
                    "change_state",
                    "move_cards",
                ):
                    if act.get("duration") is None and "得る" in (
                        act.get("text", "") or ""
                    ):
                        act["duration"] = "live_end"

    for ck in (
        "condition",
        "primary_effect",
        "followup_action",
        "optional_action",
        "conditional_action",
        "alternative_effect",
    ):
        ch = node.get(ck)
        if isinstance(ch, dict):
            _propagate_context(ch, new_ctx, t=t, eff_root=eff_root)
            # Propagate location from condition back to the parent
            # context so sibling actions in a sequential inherit it.
            # Only propagate location — target and card_type are too
            # context-dependent and would be incorrectly injected.
            for f in ("location",):
                if f in ch and f not in node and f not in ctx:
                    ctx[f] = ch[f]
            # Also inherit missing location from parent context into condition
            for f in ("location",):
                if f not in ch and f in ctx:
                    ch[f] = ctx[f]
        # For conditional_on_result: propagate card_type from the outer
        # condition into the primary_effect's condition when missing.
        if (
            action == "conditional_on_result"
            and ck == "primary_effect"
            and isinstance(ch, dict)
            and eff_root
        ):
            outer_cond = eff_root.get("condition", {})
            if isinstance(outer_cond, dict) and outer_cond.get("card_type"):
                nested_cond = ch.get("condition", {})
                if isinstance(nested_cond, dict) and not nested_cond.get("card_type"):
                    nested_cond["card_type"] = outer_cond["card_type"]

    for ak in ("actions", "options"):
        arr = node.get(ak, [])
        if isinstance(arr, list):
            for item in arr:
                _propagate_context(item, new_ctx, t=t, eff_root=eff_root)

    # Baton touch trigger on action nodes (e.g. gain_resource with
    # "このターンにバトンタッチして登場した" in a choice option)
    node_text = node.get("text", "") or ""
    _infer_baton_touch(node, node_text)

    # heart_type:all for gain_resource actions
    if action == "gain_resource" and node.get("resource") == "heart":
        if "{{icon_all.png|ハート}}" in (node.get("text", "") or t or ""):
            if not node.get("heart_type") and not node.get("heart_colors"):
                node["heart_type"] = "all"

    # card_property enrichment and heart_colors cleanup for card_count_conditions
    # Check node.get("condition") — works for effect/action nodes
    nc = node.get("condition")
    if isinstance(nc, dict) and nc.get("type") == "card_count_condition":
        nct = nc.get("text", "")
        if "ブレードハートを持たない" in nct or "ブレードハートがない" in nct:
            if not nc.get("card_property"):
                nc["card_property"] = "has_blade_heart"
        if "{{icon_score.png|スコア}}を持つ" in nct and not nc.get("card_property"):
            nc["card_property"] = "has_score_icon"
        _infer_heart_source(nc, nct)
        _infer_baton_touch(nc, nct)
        # Strip heart_colors from preceding_moved conditions that have a
        # specific location — the move already filtered by heart color.
        if (
            nc.get("source") == "preceding_moved"
            and nc.get("location")
            and nc.get("heart_colors")
        ):
            nc.pop("heart_colors", None)
    # Q148: Strip heart_colors from blade-aggregate location_conditions.
    # "ブレードの合計がN以上" is a blade total check, not a heart filter.
    # heart_colors in the effect text is the modification target, not a condition filter.
    nc = node.get("condition")
    if isinstance(nc, dict) and nc.get("type") == "location_condition":
        nc_text = nc.get("text", "")
        if (
            "ブレード" in nc_text
            and nc.get("aggregate") == "total"
            and nc.get("heart_colors")
        ):
            nc.pop("heart_colors", None)
    # Also check if node itself IS a card_count_condition (sub-condition
    # of a compound — no "condition" child, it IS the condition).
    if node.get("type") == "card_count_condition" and node is not nc:
        nct = node.get("text", "")
        if "ブレードハートを持たない" in nct or "ブレードハートがない" in nct:
            if not node.get("card_property"):
                node["card_property"] = "has_blade_heart"
        if "{{icon_score.png|スコア}}を持つ" in nct and not node.get("card_property"):
            node["card_property"] = "has_score_icon"
        _infer_heart_source(node, nct)
        _infer_baton_touch(node, nct)

    # Strip parenthetical from sub-conditions of compound conditions
    if node.get("type") == "compound" and "conditions" in node:
        for sub in node["conditions"]:
            if isinstance(sub, dict) and isinstance(sub.get("parenthetical"), list):
                sub.pop("parenthetical", None)

    # movement_condition card_type
    nc = node.get("condition")
    if isinstance(nc, dict) and nc.get("type") == "movement_condition":
        if nc.get("ability_filter") and not nc.get("card_type"):
            nc["card_type"] = "member_card"

    # temporal_condition location — for aggregate conditions about live card
    # required hearts (all 6 heart colors, not per-color totals)
    nc = node.get("condition")
    if isinstance(nc, dict) and nc.get("type") == "temporal_condition":
        ct = nc.get("text", "") or ""
        if nc.get("aggregate") == "total" and "必要ハート" in ct:
            if "成功" not in ct and not nc.get("location"):
                hc = nc.get("heart_colors") or []
                if len(hc) >= 6 or "{{icon_all.png" in ct:
                    nc["location"] = "live_card_zone"

    # Inherit target from condition into gain_resource effect (each_time
    # abilities: the condition's target identifies the member that triggered
    # the each_time, and the gain_resource effect needs it to know which card
    # receives the modifier).
    # NOTE: "both" is NOT inherited — a condition's "both" means checking
    # both sides (e.g. "self and opponent's success zones"), NOT applying
    # the effect to both players.
    if isinstance(eff_root, dict) and eff_root.get("action") == "gain_resource":
        if not eff_root.get("target") and eff_root.get("resource") == "heart":
            nc = eff_root.get("condition")
            if isinstance(nc, dict) and nc.get("target"):
                ct = nc["target"]
                if ct == "both":
                    eff_root["target"] = "self"
                else:
                    eff_root["target"] = ct


def _apply_recursive_fixes(d, fix_stats):
    if isinstance(d, dict):
        if d.get("type") == "appearance_condition" and "控え室から" in d.get(
            "text", ""
        ):
            if "appearance_source" not in d:
                d["appearance_source"] = "discard"
                fix_stats["appearance_source"] = (
                    fix_stats.get("appearance_source", 0) + 1
                )

        if d.get("action") == "move_cards":
            t = d.get("text", "")
            if "これにより控え室に置いた" in t and "より" in t and "コストの低い" in t:
                if "cost_reference" not in d:
                    d["cost_reference"] = "previous_moved_card"
                    d["cost_limit_operator"] = "<"
                    fix_stats["cost_reference"] = fix_stats.get("cost_reference", 0) + 1

        if (
            d.get("action") == "modify_score"
            and d.get("per_unit_type") == "member"
            and d.get("heart_colors")
        ):
            t = d.get("text", "")
            if "色につき" in t:
                d["per_unit_type"] = "heart_colors"
                fix_stats["heart_colors_per_unit"] = (
                    fix_stats.get("heart_colors_per_unit", 0) + 1
                )

        for v in d.values():
            if isinstance(v, (dict, list)):
                _apply_recursive_fixes(v, fix_stats)
    elif isinstance(d, list):
        for item in d:
            if isinstance(item, (dict, list)):
                _apply_recursive_fixes(item, fix_stats)


def _process_pre_fix(ability: Dict[str, Any], fix_stats: Dict[str, int]) -> None:
    """Pre-fix pass: condition re-parse, target fix, action inference, sequential chain fixes, targeted fixes."""
    # ─── Pre-fix pass (merged from 3 separate loops) ───────────────────────────
    # 1. Re-parse condition texts to pick up newer parser fields (cost_limit etc.)
    # 2. Fix target=both when comparison_target is set → target should be self.
    # 3. Infer action for effects with no action; apply sequential chain fixes.
    # ─────────────────────────────────────────────────────────────────────────────
    ability_text = ability.get("full_text") or ability.get("triggerless_text", "")
    if ability_text:
        set_debug_section(f"Ability: {ability_text}")
    eff = ability.get("effect")
    if not isinstance(eff, dict):
        return
    cond = eff.get("condition")

    # --- 1. Pre-fix pass logic ---

    # --- 1. Condition re-parse ---
    if isinstance(cond, dict) and cond.get("text"):
        cond_text = cond["text"]
        reparse = parse_condition(cond_text)
        if reparse:
            # Merge missing fields — never overwrite existing values.
            for key in ("cost_limit", "cost_limit_operator"):
                if key in reparse and key not in cond:
                    cond[key] = reparse[key]
            # Merge movement and direction fields.
            if "movement" in reparse and reparse["movement"] != cond.get("movement"):
                cond["movement"] = reparse["movement"]
            if "area_direction" in reparse and "area_direction" not in cond:
                cond["area_direction"] = reparse["area_direction"]
        # Fix missing yell_trigger for "エールにより公開された" cards.
        # Only applies to auto abilities (自動) — not ライブ成功時 etc.
        # Skip when the condition checks for a specific card_type with negation
        # (e.g. "no live card among revealed") — yell_trigger would short-circuit
        # to "did a yell happen?" instead of checking the actual revealed cards.
        if ability.get("triggers") == "自動":
            has_yell_text = (
                "エールしたとき" in cond_text or "エールにより公開された" in cond_text
            )
            has_specific_type_check = cond.get("card_type") and cond.get("negation")
            if (
                cond.get("yell_trigger") is None
                and has_yell_text
                and not has_specific_type_check
            ):
                cond["yell_trigger"] = True

    # --- 2. Fix target=both when comparison_target is set ---
    # comparison_target handles the opponent side, so target should be self.
    if isinstance(cond, dict):
        if cond.get("target") == "both" and cond.get("comparison_target"):
            cond["target"] = "self"

    # --- 3. Action inference ---
    if not eff.get("action"):
        if eff.get("source") and eff.get("destination"):
            eff["action"] = "move_cards"
        elif eff.get("actions"):
            eff["action"] = "sequential"
        elif eff.get("opponent_action"):
            eff["action"] = "opponent_action"

    # --- 4. Sequential chain fixes ---
    if eff.get("action") == "sequential":
        parent_card_type = eff.get("card_type")
        prev_was_select = False
        prev_was_look_at = False
        prev_was_baton_touch = False
        for sub in eff.get("actions", []):
            if not isinstance(sub, dict):
                prev_was_select = prev_was_look_at = prev_was_baton_touch = False
                continue
            # Propagate card_type from parent and infer missing sub-action.
            if not sub.get("card_type") and parent_card_type:
                sub["card_type"] = parent_card_type
            if not sub.get("action"):
                if sub.get("source") and sub.get("destination"):
                    sub["action"] = "move_cards"
                elif sub.get("actions"):
                    sub["action"] = "sequential"
            # Chain select→move, look_at→move, and play_baton_touch→move source inference.
            if sub.get("action") in ("select_cards", "look_and_select", "select"):
                prev_was_select = True
                prev_was_look_at = prev_was_baton_touch = False
            elif sub.get("action") == "look_at":
                prev_was_look_at = True
                prev_was_select = prev_was_baton_touch = False
            elif sub.get("action") == "play_baton_touch":
                prev_was_baton_touch = True
                prev_was_select = prev_was_look_at = False
            elif sub.get("action") == "move_cards" and prev_was_look_at:
                if sub.get("source") != "looked_at":
                    sub["source"] = "looked_at"
                if (
                    sub.get("destination") == "discard"
                    and sub.get("discard_remaining") is not False
                ):
                    sub["discard_remaining"] = False
                prev_was_look_at = False
            elif sub.get("action") == "move_cards" and prev_was_select:
                if sub.get("source") != "selected_cards":
                    sub["source"] = "selected_cards"
                if sub.get("count") is not None and "count" not in sub.get("text", ""):
                    sub.pop("count", None)
                prev_was_select = False
            elif sub.get("action") == "move_cards" and prev_was_baton_touch:
                if sub.get("source") != "those_cards":
                    sub["source"] = "those_cards"
                if not sub.get("group_names") and eff.get("group_names"):
                    sub["group_names"] = eff["group_names"]
                prev_was_baton_touch = False
            elif (
                sub.get("action") == "gain_resource"
                and sub.get("resource") == "heart"
                and prev_was_select
            ):
                sub_text = sub.get("text", "")
                if (
                    "選んだカードが持つ色" in sub_text
                    or "これにより選んだカード" in sub_text
                ):
                    sub["heart_colors_from_selected_card"] = True
                prev_was_select = False
            else:
                prev_was_select = prev_was_look_at = prev_was_baton_touch = False

    # --- 2. Targeted fixes logic ---
    t = ability.get("triggerless_text", "")

    # FIX 2: each_time sequential → conditional_on_optional
    if eff.get("trigger_type") == "each_time" and eff.get("action") == "sequential":
        acts = eff.get("actions", [])
        if len(acts) == 2:
            first, second = acts[0], acts[1]
            if isinstance(first, dict) and isinstance(second, dict):
                if (
                    first.get("action") == "pay_energy"
                    and first.get("optional") is True
                ):
                    for leak in ("exclude_self", "group_names", "optional"):
                        first.pop(leak, None)
                    eff["action"] = "conditional_on_optional"
                    eff["optional_action"] = first
                    eff["conditional_action"] = second
                    eff.pop("actions", None)
                    fix_stats["each_time"] += 1

    # FIX 3: conditional_on_optional cleanup
    if eff.get("action") == "conditional_on_optional":
        if "positive_action" in eff and "conditional_action" not in eff:
            eff["conditional_action"] = eff.pop("positive_action")
        eff.pop("negative_action", None)
        for sub_key in ("optional_action", "conditional_action"):
            sub = eff.get(sub_key)
            if isinstance(sub, dict):
                sub.pop("optional", None)

    # FIX 6: Flatten opponent_action wrappers
    if eff.get("opponent_action") and isinstance(eff["opponent_action"], dict):
        oa = eff.pop("opponent_action")
        for k, v in oa.items():
            if k not in eff:
                eff[k] = v
        inner_action = oa.get("action")
        if inner_action:
            eff["action"] = inner_action
        eff.setdefault("target", "opponent")
        eff.setdefault("action_by", "opponent")

    # FIX 7: Ability filter — 能力を持たない → ability_filter:no_ability
    if "能力を持たない" in t:
        if eff.get("action") == "modify_cost" and not eff.get("ability_filter"):
            eff["ability_filter"] = "no_ability"
            fix_stats["ability_filter"] += 1

    # FIX 7b: Ability filter for select actions within sequential compounds
    # (e.g. 黒澤ダイヤ: "ライブ開始時能力を持たない" → select action filter)
    if eff.get("action") == "sequential":
        for sub in eff.get("actions", []):
            if isinstance(sub, dict) and sub.get("action") in (
                "select",
                "select_cards",
            ):
                sub_text = sub.get("text", "")
                if "能力を持たない" in sub_text or "能力も持たない" in sub_text:
                    _enrich_from_text(sub, sub_text)
                    fix_stats["ability_filter"] += 1

    # FIX 8: Condition fixes — card_property + enrichment
    cond = eff.get("condition")
    if isinstance(cond, dict):
        ct = cond.get("text", "") or t

        # 8b: card_property enrichment for card_count_condition
        if cond.get("type") == "card_count_condition":
            if "ブレードハート" in ct:
                cond["card_property"] = "has_blade_heart"
                if "持たない" in ct or "ない" in ct:
                    cond["negation"] = True
                fix_stats["card_property"] += 1
            if "{{icon_score.png|スコア}}を持つ" in ct and not cond.get(
                "card_property"
            ):
                cond["card_property"] = "has_score_icon"
                fix_stats["card_property"] += 1
            _infer_heart_source(cond, ct)
            _infer_baton_touch(cond, ct)

        # 8d: Enrich temporal_condition with aggregate
        if cond.get("type") == "temporal_condition":
            changed = False
            hc = len(re.findall(r"\{\{heart_\d+\.png", ct))
            has_req_heart = "必要ハート" in ct
            has_aggregate_keyword = "含まれ" in ct or "のうち" in ct
            has_total_or_each = "合計" in ct or "それぞれ" in ct
            if has_req_heart and has_aggregate_keyword and has_total_or_each:
                cond["aggregate"] = "total"
                changed = True
            if not cond.get("heart_colors"):
                hm = re.findall(
                    r"{{heart_(\d+)\.png\|heart\d+}}", cond.get("text", "") or ct
                )
                if hm:
                    cond["heart_colors"] = sorted(set(f"heart{m.zfill(2)}" for m in hm))
                    changed = True
            ct2 = cond.get("text", "") or ct
            if not cond.get("count"):
                cm = re.search(r"(\d+)以上", ct2)
                if cm:
                    cond["count"] = int(cm.group(1))
                    changed = True
            if changed:
                fix_stats["temporal"] += 1

        # Remove check_self (reference doesn't have it)

    # FIX 8e: card_property: has_score_icon for effects/conditions with score icon
    if "{{icon_score.png|スコア}}を持つ" in t:
        # Set on the effect for move_cards/select actions
        if eff.get("action") in ("move_cards", "select") and not eff.get(
            "card_property"
        ):
            eff["card_property"] = "has_score_icon"
            fix_stats["card_property"] += 1
        # Also set on the condition (any action type including modify_score)
        cond = eff.get("condition")
        if isinstance(cond, dict) and not cond.get("card_property"):
            cond["card_property"] = "has_score_icon"
            fix_stats["card_property"] += 1

    # FIX 8f: need_heart_total from "ハートをNつ以上持つ" (heart threshold)
    # Only in per_unit contexts (move_cards already handled by _try_virtuoso_move_cards).
    if (
        eff.get("per_unit")
        and not eff.get("need_heart_total")
        and not eff.get("dynamic_count")
    ):
        nh = re.search(r"ハートを(\d+)つ以上持つ", t)
        if nh:
            eff["need_heart_total"] = int(nh.group(1))
            eff["need_heart_operator"] = ">="
            fix_stats["need_heart"] = fix_stats.get("need_heart", 0) + 1

    # FIX 9: Result condition enrichment in conditional_on_result
    rc = eff.get("result_condition")
    if isinstance(rc, dict) and not rc.get("card_property"):
        rct = rc.get("text", "")
        if "ブレードハート" in rct:
            rc["card_property"] = "has_blade_heart"
            if "持たない" in rct or "ない" in rct:
                rc["negation"] = True
            fix_stats["result_cond"] += 1
        if "{{icon_score.png|スコア}}を持つ" in rct:
            rc["card_property"] = "has_score_icon"
            fix_stats["result_cond"] += 1
        _infer_heart_source(rc, rct)
        _infer_baton_touch(rc, rct)

    # FIX 9b: followup_action with "このメンバー" → self_target / self_cost
    fa = eff.get("followup_action")
    if isinstance(fa, dict) and "このメンバー" in fa.get("text", ""):
        if not fa.get("target") and fa.get("self_target") is None:
            fa["self_target"] = True
        # For change_state, also set self_cost so the Rust handler
        # restricts targets to the activating card and skips when
        # the card is already in the target state (e.g. already wait).
        if fa.get("action") == "change_state" and fa.get("self_cost") is None:
            fa["self_cost"] = True

    # FIX 10: Primary effect fixes — negation condition
    pe = eff.get("primary_effect")
    if isinstance(pe, dict):
        pet = pe.get("text", "") or ""
        # Negation condition from text — extract just the condition part (before first 、after とき)
        if (
            not pe.get("condition")
            and ("ない" in pet or "いない" in pet)
            and "とき" in pet
        ):
            idx = pet.find("とき")
            if idx > 0:
                rest = pet[idx + 2 :]
                comma = rest.find("、")
                if comma > 0:
                    neg_text = pet[: idx + 2 + comma]
                else:
                    neg_text = pet[: idx + 2]
            else:
                neg_text = pet
            neg_text = neg_text.rstrip("。")
            neg_cond = {
                "type": "location_condition",
                "location": "revealed_cards",
                "target": "self",
                "text": neg_text,
                "negation": True,
            }
            pe["condition"] = neg_cond
            pe["card_type"] = "card"
            pe.pop("target", None)
            fix_stats["primary_neg"] += 1
        # all:false on single-target primary when parent has all:true
        if eff.get("all") and "all" not in pe and pe.get("count") == 1:
            pe["all"] = False

    # FIX 11: Remove leaking fields from sub-actions in sequential
    if eff.get("action") in ("sequential", "conditional_on_result"):
        for sub in eff.get("actions", []):
            if isinstance(sub, dict):
                if sub.get("action") == "pay_energy":
                    sub.pop("exclude_self", None)
                    sub.pop("group_names", None)
        fa = eff.get("followup_action")
        if isinstance(fa, dict):
            for sub in fa.get("actions", []):
                if isinstance(sub, dict):
                    sub.pop(
                        "activation_position", None
                    ) if "activation_position" in sub else None

    # FIX 12: compound condition → split gain_resource into sequential with two actions
    if isinstance(cond, dict) and cond.get("type") == "compound":
        if eff.get("action") == "gain_resource":
            et = eff.get("text", "") or t
            if "{{icon_all.png|ハート}}" in et and "{{icon_blade.png|ブレード}}" in et:
                blade_count = et.count("{{icon_blade.png|ブレード}}")
                heart_count = et.count("{{icon_all.png|ハート}}")
                actions = [
                    {
                        "action": "gain_resource",
                        "resource": "blade",
                        "count": blade_count,
                        "text": et,
                    },
                    {
                        "action": "gain_resource",
                        "resource": "heart",
                        "heart_type": "all",
                        "count": heart_count,
                        "text": et,
                    },
                ]
                eff["action"] = "sequential"
                eff["actions"] = actions
                eff.pop("resource", None)
                eff.pop("count", None)
                # Propagate group_names from condition or any sub-condition
                if isinstance(cond, dict):
                    gns = cond.get("group_names")
                    if not gns:
                        for sc in cond.get("conditions", []):
                            if isinstance(sc, dict) and sc.get("group_names"):
                                gns = sc["group_names"]
                                break
                    if gns:
                        eff["group_names"] = gns

    # FIX 13: Remove spurious change_state actions inside sequential containers.
    # When _try_implicit_sequential splits "これによりアクティブにしたメンバーと、このメンバーは"
    # the conjunction phrase "これにより...と" is incorrectly parsed as change_state.
    # This pollutes selected_cards with wrong members. Remove the spurious change_state
    # so the subsequent gain_resource with multiple_targets targets correctly.
    if eff.get("action") == "sequential":
        new_actions = []
        for sub in eff.get("actions", []):
            if isinstance(sub, dict) and sub.get("action") == "sequential":
                inner_acts = sub.get("actions", [])
                filtered = [
                    a
                    for a in inner_acts
                    if not (
                        isinstance(a, dict)
                        and a.get("action") == "change_state"
                        and "これにより" in a.get("text", "")
                        and "と" in a.get("text", "")
                    )
                ]
                if len(filtered) < len(inner_acts):
                    if len(filtered) == 1:
                        new_actions.append(filtered[0])
                    else:
                        sub["actions"] = filtered
                        new_actions.append(sub)
                else:
                    new_actions.append(sub)
            else:
                new_actions.append(sub)
        if new_actions != eff.get("actions", []):
            eff["actions"] = new_actions

    # FIX 14: sequential chaining — infer select_cards → move_cards implied source
    fix_stats["compound_split"] += 1

    # FIX 13: Auto abilities with no condition — extract from text
    if (
        ability.get("triggers") == "自動"
        and isinstance(eff, dict)
        and not eff.get("condition")
    ):
        for sep in ["とき、", "場合、", "たび、", "なら、"]:
            idx = t.find(sep)
            if idx >= 0:
                ct = t[: idx + 2]
                tc = parse_condition(ct)
                if tc and tc.get("type") not in (None, "custom"):
                    eff["condition"] = copy.deepcopy(tc)
                    fix_stats["auto_trigger"] += 1
                    break

    # FIX 15: Backfill exclude_self for single-target position_change effects
    # Rule 11.10.1: Position change MUST move to a different area.
    if eff.get("action") == "position_change" and not eff.get("exclude_self"):
        et = eff.get("text", "") or t
        # Only for single-target moves (not "メンバー1人を" or "それぞれ" formation change)
        if "それぞれ" not in et and not (
            "メンバー" in et and ("1人" in et or "N人" in et)
        ):
            eff["exclude_self"] = True
            fix_stats.setdefault("exclude_self", 0)
            fix_stats["exclude_self"] += 1

    # FIX N: All-revealed-match-heart-color condition for specify_heart_color+reveal+select_cards
    if (
        eff.get("action") == "sequential"
        and "公開されたカードの中に" in t
        and "合計" in t
        and "枚含まれる場合" in t
    ):
        acts = eff.get("actions", [])
        cond_m = re.search(r"合計(\d+)枚含まれる場合", t)
        if cond_m:
            cond_count = int(cond_m.group(1))
            cond = {
                "type": "all_revealed_match_heart_color",
                "count": cond_count,
                "operator": ">=",
                "cache": True,
                "text": f"公開されたカードの中に指定した色に合致するカードが合計{cond_count}枚含まれる場合",
            }
            for sub in acts:
                if isinstance(sub, dict) and sub.get("action") in (
                    "select_cards",
                    "gain_resource",
                ):
                    if not sub.get("condition"):
                        sub["condition"] = cond
                    if sub.get("action") == "select_cards":
                        sub.pop("discard_remaining", None)
                # Remove leaked group_names from actions that don't need them
                if isinstance(sub, dict) and sub.get("action") in (
                    "specify_heart_color",
                    "reveal",
                ):
                    sub.pop("group_names", None)


def _process_post_fixes(data: Dict[str, Any], fix_stats: Dict[str, int]) -> None:
    """Post-processing: recursive fixes, action inference & engine compat fixes, post-hoc fixes."""
    _apply_recursive_fixes(data["unique_abilities"], fix_stats)

    # ====================================================================
    # POST-PROCESSING: action inference & engine compat fixes
    # ====================================================================
    # After all abilities are parsed, this section:
    #   1. Infers action types for effects with empty actions
    #   2. Propagates card_type to sub-actions in sequential effects
    #   3. Fixes known engine compatibility gaps
    # ====================================================================

    for ability in data["unique_abilities"]:
        eff = ability.get("effect")
        if not isinstance(eff, dict):
            continue
        t = ability.get("triggerless_text", "")
        cond = eff.get("condition", {})

        # ---- Cost: card_property enrichment ----
        cost = ability.get("cost")
        if isinstance(cost, dict) and not cost.get("card_property"):
            ct = cost.get("text", "") or ""
            if "ブレードハート" in ct:
                cost["card_property"] = "has_blade_heart"
                if "持たない" in ct or "ない" in ct:
                    cost["negation"] = True

        # ---- A: Strip trailing period from primary_effect text ----
        pe = eff.get("primary_effect")
        if isinstance(pe, dict) and isinstance(pe.get("text"), str):
            if pe["text"].endswith("。"):
                pe["text"] = pe["text"].rstrip("。")

            # ---- A1: Structural transforms (keep) ----

        # C: conditional_on_result — N-action sequential with これにより condition
        _acts_pre = eff.get("actions", [])
        _has_blade_heart_seq = (
            eff.get("action") == "sequential"
            and len(_acts_pre) >= 2
            and any(
                a.get("resource") == "blade" for a in _acts_pre if isinstance(a, dict)
            )
            and any(
                a.get("resource") == "heart" for a in _acts_pre if isinstance(a, dict)
            )
        )
        if (
            eff.get("action") == "sequential"
            and "これにより" in t
            and not _has_blade_heart_seq
        ):
            acts = eff.get("actions", [])
            result_idx = -1
            for i, act in enumerate(acts):
                if isinstance(act, dict):
                    c1 = act.get("condition")
                    cond_text = c1.get("text", "") if isinstance(c1, dict) else ""
                    if "これにより" in cond_text:
                        result_idx = i
                        break
                    act_text = act.get("text", "") or ""
                    m = re.match(r"^(これにより.+?場合)[、，]?\s*", act_text)
                    if m:
                        result_idx = i
                        break
            if result_idx < 0:
                for i, act in enumerate(acts):
                    if not isinstance(act, dict):
                        continue
                    act_text = act.get("text", "") or ""
                    m = re.match(r"^(これにより.+?場合)[、，]?\s*(.*)$", act_text)
                    if m:
                        cond_text = m.group(1)
                        action_text = m.group(2)
                        cond_dict = {
                            "text": cond_text,
                            "type": "comparison_condition",
                        }
                        count_m = re.search(r"(\d+)", cond_text)
                        if count_m:
                            cond_dict["count"] = int(count_m.group(1))
                        cond_dict["operator"] = ">="
                        if "余剰ハート" in cond_text:
                            cond_dict["resource_type"] = "surplus_heart"
                        if cond_dict.get("resource_type"):
                            act["condition"] = cond_dict
                            act["text"] = action_text
                        break
            if result_idx >= 0:
                if result_idx > 0:
                    primary_acts = acts[:result_idx]
                    if len(primary_acts) == 1:
                        primary = dict(primary_acts[0])
                        if "text" not in primary:
                            primary["text"] = primary_acts[0].get("text", t)
                    else:
                        primary = {
                            "text": primary_acts[0].get("text", t),
                            "action": "sequential",
                            "actions": [dict(a) for a in primary_acts],
                        }
                else:
                    # result_idx == 0 — no primary actions
                    primary = {"action": "do_nothing"}
                result_act = acts[result_idx]
                c1 = result_act.get("condition", {})
                result_cond = dict(c1)
                if c1.get("type") == "location_condition":
                    result_cond["type"] = "card_count_condition"
                    result_cond.pop("locations", None)
                    result_cond["source"] = "preceding_moved"
                result_cond.pop("location", None)
                rct = result_cond.get("text", "")
                if "ブレードハートを持たない" in rct or "ブレードハートがない" in rct:
                    result_cond["card_property"] = "has_blade_heart"
                if re.search(r"\d+枚以上", rct) or re.search(r"以上", rct):
                    if "operator" not in result_cond:
                        result_cond["operator"] = ">="
                    if "count" not in result_cond:
                        result_cond["count"] = 1
                if "source" not in result_cond:
                    result_cond["source"] = "preceding_moved"
                _cp = result_cond.get("card_property")
                followup_acts = []
                first_fa = dict(result_act)
                full_text_r = first_fa.get("text", "")
                rct2 = rct
                if rct2 and full_text_r.startswith(rct2):
                    action_text = full_text_r[len(rct2) :].lstrip("、").lstrip("。")
                    idx = t.find(rct2)
                    if idx >= 0 and t[idx + len(rct2) :].lstrip("、，").startswith(
                        "さらに"
                    ):
                        if not action_text.startswith("さらに"):
                            action_text = "さらに" + action_text
                    first_fa["text"] = action_text
                first_fa.pop("condition", None)
                followup_acts.append(first_fa)
                remaining = acts[result_idx + 1 :]
                _repeat_procedure = None
                if remaining:
                    for rem in remaining:
                        if isinstance(rem, dict) and rem.get("action") in (
                            "repeat_procedure",
                        ):
                            _repeat_procedure = dict(rem)
                        else:
                            followup_acts.append(dict(rem))
                if len(followup_acts) == 1:
                    followup = followup_acts[0]
                else:
                    combined_text = followup_acts[0].get("text", "")
                    for fa in followup_acts[1:]:
                        ft = fa.get("text", "")
                        if ft:
                            combined_text = (
                                (combined_text.rstrip("。").rstrip("、")) + "。" + ft
                            )
                    followup = {
                        "text": combined_text,
                        "action": "sequential",
                        "actions": followup_acts,
                    }
                    if _cp:
                        for _fa in followup_acts:
                            _fc = _fa.get("condition", {})
                            if isinstance(_fc, dict) and "card_property" not in _fc:
                                if (
                                    _fc.get("source") == "preceding_moved"
                                    or _fc.get("negation") is not None
                                ):
                                    _fc["card_property"] = _cp
                if eff.get("activation_position") and not followup.get(
                    "activation_position"
                ):
                    followup["activation_position"] = eff["activation_position"]
                if _repeat_procedure:
                    # Wrap COR + repeat_procedure in a sequential
                    _cor = {
                        "action": "conditional_on_result",
                        "primary_effect": primary,
                        "result_condition": result_cond,
                        "followup_action": followup,
                    }
                    # Merge any text from the original condition
                    _cor_txt = eff.get("text", "")
                    if _cor_txt:
                        _cor["text"] = _cor_txt
                    eff["action"] = "sequential"
                    eff["actions"] = [_cor, _repeat_procedure]
                    for k in ("primary_effect", "result_condition", "followup_action"):
                        eff.pop(k, None)
                else:
                    eff["action"] = "conditional_on_result"
                    eff["primary_effect"] = primary
                    eff["result_condition"] = result_cond
                    eff["followup_action"] = followup
                    eff.pop("actions", None)

        # E0: Fix DOLLCHESTRA-type primary_effect — split select+modify_cost into sequential
        if eff.get("action") in ("conditional_on_result", "conditional_alternative"):
            pe = eff.get("primary_effect")
            if (
                isinstance(pe, dict)
                and pe.get("action") == "select"
                and pe.get("original_value") is True
            ):
                pe_text = pe.get("text") or ""
                parts = pe_text.split("。")
                if len(parts) >= 2:
                    text_select = parts[0]
                    text_cost = "。".join(parts[1:]).lstrip("。")
                    pe["action"] = "sequential"
                    pe["actions"] = [
                        {
                            "text": text_select,
                            "source": pe.get("source"),
                            "count": pe.get("count", 1),
                            "card_type": pe.get("card_type"),
                            "target": pe.get("target"),
                            "group_names": pe.get("group_names"),
                            "action": "select",
                        },
                        {
                            "text": text_cost,
                            "duration": "live_end",
                            "card_type": pe.get("card_type"),
                            "action": "modify_cost",
                            "group_names": pe.get("group_names"),
                            "original_value": True,
                        },
                    ]
                    for k in ("source", "count", "duration", "card_type", "target"):
                        pe.pop(k, None)

        # E: Revert over-eager conditional_on_result to sequential (surplus_heart)
        # Keep the result_condition as a condition on the followup_action.
        if eff.get("action") == "conditional_on_result":
            pe = eff.get("primary_effect", {})
            if isinstance(pe, dict) and pe.get("resource") == "surplus_heart":
                actions = [pe]
                fa = eff.get("followup_action")
                rc = eff.get("result_condition")
                if isinstance(fa, dict):
                    if isinstance(rc, dict):
                        fa["condition"] = rc
                    actions.append(fa)
                if actions:
                    eff["action"] = "sequential"
                    eff["actions"] = actions
                    eff.pop("primary_effect", None)
                    eff.pop("result_condition", None)
                    eff.pop("followup_action", None)

        # ---- B1: Post-restructuring fix — self_cost on このメンバー change_state ----
        # FIX 9b in _apply_recursive_fixes runs before COR restructuring creates
        # followup_action, so we add self_cost here after the structure is final.
        def _fix_change_state_self_cost(cor_eff):
            fa = cor_eff.get("followup_action")
            if isinstance(fa, dict) and fa.get("action") == "change_state":
                if "このメンバー" in fa.get("text", ""):
                    if fa.get("self_cost") is None:
                        fa["self_cost"] = True

        if eff.get("action") == "conditional_on_result":
            _fix_change_state_self_cost(eff)
        elif eff.get("action") == "sequential":
            for act in eff.get("actions", []):
                if (
                    isinstance(act, dict)
                    and act.get("action") == "conditional_on_result"
                ):
                    _fix_change_state_self_cost(act)

        # ---- B: Scoped context propagation (inherits specific fields) ----
        _propagate_context(eff, t=t, eff_root=eff)

        # Q76 rule: self-revival from discard to stage can place on occupied areas
        if (
            eff.get("action") == "move_cards"
            and eff.get("source") == "discard"
            and eff.get("destination") == "stage"
            and eff.get("self_target") is True
        ):
            eff["allow_occupied_stage"] = True
        # Text-based detection for "既にメンバーがいるエリアにも登場できる"
        if eff.get("action") == "move_cards" and eff.get("destination") == "stage":
            text = eff.get("text", "")
            parenthetical = " ".join(eff.get("parenthetical", []) or [])
            if "既にメンバーがいるエリア" in text + parenthetical:
                eff["allow_occupied_stage"] = True

        # FIX: Parser misclassifies "ライブカードセットフェイズ..." as
        # set_card_identity because "セット" matches. Re-classify to
        # reduce_live_card_set_limit.
        if eff.get("action") == "sequential":
            for sub in eff.get("actions", []):
                if isinstance(sub, dict) and sub.get("action") == "set_card_identity":
                    act_text = sub.get("text", "")
                    if (
                        "ライブカードセットフェイズ" in act_text
                        and ("上限" in act_text or "枚数" in act_text)
                        and ("減る" in act_text or "減らす" in act_text)
                    ):
                        sub["action"] = "reduce_live_card_set_limit"
                        sub.pop("card_type", None)

    # ====================================================================
    # POST-HOC FIXES & MAIN
    # ====================================================================
    # _fix_mari_gain_ability — targeted fix for a known parser gap
    # _validate_semantic — validates parsed JSON against text patterns
    # Main entry point for standalone parser.py execution
    # ====================================================================

    # FIX: 小原鞠莉 (PL!S-bp2-008) ab#1 — constant conditional_alternative is
    # actually a gain_ability.  _try_sequential splices the text before
    # parse_action's gain_ability fallback runs, so we fix it post-hoc.
    _fix_mari_gain_ability(data, fix_stats)

    # FIX 17: Clean up redundant fields on perform_yell actions with per_unit_source
    # When per_unit_source is "previous_moved_cards", the engine sums costs from
    # self.moved_cards directly — per_unit_type and count are not used for yell.
    _clean_per_unit_source(data["unique_abilities"])

    # Validate parsed JSON against text patterns for missing mechanics
    _validate_semantic(data["unique_abilities"])


def process_abilities(data: Dict[str, Any]) -> Dict[str, Any]:
    """Post-process already-parsed abilities: infer actions, apply targeted fixes."""

    # Add a DO NOT EDIT warning
    data["_warning"] = (
        "DO NOT EDIT THIS MANUALLY. Run cards/ability_extraction/parser.py to regenerate."
    )

    fix_stats = {
        "movement": 0,
        "each_time": 0,
        "card_property": 0,
        "ability_filter": 0,
        "temporal": 0,
        "local_cond": 0,
        "group_cond": 0,
        "result_cond": 0,
        "primary_neg": 0,
        "leak": 0,
        "compound_split": 0,
        "auto_trigger": 0,
    }

    for ability in data["unique_abilities"]:
        _process_pre_fix(ability, fix_stats)
    _process_post_fixes(data, fix_stats)
    return data


def _fix_mari_gain_ability(data: Dict[str, Any], fix_stats: Dict[str, int]) -> None:
    """Post-hoc fix: PL!S-bp2-008 ab#1 effect is sequential/conditional_alternative
    but should be gain_ability.  Re-parses via parse_action + enriches gained_effect."""
    for ability in data.get("unique_abilities", []):
        cards = ability.get("cards", [])
        if not any("S-bp2-008" in c and "(ab#1)" in c for c in cards):
            continue
        eff = ability.get("effect")
        if not isinstance(eff, dict):
            continue
        if eff.get("action") not in ("sequential", "conditional_alternative"):
            continue
        tt = ability.get("triggerless_text", "")
        if "得る" not in tt:
            continue
        q = extract_quoted_text(tt)
        if not q:
            continue
        cat = categorize_quoted_text(q)
        if not cat["abilities"]:
            continue
        fixed = parse_action(tt)
        if fixed.get("action") != "gain_ability":
            continue
        # parse_action's gain_ability early return skips condition extraction.
        # Re-extract condition from the triggerless text (scan for 場合、 etc.)
        if not fixed.get("condition"):
            for sep in ["とき、", "場合、", "たび、", "なら、"]:
                idx = tt.find(sep)
                if idx >= 0:
                    ct = tt[: idx + 2]
                    tc = parse_condition(ct)
                    if tc and tc.get("type") not in (None, "custom"):
                        fixed["condition"] = tc
                        break
        ability["effect"] = fixed
        fix_stats["leak"] = fix_stats.get("leak", 0) + 1
        # Re-run enrichment: gained_effect from ability_gain text
        if "gained_effect" not in fixed and fixed.get("ability_gain"):
            clean_gain = re.sub(r"【[^】]+】", "", fixed["ability_gain"]).strip()
            gained = parse_effect(clean_gain)
            if gained and gained.get("action") and gained.get("action") != "custom":
                fixed["gained_effect"] = gained


def _clean(obj):
    """Recursively remove null/false/0/empty fields from dicts/lists."""
    if isinstance(obj, dict):
        return {
            k: _clean(v)
            for k, v in obj.items()
            if v is not None and v is not False and v != [] and v != {} and v != ""
        }
    if isinstance(obj, list):
        cleaned = [_clean(item) for item in obj]
        return [x for x in cleaned if x is not None and x != {}]
    return obj


def _propagate_optional(d):
    """Walk the effect tree and set optional=True where text has optional markers."""
    if isinstance(d, dict):
        if d.get("action") and "optional" not in d:
            t = d.get("text", "")
            if t and ("もよい" in t or "てもよい" in t):
                # Skip sub-actions that are gated by a condition (the condition
                # handles the optionality). Also skip sequential/choice containers
                # whose children already propagate optionality.
                if not d.get("condition") and d.get("action") not in (
                    "sequential",
                    "choice",
                    "conditional_on_result",
                    "conditional_on_optional",
                    "conditional_alternative",
                ):
                    d["optional"] = True
        for v in d.values():
            _propagate_optional(v)
    elif isinstance(d, list):
        for item in d:
            _propagate_optional(item)


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
                # Handle "エリアにいる場合のみ" patterns (e.g. "センターエリアにいる場合のみ発動する")
                # where parse_condition returns "custom" because the text has "場合" but
                # no matching handler. Build condition directly for these.
                if "エリアにいる場合" in note:
                    pos_map = {
                        "センター": "center",
                        "左サイド": "left_side",
                        "右サイド": "right_side",
                    }
                    detected_pos = [v for k, v in pos_map.items() if k in note]
                    cond_parsed = {
                        "type": "location_condition",
                        "location": "stage",
                        "position": detected_pos[0] if len(detected_pos) == 1 else None,
                        "text": note,
                    }
                else:
                    cond_parsed = parse_condition(note)
                if cond_parsed and cond_parsed.get("type") != "custom":
                    target["activation_condition_parsed"] = cond_parsed
            # Detect all mentioned positions
            has_center = "センターエリア" in note or "センター" in note
            has_left = "左サイドエリア" in note or "左サイド" in note
            has_right = "右サイドエリア" in note or "右サイド" in note
            positions = []
            if has_center:
                positions.append("center")
            if has_left:
                positions.append("left_side")
            if has_right:
                positions.append("right_side")
            if len(positions) == 1:
                target["activation_position"] = positions[0]
            elif len(positions) > 1:
                # Multiple positions (e.g. "左サイドエリアか右サイドエリア") → set as
                # comma-separated activation_position.  Also fix the condition_parsed:
                # when parse_condition created position + position_compare (treating it
                # as a cross-comparison), replace those with activation_position so the
                # engine checks "left OR right" rather than "compare left vs right".
                target["activation_position"] = ",".join(sorted(positions))
                if "activation_condition_parsed" in target:
                    acp = target["activation_condition_parsed"]
                    if acp.get("position") or acp.get("position_compare"):
                        del acp["position"]
                        del acp["position_compare"]
                        acp["activation_position"] = target["activation_position"]
                # Remove spurious "position" field when it matches activation_position.
                # The "position" field is a cross-comparison filter (expects single value
                # like "left_side"). When both positions are listed, it should be
                # activation_position instead. Keeping "position" as a comma-separated
                # string causes the engine's can_activate_effect to merge it into the
                # appearance condition, where it breaks the cross-comparison check.
                if target.get("position") == target.get("activation_position"):
                    del target["position"]
            break


# Required field validators per action type
_VALIDATORS = {
    "gain_resource": {"required": ["resource", "count"]},
    "move_cards": {"required": ["source", "destination"]},
    "draw_card": {"required": ["count"]},
    "modify_score": {"required": ["operation", "value"]},
    "modify_required_hearts": {"required": ["heart_colors", "count"]},
    "change_state": {"required": ["state_change"]},
}

# Known action names that the Rust engine and bytecode compiler accept.
# Anything not in this set will trigger a warning during extraction.
_KNOWN_ACTIONS = {
    "activate_ability",
    "change_state",
    "choose_target_player",
    "conditional_on_optional",
    "conditional_on_result",
    "discard_until_count",
    "do_nothing",
    "draw_card",
    "draw_until_count",
    "gain_ability",
    "gain_ability_from_source",
    "gain_resource",
    "invalidate_ability",
    "look_at",
    "modify_cost",
    "modify_required_hearts",
    "modify_required_hearts_global",
    "modify_score",
    "modify_yell_count",
    "move_cards",
    "pay_energy",
    "perform_yell",
    "place_energy_under_member",
    "play_baton_touch",
    "position_change",
    "re_yell",
    "reduce_live_card_set_limit",
    "repeat_procedure",
    "restriction",
    "reveal",
    "reveal_until_live_card",
    "select",
    "select_cards",
    "select_number",
    "set_blade_count",
    "set_blade_type",
    "set_card_identity",
    "set_heart_type",
    "specify_heart_color",
    "suppress_ability_trigger",
    "conditional_alternative",
    # Compound / structural types
    "sequential",
    "choice",
    "look_and_select",
    # Legacy / alias names accepted by the engine
    "draw",
    "look",
    "reveal_effect",
    "modify_required_hearts_success",
    "set_cost",
    "set_cost_to_use",
    "activation_cost",
    "activation_restriction",
    "all_blade_timing",
    "shuffle",
    "custom",
    # Special / internal
    "conditional_optional",
    "set_card_identity_all_regions",
}

# Known condition types that the Rust engine and bytecode compiler accept.
_KNOWN_CONDITIONS = {
    "card_count_condition",
    "location_condition",
    "comparison_condition",
    "group_condition",
    "movement_condition",
    "temporal_condition",
    "appearance_condition",
    "state_condition",
    "energy_state_condition",
    "position_condition",
    "or_condition",
    "and_condition",
    "end_condition",
    "highest_cost_on_stage_condition",
    "state_change_condition",
    "card_blade_condition",
    "all_cost_comparison_condition",
    "ability_filter_condition",
    "has_moved",
    "not_moved",
    "opponent_live_success",
    "no_excess_heart",
    # Compound / structural
    "compound",
    "or_condition",
    # Legacy / alias names
    "both_condition",
    "complex_condition",
    "otherwise_condition",
    "action_success_condition",
    "custom",
    "revealed_condition",
    "member_state",
    "zone_placement_condition",
    "position_change_condition",
    "opponent_choice_condition",
    "any_of_condition",
    "all_revealed_match_heart_color",
    "score_threshold_condition",
    "choice_condition",
}


def _validate_condition_types(cond, context=""):
    """Recursively validate condition types. Non-fatal warnings."""
    import logging

    _log = logging.getLogger("parser")
    if not isinstance(cond, dict):
        return
    ctype = cond.get("type", "")
    if ctype and ctype not in _KNOWN_CONDITIONS:
        _log.warning("Unknown condition type '%s' in %s", ctype, context or "(root)")
    # Recurse into compound conditions
    if ctype in ("compound", "or_condition"):
        for sub in cond.get("conditions", []):
            _validate_condition_types(sub, context)


def _validate_effect(eff, context=""):
    """Check required fields, known action names, and condition types. Non-fatal warnings."""
    import logging

    _log = logging.getLogger("parser")
    if not isinstance(eff, dict):
        return
    action = eff.get("action", eff.get("type", ""))
    if action and action not in _KNOWN_ACTIONS:
        _log.warning("Unknown action type '%s' in %s", action, context or "(root)")

    rules = _VALIDATORS.get(action)
    if rules:
        for field in rules["required"]:
            if field not in eff or eff[field] is None:
                _log.warning(
                    "Action '%s' missing required field '%s' in %s",
                    action,
                    field,
                    context or "(root)",
                )

    # Validate condition types
    cond = eff.get("condition")
    if isinstance(cond, dict):
        ctype = cond.get("type", "")
        if ctype and ctype not in _KNOWN_CONDITIONS:
            _log.warning(
                "Unknown condition type '%s' in %s (action=%s)",
                ctype,
                context or "(root)",
                action,
            )
        # Recurse into compound conditions
        if ctype in ("compound", "or_condition"):
            for sub_cond in cond.get("conditions", []):
                _validate_condition_types(sub_cond, context)
    # Also check alternative_condition, choice_condition
    for cond_key in ("alternative_condition", "choice_condition", "result_condition"):
        sub_cond = eff.get(cond_key)
        if isinstance(sub_cond, dict):
            _validate_condition_types(sub_cond, context)

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


def _enrich_effect_type(effect, triggerless=""):
    """Extract heart colors from ability text after parsing.
    This is called by the extraction pipeline (extract_card_abilities.py)
    to patch a parser gap: heart_colors not propagated to conditions."""
    if effect is None:
        return
    heart_colors = []
    seen = set()
    for m in re.findall(r"{{heart_(\d+)\.png\|heart\d+}}", triggerless):
        h = f"heart{m.zfill(2)}"
        if h not in seen:
            seen.add(h)
            heart_colors.append(h)
    if heart_colors and "heart_colors" not in effect:
        effect["heart_colors"] = heart_colors
    if "heart_colors" in effect and "condition" in effect:
        cond = effect["condition"]
        if (
            isinstance(cond, dict)
            and cond.get("type") == "location_condition"
            and "heart_colors" not in cond
            and not cond.get("check_self")
        ):
            cond["heart_colors"] = effect["heart_colors"]


def _json_has(obj, pred):
    """Recursively check if any nested dict/list satisfies pred."""
    if isinstance(obj, dict):
        if pred(obj):
            return True
        for v in obj.values():
            if _json_has(v, pred):
                return True
    elif isinstance(obj, list):
        for item in obj:
            if _json_has(item, pred):
                return True
    return False


def _json_has_action(obj, action_str):
    return _json_has(
        obj, lambda d: isinstance(d, dict) and d.get("action") == action_str
    )


def _json_has_field(obj, field, value=None):
    def check(d):
        if not isinstance(d, dict):
            return False
        if field not in d:
            return False
        if value is not None:
            return d[field] == value
        return True

    return _json_has(obj, check)


def _validate_semantic(abilities):
    """Validate parsed JSON against text patterns to find missing mechanics.

    Each rule: (name, regex, check_fn(entry, effect) -> bool, description)
    check_fn returns True if the mechanic IS correctly handled.
    """
    issues = []

    # Matched text inside 「」 quotes is probably an ability name, not effect context.
    def _in_quotes(text, m):
        start = text.rfind("「", 0, m.start())
        end = text.find("」", m.end())
        return start != -1 and end != -1 and m.start() > start and m.end() < end

    # Matched text inside parenthetical clauses ( ... ) is game-rule reminder text,
    # not actual card effect.  Check for both ASCII and full-width parens.
    def _in_parens(text, m):
        for o, c in [("(", ")"), ("（", "）")]:
            before = text[: m.start()]
            after = text[m.end() :]
            last_open = before.rfind(o)
            if last_open == -1:
                continue
            between = before[last_open + 1 :]
            if c in between:
                continue
            next_close = after.find(c)
            if next_close != -1:
                return True
        return False

    RULES = [
        # ─── Number selection ───
        (
            "select_number",
            r"(数を選ん|[数数字]を[選選え])",
            lambda e, eff: _json_has_action(eff, "select_number")
            or _json_has_field(eff, "action", "select"),
            "Text says 'choose a number' but no select_number action found",
        ),
        # ─── Opponent choice ───
        (
            "opponent_choice",
            r"相手[はが].*[選選え]ぶ",
            lambda e, eff: _json_has_field(eff, "action_by", "opponent"),
            "Opponent chooses but no action_by: opponent",
        ),
        # ─── Distinct card names ───
        (
            "distinct_name",
            r"(カード名の異なる|カード名が異なる|名前の異なる|名前が異なる|異なるカード名)",
            lambda e, eff: _json_has_field(eff, "distinct", "card_name"),
            "Distinct card names required but no distinct field",
        ),
        # ─── Reveal until condition ───
        (
            "reveal_until",
            r"(公開するまで|まで公開し続け|現れるまで)",
            lambda e, eff: _json_has_action(eff, "reveal_until_live_card")
            or _json_has_action(eff, "reveal_until_chosen_card"),
            "Reveal until condition but no reveal_until action",
        ),
        # ─── Repeat procedure ───
        (
            "repeat_procedure",
            r"(繰り返す|まで繰り返|もう一度行う|再度)",
            lambda e, eff: _json_has_action(eff, "repeat_procedure")
            or _json_has_field(eff, "repeat_limit"),
            "Repeat/loop described but no repeat_procedure or repeat_limit",
        ),
        # ─── Conditional alternative (if not, otherwise) ───
        (
            "conditional_alt",
            r"(なかった場合|なければ|ない場合|なけれ|以外の場合)",
            lambda e, eff: _json_has_action(eff, "conditional_alternative")
            or _json_has_action(eff, "conditional_on_result")
            or _json_has_action(eff, "conditional_on_optional")
            or _json_has_field(eff, "negation")
            or _json_has_action(eff, "choice")
            or _json_has_field(
                (e.get("effect") or {}).get("condition") or {}, "operator"
            )
            or _json_has_field(eff, "no_excess_heart")
            or _json_has_action(eff, "position_change")
            or _json_has_field(eff, "otherwise_condition"),
            "Fallback/alternative (if not) but no conditional_alternative",
            # Accept: conditional actions, negation on any node, choice actions,
            # or condition with comparison operator (少ない場合 etc.)
        ),
        # ─── Placement order ───
        (
            "placement_order",
            r"(好きな順番|任意の順番|好きな順序|任意の順|好きな順)",
            lambda e, eff: _json_has_field(eff, "placement_order")
            or _json_has_field(e.get("cost", {}), "placement_order"),
            "Any-order placement but no placement_order field",
        ),
        # ─── Discard until hand count ───
        (
            "discard_until",
            r"(枚になるまで.*捨て|枚になるまで.*トラッシュ|枚になるまで.*墓地|になるまで捨て|になるまでトラッシュ)",
            lambda e, eff: _json_has_action(eff, "discard_until_count"),
            "Discard until hand size but no discard_until_count",
        ),
        # ─── Pay energy as cost ───
        (
            "pay_energy_cost",
            r"(エネルギーを.*支払|エネルギー.*払う)",
            lambda e, eff: _json_has_action(eff, "pay_energy"),
            "Pay energy described but no pay_energy action",
        ),
        # ─── ALL blade / any-color handling ───
        (
            "all_blade",
            r"(ALLブレード|全てのブレード|任意の色のブレード|ALL blade)",
            lambda e, eff: _json_has_field(eff, "all_blade_timing")
            or _json_has_action(eff, "all_blade_timing")
            or _json_has_action(eff, "set_blade_type")
            or _json_has_field(eff, "card_property", "has_all_blade"),
            "ALL blade / any-color handling but no all_blade_timing",
        ),
        # ─── Invalidate / suppress ability ───
        (
            "invalidate_ability",
            r"(無効|発動しな|発動を防|無効にす|能力を.*失)",
            lambda e, eff: _json_has_action(eff, "invalidate_ability")
            or _json_has_action(eff, "suppress_ability_trigger"),
            "Ability nullification but no invalidate_ability or suppress_ability_trigger",
        ),
        # ─── Both players / both targets ───
        (
            "both_targets",
            r"(お互い|両プレイヤー|相手と自分(?!の)|自分と相手(?!の)|両方(?!(?:とも|ある)))",
            lambda e, eff: _json_has_field(eff, "target", "both")
            or _json_has_field(eff, "comparison_target"),
            "Both players affected but no target='both'",
        ),
        # ─── Additional Yell ───
        (
            "additional_yell",
            r"(追加で.*エール|エール.*追加|もう一度.*エール|さらに.*エール|追エール)",
            lambda e, eff: _json_has_action(eff, "perform_yell")
            or _json_has_action(eff, "re_yell")
            or _json_has_action(eff, "modify_yell_count"),
            "Additional Yell but no perform_yell or re_yell",
        ),
        # ─── Under member (place or reference) ───
        (
            "under_member",
            r"(?:この)?メンバーの下(?:に置|にあ|から|に置かれ)",
            lambda e, eff: _json_has_field(eff, "source", "under_member")
            or _json_has_field(eff, "destination", "under_member")
            or _json_has_action(eff, "place_energy_under_member")
            or _json_has_field(eff, "per_unit_source", "under_member")
            or _json_has_field(eff, "location", "under_member")
            or _json_has_field(eff, "source_location", "under_member")
            or _json_has_field(e.get("cost", {}), "destination", "under_member")
            or _json_has_field(e.get("cost", {}), "location", "under_member"),
            "Under-member operation but no under_member source/destination",
        ),
        # ─── Energy deck/zone operation ───
        (
            "energy_deck_to_zone",
            r"(エネルギー置き場|エネルギーデッキ)",
            lambda e, eff: _json_has_field(eff, "source", "energy_deck")
            or _json_has_field(eff, "destination", "energy_zone")
            or _json_has_field(eff, "destination", "energy_deck")
            or _json_has_action(eff, "place_energy_under_member")
            or _json_has_field(e.get("cost", {}), "type", "place_energy_under_member")
            or _json_has_field(e.get("cost", {}), "destination", "energy_deck")
            or _json_has_field(
                (e.get("effect") or {}).get("condition") or {},
                "location",
                "energy_zone",
            ),
            "Energy deck/zone operation but no energy_deck/energy_zone field",
        ),
        # ─── Blade type conversion ───
        (
            "blade_type",
            r"ブレード(?:として扱|とみな|treat)",
            lambda e, eff: _json_has_action(eff, "set_blade_type"),
            "Blade type conversion but no set_blade_type",
        ),
        # ─── Cost modification ───
        (
            "set_cost",
            r"(コストを.*変更|コストを.*増や|コストを.*減ら|コスト.*変え|支払うコスト)",
            lambda e, eff: _json_has_action(eff, "set_cost")
            or _json_has_action(eff, "modify_cost"),
            "Cost modification but no set_cost or modify_cost",
        ),
        # ─── Draw/discard until hand count ───
        (
            "draw_until_hand",
            r"(枚になるまで|になるまで)",
            lambda e, eff: _json_has_action(eff, "draw_until_count")
            or _json_has_action(eff, "discard_until_count"),
            "Until-hand-size action but no draw_until_count or discard_until_count",
        ),
        # ─── Heart type conversion ───
        (
            "heart_type",
            r"ハート(?:として扱|とみな|treat)",
            lambda e, eff: _json_has_action(eff, "set_heart_type")
            or _json_has_action(eff, "all_blade_timing"),
            "Heart type conversion but no set_heart_type",
        ),
        # ─── Card identity setting ───
        (
            "card_identity",
            r"(としても扱|として扱う|同一として扱|として見な)",
            lambda e, eff: _json_has_action(eff, "set_card_identity")
            or _json_has_action(eff, "all_blade_timing")
            or _json_has_action(eff, "set_heart_type"),
            "Card identity/card name treated as but no set_card_identity",
        ),
        # ─── Multiple targets ───
        (
            "multiple_targets",
            r"(?:枚|人|体)(?:まで|.{0,15}?まで)",
            lambda e, eff: _json_has_field(eff, "multiple_targets")
            or _json_has_field(eff, "max")
            or _json_has_field(eff, "count")
            or _json_has_field(e.get("condition", {}), "operator")
            or _json_has_field(e.get("cost", {}), "max")
            or _json_has_field(e.get("cost", {}), "count"),
            "Multiple target count but no multiple_targets or max/count field",
        ),
        # ─── Exclude self ───
        (
            "exclude_self",
            r"(自分以外|自身以外|このカード以外|このメンバー以外|自分を除く)",
            lambda e, eff: _json_has_field(eff, "exclude_self")
            or _json_has_field(e.get("cost", {}), "exclude_self"),
            "Exclude self described but no exclude_self field",
        ),
        # ─── Shuffle ───
        (
            "shuffle",
            r"(シャッフルする|シャッフルして)",
            lambda e, eff: _json_has_action(eff, "shuffle")
            or _json_has_field(eff, "shuffle")
            or _json_has_field(e.get("cost", {}), "shuffle"),
            "Shuffle described but no shuffle action or flag",
        ),
        # ─── Restriction ───
        (
            "restriction",
            r"(?<!支払)(?:ことが|を)?できない",
            lambda e, eff: _json_has_action(eff, "restriction")
            or _json_has_field(eff, "max_repeats"),
            "Restriction/cannot described but no restriction action or max_repeats",
        ),
        # ─── Look at cards ───
        (
            "look_at",
            r"(見てもよい|見ることができる|(?<!必要ハートを)確認する)",
            lambda e, eff: _json_has_action(eff, "look_at"),
            "Look at cards described but no look_at action",
        ),
        # ─── Non-stackable ───
        (
            "non_stackable",
            r"重複しない",
            lambda e, eff: _json_has_field(eff, "non_stackable"),
            "Non-stackable described but no non_stackable flag",
        ),
        # ─── Per-group ───
        (
            "per_group",
            r"各グループ",
            lambda e, eff: _json_has_field(eff, "per_group")
            or _json_has_field(eff, "per_group_count")
            or _json_has_field(eff, "per_unit_type"),
            "Per-group described but no per_group/per_group_count/per_unit_type field",
        ),
        # ─── Baton touch ───
        (
            "baton_touch",
            r"バトンタッチして登場",
            lambda e, eff: _json_has_field(eff, "baton_touch_trigger")
            or _json_has_field(eff, "baton_touch_source"),
            "Baton touch but no baton_touch_trigger or baton_touch_source",
        ),
        # ─── Lose resource (should have sign: negative) ───
        (
            "lose_resource",
            r"失う",
            lambda e, eff: _json_has_field(eff, "sign", "negative"),
            "Lose resource described but no sign: negative",
        ),
        # ─── Same name ───
        (
            "same_name",
            r"同じ名前",
            lambda e, eff: _json_has_field(eff, "same_name")
            or _json_has_field(eff.get("condition", {}), "same_name"),
            "Same name required but no same_name field",
        ),
        # ─── Card property (has_blade_heart / has_score_icon) ───
        (
            "card_property",
            r"(ブレードハートを持|ブレードハートがない|スコアを持つ)",
            lambda e, eff: _json_has_field(eff, "card_property")
            or _json_has_field(e.get("cost", {}), "card_property"),
            "Card property (blade heart / score icon) but no card_property field",
        ),
    ]

    seen_by_rule = {}  # rule_name -> set of frozenset(cards)
    all_issues = []  # (rule_name, cards, trigger, snippet, desc)

    for i, entry in enumerate(abilities):
        t = entry.get("triggerless_text", "") or entry.get("full_text", "")
        eff = entry.get("effect") or {}
        if not t or not isinstance(eff, dict):
            continue
        cards = entry.get("cards", [])
        card_lbl = cards[0] if cards else "???"
        trigger = entry.get("triggers", "")

        for rule_name, pattern, check_fn, desc in RULES:
            for m in re.finditer(pattern, t):
                # Skip matches inside quoted ability names (「」) — those are text
                # references, not actual mechanic descriptions.
                if _in_quotes(t, m):
                    continue
                # Skip matches inside parenthetical clauses ( ) / （） — those
                # are game-rule reminder text, not card effect descriptions.
                if _in_parens(t, m):
                    continue
                # Check if JSON correctly handles the mechanic
                if not check_fn(entry, eff):
                    matched = m.group(0)
                    start = max(0, m.start() - 20)
                    end = min(len(t), m.end() + 30)
                    snippet = t[start:end]
                    dedup_key = (rule_name, frozenset(cards) if cards else i)
                    if dedup_key not in seen_by_rule:
                        seen_by_rule[dedup_key] = True
                        all_issues.append((rule_name, cards, trigger, snippet, desc))

        # ─── Structural checks (not regex-based) ─────────────────────────
        # change_state: energy needs card_type=energy_card
        if (
            eff.get("action") == "change_state"
            and "エネルギー" in t
            and "メンバー" not in t
        ):
            if eff.get("card_type") != "energy_card":
                all_issues.append(
                    (
                        "energy_card_type",
                        cards,
                        trigger,
                        t[
                            max(0, t.find("エネルギー") - 10) : t.find("エネルギー")
                            + 30
                        ],
                        "energy change_state without card_type=energy_card",
                    ),
                )

        # move_cards: cost_limit in text but not in effect
        if eff.get("action") == "move_cards" and re.search(r"コスト\d+", t):
            has_cl = (
                eff.get("cost_limit") is not None
                or eff.get("cost_limit_min") is not None
                or eff.get("cost_limit_max") is not None
                or eff.get("cost_limit_operator") is not None
            )
            if not has_cl:
                cond = eff.get("condition") or {}
                has_cl = (
                    cond.get("cost_limit") is not None
                    or cond.get("cost_limit_min") is not None
                    or cond.get("cost_limit_max") is not None
                )
            if not has_cl:
                dedup_key = ("cost_limit", frozenset(cards) if cards else i)
                if dedup_key not in seen_by_rule:
                    seen_by_rule[dedup_key] = True
                    all_issues.append(
                        (
                            "cost_limit",
                            cards,
                            trigger,
                            t[max(0, t.find("コスト") - 10) : t.find("コスト") + 30],
                            "'コスト' referenced in text but no cost_limit in effect or condition",
                        ),
                    )

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
                            dedup_key = (
                                "reveal_heart_colors",
                                frozenset(cards) if cards else i,
                            )
                            if dedup_key not in seen_by_rule:
                                seen_by_rule[dedup_key] = True
                                all_issues.append(
                                    (
                                        "reveal_heart_colors",
                                        cards,
                                        trigger,
                                        t[:60],
                                        "heart_colors on select parent but not on reveal sub-action",
                                    ),
                                )

        # Structural validation: sequential action patterns
        if eff.get("action") == "sequential":
            acts = eff.get("actions", [])
            for j, sub in enumerate(acts):
                if not isinstance(sub, dict):
                    continue
                if sub.get("action") in ("specify_heart_color", "reveal"):
                    if sub.get("group_names"):
                        dedup_key = (
                            f"leaked_group_names[{j}]",
                            frozenset(cards) if cards else i,
                        )
                        if dedup_key not in seen_by_rule:
                            seen_by_rule[dedup_key] = True
                            all_issues.append(
                                (
                                    f"leaked_group_names",
                                    cards,
                                    trigger,
                                    f"[{j}] action={sub['action']} group_names={sub['group_names']}",
                                    f"sub-action '{sub['action']}' has leaked group_names={sub['group_names']}",
                                ),
                            )
                if sub.get("action") == "select_cards" and sub.get("discard_remaining"):
                    for k in range(j + 1, len(acts)):
                        if (
                            isinstance(acts[k], dict)
                            and acts[k].get("action") == "move_cards"
                        ):
                            dedup_key = (
                                f"discard_remaining_conflict[{j},{k}]",
                                frozenset(cards) if cards else i,
                            )
                            if dedup_key not in seen_by_rule:
                                seen_by_rule[dedup_key] = True
                                all_issues.append(
                                    (
                                        "discard_remaining_conflict",
                                        cards,
                                        trigger,
                                        f"[{j}] select_cards discard_remaining + [{k}] move_cards",
                                        f"select_cards with discard_remaining=True conflicts with explicit move_cards at [{k}]",
                                    ),
                                )
                            break
                if sub.get("condition") and j > 0:
                    prev = acts[j - 1]
                    if isinstance(prev, dict) and prev.get("condition"):
                        prev_text = prev["condition"].get("text")
                        cur_text = sub["condition"].get("text")
                        if prev_text and cur_text and prev_text == cur_text:
                            if not sub["condition"].get("cache"):
                                dedup_key = (
                                    f"shared_condition_no_cache[{j}]",
                                    frozenset(cards) if cards else i,
                                )
                                if dedup_key not in seen_by_rule:
                                    seen_by_rule[dedup_key] = True
                                    # shared_condition_no_cache is an optimization hint, not a correctness issue
                                    pass

    # ─── Print report ────────────────────────────────────────────────────
    # Group by rule
    by_rule = {}
    for r in all_issues:
        by_rule.setdefault(r[0], []).append(r)

    print("=" * 80)
    print("MISSING MECHANICS ANALYSIS")
    print("=" * 80)
    print(f"\nTotal abilities checked: {len(abilities)}")
    print(f"Total mismatches found: {len(all_issues)}")
    print(f"Unique rule types triggered: {len(by_rule)}")
    print()

    priority_order = [
        "select_number",
        "distinct_name",
        "opponent_choice",
        "reveal_until",
        "repeat_procedure",
        "discard_until",
        "draw_until_hand",
        "additional_yell",
        "under_member",
        "baton_touch",
        "non_stackable",
        "placement_order",
        "multiple_targets",
        "energy_deck_to_zone",
        "conditional_alt",
        "both_targets",
        "blade_type",
        "set_cost",
        "heart_type",
        "card_identity",
        "invalidate_ability",
        "exclude_self",
        "pay_energy_cost",
        "all_blade",
        "energy_card_type",
        "cost_limit",
        "shuffle",
        "restriction",
        "look_at",
        "per_group",
        "lose_resource",
        "same_name",
        "card_property",
        "reveal_heart_colors",
        "leaked_group_names",
        "discard_remaining_conflict",
    ]

    SEP = "-" * 60
    printed_rules = set()

    for rule_name in priority_order:
        if rule_name not in by_rule:
            continue
        printed_rules.add(rule_name)
        entries = by_rule[rule_name]
        desc = entries[0][4]
        print(f"\n{SEP}")
        print(f">> {rule_name}  ({len(entries)} abilities)")
        print(f"   {desc}")
        print(SEP)
        for r in entries[:10]:
            _, cards, trigger, snippet, _ = r
            card_str = cards[0] if cards else "(no card)"
            print(f"\n  CARD: {card_str}")
            if trigger:
                print(f"  TRIGGER: {trigger}")
            print(f"  TEXT: ...{snippet}...")

    for rule_name in sorted(by_rule.keys()):
        if rule_name in printed_rules:
            continue
        printed_rules.add(rule_name)
        entries = by_rule[rule_name]
        desc = entries[0][4]
        print(f"\n{SEP}")
        print(f">> {rule_name}  ({len(entries)} abilities)")
        print(f"   {desc}")
        print(SEP)
        for r in entries[:5]:
            _, cards, trigger, snippet, _ = r
            card_str = cards[0] if cards else "(no card)"
            print(f"\n  CARD: {card_str}")
            if trigger:
                print(f"  TRIGGER: {trigger}")
            print(f"  TEXT: ...{snippet}...")

    print(f"\n\n{'=' * 80}")
    print("END OF MISSING MECHANICS REPORT")
    print(f"{'=' * 80}")

    issues = all_issues
    print(f"  Validation complete: {len(issues)} issues found")


if __name__ == "__main__":
    from extract_card_abilities import main

    main()
