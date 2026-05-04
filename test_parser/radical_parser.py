"""
radical_parser.py — A structure-first, data-driven ability parser.

The 3068-line current parser uses a cascade of procedural handlers:
if A then try B, elif C then try D, with ~37 effect handlers, ~25 condition
handlers, ~80 dispatch rules, and endless special cases. Every new card set
needs new rules, and the ordering of handlers is both critical and implicit.

This parser flips the approach entirely:

  1. STRUCTURE FIRST — identify the overall shape of the text before
     extracting details. Is this cost:effect? A conditional? Sequential?
     Per-unit? Each structure type self-identifies via markers.

  2. DATA-DRIVEN PATTERNS — ability structures are defined as DATA (dicts),
     not code. Each pattern declares: what markers to look for, how to split
     into sub-components, what sub-patterns apply, and how to assemble the
     result. The parser is a generic interpreter that walks pattern trees.

  3. SLOT-FILLING EXTRACTORS — instead of running every extractor on every
     text, we extract parameters only within the context of the matched
     structure. Each slot knows its domain (source, destination, count, etc.)
     and its extraction strategy.

  4. COMPOSABLE — patterns nest naturally. A sequential effect contains
     sub-effects. A conditional contains a condition + effect. A choice
     contains option effects. This mirrors the output JSON structure.

Adding a new pattern = adding ONE dict entry. Not writing a handler function,
not worrying about cascade ordering. The pattern declares what it needs.
"""

import re
from typing import Any, Dict, List, Optional, Callable


# =========================================================================
# PATTERN DSL — declarative descriptors for ability structures
# =========================================================================
# Each pattern is a dict with:
#   name        : str          — unique identifier
#   priority    : int          — matching priority (lower = checked first)
#   detect      : dict         — what to look for:
#       requires   : [str]       — all must be present in text
#       excludes   : [str]       — none must be present
#       any_of     : [str]       — at least one must be present
#       regex      : str         — a regex that must match
#       structural : str         — a pre-check like "has_colon"
#   split       : dict or None — how to divide text into parts:
#       at        : str         — delimiter
#       into      : [str]       — names for the parts
#       maxsplit  : int         — max splits (default 1)
#   children    : [str] or None — names of sub-patterns for each part
#   assemble    : callable      — (parsed_parts, raw_text) -> output dict
#   slots       : [dict] or None — extractions to run on the text

PATTERN_COST = {
    "name": "cost",
    "priority": 0,
    "detect": {},
    "children": {},
    "assemble": None,
}

PATTERN_EFFECT = {
    "name": "effect",
    "priority": 0,
    "detect": {},
    "children": {},
    "assemble": None,
}


def _has_structural_marker(text: str, marker: str) -> bool:
    if marker == "has_colon":
        return "：" in text
    if marker == "has_comma":
        return "、" in text
    if marker == "has_sequential_marker":
        return "その後、" in text
    if marker == "has_conditional_marker":
        for m in ("場合、", "とき、", "なら、"):
            if m in text:
                return True
        return False
    if marker == "has_choice_marker":
        return "以下から1つを選ぶ" in text
    if marker == "has_per_unit":
        return "につき" in text
    if marker == "has_each_time":
        return "たび" in text
    if marker == "has_kore_niyori":
        return "これにより" in text
    if marker == "has_look_and_select":
        return "その中から" in text
    if marker == "has_duration_marker":
        return "かぎり" in text
    if marker == "has_sou_shita":
        return "そうした場合" in text
    if marker == "has_conditional_alt":
        return "代わりに" in text
    if marker == "has_comma":
        return "、" in text
    if marker == "has_comma_and_shi":
        return "、" in text and "し" in text
    if marker == "has_furthermore":
        return "さらに" in text
    if marker == "has_parenthetical":
        return "（" in text and "）" in text
    return False


def _check_detect(text: str, detect: dict) -> bool:
    if not detect:
        return True
    for r in detect.get("requires", []):
        if r not in text:
            return False
    for r in detect.get("excludes", []):
        if r in text:
            return False
    for r in detect.get("any_of", []):
        if r in text:
            break
    else:
        if detect.get("any_of"):
            return False
    for m in detect.get("structural", []):
        if not _has_structural_marker(text, m):
            return False
    rgx = detect.get("regex")
    if rgx and not re.search(rgx, text):
        return False
    return True


# =========================================================================
# STRUCTURE TYPE PATTERNS (top-level effect structures)
# =========================================================================

STRUCTURE_PATTERNS = [
    # --- 0. Null/empty ---
    {
        "name": "null",
        "priority": 0,
        "detect": {"requires": [], "structural": []},
        "split": None,
        "match": lambda t: not t.strip(),
        "assemble": lambda p, t: {"action": "do_nothing", "text": t},
    },

    # --- 1. Duration prefix (ライブ終了時まで、...) ---
    {
        "name": "duration_prefix",
        "priority": 10,
        "detect": {"requires": [], "structural": []},
        "match": lambda t: any(
            t.startswith(p) for p in
            ["ライブ終了時まで", "このターンの間", "このライブの間",
             "ライブ終了時まで ", "このターンの間 ", "このライブの間 "]
        ),
        "split": None,
        "assemble": lambda p, t: None,  # handled inline
    },

    # --- 2. Cost-effect (： separator) ---
    {
        "name": "cost_effect",
        "priority": 20,
        "detect": {"structural": ["has_colon"]},
        "split": {"at": "：", "into": ["cost_text", "effect_text"], "maxsplit": 1},
        "assemble": lambda p, t: {
            "text": t,
            "cost": p["cost_text"],
            "effect_text": p["effect_text"],
        },
    },

    # --- 3. Conditional (場合、/ とき、/ なら、) ---
    # Checked BEFORE sequential/te-form so "A場合、B" isn't split as sequential
    {
        "name": "conditional",
        "priority": 30,
        "detect": {"structural": ["has_conditional_marker"]},
        "split": {
            "at": None,
            "custom_split": lambda t: _split_condition(t),
            "into": ["condition_text", "action_text"],
        },
        "assemble": lambda p, t: {
            "text": t,
            "condition_text": p["condition_text"],
            "action_effect": p["action_text"],
            "_is_conditional": True,
        },
    },

    # --- 4. Sequential (その後、) ---
    {
        "name": "sequential_sequential",
        "priority": 40,
        "detect": {"structural": ["has_sequential_marker"]},
        "split": {"at": "その後、", "into": ["first", "second"], "maxsplit": 1},
        "assemble": lambda p, t: {
            "text": t,
            "_is_sequential": True,
            "_sequential_parts": [p["first"], p["second"]],
        },
    },

    # --- 5. そうした場合 (conditional on optional action) ---
    {
        "name": "conditional_sequential",
        "priority": 50,
        "detect": {"structural": ["has_sou_shita"]},
        "split": {"at": "そうした場合", "into": ["first", "second"], "maxsplit": 1},
        "assemble": lambda p, t: {
            "text": t,
            "_is_conditional_sequential": True,
            "_parts": [p["first"], p["second"]],
        },
    },

    # --- 6. Look-and-select (その中から) ---
    {
        "name": "look_and_select",
        "priority": 60,
        "detect": {"structural": ["has_look_and_select"]},
        "split": {
            "at": None,
            "custom_split": lambda t: (
                re.search(r"(.+?)その中から(.+)", t).group(1),
                re.search(r"(.+?)その中から(.+)", t).group(2),
            ) if re.search(r"(.+?)その中から(.+)", t) else (t, ""),
            "into": ["look_text", "select_text"],
        },
        "assemble": lambda p, t: {
            "text": t,
            "_is_look_and_select": True,
            "look_text": p["look_text"],
            "select_text": p["select_text"],
        },
    },

    # --- 7. 代わりに (conditional alternative) ---
    {
        "name": "conditional_alternative",
        "priority": 70,
        "detect": {"structural": ["has_conditional_alt"]},
        "split": {"at": "代わりに", "into": ["primary_text", "alt_text"], "maxsplit": 1},
        "assemble": lambda p, t: {
            "text": t,
            "primary_text": p["primary_text"],
            "alt_text": p["alt_text"],
            "_is_conditional_alternative": True,
        },
    },

    # --- 8. さらに (furthermore) ---
    {
        "name": "furthermore",
        "priority": 80,
        "detect": {"structural": ["has_furthermore"]},
        "split": {
            "at": None,
            "custom_split": lambda t: _split_furthermore(t),
            "into": ["parts"],
        },
        "assemble": lambda p, t: {
            "text": t,
            "_is_sequential": True,
            "_sequential_parts": p["parts"],
        },
    },

    # --- 9. 以下から1つを選ぶ (choice) ---
    {
        "name": "choice",
        "priority": 90,
        "detect": {"structural": ["has_choice_marker"]},
        "split": None,
        "assemble": lambda p, t: {"text": t, "_is_choice": True},
    },

    # --- 10. これにより (complex cascade) ---
    {
        "name": "kore_niyori",
        "priority": 100,
        "detect": {"structural": ["has_kore_niyori"]},
        "split": None,
        "assemble": lambda p, t: {"text": t, "_is_kore_niyori": True},
    },

    # --- 11. Per-unit (につき) — before te-form sequential ---
    {
        "name": "per_unit",
        "priority": 110,
        "detect": {"structural": ["has_per_unit"]},
        "split": {
            "at": None,
            "custom_split": lambda t: _split_per_unit(t),
            "into": ["per_text", "action_text"],
        },
        "assemble": lambda p, t: {
            "text": t,
            "per_text": p["per_text"],
            "action_text": p["action_text"],
            "_is_per_unit": True,
        },
    },

    # --- 12. 回答が (answer-based choice) ---
    {
        "name": "answer_choice",
        "priority": 120,
        "detect": {"requires": ["回答が"]},
        "split": None,
        "assemble": lambda p, t: {"text": t, "_is_answer_choice": True},
    },

    # --- 13. たび (each time) ---
    {
        "name": "each_time",
        "priority": 130,
        "detect": {"structural": ["has_each_time"]},
        "split": None,
        "assemble": lambda p, t: {"text": t, "_is_each_time": True},
    },

    # --- 14. A連用形、B (te-form/continuative sequential) ---
    # Japanese uses the continuative form (連用形) to chain actions:
    #   引く→引き (godan), する→し (irregular), 来る→き (irregular)
    # The first part ends with a continuative suffix like き, ぎ, び, み, り, い, ち, し.
    # We detect this by checking if first comma-separated part ends with
    # a known continuative ending AND contains a verb.
    {
        "name": "te_form_sequential",
        "priority": 140,
        "detect": {"structural": ["has_comma"]},
        "split": {
            "at": "、",
            "into": ["parts"],
            "multiple": True,
        },
        "match": lambda t: (
            "、" in t and not any(m in t for m in
                ["場合、", "とき、", "なら、", "以下から1つを選ぶ"])
            and _is_continuative_sequential(t)
        ),
        "assemble": lambda p, t: {
            "text": t,
            "_is_sequential": True,
            "_sequential_parts": p["parts"],
        },
    },

    # --- 15. Period-separated sequential (A。B) ---
    # Must be checked BEFORE opponent_action and AFTER more specific patterns
    # like look_and_select, kore_niyori, etc. that also contain periods.
    {
        "name": "period_sequential",
        "priority": 145,
        "detect": {"structural": []},
        "match": lambda t: (
            "。" in t
            and not any(m in t for m in
                ["場合、", "とき、", "なら、", "その中から", "そうした場合",
                 "以下から1つを選ぶ", "代わりに", "回答が", "たび",
                 "さらに", "これにより"])
            and len([p for p in t.split("。") if p.strip()]) >= 2
        ),
        "split": {
            "at": None,
            "custom_split": lambda t: (
                [p.strip() for p in t.split("。") if p.strip()],
            ),
            "into": ["parts"],
            "multiple": True,
        },
        "assemble": lambda p, t: {
            "text": t,
            "_has_periods": True,
            "_period_parts": p["parts"] if isinstance(p.get("parts"), list) and p["parts"] else [],
        },
    },

    # --- 16. 相手は、 (opponent action) ---
    {
        "name": "opponent_action",
        "priority": 150,
        "detect": {"requires": ["相手は、"]},
        "split": None,
        "assemble": lambda p, t: {"text": t, "_is_opponent_action": True},
    },
]


def _split_condition(text: str):
    for keyword in ["場合", "とき", "なら"]:
        pattern = keyword + "、"
        if pattern in text:
            idx = text.find(keyword)
            comma_idx = idx + len(keyword)
            return text[:comma_idx].strip(), text[comma_idx + 1:].strip()
    return text, ""


def _split_furthermore(text: str):
    parts = [p.strip() for p in text.split("。") if p.strip()]
    result = []
    for p in parts:
        if "さらに" in p:
            p = p.replace("さらに", "", 1).strip()
        result.append(p)
    return result if result else [text]


def _split_per_unit(text: str):
    m = re.search(r"(.+?)(につき|ごとに)", text)
    if m:
        per_text = m.group(1).strip()
        action_text = text[m.end():].strip().lstrip("、")
        return per_text, action_text
    return text, ""


# Continuative verb endings used for sequential actions in Japanese
# 連用形 (ren'youkei) endings: い, き, ぎ, し, じ, ち, に, び, み, り
_CONTINUATIVE_ENDINGS = ("い", "き", "ぎ", "し", "じ", "ち", "に", "び", "み", "り", "え")

# Action verbs that commonly appear in continuative form before commas
_CONTINUATIVE_VERBS = (
    "引き", "置き", "加え", "選び", "公開し", "支払い",
    "見", "使い", "戻し", "獲得し", "失い", "送り",
)


def _is_continuative_sequential(text: str) -> bool:
    """Check if comma-separated text uses continuative form chaining.

    True if: the first comma-separated part ends with a known continuative
    verb form, suggesting A-ren'youkei、B sequential structure.
    """
    if "、" not in text:
        return False
    first_part = text.split("、", 1)[0].strip()
    # Check if ends with known continuative verb
    for v in _CONTINUATIVE_VERBS:
        if first_part.endswith(v):
            return True
    # Check if last character is a continuative ending
    if first_part and first_part[-1] in _CONTINUATIVE_ENDINGS:
        return True
    return False


# =========================================================================
# GENERIC PATTERN INTERPRETER
# =========================================================================

def match_structure(text: str) -> dict:
    """Identify the structural skeleton of a text fragment.

    FIRST MATCH WINS for the primary pattern (_pattern field).
    Later matching patterns still set their annotation flags (_is_*)
    and extract parts, allowing compatible patterns to coexist.

    Priority is determined by list order (ascending priority = checked first).
    """
    result = {"text": text, "_raw_text": text}
    _primary_set = False

    for pattern in STRUCTURE_PATTERNS:
        if "match" in pattern:
            if not pattern["match"](text):
                continue
        elif not _check_detect(text, pattern.get("detect", {})):
            continue

        if pattern["split"]:
            sp = pattern["split"]
            if "custom_split" in sp:
                parts = sp["custom_split"](text)
                if sp.get("multiple"):
                    result[sp["into"][0]] = parts if isinstance(parts, list) else [parts]
                else:
                    for i, name in enumerate(sp["into"]):
                        result[name] = parts[i] if isinstance(parts, (list, tuple)) else parts
            else:
                if sp.get("multiple"):
                    parts = text.split(sp["at"])
                    result[sp["into"][0]] = parts
                else:
                    maxsplit = sp.get("maxsplit", 1)
                    parts = text.split(sp["at"], maxsplit)
                    for i, name in enumerate(sp["into"]):
                        if i < len(parts):
                            result[name] = parts[i].strip()
                        else:
                            result[name] = ""

        assembly = pattern["assemble"]
        if assembly:
            extra = assembly(result, text)
            if extra:
                result.update(extra)

        # FIRST match wins for primary pattern; rest still annotate
        if not _primary_set:
            result["_pattern"] = pattern["name"]
            _primary_set = True

    return result


# =========================================================================
# SLOT EXTRACTORS — fine-grained parameter extraction
# =========================================================================
# These are small, focused functions that extract specific slots from text.
# They are called ONLY in the context of a matched structure, not on every
# text fragment.

def extract_source(text: str) -> Optional[str]:
    if "手札を" in text or "手札から" in text or "手札の" in text:
        return "hand"
    if "控え室から" in text or "控え室にある" in text or "控え室を" in text:
        return "discard"
    if "デッキの上から" in text or "デッキの一番上から" in text:
        return "deck_top"
    if "デッキから" in text or "山札から" in text:
        return "deck"
    if "デッキの一番下から" in text:
        return "deck_bottom"
    if "ステージから" in text:
        return "stage"
    if "エネルギー置き場から" in text:
        return "energy_zone"
    if "ライブカード置き場から" in text:
        return "live_card_zone"
    if "成功ライブカード置き場から" in text:
        return "success_live_zone"
    if "公開したカード" in text or "これにより公開した" in text:
        return "revealed_cards"
    if "このカードを手札に加えてもよい" in text:
        return "revealed_card"
    if "これにより公開されたほかのすべてのカード" in text:
        return "revealed_remaining"
    if "手札にある" in text:
        return "hand"
    return None


def extract_destination(text: str) -> Optional[str]:
    if "手札に加える" in text or "手札に加えて" in text:
        return "hand"
    if "控え室に置く" in text or "控え室に置いて" in text or "控え室に送る" in text:
        return "discard"
    if "デッキの一番上に置く" in text or "デッキの上に置く" in text or "山札の上に置く" in text:
        return "deck_top"
    if "デッキの一番下に置く" in text or "デッキの下に置く" in text or "山札の下に置く" in text:
        return "deck_bottom"
    if "デッキに置く" in text or "デッキに戻す" in text:
        return "deck"
    if "ステージに登場させる" in text or "登場させる" in text:
        return "stage"
    if "エネルギー置き場に置く" in text:
        return "energy_zone"
    if "ライブカード置き場に置く" in text:
        return "live_card_zone"
    if "成功ライブカード置き場に置く" in text:
        return "success_live_zone"
    if "このメンバーの下に置く" in text or "このメンバーの下に置いて" in text:
        return "under_member"
    if "メンバーのいないエリア" in text:
        return "empty_area"
    if "そのメンバーがいたエリア" in text:
        return "same_area"
    return None


def extract_count(text: str) -> Optional[int]:
    m = re.search(r"(\d+)枚", text)
    if m:
        return int(m.group(1))
    m = re.search(r"(\d+)人", text)
    if m:
        return int(m.group(1))
    m = re.search(r"(\d+)つ", text)
    if m:
        return int(m.group(1))
    m = re.search(r"(\d+)回", text)
    if m:
        return int(m.group(1))
    return None


def extract_card_type(text: str) -> Optional[str]:
    if "メンバーカード" in text or ("メンバー" in text and "カード" in text):
        return "member_card"
    if "ライブカード" in text:
        return "live_card"
    if "エネルギーカード" in text:
        return "energy_card"
    if "カード" in text:
        return "card"
    return None


def extract_target(text: str) -> Optional[str]:
    if "自分と相手の" in text or "自分と相手は" in text:
        return "both"
    if "自分か相手の" in text:
        return "either"
    if "相手の" in text:
        return "opponent"
    if "自分の" in text:
        return "self"
    return None


def extract_optional(text: str) -> bool:
    return "もよい" in text or "てもよい" in text


def extract_max(text: str) -> bool:
    return "人まで" in text or "枚まで" in text


def extract_state_change(text: str) -> Optional[str]:
    if "ウェイトにする" in text or "ウェイトにし" in text or "ウェイト状態" in text:
        return "wait"
    if "アクティブにする" in text:
        return "active"
    return None


def extract_group_names(text: str) -> List[str]:
    return re.findall(r"『([^』]+)』", text)


def extract_energy_cost(text: str) -> int:
    return text.count("{{icon_energy.png|E}}")


# Action detection — maps verb phrases to action types
_ACTION_KEYWORDS = [
    (["シャッフル"], "shuffle"),
    (["入れ替える", "入れ替えて"], "swap"),
    (["ポジションチェンジ"], "position_change"),
    (["無効にする", "無効にし"], "invalidate_ability"),
    (["何もしない"], "do_nothing"),
    (["引く", "引き", "引いてもよい"], "draw_card"),
    (["置く", "置いて", "置き"], "move_cards"),
    (["加える", "加えて", "加え"], "move_cards"),
    (["登場させる", "登場させ"], "appear"),
    (["送る"], "move_cards"),
    (["戻す"], "move_cards"),
    (["公開する", "公開し"], "reveal"),
    (["見る", "見て"], "look_at"),
    (["選ぶ", "選ん"], "select"),
    (["得る", "得て"], "gain_resource"),
    (["ヤル"], "re_yell"),
    (["アクティブにする"], "change_state"),
    (["ウェイトにする"], "change_state"),
    (["起動でき"], "activate_ability"),
]


def detect_action(text: str, is_ability_gain=False) -> str:
    """Detect the action type from text keywords."""
    if is_ability_gain:
        return "gain_ability"
    for keywords, action in _ACTION_KEYWORDS:
        for kw in keywords:
            if kw in text:
                return action
    return "custom"


def _is_ability_gain(text: str) -> bool:
    """Detect if text describes gaining an ability (rather than a resource).

    Abilities are enclosed in 「」 and contain trigger icons like {{...}}.
    Resources are simple words like ブレード or ハート.
    """
    if "を得る" not in text and "得る" not in text:
        return False
    if "能力" in text and "を得る" in text:
        return True
    quoted = re.findall(r"「([^」]+)」", text)
    for q in quoted:
        if "{{" in q and "}}" in q:
            return True
    # "ライブの合計スコアを+Nする" patterns in quotes = ability text
    for q in quoted:
        if "スコア" in q or "常時" in q or "起動" in q or "登場" in q:
            return True
    return False


def extract_slots(text: str) -> dict:
    """Extract all parameter slots from text."""
    slots = {}
    s = extract_source(text)
    if s:
        slots["source"] = s
    d = extract_destination(text)
    if d:
        slots["destination"] = d
    c = extract_count(text)
    if c is not None:
        slots["count"] = c
    ct = extract_card_type(text)
    if ct:
        slots["card_type"] = ct
    t = extract_target(text)
    if t:
        slots["target"] = t
    if extract_optional(text):
        slots["optional"] = True
    if extract_max(text):
        slots["max"] = True
    sc = extract_state_change(text)
    if sc:
        slots["state_change"] = sc
    gns = extract_group_names(text)
    if gns:
        slots["group_names"] = gns
        slots["group"] = {"name": gns[0]}
    energy = extract_energy_cost(text)
    if energy:
        slots["energy"] = energy
    # Ability gain detection must come before general action detection
    ag = _is_ability_gain(text)
    action = detect_action(text, is_ability_gain=ag)
    if action != "custom":
        slots["action"] = action
    if ag:
        _extract_ability_text(slots, text)
    return slots


def _extract_ability_text(d: dict, text: str):
    """Extract gained ability text from 「」 brackets."""
    quoted = re.findall(r"「([^」]+)」", text)
    for q in quoted:
        if "{{" in q and "}}" in q:
            d["ability_text"] = q
            return
    # Fallback: first quoted text with trigger-like content
    for q in quoted:
        if any(t in q for t in ("スコア", "常時", "起動", "登場")):
            d["ability_text"] = q
            return


# =========================================================================
# ASSEMBLY — compose the full ability JSON from matched structure + slots
# =========================================================================

def assemble_effect(struct: dict) -> dict:
    """Walk the structural skeleton and fill in slots at each level.

    The key architectural decision: structure defines the TREE, slots
    fill the LEAVES. The structure tells us how to decompose; slots
    tell us what to extract at each level.
    """
    text = struct.get("_raw_text", struct.get("text", ""))
    pattern = struct.get("_pattern", "simple")
    result = {"text": text}

    if pattern == "cost_effect":
        cost_text = struct.get("cost_text", "")
        effect_text = struct.get("effect_text", "")
        result["effect"] = assemble_effect(match_structure(effect_text))
        result["cost"] = assemble_cost(cost_text)
        return result

    if pattern == "conditional":
        cond_text = struct.get("condition_text", "")
        act_text = struct.get("action_effect", "")
        parsed_effect = assemble_effect(match_structure(act_text))
        result["condition"] = assemble_condition(cond_text)
        result.update(parsed_effect)
        return result

    if pattern == "sequential_sequential":
        parts = struct.get("_sequential_parts", [])
        actions = [assemble_effect(match_structure(p)) for p in parts if p.strip()]
        if len(actions) > 1:
            return {"text": text, "action": "sequential", "actions": actions}
        return actions[0] if actions else result

    if pattern == "look_and_select":
        look_text = struct.get("look_text", "")
        select_text = struct.get("select_text", "")
        look_action = extract_slots(look_text)
        look_action["text"] = look_text
        look_action.setdefault("action", "look_at")
        look_action.setdefault("source", "deck_top")
        select_action = _build_select_action(select_text)
        return {
            "text": text,
            "action": "look_and_select",
            "look_action": look_action,
            "select_action": select_action,
        }

    if pattern == "conditional_alternative":
        primary = struct.get("primary_text", "")
        alt = struct.get("alt_text", "")
        return {
            "text": text,
            "action": "conditional_alternative",
            "primary_effect": assemble_effect(match_structure(primary)),
            "alternative_effect": assemble_effect(match_structure(alt)),
        }

    if pattern == "per_unit":
        per_text = struct.get("per_text", "")
        act_text = struct.get("action_text", "")
        slots = extract_slots(act_text)
        slots["text"] = text
        slots["per_unit"] = True
        pm = re.search(r"(\d+)(人|枚|つ)(につき|ごとに)", text)
        if pm:
            slots["per_unit_count"] = int(pm.group(1))
            slots["per_unit_type"] = pm.group(2)
        else:
            for kw, t in [("メンバー", "member"), ("人", "member"), ("カード", "card"), ("枚", "card")]:
                if kw in per_text:
                    slots["per_unit_type"] = t
                    break
        return slots

    if pattern in ("te_form_sequential", "furthermore", "period_sequential"):
        if "_period_parts" in struct:
            parts = struct.get("_period_parts", [])
        else:
            parts = struct.get("_sequential_parts", [])
        if not isinstance(parts, list):
            parts = [parts]
        actions = [assemble_effect(match_structure(p)) for p in parts if isinstance(p, str) and p.strip()]
        if len(actions) > 1:
            valid = [a for a in actions if a.get("action") != "custom" or any(k not in ("text",) for k in a)]
            if len(valid) >= 2:
                return {"text": text, "action": "sequential", "actions": valid}
        return actions[0] if actions else result

    if pattern == "choice":
        return _assemble_choice(text)

    if pattern == "each_time":
        m = re.search(r"([^たび]+)たび", text)
        trigger_text = m.group(1).strip() if m else text
        rest_text = text[m.end():].strip() if m else ""
        sub = assemble_effect(match_structure(rest_text))
        sub["trigger_type"] = "each_time"
        sub["text"] = text
        return sub

    if pattern == "opponent_action":
        om = re.match(r"相手は、(.+?)。", text)
        if om:
            oa = extract_slots(om.group(1).strip())
            rest = text[len(om.group(0)):].strip()
            result["action_by"] = "opponent"
            result["opponent_action"] = oa
            if rest:
                re_eff = assemble_effect(match_structure(rest))
                result.update(re_eff)
            return result
        return result

    # Default: simple action — extract slots only
    slots = extract_slots(text)
    result.update(slots)
    if "action" not in result:
        # Infer from context
        if "エネルギー" in text and "アクティブ" in text:
            result["action"] = "change_state"
        elif "ブレード" in text or "ハート" in text:
            result["action"] = "gain_resource"
        elif "スコア" in text:
            result["action"] = "modify_score"
        elif "能力" in text and ("を得る" in text or "得る" in text):
            result["action"] = "gain_ability"
        elif "置く" in text or "置いて" in text or "加える" in text or "戻す" in text:
            result["action"] = "move_cards"
        else:
            result["action"] = "custom"

    # Post-processing for common fields
    if result.get("action") == "gain_resource":
        _infer_resource(result, text)
        result.setdefault("count", 1)
        dur = extract_duration(text)
        if dur:
            result["duration"] = dur

    if result.get("action") == "move_cards":
        result.setdefault("source", extract_source(text) or "any")
        result.setdefault("destination", extract_destination(text) or "any")
        result.setdefault("count", 1)
        ct = extract_card_type(text)
        if ct:
            result["card_type"] = ct

    if result.get("action") == "draw_card":
        result.setdefault("count", 1)
        result.setdefault("source", "deck")
        result.setdefault("destination", "hand")

    if result.get("action") == "change_state":
        sc = extract_state_change(text)
        if sc:
            result["state_change"] = sc

    if result.get("action") == "modify_score":
        vm = re.search(r"[+＋](\d+)", text)
        if vm:
            result["value"] = int(vm.group(1))
        result.setdefault("operation", "add")

    return result


def _infer_resource(d: dict, text: str):
    if "{{icon_blade.png|ブレード}}" in text:
        d["resource"] = "blade"
        d["count"] = text.count("{{icon_blade.png|ブレード}}")
    elif "{{icon_energy.png|E}}" in text:
        d["resource"] = "energy"
    elif "{{heart" in text:
        d["resource"] = "heart"
    elif "ブレード" in text:
        d["resource"] = "blade"
    elif "ハート" in text:
        d["resource"] = "heart"
    else:
        d["resource"] = "generic"


def extract_duration(text: str) -> Optional[str]:
    for p, c in [("ライブ終了時まで", "live_end"), ("このターンの間", "this_turn"),
                  ("このライブの間", "this_live")]:
        if p in text:
            return c
    return None


def _build_select_action(select_text: str) -> dict:
    """Build the select_action for その中から patterns."""
    if "手札に加え" in select_text and "残りを控え室に置く" in select_text:
        parts = re.split(r"[、。]", select_text)
        fa = extract_slots(parts[0].strip()) if parts else {}
        fa.setdefault("action", "move_cards")
        fa["destination"] = "hand"
        fa["source"] = "looked_at"
        fa["text"] = parts[0].strip() if parts else select_text
        sa = extract_slots(parts[1].strip()) if len(parts) > 1 else {}
        sa.setdefault("action", "move_cards")
        sa["destination"] = "discard"
        sa["source"] = "looked_at_remaining"
        sa["dynamic_count"] = {"type": "remaining_looked_at", "reference": "previous_look"}
        sa["text"] = parts[1].strip() if len(parts) > 1 else select_text
        return {"action": "sequential", "actions": [fa, sa], "text": select_text}

    if "好きな枚数を好きな順番でデッキの上に置き" in select_text and "残りを控え室に置く" in select_text:
        parts = select_text.split("、", 1)
        fa = extract_slots(parts[0].strip())
        fa.setdefault("action", "move_cards")
        fa["destination"] = "deck_top"
        fa["any_number"] = True
        fa["source"] = "looked_at"
        fa["text"] = parts[0].strip()
        sa = extract_slots(parts[1].strip())
        sa.setdefault("action", "move_cards")
        sa["destination"] = "discard"
        sa["source"] = "looked_at_remaining"
        sa["dynamic_count"] = {"type": "remaining_looked_at", "reference": "previous_look"}
        sa["text"] = parts[1].strip()
        return {"action": "sequential", "actions": [fa, sa], "text": select_text}

    slots = extract_slots(select_text)
    slots["text"] = select_text
    if slots.get("action") == "custom":
        if "手札に加える" in select_text:
            slots["action"] = "move_cards"
            slots["destination"] = "hand"
        elif "控え室に置く" in select_text:
            slots["action"] = "move_cards"
            slots["destination"] = "discard"
    return slots


def _assemble_choice(text: str) -> dict:
    parts = text.split("以下から1つを選ぶ", 1)
    result = {"text": text, "action": "choice"}
    if len(parts) > 1:
        opt_text = parts[1].strip()
        lines = opt_text.split("\n")
        options = []
        for line in lines:
            line = line.strip()
            if not line:
                continue
            if line.startswith("・"):
                ot = line[1:].strip()
                po = assemble_effect(match_structure(ot))
                po["text"] = ot
                options.append(po)
        if options:
            result["options"] = options
    return result


def assemble_cost(text: str) -> dict:
    """Parse a cost text into structured JSON."""
    cost = {"text": text}

    if not text.strip():
        return {"text": text, "type": "custom"}

    # Combined: energy icons + other action
    if "{{icon_energy.png|E}}" in text and text.strip().startswith("{{icon_energy.png|E}}"):
        energy_end = text.rfind("{{icon_energy.png|E}}") + len("{{icon_energy.png|E}}")
        energy_text = text[:energy_end].strip()
        other_text = text[energy_end:].strip()
        if other_text:
            other_cost = assemble_cost(other_text)
            if other_cost.get("type") not in (None, "custom"):
                return {
                    "text": text,
                    "type": "sequential_cost",
                    "costs": [
                        {"text": energy_text, "type": "pay_energy",
                         "energy": extract_energy_cost(energy_text)},
                        other_cost,
                    ],
                }

    # Sequential cost (～し、～)
    if "、" in text:
        parts = text.split("、")
        if len(parts) >= 2 and parts[0].strip().endswith("し"):
            cost_parts = [assemble_cost(p.strip()) for p in parts]
            return {"text": text, "type": "sequential_cost", "costs": cost_parts}

    # Reveal cost
    if "公開する" in text or "公開し" in text:
        cost["type"] = "reveal"
        cost["action"] = "reveal"
        if "手札" in text:
            cost["source"] = "hand"
        c = extract_count(text)
        if c:
            cost["count"] = c
        ct = extract_card_type(text)
        if ct:
            cost["card_type"] = ct
        return cost

    slots = extract_slots(text)

    # Energy cost
    en = extract_energy_cost(text)
    if en:
        cost["type"] = "pay_energy"
        cost["energy"] = en
        if "もよい" in text:
            cost["optional"] = True
        return cost

    # State change cost
    sc = extract_state_change(text)
    if sc and ("このメンバー" in text):
        cost["type"] = "change_state"
        cost["state_change"] = sc
        cost["card_type"] = "member_card"
        cost["self_cost"] = True
        if extract_optional(text):
            cost["optional"] = True
        return cost

    # Move cards cost
    src = extract_source(text)
    dst = extract_destination(text)
    if src or dst:
        cost["type"] = "move_cards"
        cost["action"] = "move_cards"
        if src:
            cost["source"] = src
        if dst:
            cost["destination"] = dst
        if "count" in slots:
            cost["count"] = slots["count"]
        if "card_type" in slots:
            cost["card_type"] = slots["card_type"]
        if extract_optional(text):
            cost["optional"] = True
        if "このメンバー" in text and "このメンバー以外" not in text:
            cost["self_cost"] = True
        return cost

    cost["type"] = "custom"
    return cost


def assemble_condition(text: str) -> dict:
    """Parse a condition text."""
    text = re.sub(r"[（）()]", "", text).strip()
    result = {"text": text, "type": "custom"}

    # Compound
    if "かつ" in text:
        parts = [p.strip() for p in text.split("かつ") if p.strip()]
        if len(parts) >= 2:
            parsed = [assemble_condition(p) for p in parts]
            return {"type": "compound", "operator": "and",
                    "conditions": parsed, "text": text}

    # Or
    if "か、" in text:
        parts = [p.strip() for p in text.split("か、") if p.strip()]
        if len(parts) >= 2:
            parsed = [assemble_condition(p) for p in parts]
            return {"type": "or_condition", "conditions": parsed, "text": text}

    slots = extract_slots(text)
    result.update(slots)

    # Card count condition
    m = re.search(r"(\d+)枚以上", text)
    if m:
        result["type"] = "card_count_condition"
        result["count"] = int(m.group(1))
        result["operator"] = ">="
        return result

    # Location condition
    loc = extract_source(text) or extract_destination(text)
    if loc and "card_type" in result:
        result["type"] = "location_condition"
        result["location"] = loc
        result.setdefault("target", "self")
        return result
    if loc:
        result["type"] = "location_condition"
        result["location"] = loc
        return result

    # Distinct names
    if "名前が異なる" in text:
        result["type"] = "location_condition"
        result["location"] = "stage"
        result["target"] = "self"
        result["distinct"] = True
        return result

    # Temporal
    if "このターン" in text:
        result["type"] = "temporal_condition"
        result["temporal"] = "this_turn"
        return result
    if "ライブ中" in text:
        result["type"] = "temporal_condition"
        result["temporal"] = "during_live"
        return result

    # Comparison
    for op_text, op in [("以上", ">="), ("以下", "<="), ("より少ない", "<"),
                         ("より多い", ">"), ("未満", "<"), ("超", ">")]:
        if op_text in text:
            result["operator"] = op
            break
    if "コスト" in text and "operator" in result:
        result["type"] = "comparison_condition"
        result["comparison_type"] = "cost"
        return result

    if "エネルギー" in text and "枚" in text:
        m = re.search(r"(\d+)枚以上", text)
        if m:
            result["type"] = "card_count_condition"
            result["count"] = int(m.group(1))
            result["operator"] = ">="
            result["resource_type"] = "energy"
            return result

    if result.get("type") == "custom" and slots:
        result.setdefault("target", "self")
        result.setdefault("count", 1)
        result.setdefault("operator", ">=")

    return result


# =========================================================================
# TOP-LEVEL API
# =========================================================================

def parse_ability(triggerless_text: str) -> dict:
    """Parse a complete ability text.

    This is the main entry point, analogous to parser.py's parse_ability.
    """
    text = triggerless_text.strip()
    result = {"triggerless_text": text}

    # Strip parenthetical notes
    without_paren = re.sub(r"（[^）]+）", "", text).strip()
    without_paren = re.sub(r"\([^)]+\)", "", without_paren).strip()

    # Strip suffix period
    without_paren = re.sub(r"[。.]$", "", without_paren).strip()

    # CRITICAL: triggerless_text already has trigger icons removed but
    # resource icons ({{icon_energy.png|E}}, {{heart_01.png|heart01}},
    # {{icon_blade.png|ブレード}}) are PRESERVED. Do NOT strip them.
    clean = without_paren

    # Identify structure first
    struct = match_structure(clean)

    if struct.get("_pattern") == "cost_effect":
        cost_text = struct.get("cost_text", "")
        effect_text = struct.get("effect_text", "")
        result["cost"] = assemble_cost(cost_text)
        result["effect"] = assemble_effect(match_structure(effect_text))
    else:
        result["effect"] = assemble_effect(struct)

    return result


def process_abilities(data: dict) -> dict:
    """Re-parse all abilities from triggerless_text."""
    for ability in data.get("unique_abilities", []):
        triggerless = ability.get("triggerless_text", "")
        if triggerless:
            parsed = parse_ability(triggerless)
            if "effect" in parsed:
                ability["effect"] = parsed["effect"]
            if "cost" in parsed:
                ability["cost"] = parsed["cost"]
    return data


if __name__ == "__main__":
    import json, sys
    from pathlib import Path

    abilities_file = Path(__file__).parent.parent / "cards" / "abilities.json"
    if not abilities_file.exists():
        print(f"File not found: {abilities_file}")
        sys.exit(1)

    with open(abilities_file, "r", encoding="utf-8") as f:
        data = json.load(f)

    result = process_abilities(data)

    out_file = Path(__file__).parent / "abilities_parsed.json"
    with open(out_file, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)

    print(f"Parsed {len(data['unique_abilities'])} abilities -> {out_file}")
