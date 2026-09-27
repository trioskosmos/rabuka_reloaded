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

=== HOW TO ADD A PHRASE ===
Every parse layer is a data table now. Adding a new phrase = adding one rule
row in the right table, then validating (see below).

  Action phrases  →  _ACTION_RULES section (module-level, above parse_action).
                      Add one ActionRule row with a stable name and explicit
                      priority. The first match wins.
  Cost phrases    →  _COST_HANDLERS section (module-level, above parse_cost).
                     Append a @_register_cost handler (text, cost) -> dict|None.
  Effect phrases  →  _EFFECT_RULES section (module-level, above _STRUCTURAL_EFFECT_RULES).
                     Append an EffectPattern(...) row; these are dispatched
                     before every legacy _try_* handler.
  Condition phrases → CONDITION_PATTERNS list (each (name, tier, handler)).

After adding a rule, regenerate + validate:
  cd cards/ability_extraction && python extract_card_abilities.py
  cd ../../engine && cargo test            # 1972 tests pin the output
  python cards/ability_extraction/tests/test_parse_action.py   # + the other test files

=== FILE STRUCTURE ===
  Imports / config / shared constants
  parse_complex_condition, _extract_basic_cost_fields, _try_duration_prefix
  parse_ability
  COST RULE REGISTRY (_COST_HANDLERS) + parse_cost
  parse_effect (dispatches _EFFECT_RULES then _STRUCTURAL_EFFECT_RULES)
  parse_condition (dispatches _condition_registry)
  ACTION RULE REGISTRY (_ACTION_RULES) + parse_action
  Condition handler functions (_try_* for conditions)
  CONDITION_PATTERNS + _condition_registry
  _extract_generic_fields, _infer_condition_type, _enrich_* helpers
  Action utility helpers, _fill_defaults
  Effect handler functions (_try_effect_*) + _EFFECT_RULES + _STRUCTURAL_EFFECT_RULES
  _walk, _normalize_parsed_effect, process_abilities (post-hoc fixes)
  _propagate_optional, _merge_parenthetical
  Validation helpers + semantic validation
  __main__ (--list-rules dumps every registered rule)

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

"""

import re
import copy
import sys
from typing import Callable, Dict, Any, NamedTuple, Optional, Tuple, List


from parser_utils import (
    extract_dynamic_count,
    COUNT_PATTERN,
    normalize_fullwidth_digits,
    as_int,
    strip_suffix_period,
    extract_by_pattern,
    extract_cost_values,
    extract_all_quoted_names,
    extract_all_groups,
    extract_field_context,
    detect_position_matches,
    position_fields_from_matches,
    FieldContext,
    ACTION_CARD_FIELDS_POLICY,
    ACTION_DESTINATION_POLICY,
    ACTION_POSITION_POLICY,
    ACTION_SOURCE_POLICY,
    HEART_ICON,
    HEART_ICON_ID,
    HEART_ICON_PAIR,
    HEART_REF,
    HEART_HAS_REF,
    HEART_LABEL,
    ALL_HEART_ICON,
    BLADE_ICON,
    BLADE_ICON_HELD_RE,
    BLADE_ICON_RE,
    ENERGY_ICON,
    HAS_SCORE_ICON,
    count_icons,
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
    detect_card_property,
    LOCATION_PATTERNS,
    POSITION_KEYWORDS,
    _ALL_KW_RE,
    iter_dict_nodes,
    transform_child_lists,
    walk_dict_tree,
    PriorityRegistry,
    ActionRule,
    ConditionPattern,
    EffectPattern,
)

from cost_parser import (
    _COST_HANDLERS,
    _COST_FLAG_RULES,
    _COST_BATON_TOUCH_PATTERNS,
    _register_cost,
    _cost_verb_choice,
    _cost_energy,
    _cost_sequential,
    _cost_reveal,
    _cost_choice_comma,
    _classify_cost,
    _extract_basic_cost_fields,
    _fill_cost_source,
    _fill_cost_destination,
    _infer_destination_from_source,
    _mark_discard_all_hand,
    _mark_self_cost,
    parse_cost,
)
from parser_fields import (
    apply_group_exclusions,
    detect_note_positions,
    detect_positions,
    detect_icon_positions,
    set_cross_position_fields,
    extract_state_change,
    extract_optional,
    extract_max,
    _quoted_names,
    _has_shuffle,
    detect_exclude_self,
)

# ============== CONFIGURATION CONSTANTS ==============
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


# ============== POSITION RESOLUTION (single owner) ==============
# Position/area logic lives in parser_fields (single owner for the keyword map,
# icon map, and every detector).

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
# 「…（ライブ開始時/ライブ成功時）能力が解決したとき/解決するたび」 — resolution
# watchers (Dancing stars on me! PL!-bp6-020-L, Victory Road PL!N-bp5-030-L).
# Same each_time semantics as 〜たび: the trigger is an ability RESOLUTION
# event, not a board state query. Marker is stem-agnostic (解決) so both
# したとき (stem+た) and するたび (dictionary+たび) match.
ABILITY_RESOLVE_MARKER = "能力が解決"
ALTERNATIVE_MARKER = "代わりに"

# ============== DURATION PREFIXES ==============
DURATION_PREFIX_MAP = {
    "ライブ終了時まで": "live_end",
    "ライブ終了まで": "live_end",
    "このターンの間": "this_turn",
    "このライブの間": "live_end",
    "ターン終了時まで": "this_turn",
    "そのターンの間": "this_turn",
}

# ======================================================================
# HEART ICON VOCABULARY + HELPERS
# ======================================================================
# The patterns themselves live in parser_utils (see HEART_ICON there) because
# the shared helpers need them too. These are the parser-side readers.
def _heart_icons(text):
    """Every rendered heart icon in `text`, in order of appearance."""
    return re.findall(HEART_ICON, text)


def _heart_count(text):
    """How many heart icons `text` renders."""
    return len(_heart_icons(text))


def _heart_id_list(text):
    """Every heart colour named by a rendered icon, in order, duplicates kept."""
    return [f"heart{n.zfill(2)}" for n in re.findall(HEART_ICON_ID, text)]


def _heart_ids(text):
    """The distinct heart colours `text` names, sorted."""
    return sorted(set(_heart_id_list(text)))


def _heart_ids_in_order(text):
    """The distinct heart colours `text` names, in first-appearance order.

    Conditions want this (the order reads as the order the clause lists them);
    effect patches want it too, because the engine treats the list positionally.
    """
    return list(dict.fromkeys(_heart_id_list(text)))


def _heart_ids_gained(text):
    """The heart colours `text` grants, in order, duplicates kept.

    「{{heart_03.png|heart03}}を得る」 grants a heart, so the icon is one the
    player receives; 「{{heart_03.png|heart03}}を持つ」 merely requires one and
    is a condition, not a count. Only the former is a gain.
    """
    return [
        f"heart{m.group(1).zfill(2)}"
        for m in re.finditer(HEART_ICON_ID, text)
        if "持つ" not in text[m.end() : m.end() + 12]
    ]


def _heart_label_counts(text):
    """How many times each heart colour label appears in `text`, in order."""
    counts = {}
    for label in re.findall(HEART_LABEL, text):
        counts[label] = counts.get(label, 0) + 1
    return counts


def _heart_ref_ids(text, unique=False):
    """Heart colours from the `heart_NN` ids written in `text`, as `heartNN`."""
    ids = [f"heart{n.zfill(2)}" for n in re.findall(HEART_REF, text)]
    return list(dict.fromkeys(ids)) if unique else ids


def _count_heart_refs(text, allowed=None):
    """Per-colour counts of the `heart_NN` ids in `text`, optionally filtered."""
    counts = {}
    for heart_id in _heart_ref_ids(text):
        if allowed is None or heart_id in allowed:
            counts[heart_id] = counts.get(heart_id, 0) + 1
    return counts


def _uniform_count(counts):
    """The single value every colour agrees on, or the smallest if they differ.

    Required-heart clauses are written with the same number per colour, so the
    usual answer is "they all agree"; `min` keeps a malformed clause from
    over-granting when they do not.
    """
    if not counts:
        return None
    values = list(counts.values())
    return values[0] if len(set(values)) == 1 else min(values)

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
# Single source for every cost-modification pattern. The two threshold entries
# also populate cost_threshold/threshold_operator (see extract_cost_modification);
# they are NOT re-coded as a second hardcoded regex there.
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

# mod_type -> threshold operator produced when that pattern matches.
COST_THRESHOLD_TYPES = {"cost_threshold": ">=", "cost_threshold_below": "<="}

# ============== COMPLEX CONDITION PATTERNS ==============
COMPLEX_CONDITION_MARKERS = ["これにより", "その結果"]

# ============== REGEX PATTERNS ==============
# Count/name/deck-position regexes are owned by parser_utils (COUNT_PATTERN,
# QUOTED_NAME_PATTERN, GROUP_PATTERN, ...); no second copy lives here.

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




def extract_cost_range(text: str) -> Optional[Dict[str, int]]:
    """Extract cost range pattern like 'コスト4以上9以下' (cost >=4 AND <=9).
    Returns {"min": min_val, "max": max_val} or None."""
    m = re.search(r"コスト(\d+)以上(\d+)以下", text)
    if m:
        return {"min": as_int(m.group(1)), "max": as_int(m.group(2))}
    return None


# Single source for blade-count-limit matching. Each entry is (regex, operator_kind)
# where operator_kind is "captured" (the operator word is group 2) or "==". Tried in
# order; the regexes are not re-coded anywhere else.
_BLADE_LIMIT_PATTERNS = [
    (r"ブレードが(\d+)[つ個](以下|以上|未満|超)のメンバー", "captured"),
    (r"ブレード[の]数[がは](\d+)[つ個](以下|以上|未満|超)", "captured"),
    (r"ブレード[の]数[がは](\d+)(以下|以上|未満|超)", "captured"),
    (r"ブレード[の]数[がは]ちょうど(\d+)[つ個]", "=="),
    (r"ブレード[の]数[がは](\d+)[つ個](?!になる)", "=="),
    # "ブレードを4つ以上持つ" / "ブレードを4つ持つ" — the icon form of a member
    # blade-count filter (e.g. Fire Bird "ブレードを4つ以上持つ『虹ヶ咲』のメンバー").
    (r"ブレード[をが](\d+)[つ個](以上|以下|未満|超)持つ", "captured"),
    (r"ブレード[をが]ちょうど(\d+)[つ個]持つ", "=="),
    (r"ブレード[をが](\d+)[つ個]持つ", "=="),
]

_BLADE_LIMIT_OPERATORS = {"以下": "<=", "以上": ">=", "未満": "<", "超": ">"}


def extract_blade_limit(text: str) -> Optional[Dict[str, Any]]:
    """Extract blade count limit from text like 'ブレードの数が3つ以下' (<=3 blades)."""
    normalized = re.sub(BLADE_ICON_RE, "ブレード", text)
    for pattern, op_kind in _BLADE_LIMIT_PATTERNS:
        m = re.search(pattern, normalized)
        if not m:
            continue
        result: Dict[str, Any] = {"blade_limit": as_int(m.group(1))}
        if op_kind == "captured":
            result["blade_limit_operator"] = _BLADE_LIMIT_OPERATORS[m.group(2)]
        else:
            result["blade_limit_operator"] = "=="
        return result
    return None


_DECK_POSITION_PATTERNS = [
    r"一番上から(\d+)[枚番]目",
    r"上から(\d+)[枚番]目",
]


def extract_deck_position(text: str) -> Optional[int]:
    """Extract deck position from text like '一番上から4枚目' / 'デッキの上から4番目'."""
    # Tried in order; single source for the deck-position regexes.
    for pattern in _DECK_POSITION_PATTERNS:
        match = re.search(pattern, text)
        if match:
            return as_int(match.group(1))
    return None


def extract_deck_position_for_action(text: str) -> Optional[Dict[str, Any]]:
    """Extract deck position for action, returns PositionInfo format."""
    pos = extract_deck_position(text)
    if pos:
        return {"position": {"position": str(pos)}}
    return None


def extract_deck_position_constraint(text: str) -> Optional[Dict[str, Any]]:
    """Extract deck-position requirement (e.g. Q226: 一番上から4枚目) with target."""
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


def extract_heart_types(text: str) -> List[str]:
    """The heart colours `text` names, as `heartNN` ids in first-appearance order."""
    return _heart_ids_in_order(text)



# Single source for full-width （） and ASCII () parentheses.
_PAREN_PATTERNS = [r"（([^）]+)）", r"\(([^)]+)\)"]


def extract_parenthetical(text: str) -> List[str]:
    """Extract all text within （） or () parentheses."""
    results: List[str] = []
    for pattern in _PAREN_PATTERNS:
        results += re.findall(pattern, text)
    return results


def strip_parenthetical(text: str) -> str:
    """Remove parenthetical notes from text (both full-width （） and ASCII ())."""
    for pattern in _PAREN_PATTERNS:
        text = re.sub(pattern, "", text)
    return text.strip()


def categorize_quoted_text(quoted_text: List[str]) -> Dict[str, List[str]]:
    """Categorize quoted text into character names and ability texts."""
    result = {"characters": [], "abilities": []}
    for q in quoted_text:
        if "{{" in q and "}}" in q:
            result["abilities"].append(q)
        else:
            result["characters"].append(q)
    return result


def deduped_groups(text):
    """Group names from 『』, deduplicated preserving first-seen order."""
    return list(dict.fromkeys(extract_all_groups(text)))


def _quoted_group_names(text: str) -> List[str]:
    """All 『...』 group names. Single owner for the group-name regex."""
    return re.findall(r"『([^』]+)』", text)


def normalize(text: str) -> str:
    """Canonicalize variant patterns before parsing."""
    text = re.sub(r"'([^']{1,10})'", r"『\1』", text)
    text = text.replace("ライブ終了まで", "ライブ終了時まで")
    text = normalize_fullwidth_digits(text)
    text = re.sub(r"\s+", " ", text).strip()
    return text


def normalize_multiline(text: str) -> str:
    """Like normalize(), but preserves line structure.

    Choice bullets are delimited by newlines (「\\n・」); collapsing all
    whitespace would break the choice parser's bullet splitting.
    """
    text = re.sub(r"'([^']{1,10})'", r"『\1』", text)
    text = text.replace("ライブ終了まで", "ライブ終了時まで")
    text = normalize_fullwidth_digits(text)
    lines = [re.sub(r"[ \t\u3000]+", " ", ln).strip() for ln in text.splitlines()]
    return "\n".join(ln for ln in lines if ln).strip()


def extract_name_exclusions(text):
    """Extract name inclusion/exclusion from 「X」以外 patterns.
    Returns (include_names, exclude_names) as lists.
    """
    includes = _quoted_names(text)
    excludes = re.findall(r"「([^」]+)」以外", text)
    # Remove excluded from included
    includes = [n for n in includes if n not in excludes]
    return includes, excludes


_ICON_SUB_RE = re.compile(r"\{\{([^|]+)\|([^}]+)\}\}")


def _strip_icon_annotations(text):
    """Turn {{icon|label}} markers into 【label】 and drop「」brackets."""
    return _ICON_SUB_RE.sub(r"【\2】", text).replace("「", "").replace("」", "").strip()


def extract_cost_modification(text: str) -> Optional[Dict[str, Any]]:
    """Extract cost modification patterns from text."""
    result = {}

    # Single pass over COST_MODIFICATION_PATTERNS (the only copy of these regexes).
    # modification_type/value come from the FIRST match; the threshold entries also
    # populate cost_threshold/threshold_operator regardless of which pattern matched
    # first, so a text containing both e.g. "コストは2減る" and "コストが3以上になった場合"
    # keeps both fields (as the previous independent inline check did).
    for pattern, mod_type in COST_MODIFICATION_PATTERNS:
        match = re.search(pattern, text)
        if not match:
            continue
        if "modification_type" not in result:
            result["modification_type"] = mod_type
            if match.groups():
                result["value"] = as_int(match.group(1))
        if mod_type in COST_THRESHOLD_TYPES:
            result["cost_threshold"] = as_int(match.group(1))
            result["threshold_operator"] = COST_THRESHOLD_TYPES[mod_type]

    return result if result else None


def extract_heart_colors_from_text(text: str) -> list:
    """Extract heart color list from {{heart_XX.png|heartXX}} icon patterns."""
    return _heart_ids(text)


def detect_duration_code(text: str) -> Optional[str]:
    """Detect duration patterns and return the duration code, or None."""
    for pat, code in DURATION_PREFIX_MAP.items():
        if pat in text:
            return code
    return None


# ====================================================================
# STRUCTURAL PARSING
# ====================================================================
# split_cost_effect, split_condition_action, and related helpers that
# break raw ability text into structural segments before parsing.
# ====================================================================


def split_cost_effect(text: str) -> Tuple[str, str]:
    """Split text into cost and effect parts, skipping colons inside brackets/templates."""
    if "：" not in text:
        return "", text
    paren_depth = 0  # （） and ()
    bracket_depth = 0  # 「」『』
    template_depth = 0  # {{ }}
    split_index = -1
    i = 0
    while i < len(text):
        if text[i : i + 2] == "{{":
            template_depth += 1
            i += 2
            continue
        if text[i : i + 2] == "}}":
            template_depth = max(0, template_depth - 1)
            i += 2
            continue
        c = text[i]
        if c in ("（", "("):
            paren_depth += 1
        elif c in ("）", ")"):
            paren_depth = max(0, paren_depth - 1)
        elif c in ("「", "『"):
            bracket_depth += 1
        elif c in ("」", "』"):
            bracket_depth = max(0, bracket_depth - 1)
        elif c == "：":
            if paren_depth == 0 and bracket_depth == 0 and template_depth == 0:
                split_index = i
                break
        i += 1
    if split_index >= 0:
        cost = text[:split_index].strip()
        effect = text[split_index + 1 :].strip()
        return cost, effect
    else:
        return "", text


def split_condition_action(text: str) -> Tuple[str, str]:
    """Split text into condition and action parts.
    The condition keyword (場合/とき/なら) is kept with the condition text.
    Markers are located with nesting-aware depth-0 search so a 場合/とき
    inside 「」 quotes or （） notes never acts as the split point."""
    for keyword in ["場合", "とき", "なら"]:
        pattern = keyword + "、"
        keyword_idx = _find_depth0(text, pattern)
        if keyword_idx >= 0:
            comma_idx = keyword_idx + len(keyword)
            condition = text[:comma_idx].strip()
            action = text[comma_idx + 1 :].strip()
            return condition, action
    return "", text




def _ir_depth_scan(text: str):
    """Yield (index, char, depth_after) for each position, tracking
    bracket/template nesting. Depth 0 = top-level structure."""
    depth = 0
    i = 0
    n = len(text)
    while i < n:
        if text.startswith("{{", i):
            depth += 1
            i += 2
            continue
        if text.startswith("}}", i):
            depth = max(0, depth - 1)
            i += 2
            continue
        ch = text[i]
        if ch in "「『（(":
            depth += 1
        elif ch in "」』）)":
            depth = max(0, depth - 1)
        yield i, ch, depth
        i += 1


def _split_sentences_nesting(text: str) -> List[str]:
    """Split on 。 at nesting depth 0. Drops the 。 and empty segments."""
    sentences = []
    start = 0
    for i, ch, depth in _ir_depth_scan(text):
        if ch == "。" and depth == 0:
            seg = text[start:i].strip()
            if seg:
                sentences.append(seg)
            start = i + 1
    rest = text[start:].strip()
    if rest:
        sentences.append(rest)
    return sentences


def _find_depth0(text: str, marker: str) -> int:
    """First index where `marker` occurs at nesting depth 0, or -1."""
    n = len(text)
    for i, ch, depth in _ir_depth_scan(text):
        if depth == 0 and text.startswith(marker, i):
            return i
    return -1


def _split_marker_depth0(text: str, marker: str):
    """Split on the first top-level `marker` occurrence. Returns
    (before, after) or None when the marker only occurs inside brackets."""
    idx = _find_depth0(text, marker)
    if idx < 0:
        return None
    return text[:idx], text[idx + len(marker) :]




# Which card type a 「～がない」 clause is talking about, most specific first.
_NEGATION_CARD_TYPES = (("ライブカード", "live_card"), ("メンバーカード", "member_card"))


def _refine_custom_effect(effect_condition, effect_text):
    """Read a specific fact out of a これにより clause the parser could not shape.

    A cause-effect tail that only parses as `custom` still names something
    concrete — an ability it cancelled, a card type it denies. Naming it beats
    passing `custom` to the engine.
    """
    if "無効にした" in effect_text or "無効に" in effect_text:
        effect_condition["action"] = "invalidate_ability"
        effect_condition["optional"] = (
            "もよい" in effect_text or "してもよい" in effect_text
        )
        return
    if "ない" not in effect_text or "カード" not in effect_text:
        return
    effect_condition["negation"] = True
    for phrase, card_type in _NEGATION_CARD_TYPES:
        if phrase in effect_text:
            effect_condition["card_type"] = card_type
            break
    if "公開された" in effect_text:
        effect_condition["location"] = "revealed_cards"


def parse_complex_condition(text: str) -> Optional[Dict[str, Any]]:
    """Parse complex conditions with cause-effect relationships (e.g., これにより)."""
    # "かつこれにより" is an AND compound, not a complex cause-effect
    if "かつこれにより" in text:
        return None
    for marker in COMPLEX_CONDITION_MARKERS:
        if marker not in text:
            continue
        cause_text, effect_text = (part.strip() for part in text.split(marker, 1))
        # The marker needs real content before it, and must not be the opening
        # of a conditional phrase like "これにより～場合".
        if not cause_text or effect_text.startswith("場合"):
            continue
        effect_condition = parse_condition(effect_text)
        if effect_condition.get("type") == "custom":
            _refine_custom_effect(effect_condition, effect_text)
        return {
            "type": "complex_condition",
            "cause": parse_condition(cause_text),
            "effect": effect_condition,
            "text": text,
        }
    return None


# Independent cost-flag rules. Each entry is (test, field, value) where `test` is
# either a substring to look for in the text or a callable returning truthy when
# the flag applies. These do not depend on other cost fields, so they are applied
# in one pass instead of being re-coded inline below.
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


# A segment that is nothing but a duration phrase (「ライブ終了時まで、」) is not
# an effect of its own — it states the duration of the one that follows it.
#
# This is DURATION_PREFIX_MAP without the two turn-scoped 「〜終了時まで」 phrases,
# which are never written as a standalone segment. Listed separately because
# the two sets are genuinely different; the assert keeps them from drifting.
_STANDALONE_DURATION_SEGMENTS = (
    "ライブ終了時まで",
    "ライブ終了まで",
    "このターンの間",
    "このライブの間",
)
assert set(_STANDALONE_DURATION_SEGMENTS) <= set(DURATION_PREFIX_MAP)
_DURATION_ONLY_SEGMENT_RE = re.compile(
    r"^(" + "|".join(re.escape(p) for p in _STANDALONE_DURATION_SEGMENTS) + r")[、，]?$"
)


def _fold_standalone_duration(parts):
    """Prepend each stranded duration segment to the part that follows it."""
    folded = []
    stash = None
    for part in parts:
        m = _DURATION_ONLY_SEGMENT_RE.match(part)
        if m:
            stash = m.group(1)
            continue
        if stash:
            part = stash + part
            stash = None
        folded.append(part)
    return folded


def _is_real_condition(cond):
    """Did a condition actually parse, or did it fall through to `custom`?

    `custom` is the parser's "I did not understand this" result, so every caller
    that wants a usable condition tests against it. One place says so, instead
    of each call site re-deriving it.
    """
    return bool(cond) and cond.get("type") != "custom"


def _is_real_action(action):
    """Did an action actually parse, or did it fall through to `custom`?"""
    return bool(action) and action.get("action") != "custom"


def _parsed_to_something(action):
    """As _is_real_action, but a node with no `action` key at all counts as
    `custom` rather than as a real action.

    The two spellings disagree only for that missing key; the surrounding code
    picked one of them, so keep whichever it assumed.
    """
    return action.get("action", "custom") != "custom"


def _phase_target_of(text):
    """Whose phase `text` names: "self" for 自分の, "opponent" for 相手の.

    自分の outranks 相手の — a clause naming both is still about the player's
    own phase.
    """
    if "自分の" in text:
        return "self"
    if "相手の" in text:
        return "opponent"
    return None


def _apply_phase_restriction(text, code, *targets):
    """Record a 「…メインフェイズに」 restriction on every dict in `targets`.

    `code` is the phase name that layer expects — the temporal conditions use
    "main_phase" while the card-count router uses "main". `phase_target` is
    filled in only when the clause says whose phase it is; an unqualified
    「メインフェイズに」 restricts to neither player. Returns True when the text
    names a main phase at all, so a caller can use it as its own condition.
    """
    if "メインフェイズ" not in text:
        return False
    owner = _phase_target_of(text)
    for target in targets:
        target["phase"] = code
        if owner:
            target["phase_target"] = owner
    return True


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
    if gate_phrase.startswith(("自分の", "相手の")):
        owner = _phase_target_of(gate_phrase)
        gate["phase_target"] = owner
        gate["trigger_event"]["phase_target"] = owner

    # Turn number (e.g. このゲームの1ターン目のライブフェイズの場合)
    turn_m = re.search(r"(\d+)ターン目", gate_phrase)
    if turn_m:
        gate["turn_number"] = as_int(turn_m.group(1))
        gate["trigger_event"]["turn_number"] = as_int(turn_m.group(1))

    return gate, remaining


def _attach_baton_touch_from_group_condition(effect, triggerless_text):
    """Attach the baton-touch-source-group gate condition for "『X』のメンバーから
    バトンタッチして登場した場合", and strip the leaked baton_touch_trigger/group_names
    off the effect and every action step (they are a gate, not a target filter)."""
    gm = re.search(r"『([^』]+)』のメンバーからバトンタッチして登場した場合", triggerless_text)
    if not gm:
        return
    groups = [gm.group(1)]
    cond = {
        "type": "movement_condition",
        "movement": "baton_touch",
        "target": "self",
        "baton_touch_trigger": True,
        "group_names": groups,
        "text": gm.group(0),
        "trigger_event": {"type": "baton_touch", "tense": "past", "location": "stage"},
    }
    if not isinstance(effect, dict):
        return
    if not effect.get("condition"):
        effect["condition"] = cond

    def _strip(node):
        if isinstance(node, dict):
            if "action" in node:
                node.pop("baton_touch_trigger", None)
                # Only the gate group itself is a condition leak; other 『Y』
                # groups in an action's own clause are legitimate target
                # filters (e.g. 控え室から『蓮ノ空』のライブカードを加える).
                gn = node.get("group_names")
                if isinstance(gn, list):
                    remaining = [g for g in gn if g not in groups]
                    if remaining:
                        node["group_names"] = remaining
                    else:
                        node.pop("group_names", None)
            for v in node.values():
                _strip(v)
        elif isinstance(node, list):
            for item in node:
                _strip(item)

    _strip(effect)


def _find_modify_cost_nodes(obj):
    """Collect modify_cost sub-action nodes (self-cost reduction clauses)."""
    return [
        node
        for node in iter_dict_nodes(obj)
        if node.get("action") == "modify_cost"
    ]


def _find_pay_energy_steps(obj):
    """Collect pay_energy cost steps from a cost tree."""
    return [
        node
        for node in iter_dict_nodes(obj, ("costs", "options", "actions"))
        if node.get("type") == "pay_energy" or node.get("action") == "pay_energy"
    ]


def _promote_self_cost_reduction(ability: Dict[str, Any]) -> None:
    """Promote a self-cost energy reduction clause onto the ability's own
    pay_energy cost as `cost_reduction_per_group`.

    e.g. 海未 bp5-004: "この能力を起動するためのコストは自分のステージにいる
    メンバーの中のグループ名1種類につき、{{E}}減る。" — the clause parses as
    a modify_cost effect step with per_unit metadata; affordability decisions
    need it attached to the cost itself so the printed energy is treated as an
    upper bound. Only group-name-per-unit ENERGY reductions qualify; other
    cards' continuous deploy-cost modifiers and non-energy reductions
    (e.g. pb1-007's hand-count reduction) are untouched.
    """
    cost = ability.get("cost")
    effect = ability.get("effect")
    if not isinstance(cost, dict) or not isinstance(effect, dict):
        return
    for node in _find_modify_cost_nodes(effect):
        if node.get("operation") != "subtract":
            continue
        if node.get("per_unit_type") != "group_name":
            continue
        amount = node.get("per_unit_count") or node.get("count")
        if not isinstance(amount, int) or amount <= 0:
            continue
        for pay in _find_pay_energy_steps(cost):
            pay.setdefault("cost_reduction_per_group", amount)


# The gate markers that can introduce an ability-level trigger condition,
# in the order they are tried.
_TRIGGER_GATE_SEPARATORS = ("とき、", "場合、", "たび、", "なら、")

# A gate marker appearing after one of these belongs to a later clause, not
# to the whole ability.
_CLAUSE_BREAKS = ("。", "・", "\n")

# The effect keys that may carry a condition on a sub-action.
_SUB_ACTION_KEYS = ("actions", "primary_effect", "conditional_action")


def _leading_gate_condition(effect_text):
    """The ability-level trigger condition in `effect_text`, or None.

    Gate markers are located with a nesting-aware depth-0 search, so a 場合/とき
    inside 「」 quotes or （） notes never creates a bogus gate. Only a LEADING
    gate qualifies: the marker must sit in the first sentence, before any
    clause break, or it belongs to a sub-action or a later bullet.
    """
    txt = effect_text
    if SEQUENTIAL_MARKER in txt:
        txt = txt.split(SEQUENTIAL_MARKER)[-1].lstrip("、").strip()
    for sep in _TRIGGER_GATE_SEPARATORS:
        idx = _find_depth0(txt, sep)
        if idx < 0:
            continue
        if any(brk in txt[:idx] for brk in _CLAUSE_BREAKS):
            continue
        cond = parse_condition(txt[: idx + len(sep)])
        if cond and cond.get("type") not in (None, "custom"):
            return cond
    return None


def _sub_action_condition(node):
    """The first condition carried by a sub-action of `node`, or None.

    The engine reads an ability's gate from `effect["condition"]`. When the
    effect handler attached the condition to a child instead, lifting the first
    one found gives the ability a gate without inventing a new one.
    """
    for key in _SUB_ACTION_KEYS:
        sub = node.get(key)
        if isinstance(sub, dict):
            cond = sub.get("condition")
            if isinstance(cond, dict) and cond:
                return cond
        elif isinstance(sub, list):
            for item in sub:
                cond = item.get("condition") if isinstance(item, dict) else None
                if isinstance(cond, dict) and cond:
                    return cond
    return None


def _demote_partial_group_names(effect):
    """Drop a sequential's own group filter when only some children share it.

    A parent `group_names` that some child contradicts is worse than none: the
    engine would then apply a filter the children do not agree with.
    """
    if not isinstance(effect, dict) or effect.get("action") != "sequential":
        return
    actions = effect.get("actions")
    if not isinstance(actions, list):
        return
    per_child = [
        isinstance(child, dict) and bool(child.get("group_names")) for child in actions
    ]
    if any(per_child) and not all(per_child):
        effect.pop("group_names", None)


def parse_ability(triggerless_text: str) -> Dict[str, Any]:
    """Parse a complete ability text."""
    triggerless_text = normalize_multiline(triggerless_text.strip())

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
    cost_icons = detect_icon_positions(triggerless_text)
    extra_pos_from_cost = cost_icons[0] if cost_icons else None

    # Parse effect
    effect = None
    if effect_text:
        effect = parse_effect(effect_text)
        if isinstance(effect, dict) and "cost" in effect:
            ability["cost"] = effect.pop("cost")
        effect = _normalize_parsed_effect(effect, triggerless_text)
        if not isinstance(effect, dict):
            effect = {}

        # Fill a missing condition from the text. Scan ONLY the effect text
        # (never the cost) and ONLY when the effect handler did not already
        # produce a condition — otherwise this pass would double-gate or
        # re-derive a worse copy of an existing condition.
        if not effect.get("condition"):
            if effect.get("action") == "sequential":
                # Sequential gating is handled by the そうした場合 executor.
                pass
            else:
                # Nothing to merge: attach the gate the text states, else
                # promote the first sub-action's own condition.
                gate = _leading_gate_condition(effect_text) or _sub_action_condition(
                    effect
                )
                if gate is not None:
                    effect["condition"] = copy.deepcopy(gate)

        # Apply activation_position from cost text to the effect
        if extra_pos_from_cost and "activation_position" not in effect:
            effect["activation_position"] = extra_pos_from_cost

        _fix_sequential_chain(effect)
        effect = _clean(effect)
        _validate_effect(effect, triggerless_text[:40])
        ability["effect"] = effect

    if (
        isinstance(effect, dict)
        and effect.get("action") == "sequential"
        and effect.get("conditional") is True
        and ability.get("cost")
        and "そうした場合、これにより公開したカード" in triggerless_text
    ):
        effect.pop("conditional", None)
        ability["effect"] = effect

    _demote_partial_group_names(effect)

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

    # Promote self-cost reduction clauses onto the cost (see helper docstring).
    _promote_self_cost_reduction(ability)

    # Clean cost too
    if "cost" in ability:
        ability["cost"] = _clean(ability["cost"])

    return ability


# ======================================================================
# COST RULE REGISTRY - ADD A COST PHRASE HERE
# Each handler (text, cost) -> Optional[dict] is tried in list order; the
# first non-None return wins and becomes the parsed cost. Recursive sub-cost
# parsing (choice/sequential) calls parse_cost itself. Handlers that merely
# set fields instead return None and fall through to the generic field
# accumulation + type classification at the bottom of parse_cost.
# ======================================================================
_DURATION_PROPAGATED_ACTIONS = frozenset(
    ("gain_resource", "modify_score", "change_state", "set_blade_count")
)


def _propagate_effect_duration(result: Dict[str, Any], duration: Optional[str]) -> None:
    if not duration:
        return
    action = result.get("action")
    if action in ("sequential", "conditional_alternative"):
        for child in result.get("actions", []):
            if "duration" not in child and child.get("action") in _DURATION_PROPAGATED_ACTIONS:
                child["duration"] = duration
        for key in ("primary_effect", "alternative_effect"):
            child = result.get(key)
            if (
                child
                and "duration" not in child
                and child.get("action") in _DURATION_PROPAGATED_ACTIONS
            ):
                child["duration"] = duration
    elif action == "choice":
        for child in result.get("options", []):
            if "duration" not in child and child.get("action") in _DURATION_PROPAGATED_ACTIONS:
                child["duration"] = duration
    elif action == "conditional_on_result":
        for key in ("primary_effect", "followup_action"):
            child = result.get(key)
            if child and "duration" not in child:
                child["duration"] = duration


def _finalize_effect(
    result: Dict[str, Any],
    prefix_effect: Dict[str, Any],
    parenthetical: List[str],
    activation_condition: Optional[Dict[str, Any]],
    activation_position: Optional[str],
) -> Dict[str, Any]:
    _merge_parenthetical(result, parenthetical)
    if "duration" in prefix_effect and "duration" not in result:
        result["duration"] = prefix_effect["duration"]
    _propagate_effect_duration(result, result.get("duration"))

    if result.get("action") not in ("choice", "conditional_on_result"):
        if activation_condition and "activation_condition_parsed" not in result:
            result["activation_condition_parsed"] = activation_condition
        if activation_position and "activation_position" not in result:
            result["activation_position"] = activation_position
    _propagate_optional(result)
    _strip_coo_child_optional(result)
    return result


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
                    if _is_real_condition(cond_parsed):
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
            _post = _effect_registry.get_post_normalize(hn)
            if _post is not None:
                result = _post(text, result)
            return _finalize_effect(
                result, effect, parenthetical, extra_activation_cond, extra_activation_pos
            )

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
    _propagate_optional(effect)

    # Post-fallback disambiguation overrides. NOTE: their position AFTER
    # parse_action is their semantics — they override wrong registry
    # decisions using normalized pattern_text. Do not fold these into the
    # main registry (dispatch order would change).
    normalized = re.sub(r"\s+", "", fallback_text)
    pattern_text = re.sub(r"\{\{[^|]+\|([^}]+)\}\}", r"\1", normalized)

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
            ic = fallback_text.count(BLADE_ICON)
            if ic > 0:
                effect["count"] = ic
        elif "ハートを得る" in fallback_text or "選んだハート" in fallback_text:
            effect["action"] = "gain_resource"
            effect["resource"] = "heart"
        elif "引く" in fallback_text:
            effect["action"] = "draw_card"

    return _strip_coo_child_optional(effect)


def _enrich_condition_common(d: Dict[str, Any], text: str) -> None:
    """Apply shared enrichments that both the handler path and fallthrough path need."""
    if "scope" not in d and "自分と相手" in text:
        d["scope"] = "both"
    if "position" not in d and "positions_characters" not in d:
        set_cross_position_fields(d, text)
    elif (
        "position_compare" not in d
        and d.get("comparison_type") == "equality"
    ):
        matched = {pos for _, pos in detect_position_matches(text)}
        matched.discard(d.get("position"))
        if matched:
            d["position_compare"] = sorted(matched)[0]
    if d.get("position") and d.get("position_compare"):
        if re.search(
            r"(?:センターエリア|左サイドエリア|右サイドエリア|センター|左サイド|右サイド).*にいる",
            text,
        ):
            d["require_position_cards"] = True
    _enrich_or_location(d, text)
    _enrich_heart_content(d, text)


def _enrich_condition_result(result: Dict[str, Any], text: str) -> Dict[str, Any]:
    if "cost_limit" not in result and "cost_limit_operator" not in result:
        cost_limit = extract_cost_limit_with_operator(text)
        if cost_limit:
            result["cost_limit"], result["cost_limit_operator"] = cost_limit
    if "card_property" not in result:
        card_property = detect_card_property(text)
        if card_property:
            result["card_property"] = card_property[0]
            if card_property[1]:
                result["negation"] = True
    if "aggregate" not in result and "合計" in text:
        result["aggregate"] = "total"
    _enrich_condition_common(result, text)
    if "より多くの" in text and "ブレード" in text and "持つ" in text:
        result["blade_greater_than_all"] = True
    return result


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
            _post = _condition_registry.get_post_normalize(name)
            if _post is not None:
                result = _post(text, result)
            return _enrich_condition_result(result, text)

    # Fall-through: generic field extraction + type inference
    condition: Dict[str, Any] = {"text": text}
    _extract_generic_fields(condition, text)
    _enrich_condition_result(condition, text)
    return _infer_condition_type(condition, text)


def _ic(t, tag):
    return t.count(tag) or None


def _handle_dynamic_count(text, action):
    """Handle dynamic count patterns in look_at actions."""
    # Check for dynamic count patterns (e.g., "スコアに2を足した数に等しい枚数")
    dynamic_count = extract_dynamic_count(text)
    if dynamic_count:
        action["dynamic_count"] = dynamic_count
    return action


def _apply_no_ability_filter(d, text):
    """Set ability_filter=no_ability/no_ability_type (+ trigger list) on `d`.

    Shared by the parse-time derivations in `_handle_cost_modification`
    (modify_cost) and `parse_action` (select/select_cards); mirrors the
    「能力を持たない」 handling in `_enrich_from_text`.
    """
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


# 「コストはNになる」 — the cost is SET rather than added to or subtracted from.
_COST_SET_RE = re.compile(r"コスト[はがを](\d+)になる")

# 「コストをN減る/増やす」 and 「コストを+N」 — the amount a cost change is worth.
_COST_DELTA_RE = re.compile(r"コスト[はがを](\d+)(減る|減らす|増える|増やす)")
_COST_SIGNED_RE = re.compile(r"コスト[をがは][+＋](\d+)")

# The operator a stated cost limit is qualified with. Ordered: an unqualified
# 「コスト10の」 means exactly that cost, so it is the fallback rather than a row.
_COST_LIMIT_OPERATORS = (
    ("以下", "<="),
    ("以上", ">="),
    ("未満", "<"),
    ("超", ">"),
)


def _apply_cost_operation(action, text):
    """Name what the cost change does: subtract, add, or set."""
    if "減る" in text or "減らす" in text or "マイナス" in text:
        action["operation"] = "subtract"
        return
    if "増える" in text or "増やす" in text or "プラス" in text or "コストを+" in text:
        action["operation"] = "add"
        return
    if not (_COST_SET_RE.search(text) or "コストは10になる" in text):
        return
    action["operation"] = "set"
    set_match = _COST_SET_RE.search(text)
    if set_match:
        action["value"] = as_int(set_match.group(1))
    # A hand-cost modifier is about the card being played, so the engine has to
    # read the card in hand rather than on the stage.
    if "このカード" in text or "このメンバーカード" in text:
        action["source"] = "hand"
        action["location"] = "hand"
        action["card_type"] = "member_card"


def _apply_cost_limit(action, text):
    """The cost range a modification applies to (e.g. 「コスト10の」)."""
    limit = extract_cost_limit(text)
    if limit is None:
        return
    action["cost_limit"] = limit
    for phrase, operator in _COST_LIMIT_OPERATORS:
        if phrase in text:
            action["cost_limit_operator"] = operator
            return
    action["cost_limit_operator"] = "="


def _apply_cost_value(action, text):
    """The numeric amount the cost change is worth."""
    delta = _COST_DELTA_RE.search(text)
    if delta:
        action["value"] = as_int(delta.group(1))
        return
    signed = _COST_SIGNED_RE.search(text)
    if signed:
        action["value"] = as_int(signed.group(1))


def _handle_cost_modification(text, action):
    """Handle cost modification patterns."""
    _apply_cost_operation(action, text)
    # Set location for hand-based cost reductions (手札にある/手札から)
    if "手札" in text:
        action["location"] = "hand"
    _apply_cost_limit(action, text)
    _apply_cost_value(action, text)
    icon_count = text.count(ENERGY_ICON)
    if icon_count > 0:
        action["count"] = icon_count
    # Replace コスト with energy texticon in the action text so the
    # frontend can render an icon instead of plain kanji for the bonus.
    # Also set count from value when no explicit icon count was found
    # (the +N value represents N energy units of cost bonus). Set/cost-becomes
    # modifiers (コストは10になる) do NOT get a count — only additive deltas do.
    if "value" in action and "コスト" in action.get("text", text):
        if "count" not in action and action.get("operation") != "set":
            action["count"] = action["value"]
        action["text"] = action.get("text", text).replace(
            "コスト", ENERGY_ICON, 1
        )
    # "このメンバーのコストを+Nする" → self_target (modifies only the activating card)
    if "このメンバー" in text:
        action["self_target"] = True
    # 「（トリガー）能力を持たない」 restricts WHICH cards the cost change
    # applies to. Derived at parse time rather than as a post-parse backfill
    # (dissolved FIX 7, formerly in the finalization pipeline).
    if "ability_filter" not in action and (
        "能力を持たない" in text or "能力も持たない" in text
    ):
        _apply_no_ability_filter(action, text)
    return action


def _set_score_op(t, a):
    """Set operation and value for modify_score from text patterns."""
    sm = re.search(r"([+\-])(\d+)", t)
    if sm:
        a["value"] = as_int(sm.group(2))
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
    raw = re.findall(HEART_LABEL, t)
    # Every colour is normally written the same number of times; a clause that
    # disagrees falls back to the smallest count so it cannot over-grant.
    per_color = _uniform_count(_heart_label_counts(t)) or 1
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


def _set_state_change_action(text, action, state_change=None):
    """A state change: ウェイト / レスト / アクティブ, and what it applies to.

    The card type is the interesting part, because the clause decides it in a
    fixed order and two of the branches overlap:
      - 「このメンバー」: this member, and unless the clause says 以外, exactly
        this one (target self, count 1) rather than "any member"
      - 「エネルギー」 with no メンバー: an energy card
      - 「メンバー」 plus a state word: any member in that state
    """
    if state_change is not None:
        action["state_change"] = state_change
    target = extract_target(text)
    if target:
        action["target"] = target
    if "このメンバー" in text:
        action["card_type"] = "member_card"
        if "このメンバー以外" not in text:
            action.update({"target": "self", "self_target": True, "count": 1})
    elif "エネルギー" in text and "メンバー" not in text:
        action["card_type"] = "energy_card"
    elif "メンバー" in text and any(
        state in text for state in ("ウェイト", "レスト", "アクティブ")
    ):
        action["card_type"] = "member_card"
    if "してもよい" in text:
        action["optional"] = True
    return action

def _nearest_invalidation_trigger(text):
    ability_pos = text.find('能力')
    if ability_pos < 0:
        return None
    prefix = text[:ability_pos]
    triggers = ('登場', 'ライブ開始時', 'ライブ成功時', '起動', '常時')
    for icon in reversed(re.findall('\\{\\{[^|{}]+\\|([^}]+)\\}\\}', prefix)):
        if icon in triggers:
            return icon
    positions = [(prefix.rfind(trigger), trigger) for trigger in triggers]
    positions = [(position, trigger) for position, trigger in positions if position >= 0]
    return max(positions)[1] if positions else None

# The colour words a 「［緑ハート］」 clause can use, in the order the printed
# text uses them. heart00 is the "no colour named" fallback.
HEART_COLOR_BY_JP = {
    "緑": "heart01",
    "赤": "heart02",
    "青": "heart03",
    "黄": "heart04",
    "紫": "heart05",
    "白": "heart06",
}

# 「［緑ハート］」 — the bracketed colour name of a heart-selection clause.
_BRACKET_HEART_RE = re.compile(r"［([^］]+)ハート］")

# 「N枚になるまで」 — a draw-until clause.
_DRAW_UNTIL_RE = re.compile(r"(\d+)枚になるまで")


def _set_heart_selection_resource(t, a):
    """「［緑ハート］」 — gain hearts of one named colour, chosen by the player.

    An unrecognised colour word falls back to heart00 rather than failing, so a
    clause the parser has not seen still produces a usable action.
    """
    m = _BRACKET_HEART_RE.search(t)
    selected = m.group(1) if m is not None else ""
    a["resource"] = "heart"
    a["heart_selection"] = True
    a["heart_colors"] = [HEART_COLOR_BY_JP.get(selected, "heart00")]
    return a


def _set_action_000(t, a):
    """Target a deck: the main deck, else the energy deck."""
    a["target"] = "deck" if "デッキ" in t else "energy_deck"
    return a


def _set_action_002(t, a):
    """An optional, multi-target action."""
    a["optional"] = extract_optional(t)
    a["multiple_targets"] = True
    return a


def _set_action_003(t, a):
    """Pay energy, optionally — the count is the number of energy icons."""
    a["energy"] = t.count(ENERGY_ICON)
    a["optional"] = "もよい" in t or "してもよい" in t
    return a


def _set_action_004(t, a):
    """The energy count, defaulting to the action's own count."""
    a["energy_count"] = a.get("count") or 1
    return a


def _set_action_005(t, a):
    """The target is whichever member this ability is on."""
    a["target_member"] = "this_member"
    return a


def _set_action_006(t, a):
    """Self-targeted; member_card when the clause says so, else any card."""
    a["self_target"] = True
    a["card_type"] = "member_card" if "メンバー" in t else "card"
    return a


def _set_action_007(t, a):
    """Target whatever is under the activating member."""
    a["under_self"] = True
    return a


def _set_action_008(t, a):
    """Draw from the deck into hand, until the hand holds N cards."""
    a["source"] = "deck"
    a["destination"] = "hand"
    a["target_count"] = as_int(_DRAW_UNTIL_RE.search(t).group(1))
    return a


def _set_action_009(t, a):
    """Draw until the hand holds N cards."""
    a["target_count"] = as_int(_DRAW_UNTIL_RE.search(t).group(1))
    return a


def _set_action_010(t, a):
    """Optionally draw one card from the deck into hand."""
    a["count"] = 1
    a["optional"] = True
    a["source"] = "deck"
    a["destination"] = "hand"
    return a


def _set_action_011(t, a):
    """Discard one card from hand."""
    a["source"] = "hand"
    a["destination"] = "discard"
    return a


def _set_action_012(t, a):
    """Look at / take from the deck into hand."""
    a["source"] = "deck"
    a["destination"] = "hand"
    return a


def _restriction_target(t):
    """Which player a restriction clause names.

    「自分と相手」 and 「お互い」 both contain 自分 AND 相手, so the both-case has to
    be tested first. Three rules resolve this independently and two of them had
    it last, so a 「自分と相手が〜できない」 clause resolved to a single player.
    """
    if "自分と相手" in t or "お互い" in t:
        return "both"
    if "相手" in t:
        return "opponent"
    if "自分" in t:
        return "self"
    return None


def _set_action_013(t, a):
    """Optionally take a card from the deck into hand."""
    a["source"] = "deck"
    a["destination"] = "hand"
    a["optional"] = True
    return a


def _set_action_014(t, a):
    """Cost modification — see _handle_cost_modification for the field rules."""
    return _handle_cost_modification(t, a)


def _set_action_017(t, a):
    """Send this member to the active state."""
    return _set_state_change_action(t, a, "active")


def _set_action_019(t, a):
    """Pay to activate, rather than activating for free."""
    a["activation_type"] = "pay_to_activate"
    return a

def _set_action_020(t, a):
    """Restriction: cannot become a live card."""
    a["restriction_type"] = "cannot_live"
    a["target"] = _restriction_target(t)
    return a


def _set_action_021(t, a):
    """Restriction: cannot activate, optionally in one phase."""
    a["restriction_type"] = "cannot_activate"
    a["target"] = _restriction_target(t)
    a["phase"] = "active_phase" if "アクティブフェイズ" in t else None
    return a

def _set_action_023(t, a):
    """Restriction: cannot be sent to the wait state.

    Two clauses collapse into one restriction here, which is why this is not a
    dictionary lookup:
      - 「効果によっては」: conditional, `cannot_wait_by_effect`
      - otherwise: unconditional `cannot_wait`

    Target is read from the clause: both players, else the opponent, else self —
    that order matters, because 「自分と相手の」 also contains 「自分」.
    """
    a["restriction_type"] = (
        "cannot_wait_by_effect" if "効果によっては" in t else "cannot_wait"
    )
    # This rule tests 「相手の」, not 「相手」 — a bare 相手 is the OPPONENT'S,
    # whereas 「相手のカード」 names the card the restriction is written on. The
    # other two restriction rules do use the bare form; the difference is real
    # and they share only the both/self fallback.
    if "自分と相手" in t or "お互い" in t:
        a["target"] = "both"
    elif "相手の" in t:
        a["target"] = "opponent"
    elif "自分" in t:
        a["target"] = "self"
    else:
        a["target"] = None
    a["card_type"] = "member_card" if "メンバー" in t else None
    a["duration"] = "live_end" if "ライブ終了時まで" in t else None
    return a


def _set_action_025(t, a):
    """Restriction: cannot be placed at the stated destination."""
    a["restriction_type"] = "cannot_place"
    a["destination"] = _extract_place_restriction_destination(t)
    return a


def _set_action_026(t, a):
    """Restriction: cannot be placed at the stated destination."""
    a["restriction_type"] = "cannot_place"
    a["destination"] = _extract_place_restriction_destination(t)
    return a


def _set_action_030(t, a):
    """Position change: move the member to the front.

    Reads the target from the clause, applies the position-change fields, and
    marks a member-selection target when the clause names one. All four steps
    write into `a` in place; the caller ignores the return.
    """
    a["target"] = extract_target(t)
    _handle_position_change_fields(t, a)
    if "正面" in t:
        a["destination"] = "front"
    if "メンバー" in t and ("1人" in t or "N人" in t):
        a["target_member"] = "select"
    return a

def _set_action_033(t, a):
    """Blade gain counted per blade icon, defaulting to 1, this turn only."""
    a["resource"] = "blade"
    a["count"] = t.count(BLADE_ICON) or 1
    a["timing_condition"] = "moved_this_turn"
    return a


def _set_action_036(t, a):
    """Blade gain counted per blade icon, defaulting to 1."""
    a["resource"] = "blade"
    a["count"] = _ic(t, BLADE_ICON) or 1
    return a


def _set_action_037(t, a):
    """Blade gain counted per blade icon, unstated when there are none."""
    a["resource"] = "blade"
    a["count"] = t.count(BLADE_ICON) or None
    return a


def _set_action_038(t, a):
    """Heart gain counted per all-heart icon, unstated when there are none."""
    a["count"] = t.count(ALL_HEART_ICON) or None
    return a


def _set_action_039(t, a):
    """Plain heart gain, with no count of its own."""
    a["resource"] = "heart"
    return a

def _set_action_040(t, a):
    """「余剰ハートBake失去」 — lose a resource, as a negative gain.

    Which resource comes from the clause, and the count from the icons: blades
    first, then heart icons (which also record their colours), and only if
    neither is present does a bare 「N枚/個/つ」 supply the number.
    """
    if "余剰ハート" in t or "余分ハート" in t or "それら" in t:
        a["resource"] = "surplus_heart"
    elif "ブレード" in t:
        a["resource"] = "blade"
    else:
        a["resource"] = "heart"
    blade_count = len(re.findall(BLADE_ICON_RE, t))
    heart_count = _heart_count(t)
    if blade_count:
        a["count"] = blade_count
    elif heart_count:
        a["count"] = heart_count
        a["heart_colors"] = extract_heart_types(t)
    elif "count" not in a:
        count_match = re.search(r"(\d+)(?:つ|個|枚)", t)
        if count_match:
            a["count"] = as_int(count_match.group(1))
    if "すべて" in t or "全て" in t:
        a["all"] = True
    if "メンバーカード" in t or "ステージのメンバー" in t:
        a["card_type"] = "member_card"
    if "ライブ終了時まで" in t:
        a["duration"] = "live_end"
    cost_limit = extract_cost_limit_with_operator(t)
    if cost_limit:
        a["cost_limit"], a["cost_limit_operator"] = cost_limit
    a["sign"] = "negative"
    return a

def _set_action_041(t, a):
    """「ブレードハートを失い」 — lose the blade heart, then re-yell once.

    Three shapes of the same printed clause:
      - 「…できない」: cannot, so this rule does not apply at all
      - 「ブレードハートを失い」: the re-yell is one compound step, not two
      - otherwise: the plain blade-heart loss
    """
    if "できない" in t:
        return None
    if "ブレードハートを失い" in t:
        # The loss and the re-yell are one printed clause; the engine gets
        # them as a sequential so the re-yell happens after the loss.
        a.pop("lose_blade_hearts", None)
        a.pop("location", None)
        a["action"] = "sequential"
        a["actions"] = [
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
        ]
        return a
    a["lose_blade_hearts"] = True
    return a

def _set_action_042(t, a):
    """Look at the top of the deck.

    「デッキの上」 fixes the source; the look itself may still be a dynamic
    count, which is applied before the action is named.
    """
    if "デッキの上" in t:
        a["source"] = "deck_top"
    _handle_dynamic_count(t, a)
    a["action"] = "look_at"
    return a


def _set_action_043(t, a):
    """Reveal a card from hand, or from an opponent's hand.

    The source defaults to hand, and the two optional clauses are independent:
    「見ないで」 is a blind reveal (no picker) and 「～に選んでもらう」 names one.
    """
    a["source"] = a.get("source") or "hand"
    if "見ないで" in t:
        a["blind"] = True
    picker = extract_picker(t)
    if picker:
        a["picker"] = picker
    return a


def _set_action_046(t, a):
    """Heart colours, but only when nothing else already says what is targeted.

    A clause that names a source or a card type is about THAT, so a heart icon
    in it belongs to the other clause and must not become a filter here.
    """
    if a.get("source") or a.get("card_type") or "{{heart_" not in t:
        return None
    a["heart_colors"] = extract_heart_colors_from_text(t)
    return a


def _set_action_050(t, a):
    """Invalidate the nearest trigger mentioned in the text."""
    a["target_trigger"] = _nearest_invalidation_trigger(t)
    return a


def _set_action_051(t, a):
    """Suppress the ability whose trigger icon precedes 「能力」 in the text.

    The icon is whatever appears first before that word — reading the slice
    rather than the whole clause is what keeps a later icon from being picked up.
    """
    cut = t.find("能力")
    m = re.search(r"\{\{(\w+)\.png\|", t[:cut]) if cut >= 0 else None
    a["suppressed_trigger"] = m.group(1) if m else None
    return a

def _set_heart_type(t, a, allow_selected=False):
    """The heart type a 「ハート○に変える」 clause names.

    Shared by the two rules that read it. They differ only in whether
    「選んだハート」 is a valid answer, so that is the one parameter — the other
    three fields are the same clause and used to be written out twice.
    """
    m = re.search(HEART_ICON_ID, t)
    if m:
        a["heart_type"] = f"heart{m.group(1)}"
    elif allow_selected and "選んだハート" in t:
        a["heart_type"] = "selected"
    else:
        a["heart_type"] = None
    a["original_value"] = "元々" in t
    a["self_target"] = "このメンバー" in t or "このカード" in t
    a["card_type"] = "member_card" if "メンバー" in t else None
    return a


def _set_action_057(t, a):
    """Set a named heart type, from a printed icon only."""
    return _set_heart_type(t, a)

def _set_action_058(t, a):
    """Offer a choice, aimed at self."""
    a["choice"] = True
    a["target"] = "self"
    return a


def _set_action_059(t, a):
    """Gain a heart of the chosen colour."""
    a["resource"] = "heart"
    a["heart_selection"] = True
    return a


def _set_action_060(t, a):
    """Set a heart type: a printed icon, or the chosen heart."""
    return _set_heart_type(t, a, allow_selected=True)

def _set_action_062(t, a):
    """Cost modification — see _handle_cost_modification for the field rules."""
    return _handle_cost_modification(t, a)


def _set_action_063(t, a):
    """「N回」 — a use limit of N per turn."""
    m = re.search(r"(\d+)回", t)
    if m is not None:
        a["max_repeats"] = as_int(m.group(1))
    return a


def _set_action_069(t, a):
    """「Nになる/なった」 — set a value to N."""
    a["operation"] = "set"
    m = re.search(r"(\d+).*(になる|なった|なっている)", t)
    if m:
        a["value"] = as_int(m.group(1))
    return a


def _set_action_070(t, a):
    """Score operation — see _set_score_op for the field rules."""
    return _set_score_op(t, a)


def _set_action_071(t, a):
    """Place on top of the deck; 「好きな順番で」 also makes the order free."""
    a["destination"] = "deck_top"
    if "好きな順番で" in t:
        a["placement_order"] = "any_order"
    return a


# 「{{icon.png|label}}能力」 — any {{...}} icon markup.
_ICON_MARKUP_RE = re.compile(r"\{\{([^}]+)\}\}")


def _trigger_filter_labels(t):
    """The text label of the icon naming which ability triggers, or [].

    The label is the LAST `|`-separated field, because the clause is written
    with the icon file first and the printed text second
    (`{{icon.png|このカード}}能力`). The icon named is the one between 「持つ」
    and 「能力」.
    """
    if "持つ" not in t:
        return []
    between = t.split("持つ", 1)[1].split("能力", 1)[0]
    m = _ICON_MARKUP_RE.search(between)
    return [m.group(1).split("|")[-1]] if m else []


def _set_action_073(t, a):
    """Trigger on the ability of whatever sits under the member."""
    a["source_location"] = "under_member"
    a["trigger_filter"] = _trigger_filter_labels(t)
    a["all"] = True
    return a


def _set_action_074(t, a):
    """Which ability is gained, read from the clause, unless already known."""
    if a.get("ability_gain") is None:
        a["ability_gain"] = (
            _strip_icon_annotations(t)
            .replace("を失う", "")
            .replace("を得る", "")
            .replace("をえる", "")
        )
    return a


def _set_action_079(t, a):
    """Named identities, matched against any group, from any region."""
    a["identities"] = _quoted_group_names(t) or None
    a["all_regions"] = True
    return a


def _set_action_080(t, a):
    """A count, defaulting to 1 when the clause does not state one."""
    a["count"] = extract_count(t) or 1
    return a


def _set_action_081(t, a):
    """Keep the whole clause as the condition text."""
    a["condition_text"] = t
    return a

_ACTION_RULES: List[ActionRule] = [
    ActionRule(name='action_000_shuffle', condition=lambda t: _has_shuffle(t), action='shuffle', setter=_set_action_000, priority=0, order=0),
    ActionRule(name='action_001_position_change', condition=lambda t: '入れ替える' in t or '入れ替えて' in t, action='position_change', setter=None, priority=1, order=1),
    ActionRule(name='action_002_position_change', condition=lambda t: 'フォーメーションチェンジ' in t, action='position_change', setter=_set_action_002, priority=2, order=2),
    ActionRule(name='action_003_pay_energy', condition=lambda t: '{{icon_energy.png|E}}' in t and ('支払う' in t or '支払って' in t) and ('選び' not in t), action='pay_energy', setter=_set_action_003, priority=3, order=3),
    ActionRule(name='action_004_place_energy_under_member', condition=lambda t, a: a.get('destination') == 'under_member' and ('エネルギー' in t or 'energy_card' in t), action='place_energy_under_member', setter=_set_action_004, priority=4, order=4),
    ActionRule(name='action_005_place_energy_under_member', condition=lambda t, a: a.get('source') == 'energy_deck' and 'このメンバーの下にある' in t, action='place_energy_under_member', setter=_set_action_005, priority=5, order=5),
    ActionRule(name='action_006_move_cards', condition=lambda t, a: a.get('destination') == 'under_member' and 'source' not in a and ('エネルギー' not in t) and ('置く' in t or '置いて' in t), action='move_cards', setter=_set_action_006, priority=6, order=6),
    ActionRule(name='action_007_move_cards', condition=lambda t, a: a.get('destination') == 'under_member' and 'このメンバーの下に' in t, action='move_cards', setter=_set_action_007, priority=7, order=7),
    ActionRule(name='action_008_draw_until_count', condition=lambda t: '枚になるまで' in t and '引く' in t, action='draw_until_count', setter=_set_action_008, priority=8, order=8),
    ActionRule(name='action_009_discard_until_count', condition=lambda t: '枚になるまで' in t and ('控え室に置く' in t or '控え室に置き' in t), action='discard_until_count', setter=_set_action_009, priority=9, order=9),
    ActionRule(name='action_010_draw_card', match='カードを1枚引いてもよい', action='draw_card', setter=_set_action_010, priority=10, order=10),
    ActionRule(name='action_011_move_cards', condition=lambda t: '引いた枚数' in t and '手札から' in t and ('控え室に置く' in t), action='move_cards', setter=_set_action_011, priority=11, order=11),
    ActionRule(name='action_012_draw_card', condition=lambda t: ('引く' in t or '引き' in t or '引い' in t) and '手札から控え室に置く' not in t, action='draw_card', setter=_set_action_012, priority=12, order=12),
    ActionRule(name='action_013_draw_card', condition=lambda t: '引いてもよい' in t, action='draw_card', setter=_set_action_013, priority=13, order=13),
    ActionRule(name='action_014_modify_cost', condition=lambda t: re.search('コスト[はが](\\d+)(減る|減らす|増える|増やす)', t) or re.search('ためのコストは(\\d+)減る', t), action='modify_cost', setter=_set_action_014, priority=14, order=14),
    ActionRule(name='action_015_move_cards', condition=lambda t, a: 'source' in a and a.get('source') and ('destination' in a) and a.get('destination') and ('選ぶ' not in t) and ('選び' not in t) and (not ('手札に加える' in t and extract_count(t) is not None)), action='move_cards', setter=None, priority=15, order=15),
    ActionRule(name='action_016_change_state', condition=lambda t, a: a.get('state_change') and a.get('state_change') != '', action='change_state', setter=_set_state_change_action, priority=16, order=16),
    ActionRule(name='action_017_change_state', condition=lambda t: 'アクティブにしてもよい' in t or 'アクティブにする' in t or ('アクティブにし' in t and 'しない' not in t), action='change_state', setter=_set_action_017, priority=17, order=17),
    ActionRule(name='action_018_activation_restriction', match_any=['のみ起動できる', 'のみ発動する'], action='activation_restriction', defaults={'restriction_type': 'only'}, priority=18, order=18),
    ActionRule(name='action_019_activate_ability', match='支払って発動させる', action='activate_ability', setter=_set_action_019, priority=19, order=19),
    ActionRule(name='action_020_restriction', match='ライブできない', action='restriction', setter=_set_action_020, priority=20, order=20),
    ActionRule(name='action_021_restriction', match='アクティブにしない', action='restriction', setter=_set_action_021, priority=21, order=21),
    ActionRule(name='action_022_restriction', condition=lambda t: 'アクティブしない' in t and 'アクティブにしない' not in t, action='restriction', defaults={'restriction_type': 'cannot_active', 'delayed': True}, priority=22, order=22),
    ActionRule(name='action_023_restriction', match='ウェイトしない', action='restriction', setter=_set_action_023, priority=23, order=23),
    ActionRule(name='action_024_restriction', match='バトンタッチで控え室に置けない', action='restriction', defaults={'restriction_type': 'cannot_baton_touch'}, priority=24, order=24),
    ActionRule(name='action_025_restriction', match='置くことができない', action='restriction', setter=_set_action_025, priority=25, order=25),
    ActionRule(name='action_026_restriction', match='置けない', action='restriction', setter=_set_action_026, priority=26, order=26),
    ActionRule(name='action_027_restriction', match='登場できない', action='restriction', defaults={'restriction_type': 'cannot_appear'}, priority=27, order=27),
    ActionRule(name='action_028_restriction', match='移動できない', action='restriction', defaults={'restriction_type': 'cannot_move'}, priority=28, order=28),
    ActionRule(name='action_029_move_cards', match_any=['加える', '加え'], exclude_any=['選ぶ', '選び'], action='move_cards', defaults={'destination': 'hand'}, priority=29, order=29),
    ActionRule(name='action_030_position_change', match='ポジションチェンジ', action='position_change', setter=_set_action_030, priority=30, order=30),
    ActionRule(name='action_031_position_change', match_all=['移動させ', 'エリア'], action='position_change', priority=31, order=31),
    ActionRule(name='action_032_move_cards', match='移動させ', exclude='エリア', action='move_cards', priority=32, order=32),
    ActionRule(name='action_033_gain_resource', match_all=['エリアを移動', 'ブレード'], action='gain_resource', setter=_set_action_033, priority=33, order=33),
    ActionRule(name='action_034_position_change', match_any=['移動する', '移動し'], action='position_change', priority=34, order=34),
    ActionRule(name='action_035_move_cards', condition=lambda t: (('置く' in t or '置いて' in t) or ('置き' in t and '置き場' not in t)) and '選ぶ' not in t and ('選び' not in t), action='move_cards', priority=35, order=35),
    ActionRule(name='action_036_gain_resource', condition=lambda t: 'ブレードを得る' in t or '選んだブレード' in t, action='gain_resource', setter=_set_action_036, priority=36, order=36),
    ActionRule(name='action_037_gain_resource', condition=lambda t: BLADE_ICON in t and '得る' in t and (not _blade_icon_is_target_filter(t)), action='gain_resource', setter=_set_action_037, priority=37, order=37),
    ActionRule(name='action_038_gain_resource', match='得る', condition=lambda t: '{{icon_all.png' in t, action='gain_resource', defaults={'resource': 'heart', 'heart_type': 'all'}, setter=_set_action_038, priority=38, order=38),
    ActionRule(name='action_039_gain_resource', condition=lambda t: '{{heart' in t and '得る' in t or bool(re.search('ハート.*得る', t)) or ('選んだハート' in t and 'になる' not in t), action='gain_resource', setter=_set_action_039, priority=39, order=39),
    ActionRule(name='action_040_gain_resource', condition=lambda t: re.search(r'(ブレード|ハート|余剰ハート|それら).*?失[うい]', t) is not None and 'もう一度エール' not in t and 'もう1度エール' not in t, action='gain_resource', setter=_set_action_040, priority=40, order=40),
    ActionRule(name='action_041_re_yell', condition=lambda t: 'もう一度エール' in t or 'もう1度エール' in t, action='re_yell', setter=_set_action_041, priority=41, order=41),
    ActionRule(name='action_042_look_at', condition=lambda t: '見る' in t or '見て' in t or t.endswith('見'), action='look_at', setter=_set_action_042, priority=42, order=42),
    ActionRule(name='action_043_reveal', condition=lambda t: '公開する' in t or '公開して' in t, action='reveal', setter=_set_action_043, priority=43, order=43),
    ActionRule(name='action_044_choice', condition=lambda t: '1つを選ぶ' in t and ('以下から' in t or 'のうち' in t) and ('{{heart_' not in t) and ('ハート' not in t), action='choice', priority=44, order=44),
    ActionRule(name='action_045_select_number', condition=lambda t: bool(re.search('数\\d*つを選ぶ', t)), action='select_number', priority=45, order=45),
    ActionRule(name='action_046_select', condition=lambda t: '選ぶ' in t or '選び' in t or bool(re.search('選ん(?!だ)', t)), action='select', setter=_set_action_046, priority=46, order=46),
    ActionRule(name='action_047_gain_resource', condition=lambda t: bool(re.search('ハート.*得る', t)) or ('選んだハート' in t and 'になる' not in t), action='gain_resource', priority=47, order=47),
    ActionRule(name='action_048_move_cards', match='登場させ', action='move_cards', defaults={'destination': 'stage'}, priority=48, order=48),
    ActionRule(name='action_049_activate_ability', match_any=['起動でき', '起動して'], action='activate_ability', priority=49, order=49),
    ActionRule(name='action_050_invalidate_ability', match='無効に', exclude='無効にできない', action='invalidate_ability', setter=_set_action_050, priority=50, order=50),
    ActionRule(name='action_051_suppress_ability_trigger', match='能力は発動しない', action='suppress_ability_trigger', setter=_set_action_051, priority=51, order=51),
    ActionRule(name='action_052_modify_required_hearts', match_any=['必要ハート', 'ハートを増やす', 'ハートを減らす', '増やす', '減らす', '減らし'], action='modify_required_hearts', priority=52, order=52),
    ActionRule(name='action_053_modify_score', match='追加', exclude='エール', action='modify_score', defaults={'operation': 'add'}, priority=53, order=53),
    ActionRule(name='action_054_modify_score', match_any=['スコアを1プラス', 'スコアをプラス'], action='modify_score', defaults={'operation': 'add', 'value': 1}, priority=54, order=54),
    ActionRule(name='action_055_modify_score', match='スコアを1マイナス', action='modify_score', defaults={'operation': 'remove', 'value': 1}, priority=55, order=55),
    ActionRule(name='action_056_set_blade_type', match='ブレードの色を', action='set_blade_type', priority=56, order=56),
    ActionRule(name='action_057_set_heart_type', condition=lambda t: 'ハートを' in t and 'すべて' in t and ('{{heart_' in t) and ('にする' in t) and (not 'ハートの色を' in t), action='set_heart_type', setter=_set_action_057, priority=57, order=57),
    ActionRule(name='action_058_specify_heart_color', condition=lambda t: 'ハートの色を' in t, action='specify_heart_color', setter=_set_action_058, priority=58, order=58),
    ActionRule(name='action_059_gain_resource', condition=lambda t: 'ハートを' in t and 'にする' in t and (not ('すべて' in t and '{{heart_' in t)) and (not 'ハートの色を' in t), action='gain_resource', setter=_set_action_059, priority=59, order=59),
    ActionRule(name='action_060_set_heart_type', condition=lambda t: ('ハートは' in t or 'ハートが' in t) and 'になる' in t and ('ハートを' not in t) and ('すべて' in t and '{{heart_' in t or '選んだハート' in t), action='set_heart_type', setter=_set_action_060, priority=60, order=60),
    ActionRule(name='action_061_modify_required_hearts', condition=lambda t: ('コストを' in t or 'コストが' in t or 'コストは' in t) and '{{heart_' in t, action='modify_required_hearts', setter=_handle_required_hearts, priority=61, order=61),
    ActionRule(name='action_062_modify_cost', condition=lambda t: 'コストを' in t or 'コストが' in t or 'コストは' in t, action='modify_cost', setter=_set_action_062, priority=62, order=62),
    ActionRule(name='action_063_repeat_procedure', match='繰り返してもよい', action='repeat_procedure', setter=_set_action_063, priority=63, order=63),
    ActionRule(name='action_064_do_nothing', match='何もしない', action='do_nothing', priority=64, order=64),
    ActionRule(name='action_065_do_nothing', condition=lambda t: t.strip() == '', action='do_nothing', priority=65, order=65),
    ActionRule(name='action_066_pay_energy', match_all=['{{icon_energy.png|E}}', 'エネルギー'], action='pay_energy', priority=66, order=66),
    ActionRule(name='action_067_play_baton_touch', match_any=['バトンタッチ', 'baton touch'], action='play_baton_touch', priority=67, order=67),
    ActionRule(name='action_068_invalidate_ability', match='無効にできない', action='invalidate_ability', defaults={'optional': True}, priority=68, order=68),
    ActionRule(name='action_069_modify_score', condition=lambda t: ('スコアは' in t or 'スコアが' in t) and ('になる' in t or 'なった' in t or 'なっている' in t), action='modify_score', setter=_set_action_069, priority=69, order=69),
    ActionRule(name='action_070_modify_score', match='スコアを', action='modify_score', setter=_set_action_070, priority=70, order=70),
    ActionRule(name='action_071_move_cards', match_any=['デッキの上に置き', 'デッキの上に置く'], action='move_cards', setter=_set_action_071, priority=71, order=71),
    ActionRule(name='action_072_modify_yell_count', match_all=['エール'], match_any=['枚数', '数'], action='modify_yell_count', priority=72, order=72),
    ActionRule(name='action_073_gain_ability_from_source', condition=lambda t: '持つ' in t and '能力' in t and ('得る' in t) and ('すべて' in t), action='gain_ability_from_source', setter=_set_action_073, priority=73, order=73),
    ActionRule(name='action_074_gain_ability', condition=lambda t: '得る' in t and any((kw in t for kw in ('能力', '常時', 'ライブ成功時', 'ライブ開始時', '登場', '起動'))), action='gain_ability', setter=_set_action_074, priority=74, order=74),
    ActionRule(name='action_075_reduce_live_card_set_limit', match_all=['ライブカードセットフェイズ', '減る'], match_any=['上限', '枚数'], action='reduce_live_card_set_limit', priority=75, order=75),
    ActionRule(name='action_076_set_card_identity', match_any=['セット', '設定'], exclude='コスト', action='set_card_identity', priority=76, order=76),
    ActionRule(name='action_077_choose_required_hearts', match='必要ハートを選ぶ', action='choose_required_hearts', priority=77, order=77),
    ActionRule(name='action_078_all_blade_timing', match_all=['必要ハートを確認する時', 'ALLブレード', '任意の色のハートとして扱う'], action='all_blade_timing', defaults={'timing': 'check_required_hearts', 'treat_as': 'any_heart_color'}, priority=78, order=78),
    ActionRule(name='action_079_set_card_identity', match_all=['すべての領域にあるこのカードは', 'として扱う'], action='set_card_identity', setter=_set_action_079, priority=79, order=79),
    ActionRule(name='action_080_perform_yell', condition=lambda t: bool(re.search('追加で.*エール.*行', t)), action='perform_yell', setter=_set_action_080, priority=80, order=80),
    ActionRule(name='action_081_conditional_alternative', match_all=['代わりに', '置く', '場合'], action='conditional_alternative', setter=_set_action_081, priority=81, order=81),
    ActionRule(name='action_082_gain_resource', condition=lambda t: bool(re.search('［[^］]+ハート］', t)), action='gain_resource', setter=_set_heart_selection_resource, priority=82, order=82),
]



def _blade_icon_is_target_filter(text: str) -> bool:
    """True when the {{icon_blade.png|ブレード}} icon describes the TARGET member's
    blade count (e.g. "ブレードを4つ以上持つ『虹ヶ咲』のメンバー") rather than a
    resource being granted. Used to keep the concurrent heart+blade-grant split
    from mistaking a blade-count filter for a blade gain (fix D19)."""
    if "ブレードの数が" in text:
        return True
    # The blade icon sits inside a "持つ" clause that precedes the gain clause.
    gain_head = text.split("得る", 1)[0]
    return bool(re.search(r"{{icon_blade\.png\|ブレード}}[^得]*持つ", gain_head))


def _per_unit_effect(text, info):
    """What the per-unit clause grants, and for how long.

    A 「〜につき」 clause can introduce one of three things — a blade, a heart, or
    a draw — and all three take the same duration, so the duration is read once
    here instead of in each branch.
    """
    gains_blade = "ブレードを得る" in text or "選んだブレード" in text
    gains_heart = bool(re.search(r"ハート.*得る", text)) or "選んだハート" in text
    if gains_blade:
        info["action"] = "gain_resource"
        info["resource"] = "blade"
        icon_count = text.count(BLADE_ICON)
        if icon_count > 0:
            info["count"] = icon_count
            info["resource_icon_count"] = icon_count
    elif gains_heart:
        info["action"] = "gain_resource"
        info["resource"] = "heart"
    elif "引く" in text:
        info["action"] = "draw_card"
    else:
        return False
    if "ライブ終了時まで" in text:
        info["duration"] = "live_end"
    return True


def _extract_per_unit_info_from_text(text):
    """Extract per-unit info from text. Returns (per_unit_info_dict or None, cleaned_text)."""
    per_unit_match = re.search(r"(.*?)につき", text)
    if not per_unit_match:
        return None, text
    per_unit_text = per_unit_match.group(1).strip()
    if "。" in per_unit_text:
        return None, text
    count_match = re.search(r"(\d+)(?:人|枚|つ)", per_unit_text)
    per_unit_count = as_int(count_match.group(1)) if count_match else 1
    per_unit_type = None
    if "成功ライブカード置き場" in per_unit_text:
        per_unit_type = "success_live_card_zone"
    elif (
        "ライブ中のカード" in per_unit_text
        or "ライブ中のライブカード" in per_unit_text
        or "ライブカード置き場" in per_unit_text
    ):
        per_unit_type = "live_card_zone"
    elif "メンバー" in per_unit_text:
        per_unit_type = "member"
    elif "カード" in per_unit_text:
        per_unit_type = "card"
    info: Dict[str, Any] = {"per_unit": True, "per_unit_count": per_unit_count}
    if per_unit_type:
        info["per_unit_type"] = per_unit_type
    if "メンバーの下" in per_unit_text:
        info["location"] = "under_member"
    if "ステージ" in per_unit_text:
        info["per_unit_location"] = "stage"
    if "エネルギーカード" in per_unit_text:
        info["card_type"] = "energy_card"
        if "エネルギーデッキ" in text or "これによって置いた" in per_unit_text:
            info["per_unit_type"] = "energy_deck"
    elif "メンバーカード" in per_unit_text:
        info["card_type"] = "member_card"
    if "ブレードハートを持たない" in per_unit_text:
        info["card_property"] = "has_blade_heart"
        info["negation"] = True
    elif "ブレードハートを持つ" in per_unit_text:
        info["card_property"] = "has_blade_heart"
    _per_unit_effect(text, info)
    cleaned = text.replace(per_unit_match.group(0), "").strip()
    return info, cleaned


def _check_ability_gain_from_text(text, action):
    """Check for ability gain pattern. Returns True if gain_ability (with early return)."""
    quoted_text = extract_all_quoted_names(text)
    is_ability_gain = False
    if "を得る" in text and "能力" in text and quoted_text:
        is_ability_gain = True
    elif "を得る" in text and quoted_text:
        categorized = categorize_quoted_text(quoted_text)
        if categorized["abilities"]:
            is_ability_gain = True
        elif any(
            kw in text
            for kw in ("常時", "登場", "起動", "ライブ成功時", "ライブ開始時", "ライブ中")
        ):
            is_ability_gain = True
    if not is_ability_gain:
        return False
    action["action"] = "gain_ability"
    if "これによ" in text:
        action["anaphora"] = "cost_waited"
    if quoted_text:
        categorized = categorize_quoted_text(quoted_text)
        if categorized["abilities"]:
            trigger_match = re.search(r"\{\{[^|]+\|([^}]+)\}\}", categorized["abilities"][0])
            if trigger_match:
                action["ability_gain_trigger"] = trigger_match.group(1)
            action["ability_gain"] = _strip_icon_annotations(categorized["abilities"][0])
        elif categorized["characters"]:
            if len(categorized["characters"]) == 1:
                action["quoted_text"] = {"text": categorized["characters"][0], "quoted_type": "character"}
    _fill_defaults(action, text)
    return True


def _check_heart_blade_split_from_text(text, action):
    """Check for heart+blade concurrent grant → sequential. Returns result dict or None."""
    all_heart_count = text.count(ALL_HEART_ICON)
    if (
        BLADE_ICON not in text
        or ("{{heart" not in text and all_heart_count == 0)
        or "得る" not in text
        or "N人が" in text
        or _blade_icon_is_target_filter(text)
    ):
        return None
    blade_count = text.count(BLADE_ICON)
    heart_colors = _heart_ids_gained(text)
    actions = []
    if blade_count:
        actions.append({"action": "gain_resource", "resource": "blade", "count": blade_count})
    if all_heart_count:
        actions.append(
            {
                "action": "gain_resource",
                "resource": "heart",
                "heart_type": "all",
                "count": all_heart_count,
            }
        )
    if heart_colors:
        actions.append(
            {
                "action": "gain_resource",
                "resource": "heart",
                "heart_colors": heart_colors,
                "count": len(heart_colors),
            }
        )
    if not actions:
        return None
    tc_match = re.search(r"(\d+)人", text)
    target_count = as_int(tc_match.group(1)) if tc_match else None
    is_same_name = "と同じ名前" in text or ("同じ名前" in text and "持つ" in text)
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


# ======================================================================
# ACTION PARSER
# ======================================================================

# Effect constraints (最小/最大/N未満にはならない/N以上にはならない) for
# parse_action. Keyed by the phrase that selects the rule; the pattern pulls
# out the bound. First match wins, so a more specific phrase listed earlier
# wins over a later one.
ACTION_CONSTRAINT_PATTERNS = (
    ("最小", "min", r"最小(\d+)"),
    ("最大", "max", r"最大(\d+)"),
    ("未満にはならない", "min", r"(\d+)未満にはならない"),
    ("以上にはならない", "max", r"(\d+)以上にはならない"),
)


def _split_period_before_per_unit(text):
    """「…。〜につき…」 — two sentences, so two actions.

    A period before the first 「につき」 means the per-unit clause belongs to a
    LATER sentence, so the two are split rather than parsed as one clause.

    When the first action counts from the discard (a baton touch replaced the
    member and put it in the waitroom) and a later action has a per-unit type
    that came from generic keyword matching ("メンバー" → "member"), the later
    one inherits the discard type: both sentences describe the same cards.
    Returns None when the text is a single clause.
    """
    first_period = text.find("。")
    first_perunit = text.find(PER_UNIT_MARKER)
    if not (0 <= first_period < first_perunit):
        return None
    parts = [part.strip().rstrip("。") for part in text.split("。") if part.strip()]
    actions = [parse_action(part) for part in parts]
    if actions and actions[0].get("per_unit_type") == "discard":
        for sub in actions[1:]:
            if sub.get("per_unit") and sub.get("per_unit_type") in ("member", "枚"):
                sub["per_unit_type"] = "discard"
    return {"text": text, "action": "sequential", "actions": actions}


def parse_action(text: str) -> Dict[str, Any]:
    """Parse an action text."""
    # (The "カードを1枚引いてもよい" optional-draw phrase is handled by the
    # _ACTION_RULES registry dispatch below; no dedicated early-return needed.)

    # Strip parenthetical notes first (before any other processing)
    text = strip_parenthetical(text)

    # A period before the first 「につき」 means the per-unit reference belongs to
    # a later sentence: split into a sequential so the earlier sentence (e.g. a
    # draw effect) is not consumed by the per-unit match.
    if "。" in text and PER_UNIT_MARKER in text:
        sequential = _split_period_before_per_unit(text)
        if sequential:
            return sequential

    per_unit_info = None
    if PER_UNIT_MARKER in text:
        per_unit_info, text = _extract_per_unit_info_from_text(text)

    # Strip duration prefixes
    text, dur_code = _strip_duration_prefix(text)
    action: Dict[str, Any] = {"text": text}
    if dur_code:
        action["duration"] = dur_code
    # Also check for duration keywords embedded in text (as opposed to
    # stripped as a prefix by _strip_duration_prefix above). Same vocabulary:
    # DURATION_PREFIX_MAP is the one place a duration phrase is spelled.
    if "duration" not in action:
        for _dur_phrase, _dur_code in DURATION_PREFIX_MAP.items():
            if _dur_phrase in text:
                action["duration"] = _dur_code
                break

    # Shared extraction context owns one scan for the common action fields.
    field_context = extract_field_context(
        text,
        source=extract_source,
        destination=extract_destination,
        target=extract_target,
        card_type=extract_card_type,
        count=extract_count,
    )
    count = field_context.count
    target = field_context.target
    card_type = field_context.card_type
    state_change = extract_state_change(text)

    if per_unit_info is not None:
        action.update(per_unit_info)

    # Extract effect constraints (最小/最大/N未満にはならない/N以上にはならない)
    constraint_text = normalize_fullwidth_digits(text)
    for keyword, constraint_type, pattern in ACTION_CONSTRAINT_PATTERNS:
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

    source = field_context.source
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

    destination = field_context.destination
    if destination:
        action["destination"] = destination
    # Check for "好きな順番で" (in any order) placement
    if "好きな順番で" in text:
        action["placement_order"] = "any_order"
    # Extract deck position (Q226: 一番上から4枚目)
    deck_position = extract_deck_position_for_action(text)
    if deck_position:
        action.update(deck_position)
        # "デッキの上からN番目に置く" — inserting at position N of the deck top.
        # The engine reads effect.position for the DeckTop insert index.
        if "destination" not in action and re.search(r"(デッキ|山札)の上から\d+[枚番]目", text):
            action["destination"] = "deck_top"

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
            action[op] = extract_operator(text) or "="

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

    # Apply the common fields at their original insertion points. These
    # policies intentionally overwrite rule fields to preserve parse ordering.
    if not count and "これにより引いた枚数と同じ枚数を" in text:
        action["dynamic_count"] = {"type": "drawn_cards", "reference": "previous_draw"}
    else:
        dynamic_count = extract_dynamic_count(text) if not count else None
        if dynamic_count:
            action["dynamic_count"] = dynamic_count
            if (
                action.get("action") == "place_energy_under_member"
                and "energy_count" in action
            ):
                del action["energy_count"]
    field_context.apply(action, ACTION_CARD_FIELDS_POLICY)

    # Extract position restrictions (e.g., "センター", "センターエリア")
    # Also detect cross-position patterns (e.g. "右サイドエリアと左サイドエリア")
    field_context.apply(action, ACTION_POSITION_POLICY)

    # Extract exclude_self for actions (e.g., "このメンバー以外の" or "「character name」以外")
    # Only for filtering actions, NOT for gain_resource/select (self-buffs)
    if detect_exclude_self(text) and action.get("action") not in ("gain_resource", "select", "heart_selection"):
        action["exclude_self"] = True
    # Also check for specific character name exclusions like "「鬼塚冬毬」以外"
    if re.search(r"「.+」以外", text):
        action["exclude_self"] = True
        # NOTE: the quoted names themselves are NOT captured here — a previous
        # revision computed categorize_quoted_text(...) and dropped the result.
        # If an effect ever needs structured exclusion lists, capture them here.

    # Extract group names from 『』 brackets
    group_names = extract_all_groups(text)
    if group_names:
        action["group_names"] = group_names
        # Re-apply exclude_group_names if already set (line 2143 may have set it
        # but group_names extraction above may re-add the excluded groups)
        exc_gns = action.get("exclude_group_names")
        if exc_gns:
            action["group_names"] = [
                g for g in action["group_names"] if g not in exc_gns
            ]

    # Check for ability gain pattern - early return
    if _check_ability_gain_from_text(text, action):
        return action
    # Extract quoted text from 「」 for other contexts
    quoted_text = extract_all_quoted_names(text)
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
        action["target_count"] = as_int(tc_match.group(1))

    # Extract deck position
    position = extract_deck_position_constraint(text)
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

    # Heart+blade concurrent grant → sequential
    hb_result = _check_heart_blade_split_from_text(text, action)
    if hb_result:
        return hb_result

    # DISPATCH TABLE
    action["action"] = "custom"
    for entry in _ACTION_RULES:
        if entry.matches(text, action):
            entry.apply(text, action)
            break

    # 「（トリガー）能力を持たない」 on selection actions filters WHAT may be
    # selected. Derived at parse time rather than as a post-parse backfill
    # (dissolved FIX 7b, formerly in the finalization pipeline).
    if (
        "ability_filter" not in action
        and action.get("action") in ("select", "select_cards")
        and ("能力を持たない" in text or "能力も持たない" in text)
    ):
        _apply_no_ability_filter(action, text)

    # Rule 11.10.1: a position change MUST move to a different area — exclude
    # the member's current position for single-target moves (not "メンバー1人を"
    # player selection or "それぞれ" formation change). Derived at parse time
    # rather than as a post-parse backfill (dissolved FIX 15).
    if (
        action.get("action") == "position_change"
        and "source_position" not in action
        and not action.get("exclude_self")
        and "それぞれ" not in text
        and not ("メンバー" in text and ("1人" in text or "N人" in text))
    ):
        action["exclude_self"] = True

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


def _try_this_turn_opponent_live_success(text):
    """「このターン、相手もライブを成功している場合」 — compound of this_turn + opponent success.
    Also handles bare 「相手もライブを成功している場合」 (without このターン、)."""
    if "相手もライブを成功している場合" in text:
        has_this_turn = "このターン" in text
        if has_this_turn:
            return {
                "type": "compound",
                "operator": "and",
                "conditions": [
                    {
                        "type": "temporal_condition",
                        "temporal": "this_turn",
                        "text": "このターン",
                        "trigger_event": {"type": "temporal", "temporal": "this_turn"},
                    },
                    {
                        "type": "opponent_live_success",
                        "text": "相手もライブを成功している場合",
                        "trigger_event": {"type": "opponent_live_success"},
                    },
                ],
                "text": text,
            }
        else:
            return {
                "type": "opponent_live_success",
                "text": text,
                "trigger_event": {"type": "opponent_live_success"},
            }
    return None


def _try_complex(text):
    return parse_complex_condition(text)


def _try_character_each(text):
    """「A」と「B」と「C」の...メンバーカードをそれぞれ1枚ずつ...控え室に置いた場合
    — "one of EACH named character placed into the discard". Emits a compound
    AND of one per-character location_condition so the engine requires ≥1 of
    every name, not merely 1 card matching any name.
    """
    if "それぞれ" not in text or "1枚ずつ" not in text:
        return None
    # Names joined by と/、 before のメンバーカード / のカード
    name_part = re.search(
        r"((?:「[^」]+」[と、]? ?)+)(?:」?の)?(?:メンバーカード|カード)", text
    )
    if not name_part:
        return None
    names = _quoted_names(name_part.group(1))
    if len(names) < 2:
        return None
    # Only for placement-to-zone conditions (手札から...控え室/下に置いた)
    if "控え室" not in text and "の下に置" not in text:
        return None
    # Location the cards were placed into (default discard / waitroom)
    location = "discard"
    if "デッキ" in text:
        location = "deck"
    subs = []
    for name in names:
        sub = {
            "type": "location_condition",
            "characters": [name],
            "location": location,
            "count": 1,
            "operator": ">=",
            "target": "self",
            # Short text WITHOUT "手札から" so the engine treats this as a
            # state-based discard check, not an event-based recent-move check.
            "text": "「{}」のメンバーカードが控え室にある".format(name),
        }
        subs.append(sub)
    result = {"type": "compound", "operator": "and", "conditions": subs, "text": text}
    if "控え室" in text:
        result["location"] = "discard"
    return result


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
        count = as_int(cm.group(1))
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
    # 「メンバーがN人(以上)いる」 restricts the counted cards to members;
    # without this a group live card (e.g. Hasunosora live in the waitroom,
    # PL!HS-pb1-026-L) satisfies 「蓮ノ空のメンバー」 via its series name.
    if "メンバー" in text:
        result["card_type"] = "member_card"
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
    if not m:
        # "名前の異なる『X』のメンバーが2人いる場合" — no 以上, exact people count.
        m = re.search(r"(\d+)人(?:いる|ある)", text)
    if m:
        result["count"] = as_int(m.group(1))
        result["operator"] = ">="
        result["unit"] = m.group(2) if len(m.groups()) >= 2 and m.group(2) else "人"
    gns = extract_all_groups(text)
    if gns:
        result["group_names"] = gns
    # Extract 「』-bracketed character names from のうち patterns
    cm = re.search(r"((?:「[^」]+」[とか、]? ?)+)(?:」?がいる|」?のうち)", text)
    if cm:
        names = _quoted_names(cm.group(1))
        if names:
            result["characters"] = names
    return result


def _blade_count_match(text):
    clean = re.sub(r"\{\{.*?\|([^}]+)\}\}", r"\1", text)
    for pattern, operator in (
        (r"ブレードが(\d+)つ以上", ">="),
        (r"ブレードの数が(\d+)以上", ">="),
        (r"ブレードが(\d+)より多い", ">"),
        (r"ブレードが(\d+)つ", ">="),
        (r"ブレードの数が(\d+)つ以上", ">="),
    ):
        match = re.search(pattern, clean)
        if match:
            return match, operator
    return None, None


def _set_blade_count_condition(text, result):
    match, operator = _blade_count_match(text)
    if match:
        result.update(
            {
                "type": "card_blade_condition",
                "count": as_int(match.group(1)),
                "operator": operator,
                "source": "selected_cards",
            }
        )


def _matches_blade_count_condition(text):
    match, _operator = _blade_count_match(text)
    return match is not None


_try_blade_count = ConditionPattern(
    condition=_matches_blade_count_condition,
    setter=_set_blade_count_condition,
)


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
    # Heart colors leaked from effect text into condition are incorrect.
    # The condition text "恰好で2人" has no heart icons; heart05 belongs only
    # in the effect's gain_resource. If condition text has no {{heart_ icons
    # but heart_colors is set, strip it — this is effect metadata, not a filter.
    hc = result.get("heart_colors")
    if hc and "{{heart_" not in text:
        del result["heart_colors"]
    # Unit → card_type inference
    u = result.get("unit", "")
    if u == "人":
        result["card_type"] = "member_card"
    # Extract exclude_self for "このメンバー以外" pattern
    if detect_exclude_self(text) or "このカード以外" in text:
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
        bracketed = _quoted_names(text)
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
    # Comparison targets: a literal phrase first, else a 「<noun>より」 phrasing.
    cmp_target = _contiguous_comparison_target(text) or _noun_comparison_target(text)
    if cmp_target:
        result["comparison_target"] = cmp_target
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
    gns = extract_all_groups(text)
    if gns and "を含む" not in text:
        result["group_names"] = gns
    # Cost limit
    cl_op = extract_cost_limit_with_operator(text)
    if cl_op:
        result["cost_limit"] = cl_op[0]
        result["cost_limit_operator"] = cl_op[1]
    # Heart colors
    colors = _heart_ids(text)
    if colors:
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
    _apply_phase_restriction(text, "main", result)


def _try_hand_count_compound(result, text):
    """Detect 手札がN枚以下 (hand ≤ N cards) pattern within a card_count_condition
    and promote to a compound AND condition: [original_condition, hand_count_condition].
    Returns the compound condition, or None if no hand count pattern found.
    """
    hand_m = re.search(r"手札が(\d+)枚以下(の|の場)", text)
    if not hand_m:
        return None
    hand_count = as_int(hand_m.group(1))
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
        (r"(\d+)種類以上の.+?色がある場合", ">=", "types"),
        (r"(\d+)枚ある", "=", None),
        (r"(\d+)枚以上", ">=", None),
        (r"(\d+)人以上", ">=", "人"),
        (r"(\d+)(人|枚|つ)以上いる", ">=", None),
    ]:
        m = re.search(pat, text)
        if m:
            result = {
                "type": "card_count_condition",
                "count": as_int(m.group(1)),
                "operator": op,
                "text": text,
            }
            if unit:
                result["unit"] = unit
            elif len(m.groups()) >= 2 and m.group(2):
                result["unit"] = m.group(2)
            # G18: "それらのメンバーカードの中に…色がある場合" — the color diversity
            # is counted among the cards just moved (preceding_moved), not the stage.
            if "それらの" in text and "色がある" in text:
                result["source"] = "preceding_moved"
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
            gns = extract_all_groups(text)
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
    """Simple phase gate: Xフェイズの場合/の間/に — restricts activation to a specific phase.

    Single owner: delegates to extract_phase_gate (same regex, phase map,
    turn-number/phase-target extraction, and consequence-skip for 次の/来
    references). The condition-registry path previously duplicated all of
    this with subtly different behavior."""
    gate, remaining = extract_phase_gate(text)
    if not gate:
        return None
    return gate


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
        result["turn_number"] = as_int(tm.group(1))
        result["trigger_event"]["turn_number"] = as_int(tm.group(1))
    return result


def _try_baton_touch(text):
    is_teita = (
        "とバトンタッチしていた" in text
        or "とバトンタッチしていて" in text
        or "バトンタッチしていた" in text
    )
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
        gns = extract_all_groups(text)
        if gns:
            result["group_names"] = gns
            if "characters" not in result:
                result["characters"] = gns[:1]
    count_m = re.search(rf"(\d+)人{sep}バトンタッチ", text)
    if count_m:
        te_data["min_count"] = as_int(count_m.group(1))
    # Extract cost limit (e.g., "コスト10以上" → cost_limit=10, cost_limit_operator=">=")
    cl_op = extract_cost_limit_with_operator(text)
    if cl_op:
        result["cost_limit"] = cl_op[0]
        result["cost_limit_operator"] = cl_op[1]
    else:
        # Exact cost like "コスト7のメンバーから" → cost 7, operator ==
        # Parser previously missed this because extract_cost_limit_with_operator requires 以上/以下
        cl = extract_cost_limit(text)
        if cl is not None and "コスト" in text and "メンバーから" in text and "バトンタッチ" in text:
            result["cost_limit"] = cl
            result["cost_limit_operator"] = "=="
    # Extract group_names (for から-form, this is handled below)
    gns = extract_all_groups(text)
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
    if detect_exclude_self(text):
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
        result["count"] = as_int(m.group(1))
        result["trigger_event"]["count"] = as_int(m.group(1))
    elif "登場" in text and "回" not in text:
        result["count"] = 1
    if "ライブフェイズ" in text:
        result["phase"] = "live_phase"
    else:
        _apply_phase_restriction(text, "main_phase", result, result["trigger_event"])
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
    gns = extract_all_groups(text)
    if gns:
        result["group_names"] = gns
    return result


def _try_or(text):
    # Delegate to the specialized OR helpers first — they build richer
    # movement/live_success legs than a generic split can. Falls back to the
    # generic split (handles compound triggers like this B4/かのん pattern).
    for specialized in (
        _try_live_success_or_move,
        _try_appear_or_move,
    ):
        try:
            specialized_result = specialized(text)
        except (KeyError, IndexError):
            specialized_result = None
        if specialized_result is not None:
            return specialized_result
    # OR trigger patterns: "A あるか、B" / "A するか、B" / "A たか、B" / "A か、B"
    # Split candidates from most-specific marker to generic "か、".
    # Only accept when every part parses into a real condition (guards against
    # generic "か、" that appears inside single clauses, e.g. "できるか、できないか").
    markers = ["あるか、", "するか、", "たか、", "か、"]
    for marker in markers:
        if marker not in text:
            continue
        parts = [p.strip() for p in text.split(marker) if p.strip()]
        if len(parts) < 2:
            continue
        # Restore "ある" suffix lost during split on "あるか、"
        if marker == "あるか、":
            parts = [
                p + "ある" if i < len(parts) - 1 else p
                for i, p in enumerate(parts)
            ]
        parsed = [parse_condition(p) for p in parts]
        if len(parsed) < 2 or any(
            p is None or p.get("type") in ("custom",) for p in parsed
        ):
            continue
        result = {
            "type": "or_condition",
            "operator": "or",
            "conditions": parsed,
            "text": text,
        }
        result = _fix_distributed_baton_arrival(result, text)
        leg_events = []
        for leg in result.get("conditions", []):
            event = leg.get("trigger_event")
            if event and event not in leg_events:
                leg_events.append(event)
        if leg_events:
            result["trigger_event"] = {
                "type": "or",
                "events": leg_events,
            }
        return result
    return None


def _fix_distributed_baton_arrival(result, text):
    """Fix "このメンバーか、ほかのメンバーがバトンタッチして登場(したとき)".

    The baton-touch arrival distributes over BOTH disjuncts (self arrival OR
    other-member arrival). The generic か、 split above misreads the first leg
    as a static presence check ("このメンバー" on stage — always true once
    staged), so the ability fires on any trigger scan, e.g. a plain debut.
    Rewrite that leg as the self-arrival movement leg; the engine's
    baton_touch evaluator (replaced-in-own-waitroom gate) handles the rest.
    """
    if result.get("type") != "or_condition":
        return result
    if "このメンバーか、" not in text or "バトンタッチして登場" not in text:
        return result
    conds = result.get("conditions", [])
    if len(conds) < 2:
        return result
    last = conds[-1]
    if not (last.get("type") == "movement_condition"
            and last.get("movement") == "baton_touch"):
        return result
    fixed = False
    new_conds = []
    for leg in conds[:-1]:
        if (leg.get("type") == "location_condition"
                and "このメンバー" in leg.get("text", "")
                and leg.get("trigger_event") is None):
            new_conds.append({
                "type": "movement_condition",
                "movement": "baton_touch",
                "target": leg.get("target", "self"),
                "baton_touch_trigger": True,
                "text": text,
                "trigger_event": {
                    "type": "baton_touch",
                    "tense": "past",
                    "location": "stage",
                },
            })
            fixed = True
        else:
            new_conds.append(leg)
    if not fixed:
        return result
    new_conds.append(last)
    result = dict(result)
    result["conditions"] = new_conds
    return result


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
    Also handles G17: "エネルギーが置かれたとき" (energy placed into the zone,
    past-tense, no から/source) — an event-based auto trigger that must fire
    only on the actual placement. e.g. "自分のカードの効果によって、自分の
    エネルギー置き場にエネルギーが置かれたとき". Past-tense 置かれた only:
    "置かれるたび" (each_time, hazuki PL!SP-bp4-016-N) stays a comparison_condition
    gated by the each_time scan gate, and "から...に置かれた" (zone_change) is
    handled by _try_zone_placement.
    Sets trigger_event.watches_area_move=True when the text also watches an
    area move (absent otherwise — the normalizer strips False).
    """
    # G17: energy placed INTO the zone, past-tense, no source zone.
    is_energy_placed = (
        "エネルギー" in text and "置かれた" in text and "から" not in text
    )
    if not (
        is_energy_placed
        or "移動した" in text
        or "移動している" in text
        or "移動する" in text
    ):
        return None
    tense = (
        "past"
        if (is_energy_placed or "移動した" in text or "移動している" in text)
        else "nonpast"
    )
    te_data: Dict[str, Any] = {
        "type": "energy_placed" if is_energy_placed else "area_move",
        "tense": tense,
    }
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
        if (is_future or is_energy_placed)
        else None,  # placeholder, set after generic fields
    }
    _extract_generic_fields(result, text)
    # Override movement: _extract_generic_fields may set "moved" from "置かれた"
    # patterns, but _try_movement handles area moves / energy placement, not
    # generic zone placements. Energy-placed and future area moves use "moves".
    result["movement"] = (
        "moves" if (is_future or is_energy_placed) else "position_change"
    )
    # Extract position (センター/左サイド/右サイド) and direction.
    # "センターエリアにいるメンバーがエリアを移動" → position=center, area_direction=from
    # "メンバーがセンターエリアに移動"               → position=center, area_direction=to
    if "position" not in result:
        _m = detect_position_matches(text)
        if _m:
            kw, pos = _m[0]
            result["position"] = pos
            if f"{kw}にいる" in text or f"{kw}にいて" in text:
                result["area_direction"] = "from"
        matched = {pos for _, pos in _m}
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
    # Whether the text ALSO watches an area move (〜エリアを移動 / 移動した…).
    # Pure energy-placement texts (e.g. Ren bp7-005 ab#1:
    # "エネルギー置き場にエネルギーが置かれたとき") carry no move language,
    # while 〜か compounds (e.g. Sumire bp5-004 ab#0:
    # "このメンバーがエリアを移動するか…" ) do. The engine uses this to
    # decide whether the area-move disjunct applies; without it every S1
    # energy watcher also fires on its own area moves. True is persisted;
    # False is dropped by the pipeline normalizer, so absent means False.
    if re.search(r"エリアを移動|移動した|移動している|移動する", text):
        te_data["watches_area_move"] = True
    # Extract "エネルギーが置かれ" (energy placed) trigger. The placement verb
    # may be separated from エネルギー by a location phrase ("エネルギーが...メンバーの下に置かれた"),
    # so match on the energy subject + the 置かれ/置かれた placement verb.
    if re.search(r"エネルギーが.*?(置かれ|置かれた)", text):
        te_data["energy_placed"] = True
        # "エネルギーが...メンバーの下に置かれた" → placed UNDER a member.
        if "メンバーの下に" in text:
            te_data["destination"] = "under_member"
            result["destination"] = "under_member"
    # Phase restriction: 自分のメインフェイズ / 相手のメインフェイズ
    _apply_phase_restriction(text, "main", te_data)
    return result


# 「AからBに…」 — the verbs a zone-change clause ends with.
_ZONE_CHANGE_VERBS = "置かれ|加えられ|加わる|移され|送られ"
# Same clause, with and without the destination captured. The bare form is the
# gate (it also matches when nothing sits between から and に) and the
# capturing form names the zone.
_ZONE_CHANGE_RE = re.compile(rf"から(.+?)に(?:{_ZONE_CHANGE_VERBS})")
_ZONE_CHANGE_PRESENT_RE = re.compile(rf"から.*?に(?:{_ZONE_CHANGE_VERBS})")

# Which zone a movement clause names, per direction. Both lists are ordered
# and first-match-wins because the phrases overlap — 「エネルギーデッキ」 contains
# 「デッキ」, 「エネルギー置き場」 contains 「置き場」 — and the overlap that has
# to win differs by direction: a source reads more specific energy zones
# first, a destination lists the bare zones first.
_ZONE_FROM_PHRASES = (
    (("控え室",), "discard"),
    (("ライブカード置き場",), "live_card_zone"),
    (("エネルギーデッキ",), "energy_deck"),
    (("エネルギー置き場",), "energy_zone"),
    (("デッキ",), "deck"),
    (("手札", "手元"), "hand"),
    (("ステージ",), "stage"),
)
_ZONE_TO_PHRASES = (
    (("控え室",), "discard"),
    (("手札", "手元"), "hand"),
    (("ステージ",), "stage"),
    (("エネルギーデッキ",), "energy_deck"),
    (("デッキ",), "deck"),
)


def _zone_named(fragment, phrases):
    """The zone `fragment` names, or None when it names none."""
    for keywords, zone in phrases:
        if any(keyword in fragment for keyword in keywords):
            return zone
    return None


def _try_zone_placement(text):
    """Zone change event: card moves from one zone to another.
    E.g. このカードが控え室から手札に加えられた (this card added from discard to hand).

    Sets flat `source`/`destination` fields on the condition for the engine's
    `evaluate_card_count_condition` routing, plus keeps `trigger_event` for
    documentary use by `movement_condition` and phase-gate evaluation.
    """
    if not _ZONE_CHANGE_PRESENT_RE.search(text) or "バトンタッチ" in text:
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
    # The FROM zone is whatever precedes から; the TO zone is what sits between
    # から and the placement verb. Both are mirrored onto `trigger_event`.
    from_at = text.find("から")
    src = _zone_named(text[:from_at], _ZONE_FROM_PHRASES) if from_at >= 0 else None
    if src:
        result["source"] = src
        result["trigger_event"]["source"] = src
    dest_match = _ZONE_CHANGE_RE.search(text)
    dest = _zone_named(dest_match.group(1), _ZONE_TO_PHRASES) if dest_match else None
    if dest:
        result["destination"] = dest
        result["trigger_event"]["destination"] = dest
    # Phase restriction: 自分のメインフェイズに / 相手のメインフェイズに
    _apply_phase_restriction(text, "main", result["trigger_event"])
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
        "operator": "or",
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
        "operator": "or",
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
        quoted = _quoted_names(text[: text.find("登場")])
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
        # (comparison dimension is always cost — no type field emitted)
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
        all_quoted = _quoted_names(text)
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
    # Exclude self
    if detect_exclude_self(text) or "このカード以外" in text:
        result["exclude_self"] = True
    gns = extract_all_groups(text)
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
            result["min_baton_touch_count"] = as_int(count_m.group(1))
        # 「バトンタッチして登場していないかぎり、…する」 gates the action on
        # the event NOT having happened — encode the polarity explicitly.
        if re.search(r"バトンタッチ[^。]*?していない", text):
            result["negation"] = True
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


def _set_energy_state_condition(text, result):
    result["type"] = "energy_state_condition"
    if "エネルギーがない" in text:
        result["negation"] = True
    if "アクティブ状態" in text:
        result["energy_state"] = "active"


def _matches_energy_state_condition(text):
    return "エネルギーがある" in text or "エネルギーがない" in text


_try_energy_state = ConditionPattern(
    condition=_matches_energy_state_condition,
    setter=_set_energy_state_condition,
)


def _set_state_condition(text, result):
    state = None
    for patterns, candidate in (
        (["ウェイト状態である", "ウェイト状態にある", "ウェイト状態の"], "wait"),
        (
            ["アクティブ状態である", "アクティブ状態にある", "アクティブ状態の"],
            "active",
        ),
    ):
        if any(pattern in text for pattern in patterns):
            state = candidate
            break
    if state is None:
        return
    result["type"] = "state_condition"
    result["state"] = state
    if state == "active" and "エネルギー" in text:
        result["resource_type"] = "energy"
    if "すべて" in text:
        result["all"] = True
    target = extract_target(text)
    if target:
        result["target"] = target
    location = extract_location(text)
    if location:
        result["location"] = location
    elif "ステージ" in text:
        result["location"] = "stage"
    card_type = extract_card_type(text)
    if card_type:
        result["card_type"] = card_type
    elif "メンバー" in text:
        result["card_type"] = "member_card"
    if "いる場合" in text or "いる" in text:
        result["count"] = 1
        result["operator"] = ">="
    groups = extract_all_groups(text)
    if groups:
        result["group_names"] = groups
    if re.search(r"この(メンバー|カード)[がは]", text) and "以外" not in text:
        result["self_target"] = True


def _matches_state_condition(text):
    return any(
        pattern in text
        for pattern in (
            "ウェイト状態である",
            "ウェイト状態にある",
            "ウェイト状態の",
            "アクティブ状態である",
            "アクティブ状態にある",
            "アクティブ状態の",
        )
    )


_try_state = ConditionPattern(
    condition=_matches_state_condition,
    setter=_set_state_condition,
)


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
    if HAS_SCORE_ICON in text:
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
    ec = text.count(ENERGY_ICON)
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
        if "センターにいる" in text or "センターにある" in text:
            action["source_position"] = "center"
            action.pop("position", None)
        else:
            action["position"] = "center"

    # Rule 11.10.1: Position change MUST move to a different area.
    # Exclude the member's current position for single-target moves.
    # NOT for "メンバー1人を" (where the player selects a target first)
    # and NOT for formation change ("それぞれ" / multiple_targets).
    if (
        "それぞれ" not in text
        and "source_position" not in action
        and not ("メンバー" in text and ("1人" in text or "N人" in text))
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
    if detect_positions(text):
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
        # Capture the JA alt-text (icon label), NOT the filename: the engine
        # matches these against stored trigger strings ("ライブ開始時"...).
        triggers = re.findall(r"\{\{\w+\.png\|([^}]+)\}\}能力も", text)
        if triggers:
            result["ability_filter_triggers"] = triggers
    if "ライブ" in text:
        # 「自分のライブ中のライブカードに…」 — the scanned zone is the
        # live-card zone, not the stage default.
        result["location"] = "live_card_zone"
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

    # Extract "自分のカードの効果" (own card effect) constraint, e.g. Maki
    # pb1-015 ab#1: 自分のカードの効果によって、相手のステージにいる...
    # メンバーがウェイト状態になったとき. Absent means False (the pipeline
    # normalizer strips False).
    if "自分のカードの効果" in text:
        result["trigger_event"]["self_effect_only"] = True

    _apply_phase_restriction(text, "main", result["trigger_event"])

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
            result["count"] = as_int(m.group(1))
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
    if text.strip().startswith("それが") and not extract_location(text):
        result.pop("location")
        result.update(
            type="card_count_condition",
            source="preceding_moved",
            count=1,
            operator=">=",
        )
    if "持たない" in text:
        result["negation"] = True
    if "icon_all" in text:
        result["heart_type"] = "all"
    # "元々持つハートの数より多い" → compare current hearts > base hearts per member
    if "元々" in text and "ハート" in text and "より多い" in text:
        result["original_value"] = True
        result["operator"] = ">"
        result["count"] = 1
    # Exclude self
    if detect_exclude_self(text) or "このカード以外" in text:
        result["exclude_self"] = True
    return result


def _try_live_mid(text):
    if "ライブ中" not in text:
        return None
    result = {"text": text}
    score_match = re.search(r"スコア([0-9０-９]+)以下のライブカード", text)
    count_match = re.search(r"([0-9０-９]+)枚以上", text)
    if score_match:
        result["type"] = "card_count_condition"
        result["count"] = 1
        result["operator"] = ">="
        result["card_type"] = "live_card"
        result["location"] = "live_card_zone"
        result["cost_limit"] = as_int(score_match.group(1))
        result["cost_limit_operator"] = "<="
    elif count_match:
        result["type"] = "card_count_condition"
        result["count"] = as_int(count_match.group(1))
        result["operator"] = ">="
        result["card_type"] = "live_card"
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
    ConditionPattern("this_turn_opponent_live_success", 1, _try_this_turn_opponent_live_success),
    ConditionPattern("complex", 1, _try_complex),
    ConditionPattern("character_each", 1, _try_character_each),
    ConditionPattern("compound", 1, _try_compound),
    ConditionPattern("distinct", 1, _try_distinct),
    ConditionPattern("dual_distinct", 1, _try_dual_distinct),
    ConditionPattern("state_change", 2, _try_state_change),
    ConditionPattern("or", 2, _try_or),
    ConditionPattern("blade_count", 2, _try_blade_count),
    ConditionPattern("card_count", 3, _try_card_count),
    ConditionPattern("highest_cost_on_stage", 3, _try_highest_cost_on_stage),
    ConditionPattern("cost_override_condition", 3, _try_cost_override_condition),
    ConditionPattern("both", 3, _try_both),
    ConditionPattern("temporal_this_turn", 4, _try_temporal_this_turn),
    ConditionPattern("phase_gate", 4, _try_phase_gate),
    ConditionPattern("temporal_turn_phase", 4, _try_temporal_turn_phase),
    ConditionPattern("baton_touch", 4, _try_baton_touch),
    ConditionPattern("temporal_count", 4, _try_temporal_count),
    ConditionPattern("either_target", 5, _try_either_target),
    ConditionPattern("live_success_or_move", 5, _try_live_success_or_move),
    ConditionPattern("appear_or_move", 5, _try_appear_or_move),
    ConditionPattern("movement", 5, _try_movement),
    ConditionPattern("appearance", 5, _try_appearance),
    ConditionPattern("zone_placement", 5, _try_zone_placement),
    ConditionPattern("energy_state", 6, _try_energy_state),
    ConditionPattern("state", 6, _try_state),
    ConditionPattern("revealed", 6, _try_revealed),
    ConditionPattern("opponent_choice", 6, _try_opponent_choice),
    ConditionPattern("unless_pay", 6, _try_unless_pay),
    ConditionPattern("position_change", 7, _try_position_change),
    ConditionPattern("position", 7, _try_position),
    ConditionPattern("ability_filter", 7, _try_ability_filter),
    ConditionPattern("otherwise", 7, _try_otherwise),
    ConditionPattern("heart_possession", 7, _try_heart_possession),
    ConditionPattern("live_mid", 7, _try_live_mid),
]

_condition_registry = PriorityRegistry("condition_rules")
for _ci, _rule in enumerate(CONDITION_PATTERNS):
    _condition_registry.register(_rule.tier * 100 + _ci, _rule.name, _rule)


def _try_placed_discard_live_or_member(text):
    """B5 近江彼方 ab#1: "…控え室に置いたカードの中に『X』のライブカードか
    ブレードハートを持たない『X』のメンバーカードがある場合".

    Among the cards just placed into the discard by the preceding cost, is there
    a group-X LIVE card OR a group-X member card without a blade heart?  Emits an
    or_condition of two card_count_conditions scoped to source:"preceding_moved".
    """
    if "の中に" not in text or "ライブカードか" not in text or "持たない" not in text:
        return None
    if not ("控え室に置いた" in text or "置いたカード" in text):
        return None
    # Group name(s) from 『』 (e.g. 『虹ヶ咲』).
    groups = deduped_groups(text)
    if not groups:
        return None
    # Only fire when the live-card alternative and the blade-heartless member
    # alternative both refer to the same group clause (the target pattern).
    if "メンバーカード" not in text:
        return None
    leg_live = {
        "type": "card_count_condition",
        "source": "preceding_moved",
        "card_type": "live_card",
        "group_names": groups,
        "count": 1,
        "operator": ">=",
    }
    leg_member = {
        "type": "card_count_condition",
        "source": "preceding_moved",
        "card_type": "member_card",
        "card_property": "has_blade_heart",
        "negation": True,
        "group_names": groups,
        "count": 1,
        "operator": ">=",
    }
    return {
        "type": "or_condition",
        "operator": "or",
        "source": "preceding_moved",
        "conditions": [leg_live, leg_member],
        "text": text,
    }


_condition_registry.register(
    1,
    "placed_discard_live_or_member",
    ConditionPattern(handler=_try_placed_discard_live_or_member),
)


def _try_discard_live_and_member_optional(text):
    """B6 Cooking with Love ab#0:
    控え室に『虹ヶ咲』のライブカードと、ブレードハートを持たない『虹ヶ咲』の
    メンバーカードがある場合、自分の控え室にあるすべてのカードをシャッフルし、
    デッキの下に置いてもよい。そうしたとき、ライブ終了時まで、自分のステージに
    いるすべての『虹ヶ咲』のメンバーはheart01を得る。

    AND-condition (live card present AND member-without-blade-heart present in
    DISCARD) gates a してもよい shuffle-to-deck-bottom; on acceptance all group
    members on stage gain heart01. Returns a conditional_on_optional whose
    conditional_action (the "accepted" branch) is the sequential [shuffle, heart].
    """
    if "控え室" not in text or "ライブカードと" not in text or "持たない" not in text:
        return None
    if "メンバーカードがある場合" not in text or "デッキの下に置いてもよい" not in text:
        return None
    if "そうしたとき" not in text:
        return None
    groups = deduped_groups(text)
    if not groups:
        return None

    cond_live = {
        "type": "card_count_condition",
        "card_type": "live_card",
        "location": "discard",
        "group_names": groups,
        "count": 1,
        "operator": ">=",
        "target": "self",
        "text": "控え室にグループのライブカードがある場合",
    }
    cond_member = {
        "type": "card_count_condition",
        "card_type": "member_card",
        "location": "discard",
        "card_property": "has_blade_heart",
        "negation": True,
        "group_names": groups,
        "count": 1,
        "operator": ">=",
        "target": "self",
        "text": "ブレードハートを持たないグループのメンバーカードがある場合",
    }
    condition = {
        "type": "compound",
        "operator": "and",
        "conditions": [cond_live, cond_member],
        "text": "控え室にグループのライブカードとブレードハートを持たないグループのメンバーカードがある場合",
    }

    shuffle = {
        "action": "move_cards",
        "source": "discard",
        "destination": "deck_bottom",
        "all": True,
        "shuffle": True,
        "target": "self",
        "text": "自分の控え室にあるすべてのカードをシャッフルし、デッキの下に置いてもよい",
    }
    heart = {
        "action": "gain_resource",
        "resource": "heart",
        "heart_colors": ["heart01"],
        "card_type": "member",
        "all": True,
        "duration": "live_end",
        "group_names": groups,
        "target": "self",
        "text": "ライブ終了時まで、自分のステージにいるすべてのグループのメンバーはheart01を得る",
    }
    return make_conditional_on_optional(
        text,
        shuffle,
        {"action": "sequential", "actions": [shuffle, heart]},
        condition=condition,
    )


def make_conditional_on_optional(
    text, optional_action, conditional_action, *, negation=False, **extra
):
    """Single constructor for every してもよい/そうしたとき-style container.

    The optionality belongs to the CONTAINER (the player's may-I choice);
    sub-actions keep their own fields untouched. Extra context (condition,
    etc.) rides in via **extra so all builders share one shape contract.
    """
    node = {
        "text": text,
        "action": "conditional_on_optional",
        "optional_action": optional_action,
        "conditional_action": conditional_action,
    }
    if negation:
        node["conditional_negation"] = True
    node.update(extra)
    return node


# --- the shared shape of every 「〜もよい。そうしたとき、〜」 offer ----------
_OPTIONAL_CONSEQUENCE_MARKER = "そうしたとき"


def _split_optional_offer(text):
    """Split 「<optional clause>そうしたとき、<consequence>」.

    Returns (optional_text, consequence_text), or None when `text` is not an
    optional offer. The leading 「、」 on the consequence is dropped here so no
    handler has to remember to strip it.
    """
    if _OPTIONAL_CONSEQUENCE_MARKER not in text:
        return None
    opt_text, _, cons_text = text.partition(_OPTIONAL_CONSEQUENCE_MARKER)
    return opt_text.strip(), cons_text.strip().lstrip("、")


def _count_from_cards(text, default=1):
    """The 「N枚」 count in `text`, or `default` when the count is unstated."""
    m = re.search(r"(\d+)枚", text)
    return as_int(m.group(1)) if m else default


def _committed(action):
    """`action` with the optional flag cleared — what runs on acceptance."""
    done = dict(action)
    done["optional"] = False
    return done


def _on_accept(optional_action, consequence):
    """The sequential that runs when the player accepts the optional offer."""
    return {
        "action": "sequential",
        "actions": [_committed(optional_action), consequence],
    }


# 「控え室からこのカードを手札に加える」 — the recovery an optional discard buys.
_RECOVER_SELF_FROM_DISCARD = {
    "action": "move_cards",
    "source": "discard",
    "destination": "hand",
    "count": 1,
    "card_type": "card",
    "self_target": True,
}


def _try_those_cards_add_hand_optional(text):
    """G13: 'それらのカードの中から『X』のライブカードをN枚手札に加えてもよい。
    そうしたとき、[consequence]' → conditional_on_optional. Accepting runs the
    move-to-hand AND the consequence."""
    if "それらのカードの中から" not in text or "手札に加えてもよい" not in text:
        return None
    split = _split_optional_offer(text)
    if split is None:
        return None
    opt_text, cons_text = split
    groups = deduped_groups(opt_text)
    move_opt = {
        "action": "move_cards",
        "source": "those_cards",
        "destination": "hand",
        "card_type": "live_card" if "ライブカード" in opt_text else "card",
        "count": _count_from_cards(opt_text),
        "target": "self",
        "optional": True,
        "text": opt_text,
    }
    if groups:
        move_opt["group_names"] = groups
    return make_conditional_on_optional(
        text,
        move_opt,
        _on_accept(move_opt, parse_effect(cons_text)),
    )


def _try_discard_shuffle_to_bottom_optional(text):
    """G16 / 澁谷かのん: '自分の控え室にある…メンバーカードをN枚選び、それらを
    シャッフルし／好きな順番で、デッキの(一番)下に置いてもよい。そうしたとき、
    [consequence]' → conditional_on_optional{optional: move discard→deck_bottom
    (shuffle / any order), conditional: consequence}."""
    if "選び" not in text:
        return None
    if "シャッフル" not in text and "好きな順番で" not in text:
        return None
    if "デッキの下に置いてもよい" not in text and "デッキの一番下に置いてもよい" not in text:
        return None
    split = _split_optional_offer(text)
    if split is None:
        return None
    opt_text, cons_text = split
    groups = deduped_groups(opt_text)
    # "それぞれ1枚ずつ" = 1 from EACH group → the total count equals the number of groups.
    per_each = "それぞれ" in opt_text or "ずつ" in opt_text
    count = len(groups) if (per_each and groups) else _count_from_cards(opt_text)
    card_type = "member_card" if "メンバーカード" in opt_text else "card"
    move_opt = {
        "action": "move_cards",
        "source": "discard",
        "destination": "deck_bottom",
        "count": count,
        "card_type": card_type,
        "target": "self",
        "placement_order": "any_order",
        "optional": True,
        "text": opt_text,
    }
    if per_each:
        move_opt["per_group"] = True
    if "シャッフル" in opt_text:
        move_opt["shuffle"] = True
    if groups:
        move_opt["group_names"] = groups

    cons = parse_effect(cons_text)
    # The consequence (e.g. "カードを1枚引く") must NOT inherit the selection's
    # group_names filter.
    if isinstance(cons, dict):
        cons.pop("group_names", None)

    # "それぞれ1枚ずつ" of several groups = one selection PER group. Model it as a
    # sequential of single-group moves so the engine's normal single-group select
    # handles each group independently (no per-group engine plumbing needed).
    if per_each and groups:
        seq_actions = [
            {
                "action": "move_cards",
                "source": "discard",
                "destination": "deck_bottom",
                "count": 1,
                "card_type": card_type,
                "target": "self",
                "placement_order": "any_order",
                "optional": True,
                "group_names": [g],
                "text": opt_text,
                **({"shuffle": True} if "シャッフル" in opt_text else {}),
            }
            for g in groups
        ]
        return make_conditional_on_optional(
            text,
            {"action": "sequential", "actions": seq_actions},
            {"action": "sequential", "actions": seq_actions + [cons]},
        )

    return make_conditional_on_optional(
        text,
        move_opt,
        _on_accept(move_opt, cons),
    )


def _try_discard_hand_recover_self_optional(text):
    """G7 (ミア・テイラー ab#0): '手札をN枚控え室に置いてもよい。そうしたとき、
    控え室からこのカードを手札に加える' → conditional_on_optional.
    optional_action: discard N cards from hand (may). conditional_action (on
    accept): recover THIS card (self_target) from discard back to hand. The
    recover is gated on the optional actually being performed."""
    if "手札を" not in text or "控え室に置いてもよい" not in text:
        return None
    if "手札に加える" not in text:
        return None
    if "控え室から" not in text and "控え室にある" not in text:
        return None
    split = _split_optional_offer(text)
    if split is None:
        return None
    opt_text, cons_text = split
    discard_opt = {
        "action": "move_cards",
        "source": "hand",
        "destination": "discard",
        "count": _count_from_cards(opt_text),
        "card_type": "card",
        "target": "self",
        "optional": True,
        "text": opt_text,
    }
    cons = parse_action(cons_text)
    if not isinstance(cons, dict) or cons.get("action") != "move_cards":
        cons = parse_effect(cons_text)
    # The consequence recovers THIS card out of the discard the optional just
    # created. A parse that already produced a move gets redirected; anything
    # else is replaced with the explicit recovery.
    # The consequence recovers THIS card out of the discard the optional just
    # created. A parse that already produced a move gets redirected (keeping
    # whatever count it inferred); anything else is replaced outright.
    if isinstance(cons, dict) and cons.get("action") == "move_cards":
        cons.update(_RECOVER_SELF_FROM_DISCARD)
        cons.setdefault("count", 1)
    else:
        cons = dict(_RECOVER_SELF_FROM_DISCARD, text=cons_text)
    return make_conditional_on_optional(
        text,
        discard_opt,
        _on_accept(discard_opt, cons),
    )


def _try_discard_hand_reactivate_optional(text):
    """Shioriko bp7-022 ab#0: '<trigger>になったとき、手札をN枚控え室に置いても
    よい。そうしたとき、そのメンバーをアクティブにする' → sequential
    conditional:true with the trigger as its condition.
    optional: discard N from hand (may). On accept: activate the waited
    trigger member (Niji group hardcoded per the card-specific precedent of
    G7/G13/G16 — the group lives in the trigger clause, not this substring).
    Without this, the generic conditional path misreads the discard as a
    comparison condition and the reactivate as unconditional (free activate
    with no discard — the bp7-022 bug)."""
    if "手札を" not in text or "控え室に置いてもよい" not in text:
        return None
    if "そうしたとき" not in text:
        return None
    pre, _, post = text.partition("そうしたとき")
    # The optional clause starts at 手札を; anything before it is trigger.
    hi = pre.find("手札を")
    if hi < 0:
        return None
    trig_text = pre[:hi].strip().rstrip("、")
    opt_text = pre[hi:].strip()
    cons_text = post.strip().lstrip("、")
    if "そのメンバーをアクティブにする" not in cons_text:
        return None
    # Trigger is the "<group-wait>になったとき" clause (any phase gate was
    # stripped before parse_effect and is merged back by parse_ability).
    # It must be the group wait-watcher; anything else falls through.
    # Rebuilt as a state_change_condition (active→wait on the group member):
    # a bare group_condition would only check group PRESENCE on stage and
    # fire for any wait event regardless of who waited (the bp7-022 trigger
    # bug). Mirrors pb1-015's state_change shape.
    trig_text = trig_text.strip()
    if trig_text.endswith("になったとき"):
        trig_text = trig_text[: -len("になったとき")] + "になったとき"
    group_leg = parse_condition(trig_text)
    if (not isinstance(group_leg, dict)
            or group_leg.get("type") != "group_condition"
            or not group_leg.get("group_names")):
        return None
    trigger_cond = dict(group_leg)
    trigger_cond["type"] = "state_change_condition"
    trigger_cond["from_state"] = "active"
    trigger_cond["to_state"] = "wait"
    trigger_cond["trigger_event"] = {
        "type": "state_change",
        "from_state": "active",
        "to_state": "wait",
    }
    m = re.search(r"(\d+)枚", opt_text)
    count = as_int(m.group(1)) if m else 1
    discard_opt = {
        "action": "move_cards",
        "source": "hand",
        "destination": "discard",
        "count": count,
        "card_type": "card",
        "target": "self",
        "optional": True,
        "text": opt_text,
    }
    cons = {
        "action": "change_state",
        "state_change": "active",
        "count": 1,
        "card_type": "member_card",
        "group_names": ["虹ヶ咲"],
        "target": "self",
        "text": cons_text,
    }
    return {
        "text": text,
        "action": "sequential",
        "conditional": True,
        "condition": trigger_cond,
        "actions": [discard_opt, cons],
    }


# 「N枚より」 — a plain numeric threshold, not a comparison against a noun.
_NUMERIC_MORE_RE = re.compile(r"\d+[枚人個種類つ]?より")


def _contiguous_comparison_target(text):
    """The comparison target named literally in `text`, or None."""
    for phrase, target in COMPARISON_TARGETS.items():
        if phrase in text:
            return target
    return None


def _noun_comparison_target(text, reject_numeric=False):
    """The comparison target from a 「<noun>より」 phrasing, or None.

    `reject_numeric` skips 「N枚より」-style thresholds, which are a number
    rather than another thing being compared against.
    """
    for phrase, target in COMPARISON_TARGETS.items():
        if not (phrase.endswith("より") and len(phrase) >= 4):
            continue
        noun = phrase[:-2]
        if noun not in text or "より" not in text:
            continue
        noun_pos = text.find(noun)
        if text.find("より", noun_pos + len(noun)) <= noun_pos:
            continue
        if reject_numeric and _NUMERIC_MORE_RE.search(text):
            continue
        return target
    return None


def _extract_comparison_fields(condition, text):
    """Extract comparison_target, comparison_type, aggregate, operator from text."""
    target = _contiguous_comparison_target(text) or _noun_comparison_target(
        text, reject_numeric=True
    )
    if target:
        condition["comparison_target"] = target
    # The comparison already covers the opponent side, so a "both" target would
    # double-count it.
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
    if condition.get("comparison_type") == "score":
        _extract_score_threshold(condition, text)
    if "合計" in text:
        condition["aggregate"] = "total"
    if "自分と相手の" in text and "合計" in text and "同じ" in text:
        condition["target"] = "self"
        condition["comparison_target"] = "opponent"
        if "成功ライブカード" in text:
            condition["location"] = "success_live_zone"
        elif "ライブカード" in text or "ライブ" in text:
            condition["location"] = "live_card_zone"
        if "スコア" in text:
            condition["comparison_type"] = "score"
            condition["resource_type"] = "score"
        elif "コスト" in text:
            condition["resource_type"] = "cost"
    if "ちょうど" in text or "同じ" in text:
        condition["operator"] = "="
        if "同じ" in text and condition.get("comparison_type") != "score":
            condition["comparison_type"] = "equality"
            condition["type"] = "comparison_condition"


def _extract_score_threshold(condition, text):
    """Read 「スコアがN」 as the compared count, unless it is only an option.

    「スコアが3か5の場合」 lists alternatives rather than setting the threshold,
    so the digits there are not the count to compare against.
    """
    score_text = normalize_fullwidth_digits(text)
    match = re.search(r"スコア(?:が|は)\s*(\d+)", score_text)
    if not match:
        return
    if re.match(r"\s*(?:か|または|や|のいずれか)", score_text[match.end() :]):
        return
    condition["count"] = as_int(match.group(1))
    condition.setdefault("operator", "=")


def _extract_heart_resource(condition, text):
    if not ("heart" in text and (
        "つ以上持つ" in text or "枚持つ" in text or "つ持つ" in text
    )):
        return
    hc = extract_count(text)
    if hc:
        condition["count"] = hc
        if re.search(rf"{HEART_HAS_REF}.*?{HEART_HAS_REF}", text):
            hts = []
            for i in range(1, 7):
                if f"heart_0{i}" in text:
                    hts.append(f"heart_0{i}")
            if hts:
                condition["resource_type"] = "heart"
                condition["heart_types"] = hts
                tm = re.search(r"合計(\d+)種類以上", text)
                if tm:
                    condition["types_count"] = as_int(tm.group(1))
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


def _extract_energy_resource(condition, text):
    if "エネルギー" in text:
        condition["resource_type"] = "energy"
        ec = extract_count(text)
        if ec:
            condition["count"] = ec


def _extract_surplus_resource(condition, text):
    if "余剰ハート" in text:
        condition["resource_type"] = "surplus_heart"
        sc = extract_count(text)
        if sc:
            condition["count"] = sc


def _extract_resource_fields(condition, text):
    """Extract heart count, heart_colors, energy, surplus_heart from text."""
    _extract_heart_resource(condition, text)
    hc = extract_heart_colors_from_text(text)
    if hc:
        condition["heart_colors"] = hc
    _extract_energy_resource(condition, text)
    _extract_surplus_resource(condition, text)


# Ordered movement markers for conditions: first match wins.
# Same order as the legacy if-chain.
_CONDITION_MOVEMENT_RULES = (
    ("移動した", "moved"),
    ("移動する", "moves"),
)

# Ordered distinct markers for conditions: first match wins.
# Same order as the legacy if-chain.
_DISTINCT_MARKER_RULES = (
    (("コストがそれぞれ異なる",), "cost"),
    (("名前が異なる", "名前の異なる", "カード名が異なる"), "card_name"),
    (("グループ名が異なる", "グループ名がそれぞれ異なる"), "group_name"),
)


def _extract_character_names(condition, text):
    # Character names: 「A」か「B」か「C」がいる, 「A」と「B」がいる,
    # 「A」、「B」、「C」のうち (any number of names with か/と/、 separators)
    cm = re.search(r"((?:「[^」]+」[とか、]? ?)+)(?:」?がいる|」?のうち)", text)
    if cm:
        names = _quoted_names(cm.group(1))
        if names:
            condition["characters"] = names
    else:
        # 「A」か「B」の場合 / 「A」か「B」を...した場合 (character-name condition
        # in a conditional / result-condition phrase, e.g. "それが「松浦果南」か
        # 「黒澤ダイヤ」の場合" or "これによって「津島善子」か「黒澤ルビィ」を
        # 手札に加えた場合"). Any bracketed names in a condition context are
        # character filters for the counted/referenced card.
        cm2 = re.search(r"「([^」]+)」\s*(?:か|と|、)\s*「([^」]+)」", text)
        if cm2:
            condition["characters"] = list(cm2.groups())
            more = re.search(
                r"「([^」]+)」\s*(?:か|と|、)\s*「([^」]+)」\s*(?:か|と|、)?\s*「([^」]+)」",
                text,
            )
            if more:
                condition["characters"] = list(more.groups())


def _extract_condition_card_names(condition, text):
    # Card name filter: カード名に「DreamBelievers」を含む
    cn = re.search(r"カード名に「([^」]+)」を含む", text)
    if cn:
        condition["card_names"] = [cn.group(1)]

    # Card name exact: カード名が「EMOTION」のカード
    cn_exact = re.search(r"カード名が「([^」]+)」のカード", text)
    if cn_exact:
        condition["card_names"] = [cn_exact.group(1)]


def _extract_target_location(condition, text):
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

    # Under-member scope: "…の下に置かれているかぎり" / "…の下に置かれている"
    # (e.g. 澁谷かのん ab#0 「このカードが『Liella!』のメンバーの下に置かれて
    # いるかぎり」). The subject card being under a member is an `under_member`
    # location, not expressible via the zone-name LOCATION_PATTERNS above.
    if condition.get("location") is None and (
        "の下に置かれている" in text or "の下に置かれていて" in text
    ):
        condition["location"] = "under_member"


def _extract_ability_filter(condition, text):
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


def _extract_self_markers(condition, text):
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


def _extract_movement_marker(condition, text):
    # Movement
    for phrase, movement in _CONDITION_MOVEMENT_RULES:
        if phrase in text:
            condition["movement"] = movement
            break
    # "置かれた" (was placed) — single-location self_target triggers
    # (multi-location "置かれた" like discard-from-stage use the engine's
    # 2-locations mechanism and should NOT get movement:"moved")
    if "置かれた" in text and "locations" not in condition:
        condition["movement"] = "moved"


def _extract_distinct_marker(condition, text):
    # Distinct flags
    for phrases, distinct in _DISTINCT_MARKER_RULES:
        if any(kw in text for kw in phrases):
            condition["distinct"] = distinct
            return


def _extract_group_scope(condition, text):
    # Group
    gns = extract_all_groups(text)
    if gns:
        condition["group_names"] = gns
    # Exclude group (以外)
    apply_group_exclusions(condition, text)
    # Detect "のみ" (only/all members must match the group)
    if gns:
        if "のみの場合" in text or (
            "のみ" in text
            and ("ステージ" in text or "メンバー" in text or "カードが" in text)
        ):
            condition["all_members"] = True


def _extract_generic_fields(condition, text):
    """Extract all generic fields from text into condition dict (no early return)."""
    _extract_character_names(condition, text)
    _extract_condition_card_names(condition, text)

    # Group/unit names: 『虹ヶ咲』 etc. (reconciled with exclusions later in
    # _extract_group_scope; this early pass feeds the middle blocks below)
    gns = extract_all_groups(text)
    if gns:
        condition["group_names"] = gns

    _extract_target_location(condition, text)
    _extract_ability_filter(condition, text)

    # Delta tracking for surplus heart loss ("これにより失っている場合")
    if "失っている" in text and "これにより" in text:
        condition["delta"] = True

    _extract_resource_fields(condition, text)

    # Card type, count, operator
    ct = extract_card_type(text)
    # Self-appearance ("このメンバーが登場"/"このカードが登場") card_type is
    # stripped centrally by _strip_self_appearance_card_type in
    # process_abilities — do not add per-site guards here.
    if ct:
        condition["card_type"] = ct
    cnt = extract_count(text)
    if cnt is not None:
        condition["count"] = cnt
    op = extract_operator(text)
    if op:
        condition["operator"] = op

    _extract_comparison_fields(condition, text)

    # Negation (〜がない / 〜がなく / 〜が〜ない / 〜いない / 〜を持たない)
    if (
        re.search(r"がない", text)
        or re.search(r"がなく", text)
        or re.search(r"が\d*ない", text)
        or "いない" in text
        or "を持たない" in text
    ):
        condition["negation"] = True

    _extract_self_markers(condition, text)
    _extract_movement_marker(condition, text)

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
    _extract_distinct_marker(condition, text)

    # All areas
    if "エリアすべて" in text:
        condition["all_areas"] = True

    # Exclude self
    if detect_exclude_self(text) or "このカード以外" in text:
        condition["exclude_self"] = True

    # Exclude specific card names (e.g. 「MY舞☆TONIGHT」以外)
    quoted_exclusions = re.findall(r"「([^」]+)」以外", text)
    if quoted_exclusions:
        condition["exclude_characters"] = quoted_exclusions

    # Any_of values
    if "いずれか" in text:
        vm = re.search(r"(\d+)(?:、(\d+))+(?:のいずれか)", text)
        if vm:
            condition["values"] = [as_int(v) for v in re.findall(r"\d+", vm.group(0))]
    # Also handle "1か5" pattern (score is 1 or 5)
    vm = re.search(r"(\d+)[か](\d+)", text)
    if vm:
        condition["values"] = [as_int(v) for v in re.findall(r"\d+", vm.group(0))]

    _extract_group_scope(condition, text)

    # Cost limit
    cl = extract_cost_limit(text)
    if cl:
        condition["cost_limit"] = cl

    # Position
    pos = extract_deck_position_constraint(text)
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

    # G11: "…ほかのすべてのメンバーより多くのブレードを持つ場合" — the referenced
    # member has strictly MORE blade than every other stage member on both sides.
    # Emits a dedicated flag the engine resolves as a max-comparison
    # (see evaluate_blade_greater_than_all). Distinct from a plain blade count limit.
    if "より多くの" in text and "ブレード" in text and "持つ" in text:
        condition["blade_greater_than_all"] = True


def _infer_comparison_target_type(condition, text):
    condition["type"] = "comparison_condition"


def _infer_comparison_kind_type(condition, text):
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
            condition["count"] = as_int(cm.group(1))
            condition["cost_total"] = as_int(cm.group(1))


def _infer_resource_type(condition, text):
    if (
        condition.get("resource_type") == "blade"
        and condition.get("aggregate") == "total"
    ):
        condition["type"] = "resource_condition"
    else:
        condition["type"] = "comparison_condition"


def _infer_group_type(condition, text):
    condition["type"] = "group_condition"
    # 「ハートの総数がN以上」 — aggregate over group members' heart totals,
    # not a member-count. Engine dispatches via get_group_card_count's
    # aggregate branch (sum_group_hearts_in_stage).
    if "ハートの総数" in text or ("ハート" in text and "総数" in text):
        condition["aggregate"] = "total"
    if "コスト" in text and ("低い" in text or "高い" in text):
        condition["comparison_type"] = "cost"
        condition["operator"] = "<" if "低い" in text else ">"
        cm = extract_cost_modification(text)
        if cm:
            condition.update(cm)


def _infer_location_card_type(condition, text):
    condition["type"] = "location_condition"


def _infer_location_position_type(condition, text):
    condition["type"] = "position_condition"


def _infer_operator_target_type(condition, text):
    condition["type"] = "comparison_condition"


def _infer_aggregate_total_type(condition, text):
    if "コスト" in text or "合計が" in text:
        # "コストの合計がN" or "合計がN" → cost comparison, not score threshold
        condition["type"] = "comparison_condition"
        condition["comparison_type"] = "cost"
        condition["operator"] = "="
        # Extract the number from "合計がN" or "合計が、N", remembering any
        # quantifier that follows it.
        cm = re.search(r"合計が、?(\d+)\s*(以上|以下|未満|超|未満)?", text)
        quantifier = None
        if cm:
            condition["count"] = as_int(cm.group(1))
            quantifier = cm.group(2)
        # cost_total mirrors count for the EXACT-total shape ("合計が8の場合"),
        # not only the ones that spell out コスト. Follow-up branches of a
        # multi-reward sequential drop the noun and say only "合計が8の場合" /
        # "合計が25の場合" (PL!N-bp3-009-R＋ ライブ開始時); the engine reads the
        # threshold from cost_total, so gating it on "コスト" left those
        # branches permanently false — the card printed three rewards and only
        # the first could fire.
        # A quantifier is deliberately excluded: "必要ハート…合計が12以上" is a
        # lower bound on a heart sum, not a cost total, and giving it a
        # cost_total would make the engine compare zone costs against 12.
        if condition.get("count") and quantifier is None:
            condition["cost_total"] = condition["count"]
    else:
        condition["type"] = "score_threshold_condition"


def _infer_location_target_type(condition, text):
    condition["type"] = "location_condition"


def _infer_location_operator_type(condition, text):
    condition.setdefault("target", "self")
    condition["type"] = "location_condition"


def _infer_card_type_only(condition, text):
    condition.setdefault("count", 1)
    condition.setdefault("operator", ">=")
    condition["type"] = "card_count_condition"


_CONDITION_TYPE_FALLBACK_KEYS = (
    "operator",
    "count",
    "location",
    "card_type",
    "target",
    "comparison_type",
    "group_names",
)


def _infer_fielded_fallback_type(condition, text):
    condition.setdefault("count", 1)
    condition.setdefault("operator", ">=")
    condition.setdefault("target", "self")
    condition["type"] = "comparison_condition"


def _infer_characters_type(condition, text):
    # "それが「X」か「Y」の場合" — a character-name filter on the card just
    # placed/moved by the preceding step. Counts that card(s) in the
    # preceding move filtered by character.
    condition.setdefault("source", "preceding_moved")
    condition.setdefault("count", 1)
    condition.setdefault("operator", ">=")
    condition.setdefault("target", "self")
    condition["type"] = "location_condition"


def _infer_custom_type(condition, text):
    condition["type"] = "custom"


# Priority-ordered type-inference rules for `_infer_condition_type`:
# (predicate, apply), first match wins. Same order and bodies as the
# legacy if/elif chain:
# - comparison_target (directional: self vs opponent) takes priority over
#   location/card_type; comparison_type "equality" does NOT override — the
#   engine handles equality in location_condition via target="both" logic.
# - bare-text custom is last; empty text falls through every row.
_CONDITION_TYPE_RULES = [
    (lambda c, t: bool(c.get("comparison_target")), _infer_comparison_target_type),
    (
        lambda c, t: bool(c.get("comparison_type"))
        and c.get("comparison_type") != "equality",
        _infer_comparison_kind_type,
    ),
    (lambda c, t: bool(c.get("resource_type")), _infer_resource_type),
    (lambda c, t: bool(c.get("group_names")), _infer_group_type),
    (
        lambda c, t: bool(c.get("location") and c.get("card_type")),
        _infer_location_card_type,
    ),
    (
        lambda c, t: bool(c.get("location") and c.get("position")),
        _infer_location_position_type,
    ),
    (
        lambda c, t: bool(c.get("operator") and c.get("target")),
        _infer_operator_target_type,
    ),
    (
        lambda c, t: c.get("aggregate") == "total",
        _infer_aggregate_total_type,
    ),
    (
        lambda c, t: bool(c.get("location") and c.get("target")),
        _infer_location_target_type,
    ),
    (
        lambda c, t: bool(c.get("location") and c.get("operator")),
        _infer_location_operator_type,
    ),
    (lambda c, t: bool(c.get("card_type")), _infer_card_type_only),
    (
        lambda c, t: bool(t.strip())
        and any(k in c for k in _CONDITION_TYPE_FALLBACK_KEYS),
        _infer_fielded_fallback_type,
    ),
    (lambda c, t: bool(c.get("characters")), _infer_characters_type),
    (lambda c, t: bool(t.strip()), _infer_custom_type),
]


def _infer_condition_type(condition, text):
    """Determine condition type from extracted fields (mutates condition in place)."""
    # "それが…の場合" / "それらが…の場合" refer to the card(s) moved or
    # selected by the PRECEDING sequential step ("それ" = that card), NOT to a
    # zone state. Tag them so the engine evaluates them against moved_cards
    # (the character-name variant further down already does this in its own
    # branch; this covers the group/score/cost/card-type variants).
    stripped = text.strip()
    if (
        stripped.startswith(("それが", "それらが"))
        and not condition.get("location")
        and not condition.get("locations")
    ):
        condition.setdefault("source", "preceding_moved")

    for pred, apply in _CONDITION_TYPE_RULES:
        if pred(condition, text):
            apply(condition, text)
            break
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
    hc_m = re.search(rf"必要ハートに含まれる{HEART_ICON_ID}が(\d+)", text)
    if hc_m:
        heart_color = f"heart{hc_m.group(1).zfill(2)}"
        heart_count = as_int(hc_m.group(2))
        # 必要ハート lives on the live card — the engine's aggregate-total
        # evaluator reads need_heart from live_card_zone (+success zone).
        # Without location the condition can't be routed to that evaluator;
        # without aggregate="total" it counts cards instead of summing hearts.
        if "location" not in cond:
            cond["location"] = "live_card_zone"
        if "aggregate" not in cond:
            cond["aggregate"] = "total"
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

# --- card-type vocabulary (ONE definition) -------------------------------
# Which Japanese phrase means which card type. Every card-type scan in the
# parser — `_infer_card_type`, `_fill_or_card_types`, the move_cards or/and
# expansion, the 『group』の<type> group scan — reads this table, so the
# recognised set can never drift between them.
CARD_TYPE_KEYWORDS = (
    ("live_card", "ライブカード"),
    ("member_card", "メンバーカード"),
    ("energy_card", "エネルギーカード"),
)

# Reverse lookups, named for the direction they read in.
CARD_TYPE_PHRASE = {card_type: phrase for card_type, phrase in CARD_TYPE_KEYWORDS}
CARD_TYPE_ID = {phrase: card_type for card_type, phrase in CARD_TYPE_KEYWORDS}

# The three phrases as one alternation, for building the "two card types
# joined by X" patterns below.
_CARD_TYPE_ALT = "|".join(phrase for _, phrase in CARD_TYPE_KEYWORDS)

# 「AかB」 is an either-or choice; 「AとB」 is a both-at-once requirement.
# Each is matched on the raw text to decide whether a list of card types is
# being expressed at all; `_card_types_in` then pulls the actual list out.
_OR_CARD_TYPE_PAIR_RE = re.compile(rf"({_CARD_TYPE_ALT}).*か.*({_CARD_TYPE_ALT})")
_AND_CARD_TYPE_PAIR_RE = re.compile(rf"({_CARD_TYPE_ALT}).*と.*({_CARD_TYPE_ALT})")

# 『group』のメンバーカード — which group a typed card count refers to.
_TYPED_GROUP_RE = re.compile(rf"『([^』]+)』の({_CARD_TYPE_ALT})")

# A bare typed card count (メンバーカード3枚) with no group named.
_TYPED_COUNT_RE = re.compile(rf"(?:{_CARD_TYPE_ALT})\d+枚")


def _card_types_in(text, in_text_order=False):
    """The card types named by `text`, from CARD_TYPE_KEYWORDS.

    `in_text_order` returns them in the order the phrases appear in the text
    (what 「AとB」 means); otherwise they come back in table order (what the
    either-or lists have always used).
    """
    found = [(phrase, card_type) for card_type, phrase in CARD_TYPE_KEYWORDS if phrase in text]
    if in_text_order:
        found.sort(key=lambda pair: text.index(pair[0]))
    return [card_type for _, card_type in found]


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
    heart_count = _heart_count(text)
    blade_count = text.count(BLADE_ICON)
    energy_count = text.count(ENERGY_ICON)
    all_heart_count = text.count(ALL_HEART_ICON)
    total = heart_count + blade_count + energy_count + all_heart_count
    return total


# Which resource a gain is of, most specific first. The icon forms precede the
# word forms so 「{{icon_blade.png|ブレード}}」 reads as a blade gain, and a
# named heart icon beats a bare 「ハート」.
_RESOURCE_BY_PHRASE = (
    (BLADE_ICON, "blade"),
    (ENERGY_ICON, "energy"),
    ("{{heart_03.png|heart03}}", "heart03"),
    ("{{heart_02.png|heart02}}", "heart02"),
    ("{{heart_01.png|heart01}}", "heart01"),
    ("ブレード", "blade"),
    ("ハート", "heart"),
)


def infer_resource(d, text):
    """Infer resource type for gain_resource actions."""
    for phrase, resource in _RESOURCE_BY_PHRASE:
        if phrase in text:
            d["resource"] = resource
            return
    d["resource"] = "generic"


def _is_heart_gain(d, text):
    """True when the action is a heart (ハート) resource gain."""
    res = d.get("resource")
    if res in ("heart", "ハート"):
        return True
    if isinstance(res, str) and res.startswith("heart0"):
        return True
    if d.get("action") == "gain_resource" and "{{heart_" in text:
        return True
    return False


def _enrich_heart_gain_multiset(d, effect_text):
    """Represent a heart gain's quantity as a multiset of heart-color tokens.

    One entry is emitted per granted heart so the engine grants exactly that
    many hearts of each color. This fixes the prior behaviour where ``count``
    collapsed the ``{{heart_XX}}`` icons into a distinct set and the constant-
    ability grant path multiplied ``count`` across every color (giving N of
    EACH color instead of one of each).
    """
    # Only count heart icons that are actually GAINED. A heart icon that is
    # part of a "Xを持つ" (target HAS heart X) filter clause is a condition,
    # not a gain — otherwise e.g. "...heart06を持つメンバーはheart06×4を得る"
    # would over-count to 5.
    colors = _heart_ids_gained(effect_text)
    if colors:
        m_nts = re.search(r"(\d+)つ得る", effect_text)
        if m_nts:
            n = as_int(m_nts.group(1))
            distinct = list(dict.fromkeys(colors))
            if len(distinct) == 1:
                # "heart02を3つ得る" -> 3 tokens of heart02
                colors = [distinct[0]] * n
            # (distinct > 1 with Nつ is ambiguous; keep tokens as written)
        d["heart_colors"] = colors
        d["count"] = len(colors)
        if detect_require_all_hearts(effect_text):
            d["require_all_heart_colors"] = True
        return
    # No explicit heart icons: a color-choice / generic heart gain. Preserve an
    # explicit "Nつ" count (e.g. "選んだハートを2つ得る").
    m_nts = re.search(r"(\d+)つ得る", effect_text)
    if m_nts:
        d["count"] = as_int(m_nts.group(1))


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
    # Heart gains use a multiset representation; handled separately so a single
    # heart icon used as a condition is not miscounted as a gain.
    if _is_heart_gain(d, effect_text):
        _enrich_heart_gain_multiset(d, effect_text)
        return
    # Issue 9: Prefer explicit numeric count (e.g. "2つ得る") over icon counting
    # so that "ハートを2つ得る" with a single heart icon correctly gets count=2
    count_match = re.search(r"(\d+)つ", effect_text)
    if count_match:
        d["count"] = as_int(count_match.group(1))
        return
    # Fix D19: a numeric count in the FULL text may belong to a blade-count
    # filter clause ("ブレードを4つ以上持つ") rather than to the resource being
    # gained. When the blade icon is a target filter, skip that count and fall
    # through to icon-based counting so the heart gain gets the right amount.
    if not _blade_icon_is_target_filter(text):
        count_match = re.search(r"(\d+)つ", text)
        if count_match:
            d["count"] = as_int(count_match.group(1))
            return
    blade_count = effect_text.count(BLADE_ICON)
    if blade_count > 0:
        d["count"] = blade_count
        return
    all_heart_count = effect_text.count(ALL_HEART_ICON)
    if all_heart_count > 0:
        d["count"] = all_heart_count
        return
    heart_count = _heart_count(effect_text)
    if heart_count > 0:
        # Check for consecutive heart icons (e.g. 4 heart06 in a row = gain 4)
        # This correctly handles "{{heart_06.png|heart06}}{{heart_06.png|heart06}}... = gain N"
        # vs "{{heart_06.png|heart06}}を持つ" (has heart06, used as condition not count)
        consecutive = re.findall(rf"(?:{HEART_ICON}){{2,}}", effect_text)
        if consecutive:
            # Use the longest consecutive run as the actual gain count
            d["count"] = max(_heart_count(run) for run in consecutive)
        else:
            d["count"] = heart_count
        return


def _fill_defaults_count_and_refine(action, text, action_text, a):
    """under_member promotion, revealed_cards defaults, count extraction, custom refinement. Returns updated action type."""
    # NOTE: unconditional promotion is LOAD-BEARING. The engine's
    # execute_place_energy_under_member special-cases source=under_member
    # with destination=energy_zone/empty_area/energy_deck (bp5-012 deck
    # deploys counted from under-member, Burn!!-style returns, member
    # deploys). Relabeling these as move_cards changes which engine path
    # runs and breaks gameplay even when the label looks wrong.
    if action.get("source") == "under_member" and a != "place_energy_under_member":
        action["action"] = "place_energy_under_member"
        a = "place_energy_under_member"
        action.setdefault("energy_count", 1)
        action.setdefault("target_member", "this_member")
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
    if not action.get("all") and _ALL_KW_RE.search(text):
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
    _infer_missing_count(action, text, action_text)
    if action.get("action") == "custom":
        _rescue_custom_action(action, text)
    return action.get("action")


# Actions that act on exactly one card by default when the text states no
# number. Naming them keeps an uncounted action from reaching the engine with
# no count at all.
_SINGLE_CARD_ACTIONS = (
    "move_cards",
    "draw_card",
    "gain_resource",
    "reveal",
    "look_at",
    "change_state",
    "restriction",
)

# 「置いた枚数[分]」 — a draw whose count is however many were just drawn.
_DRAWN_COUNT_RE = re.compile(r"置いた.*枚数")

# 「枚数…増やす/減らす」 — a `custom` action that is really a limit change.
# Increase is tested before decrease, first match wins.
_LIMIT_CHANGE_PATTERNS = (
    (re.compile(r"枚数.*\d*枚(増やす|増え)"), "increase"),
    (re.compile(r"枚数.*\d*枚(減らす|減る)"), "decrease"),
)

_SCORE_ADD_RE = re.compile(r"スコアを[+＋]\d+する")
_SIGNED_NUMBER_RE = re.compile(r"([+＋])(\d+)")


def _count_from_icons(action, action_text):
    """The count implied by the resource icons in `action_text`, or None.

    A required-heart clause counts per colour (so the colours must agree), while
    every other action counts the icons themselves.
    """
    if action.get("action") == "modify_required_hearts":
        target_colors = action.get("heart_colors", [])
        per_colour = {
            colour: n
            for colour, n in _heart_label_counts(action_text).items()
            if not target_colors or colour in target_colors
        }
        return _uniform_count(per_colour) if per_colour else None
    icon_count = _count_resource_icons(action_text)
    return icon_count if icon_count > 0 else None


def _infer_missing_count(action, text, action_text):
    """Fill an action's count when the text states none."""
    if (
        action.get("count") is not None
        or "dynamic_count" in action
        or action.get("any_number")
        or action.get("all")
    ):
        return
    extracted = extract_count(text)
    if extracted is not None:
        action["count"] = extracted
        return
    counted = _count_from_icons(action, action_text)
    if counted is not None:
        action["count"] = counted
    if action.get("count") is not None or action.get("action") not in _SINGLE_CARD_ACTIONS:
        return
    if action.get("action") == "change_state" and action.get("group_names"):
        return
    if action.get("action") == "draw_card" and _DRAWN_COUNT_RE.search(text):
        action["dynamic_count"] = {
            "type": "drawn_cards",
            "reference": "previous_draw",
        }
        return
    action["count"] = 1


def _rescue_custom_action(action, text):
    """Give a `custom` action a real name when the text obviously implies one.

    The dispatch table gives up on some phrases; these are the shapes that were
    still recognisable from the text on its own.
    """
    if action.get("ability_gain"):
        action["action"] = "gain_ability"
        return
    for pattern, operation in _LIMIT_CHANGE_PATTERNS:
        if pattern.search(text):
            action["action"] = "modify_limit"
            action.setdefault("operation", operation)
            count = extract_count(text)
            if count:
                action["count"] = count
            return
    if _SCORE_ADD_RE.search(text):
        action["action"] = "modify_score"
        action.setdefault("operation", "add")
        signed = _SIGNED_NUMBER_RE.search(text)
        if signed:
            action["value"] = as_int(signed.group(2))


# 「AをBに」 with no explicit source: which zone the cards come from. Ordered,
# first match wins — それら must be tested after それらのカード, and both before
# any destination-driven fallback.
_PRONOUN_SOURCE_PHRASES = (
    (("それらのカード",), "revealed_cards"),
    (("それら",), "selected_cards"),
)

# 「〜を持つ」 — a card-property filter and whether the clause negates it.
_CARD_PROPERTY_PHRASES = (
    (("ブレードハートを持たない",), "has_blade_heart", True),
    (("ブレードハートを持つ",), "has_blade_heart", False),
    ((HAS_SCORE_ICON,), "has_score_icon", False),
)

# 「メンバーのいないエリアに登場」 — a stage slot with no member in it.
_EMPTY_AREA_MARKER = "メンバーのいないエリア"

# Destinations that only exist for a card already accounted for elsewhere.
# With no source the engine has nothing to move, so these fall through to
# `custom` rather than becoming a bogus move.
_ZONE_ONLY_DESTINATIONS = ("live_card_zone", "success_live_zone", "stage")

# 「…をそのメンバーのコストにNを足した数に等しいコスト」 — a cost that tracks
# the cards a preceding action just moved.
_PREVIOUS_MOVE_COST_RE = re.compile(r"コストに(\d+)を足した数に等しいコスト")


def _infer_move_source(action, text, cached_source):
    """Fill in a move_cards source the parse did not state."""
    if "source" in action:
        return
    if cached_source:
        action["source"] = cached_source
    if action.get("source") is None and "控え室から" in text:
        action["source"] = "discard"
    # "自分のエネルギー1枚をエネルギーデッキに置く" — the engine defaults an
    # empty source to discard, which never contains energy cards, so the move
    # would silently no-op. Energy returns to the energy deck from the zone.
    if (
        action.get("source") is None
        and action.get("destination") == "energy_deck"
        and "エネルギー" in text
    ):
        action["source"] = "energy_zone"
    if "source" in action:
        return
    dest = action.get("destination", "")
    for phrases, zone in _PRONOUN_SOURCE_PHRASES:
        if any(phrase in text for phrase in phrases):
            action["source"] = zone
            return
    if dest in ("deck_top", "deck_bottom", "deck"):
        # A deck-bound move that names no member and makes no choice is a
        # hand shuffle-in.
        if not any(keyword in text for keyword in ("メンバー", "選ぶ", "選び")):
            action["source"] = "hand"
    elif dest == "discard":
        if "このカード" in text:
            action["source"] = "deck_top"
        elif "そのカード" in text:
            action["source"] = "looked_at"
        elif "エネルギー" not in text:
            action["source"] = "hand"


def _apply_card_property(action, text):
    """Fill a card-property filter the parse did not state."""
    if "card_property" in action:
        return
    for phrases, prop, negated in _CARD_PROPERTY_PHRASES:
        if any(phrase in text for phrase in phrases):
            action["card_property"] = prop
            if negated:
                action["negation"] = True
            return


def _apply_previous_move_cost_reference(action, text):
    """Point a same_area move's cost at the card a preceding action moved."""
    if action.get("source") != "discard" or action.get("destination") != "same_area":
        return
    if "そのメンバーのコストに" not in text:
        return
    m = _PREVIOUS_MOVE_COST_RE.search(text)
    if m:
        action["cost_reference"] = "previous_moved_card"
        action["cost_offset"] = as_int(m.group(1))
        action.setdefault("cost_limit_operator", "=")


def _redirect_to_empty_area(action):
    """Redirect 「メンバーのいないエリアに登場」 to the empty_area destination.

    Returns the destination in force afterwards.
    """
    step_text = action.get("text") or ""
    if _EMPTY_AREA_MARKER not in step_text or action.get("destination") != "stage":
        return action.get("destination")
    # 「そのカード」 refers to a card from a preceding action; anything else
    # needs a real source before it can go anywhere.
    if "そのカード" in step_text:
        action["source"] = "preceding_moved"
    elif action.get("source") is None:
        return action.get("destination")
    action["destination"] = "empty_area"
    return "empty_area"


def _needs_custom_fallback(action, destination):
    """True when a move is too under-specified for the engine to run."""
    has_source = action.get("source") is not None
    if not has_source and action.get("destination") is None:
        return True
    return destination in _ZONE_ONLY_DESTINATIONS and not has_source


def _fill_defaults_move_cards(action, text, _cached_source, _cached_dest):
    """Source/destination/card_type inference for move_cards actions. Returns updated action type."""
    a = action.get("action")
    if (
        a == "move_cards"
        and action.get("destination") == "hand"
        and "source" not in action
    ):
        action["source"] = "discard"
    if a != "move_cards":
        return a
    _infer_move_source(action, text, _cached_source)
    if "destination" not in action and _cached_dest:
        action["destination"] = _cached_dest
    _apply_previous_move_cost_reference(action, text)
    if "card_type" not in action and "or_card_types" not in action:
        card_type = _infer_card_type(text, action)
        if card_type:
            action["card_type"] = card_type
    _apply_card_property(action, text)
    if "state_change" not in action and "ウェイト状態" in text:
        action["state_change"] = "wait"
    destination = _redirect_to_empty_area(action)
    if _needs_custom_fallback(action, destination):
        action["action"] = "custom"
        return "custom"
    _expand_typed_card_move(action, text)
    return action.get("action")


def _expand_typed_card_move(action, text):
    """Split a move that names two card types into one move per type.

    「AかB」 becomes a single move with `or_card_types`; 「AとB」 becomes a
    sequential of one fully-typed move per type. Both only apply when the move
    already has a single inferred card_type, so an explicit card_type from an
    earlier rule is never overwritten.
    """
    if not action.get("card_type"):
        return
    if _OR_CARD_TYPE_PAIR_RE.search(text):
        or_types = _or_card_types_in(text)
        if or_types:
            action["or_card_types"] = or_types
            action.pop("card_type", None)
        return
    if not _AND_CARD_TYPE_PAIR_RE.search(text):
        return
    and_types = _card_types_in(text, in_text_order=True)
    if len(and_types) < 2 or not action.get("source") or not action.get("destination"):
        return
    # 『group』のメンバーカード — each type may name its own group.
    typed_groups = {
        CARD_TYPE_ID[phrase]: group
        for group, phrase in _TYPED_GROUP_RE.findall(text)
    }
    sub_actions = []
    for card_type in and_types:
        group = typed_groups.get(card_type)
        sub_text = action.get("text", "")
        if group:
            phrase = CARD_TYPE_PHRASE[card_type]
            sub_text = re.search(
                rf"『{re.escape(group)}』の{phrase}\d*枚?", text
            ).group(0)
        sub = {
            "text": sub_text,
            "action": "move_cards",
            "source": action["source"],
            "destination": action["destination"],
            "card_type": card_type,
            "count": action.get("count", 1),
            "max": True,
            "target": action.get("target", "self"),
        }
        if group:
            sub["group_names"] = [group]
        if action.get("optional") is not None:
            sub["optional"] = action["optional"]
        sub_actions.append(sub)
    action["action"] = "sequential"
    action["actions"] = sub_actions
    action.pop("card_type", None)
    action.pop("multiple_targets", None)


# Ordered operation phrases for modify_required_hearts: first match wins.
# Same order as the legacy if-chain (減らす-family before 増やす-family
# before になる, defaulting to decrease).
_MODIFY_HEARTS_OPERATION_RULES = (
    (("減らす", "減らし", "減る", "減って"), "decrease"),
    (("増やす", "増える", "増やし"), "increase"),
    (("になる", "にする"), "set"),
)

# Ordered source zones for select actions: first matching phrase wins.
# Same order as the legacy if-chain.
_SELECT_SOURCE_RULES = (
    (("控え室にある", "控え室の"), "discard"),
    (("手札の", "手札にある"), "hand"),
    (("ステージにいる", "ステージの"), "stage"),
    (("ライブ中の", "ライブカード置き場"), "live_card_zone"),
    (("それらの中から",), "revealed_cards"),
)


def _fill_draw_shuffle(action, text, _cached_source, _cached_dest):
    """Draw/shuffle normalization. Returns the (possibly updated) action name."""
    a = action.get("action")
    if a == "draw":
        action["action"] = "draw_card"
        a = "draw_card"
    if a == "draw_card":
        action.setdefault("source", "deck")
        action.setdefault("destination", "hand")
        return a
    # Shuffle is always combined with a move action (shuffle then place).
    # If dispatch matched shuffle but text also has a destination pattern, emit move_cards with shuffle flag.
    if a == "shuffle":
        dest = _cached_dest
        if dest:
            action["action"] = "move_cards"
            action["shuffle"] = True
            action["destination"] = dest
            if "source" not in action:
                s = _cached_source
                if s:
                    action["source"] = s
            if "card_type" not in action:
                ct = _infer_card_type(text, action)
                if ct:
                    action["card_type"] = ct
            return "move_cards"
    return a


def _fill_gain_resource(action, text, action_text):
    if action.get("action") != "gain_resource":
        return
    if "resource" not in action:
        infer_resource(action, text)
    infer_count_from_icons(action, action_text)
    if action.get("count") is None:
        action["count"] = 1
    # Extract target_count from "N人" (e.g., "メンバー1人" → target_count=1)
    tc_match = re.search(r"(\d+)人", text)
    if tc_match:
        action["target_count"] = as_int(tc_match.group(1))
    # Extract distinct_card_name from "名前の異なる" (different name constraint)
    if "名前の異なる" in text:
        action["distinct"] = "card_name"


def _fill_heart_colors(action, text):
    # Heart gains already have their multiset set by infer_count_from_icons.
    # For those, do NOT re-derive heart_colors from the full text,
    # which would leak condition/requirement hearts onto the gain effect.
    if "heart_colors" not in action and not _is_heart_gain(action, text):
        hc = extract_heart_colors_from_text(text)
        if hc:
            action["heart_colors"] = hc
            if detect_require_all_hearts(text):
                action["require_all_heart_colors"] = True
    # Strip heart_colors that leak from condition/filter clauses. Only a heart
    # gain (resource heart) legitimately carries heart_colors; for non-heart
    # resources (blade/energy) it is leakage, and for heart_type="all" gains the
    # icon_all already encodes every color so the list is redundant.
    if action.get("action") == "gain_resource":
        res = action.get("resource")
        if res in ("heart", "ハート") or (
            isinstance(res, str) and res.startswith("heart0")
        ):
            if action.get("heart_type") == "all" and action.get("heart_colors"):
                action.pop("heart_colors", None)
                action.pop("require_all_heart_colors", None)
        else:
            if action.get("heart_colors"):
                action.pop("heart_colors", None)
                action.pop("require_all_heart_colors", None)


def _fill_hearts_operation(action, text):
    if action.get("action") != "modify_required_hearts" or "operation" in action:
        return
    # 減らし is the continuative (連用) form used mid-sentence: "A減らし、B増やす"
    for phrases, op in _MODIFY_HEARTS_OPERATION_RULES:
        if any(p in text for p in phrases):
            action["operation"] = op
            return
    action["operation"] = "decrease"


def _fill_exclude_groups(action, text):
    # Exclude group names: detect 『X』以外 pattern (shared helper, also
    # used by the condition extractor).
    if "以外" in text:
        apply_group_exclusions(action, text)


def _fill_per_unit_extras(action, text, a):
    if a not in (
        "modify_score",
        "gain_resource",
        "modify_cost",
        "perform_yell",
    ):
        return
    if action.get("per_unit"):
        if "色につき" in text or "色に付き" in text:
            action["per_unit_type"] = "heart_colors"
        elif "コスト" in text and "につき" in text:
            action["per_unit_type"] = "cost"
            cm = re.search(r"コスト(\d+)につき", text)
            if cm:
                action["per_unit_count"] = as_int(cm.group(1))
        # Issue 15: per_unit_source from "これにより控え室に置いたカード" pattern
        if "これにより" in text and ("置いた" in text or "置かれた" in text):
            action["per_unit_source"] = "previous_moved_cards"
    # Issue 15: max_repeats from "N枚までしか" / "N回までしか" patterns
    max_m = re.search(r"(\d+)(?:枚|回|つ)までしか", text)
    if not max_m:
        max_m = re.search(r"(\d+)までしか", text)
    if max_m:
        action["max_repeats"] = as_int(max_m.group(1))
    # Issue 6: Detect timing constraint for gain_resource
    if "このターンに登場" in text and a == "gain_resource":
        action["timing_condition"] = "appeared_this_turn"


def _fill_select_source(action, text):
    # Infer source for select actions from common location patterns.
    # When multiple zones appear in text (e.g. count reference mentions stage
    # but actual selection is from waiting room), prefer the selection source.
    for phrases, source in _SELECT_SOURCE_RULES:
        if any(p in text for p in phrases):
            action["source"] = source
            return


def _fill_select_target(action, text):
    # Fix target for select actions: when both "自分の" and "相手の" appear in text
    # (e.g., count reference mentions opponent but selection is from your waiting room),
    # extract_target may incorrectly return "both". Override to the correct player
    # based on the source zone's owner.
    src = action.get("source", "")
    zones = (("discard", "控え室"), ("stage", "ステージ"), ("hand", "手札"))
    for zone, word in zones:
        if src == zone:
            own = f"自分の{word}"
            opp = f"相手の{word}"
            if own in text and opp not in text:
                action["target"] = "self"
            elif opp in text and own not in text:
                action["target"] = "opponent"
            return


def _or_card_types_in(text):
    """Card types named by an either-or phrase, in table order.

    Returns [] unless the text names at least two — a single card type is not
    an either-or, it is just a card_type.
    """
    or_types = _card_types_in(text)
    return or_types if len(or_types) >= 2 else []


def _fill_or_card_types(action, text, pop_card_type):
    """Record 「AかB」 / 「Aのどちらか」 as `or_card_types`.

    When the member-card variant carries a cost limit, lift it onto the action
    at the same time — that clause is only ever written on the member-card
    half of the either-or.
    """
    for card_type, phrase in CARD_TYPE_KEYWORDS:
        if phrase in text and card_type == "member_card" and pop_card_type:
            cl = extract_cost_limit(text)
            if cl:
                action["cost_limit"] = cl
    or_types = _or_card_types_in(text)
    if not or_types:
        return
    action["or_card_types"] = or_types
    if pop_card_type:
        action.pop("card_type", None)


def _fill_need_heart(action, text):
    # Parse need_heart constraints like "{{heart_06.png|heart06}}を3以上含むライブカード"
    nh = re.search(r"(?:heart\d{2}|heart\d{2}[^」]*?})を(\d+)以上含む", text)
    if nh:
        # Extract the heart color from the raw text (either bare "heart06" or icon "{{heart_06.png|heart06}}")
        color_match = re.search(r"heart(\d{2})", text[: nh.end()])
        if color_match:
            action["need_heart_color"] = f"heart{as_int(color_match.group(1)):02d}"
            action["need_heart_total"] = as_int(nh.group(1))
            action["need_heart_operator"] = ">="

    # Also parse plain-text patterns like "ハートを4つ以上持つ" (without heart icon)
    nh2 = re.search(r"ハートを(\d+)つ以上持つ", text)
    if nh2:
        # Total heart icon count threshold (any colors)
        action["need_heart_total"] = as_int(nh2.group(1))
        action["need_heart_operator"] = ">="


_UNSET = object()


def _fill_defaults(action, text, _cached_source=_UNSET, _cached_dest=_UNSET):
    """Consolidated post-dispatch normalization. Fills defaults every action needs.

    Field ownership (C6): parse_action owns ALL field extraction (source,
    destination, count, card_type, target, state_change, cost_limit,
    optional, max, position and group_names). This function ONLY fills
    defaults for fields left unset (guarded by `not in action`) plus
    action-type-specific defaults, so it is safe to call on fresh
    sub-action dicts. No extraction should be added here — add it to
    parse_action instead.
    """
    if _cached_source is _UNSET:
        _cached_source = extract_source(text)
    if _cached_dest is _UNSET:
        _cached_dest = extract_destination(text)
    action_text = action.get("text", text) or text
    a = action.get("action")
    # Normalize "revealed_card" (singular) to "revealed_cards" (plural) for consistency
    if action.get("source") == "revealed_card":
        action["source"] = "revealed_cards"
    a = _fill_draw_shuffle(action, text, _cached_source, _cached_dest)

    if action.get("source") == "selected_cards":
        action.setdefault("count", 1)
    _fill_gain_resource(action, text, action_text)
    # same_name: applied to ALL action types, not just gain_resource
    if "same_name" not in action and (
        "と同じ名前" in text or ("同じ名前" in text and "持つ" in text)
    ):
        action["same_name"] = True
    _fill_heart_colors(action, text)
    _fill_hearts_operation(action, text)
    _fill_exclude_groups(action, text)
    _fill_per_unit_extras(action, text, a)
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
        _m = detect_position_matches(text)
        if _m:
            action["position"] = _m[0][1]
    a = _fill_defaults_move_cards(action, text, _cached_source, _cached_dest)
    if (
        "cost_limit" not in action
        and "cost_total" not in action
        and "cost_limit_min" not in action
    ):
        cl = extract_cost_limit(text)
        if cl:
            action["cost_limit"] = cl
            if "cost_limit_operator" not in action:
                action["cost_limit_operator"] = extract_operator(text) or "="
    # OR card types for ALL action types (not just move_cards/select)
    if a not in ("move_cards", "select") and "or_card_types" not in action:
        if _OR_CARD_TYPE_PAIR_RE.search(text):
            _fill_or_card_types(action, text, pop_card_type=False)
    a = _fill_defaults_count_and_refine(action, text, action_text, a)

    if "optional" not in action and extract_optional(text):
        action["optional"] = True
    # Infer source for select actions from common location patterns.
    # When multiple zones appear in text (e.g. count reference mentions stage
    # but actual selection is from waiting room), prefer the selection source.
    if a == "select" and "source" not in action:
        _fill_select_source(action, text)
    # Fix target for select actions: when both "自分の" and "相手の" appear in text
    # (e.g., count reference mentions opponent but selection is from your waiting room),
    # extract_target may incorrectly return "both". Override to the correct player
    # based on the source zone's owner.
    if a == "select" and action.get("target") == "both":
        _fill_select_target(action, text)
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
        _fill_or_card_types(action, text, pop_card_type=True)
    if action.get("per_unit") and "dynamic_count" not in action:
        action["dynamic_count"] = {"type": "per_unit", "reference": "unit_count"}
        if "count" in action and action["count"] is None:
            del action["count"]
    _fill_need_heart(action, text)


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
# See the _STRUCTURAL_EFFECT_RULES list at the end of this section for the
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


def _per_unit_gate(result, per_text):
    """Strip a leading gate (場合 / とき、 / 時、) off the per-unit reference,
    parse it onto result["condition"], and return the remaining per_text."""
    # Extract condition from per_text if present
    # Pattern: "条件場合、per_unit_reference" e.g.
    # "自分のセンターエリアに『μ's』のメンバーがいる場合、そのメンバーが持つheart03 2つ"
    cond_part, remaining = split_condition_action(per_text)
    if cond_part and remaining:
        parsed_cond = parse_condition(cond_part)
        if _is_real_condition(parsed_cond):
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
                        if _is_real_condition(cond):
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
                if _is_real_condition(cond):
                    result["condition"] = cond
                per_text = remaining

    return per_text


def _per_unit_count_type(result, text, per_text):
    """Resolve per_unit_count / per_unit_type from reference + full text."""
    pm = re.search(r"(\d+)(人|枚|つ)(につき|ごとに)", text)
    if pm:
        result["per_unit_count"] = as_int(pm.group(1))
        result["per_unit_type"] = pm.group(2)
        if "ライブ中のカード" in text or "ライブ中のライブカード" in text:
            result["per_unit_type"] = "live_card_zone"
    else:
        # Handle "コストNにつき" (cost-based scaling without explicit counter unit)
        cm = re.search(r"コスト(\d+)(につき|ごとに)", text)
        if cm:
            result["per_unit_count"] = as_int(cm.group(1))
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


def _per_unit_filters(result, text, per_text):
    """Filter fields from the per-unit reference: excluded/included groups,
    counted heart colors, distinct, card_property, exclude_self, cost_limit,
    timing_condition, state, this-cost-waited source, target, card_type,
    location."""
    # Check for excluded groups: 『group』以外
    exc_gns = re.findall(r"『([^』]+)』以外", per_text)
    if exc_gns:
        result["exclude_group_names"] = exc_gns
    # Extract exclude_heart_colors from per_text
    # Pattern: "{{heart_01.png|heart01}}と{{heart_06.png|heart06}}以外の色のハートを持つ"
    if "以外" in per_text:
        before_igai = per_text.split("以外")[0]
        excluded = _heart_ref_ids(before_igai, unique=True)
        if excluded:
            result["exclude_heart_colors"] = excluded
    # Extract included groups only (groups NOT followed by 以外)
    remaining_per_text = re.sub(r"『[^』]+』以外", "", per_text)
    gm = re.search(r"『([^』]+)』", remaining_per_text)
    if gm:
        result["group_names"] = [gm.group(1)]
    # Extract heart colors from per_text for per-unit counting (e.g. heart03 from "そのメンバーが持つ{{heart_03.png|heart03}}2つ")
    # Exclude heart colors in exclusion patterns (e.g. "heart01とheart06以外の色" — those are excluded colors, not counted)
    counted_colors = _counted_per_unit_colors(per_text)
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
        detect_exclude_self(per_text)
        or "このカード以外" in per_text
        or "自分以外" in per_text
        or re.search(r"他の.*?(?:メンバー|カード)", per_text)
        or "これを除く" in per_text
    ):
        result["exclude_self"] = True

    # Extract cost_limit from per-text (e.g., "コスト4以上")
    cl = extract_cost_limit(per_text)
    if cl:
        result["cost_limit"] = cl
        op = extract_operator(per_text)
        if op:
            result["cost_limit_operator"] = op

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

    # Issue 15: 「これにより…ウェイト（状態）にしたメンバー1人につき」 counts
    # members newly waited BY THIS ABILITY'S COST, not all currently-waited
    # members on stage (Q183). The engine resolves this via the cost-wait
    # tracker (GameState::last_cost_waited_members) instead of a stage scan.
    if (
        "これにより" in text
        and ("ウェイト状態にした" in per_text or "ウェイトにした" in per_text)
    ):
        result["per_unit_source"] = "this_cost_waited"

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
        ("メンバーの下に置かれている", "under_member"),
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

    per_text = _per_unit_gate(result, per_text)

    # Extract duration from per_text (e.g., "ライブ終了時まで、カード1枚につき")
    per_text, duration = _strip_duration_prefix(per_text)
    if duration:
        result["duration"] = duration

    _per_unit_count_type(result, text, per_text)
    _per_unit_filters(result, text, per_text)

    action_text = text.split("につき", 1)[1].strip().lstrip("、")

    # A per-unit effect can itself chain two actions. その後 is left to
    # _try_per_unit_sono_ato below, or its conditional tail would be shredded
    # into bogus per-comma actions.
    if "、" in action_text and "し" in action_text and "その後" not in action_text:
        actions = _per_unit_comma_chain(result, action_text)
        if actions:
            return {"text": text, "action": "sequential", "actions": actions}
    if "して" in action_text:
        steps = _per_unit_te_form_chain(result, action_text)
        if steps:
            return {"text": text, "action": "sequential", "actions": steps}

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
    if action.get("action") == "sequential":
        steps = action.get("actions", [])
        for sub in steps:
            _propagate(result, sub, skip_existing=True)
        _share_discard_per_unit_type(
            steps, next((s for s in steps if s.get("per_unit_type")), None)
        )

    # Detect cost reduction per unit patterns (コストが～につき～少なくなる/減る)
    if (
        action.get("action") == "custom"
        and result.get("location") == "hand"
        and result.get("per_unit_type") in ("member", "人", "枚")
    ):
        if "少なくなる" in action_text or "減る" in action_text:
            action["action"] = "modify_cost"
            action["operation"] = "subtract"

    if "その後" in action_text:
        return _per_unit_sono_ato(result, text, action_text)

    # Issue 15: Extract per_unit_source from "これにより控え室に置いた" patterns
    if "これにより" in text and ("置いた" in text or "置かれた" in text):
        action["per_unit_source"] = "previous_moved_cards"
    # Issue 15: Extract max_repeats from "N枚/回/つまでしか" patterns
    max_m = _MAX_REPEATS_RE.search(text)
    if max_m:
        action["max_repeats"] = as_int(max_m.group(1))

    action["text"] = text
    return action


# 「N(枚|回|つ)までしか」 — a cap on how many times an effect may repeat.
_MAX_REPEATS_RE = re.compile(r"(\d+)(?:枚|回|つ)?までしか")


def _icon_pair_colors(text):
    """The heart colours `text` names, from the two-capture icon form."""
    return sorted({f"heart{m.zfill(2)}" for _, m in re.findall(HEART_ICON_PAIR, text)})


def _counted_per_unit_colors(per_text):
    """The heart colours a 「〜につき」 clause counts, excluding 「以外」 ones.

    「heart01とheart06以外の色」 names two colours it does NOT count, so the
    colours before 以外 are dropped rather than counted.
    """
    colors = _icon_pair_colors(per_text)
    if not colors:
        return []
    excluded = (
        set(_icon_pair_colors(per_text.split("以外")[0]))
        if "以外" in per_text
        else set()
    )
    return [colour for colour in colors if colour not in excluded]


def _per_unit_comma_chain(result, action_text):
    """「Aし、B」 split on commas — a per-unit effect chaining two actions."""
    parts = [part.strip().rstrip("、") for part in action_text.split("、")]
    if len(parts) < 2 or "し" not in parts[0]:
        return None
    actions = []
    for part in parts:
        parsed = parse_action(part)
        if _is_real_action(parsed):
            _propagate(result, parsed)
            actions.append(parsed)
    return actions if len(actions) >= 2 else None


def _per_unit_te_form_chain(result, action_text):
    """「AしてB」 — the same chain with no comma (コストを+4してheart05を得る)."""
    index = action_text.find("して")
    left = action_text[:index].rstrip()
    right = action_text[index + 2 :].strip().lstrip("、")
    if not left or not right:
        return None
    steps = [parse_action(left), parse_action(right)]
    if any(step.get("action", "custom") in _INERT_ACTIONS for step in steps):
        return None
    for step in steps:
        _propagate(result, step)
    return steps


def _share_discard_per_unit_type(steps, first):
    """Give later per-unit steps the first step's discard-based counting.

    When the first per-unit step counts from the discard (a card replaced by a
    baton touch → placed in the waitroom), the later ones are counting the same
    set of cards.
    """
    if first is None or first.get("per_unit_type") != "discard":
        return
    for sub in steps:
        if (
            sub is not first
            and sub.get("per_unit")
            and sub.get("per_unit_type") in ("member", "枚")
        ):
            sub["per_unit_type"] = "discard"


def _per_unit_sono_ato(result, text, action_text):
    """「…」 then 「その後、Y」 — the per-unit effect plus its tail.

    The tail is a whole sentence and is often conditional
    ("…が9以上の場合、スコアを+1する"), so it is parsed as an effect: parse_action
    would misread the condition fragment as a bogus action of its own.
    """
    before, _, tail_text = action_text.partition("その後")
    head_text = before.strip()
    tail_text = tail_text.strip().lstrip("、")
    # A "。" inside the head means another per-unit（につき) clause follows, so
    # the head is a compound of sub-effects (e.g. reveal + per-unit score).
    compound = "。" in head_text
    if compound:
        steps = [
            parsed
            for parsed in (
                parse_effect(part.strip())
                for part in head_text.split("。")
                if part.strip()
            )
            if _is_real_action(parsed) or parsed.get("actions")
        ]
        for step in steps:
            _propagate(result, step, skip_existing=True)
        if len(steps) >= 2:
            head = {"action": "sequential", "actions": steps}
        elif len(steps) == 1 and steps[0].get("action") == "sequential":
            # One "。"-sentence that itself parsed as a nested sequential
            # (e.g. "A減らし、B増やす" → two mrh actions). Use it directly rather
            # than re-parsing the whole head, which would collapse it into one
            # wrong action.
            head = steps[0]
        else:
            head = parse_action(head_text)
    else:
        head = parse_action(head_text)
    _propagate(result, head, skip_existing=compound)
    tail = parse_effect(tail_text)
    if tail.get("action") == "custom":
        tail = parse_action(tail_text)
    return {"text": text, "action": "sequential", "actions": [head, tail]}


_PROPAGATE_FIELDS = (
    "per_unit",
    "per_unit_count",
    "per_unit_type",
    "per_unit_heart_colors",
    "per_unit_source",
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
        if k not in src or (skip_existing and k in dst):
            continue
        if k in ("group_names", "exclude_group_names"):
            own_text = str(dst.get("text", ""))
            groups = src[k] or []
            if (
                not any(str(group) in own_text for group in groups)
                and not dst.get("per_unit")
                and not dst.get("per_unit_type")
            ):
                continue
        dst[k] = src[k]


_try_yell_source_modifier = EffectPattern(
    condition=lambda t: "エール" in t
    and "代わりに" in t
    and "行う" in t
    and ("デッキの上から" in t or "デッキの下から" in t)
    and re.search(r"エール.*?代わりに.*?(デッキの[上下]から|山札の[上下]から).*?行う", t)
    is not None,
    action="modify_yell_source",
    setter=lambda t, r: r.update(
        {
            "yell_source": "deck_bottom"
            if "デッキの下から" in t or "山札の下から" in t
            else "deck_top"
        }
    ),
)


def _try_activation_history_tiers(text):
    """このターン…アクティブにしていた場合 tiers (Q203 Cara Tesoro).

    「このターン、自分の『G』のカードの効果によってウェイト状態の自分の
    エネルギーをアクティブにしていた場合、E1。さらに、…メンバーもアクティブに
    していた場合、代わりにE2。」

    The clauses are HISTORICAL conditions over this turn's activations — not
    state checks. Emitted as from/to state_change conditions with turn scope
    and source-group attribution; the engine evaluates them against
    GameState::turn_state_changes (see evaluate_state_change_condition).
    """
    if "アクティブにしていた場合" not in text or "さらに" not in text:
        return None

    def activation_cond(clause):
        gm = re.search(r"『([^』]+)』のカードの効果によって", clause)
        kind = "energy_card" if "エネルギー" in clause else "member_card"
        body = clause if clause.startswith("このターン") else "このターン、" + clause
        cond = {
            "type": "state_condition",
            "state": "active",
            "from_state": "wait",
            "to_state": "active",
            "temporal": "this_turn",
            "card_type": kind,
            "target": "self",
            "text": body,
        }
        if gm:
            cond["group_names"] = [gm.group(1)]
        else:
            return None
        return cond

    parts = [p.strip() for p in text.split("。") if p.strip()]
    if len(parts) < 2 or "さらに" not in parts[1]:
        return None

    m1 = re.match(r"^(.*?アクティブにしていた場合)、?(.*)$", parts[0])
    if not m1:
        return None
    cond1 = activation_cond(m1.group(1))
    if cond1 is None:
        return None
    effect1_text = m1.group(2).strip().lstrip("、")
    e1 = parse_effect(effect1_text)
    if e1.get("action") in ("custom", "do_nothing", None):
        return None

    second = parts[1]
    if second.startswith("さらに、"):
        second = second[len("さらに、"):]
    elif second.startswith("さらに"):
        second = second[len("さらに"):]
    m2 = re.match(r"^(.*?アクティブにしていた場合)、?(.*)$", second)
    if not m2:
        return None
    cond2 = activation_cond(m2.group(1))
    if cond2 is None:
        return None
    effect2_text = m2.group(2).strip().lstrip("、")
    if effect2_text.startswith("代わりに"):
        effect2_text = effect2_text[len("代わりに"):]
    e2 = parse_effect(effect2_text)
    if e2.get("action") in ("custom", "do_nothing", None):
        return None

    # Tier 2 requires BOTH activations: energy AND member.
    return {
        "text": text,
        "action": "conditional_alternative",
        "condition": cond1,
        "primary_effect": dict(e1, text=effect1_text),
        "alternative_condition": {
            "type": "compound",
            "operator": "and",
            "conditions": [cond1, cond2],
            "text": "さらに、" + m2.group(1),
        },
        "alternative_effect": dict(e2, text=effect2_text),
    }


def _try_energy_ahead_alternative(text):
    """〜置いてもよい。そうしたとき、相手のエネルギーが自分よりN1枚多い場合、E1。
    N2枚以上多い場合、代わりにE2。 — tiered relative-energy alternative
    (bp7-023-L family). The optional move gates the tiered comparison:
    「そうしたとき」 means E1/E2 only apply if the move was made."""
    if "そうしたとき" not in text or "相手のエネルギーが自分より" not in text:
        return None
    pre, sep, rest = text.partition("そうしたとき、")
    if not sep:
        return None
    m = re.search(
        r"相手のエネルギーが自分より(\d+)枚多い場合、([^。]+)。(\d+)枚以上多い場合、代わりに([^。]+)",
        rest,
    )
    if not m:
        return None
    n1, eff1_text, n2, eff2_text = m.group(1), m.group(2), m.group(3), m.group(4)
    move_text = pre.strip().rstrip("。")
    ct, at = split_condition_action(move_text)
    move = parse_action(at or move_text)
    if not isinstance(move, dict):
        return None
    primary = parse_action(eff1_text)
    alternative = parse_action(eff2_text)
    if not isinstance(primary, dict) or not isinstance(alternative, dict):
        return None
    inner = {
        "text": "そうしたとき、" + m.group(0),
        "action": "conditional_alternative",
        "condition": {
            "type": "comparison_condition",
            "comparison_type": "energy_relative",
            "count": as_int(n1),
            "operator": "==",
            "target": "self",
            "text": f"相手のエネルギーが自分より{n1}枚多い場合",
        },
        "primary_effect": primary,
        "alternative_condition": {
            "type": "comparison_condition",
            "comparison_type": "energy_relative",
            "count": as_int(n2),
            "operator": ">=",
            "target": "self",
            "text": f"{n2}枚以上多い場合",
        },
        "alternative_effect": alternative,
    }
    result = {
        "text": text,
        "action": "sequential",
        "actions": [move, inner],
    }
    if ct:
        cond = parse_condition(ct)
        if _is_real_condition(cond):
            result["condition"] = cond
    return result


def _try_cost_set_from_reference(text):
    """「…を選ぶ。ライブ終了時まで、このメンバーのコストは、選んだメンバーが
    元々持つコストよりN低い/高い値に等しくなる。これによりこのカードのコストが
    M以上になった場合、…を得る。」 — cost mirror from a selected member plus an
    optional threshold followup (bp5-005-R 徒町小鈴 family)."""
    key = "のコストは、選んだメンバーが元々持つコストより"
    if key not in text:
        return None
    head, sep, tail = text.partition("これにより")
    m = re.search(
        r"^(?P<sel>[^。]*を選ぶ)。"
        r"(?P<dur>ライブ終了時まで|このターン中)?、?"
        r"この(?:メンバー|カード)のコストは、"
        r"選んだメンバーが元々持つコストより(?P<n>\d+)(?P<dir>低い|高い)値に等しくなる",
        head.strip(),
    )
    if not m:
        return None
    select_part = parse_action(m.group("sel"))
    if not isinstance(select_part, dict):
        return None
    n = as_int(m.group("n"))
    offset = -n if m.group("dir") == "低い" else n
    dur_txt = m.group("dur") or "ライブ終了時まで"
    duration = "live_end" if "ライブ" in dur_txt else "this_turn"
    modify = {
        "action": "modify_cost",
        "operation": "set_from_reference",
        "cost_reference": "previous_moved_card",
        "cost_offset": offset,
        "duration": duration,
        "card_type": "member_card",
        "target": "self",
        "text": m.group(0)[len(m.group("sel")) + 1 :],
    }
    result = {
        "action": "conditional_on_result",
        "primary_effect": {
            "action": "sequential",
            "actions": [select_part, modify],
        },
    }
    if sep:
        mt = re.match(r"このカードのコストが(\d+)以上になった場合、(.*)", tail.strip().rstrip("。"))
        if mt:
            result["result_condition"] = {
                "type": "comparison_condition",
                "comparison_type": "cost",
                # Explicit subject marker: compare THE ACTIVATING CARD's current
                # effective cost. Distinguishes from Rina-style 「合計コストがN」
                # conditions (no location), which sum moved-card costs.
                "location": "activating_card",
                "count": as_int(mt.group(1)),
                "operator": ">=",
                "target": "self",
                "text": f"これによりこのカードのコストが{mt.group(1)}以上になった場合",
            }
            followup = parse_action(mt.group(2))
            if isinstance(followup, dict):
                result["followup_action"] = followup
    return result


# Context a shorthand secondary condition cannot state for itself and has to
# borrow from the main one: a clause like 「2枚以上いる場合」 names no zone, no
# group and no position, so without this the engine would evaluate it against
# the whole board.
#
# The 「両方」 form additionally borrows the main condition's allowed `values`;
# the 「N枚以上」 form does not. The two tuples are listed separately and in
# this order deliberately — they are the insertion order of keys in the
# emitted JSON, so normalising them would change the generated file.
_ALT_INHERITED_KEYS = (
    "location",
    "group_names",
    "target",
    "position",
    "distinct",
    "card_type",
)
_ALT_INHERITED_KEYS_WITH_VALUES = (
    "location",
    "group_names",
    "target",
    "position",
    "values",
    "distinct",
    "card_type",
)


def _inherit_condition_context(result, alt_cond, keys=_ALT_INHERITED_KEYS):
    """Let a shorthand secondary condition borrow context from the main one."""
    main_cond = result.get("condition")
    if not main_cond:
        return
    for key in keys:
        if key in main_cond and key not in alt_cond:
            alt_cond[key] = main_cond[key]


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
                "text": ct,
            }
            # The alternative replaces the original placement, so it lands in
            # the SAME zone. Without an explicit destination the engine's move
            # would have nowhere to put the card.
            alt = result.get("alternative_effect")
            if isinstance(alt, dict) and not alt.get("destination"):
                alt["destination"] = "success_live_zone"
        else:
            cond = parse_condition(ct)
            if _is_real_condition(cond):
                result["condition"] = cond
    if at:
        # Check for secondary condition in action text (e.g. "2枚以上いる場合")
        # This handles the "代わりに" (instead of) pattern where a stricter
        # tiered condition qualifies the alternative effect.
        # Also handle "それらが両方ある場合" (both exist) pattern.
        if "両方" in at:
            sec_text = "それらが両方ある場合"
            alt_cond = parse_condition(sec_text)
            if _is_real_condition(alt_cond):
                _inherit_condition_context(
                    result, alt_cond, _ALT_INHERITED_KEYS_WITH_VALUES
                )
                result["alternative_condition"] = alt_cond
            at = at.replace(sec_text, "").strip().strip("、").strip()
        secondary_m = re.search(r"([^、。]*\d+)枚以上[いあ]る場合", at)
        if secondary_m:
            sec_text = secondary_m.group(0)
            alt_cond = parse_condition(sec_text)
            if _is_real_condition(alt_cond):
                # Inherit location/group/position context from the main condition
                # since the secondary text ("2枚以上ある場合") omits the zone/group
                _inherit_condition_context(result, alt_cond)
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


def _character_gain(eff, resource, count, heart_color=None):
    """A 「「X」N人は…を得る」 clause resolved to a gain_resource action.

    `heart_color` goes directly after `resource` because that is where the
    emitted JSON has always put it; appending it at the end would reorder keys
    in the generated file.
    """
    action = {"action": "gain_resource", "resource": resource}
    if heart_color:
        action["heart_color"] = heart_color
    action.update(
        {
            "count": count,
            "characters": [eff["character"]],
            "card_type": "member_card",
            "target": "self",
            "target_count": eff["count"],
        }
    )
    return action


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
                    "count": as_int(pm.group(2)),
                    "resources": pm.group(3),
                }
            )
    if effects:
        # Build gain_resource actions per character, then group by character
        per_char_actions = []
        for eff in effects:
            char_acts = []
            resources_text = eff["resources"]
            heart_m = re.search(HEART_REF, resources_text)
            blade_count = resources_text.count("icon_blade.png")
            heart_color = f"heart{heart_m.group(1)}" if heart_m else None
            if blade_count > 0:
                char_acts.append(
                    _character_gain(eff, "blade", blade_count)
                )
            if heart_color:
                char_acts.append(
                    _character_gain(eff, "heart", 1, heart_color=heart_color)
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
    if _is_real_condition(cond_parsed):
        result["activation_condition_parsed"] = cond_parsed
    return result


def _make_cost_mod_action(text_part, operation="decrease"):
    """Build a modify_cost action dict from text."""
    a = parse_action(text_part)
    a["action"] = "modify_cost"
    a["operation"] = operation
    ic = text_part.count(ENERGY_ICON)
    if ic > 0:
        a["count"] = ic
    if "グループ名" in text_part and "につき" in text_part:
        a["per_unit"] = True
        a["per_unit_type"] = "group_name"
        cm = re.search(r"(\d+)種類", text_part)
        if cm:
            a["per_unit_count"] = as_int(cm.group(1))
    return a


def _try_cost_modification(text):
    """コストは～につき～減る — cost modification with per-unit scaling.
    Handles: "action。コストは～につき～減る" (sequential) and
    "この能力を起動するためのコストは～につき～減る" (flat modify_cost)."""
    energy_count = text.count(ENERGY_ICON)
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
    value = as_int(value_match.group(1)) if value_match else energy_count
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
        result["per_unit_count"] = as_int(unit_match.group(1))
        result["per_unit_type"] = unit_match.group(2)
        # Detect when per-unit count targets stage members
        # (「ステージにいる...メンバー」) vs the effect's location
        # (e.g. "手札にある" → hand).
        if "ステージ" in text:
            result["per_unit_location"] = "stage"
    return result


def _answer_choice_options(text):
    options = []
    for segment in re.split(r"(?=回答が)", text):
        segment = segment.strip()
        if not segment.startswith("回答が"):
            continue
        index = segment.find("場合、")
        if index == -1:
            continue
        answers_text = segment[len("回答が") : index].strip()
        action_text = segment[index + len("場合、") :].strip().rstrip("。")
        if not answers_text or not action_text:
            continue
        answers = [
            answer.strip().rstrip("の")
            for answer in answers_text.split("か")
            if answer.strip()
        ]
        action = parse_action(action_text)
        action["answers"] = answers
        options.append(action)
    return options


def _set_answer_choice(text, result):
    question_match = re.search(r"(.+?)(?=回答が)", text, re.DOTALL)
    if question_match:
        question = re.sub(r"[\n。]+$", "", question_match.group(1).strip())
        if question:
            result["question"] = question
    result["options"] = _answer_choice_options(text)
    result["choice_maker"] = "opponent"


def _matches_answer_choice(text):
    return "回答が" in text and bool(_answer_choice_options(text))


_try_answer_choice = EffectPattern(
    condition=_matches_answer_choice,
    action="choice",
    defaults={"choice_type": "answer_based"},
    setter=_set_answer_choice,
)


def _finish_each_time(text, trigger_text, sub):
    """Set each_time metadata and merge the 〜たび trigger condition into `sub`."""
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
    if _is_real_condition(trigger_cond):
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


def _try_each_time(text):
    """たび／〜能力が解決したとき — each-time triggers."""
    # Resolution watchers: 「（…メンバーの）X能力が解決したとき/解決するたび」.
    # The trigger clause is everything up to the marker's tense suffix;
    # したとき and するたび are equivalent triggering shapes (any per-turn
    # cap comes from ターン1回, parsed separately).
    if ABILITY_RESOLVE_MARKER in text:
        m = re.search(r"([^。、]*能力が解決(?:したとき|するたび))(?:、|$)", text)
        if m:
            rest = text[m.end() :].strip()
            sub = parse_effect(rest)
            if sub:
                # The trigger clause's positional qualifier (「センターエリアに
                # いる…」) describes WHERE THE RESOLVER SITS, not where the
                # effect moves anything. It belongs in `condition` only
                # (_finish_each_time merges it there); a `position` left on the
                # effect body would be read as a source/destination by
                # position_change execution.
                sub.pop("position", None)
                sub["watches_ability_resolution"] = True
                return _finish_each_time(text, m.group(1).strip(), sub)
    if EACH_TIME_MARKER not in text:
        return None
    tm = re.search(r"([^たび]+)たび", text)
    if not tm:
        return None
    trigger_text = tm.group(1).strip()
    rest = text[tm.end() :].strip().lstrip("、，")
    sub = parse_effect(rest)
    # "〜たび" bodies shaped [optional pay_energy, effect] are
    # conditional_on_optional: the player MAY pay, and paying gates the
    # effect. Reshaped here at the producer rather than as a post-parse FIX
    # block (dissolved FIX 2, formerly in the finalization pipeline).
    acts = sub.get("actions") or []
    if (
        sub.get("action") == "sequential"
        and len(acts) == 2
        and isinstance(acts[0], dict)
        and isinstance(acts[1], dict)
        and acts[0].get("action") == "pay_energy"
        and acts[0].get("optional") is True
    ):
        first, second = acts
        for leak in ("exclude_self", "group_names", "optional"):
            first.pop(leak, None)
        carried = {
            k: v
            for k, v in sub.items()
            if k not in ("text", "action", "actions")
        }
        sub = make_conditional_on_optional(
            text, first, second, **carried
        )
    return _finish_each_time(text, trigger_text, sub)


def _set_opponent_action(text, result):
    match = re.match(r"相手は[、]?(.+?)(?:。|$)", text)
    opponent_text = match.group(0)
    opponent = parse_action(match.group(1).strip())
    opponent["text"] = opponent_text
    opponent["target"] = "opponent"
    opponent["action_by"] = "opponent"
    opponent.pop("condition", None)
    opponent.pop("group_names", None)
    rest = text[len(opponent_text) :].strip()
    if rest:
        result["action"] = "sequential"
        result["actions"] = [opponent, parse_effect(rest)]
    else:
        result.clear()
        result.update(opponent)


def _matches_opponent_action(text):
    return text.startswith("相手は") and re.match(r"相手は[、]?(.+?)(?:。|$)", text)


_try_opponent_action = EffectPattern(
    condition=_matches_opponent_action,
    action="custom",
    setter=_set_opponent_action,
)


def _set_choose_self_opponent(text, result):
    inner = parse_effect(text[len("自分か相手を選ぶ。") :].strip())
    if isinstance(inner, dict):
        inner["action_by"] = "self"
    result["choice_options"] = ["自分", "相手"]
    result["effect_steps"] = [inner]


_try_choose_self_opponent = EffectPattern(
    match="自分か相手を選ぶ。",
    action="choose_target_player",
    setter=_set_choose_self_opponent,
)


def _matches_opponent_after_conditional(text):
    if "、相手は" not in text:
        return False
    parts = text.split("、相手は、", SPLIT_LIMIT)
    return len(parts) == 2 and re.match(r"相手は、(.+?)。", "相手は、" + parts[1])


def _set_opponent_after_conditional(text, result):
    first, second = text.split("、相手は、", SPLIT_LIMIT)
    opponent_text = "相手は、" + second
    match = re.match(r"相手は、(.+?)。", opponent_text)
    first_action = parse_action(first.replace("そうした場合、", "").strip())
    opponent_action = parse_action(match.group(1).strip())
    opponent_action["action_by"] = "opponent"
    actions = [first_action, opponent_action]
    rest = opponent_text[len(match.group(0)) :].strip()
    if rest:
        actions.append(parse_action(rest))
    result["actions"] = actions
    result["conditional"] = True


_try_opponent_after_conditional = EffectPattern(
    condition=_matches_opponent_after_conditional,
    action="sequential",
    setter=_set_opponent_after_conditional,
)


def _set_kore_niyori_case(text, result):
    first, second = text.split("以外の場合", 1)
    condition_part, action_part = first.strip().split("場合、", 1)
    condition_text = "これにより" + condition_part.replace("これにより", "").strip() + "場合"
    primary_text = re.sub(r"『.+』のカード$", "", action_part.strip()).strip()
    condition = parse_condition(condition_text)
    if condition and condition.get("location") == "discard":
        condition["source"] = "preceding_moved"
        condition.pop("location", None)
        condition["negation"] = True
    result["condition"] = condition
    result["primary_effect"] = parse_effect(primary_text)
    result["alternative_effect"] = parse_effect(second.lstrip("、。").strip())


def _matches_kore_niyori_case(text):
    return (
        "これにより" in text
        and "の場合" in text
        and "以外の場合" in text
        and len(text.split("以外の場合", 1)) == 2
        and "場合、" in text.split("以外の場合", 1)[0]
    )


_try_kore_niyori_case = EffectPattern(
    condition=_matches_kore_niyori_case,
    action="conditional_alternative",
    setter=_set_kore_niyori_case,
)


def _apply_card_property_filter(d, text):
    """Extract a card-property filter (e.g. has_blade_heart) from `text`.

    "…を持たない" maps to negation=True (e.g. "ブレードハートを持たない").
    Sets nothing when the dict already carries a card_property.
    """
    if d.get("card_property"):
        return
    if "ブレードハートを持たない" in text:
        d["card_property"] = "has_blade_heart"
        d["negation"] = True
    elif "ブレードハートを持つ" in text:
        d["card_property"] = "has_blade_heart"
    elif HAS_SCORE_ICON in text:
        d["card_property"] = "has_score_icon"


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
    hc = _heart_ref_ids(select_text, unique=True)
    if hc:
        result["heart_colors"] = hc
        if detect_require_all_hearts(select_text):
            result["require_all_heart_colors"] = True
    if extract_max(select_text):
        result["max"] = True
    if extract_optional(select_text):
        result["optional"] = True
    gns = extract_all_groups(select_text)
    if gns:
        result["group_names"] = gns
    _apply_card_property_filter(result, select_text)
    char_names = _quoted_names(select_text)
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
        result["per_group_count"] = as_int(pg.group(1))
    _add_or_card_types_if_needed(result, select_text)
    return result


def _add_or_card_types_if_needed(d, text):
    """「メンバーカードか…ライブカード」 — record or_card_types, drop card_type."""
    if _OR_CARD_TYPE_PAIR_RE.search(text):
        or_types = _or_card_types_in(text)
        if or_types:
            d["or_card_types"] = or_types
            d.pop("card_type", None)


def _add_heart_color_threshold(d, text):
    """Extract heart color count threshold from patterns like 'heart05を2個以上' or 'heart05を2以上'.
    Stores as heart_color_count on the dict."""
    if "heart_color_count" in d or "heart_colors" not in d:
        return
    m = re.search(rf"{HEART_ICON_ID}を(\d+)(?:個)?以上", text)
    if m:
        count = as_int(m.group(2))
        if count > 0:
            d["heart_color_count"] = count


# 「{{icon.png|…}}能力を持つ」 — the trigger icon an ability-filter clause pins
# the filter to. The two forms differ: the positive one wants the icon's FILE
# name, the negative ones the text label printed beside it.
_ABILITY_ICON_NAME_RE = re.compile(r"\{\{(\w+)\.png\|")
_ABILITY_ICON_LABEL_RE = re.compile(r"\{\{([^}]+?)\.png\|([^}]+?)\}\}能力")
_ABILITY_ICON_LABEL_ALL_RE = re.compile(r"\{\{([^}]+?)\.png\|([^}]+?)\}\}能力も")


def _apply_ability_filter(d, text):
    """Read an 「…能力を持つ / 能力を持たない」 ability filter out of the text.

    The filter names a kind of ability and, when a trigger icon pins it, which
    trigger types that kind is allowed to have. 「能力も持たない」 lists several
    icons, so its filter is satisfied by any of them; an unadorned
    「能力を持たない」 is a plain no-ability filter with no trigger restriction.
    """
    if "能力を持つ" in text and "能力を持たない" not in text:
        d["ability_filter"] = "has_ability"
        icon = _ABILITY_ICON_NAME_RE.search(text)
        if icon:
            d["ability_filter_triggers"] = [icon.group(1)]
        return
    if "能力を持たない" not in text and "能力も持たない" not in text:
        return
    labelled = _ABILITY_ICON_LABEL_RE.search(text)
    if labelled:
        d["ability_filter"] = "no_ability_type"
        d["ability_filter_triggers"] = [labelled.group(2)]
        return
    if "能力も" in text:
        d["ability_filter"] = "no_ability_type"
        triggers = [label for _, label in _ABILITY_ICON_LABEL_ALL_RE.findall(text)]
        if triggers:
            d["ability_filter_triggers"] = triggers
        return
    d["ability_filter"] = "no_ability"


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
    hc = _heart_ref_ids(text, unique=True)
    if hc:
        d["heart_colors"] = hc
        _add_heart_color_threshold(d, text)
        if detect_require_all_hearts(text):
            d["require_all_heart_colors"] = True
    if extract_optional(text):
        d["optional"] = True
    gns = extract_all_groups(text)
    if gns:
        d["group_names"] = gns
    # Extract 「」-bracketed character names (e.g. 「朝香果林」のメンバーカード)
    char_names = _quoted_names(text)
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
    _apply_ability_filter(d, text)


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
        gns = extract_all_groups(p)
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


# 「好きな枚数を好きな順番で…」 — take any number, in any order, onto the deck.
# Each row is the pair of phrases that identify the shape, the destination the
# selected cards go to, and any extra fields the shape needs (non-empty only
# for the one that routes the rest somewhere other than the discard). Tested in
# order, first match wins.
_ANY_NUMBER_DECK_MODES = (
    (
        ("好きな枚数を好きな順番でデッキの上に置き", "残りを控え室に置く"),
        "deck_top",
        {},
    ),
    (
        # C7 黒澤ダイヤ ab#1
        ("好きな枚数を好きな順番でデッキの下に置き", "残りを控え室に置く"),
        "deck_bottom",
        {},
    ),
    (
        # C8: to the top, the rest to the bottom (小原鞠莉 ab#0)
        (
            "好きな枚数を好きな順番でデッキの上に置き",
            "残りを好きな順番でデッキの下に置く",
        ),
        "deck_top",
        {
            "remainder_destination": "deck_bottom",
            "remainder_placement_order": "any_order",
        },
    ),
)

# Where a selected card goes when the text does not spell it out. deck_top is
# tested first because a select clause usually names BOTH 「デッキの上に置く」 (the
# selected card) and 「残りを控え室に置く」 (the rest), and the first is the one
# that describes the selection.
_SELECT_DESTINATION_PHRASES = (
    (("デッキの上に置く", "デッキの上に", "デッキの一番上に"), "deck_top"),
    (("手札に加える", "手札に加え"), "hand"),
    (("控え室に置く",), "discard"),
)


def _try_any_number_to_deck(result, select_text):
    """Fill in the 「好きな枚数を好きな…」 shape. True when it matched."""
    for phrases, destination, extra in _ANY_NUMBER_DECK_MODES:
        if not all(phrase in select_text for phrase in phrases):
            continue
        result["destination"] = destination
        result["placement_order"] = "any_order"
        result["any_number"] = True
        result["reveal"] = False
        if extra:
            result.pop("discard_remaining", None)
            result.update(extra)
        return True
    return False


def _fill_select_count_and_type(result, text):
    """The card count and card type a select clause states."""
    count = extract_count(text)
    if count:
        result["count"] = count
    card_type = extract_card_type(text)
    if card_type:
        result["card_type"] = card_type


def _build_look_select_actions_inner(select_text):
    """Build the select_action for その中から patterns."""
    result = {"action": "select_cards", "discard_remaining": True}
    result["text"] = select_text

    # Pattern: reveal → add → discard
    if "手札に加え" in select_text and "残りを控え室に置く" in select_text:
        parts = re.split(r"[、。]", select_text)
        if len(parts) >= 2:
            first_part = parts[0].strip()
            if "公開し" in first_part:
                revealed = _build_reveal_add_discard(
                    first_part, parts[1].strip(), select_text
                )
                if revealed:
                    return revealed
            if "公開し" not in first_part:
                result["destination"] = "hand"
                _fill_select_count_and_type(result, select_text)
                heart_colors = _heart_ref_ids(select_text, unique=True)
                if heart_colors:
                    result["heart_colors"] = heart_colors
                _add_heart_color_threshold(result, select_text)
                _add_or_card_types_if_needed(result, select_text)
                _enrich_from_text(result, select_text)
                _apply_card_property_filter(result, select_text)
                if extract_optional(select_text):
                    result["optional"] = True
                # 「カードを…エリアに登場させるか、手札に加える」 — an either-or
                # between appearing and adding.
                if "登場させる" in select_text:
                    followup = _build_or_destination_followup(select_text)
                    if followup:
                        result["followup_action"] = followup
                return result

    if _try_any_number_to_deck(result, select_text):
        return result

    # Issue 12: hand + deck_top remainder ("1枚を手札に加え、残りをデッキの上に戻す")
    if "手札に加え" in select_text and "残りをデッキの上" in select_text:
        result["destination"] = "hand"
        result["reveal"] = False
        result.pop("discard_remaining", None)
        result["remainder_destination"] = "deck_top"
        _fill_select_count_and_type(result, select_text)
        _enrich_from_text(result, select_text)
        _add_or_card_types_if_needed(result, select_text)
        return result

    # Default: detect destination from text
    result["reveal"] = False
    for phrases, destination in _SELECT_DESTINATION_PHRASES:
        if any(phrase in select_text for phrase in phrases):
            result["destination"] = destination
            break

    # Propagate selection criteria
    _enrich_from_text(result, select_text)

    # A heart-coloured clause with no stated destination is a hand gain.
    if result.get("destination") is None and (
        "{{heart_" in select_text or "ハートに" in select_text
    ):
        result["destination"] = "hand"

    _apply_card_property_filter(result, select_text)

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
    reveal_count = as_int(reveal_m.group(1))
    # Parse the "合計N枚含まれる場合" condition from the before-text
    # "公開されたカードの中に...合計N枚含まれる場合"
    heart_match_condition = None
    cond_m = re.search(
        r"公開されたカードの中に.+?合計(\d+)枚含まれる場合",
        before,
    )
    if cond_m:
        cond_count = as_int(cond_m.group(1))
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
    has_blade = BLADE_ICON in after or "ブレード" in after
    blade_count = after.count(BLADE_ICON)
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
        if select_actions.get("card_property"):
            sel["card_property"] = select_actions["card_property"]
        if select_actions.get("negation") is not None:
            sel["negation"] = select_actions["negation"]
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


# Zones a look_action reads from. The stage and the hand are never named as a
# look source, so a condition mentioning them does not say where to look.
_LOOK_SOURCE_EXCLUDED_ZONES = ("stage", "hand")

# 「その後」 — the clause after it runs once the look_and_select has resolved.
_SONOGO_SPLIT_RE = re.compile(r"[。、]?\s*その後[、。]?\s*")

# A trailing 「。N以上の場合、…」 / 「。そうした場合、…」 is a followup action,
# not part of the select filter.
_COND_FOLLOWUP_RE = re.compile(r"[。]\s*(?:(\d+以上の場合、)|(そうした場合、))")

# A shorthand followup condition ("30以上の場合") names no comparison type,
# group, location, card type or aggregate, so it borrows them from the parent.
_FOLLOWUP_INHERITED_FIELDS = (
    "comparison_type",
    "group_names",
    "location",
    "card_type",
    "aggregate",
)


def _look_action_for(result, text):
    """Build the look half of a 「…その中から…」 effect.

    A condition naming a zone becomes the look source, so the engine looks
    where the condition was about rather than at a default (「ライブカード置き場に
    カードが2枚以上ある場合、その从中から…」 looks at the live-card zone). A
    condition with no zone, or an unparseable look clause, falls back to
    whatever the look text parses to on its own.
    """
    prefix = re.search(r"(.+?)その中から", text)
    if not prefix:
        return
    cond_text, action_text = split_condition_action(prefix.group(1).strip())
    cond = parse_condition(cond_text) if cond_text else None
    if _is_real_condition(cond):
        result["condition"] = cond
        zone = cond.get("location")
        if zone and zone not in _LOOK_SOURCE_EXCLUDED_ZONES:
            if action_text:
                parsed = parse_action(action_text)
                if _is_real_action(parsed):
                    parsed.setdefault("source", zone)
                    result["look_action"] = parsed
            else:
                look = {"action": "look_at", "source": zone, "target": "self"}
                if zone == "live_card_zone":
                    look["all"] = True
                result["look_action"] = look
    if "look_action" not in result and action_text:
        parsed = parse_action(action_text)
        if _is_real_action(parsed):
            result["look_action"] = parsed


def _split_cond_followup(select_text):
    """Peel a trailing conditional clause off the select text.

    Returns (select_text, followup_text_or_None).
    """
    split = _COND_FOLLOWUP_RE.search(select_text)
    if not split:
        return select_text, None
    followup = select_text[split.start() + 1 :].strip()  # skip the period
    if followup.startswith("そうした場合、"):
        followup = followup[len("そうした場合、") :].strip()
    return select_text[: split.start()].strip(), followup


def _build_select_with_followup(result, select_text):
    """Build the select half, plus any 「その後」 followup it names."""
    parts = _SONOGO_SPLIT_RE.split(select_text, maxsplit=1)
    if len(parts) == 1:
        _build_look_select_with_followup(select_text, result)
        return
    _build_look_select_with_followup(parts[0].strip(), result)
    followup_text = parts[1].strip()
    if not followup_text:
        return
    parsed = parse_effect(followup_text)
    if parsed:
        result["followup_action"] = parsed


def _apply_conditional_followup(result, cond_followup):
    """Attach a trailing 「…の場合、…」 clause as the followup action."""
    if not cond_followup:
        return
    parsed = parse_effect(cond_followup)
    if not parsed or parsed.get("action") == "custom":
        return
    parent_cond = result.get("condition")
    if isinstance(parent_cond, dict):
        follow_cond = parsed.get("condition") or {}
        if follow_cond.get("type") == "comparison_condition":
            for field in _FOLLOWUP_INHERITED_FIELDS:
                if field in parent_cond and field not in follow_cond:
                    follow_cond[field] = parent_cond[field]
            # cost_total is special: it comes from the followup's own count (the
            # threshold in "N以上"), not from the parent's.
            if "cost_total" in parent_cond and "cost_total" not in follow_cond:
                follow_cond["cost_total"] = follow_cond.get(
                    "count", parent_cond["cost_total"]
                )
    # A その後 followup may already be there, so the conditional runs after it.
    existing = result.get("followup_action")
    result["followup_action"] = (
        {"action": "sequential", "actions": [existing, parsed]}
        if existing
        else parsed
    )


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
    _look_action_for(result, text)
    select_clause = re.search(r"その中から(.+)", text)
    if select_clause:
        # Issue 12: split off a trailing period-separated conditional
        # ("...戻す。N以上の場合、さらに...") so it becomes a separate followup
        # action rather than part of the select filter.
        select_text, cond_followup = _split_cond_followup(
            select_clause.group(1).strip()
        )
        _build_select_with_followup(result, select_text)
        _apply_conditional_followup(result, cond_followup)
    return result


def _set_reveal_until_chosen_card(text, result):
    cost_match = re.search(r"コスト(\d+)以上", text)
    first = {
        "action": "select",
        "or_card_types": ["live_card", "member_card"],
        "count": 1,
        "all": False,
    }
    if cost_match:
        first["cost_limit"] = as_int(cost_match.group(1))
        first["cost_limit_operator"] = ">="
    result["actions"] = [
        first,
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
    ]


_try_reveal_until_chosen_card = EffectPattern(
    condition=lambda t: "ライブカードか" in t
    and "メンバーカードのどちらか" in t
    and "選んだカードが公開されるまで" in t,
    action="sequential",
    setter=_set_reveal_until_chosen_card,
)


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
    other_count = as_int(tc_match.group(1)) if tc_match else 1
    # Extract group names from the other-target part
    other_groups = extract_all_groups(other_part)
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


def _set_reveal_until_live(text, result):
    result["actions"] = [
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
    ]


_try_reveal_until_live = EffectPattern(
    match="ライブカードが公開されるまで",
    action="sequential",
    setter=_set_reveal_until_live,
)


def _set_furthermore(text, result):
    actions = []
    for part in _split_sentences_nesting(text):
        part = part.strip()
        if not part:
            continue
        if "さらに" in part:
            part = part.replace("さらに", "", 1).strip()
        actions.append(parse_effect(part))
    result["actions"] = actions


def _matches_furthermore(text):
    parts = _split_sentences_nesting(text)
    return "さらに" in text and len(parts) >= 2 and any(
        "さらに" in part for part in parts[1:]
    )


_try_furthermore = EffectPattern(
    condition=_matches_furthermore,
    action="sequential",
    setter=_set_furthermore,
)


def _matches_sequential_duration(text):
    return (
        "その後、" in text
        and "かぎり、" in text
        and len(text.split("その後、", SPLIT_LIMIT)) == 2
        and "かぎり、" in text.split("その後、", SPLIT_LIMIT)[1]
    )


def _set_sequential_duration(text, result):
    first, second = text.split("その後、", SPLIT_LIMIT)
    condition_text, action_text = second.strip().split("かぎり、", 1)
    second_action = parse_action(action_text.strip())
    second_action["condition"] = parse_condition(condition_text.strip())
    second_action["duration"] = "unless"
    result["actions"] = [parse_action(first.strip()), second_action]


_try_sequential_duration = EffectPattern(
    condition=_matches_sequential_duration,
    action="sequential",
    setter=_set_sequential_duration,
)


def _try_compound_select(text):
    """Aのうちのメンバー1人と、これにより選んだメンバー以外のBのメンバー1人は — compound selection."""
    if "のうちの" not in text or "これにより選んだメンバー以外" not in text:
        return None
    # Extract character names from the first group 「」
    char_names = _quoted_names(text)
    if len(char_names) < 2:
        return None
    # Extract second group from 『』
    group_names = _quoted_group_names(text)
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
    if _is_real_action(gain):
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
    # Prefer 。as separator when present (sentence boundaries).
    # _split_sentences_nesting handles 「」/（）/{{}} natively — no \x00 tricks.
    if "。" in text:
        # Drop segments that are only parenthetical notes
        # (e.g. （対戦相手のカードの効果でも発動する）) so they don't become do_nothing.
        parts = [p for p in _split_sentences_nesting(text) if strip_parenthetical(p)]
    else:
        parts = [p for p in text.split("、") if p.strip()]
    # A duration phrase stranded as its own segment is not an effect; it states
    # the duration of the one that follows it.
    parts = _fold_standalone_duration(parts)
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
        if _parsed_to_something(a) and a.get("action") != "do_nothing":
            actions.append(a)
    if len(actions) >= 2:
        return {"text": text, "action": "sequential", "actions": actions}
    return None


def _split_select_and_energy_payment(fa, fp, restate_text):
    """Split a leading 「select, pay E」 pair into a select plus a payment step.

    「ライブカードを1枚選び、そのカードのスコアに等しい数のEを支払ってもよい」
    reaches the engine as one clause, but it has to run as two steps.

    `restate_text` mirrors an existing asymmetry and must be passed in rather
    than decided here: when the clause carried its own condition the re-parsed
    select is given an explicit `text`, and when it did not, it is not.
    """
    if fa.get("action") != "select" or ENERGY_ICON not in fp:
        return fa, None
    if "支払う" not in fp and "支払って" not in fp:
        return fa, None
    segments = fp.split("、")
    if len(segments) < 2 or "選び" not in segments[0] or ENERGY_ICON not in segments[1]:
        return fa, None
    payment = parse_action(segments[1])
    if payment.get("action") != "pay_energy":
        # Not a payment after all — keep the select unsplit.
        return fa, None
    select_text = segments[0] + "、"
    select = parse_action(select_text)
    if restate_text:
        select["text"] = select_text
    return select, payment


# 「…を選び、デッキの一番上に置いてもよい」 — the select clause names where the
# chosen card goes, so the engine needs an explicit move between the select and
# the followup. Each row is (phrase, destination, placement_order or None).
# First match wins.
_SELECT_STATED_DESTINATIONS = (
    ("デッキの一番上", "deck_top", None),
    ("デッキの下", "deck_bottom", "any_order"),
)


def _select_move_step(fa, fp):
    """The move step a select-with-stated-destination needs, or None."""
    if fa.get("action") != "select" or ("置く" not in fp and "置いて" not in fp):
        return None
    for phrase, destination, placement_order in _SELECT_STATED_DESTINATIONS:
        if phrase in fp:
            fa["destination"] = destination
            if placement_order:
                fa.setdefault("placement_order", placement_order)
            break
    if not fa.get("destination"):
        return None
    return {
        "action": "move_cards",
        "source": "selected_cards",
        "destination": fa.pop("destination"),
        "count": 0,
        "all": True,
        "target": fa.get("target", "self"),
    }


def _try_conditional_sequential(text):
    """そうした場合 — conditional sequential actions."""
    if CONDITIONAL_SEQUENTIAL_MARKER not in text:
        return None
    parts = _split_marker_depth0(text, CONDITIONAL_SEQUENTIAL_MARKER)
    if parts is None:
        return None
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
    fa, middle_pay = _split_select_and_energy_payment(fa, fp, bool(fc and fat))

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
                # Stamp the selection reference only when the follow-up clause
                # does NOT scope its own objects with an existence qualifier.
                # A clause like 「そうした場合、相手のステージにいる…メンバー
                # 1人をウェイトにする」 (百生吟子 PL!HS-PR-035-PR) resolves
                # against the members IN that zone; forcing
                # source="selected_cards" made the engine look for the chosen
                # cards on the stage (they were just placed under the deck)
                # and silently no-op. Backreference verbs like
                # 「そのライブカードを手札に加える」 (桜坂しずく
                # PL!N-bp5-003-R) must keep the stamp, so destination phrases
                # (手札に加える) do NOT count — only 〜にいる／〜にある do.
                names_own_zone = any(
                    k in clean
                    for k in (
                        "ステージにい",
                        "控え室にある",
                        "控え室にい",
                        "デッキにある",
                        "手札にある",
                    )
                )
                if not names_own_zone:
                    sa["source"] = "selected_cards"

    # For select actions with an explicit destination in the first-part text
    # (e.g. "選び、デッキの一番上に置いてもよい"), insert a move_cards step
    # between the select and the followup so the selected card actually moves.
    move_step = _select_move_step(fa, fp)

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
    (moved from position 17 to position 12 in _STRUCTURAL_EFFECT_RULES)."""
    if SEQUENTIAL_MARKER not in text:
        return None
    parts = _split_marker_depth0(text, SEQUENTIAL_MARKER)
    if parts is None:
        return None
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
        # The raw modifier sentence is deliberately NOT emitted: the
        # structured choice_condition / alternative_condition /
        # alternative_count_type fields below carry everything the engine's
        # tiered-choice evaluation reads.
        cond = parse_condition(cond_mod)
        cond = _resolve_preceding_moved_condition(cond, cond_mod)
        if _is_real_condition(cond):
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
            if _is_real_condition(alt_cond):
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


def _set_kore_niyori_cascade(text, result):
    match = re.search(r"^(.*?)。これにより(.+?)場合、(.+)$", text, re.DOTALL)
    if not match:
        return
    action_text, cond_text, result_text = (
        match.group(1).strip(),
        match.group(2).strip(),
        match.group(3).strip(),
    )
    actions = [parse_action(part) for part in action_text.split("。") if part.strip()]
    if not actions:
        return
    follow = {"condition": parse_condition(cond_text + "場合")}
    follow.update(parse_effect(result_text))
    actions.append(follow)
    result["actions"] = actions


def _matches_kore_niyori_cascade(text):
    match = re.search(r"^(.*?)。これにより(.+?)場合、(.+)$", text, re.DOTALL)
    return bool(match and any(part.strip() for part in match.group(1).split("。")))


_try_kore_niyori_cascade = EffectPattern(
    condition=_matches_kore_niyori_cascade,
    action="sequential",
    setter=_set_kore_niyori_cascade,
)


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
    parts = _split_sentences_nesting(text)
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
        if _parsed_to_something(fa):
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
        cm = re.search(COUNT_PATTERN, at)
        cnt = as_int(cm.group(1)) if cm else None
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

    # "それ" / "これによって" follow-ups — the action targets the SPECIFIC card
    # the preceding sequential step moved, not any discard card. The condition
    # is a character/location check on `preceding_moved`; mirror that source on
    # the move so "それを手札に加える" moves THAT card (e.g. 小原鞠莉 ab#1
    # "それが「松浦果南」か「黒澤ダイヤ」の場合、それを手札に加える").
    if (
        action.get("action") == "move_cards"
        and action.get("source") == "discard"
        and (cond.get("source") == "preceding_moved" or at.startswith("それを手札に"))
    ):
        action["source"] = "preceding_moved"

    # Handle "条件Aの場合、または条件Bの場合、行動" — merge into OR condition
    if action.get("condition") and at.lstrip().startswith("または"):
        cond = {
            "type": "or_condition",
        "operator": "or",
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
                    if _is_real_action(pa):
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
    # "そのカード/そのメンバーの…能力を発動させる" / "それらが持つ…能力を発動させる" — the
    # fired ability belongs to the card(s) selected by the preceding `select` step.
    elif "その" in target_raw or "それら" in target_raw:
        result["source_card"] = "previous_selected"
    # The ability-reference phrase ("そのカードの{{toujyou.png|登場}}能力") is
    # deliberately NOT stored in `target`: `target` is a player-target field,
    # and stuffing prose into it leaked icon markup through decode and made
    # generic player-resolution fall through to player1. The trigger is
    # extracted into `target_trigger` below; the raw text remains in `text`.
    tm = re.search(r"\{\{(.+?)\}\}", target_raw)
    if tm:
        trigger_raw = tm.group(1)
        if "|" in trigger_raw:
            trigger_raw = trigger_raw.split("|")[1]
        result["target_trigger"] = trigger_raw
        result["ability_text"] = "%s_ability" % trigger_raw
    else:
        for kw in ("登場", "ライブ開始時", "ライブ成功時", "起動", "常時"):
            if kw in target_raw:
                result["target_trigger"] = kw
                result["ability_text"] = "%s_ability" % kw
                break
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
        action.get("action") == "move_cards"
        and not action.get("source")
        and "このカード" in action_text
        and "そのバトンタッチで登場した" in action_text
        and "下に置く" in action_text
    ):
        action["source"] = "discard"
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


PLACEMENT_MARKERS = ("置いた", "置かれ", "置いて", "置く")


def _fix_placement_condition(cond, cp):
    """Issue 11 helper: placement-referencing conditions (置いた/置かれ/置いて/置く)
    refer to preceding_moved, not the whole zone. For compound (AかつB) only the
    placement half is fixed (e.g. Burn!! first half energy_card置いており→preceding_moved,
    second half total>=10 stays energy_zone)."""
    if not cond or not any(m in cp for m in PLACEMENT_MARKERS):
        return cond
    if cond.get("type") == "compound" and cond.get("conditions"):
        for sub in cond["conditions"]:
            sub_text = sub.get("text", "")
            if any(m in sub_text for m in PLACEMENT_MARKERS):
                sub["source"] = "preceding_moved"
                sub.pop("location", None)
                if sub.get("type") in ("location_condition",):
                    sub["type"] = "card_count_condition"
        cond.pop("location", None)
        cond.pop("source", None)
    else:
        cond["source"] = "preceding_moved"
        cond.pop("location", None)
        if cond.get("type") in ("location_condition",):
            cond["type"] = "card_count_condition"
    return cond


def _try_kore_niyori_result(text):
    """これにより/これによって～した場合/とき — conditional on result (invalidation follow-up, discard follow-up, etc.)."""
    marker = None
    for m in ("これにより", "これによって", "この効果によって"):
        if m in text:
            marker = m
            break
    if marker is None:
        return None
    # If the text contains a choice marker (以下から1つを選ぶ), the choice handler
    # should process this instead — this is a "choose from options with conditional
    # count upgrade" pattern, not a "do X, then if result condition Y, do Z" pattern.
    if CHOICE_MARKER in text:
        return None
    # Support both 場合 and とき as condition markers
    cond_marker = None
    for m in ["場合", "とき"]:
        mm = re.search(re.escape(marker) + r"(.+?)" + m, text)
        if mm:
            cond_marker = m
            break
    if cond_marker is None:
        return None
    m = re.search(re.escape(marker) + r"(.+?)" + cond_marker, text)
    if not m:
        return None
    parts = text.split(marker, 1)
    sp = marker + parts[1].strip()
    if cond_marker not in sp:
        return None
    cp, fp = sp.split(cond_marker, 1)
    cond_raw = cp.strip() + cond_marker
    cond = parse_condition(cond_raw)
    # Infer location from context: "公開された" means revealed_cards
    if cond and "公開された" in cp and not cond.get("location"):
        cond["location"] = "revealed_cards"
    cond = _fix_placement_condition(cond, cp)
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
    # Result-condition property enrichment, done at the producer rather than
    # as a post-parse backfill (dissolved FIX 9, formerly in the finalization pipeline).
    if isinstance(cond, dict) and not cond.get("card_property"):
        cond_text = cond.get("text", "")
        if "ブレードハート" in cond_text:
            cond["card_property"] = "has_blade_heart"
            if "持たない" in cond_text or "ない" in cond_text:
                cond["negation"] = True
        if HAS_SCORE_ICON in cond_text:
            cond["card_property"] = "has_score_icon"
        _infer_heart_source(cond, cond_text)
        _infer_baton_touch(cond, cond_text)
    primary_text = parts[0].strip()
    # Skip empty/trivial primary text (e.g. cost text already consumed, or just bracket fragments)
    if not primary_text or re.match(r"^[\s）」）』」、。]*$", primary_text):
        return None
    primary = parse_action(primary_text)
    # If the primary carries a leading gate condition ("…場合、"/"…とき、"), only
    # parse_effect attaches it to the action (e.g. 果南 ab#0 "…のみがいる場合、
    # フォーメーションチェンジしてもよい"). For plain primaries (no gate) keep
    # parse_action's exact structure so other conditional_on_result abilities
    # (e.g. c8 百生吟子) are unchanged.
    if re.search(r"(?:場合|とき)、", primary_text):
        pe = parse_effect(primary_text)
        if isinstance(pe, dict):
            primary = pe
    followup = parse_effect(fp.strip())
    # "このメンバー" in a これにより followup acts on the activating card
    # itself. Derived at the producer rather than as a post-parse backfill
    # (dissolved FIX 9b, formerly in the finalization pipeline).
    if isinstance(followup, dict) and "このメンバー" in followup.get("text", ""):
        if not followup.get("target") and followup.get("self_target") is None:
            followup["self_target"] = True
        # For change_state, also set self_cost so the Rust handler restricts
        # targets to the activating card and skips when the card is already in
        # the target state (e.g. already wait).
        if (
            followup.get("action") == "change_state"
            and followup.get("self_cost") is None
        ):
            followup["self_cost"] = True
    return {
        "text": text,
        "action": "conditional_on_result",
        "primary_effect": primary,
        "result_condition": cond,
        "followup_action": followup,
    }


def _set_sou_shinakatta(text, result):
    parts = _split_marker_depth0(text, "そうしなかった場合")
    if parts is None:
        return
    optional_text = parts[0].strip()
    first = parse_action(optional_text)
    if optional_text.startswith("相手は"):
        first["target"] = "opponent"
    alternative = parse_effect(parts[1].strip().lstrip("、"))
    result.update(make_conditional_on_optional(text, first, alternative, negation=True))


def _matches_sou_shinakatta(text):
    return "そうしなかった場合" in text and _split_marker_depth0(
        text, "そうしなかった場合"
    ) is not None


_try_sou_shinakatta = EffectPattern(
    condition=_matches_sou_shinakatta,
    action="conditional_on_optional",
    setter=_set_sou_shinakatta,
)


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
    if ENERGY_ICON in unless_text:
        ec = unless_text.count(ENERGY_ICON)
        fa = {"action": "pay_energy", "energy": ec, "count": ec, "target": "self"}
        aa = parse_effect(eff_text)
        return make_conditional_on_optional(
            text, fa, aa, negation=True
        )

    # Non-energy unless: detect "手札をN枚控え室に置かないかぎり" pattern
    # (unless you discard N from hand to waitroom). The split leaves the
    # imperfective stem (e.g. "置か" from 置かない), so we reconstruct
    # the action manually.
    m = re.search(r"手札を(\d+)枚控え室に置", unless_text)
    if m:
        count = as_int(m.group(1))
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
        if aa.get("card_type") == "energy_card" and aa.get("destination") in ("deck", "energy_deck"):
            aa["source"] = "energy_zone"
            aa["destination"] = "energy_deck"
        if _is_real_action(aa):
            return make_conditional_on_optional(
                text, fa, aa, negation=True
            )

    return None


# 「Aし、B」 and 「Aを得て、B」 — two effects joined by a connector. The
# matcher has to parse both legs to decide whether this is the pattern, and
# the setter needs the very same two parses, so each connector gets one
# splitter instead of the work being written twice.
_SHI_SEQUENTIAL_MARKER = "し、"
_TE_SEQUENTIAL_MARKER = "を得て、"

# A leg that parses to one of these carries no effect, so the connector was
# not joining two effects after all.
_INERT_ACTIONS = ("custom", "do_nothing")


def _split_shi_sequential(text):
    """Both parsed legs of 「Aし、B」, or None when either leg is inert."""
    index = text.find(_SHI_SEQUENTIAL_MARKER)
    if index < 0:
        return None
    legs = [
        parse_effect(text[: index + 1]),
        parse_effect(text[index + 2 :].strip().lstrip("、")),
    ]
    if any(leg.get("action", "custom") in _INERT_ACTIONS for leg in legs):
        return None
    return legs


def _matches_shi_sequential(text):
    if _SHI_SEQUENTIAL_MARKER not in text or CHOICE_MARKER in text:
        return False
    if any(marker in text for marker in CONDITION_MARKERS):
        return False
    return _split_shi_sequential(text) is not None


def _set_shi_sequential(text, result):
    result["actions"] = _split_shi_sequential(text)


_try_shi_sequential = EffectPattern(
    condition=_matches_shi_sequential,
    action="sequential",
    setter=_set_shi_sequential,
)


def _split_te_sequential(text):
    """Both parsed legs of 「Aを得て、B」, or None when either leg is inert.

    The left leg lost its verb to the connector, so 「…を得て」 becomes
    「…を得る」 before parsing.
    """
    if _TE_SEQUENTIAL_MARKER not in text:
        return None
    left, right = text.split(_TE_SEQUENTIAL_MARKER, 1)
    legs = [
        parse_action(left.strip() + "を得る"),
        parse_action(right.strip()),
    ]
    if any(leg.get("action") == "custom" for leg in legs):
        return None
    return legs


def _matches_te_sequential(text):
    if _TE_SEQUENTIAL_MARKER not in text or "を得る" not in text:
        return False
    return _split_te_sequential(text) is not None


def _set_te_sequential(text, result):
    result["actions"] = _split_te_sequential(text)


_try_te_sequential = EffectPattern(
    condition=_matches_te_sequential,
    action="sequential",
    setter=_set_te_sequential,
)


def _set_global_modifier_fields(text, result):
    result["operation"] = "increase" if "多くなる" in text else "decrease"
    target_match = re.search(r"([^は]+)は", text)
    if target_match:
        raw_target = target_match.group(1).strip()
        result["target"] = "opponent" if "相手の" in raw_target else raw_target
    if "すべて" in text:
        result["all"] = True
    heart_match = re.search(HEART_ICON_ID, text)
    if heart_match:
        result["heart_colors"] = [f"heart{heart_match.group(1).zfill(2)}"]
    value_match = re.search(r"(\d+)つ多", text)
    result["value"] = as_int(value_match.group(1)) if value_match else 1


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


def _set_blade_conversion(text, result):
    match = re.search(r"すべて\[([^\]]+)\]", text)
    if not match:
        return
    result["blade_type"] = match.group(1)
    if "ライブ終了時まで" in text:
        result["duration"] = "live_end"


def _set_blade_equal_gain(text, result):
    result["resource"] = "blade"
    icon_count = text.count(BLADE_ICON)
    if icon_count:
        result["count"] = icon_count


def _set_blade_same_thing_gain(text, result):
    result["resource"] = "blade"
    icon_count = text.count(BLADE_ICON)
    result["count"] = icon_count or 1
    if "ライブ終了時まで" in text:
        result["duration"] = "live_end"


def _set_blade_count_set(text, result):
    normalized = re.sub(r"\s+", "", text)
    pattern_text = re.sub(r"\{\{[^|]+\|([^}]+)\}\}", r"\1", normalized)
    match = re.search(r"(\d+)つになる", pattern_text) or re.search(
        r"(\d+)になる", pattern_text
    )
    if match:
        result["count"] = as_int(match.group(1))
    if "ライブ終了時まで" in pattern_text:
        result["duration"] = "live_end"


def _set_restriction_fields(text, result):
    has_wait = "ウェイトしない" in text
    has_active = "アクティブにならない" in text or "アクティブにしない" in text
    result["restriction_type"] = (
        "cannot_wait_by_effect"
        if has_wait and "効果によっては" in text
        else "cannot_activate_by_effect"
        if has_active and "効果によっては" in text
        else "cannot_wait"
        if has_wait
        else "cannot_activate"
    )
    if "アクティブフェイズ" in text:
        result["phase"] = "active_phase"
    if "自分と相手の" in text:
        result["target"] = "both"
    elif "自分の" in text:
        result["target"] = "self"
    elif "相手の" in text:
        result["target"] = "opponent"
    if "このターン" in text:
        result["duration"] = "this_turn"
    if "ライブ終了時まで" in text or "ライブ終了まで" in text:
        result["duration"] = "live_end"
    if "メンバー" in text:
        result["card_type"] = "member_card"
    if "エネルギー" in text:
        result["card_type"] = "energy_card"
    if has_wait:
        blade_limit = extract_blade_limit(text)
        if blade_limit:
            result.update(blade_limit)
        group_names = extract_all_groups(text)
        if group_names:
            result["group_names"] = group_names
        if "元々" in text:
            result["original_value"] = True


def _set_both_discard_until(text, result):
    first_text, second_text = re.split(r"その後[、。]?", text, maxsplit=1)
    first = {
        "text": first_text.strip(),
        "action": "discard_until_count",
        "target": "both",
        "multiple_targets": True,
    }
    count_match = re.search(r"(\d+)枚になるまで", first["text"])
    if count_match:
        first["target_count"] = as_int(count_match.group(1))
    result["target"] = "both"
    result["multiple_targets"] = True
    result["actions"] = [first, parse_effect(second_text.strip())]


def _matches_both_discard_until(text):
    return (
        "自分と相手はそれぞれ" in text
        and "枚になるまで" in text
        and ("控え室に置き" in text or "控え室に置く" in text)
        and len(re.split(r"その後[、。]?", text, maxsplit=1)) == 2
    )


_try_both_discard_until = EffectPattern(
    condition=_matches_both_discard_until,
    action="sequential",
    setter=_set_both_discard_until,
)


def _set_re_yell(text, result):
    actions = []
    source_text = text
    if "。" in text:
        preceding, source_text = text.rsplit("。", 1)
        preceding = preceding.strip()
        if preceding:
            parsed = parse_effect(preceding)
            if isinstance(parsed, dict):
                actions.extend(
                    parsed.get("actions", [])
                    if parsed.get("action") == "sequential"
                    else [parsed]
                )
        source_text = source_text.strip().lstrip("、")
    promoted_condition = None
    if actions and isinstance(actions[0].get("condition"), dict):
        promoted_condition = actions[0].pop("condition")
    actions.extend(
        [
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
        ]
    )
    result["text"] = source_text
    result["actions"] = actions
    if promoted_condition:
        result["condition"] = promoted_condition


_try_re_yell = EffectPattern(
    condition=lambda t: "もう一度エールを行う" in t and "ブレードハートを失い" in t,
    action="sequential",
    setter=_set_re_yell,
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
        count = as_int(cm.group(1))
    optional = "してもよい" in at or "てもよい" in at
    operation = "set"
    if "減らす" in at or "減る" in at:
        operation = "decrease"
    elif "増やす" in at or "増える" in at:
        operation = "increase"
    raw_options = re.split(r"か[、，]?", options_text)
    options = []
    icon_pat = HEART_ICON_ID
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
    if _is_real_condition(cond):
        result["condition"] = cond
    result["count"] = count
    if optional:
        result["optional"] = True
    return result


# ====================================================================
# EFFECT HANDLER DEFINITIONS & DISPATCH
# ====================================================================
# The _STRUCTURAL_EFFECT_RULES list defines the priority-ordered effect rules.
# Each EffectPattern takes raw effect text and returns a parsed dict or None.
# The first match wins — ordering is CRITICAL.
#
# Key constraints:
#   - Per-unit (につき) must be checked early because it restructures text
#   - Conditional shapes (場合/そうした場合) must be checked before single actions
#   - Sequential patterns (その後) must be checked before _try_conditional
# ====================================================================


def _set_timing_condition_gain(text, result):
    match = re.search(
        r"このターン中にエリアを移動した(?:(?:全て|すべて)の)?(?:(.+?)の)?メンバー[は、]+(.+?)を得る",
        text,
    )
    if not match:
        return
    resource_text = match.group(2)
    blade_count = resource_text.count(BLADE_ICON)
    result["text"] = resource_text + "を得る"
    result["resource"] = "blade"
    result["count"] = blade_count
    result["card_type"] = "member_card"
    result["timing_condition"] = "moved_this_turn"
    result["target"] = "self"
    if re.search(r"このターン中にエリアを移動した(?:全て|すべて)の", text):
        result["all"] = True
    if match.group(1):
        result["group_names"] = [
            match.group(1).strip("｢「『　 ").rstrip("｣」』　 ")
        ]


_try_timing_condition_gain = EffectPattern(
    condition=lambda t: re.search(
        r"このターン中にエリアを移動した(?:(?:全て|すべて)の)?(?:(.+?)の)?メンバー[は、]+(.+?)を得る",
        t,
    )
    is not None
    and t.count(BLADE_ICON) > 0,
    action="gain_resource",
    setter=_set_timing_condition_gain,
)


def _parse_place_under_heart_copy(text):
    """The two legs of 「…下に置く。そうしたとき、元々持つハートと同じになる」.

    Returns (move, heart_text) when both legs are present and the placement
    really does parse as a move; None otherwise. Matcher and setter share this
    so "does this phrase match?" and "build the effect" can never disagree.
    """
    if "同じになる" not in text or "そうしたとき" not in text:
        return None
    separator = "そうしたとき、" if "そうしたとき、" in text else "そうしたとき"
    place_text, _, heart_text = text.partition(separator)
    place_text = place_text.strip()
    heart_text = heart_text.strip().lstrip("、")
    if "このメンバーの下に置く" not in place_text:
        return None
    if "元々持つハート" not in heart_text or "と同じになる" not in heart_text:
        return None
    move = parse_action(place_text)
    if not move or move.get("action") != "move_cards":
        move = parse_effect(place_text)
    if not isinstance(move, dict) or move.get("action") != "move_cards":
        return None
    return move, heart_text


def _matches_place_under_heart_copy(text):
    return _parse_place_under_heart_copy(text) is not None


def _set_place_under_heart_copy(text, result):
    move, heart_text = _parse_place_under_heart_copy(text)
    move.setdefault("destination", "under_member")
    move.setdefault("count", 1)
    result["conditional"] = True
    result["actions"] = [
        move,
        {
            "text": heart_text,
            "action": "set_heart_type",
            "heart_type": None,
            "ref_value": "placed_under",
            "original_value": True,
            "self_target": True,
            "card_type": "member_card",
            "duration": "live_end",
        },
    ]


_try_place_under_heart_copy = EffectPattern(
    condition=_matches_place_under_heart_copy,
    action="sequential",
    setter=_set_place_under_heart_copy,
)


_STRUCTURAL_EFFECT_RULES = [
    EffectPattern(action="custom", handler=_try_timing_condition_gain),
    EffectPattern(action="custom", handler=_try_self_and_other),
    EffectPattern(action="custom", handler=_try_per_unit),
    EffectPattern(action="custom", handler=_try_yell_source_modifier),
    EffectPattern(action="custom", handler=_try_activation_history_tiers),
    EffectPattern(action="custom", handler=_try_cost_set_from_reference),
    EffectPattern(action="custom", handler=_try_energy_ahead_alternative),
    EffectPattern(action="custom", handler=_try_conditional_alternative),
    EffectPattern(action="custom", handler=_try_character_specific),
    EffectPattern(action="custom", handler=_try_activation_suffix),
    EffectPattern(action="custom", handler=_try_cost_modification),
    EffectPattern(action="custom", handler=_try_kore_niyori_case),
    EffectPattern(action="custom", handler=_try_heart_select_reveal),
    EffectPattern(action="custom", handler=_try_choose_self_opponent),
    EffectPattern(action="custom", handler=_try_look_and_select),
    EffectPattern(action="custom", handler=_try_answer_choice),
    EffectPattern(action="custom", handler=_try_each_time),
    EffectPattern(action="custom", handler=_try_those_cards_add_hand_optional),
    EffectPattern(action="custom", handler=_try_discard_shuffle_to_bottom_optional),
    EffectPattern(action="custom", handler=_try_discard_hand_recover_self_optional),
    EffectPattern(action="custom", handler=_try_discard_hand_reactivate_optional),
    EffectPattern(action="custom", handler=_try_discard_live_and_member_optional),
    EffectPattern(action="custom", handler=_try_unless_effect),
    EffectPattern(action="custom", handler=_try_opponent_action),
    EffectPattern(action="custom", handler=_try_opponent_after_conditional),
    EffectPattern(action="custom", handler=_try_reveal_until_chosen_card),
    EffectPattern(action="custom", handler=_try_reveal_until_live),
    EffectPattern(action="custom", handler=_try_furthermore),
    EffectPattern(action="custom", handler=_try_kore_niyori_result),
    EffectPattern(action="custom", handler=_try_sequential_duration),
    _try_place_under_heart_copy,
    EffectPattern(action="custom", handler=_try_conditional_sequential),
    EffectPattern(action="custom", handler=_try_sequential),
    EffectPattern(action="custom", handler=_try_duration_effect),
    EffectPattern(action="custom", handler=_try_sou_shinakatta),
    EffectPattern(action="custom", handler=_try_period_conditional),
    EffectPattern(action="custom", handler=_try_compound_select),
    EffectPattern(action="custom", handler=_try_shi_sequential),
    EffectPattern(action="custom", handler=_try_te_sequential),
    EffectPattern(action="custom", handler=_try_re_yell),
    EffectPattern(action="custom", handler=_try_implicit_sequential),
    EffectPattern(action="custom", handler=_try_conditional),
    EffectPattern(action="custom", handler=_try_ability_activation),
    EffectPattern(action="custom", handler=_try_heart_choice),
    EffectPattern(action="custom", handler=_try_choice),
    EffectPattern(action="custom", handler=_try_kore_niyori_cascade),
    EffectPattern(action="custom", handler=_try_baton_touch_effect),
    EffectPattern(action="custom", handler=_try_play_baton_touch),
    EffectPattern(action="custom", handler=_try_both_discard_until),
]

# ======================================================================
# EFFECT RULE REGISTRY (ADD NEW EFFECT PHRASES HERE)
# Each EffectPattern(...) row is dispatched before structural handlers. Rows use
# the same contract as every other effect rule. Keep rows narrow so they only
# fire on their phrase and do not shadow longer structural text.
# ======================================================================
_EFFECT_RULES: List[Any] = []


def _register_effect_rule(rule: EffectPattern) -> EffectPattern:
    _EFFECT_RULES.append(rule)
    return rule


# --- example rows: simple phrase → shape (proven identical to legacy output) ---
# Each setter calls _fill_defaults so the row's output carries the same
# source/destination enrichment as the structural effect rules. The condition
# matches the exact phrase (post-normalization) so a row never shadows longer
# structural text.
def _exact_effect_phrase(phrase: str):
    return lambda t: t.strip().rstrip("。") == phrase


_register_effect_rule(
    EffectPattern(
        condition=_exact_effect_phrase("カードを1枚引く"),
        action="draw_card",
        defaults={"count": 1},
        setter=lambda t, r: _fill_defaults(r, t),
    )
)
_register_effect_rule(
    EffectPattern(
        condition=_exact_effect_phrase("カードを3枚引く"),
        action="draw_card",
        defaults={"count": 3},
        setter=lambda t, r: _fill_defaults(r, t),
    )
)
# C5: dynamic blade-limit wait — "元々持つブレードの数がこのメンバーの下にある
# エネルギーカードの枚数に1を足した数以下のメンバー1人をウェイトにする".
# The limit is computed at resolution time as (energy cards under this member) + 1.
_register_effect_rule(
    EffectPattern(
        match="の下にあるエネルギーカードの枚数に1を足した数以下",
        action="change_state",
        defaults={
            "state_change": "wait",
            "count": 1,
            "card_type": "member_card",
            "target": "opponent",
            "original_value": True,
            "blade_limit_from_energy_under": True,
            "blade_limit_offset": 1,
            "blade_limit_operator": "<=",
            "source": None,
        },
        setter=lambda t, r: _fill_defaults(r, t),
    )
)
# Q266: dynamic blade-limit wait referenced to the COSTED member —
# "元々持つブレードの数がこれによりウェイトにしたメンバーが元々持つブレードの数より2つ以上
# 少ないメンバー1人をウェイトにする". Limit = (original blade of the member paid as the wait
# cost) − 2, computed at resolution time. "2つ以上少ない" = at most that many fewer → <=.
_register_effect_rule(
    EffectPattern(
        match_all=[
            "これによりウェイトにしたメンバー",
            "より",
            "以上少ないメンバー",
        ],
        action="change_state",
        defaults={
            "state_change": "wait",
            "count": 1,
            "card_type": "member_card",
            "target": "opponent",
            "original_value": True,
            "blade_limit_from_cost_member": True,
            "blade_limit_offset": 2,
            "blade_limit_operator": "<=",
            "source": None,
        },
        setter=lambda t, r: _fill_defaults(r, t),
    )
)


def _set_both_hand_keep_shuffle_under(t: str, r: Dict[str, Any]) -> None:
    """C6: 自分と相手はそれぞれ、手札のカードをN枚まで選び、選んだカード以外を
    シャッフルして自身のデッキの下に置く。その後、それぞれカードをN枚引く。
      select (hand, both, count=N, max, keep_shuffle_under)  keep up to N,
              shuffle the REST under own deck (handled by the engine)
      draw   (deck -> hand, both, count=N)
    """
    cm = re.search(r"カードを(\d+)枚まで選び", t)
    count = as_int(cm.group(1)) if cm else 1
    cm2 = re.search(r"その後、[^。]*カードを(\d+)枚引く", t)
    draw_count = as_int(cm2.group(1)) if cm2 else count
    actions = [
        {
            "text": t,
            "action": "select",
            "source": "hand",
            "count": count,
            "max": True,
            "keep_shuffle_under": True,
            "target": "both",
            "multiple_targets": True,
        },
        {
            "text": t,
            "action": "draw_card",
            "source": "deck",
            "destination": "hand",
            "count": draw_count,
            "target": "both",
            "multiple_targets": True,
        },
    ]
    r.clear()
    r.update({"text": t, "action": "sequential", "actions": actions})


_register_effect_rule(
    EffectPattern(
        match="選んだカード以外のカードをシャッフルし",
        action="sequential",
        setter=_set_both_hand_keep_shuffle_under,
    )
)
# Structural effect rules share the canonical EffectPattern dispatch contract.
_effect_registry = PriorityRegistry("effect_rules")
for _ri, _h in enumerate(_EFFECT_RULES):
    _hn = getattr(_h, "__name__", f"effect_rule_{_ri}")
    _effect_registry.register(_ri, _hn, _h)
for _i, _rule in enumerate(_STRUCTURAL_EFFECT_RULES):
    _hn = getattr(_rule, "__name__", f"structural_rule_{_i}")
    _effect_registry.register(100 + _i, _hn, _rule)


_effect_registry.register(
    10000,
    "blade_conversion",
    EffectPattern(
        condition=lambda t: "すべて[" in t and "]になる" in t,
        action="set_blade_type",
        setter=_set_blade_conversion,
    ),
)
_effect_registry.register(
    10001,
    "blade_equal_gain",
    EffectPattern(
        condition=lambda t: "を得る" in t and "コストが同じ" in t,
        action="gain_resource",
        setter=_set_blade_equal_gain,
    ),
)
_effect_registry.register(
    10002,
    "blade_same_thing_gain",
    EffectPattern(
        match="同じことを行う",
        action="gain_resource",
        setter=_set_blade_same_thing_gain,
    ),
)
_effect_registry.register(
    10003,
    "blade_count_set",
    EffectPattern(
        condition=lambda t: re.search(
            r"ブレードの数は(\d+)つになる",
            re.sub(r"\{\{[^|]+\|([^}]+)\}\}", r"\1", re.sub(r"\s+", "", t)),
        )
        is not None,
        action="set_blade_count",
        setter=_set_blade_count_set,
    ),
)
_effect_registry.register(
    10004,
    "restriction",
    EffectPattern(
        condition=lambda t: "ウェイトしない" in t
        or "アクティブにならない" in t
        or "アクティブにしない" in t,
        action="restriction",
        setter=_set_restriction_fields,
    ),
)
_effect_registry.register(
    10005,
    "global_modifier",
    EffectPattern(
        condition=lambda t: re.search(r".+は、.+", t)
        is not None
        and "ある場合" not in t
        and "必要ハート" in t
        and ("多くなる" in t or "少なくなる" in t),
        action="modify_required_hearts_global",
        setter=_set_global_modifier_fields,
    ),
)


def _try_play_time_cost_set(text):
    """「このカードのプレイに際し、<move>。そうしたとき、このカードのコストはNになる。」

    Play-time alternative cost (rule 7.x): <move> is an optional payment
    made when playing the card; if paid, this card's play cost becomes N.
    Generic handler for any card with this template — replaces the former
    LL-bp7-001-specific override.

    Shape contract with the engine's play-time cost hook:
      effect = {action: modify_cost, operation: set, value: N,
                source/location: hand, characters, count, optional}
      cost   = the move itself (move_cards, per_character) — embedded as
               effect["cost"]; parse_ability lifts it to ability level.
    """
    if "プレイに際し" not in text:
        return None
    vm = re.search(r"このカードのコストは(\d+)になる", text)
    mm = re.search(r"プレイに際し、(.+?)。そうしたとき、", text)
    if not vm or not mm:
        return None
    move_text = mm.group(1)
    characters = extract_all_quoted_names(move_text)
    # 控え室に置く / 置いて / 置き — any placement form
    if not characters or "控え室に置" not in move_text:
        return None
    optional = extract_optional(move_text)
    effect = {
        "text": text,
        "action": "modify_cost",
        "operation": "set",
        "value": as_int(vm.group(1)),
        "source": "hand",
        "location": "hand",
        "card_type": "member_card",
        "characters": characters,
        "count": len(characters),
        "optional": optional,
        # Embedded play-time payment; parse_ability lifts this to
        # ability["cost"].
        "cost": {
            "text": move_text,
            "type": "move_cards",
            "source": "hand",
            "zone": "hand",
            "destination": "discard",
            "card_type": "member_card",
            "characters": characters,
            "count": 1,
            "per_character": True,
            "optional": optional,
        },
    }
    return effect


# Negative priority: play-time costs are the most specific shape in effect
# text and must win before generic handlers mis-parse them.
_effect_registry.register(
    -10,
    "play_time_cost_set",
    EffectPattern(action="custom", handler=_try_play_time_cost_set),
)

# ======================================================================
# POST-PROCESSING NORMALIZERS & CHAINING
# ======================================================================


def _has_position_keywords(text):
    positions = detect_positions(text)
    if not positions and "center" in text.lower():
        return "center"
    return ",".join(positions) if positions else None




# ─────────────────────────────────────────────────────────────────────────
# Field-propagation schema (single source of truth)
# ─────────────────────────────────────────────────────────────────────────
# The walkers below copy fields from parent context onto nested sub-actions
# without knowing each action's semantics. That is fine for generic fields
# (target, position, duration) but several fields are type-specific and must
# only ever land on an action whose own text says so — otherwise they silently
# change what the action targets (e.g. heart_colors on a move filters to one
# color; card_type/group_names on gain_resource redirects to group members).
#
# _CONTEXT_FIELDS        : the generic fields we propagate from parent context.
# _BLOCKED_FOR_ACTION    : action -> fields NEVER copied from context.
# _OWN_TEXT_REQUIRED     : (action, field) -> the sub-action must mention the
#                          field's subject in its own text, or the field is
#                          dropped (parent-context inheritance would be wrong).
#                          A callable value gets (sub, field_value) and returns
#                          whether to allow propagation.
# These tables are consulted by _clean_action_list AND _walk_extract_heart_colors
# so a single edit fixes every propagation path (no more ad-hoc scattered guards).
_CONTEXT_FIELDS = (
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
)

_BLOCKED_FOR_ACTION = {
    # A gain's heart color is chosen by a preceding select; never inherit it.
    # card_type also never propagates onto a gain (only when text targets it).
    "gain_resource": {"heart_colors", "card_type"},
    # Heart color on a move means "move cards OF this color"; never from context.
    "move_cards": {"heart_colors"},
}


def _propagation_allowed(f, sub, parent_effect):
    """Return True if parent field `f` may be copied onto sub-action `sub`."""
    sub_action = sub.get("action")
    # exclude_self never lands on self-targeting / resource-grant actions.
    if f == "exclude_self" and (
        sub.get("target") == "self"
        or sub_action in ("gain_resource", "set_heart_type", "heart_selection", "modify_score")
    ):
        return False
    if f == "group_names" and sub_action == "change_state":
        if sub.get("card_type") == "energy_card":
            return False
        sub_text = sub.get("text", "")
        return any(g in sub_text for g in parent_effect[f])
    if f == "group_names" and sub_action in (
        "draw_card",
        "move_cards",
        "modify_score",
        "sequential",
    ):
        sub_text = sub.get("text", "")
        return any(g in sub_text for g in parent_effect[f])
    if f == "group_names" and sub_action == "gain_resource":
        return False
    # group_names on specify_heart_color / reveal: no group filtering.
    if f == "group_names" and sub_action in ("specify_heart_color", "reveal"):
        return False
    # group_names on modify_cost only for per-unit (needs the count filter).
    if (
        f == "group_names"
        and sub_action == "modify_cost"
        and not sub.get("per_unit")
    ):
        return False
    # group_names / heart_colors on move_cards only if the move's own text
    # mentions the subject (parent-context inheritance would mis-target the move).
    if f == "group_names" and sub_action == "move_cards":
        sub_text = sub.get("text", "")
        return any(g in sub_text for g in parent_effect[f])
    if f == "heart_colors" and sub_action == "move_cards":
        sub_text = sub.get("text", "")
        return "heart" in sub_text or "ハート" in sub_text or "色" in sub_text
    if f in _BLOCKED_FOR_ACTION.get(sub_action, set()):
        return False
    return True


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
    # Propagate fields from parent to each sub-action (schema-driven).
    if parent_effect:
        for f in _CONTEXT_FIELDS:
            if f in parent_effect:
                for sub in cleaned:
                    if f not in sub and _propagation_allowed(f, sub, parent_effect):
                        sub[f] = parent_effect[f]
        # Propagate card_type from parent to sub-actions that don't have it
        pt = parent_effect.get("card_type")
        if pt:
            for sub in cleaned:
                if "card_type" not in sub and _propagation_allowed("card_type", sub, parent_effect):
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


# The clause separators that may sit between a condition's text and the part
# of the action text a group can actually filter.
_CLAUSE_SEPARATORS = ("、", "場合、", "とき、", "なら、")


def _accepts_group_filter(d):
    """True when a group_names filter is meaningful on this node.

    Four kinds of node must never be group-filtered:
      - an energy change_state: energy cards are generic
      - a gain_resource that is not per-unit: a leaked group makes the engine
        hand the resource to ALL matching group members, not the one that gained
      - a non-per-unit modify_cost, for the same reason
      - a card_count_condition: the parser already handles pure group-filtered
        counts, and _walk would wrongly add a filter to inclusion-pattern
        counts ("1人を含む")
    """
    if d.get("action") == "change_state" and d.get("card_type") == "energy_card":
        return False
    if d.get("action") in ("gain_resource", "modify_cost") and not d.get("per_unit"):
        return False
    return d.get("type") != "card_count_condition"


def _condition_text(d):
    """The text of this node's attached condition, or ""."""
    cond = d.get("condition")
    return cond.get("text", "") if isinstance(cond, dict) else ""


def _text_after_condition(node_text, cond_text):
    """`node_text` trimmed to the part following the condition clause.

    A group named in the condition but not after the clause boundary belongs
    to the condition, not to the action. With no condition text there is no
    boundary to find, so the text is returned whole.
    """
    if not cond_text:
        return node_text
    for sep in _CLAUSE_SEPARATORS:
        joined = cond_text + sep
        if joined in node_text:
            return node_text.split(joined, 1)[-1]
    return node_text


def _group_names_from_context(d, d_ctx, ctx_text):
    """The group filter this node should inherit from context, or [].

    The group is read from the node's own text first, then the text context,
    then the parent context. A parent-context group reaches the node only when
    the node actually depends on that condition's clause.
    """
    if not (d.get("action") or d.get("type") or d.get("condition") or "text" in d):
        return []
    groups = _quoted_group_names(d_ctx or "")
    from_parent = not groups and bool(ctx_text)
    if from_parent:
        groups = _quoted_group_names(ctx_text or "")
    if not groups or not _accepts_group_filter(d):
        return []
    # The same group may be named several times in one clause.
    groups = list(dict.fromkeys(groups))
    if from_parent:
        # Only a node that names the group itself, or that compares names
        # (sub-conditions with `distinct`, which need the group to know which
        # cards to compare), may take a parent's group. Pushing it anywhere
        # else would make e.g. "『みらくらぱーく！』" or a condition-only
        # "『Liella!』のメンバーからバトンタッチ" filter the action itself.
        named_own = any(group in (d.get("text") or "") for group in groups)
        return groups if named_own or d.get("distinct") else []
    # The group may belong to the condition alone ("『スリーズブーケ』のメンバーが
    # いる場合"), so it filters the action only when the action draws on the
    # condition's cards or names the group after the clause boundary.
    node_text = _text_after_condition(d.get("text") or "", _condition_text(d))
    if d.get("source") == "those_cards" or any(g in node_text for g in groups):
        return groups
    return []


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
    if "original_value" not in d and d_ctx and check_original_value(d_ctx):
        d["original_value"] = True

    # Propagate group_names from text context (including parent context) to any dict node
    if "group_names" not in d:
        inherited = _group_names_from_context(d, d_ctx, ctx_text)
        if inherited:
            d["group_names"] = inherited

    # Propagate shuffle from text context
    if "shuffle" not in d and d_ctx and "シャッフル" in d_ctx:
        d["shuffle"] = True


def _normalize_heart_ids(text: str) -> List[str]:
    """Deduplicated, zero-padded heart color ids from bare 'heart_N' mentions."""
    return _heart_ref_ids(text, unique=True)


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
            # should not inherit aggregate heart_colors from parent context.
            # Heart color is only inherited from context when the schema allows
            # it for this action type (never for blanket/all moves or gains).
            if not re.search(HEART_HAS_REF, search_text) and ctx_text:
                if not d.get("heart_color"):
                    if _propagation_allowed("heart_colors", d, {"text": ctx_text}):
                        search_text = ctx_text
        hc = _normalize_heart_ids(search_text)
        if not hc and "heart_00" in search_text:
            hc = ["heart00"]
        if hc:
            d["heart_colors"] = hc
    # For modify_required_hearts, set value = per-color count (not total).
    # The icon sequence {heart02×3, heart03×3, ...} means 3 per color, not 12 total.
    if d.get("action") == "modify_required_hearts" and "value" not in d:
        search_val = d_text or ""
        if not re.search(HEART_HAS_REF, search_val) and ctx_text:
            search_val = ctx_text
        target_colors = d.get("heart_colors", [])
        color_counts = _count_heart_refs(search_val, target_colors or None)
        per_color = _uniform_count(color_counts)
        if per_color is not None:
            d["value"] = per_color

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
            if re.search(HEART_ICON, before):
                d["filter_targets_by_heart_colors"] = True

    # Extract heart_colors from the node's own text for look_and_select select_actions.
    # Uses d.get("text") instead of ctx_text to avoid inheriting heart colors
    # from unrelated parts of the parent effect (e.g. a followup condition like
    # "30以上の場合、さらにこのカードの必要ハートを{{heart_00.png|heart0}}減らす").
    node_text = d.get("text") or ""
    if "heart_colors" not in d and d.get("action") == "select_cards" and node_text:
        hc = _normalize_heart_ids(node_text)
        if hc:
            d["heart_colors"] = hc
            if detect_require_all_hearts(node_text):
                d["require_all_heart_colors"] = True


def _walk_propagate_all_and_targets(d, d_ctx):
    # Propagate all from text context (must match _fill_defaults patterns)
    if (
        "all" not in d
        and d_ctx
        and _ALL_KW_RE.search(d_ctx)
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


def _bind_condition_to_moved_count(cond, prev_count):
    """Point a 「すべて/全部」 condition at the cards the previous step moved.

    「すべて」 means ALL of them, so the comparison becomes exact: count = how
    many moved, operator "=", source "preceding_moved". A card-count condition
    that already carries a real number or a non-default operator is left alone —
    the clause is then about something other than the moved set. A group
    condition has no count of its own, so it converts into one.
    """
    if cond.get("type") == "card_count_condition":
        if cond.get("count", 1) != 1 or cond.get("operator", ">=") != ">=":
            return
    elif cond.get("type") == "group_condition":
        cond["type"] = "card_count_condition"
    else:
        return
    cond["count"] = prev_count
    cond["operator"] = "="
    cond["source"] = "preceding_moved"


def _walk_propagate_sequential_links(d):
    """Link a sequential's conditions to the step before them.

    Two phrasings, two different conditions:
      「すべて」/「全部」  "if ALL moved cards match" → exact count over the moved
                          set, so a group condition converts into a count one
      「それらの中に」/「これにより」  "if any among moved cards match" → source
                          only, keeping the count the condition already had

    Plus a trailing rule: a bare 「2枚ある場合」 with no filter of its own
    inherits the fields of the preceding_moved condition, because it is talking
    about the same cards.
    """
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
                _bind_condition_to_moved_count(cond, prev_count)
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


# Condition kinds that carry a count and therefore need an operator.
_COUNTED_CONDITION_TYPES = ("comparison_condition", "card_count_condition", "location_condition")

# 「高い/多い/大きい」 and 「低い/少ない/小さい」 — a comparison that is not
# written with a numeric 以上/以下.
_GREATER_WORDS = ("高い", "多い", "大きい")
_LESSER_WORDS = ("低い", "少ない", "小さい")


def _infer_comparison_operator(d, text):
    """Fill the comparison operator a count condition leaves unstated.

    「以下」 is tested before 「以上」 so a compound like 「1枚以上公開…2枚以下の
    場合」 picks ≤2 (the real condition) rather than ≥1 (the trigger clause).
    """
    if d.get("count") is None or d.get("comparison_target"):
        return
    # Always override a pre-set operator for an explicit 以上/以下.
    if "以下" in text:
        d["operator"] = "<="
    elif "以上" in text:
        d["operator"] = ">="
    elif "operator" not in d:
        d["operator"] = "="


def _infer_unbounded_comparison_operator(d, text):
    """Fill the operator for a comparison that names no count at all."""
    if "operator" in d:
        return
    if d.get("values"):
        d["operator"] = "in"
    elif d.get("comparison_target"):
        if any(word in text for word in _GREATER_WORDS):
            d["operator"] = ">"
        elif any(word in text for word in _LESSER_WORDS):
            d["operator"] = "<"


def _walk_set_defaults(d, d_text, ct):
    # Default target to "self" for location_conditions if missing
    if d.get("type") == "location_condition" and "target" not in d:
        d["target"] = "self"

    if ct in _COUNTED_CONDITION_TYPES:
        text = d.get("text", "")
        # A cost_limit already states the number, so the count must not
        # override it with the text's own 以上/以下.
        if not (ct == "card_count_condition" and "cost_limit" in d):
            _infer_comparison_operator(d, text)
        _infer_unbounded_comparison_operator(d, text)

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
    #
    # If the merged condition ALREADY carries the positional qualifier
    # (e.g. each_time resolution watchers: 「センターエリアにいる…能力が
    # 解決したとき、そのメンバーをポジションチェンジする」), the qualifier
    # describes the TRIGGER context — duplicating it onto the action body
    # would make position_change read it as a source/destination.
    cond = d.get("condition")
    if (
        d.get("trigger_type") == "each_time"
        and isinstance(cond, dict)
        and cond.get("position")
    ):
        return
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
    if "original_value" not in d and d_text and check_original_value(d_text):
        d["original_value"] = True

    # Operator ownership for original-value conditions: the operator is
    # consumed per-card (current vs original hearts/blade). 「元々…より多い」
    # is a STRICT comparison; ">=" here would make every unboosted member
    # qualify. The outer member-count threshold stays >= engine-side.
    if (
        d.get("original_value")
        and "blade_limit" not in d
        and d.get("operator") == ">="
        and d_text
        and ("より多い" in d_text or "より多く" in d_text)
    ):
        d["operator"] = ">"


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

    # Propagate same_name from text context — "と同じ名前" / "同じ名前"
    if (
        "same_name" not in d
        and d_ctx
        and ("相同的名前" in d_ctx or ("同じ名前" in d_ctx and "持つ" in d_ctx))
    ):
        d["same_name"] = True
    if "same_name" not in d and "相同的名前" in (d.get("text", "") or ""):
        d["same_name"] = True





def _fix_unqualified_group_card_move(d):
    if d.get("action") != "move_cards":
        return
    if d.get("group_reference") != "different_group_names":
        return
    if d.get("source") != "discard" or d.get("destination") != "hand":
        return
    text = d.get("text") or ""
    if not re.search(r"カード\d+枚", text):
        return
    if _TYPED_COUNT_RE.search(text):
        return
    d["card_type"] = "card"
    d["all"] = False
    d["count"] = 1


def _walk_cleanup_text(d, d_text):
    # Strip leading comma from text artifacts (e.g. "、{{icon_energy.png|E}}支払ってもよい")
    if d_text and (d_text.startswith("、") or d_text.startswith("，")):
        d["text"] = d_text.lstrip("、，").strip()


_WALK_CHILD_KEYS = (
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
)


def _walk(d, full_text, original_text, ctx_text=None):
    if not isinstance(d, dict):
        return d

    def enter(node, parent_text):
        d_ctx = node.get("text") or parent_text or full_text
        d_text = node.get("text") or ""
        ct = node.get("condition_type") or node.get("type")
        _walk_ensure_text(node, parent_text, full_text)
        _walk_propagate_activation_position(node, original_text, full_text)
        _walk_propagate_text_context_fields(node, d_ctx, parent_text)
        _walk_extract_heart_colors(node, d_text, parent_text)
        _walk_propagate_all_and_targets(node, d_ctx)
        _walk_propagate_sequential_links(node)
        _walk_set_defaults(node, d_text, ct)
        _walk_propagate_position(node, d_ctx, d_text)
        _walk_propagate_flags(node, d_ctx)
        _fix_unqualified_group_card_move(node)
        _walk_cleanup_text(node, d_text)
        if isinstance(node.get("actions"), list):
            node["actions"] = _clean_action_list(node["actions"], node, d_ctx)
        return d_ctx

    def leave(node, _parent_text):
        if node.get("action") == "sequential" and not node.get("actions"):
            node.pop("action", None)

    walk_dict_tree(
        d,
        keys=_WALK_CHILD_KEYS,
        context=ctx_text,
        enter=enter,
        leave=leave,
        child_context=lambda node, node_context, _key, _child: node_context,
    )
    return d


def _mark_under_card_gain(effect, text):
    """CLEAN-G3: '…メンバーカードが下に置かれている『X』のメンバーは、…を得る' —
    a constant gain whose subject is only members that HAVE a member card under
    them. Mark it all-targeting + requires_under_card so the engine filters."""
    if not isinstance(effect, dict):
        return
    if "メンバーカードが下に置かれている" in text and effect.get("action") == "gain_resource":
        effect["all"] = True
        effect["requires_under_card"] = True


def _fix_energy_difference_dynamic_count(effect):
    """G6: 'エネルギーがN枚より多いかぎり、その差に等しい数のXを得る' — the
    dynamic_count reference 'その差' is unresolvable prose. Rewrite it to
    'energy_difference' + base_reference=N (the threshold) when the effect's
    condition is an energy-count comparison, so the engine computes energy − N."""
    if not isinstance(effect, dict):
        return
    dc = effect.get("dynamic_count")
    cond = effect.get("condition")
    if not isinstance(dc, dict) or dc.get("reference") != "その差":
        return
    if not isinstance(cond, dict):
        return
    if (
        cond.get("resource_type") == "energy"
        and cond.get("operator") in (">", ">=")
        and isinstance(cond.get("count"), int)
    ):
        dc["reference"] = "energy_difference"
        dc["base_reference"] = str(cond["count"])


def _fix_select_self_and_other(effect):
    """CLEAN-G19:「このメンバーと…ほかの『X』のメンバー1人を選ぶ」 — select and follow-up activation include this member."""
    if not isinstance(effect, dict):
        return
    root_text = effect.get("text", "") or ""
    has_self_and = "このメンバーと" in root_text and "を選ぶ" in root_text
    if not has_self_and:
        return
    for node in iter_dict_nodes(effect):
        if node.get("action") in ("select", "activate_ability"):
            node.pop("exclude_self", None)
            if node.get("action") == "select":
                node["count"] = 2


def _mark_live_total_score(node):
    """Mark score effects that modify the player's live total."""
    for current in iter_dict_nodes(node):
        if current.get("action") != "modify_score":
            continue
        text = current.get("text") or ""
        if ("合計スコア" in text or "ライブのスコア" in text) and current.get(
            "target"
        ) not in ("opponent", "相手"):
            current["target"] = "live_total"


def _mark_live_total_clamp(node, original_text: str = ""):
    """Stamp the printed per-effect live-total score floor on the whole tree."""
    source = original_text or node.get("text", "") or ""
    has_clamp_phrase = ("この効果では" in source and "未満にはならない" in source) or (
        "この効果では" in node.get("text", "")
        and "未満にはならない" in node.get("text", "")
    )
    if not has_clamp_phrase:
        has_clamp_phrase = any(
            isinstance(v, str)
            and "この効果では" in v
            and "未満にはならない" in v
            for v in node.values()
        )
    if not has_clamp_phrase or "0未満" not in source.replace("０", "0"):
        return

    for current in iter_dict_nodes(node):
        if current.get("action") == "sequential":
            current["effect_constraint"] = "min:0"
            current["score_floor"] = 0
        elif current.get("action") == "modify_score":
            current["effect_constraint"] = "min:0"
            current["score_floor"] = 0


def _state_energy_is_object(txt: str) -> bool:
    """True when エネルギー is the OBJECT of a state change (activate/wait),
    not merely mentioned (e.g. as an under-member count reference)."""
    return bool(
        re.search(
            r"エネルギー(?:[0-9０-９一二三四五六]+枚)?を(?:すべて?)?(?:アクティブ|ウェイト)"
            r"|エネルギーを[0-9０-９]*枚*(?:アクティブ|ウェイト)"
            r"|すべてのエネルギーを"
            r"|エネルギー[0-9０-９]*枚?か",
            txt,
        )
    )


def _fw_int(txt: str, pattern: str) -> int:
    """First integer capture of `pattern` in `txt`, full-width normalized."""
    m = re.search(pattern, txt.translate(str.maketrans("０１２３４５６７８９", "0123456789")))
    return as_int(m.group(1)) if m else 1


def _energy_state_option(state, count, text):
    """The 「エネルギーN枚を<state>にする」 half of a …か… choice."""
    return {
        "action": "change_state",
        "state_change": state,
        "card_type": "energy_card",
        "count": count,
        "target": "self",
        "text": text,
    }


def _split_mixed_state_change(node):
    """Split change_state steps whose OBJECT spans two card kinds.

    - 「…メンバーと、…エネルギーをアクティブにする」 (AND) → sequential of a
      member step and an energy step.
    - 「エネルギー1枚か『虹ヶ咲』のメンバー1人をアクティブにする」 (OR) →
      choice with one option per kind, so the PLAYER picks a side (rules:
      either/or wording never resolves both).

    The engine consumes plain change_state steps afterwards — no effect-text
    sniffing. Steps where エネルギー appears only as a count reference
    (「…の下にあるエネルギーカードの枚数…」) match no pattern and stay intact.
    """
    if not isinstance(node, dict) or node.get("action") != "change_state":
        return node
    txt = node.get("text") or ""
    sc = node.get("state_change") or "active"

    # ── AND: 「メンバーと、エネルギー…」 ──
    if "メンバーと、" in txt and _state_energy_is_object(txt):
        grp = re.search(r"『([^』]+)』のメンバー", txt)
        member_txt = txt.split("と、")[0]
        member_step = {
            "action": "change_state",
            "state_change": sc,
            "card_type": "member_card",
            "count": 0,  # 0 = all matching (すべての)
            "target": node.get("target") or "self",
            "text": member_txt,
        }
        if grp:
            member_step["group_names"] = [grp.group(1)]
        energy_step = {
            "action": "change_state",
            "state_change": sc,
            "card_type": "energy_card",
            "count": 0,
            "target": "self",
            "text": "自分のすべてのエネルギーをアクティブにする"
            if sc == "active"
            else f"自分のすべてのエネルギーを{sc}にする",
        }
        seq = {
            "action": "sequential",
            "text": txt,
            "actions": [member_step, energy_step],
        }
        if node.get("condition"):
            seq["condition"] = node["condition"]
        return seq

    # ── OR: 「AかB」 ──
    if not ("か" in txt and _state_energy_is_object(txt)):
        return node

    def member_option(member_txt: str) -> dict:
        grp = re.search(r"『([^』]+)』", member_txt)
        opt = {
            "action": "change_state",
            "state_change": sc,
            "card_type": "member_card",
            "count": _fw_int(member_txt, r"メンバー([0-9]+)人"),
            "target": node.get("target") or "self",
            "text": member_txt,
        }
        if grp:
            opt["group_names"] = [grp.group(1)]
        return opt

    energy_n = _fw_int(txt, r"エネルギー([0-9]+)枚")
    m = re.search(r"エネルギー[0-9]+枚か(.+)", txt)
    if m:
        # energy-first ordering
        energy_text = (
            f"エネルギー{energy_n}枚をアクティブにする"
            if sc == "active"
            else f"エネルギー{energy_n}枚を{sc}にする"
        )
        options = [
            _energy_state_option(sc, energy_n, energy_text),
            member_option(m.group(1)),
        ]
    else:
        m2 = re.search(r"(.+?)か、エネルギー(?:([0-9]+)枚)?を(アクティブ|ウェイト)にする", txt)
        m2b = None
        if not m2:
            m2b = re.search(
                r"(.+?)か、エネルギーを([0-9]+)枚(アクティブ|ウェイト)", txt
            )
            if not m2b:
                return node
        if m2b:
            energy_n = as_int(m2b.group(2))
            options = [
                member_option(m2b.group(1)),
                _energy_state_option(
                    sc,
                    energy_n,
                    f"エネルギーを{energy_n}枚{m2b.group(3)}にする",
                ),
            ]
        else:
            energy_n = as_int(m2.group(2)) if m2.group(2) else 1
            options = [
                member_option(m2.group(1)),
                {
                    "action": "change_state",
                    "state_change": sc,
                    "card_type": "energy_card",
                    "count": energy_n,
                    "target": "self",
                    "text": f"エネルギー{energy_n}枚を{m2.group(3)}にする",
                },
            ]
    choice = {"action": "choice", "text": txt, "count": 1, "options": options}
    if node.get("condition"):
        choice["condition"] = node["condition"]
    return choice


def _walk_split_mixed(node):
    return transform_child_lists(node, _split_mixed_state_change)


def _mark_success_pile_difference(node):
    """Canonicalize success-pile card-count differences throughout the tree."""
    for current in iter_dict_nodes(node):
        dc = current.get("dynamic_count")
        cond = current.get("condition") or {}
        if (
            isinstance(dc, dict)
            and dc.get("reference") == "その差"
            and (
                cond.get("location") == "success_live_card_zone"
                or current.get("resource") == "blade"
            )
        ):
            dc["reference"] = "success_pile_count_difference"


def _canonicalize_dynamic_counts(node):
    """Replace raw dynamic_count references with engine-dispatchable tokens."""
    for current in iter_dict_nodes(node):
        dc = current.get("dynamic_count")
        if not isinstance(dc, dict):
            continue
        ref = str(dc.get("reference") or "")
        if "ウェイト状態" in ref and "ステージ" in ref and "メンバー" in ref:
            dc["reference"] = "opponent_waited_member_count"
        elif (
            ("その差" in ref and "デッキ" in ref)
            or ref == "その差"
            or "その差に等しい枚数" in ref
        ):
            cond = current.get("condition") or {}
            if "控え室" in str(cond.get("text", "")):
                dc["reference"] = "waitroom_count_below_base"
                match = re.search(r"(\d+)枚未満", str(cond.get("text", "")))
                dc["base_reference"] = match.group(1) if match else "8"
                current.pop("count", None)
        elif "これにより控え室に置いた" in ref:
            dc["reference"] = "these_waitroom_placed_count"
        elif "ステージ" in ref and "メンバー" in ref:
            dc["reference"] = (
                "opponent_stage_member_count"
                if "相手" in ref
                else "stage_member_count"
            )


def _split_look_three_way(node):
    """「その中から1枚を手札に加え、1枚をデッキの上に置き、1枚を控え室に置く」
    needs THREE destinations from one look — look_and_select only carries one
    select destination plus a remainder, so the hand leg was silently dropped.
    Decompose into look_at + explicit selects/move over the looked-at pool."""
    if not isinstance(node, dict) or node.get("action") != "look_and_select":
        return node
    sa = node.get("select_action") or {}
    t = str(sa.get("text") or "")
    if not ("手札に加え" in t and "デッキの上に置き" in t and "控え室に置く" in t):
        return node
    look = node.get("look_action") or {
        "action": "look_at",
        "source": "deck_top",
        "count": 3,
        "target": "self",
    }

    def sel(dest, txt):
        return {
            "action": "select_cards",
            "source": "looked_at",
            "destination": dest,
            "count": 1,
            # Each leg takes exactly one card; leftovers stay in the looked-at
            # pool for the NEXT leg instead of being swept to one destination.
            "discard_remaining": False,
            "remainder_destination": "looked_at",
            "target": node.get("target") or "self",
            "text": txt,
        }

    return {
        "action": "sequential",
        "text": node.get("text"),
        "actions": [
            look,
            sel("hand", "1枚を手札に加え"),
            # Last leg: its native remainder sweep places the final card into
            # the waitroom — no separate move step needed.
            {
                "action": "select_cards",
                "source": "looked_at",
                "destination": "deck_top",
                "count": 1,
                "discard_remaining": True,
                "target": node.get("target") or "self",
                "text": "1枚をデッキの上に置き、1枚を控え室に置く",
            },
        ],
    }


def _walk_split_look(node):
    return transform_child_lists(node, _split_look_three_way)


def _stamp_mid_sentence_duration(node, root_text):
    """Stamp duration on temp actions whose own text carries the phrase mid-sentence.
    No text is mutated. Choice options inherit nothing (local phrase only)."""
    TEMP = {
        "gain_resource", "modify_score", "change_state",
        "set_blade_count", "restriction", "gain_ability",
        "modify_cost", "modify_required_hearts",
    }
    if not isinstance(node, dict):
        return
    if node.get("action") == "choice":
        for opt in node.get("options", []):
            if isinstance(opt, dict) and "duration" not in opt:
                code = detect_duration_code(opt.get("text", "") or "")
                if code:
                    opt["duration"] = code
        return
    local = None
    txt = node.get("text", "") or ""
    if txt:
        local = detect_duration_code(txt)
    if local and node.get("action") in TEMP and "duration" not in node:
        node["duration"] = local
    prop = node.get("duration") or local
    for key in ("actions", "options", "conditions"):
        arr = node.get(key)
        if isinstance(arr, list):
            for child in arr:
                if isinstance(child, dict):
                    if prop and child.get("action") in TEMP and "duration" not in child:
                        child["duration"] = prop
                    _stamp_mid_sentence_duration(child, root_text)
    for key in ("condition", "primary_effect", "alternative_effect",
                "followup_action", "optional_action", "conditional_action",
                "gained_effect", "select_action", "look_action"):
        ch = node.get(key)
        if isinstance(ch, dict):
            if prop and ch.get("action") in TEMP and "duration" not in ch:
                ch["duration"] = prop
            _stamp_mid_sentence_duration(ch, root_text)
    if (node.get("action") in ("conditional_alternative", "conditional_on_result")
            and node.get("primary_effect")):
        pd = node["primary_effect"].get("duration")
        for key in ("alternative_effect", "followup_action"):
            alt = node.get(key)
            if pd and isinstance(alt, dict) and "duration" not in alt:
                if alt.get("action") in TEMP:
                    alt["duration"] = pd


def _infer_baton_placement_source(effect):
    if isinstance(effect, dict):
        text = effect.get("text", "") or ""
        if (
            effect.get("action") == "move_cards"
            and not effect.get("source")
            and "このカード" in text
            and "そのバトンタッチで登場した" in text
            and "下に置く" in text
        ):
            effect["source"] = "discard"
        for value in effect.values():
            _infer_baton_placement_source(value)
    elif isinstance(effect, list):
        for item in effect:
            _infer_baton_placement_source(item)
    return effect


def _normalize_parsed_effect(effect, original_text=None):
    if not effect or not isinstance(effect, dict):
        return effect
    _full_text = effect.get("text") or original_text or ""
    effect = _walk(effect, _full_text, original_text, original_text)
    effect = _collapse_position_changes(effect)
    # Strip leaked group from draw that doesn't contain the group in its own text.
    # Exception: per_unit_type="discard" draws (「バトンタッチによって控え室に置かれた
    # 『X』のメンバーカード1枚につき、カードを1枚引く」, Q-pb2-000). Their text is
    # stored as the tail AFTER「につき」, so the group can never appear in it —
    # yet the group legitimately defines WHICH discarded cards count.
    def _strip_leaked_draw_g(node):
        for current in iter_dict_nodes(node):
            if (
                current.get("action") == "draw_card"
                and current.get("group_names")
                and current.get("per_unit_type") != "discard"
            ):
                txt = current.get("text") or ""
                if not any(g in txt for g in current["group_names"]):
                    current.pop("group_names", None)
    _strip_leaked_draw_g(effect)
    _enrich_gain_abilities(effect)
    _mark_live_total_score(effect)
    _mark_live_total_clamp(effect, original_text or effect.get("text", "") or "")
    _mark_success_pile_difference(effect)
    _canonicalize_dynamic_counts(effect)
    effect = _walk_split_mixed(effect)
    effect = _walk_split_look(effect)
    _enrich_characters(effect)
    _clean_gain_resource(effect)
    _fix_select_self_and_other(effect)
    _fix_energy_difference_dynamic_count(effect)
    src = original_text or effect.get("text", "") or ""
    _stamp_mid_sentence_duration(effect, src)
    _mark_under_card_gain(effect, src)
    # "『X』のメンバーからバトンタッチして登場した場合" — the baton-touch source group
    # is a GATING CONDITION, not a target filter on the action steps. Attach it as
    # the effect's condition and strip the leaked baton_touch_trigger/group_names off
    # the effect + every action step (they would otherwise filter the actions' targets).
    src = original_text or effect.get("text", "") or ""
    if "からバトンタッチして登場した場合" in src:
        _attach_baton_touch_from_group_condition(effect, src)
    _infer_baton_placement_source(effect)
    return effect


# Fields a collapsed step inherits from the step it absorbed, when it has
# none of its own.
_COLLAPSE_FIELDS = ("duration", "all", "card_type", "target")


def _inherit_missing_fields(target, source, fields=_COLLAPSE_FIELDS, default_target=False):
    """Copy `fields` from `source` into `target`, but only where absent.

    `default_target` fills in "self" for an absent target as well, which the
    position-change collapse needs (a gain that states no target is always the
    player's own cards) and the sequential collapse does not.
    """
    for field in fields:
        if target.get(field):
            continue
        if source.get(field):
            target[field] = source[field]
        elif field == "target" and default_target:
            target["target"] = "self"


def _collapse_position_change_into_gain(act, gain):
    """Fold 「…ポジションチェンジする。その後その中获得」 into one timed gain."""
    gr = dict(gain)
    gr["timing_condition"] = "moved_this_turn"
    _inherit_missing_fields(
        gr, act, ("card_type", "all", "target"), default_target=True
    )
    return gr


def _collapsed_steps(actions):
    """The step list with the collapsible pairs folded together."""
    collapsed = []
    skip_next = False
    for i, act in enumerate(actions):
        if skip_next:
            skip_next = False
            continue
        if isinstance(act, dict) and act.get("action") == "position_change":
            nxt = actions[i + 1] if i + 1 < len(actions) else None
            if isinstance(nxt, dict) and nxt.get("action") == "gain_resource":
                collapsed.append(_collapse_position_change_into_gain(act, nxt))
                skip_next = True
                continue
        if isinstance(act, dict) and act.get("action") == "sequential":
            steps = act.get("actions", [])
            if len(steps) == 1:
                item = dict(steps[0])
                _inherit_missing_fields(item, act)
                collapsed.append(item)
                continue
        collapsed.append(act)
    return collapsed


def _collapse_position_changes(node):
    """Fold position_change → gain_resource and single-step sequentials away.

    Both shapes are two steps the engine can execute in one: a position change
    immediately followed by a gain of what moved, and a sequential wrapping a
    lone step. Running the two steps separately is what the engine would
    otherwise have to special-case.
    """
    if isinstance(node, dict):
        for v in node.values():
            _collapse_position_changes(v)
        if node.get("action") == "sequential":
            node["actions"] = _collapsed_steps(node.get("actions", []))
    elif isinstance(node, list):
        for item in node:
            _collapse_position_changes(item)
    return node


def _collect_gain(d, nodes):
    nodes.extend(
        current
        for current in iter_dict_nodes(d)
        if current.get("action") == "gain_ability" and current.get("ability_gain")
    )


def _enrich_gain_abilities(effect):
    gain_nodes = []
    _collect_gain(effect, gain_nodes)
    for node in gain_nodes:
        if "gained_effect" not in node:
            clean_gain = re.sub(r"【[^】]+】", "", node["ability_gain"]).strip()
            gained = parse_effect(clean_gain)
            if gained and gained.get("action") and _is_real_action(gained):
                node["gained_effect"] = gained


def _enrich_characters(d):
    for current in iter_dict_nodes(d):
        if current.get("action") in (
            "sequential",
            "conditional_on_optional",
            "conditional_on_result",
            "conditional_alternative",
            "look_and_select",
        ):
            continue
        if "text" not in current:
            continue
        text = current["text"]
        if not current.get("characters"):
            is_self_cost_set = (
                current.get("action") == "modify_cost"
                and current.get("operation") == "set"
                and ("このカード" in text or "このメンバーカード" in text)
            )
            cm = re.search(
                r"((?:「[^」]+」[か、]? ?)+)の(?:メンバーカード|ライブカード)", text
            )
            if cm and not is_self_cost_set:
                names = _quoted_names(cm.group(1))
                if names:
                    current["characters"] = names
        if not current.get("card_names"):
            cn = re.search(r"カード名(?:に|が)「([^」]+)」", text)
            if cn:
                current["card_names"] = [cn.group(1)]


# ====================================================================
# ====================================================================
# PROCESSING: process_abilities() & parse_ability()
# ====================================================================
# Top-level orchestration: loads abilities.json, runs post-processing
# fixes, semantic validation, and targeted patches for known parser gaps.
# Also handles gain_ability text re-parsing and the main entry point.
# ====================================================================


def _clean_gain_resource(node):
    """Remove inappropriate fields from every gain_resource action node."""
    for current in iter_dict_nodes(node):
        if current.get("action") != "gain_resource":
            continue
        res = current.get("resource")
        if res in ("blade", "ブレード"):
            current.pop("heart_colors", None)
        elif res in ("heart", "ハート") and current.get("heart_type") == "all":
            current.pop("heart_colors", None)
        current.pop("source", None)
        cond = current.get("condition", {})
        if current.get("position") and cond.get("position") == current.get("position"):
            trigger_types = (
                "movement_condition",
                "appearance_condition",
                "baton_touch",
                "state_change_condition",
            )
            is_trigger = cond.get("type") in trigger_types
            is_comparison_subject = cond.get("comparison_type") is not None
            is_highest_cost = cond.get("type") == "highest_cost_on_stage_condition"
            if is_trigger or is_comparison_subject or is_highest_cost:
                current.pop("position", None)


def _clean_per_unit_source(d):
    """Remove redundant fields on perform_yell with per_unit_source."""
    for current in iter_dict_nodes(d):
        if (
            current.get("per_unit")
            and current.get("action") == "perform_yell"
            and current.get("per_unit_source") == "previous_moved_cards"
        ):
            current.pop("per_unit_type", None)
            current.pop("count", None)


_PROPAGATE_CHILD_KEYS = (
    "condition",
    "primary_effect",
    "followup_action",
    "optional_action",
    "conditional_action",
    "alternative_effect",
    "actions",
    "options",
)


def _propagate_context(node, ctx=None, *, t="", eff_root=None):
    def enter(current, parent_ctx):
        context = {} if parent_ctx is None else parent_ctx
        action = current.get("action")
        child_ctx = dict(context)
        for field in (
            "location",
            "target",
            "card_type",
            "duration",
            "timing_condition",
            "all",
        ):
            if field in current:
                child_ctx[field] = current[field]

        if current.get("type") == "compound" and "conditions" in current:
            first_location = next(
                (
                    sub["location"]
                    for sub in current["conditions"]
                    if isinstance(sub, dict) and sub.get("location")
                ),
                None,
            )
            for sub in current["conditions"]:
                if not isinstance(sub, dict):
                    continue
                location = child_ctx.get("location") or first_location
                if location and not sub.get("location"):
                    if (
                        not sub.get("temporal")
                        and not sub.get("resource_type")
                        and sub.get("comparison_type") != "score"
                    ):
                        sub["location"] = location
            walk_dict_tree(
                current["conditions"],
                keys=_PROPAGATE_CHILD_KEYS,
                list_keys=("actions", "options"),
                context=child_ctx,
                enter=enter,
                leave=leave,
                child_context=lambda _node, node_ctx, _key, _child: node_ctx,
                after_child=lambda _node, _ctx, _key, _child: None,
            )

        if action == "sequential" and not child_ctx.get("duration") and t:
            if "ライブ終了時まで" in t:
                for act in current.get("actions", []):
                    if isinstance(act, dict) and act.get("action") in (
                        "gain_resource",
                        "change_state",
                        "move_cards",
                    ):
                        if act.get("duration") is None and "得る" in (
                            act.get("text", "") or ""
                        ):
                            act["duration"] = "live_end"
        return child_ctx

    def after_child(current, context, key, child):
        if key not in ("actions", "options"):
            if "location" in child and "location" not in current and "location" not in context:
                context["location"] = child["location"]
            if "location" not in child and "location" in context:
                child["location"] = context["location"]
        if (
            current.get("action") == "conditional_on_result"
            and key == "primary_effect"
            and eff_root
        ):
            outer_cond = eff_root.get("condition", {})
            if isinstance(outer_cond, dict) and outer_cond.get("card_type"):
                nested_cond = child.get("condition", {})
                if isinstance(nested_cond, dict) and not nested_cond.get("card_type"):
                    nested_cond["card_type"] = outer_cond["card_type"]

    def leave(current, _context):
        _infer_baton_touch(current, current.get("text", "") or "")

        nested = current.get("condition")
        if current.get("type") == "card_count_condition" and current is not nested:
            text = current.get("text", "")
            if "ブレードハートを持たない" in text or "ブレードハートがない" in text:
                if not current.get("card_property"):
                    current["card_property"] = "has_blade_heart"
            if HAS_SCORE_ICON in text and not current.get("card_property"):
                current["card_property"] = "has_score_icon"
            _infer_heart_source(current, text)
            _infer_baton_touch(current, text)

        if current.get("type") == "compound" and "conditions" in current:
            first_location = next(
                (
                    sub["location"]
                    for sub in current["conditions"]
                    if isinstance(sub, dict) and sub.get("location")
                ),
                None,
            )
            location = _context.get("location") or first_location
            for sub in current["conditions"]:
                if (
                    isinstance(sub, dict)
                    and not sub.get("location")
                    and location
                    and not sub.get("temporal")
                    and not sub.get("resource_type")
                    and sub.get("comparison_type") != "score"
                ):
                    sub["location"] = location
            for sub in current["conditions"]:
                if isinstance(sub, dict) and isinstance(sub.get("parenthetical"), list):
                    sub.pop("parenthetical", None)

        nested = current.get("condition")
        if (
            current.get("type") == "temporal_condition"
            and isinstance(nested, dict)
            and not nested.get("location")
            and "location" in _context
        ):
            nested["location"] = _context["location"]

        nested = current.get("condition")
        if isinstance(nested, dict) and nested.get("type") == "movement_condition":
            if nested.get("ability_filter") and not nested.get("card_type"):
                nested["card_type"] = "member_card"

        nested = current.get("condition")
        if (
            isinstance(nested, dict)
            and nested.get("type") == "temporal_condition"
            and nested.get("aggregate") == "total"
            and "必要ハート" in (nested.get("text", "") or "")
            and "成功" not in (nested.get("text", "") or "")
            and not nested.get("location")
        ):
            colors = nested.get("heart_colors") or []
            if len(colors) >= 6 or "{{icon_all.png" in (nested.get("text", "") or ""):
                nested["location"] = "live_card_zone"

        if (
            isinstance(eff_root, dict)
            and eff_root.get("action") == "gain_resource"
            and not eff_root.get("target")
            and eff_root.get("resource") == "heart"
        ):
            root_condition = eff_root.get("condition")
            if isinstance(root_condition, dict) and root_condition.get("target"):
                target = root_condition["target"]
                eff_root["target"] = "self" if target == "both" else target

    walk_dict_tree(
        node,
        keys=_PROPAGATE_CHILD_KEYS,
        list_keys=("actions", "options"),
        context={} if ctx is None else ctx,
        enter=enter,
        leave=leave,
        child_context=lambda current, child_ctx, _key, _child: child_ctx,
        after_child=after_child,
    )


def _apply_recursive_fixes(d):
    for current in iter_dict_nodes(d):
        if current.get("type") == "compound" and current.get("conditions"):
            first_location = next(
                (
                    sub["location"]
                    for sub in current["conditions"]
                    if isinstance(sub, dict) and sub.get("location")
                ),
                None,
            )
            if first_location:
                for sub in current["conditions"]:
                    if (
                        isinstance(sub, dict)
                        and sub.get("type") == "temporal_condition"
                        and not sub.get("location")
                        and not sub.get("temporal")
                        and not sub.get("resource_type")
                        and sub.get("comparison_type") != "score"
                    ):
                        sub["location"] = first_location

        if current.get("type") == "appearance_condition" and "控え室から" in current.get(
            "text", ""
        ):
            if "appearance_source" not in current:
                current["appearance_source"] = "discard"

        if current.get("action") == "move_cards":
            t = current.get("text", "")
            if "これにより控え室に置いた" in t and "より" in t and "コストの低い" in t:
                if "cost_reference" not in current:
                    current["cost_reference"] = "previous_moved_card"
                    current["cost_limit_operator"] = "<"

        if (
            current.get("action") == "modify_score"
            and current.get("per_unit_type") == "member"
            and current.get("heart_colors")
        ):
            t = current.get("text", "")
            if "色につき" in t:
                current["per_unit_type"] = "heart_colors"


# The action each kind of chain step is recognised by. A step that opens one
# of these chains hands its cards to the move that follows: a selection hands
# over the selected cards, a look_at the looked-at ones, a baton touch the
# cards that arrived.
_CHAIN_KIND_BY_ACTION = {
    "select_cards": "select",
    "look_and_select": "select",
    "select": "select",
    "look_at": "look_at",
    "play_baton_touch": "baton_touch",
}

# 「それらを好きな順番でデッキの上に置く」 — a look-at chain that moves ALL of
# them, in any order, rather than a stated number.
_ALL_LOOKED_CARDS_ON_DECK_RE = re.compile(
    r"それら(?:のカード)?を好きな順番でデッキの上に置く[。]?"
)


def _infer_missing_action(sub):
    """Name the action of a chain step that only describes a shape."""
    if sub.get("source") and sub.get("destination"):
        sub["action"] = "move_cards"
    elif sub.get("actions"):
        sub["action"] = "sequential"


def _adopt_looked_at_source(sub):
    """A move straight after a look_at moves the cards that were looked at."""
    if sub.get("source") != "looked_at":
        sub["source"] = "looked_at"
    if _ALL_LOOKED_CARDS_ON_DECK_RE.fullmatch(sub.get("text", "").strip()):
        sub["all"] = True
        sub.pop("count", None)
    if sub.get("destination") == "discard" and sub.get("discard_remaining") is not False:
        sub["discard_remaining"] = False


def _adopt_selected_source(sub):
    """A move straight after a selection moves the selected cards."""
    if sub.get("source") != "selected_cards":
        sub["source"] = "selected_cards"
    if sub.get("count") is not None and "count" not in sub.get("text", ""):
        sub.pop("count", None)


def _adopt_baton_touch_source(sub, eff):
    """A move straight after a baton touch moves the cards that arrived."""
    if sub.get("source") != "those_cards":
        sub["source"] = "those_cards"
    if not sub.get("group_names") and eff.get("group_names"):
        sub["group_names"] = eff["group_names"]


def _fix_sequential_chain(eff):
    """Propagate card_type, infer missing actions, chain select/look_at → move_cards sources."""
    if eff.get("action") != "sequential":
        return
    parent_card_type = eff.get("card_type")
    # Which kind of chain step ran last. A move_cards clears the flag it
    # consumed and leaves the others alone: a select followed by a look_at
    # leaves the select flag set, so a later heart gain can still see it.
    chain = {"select": False, "look_at": False, "baton_touch": False}
    prev_source = None
    for sub in eff.get("actions", []):
        if not isinstance(sub, dict):
            for kind in chain:
                chain[kind] = False
            continue
        if not sub.get("card_type") and parent_card_type:
            sub["card_type"] = parent_card_type
        if not sub.get("action"):
            _infer_missing_action(sub)
        action = sub.get("action")
        opener = _CHAIN_KIND_BY_ACTION.get(action)
        if opener is not None:
            for kind in chain:
                chain[kind] = kind == opener
        elif action == "move_cards" and chain["look_at"]:
            _adopt_looked_at_source(sub)
            chain["look_at"] = False
        elif action == "move_cards" and chain["select"]:
            _adopt_selected_source(sub)
            chain["select"] = False
        elif action == "move_cards" and chain["baton_touch"]:
            _adopt_baton_touch_source(sub, eff)
            chain["baton_touch"] = False
        elif (
            action == "gain_resource"
            and sub.get("resource") == "heart"
            and chain["select"]
        ):
            sub_text = sub.get("text", "")
            if "選んだカードが持つ色" in sub_text or "これにより選んだカード" in sub_text:
                sub["heart_colors_from_selected_card"] = True
            chain["select"] = False
        else:
            for kind in chain:
                chain[kind] = False
        # Track source for empty_area propagation (e.g. "メンバーのいないエリアに登場")
        if sub.get("source"):
            prev_source = sub["source"]
        elif (
            action == "move_cards"
            and sub.get("destination") == "empty_area"
            and prev_source
        ):
            sub["source"] = prev_source


def _fix_condition_enrichment(eff, t):
    """FIX 8/8e/8f: condition card_property, temporal aggregate, need_heart_total."""
    cond = eff.get("condition")
    if isinstance(cond, dict):
        ct = cond.get("text", "") or t
        if cond.get("type") == "card_count_condition":
            if "ブレードハート" in ct:
                cond["card_property"] = "has_blade_heart"
                if "持たない" in ct or "ない" in ct:
                    cond["negation"] = True
            if HAS_SCORE_ICON in ct and not cond.get(
                "card_property"
            ):
                cond["card_property"] = "has_score_icon"
            _infer_heart_source(cond, ct)
            _infer_baton_touch(cond, ct)
        if cond.get("type") == "temporal_condition":
            changed = False
            has_req_heart = "必要ハート" in ct
            has_aggregate_keyword = "含まれ" in ct or "のうち" in ct
            has_total_or_each = "合計" in ct or "それぞれ" in ct
            if has_req_heart and has_aggregate_keyword and has_total_or_each:
                cond["aggregate"] = "total"
                changed = True
            if not cond.get("heart_colors"):
                found = _heart_ids(cond.get("text", "") or ct)
                if found:
                    cond["heart_colors"] = found
                    changed = True
            ct2 = cond.get("text", "") or ct
            if not cond.get("count"):
                cm = re.search(r"(\d+)以上", ct2)
                if cm:
                    cond["count"] = as_int(cm.group(1))
                    changed = True
    if HAS_SCORE_ICON in t:
        if eff.get("action") in ("move_cards", "select") and not eff.get(
            "card_property"
        ):
            eff["card_property"] = "has_score_icon"
        cond = eff.get("condition")
        if isinstance(cond, dict) and not cond.get("card_property"):
            cond["card_property"] = "has_score_icon"
    if (
        eff.get("per_unit")
        and not eff.get("need_heart_total")
        and not eff.get("dynamic_count")
    ):
        nh = re.search(r"ハートを(\d+)つ以上持つ", t)
        if nh:
            eff["need_heart_total"] = as_int(nh.group(1))
            eff["need_heart_operator"] = ">="


def _prefix_condition_reparse(ability, eff, cond):
    """Re-parse condition texts to pick up newer parser fields (cost_limit
    etc.), merging missing fields without overwriting existing values."""
    if not (isinstance(cond, dict) and cond.get("text")):
        return
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


# Ordered action inference for effects with no action: first field-set
# present wins. Same order as the legacy if-chain.
_ACTION_INFERENCE_RULES = (
    (("source", "destination"), "move_cards"),
    (("actions",), "sequential"),
    (("opponent_action",), "opponent_action"),
)


def _infer_effect_action(eff):
    if not eff.get("action"):
        for fields, action in _ACTION_INFERENCE_RULES:
            if all(f in eff for f in fields):
                eff["action"] = action
                return


def _fix_primary_negation(eff):
    # FIX 10: Primary effect fixes — negation condition
    pe = eff.get("primary_effect")
    if not isinstance(pe, dict):
        return
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
                neg_text = pet
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
    # all:false on single-target primary when parent has all:true
    if eff.get("all") and "all" not in pe and pe.get("count") == 1:
        pe["all"] = False


def _fix_compound_gain_split(eff, cond, t):
    # FIX 12: compound condition → split gain_resource into sequential with two actions
    if not (isinstance(cond, dict) and cond.get("type") == "compound"):
        return
    if eff.get("action") != "gain_resource":
        return
    et = eff.get("text", "") or t
    if ALL_HEART_ICON in et and BLADE_ICON in et:
        blade_count = et.count(BLADE_ICON)
        heart_count = et.count(ALL_HEART_ICON)
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


def _fix_spurious_sequential_change_state(eff):
    # FIX 13: Remove spurious change_state actions inside sequential containers.
    # When _try_implicit_sequential splits "これによりアクティブにしたメンバーと、このメンバーは"
    # the conjunction phrase "これにより...と" is incorrectly parsed as change_state.
    # This pollutes selected_cards with wrong members. Remove the spurious change_state
    # so the subsequent gain_resource with multiple_targets targets correctly.
    if eff.get("action") != "sequential":
        return
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


def _fix_auto_condition(ability, eff, t):
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
                    break


def _fix_conditional_on_result(eff, t):
    """Restructure sequential+そうした場合 into conditional_on_result."""

    # "ウェイト状態のメンバー1人をアクティブにする。これにより相手の…アクティブに
    # した場合、…" — the primary has no explicit owner, so ANY player's waited
    # member may be chosen. Without a target the engine defaults to self and
    # never offers the opponent's members, making the 相手 branch unreachable.
    prim = eff.get("primary_effect")
    rc = eff.get("result_condition")
    if (
        eff.get("action") == "conditional_on_result"
        and isinstance(prim, dict)
        and prim.get("action") == "change_state"
        and not prim.get("target")
        and isinstance(rc, dict)
        and rc.get("target") == "opponent"
    ):
        prim["target"] = "both"

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
    if not (
        eff.get("action") == "sequential"
        and "これにより" in t
        and not _has_blade_heart_seq
    ):
        return
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
                    cond_dict["count"] = as_int(count_m.group(1))
                cond_dict["operator"] = ">="
                if "余剰ハート" in cond_text:
                    cond_dict["resource_type"] = "surplus_heart"
                if cond_dict.get("resource_type"):
                    act["condition"] = cond_dict
                    act["text"] = action_text
                break
    if result_idx < 0:
        return
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
        _cor = {
            "action": "conditional_on_result",
            "primary_effect": primary,
            "result_condition": result_cond,
            "followup_action": followup,
        }
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


def _repair_corpus(data: Dict[str, Any]) -> None:
    """Post-processing: recursive fixes, action inference & engine compat fixes, post-hoc fixes."""
    _apply_recursive_fixes(data["unique_abilities"])

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

        # ---- Strip leaked heart_colors from conditions ----
        # heart_colors on a condition means "only count cards with this heart color".
        # If heart_colors is present but the condition text has no {{heart_ icons,
        # it's effect metadata that leaked into a pure-count condition. Strip it.
        if isinstance(cond, dict) and cond.get("heart_colors"):
            cond_text = cond.get("text", "")
            if "{{heart_" not in cond_text:
                del cond["heart_colors"]

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

        _fix_conditional_on_result(eff, t)

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

    # Card-specific post-hoc fixes (e.g. PL!S-bp2-008 gain_ability) live in
    # card_overrides.py and are applied by the pipeline.


# Patterns whose appearance condition refers to the card's OWN debut. A
# self-appearance must stay a bare appearance with NO card_type filter, because
# the engine's self-trigger guard uses "card_type is absent" to require a real
# debut event. This is the SINGLE source of truth for that rule: it is applied
# once, over the whole effect tree, at the end of parsing.
_SELF_APPEARANCE_PATTERNS = ("このメンバーが登場", "このカードが登場")


def _strip_self_appearance_card_type(node):
    """Remove card_type from every self-appearance condition."""
    for current in iter_dict_nodes(node):
        if current.get("type") != "appearance_condition":
            continue
        text = current.get("text", "")
        if any(pattern in text for pattern in _SELF_APPEARANCE_PATTERNS):
            current.pop("card_type", None)


class _FinalizationPipeline:
    def _apply_card_specific_overrides(self, data):
        from card_overrides import apply_card_overrides

        apply_card_overrides(data)

    def _validate_structured_output(self, data):
        for ability in data.get("unique_abilities", []):
            effect = ability.get("effect")
            if isinstance(effect, dict):
                _validate_effect(effect, ability.get("full_text", ""))
                _strip_self_appearance_card_type(effect)

    def _normalize_generated_metadata(self, data):
        data["_warning"] = (
            "DO NOT EDIT THIS MANUALLY. Run cards/ability_extraction/parser.py to regenerate."
        )
        _clean_per_unit_source(data["unique_abilities"])

    def run(self, data):
        for ability in data["unique_abilities"]:
            effect = ability.get("effect")
            if isinstance(effect, dict):
                condition = effect.get("condition")
                _prefix_condition_reparse(ability, effect, condition)
                if isinstance(condition, dict) and condition.get("target") == "both" and condition.get("comparison_target"):
                    condition["target"] = "self"
                _infer_effect_action(effect)
                triggerless_text = ability.get("triggerless_text", "")
                _fix_condition_enrichment(effect, triggerless_text)
                _fix_primary_negation(effect)
                _fix_compound_gain_split(effect, condition, triggerless_text)
                _fix_spurious_sequential_change_state(effect)
                _fix_auto_condition(ability, effect, triggerless_text)
        _repair_corpus(data)
        self._apply_card_specific_overrides(data)
        self._validate_structured_output(data)
        self._normalize_generated_metadata(data)
        return data


def process_abilities(data: Dict[str, Any]) -> Dict[str, Any]:
    return _FinalizationPipeline().run(data)


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


def _strip_coo_child_optional(effect):
    """conditional_on_optional: the optionality belongs to the CONTAINER (the
    player's may-I choice), not its sub-actions — handlers emit optional=True
    on inner nodes from 「〜してもよい」, and leaving it there makes the engine
    double-prompt. Runs after `_propagate_optional` at parse time, replacing
    the post-parse FIX 3 sweep in the finalization pipeline."""
    if isinstance(effect, dict) and effect.get("action") == "conditional_on_optional":
        for sub_key in ("optional_action", "conditional_action"):
            sub = effect.get(sub_key)
            if isinstance(sub, dict):
                sub.pop("optional", None)
    return effect


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
        # 「(対戦相手のカードの効果でも発動する。)」 — structured stamp so the
        # engine never re-matches JP text at runtime (mirrors
        # watches_ability_resolution).
        if "発動する" in note and "相手" in note:
            target["fires_on_opponent_effects"] = True
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
                    detected_pos = detect_note_positions(note)
                    cond_parsed = {
                        "type": "location_condition",
                        "location": "stage",
                        "position": detected_pos[0] if len(detected_pos) == 1 else None,
                        "text": note,
                    }
                else:
                    cond_parsed = parse_condition(note)
                if _is_real_condition(cond_parsed):
                    target["activation_condition_parsed"] = cond_parsed
            # Detect all mentioned positions
            positions = detect_note_positions(note)
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


# Required field validators per action type.
# Only list fields the engine cannot default: count falls back to
# count_or(1)/dynamic_count/all paths, and modify_required_hearts reads
# value_or_count (value OR count), so requiring "count" there was a false
# positive that drowned the real signals.
_VALIDATORS = {
    "gain_resource": {"required": ["resource"]},
    "move_cards": {"required": ["source", "destination"]},
    "modify_score": {"required": ["operation", "value"]},
    "modify_required_hearts": {"required": ["heart_colors"]},
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
    "modify_yell_source",
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
    """Recursively validate condition types."""
    if not isinstance(cond, dict):
        return
    ctype = cond.get("type", "")
    if not ctype or ctype not in _KNOWN_CONDITIONS:
        raise ValueError(
            f"Unsupported condition type {ctype!r} in {context or '(root)'}"
        )
    for key in ("cause", "condition"):
        nested = cond.get(key)
        if nested is not None:
            if isinstance(nested, list):
                for sub in nested:
                    _validate_condition_types(sub, context)
            else:
                _validate_condition_types(nested, context)
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

    cond = eff.get("condition")
    if cond is not None:
        _validate_condition_types(cond, context)
    # Also check alternative_condition, choice_condition
    for cond_key in (
        "alternative_condition",
        "choice_condition",
        "result_condition",
        "activation_condition_parsed",
    ):
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
    heart_colors = _heart_ids_in_order(triggerless)
    # Never patch heart_colors onto a gain_resource effect. Heart-gain colors
    # must come from the parser's multiset logic (one token per granted heart);
    # the full-text heart icons here belong to a CONDITION/requirement clause
    # (e.g. "heart01..06がすべてある場合、ブレードを得る") and would otherwise
    # leak onto the gain.
    if (
        heart_colors
        and "heart_colors" not in effect
        and effect.get("action") != "gain_resource"
    ):
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


def _match_in_quotes(text, m):
    """Matched text inside 「」 quotes is probably an ability name, not effect context."""
    start = text.rfind("「", 0, m.start())
    end = text.find("」", m.end())
    return start != -1 and end != -1 and m.start() > start and m.end() < end


def _match_in_parens(text, m):
    """Matched text inside parenthetical clauses ( ... ) is game-rule reminder
    text, not actual card effect. Checks both ASCII and full-width parens."""
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


def _report_semantic_issue(all_issues, seen_by_rule, rule_name, cards, i, trigger, snippet, desc):
    """Append a semantic issue with per-rule+cards dedup.

    Unifies the dozen copy-pasted `dedup_key` blocks across the structural
    checks and the regex-rule loop below."""
    dedup_key = (rule_name, frozenset(cards) if cards else i)
    if dedup_key not in seen_by_rule:
        seen_by_rule[dedup_key] = True
        all_issues.append((rule_name, cards, trigger, snippet, desc))


def _check_change_state_energy_type(eff, t, cards, i, trigger, all_issues, seen_by_rule):
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


def _check_move_cards_cost_limit(eff, t, cards, i, trigger, all_issues, seen_by_rule):
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
            _report_semantic_issue(
                all_issues,
                seen_by_rule,
                "cost_limit",
                cards,
                i,
                trigger,
                t[max(0, t.find("コスト") - 10) : t.find("コスト") + 30],
                "'コスト' referenced in text but no cost_limit in effect or condition",
            )


def _check_look_select_reveal_hearts(eff, t, cards, i, trigger, all_issues, seen_by_rule):
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
                        _report_semantic_issue(
                            all_issues,
                            seen_by_rule,
                            "reveal_heart_colors",
                            cards,
                            i,
                            trigger,
                            t[:60],
                            "heart_colors on select parent but not on reveal sub-action",
                        )


def _check_sequential_sub_patterns(sub, j, acts, cards, i, trigger, all_issues, seen_by_rule):
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
                        "leaked_group_names",
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
                    # shared_condition_no_cache is an optimization hint, not a correctness issue
                    pass


def _check_sequential_patterns(eff, cards, i, trigger, all_issues, seen_by_rule):
    # Structural validation: sequential action patterns
    if eff.get("action") == "sequential":
        acts = eff.get("actions", [])
        for j, sub in enumerate(acts):
            if not isinstance(sub, dict):
                continue
            _check_sequential_sub_patterns(sub, j, acts, cards, i, trigger, all_issues, seen_by_rule)


def _check_structural_semantics(eff, t, cards, i, trigger, all_issues, seen_by_rule):
    """Non-regex structural checks for one ability entry.

    The dispatcher for every structural check, so adding one is a line here
    rather than a fourth argument list to remember at the call site.
    """
    _check_change_state_energy_type(eff, t, cards, i, trigger, all_issues, seen_by_rule)
    _check_move_cards_cost_limit(eff, t, cards, i, trigger, all_issues, seen_by_rule)
    _check_look_select_reveal_hearts(eff, t, cards, i, trigger, all_issues, seen_by_rule)
    _check_sequential_patterns(eff, cards, i, trigger, all_issues, seen_by_rule)


class SemanticRule(NamedTuple):
    """One "did the parser actually handle this phrase?" check.

    `pattern` matches the ability text; a hit outside quotes and parentheses is
    a candidate mechanic. `handled` receives (ability_entry, effect) and returns
    True when the parsed JSON already represents it — False is the reportable
    case, and means the phrase survived parsing with nothing standing for it.
    """

    name: str
    pattern: str
    handled: Callable[[dict, dict], bool]
    description: str


def _validate_semantic(abilities):
    """Validate parsed JSON against text patterns to find missing mechanics.

    Returns a list of issues: (rule_name, cards, trigger, snippet, desc).
    """
    # The rules are grouped by what they are about rather than by where the
    # phrase sits in the text, so a mechanic can be found by its family. The
    # order of this tuple does not affect the report: _print_semantic_report
    # groups hits by rule name and prints in its own priority order.
    RULES: Tuple[SemanticRule, ...] = (
        # ═══ Counting, selection and repetition ═══
        # Scaling sense only ("…1枚/1人/1つにつき"); "各グループ名につき" is
        # per-group selection inside look_and_select, not a count multiplier.
        SemanticRule(
            "per_unit",
            r"[枚人つ個]につき",
            lambda e, eff: _json_has_field(eff, "per_unit", True)
            or _json_has_field(eff, "dynamic_count")
            or _json_has_field(eff, "type", "dynamic_count"),
            "Per-unit scaling (につき) but no per_unit/dynamic_count structure",
        ),
        SemanticRule(
            "select_number",
            r"(数を選ん|[数数字]を[選選え])",
            lambda e, eff: _json_has_action(eff, "select_number")
            or _json_has_field(eff, "action", "select"),
            "Text says 'choose a number' but no select_number action found",
        ),
        SemanticRule(
            "distinct_name",
            r"(カード名の異なる|カード名が異なる|名前の異なる|名前が異なる|異なるカード名)",
            lambda e, eff: _json_has_field(eff, "distinct", "card_name"),
            "Distinct card names required but no distinct field",
        ),
        SemanticRule(
            "reveal_until",
            r"(公開するまで|まで公開し続け|現れるまで)",
            lambda e, eff: _json_has_action(eff, "reveal_until_live_card")
            or _json_has_action(eff, "reveal_until_chosen_card"),
            "Reveal until condition but no reveal_until action",
        ),
        SemanticRule(
            "repeat_procedure",
            r"(繰り返す|まで繰り返|もう一度行う|再度)",
            lambda e, eff: _json_has_action(eff, "repeat_procedure")
            or _json_has_field(eff, "repeat_limit"),
            "Repeat/loop described but no repeat_procedure or repeat_limit",
        ),
        SemanticRule(
            "discard_until",
            r"(枚になるまで.*捨て|枚になるまで.*トラッシュ|枚になるまで.*墓地|になるまで捨て|になるまでトラッシュ)",
            lambda e, eff: _json_has_action(eff, "discard_until_count"),
            "Discard until hand size but no discard_until_count",
        ),
        SemanticRule(
            "draw_until_hand",
            r"(枚になるまで|になるまで)",
            lambda e, eff: _json_has_action(eff, "draw_until_count")
            or _json_has_action(eff, "discard_until_count"),
            "Until-hand-size action but no draw_until_count or discard_until_count",
        ),
        SemanticRule(
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
        # ═══ Cost and payment ═══
        SemanticRule(
            "pay_energy_cost",
            r"(エネルギーを.*支払|エネルギー.*払う)",
            lambda e, eff: _json_has_action(eff, "pay_energy"),
            "Pay energy described but no pay_energy action",
        ),
        SemanticRule(
            "set_cost",
            r"(コストを.*変更|コストを.*増や|コストを.*減ら|コスト.*変え|支払うコスト)",
            lambda e, eff: _json_has_action(eff, "set_cost")
            or _json_has_action(eff, "modify_cost"),
            "Cost modification but no set_cost or modify_cost",
        ),
        SemanticRule(
            "state_change",
            r"(ウェイト|レスト|スタンド)(状態)?(にす|にで)(る|き)",
            lambda e, eff: _json_has_field(eff, "state_change")
            or _json_has_field(e.get("cost", {}), "state_change"),
            "State change described but no state_change field",
        ),
        SemanticRule(
            "shuffle",
            r"(シャッフルする|シャッフルして)",
            lambda e, eff: _json_has_action(eff, "shuffle")
            or _json_has_field(eff, "shuffle")
            or _json_has_field(e.get("cost", {}), "shuffle"),
            "Shuffle described but no shuffle action or flag",
        ),
        # ═══ Restrictions and negation ═══
        SemanticRule(
            "cannot_restriction",
            r"できない",
            lambda e, eff: _json_has_field(eff, "restriction_type")
            or _json_has_field(eff, "negation")
            or _json_has_field(eff, "conditional_negation")
            or _json_has_action(eff, "restriction"),
            "できない but no restriction/negation structure",
        ),
        SemanticRule(
            "restriction",
            r"(?<!支払)(?:ことが|を)?できない",
            lambda e, eff: _json_has_action(eff, "restriction")
            or _json_has_field(eff, "max_repeats"),
            "Restriction/cannot described but no restriction action or max_repeats",
        ),
        SemanticRule(
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
        ),
        SemanticRule(
            "invalidate_ability",
            r"(無効|発動しな|発動を防|無効にす|能力を.*失)",
            lambda e, eff: _json_has_action(eff, "invalidate_ability")
            or _json_has_action(eff, "suppress_ability_trigger"),
            "Ability nullification but no invalidate_ability or suppress_ability_trigger",
        ),
        # ═══ Card movement and zones ═══
        SemanticRule(
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
        SemanticRule(
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
        SemanticRule(
            "placement_order",
            r"(好きな順番|任意の順番|好きな順序|任意の順|好きな順)",
            lambda e, eff: _json_has_field(eff, "placement_order")
            or _json_has_field(e.get("cost", {}), "placement_order"),
            "Any-order placement but no placement_order field",
        ),
        # Energy returned to the energy deck must come from the zone. Covers both
        # effect moves and activation costs (エネルギーN枚を…デッキに置く：).
        # Engine defaults an empty source to discard, which never holds energy,
        # so a missing source makes the move silently no-op.
        SemanticRule(
            "energy_to_deck_source",
            r"エネルギー\d*枚をエネルギーデッキに置",
            lambda e, eff: _json_has(
                eff,
                lambda d: isinstance(d, dict)
                and d.get("action") == "move_cards"
                and d.get("destination") == "energy_deck"
                and bool(d.get("source")),
            )
            or _json_has(
                e.get("cost") or {},
                lambda d: isinstance(d, dict)
                and d.get("destination") == "energy_deck"
                and bool(d.get("source")),
            ),
            "Energy-to-energy-deck move but move/cost has no source "
            "(engine defaults empty source to discard, which never holds energy)",
        ),
        SemanticRule(
            "replacement_destination",
            r"成功ライブカード置き場に置く場合、代わりに",
            lambda e, eff: _json_has(
                eff,
                lambda d: isinstance(d, dict)
                and d.get("action") == "move_cards"
                and d.get("destination") == "success_live_zone",
            ),
            "Replacement effect but alternative move has no "
            "destination=success_live_zone",
        ),
        SemanticRule(
            "or_location",
            r"(?:成功)?ライブカード置き場(?:か(?!ら)|又は)",
            lambda e, eff: len(
                (eff.get("condition") or {}).get("locations", [])
            )
            >= 2,
            "OR location pattern but fewer than 2 locations in condition",
        ),
        # ═══ Types, identity and conversion ═══
        SemanticRule(
            "blade_type",
            r"ブレード(?:として扱|とみな|treat)",
            lambda e, eff: _json_has_action(eff, "set_blade_type"),
            "Blade type conversion but no set_blade_type",
        ),
        SemanticRule(
            "heart_type",
            r"ハート(?:として扱|とみな|treat)",
            lambda e, eff: _json_has_action(eff, "set_heart_type")
            or _json_has_action(eff, "all_blade_timing"),
            "Heart type conversion but no set_heart_type",
        ),
        SemanticRule(
            "card_identity",
            r"(としても扱|として扱う|同一として扱|として見な)",
            lambda e, eff: _json_has_action(eff, "set_card_identity")
            or _json_has_action(eff, "all_blade_timing")
            or _json_has_action(eff, "set_heart_type"),
            "Card identity/card name treated as but no set_card_identity",
        ),
        SemanticRule(
            "all_blade",
            r"(ALLブレード|全てのブレード|任意の色のブレード|ALL blade)",
            lambda e, eff: _json_has_field(eff, "all_blade_timing")
            or _json_has_action(eff, "all_blade_timing")
            or _json_has_action(eff, "set_blade_type")
            or _json_has_field(eff, "card_property", "has_all_blade"),
            "ALL blade / any-color handling but no all_blade_timing",
        ),
        SemanticRule(
            "baton_touch",
            r"バトンタッチして登場",
            lambda e, eff: _json_has_field(eff, "baton_touch_trigger")
            or _json_has_field(eff, "baton_touch_source"),
            "Baton touch but no baton_touch_trigger or baton_touch_source",
        ),
        SemanticRule(
            "non_stackable",
            r"重複しない",
            lambda e, eff: _json_has_field(eff, "non_stackable"),
            "Non-stackable described but no non_stackable flag",
        ),
        # ═══ Card properties and filters ═══
        SemanticRule(
            "card_property",
            r"(ブレードハートを持|ブレードハートがない|スコアを持つ)",
            lambda e, eff: _json_has_field(eff, "card_property")
            or _json_has_field(e.get("cost", {}), "card_property"),
            "Card property (blade heart / score icon) but no card_property field",
        ),
        SemanticRule(
            "same_name",
            r"同じ名前",
            lambda e, eff: _json_has_field(eff, "same_name")
            or _json_has_field(eff.get("condition", {}), "same_name"),
            "Same name required but no same_name field",
        ),
        SemanticRule(
            "heart_content",
            rf"必要ハートに含まれる{HEART_ICON}が\d+",
            lambda e, eff: _json_has_field(eff, "heart_colors")
            and _json_has_field(eff, "count"),
            "Heart content pattern but missing heart_colors or count",
        ),
        SemanticRule(
            "exclude_self",
            r"(自分以外|自身以外|このカード以外|このメンバー以外|自分を除く)",
            lambda e, eff: _json_has_field(eff, "exclude_self")
            or _json_has_field(e.get("cost", {}), "exclude_self"),
            "Exclude self described but no exclude_self field",
        ),
        # ═══ Targeting, choice and turn state ═══
        SemanticRule(
            "both_targets",
            r"(お互い|両プレイヤー|相手と自分(?!の)|自分と相手(?!の)|両方(?!(?:とも|ある)))",
            lambda e, eff: _json_has_field(eff, "target", "both")
            or _json_has_field(eff, "comparison_target"),
            "Both players affected but no target='both'",
        ),
        SemanticRule(
            "opponent_choice",
            r"相手[はが].*[選選え]ぶ",
            lambda e, eff: _json_has_field(eff, "action_by", "opponent"),
            "Opponent chooses but no action_by: opponent",
        ),
        SemanticRule(
            "additional_yell",
            r"(追加で.*エール|エール.*追加|もう一度.*エール|さらに.*エール|追エール)",
            lambda e, eff: _json_has_action(eff, "perform_yell")
            or _json_has_action(eff, "re_yell")
            or _json_has_action(eff, "modify_yell_count"),
            "Additional Yell but no perform_yell or re_yell",
        ),
        SemanticRule(
            "per_group",
            r"各グループ",
            lambda e, eff: _json_has_field(eff, "per_group")
            or _json_has_field(eff, "per_group_count")
            or _json_has_field(eff, "per_unit_type"),
            "Per-group described but no per_group/per_group_count/per_unit_type field",
        ),
        SemanticRule(
            "look_at",
            r"(見てもよい|見ることができる|(?<!必要ハートを)確認する)",
            lambda e, eff: _json_has_action(eff, "look_at"),
            "Look at cards described but no look_at action",
        ),
        SemanticRule(
            "lose_resource",
            r"失う",
            lambda e, eff: _json_has_field(eff, "sign", "negative"),
            "Lose resource described but no sign: negative",
        ),
        # The engine only implements "ALL moved cards match" as
        # card_count_condition{source: preceding_moved, operator: "="}.
        SemanticRule(
            "all_preceding_match",
            r"それらがすべて",
            lambda e, eff: _json_has(
                eff,
                lambda d: isinstance(d, dict)
                and d.get("type") == "card_count_condition"
                and d.get("source") == "preceding_moved"
                and d.get("operator") == "=",
            ),
            "'それらがすべて' (all moved cards match) but no "
            "card_count_condition with source=preceding_moved + operator='='",
        ),
        # Engine evaluates temporal=this_turn + location=deck as
        # deck_refreshed_this_turn — anything else loses the mechanic.
        SemanticRule(
            "refresh_condition",
            r"リフレッシュし(?:ていた|た)場合",
            lambda e, eff: _json_has(
                eff.get("condition") or {},
                lambda d: isinstance(d, dict)
                and d.get("location") == "deck"
                and d.get("temporal") == "this_turn",
            ),
            "Deck-refresh condition but condition is not "
            "{location: deck, temporal: this_turn}",
        ),
        SemanticRule(
            "exact_count",
            r"ちょうど\d+(人|枚|つ)",
            lambda e, eff: _json_has(
                eff,
                lambda d: isinstance(d, dict)
                and (
                    (
                        d.get("operator") in ("=", "==")
                        and d.get("count") is not None
                    )
                    # blade-count filters encode ちょうど as blade_limit + "=="
                    or d.get("blade_limit_operator") in ("=", "==")
                ),
            ),
            "'ちょうどN' (exactly N) but no condition with operator '='",
        ),
        # 「この効果ではライブの合計スコアは０未満にならない」 → represented as
        # score_floor:0 / effect_constraint:"min:0" on the parent and on each
        # modify_score child.
        SemanticRule(
            "effect_self_clamp",
            r"この効果では.{0,20}ない",
            lambda e, eff: _json_has_field(eff, "score_floor", 0)
            or _json_has_field(eff, "effect_constraint", "min:0"),
            "Self-referential effect clamp (この効果では…) has no structural representation",
        ),
    )

    seen_by_rule = {}  # rule_name -> set of frozenset(cards)
    all_issues = []  # (rule_name, cards, trigger, snippet, desc)

    for i, entry in enumerate(abilities):
        t = entry.get("triggerless_text", "") or entry.get("full_text", "")
        eff = entry.get("effect") or {}
        if not t or not isinstance(eff, dict):
            continue
        cards = entry.get("cards", [])
        trigger = entry.get("triggers", "")

        for rule in RULES:
            for m in re.finditer(rule.pattern, t):
                # Skip matches inside quoted ability names (「」) — those are text
                # references, not actual mechanic descriptions.
                if _match_in_quotes(t, m):
                    continue
                # Skip matches inside parenthetical clauses ( ) / （） — those
                # are game-rule reminder text, not card effect descriptions.
                if _match_in_parens(t, m):
                    continue
                # Report the phrase only when the JSON has nothing standing for it.
                if not rule.handled(entry, eff):
                    start = max(0, m.start() - 20)
                    end = min(len(t), m.end() + 30)
                    _report_semantic_issue(
                        all_issues,
                        seen_by_rule,
                        rule.name,
                        cards,
                        i,
                        trigger,
                        t[start:end],
                        rule.description,
                    )

        # ─── Structural checks (not regex-based) ─────────────────────────
        _check_structural_semantics(eff, t, cards, i, trigger, all_issues, seen_by_rule)

    _print_semantic_report(all_issues, len(abilities))
    return all_issues


def _print_semantic_report(all_issues, total_abilities):
    """Print the grouped missing-mechanics report. Extracted from
    `_validate_semantic` so the validator reads as collect → report."""
    # Group by rule
    by_rule = {}
    for r in all_issues:
        by_rule.setdefault(r[0], []).append(r)

    print("=" * 80)
    print("MISSING MECHANICS ANALYSIS")
    print("=" * 80)
    print(f"\nTotal abilities checked: {total_abilities}")
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
    listed = set(priority_order)

    # Priority rules first, in the order above; anything that triggered but is
    # not listed follows, sorted by name. The two groups differ only in how
    # many example cards they show, so they print through one path.
    ordered = [(name, 10) for name in priority_order if name in by_rule]
    ordered += [(name, 5) for name in sorted(by_rule) if name not in listed]

    for rule_name, sample_size in ordered:
        entries = by_rule[rule_name]
        print(f"\n{SEP}")
        print(f">> {rule_name}  ({len(entries)} abilities)")
        print(f"   {entries[0][4]}")
        print(SEP)
        for _, cards, trigger, snippet, _ in entries[:sample_size]:
            card_str = cards[0] if cards else "(no card)"
            print(f"\n  CARD: {card_str}")
            if trigger:
                print(f"  TRIGGER: {trigger}")
            print(f"  TEXT: ...{snippet}...")

    print(f"\n\n{'=' * 80}")
    print("END OF MISSING MECHANICS REPORT")
    print(f"{'=' * 80}")

    print(f"  Validation complete: {len(all_issues)} issues found")


def _list_rules() -> None:
    """Dump every registered rule across all four parse layers. Useful for
    seeing the whole phrase → shape mapping at once."""
    sep = "=" * 80
    print(sep)
    print("ACTION RULES  (_ACTION_RULES — explicit priority, registration order)")
    print(sep)
    for i, entry in enumerate(_ACTION_RULES):
        desc = (
            f"name={entry.name!r} priority={entry.priority:3} match={entry.match!r} "
            f"match_any={entry.match_any} action={entry.action!r}"
        )
        print(f"  {i:3}  {desc}")
    print(sep)
    print("COST HANDLERS  (_COST_HANDLERS — order = priority)")
    print(sep)
    for i, h in enumerate(_COST_HANDLERS):
        print(f"  {i:3}  {getattr(h, '__name__', h)}")
    print(sep)
    print("EFFECT RULES  (_EFFECT_RULES — canonical effect grammar)")
    print(sep)
    for i, r in enumerate(_EFFECT_RULES):
        print(
            f"  {i:3}  match={getattr(r, 'match', None)!r} match_any={getattr(r, 'match_any', None)} action={r.action!r}"
        )
    print(sep)
    print("STRUCTURAL EFFECT RULES  (_STRUCTURAL_EFFECT_RULES — canonical rules, priority 100+)")
    print(sep)
    for i, h in enumerate(_STRUCTURAL_EFFECT_RULES):
        print(f"  {i:3}  {getattr(h, '__name__', h)}")
    print(sep)
    print("CONDITION PATTERNS  (name, tier — tier*100 = priority base)")
    print(sep)
    for name, tier, handler in CONDITION_PATTERNS:
        print(f"  t{max(1, tier):3}  {name:40} {getattr(handler, '__name__', handler)}")


if __name__ == "__main__":
    if "--list-rules" in sys.argv:
        _list_rules()
        sys.exit(0)
    from extract_card_abilities import main

    main()
