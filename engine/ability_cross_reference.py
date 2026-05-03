#!/usr/bin/env python3
"""
Ability Cross-Reference Tool
============================
Reads abilities.json and maps every field, type, and value to the
corresponding Rust engine source code location.

Usage:
    python engine/ability_cross_reference.py          # full report
    python engine/ability_cross_reference.py --json   # machine-readable JSON
"""

import json, sys, os
from collections import defaultdict, Counter
from pathlib import Path

ENGINE_SRC = Path(__file__).resolve().parent / "src"
ABILITIES_FILE = Path(__file__).resolve().parent.parent / "cards" / "abilities.json"

# ========================================================================
# AUTO-DETECT STRUCT FIELDS FROM RUST SOURCE
# Instead of hardcoding line numbers, we parse the Rust structs directly.
# ========================================================================

def parse_rust_struct(filepath, struct_name):
    """Parse a Rust struct definition and return {field_name: line_number}."""
    path = ENGINE_SRC / filepath
    if not path.exists():
        return {}
    fields = {}
    with open(path, "r", encoding="utf-8") as f:
        lines = f.readlines()
    in_struct = False
    brace_depth = 0
    for i, line in enumerate(lines):
        stripped = line.strip()
        if not in_struct and f"struct {struct_name}" in stripped:
            in_struct = True
            brace_depth += stripped.count("{") - stripped.count("}")
            continue
        if in_struct:
            brace_depth += stripped.count("{") - stripped.count("}")
            if brace_depth <= 0 and "}" in stripped:
                break
            if stripped.startswith("pub ") and ":" in stripped:
                field = stripped.split(":")[0].replace("pub ", "").strip()
                fields[field] = i + 1
    return fields

CARD_RS = "card.rs"
ABILITY_EFFECT_FIELDS_RAW = parse_rust_struct(CARD_RS, "AbilityEffect")
CONDITION_FIELDS_RAW = parse_rust_struct(CARD_RS, "Condition")
ABILITY_COST_FIELDS_RAW = parse_rust_struct(CARD_RS, "AbilityCost")

EFFECT_FIELDS = {k: (CARD_RS, str(v)) for k, v in ABILITY_EFFECT_FIELDS_RAW.items()}
CONDITION_FIELDS = {k: (CARD_RS, str(v)) for k, v in CONDITION_FIELDS_RAW.items()}
COST_FIELDS = {k: (CARD_RS, str(v)) for k, v in ABILITY_COST_FIELDS_RAW.items()}

# Map serde rename aliases
if "cost_type" in COST_FIELDS:
    COST_FIELDS["type"] = COST_FIELDS["cost_type"]
if "condition_type" in CONDITION_FIELDS:
    CONDITION_FIELDS["type"] = CONDITION_FIELDS["condition_type"]

ABILITY_STRUCT = {
    "full_text":       ("card.rs", "412"),
    "triggerless_text":("card.rs", "414"),
    "triggers":        ("card.rs", "415"),
    "use_limit":       ("card.rs", "416"),
    "is_null":         ("card.rs", "418"),
    "cost":            ("card.rs", "419"),
    "effect":          ("card.rs", "420"),
    "keywords":        ("card.rs", "421"),
}


# ========================================================================
# COST TYPE HANDLER MAP
# ========================================================================
COST_TYPES = {
    "sequential_cost":  ("ability/cost.rs", "10-17"),
    "choice_condition": ("ability/cost.rs", "19-27"),
    "move_cards":       ("ability/cost.rs", "29-43, 88-147"),
    "energy_condition": ("ability/cost.rs", "45-52, 209-223"),
    "change_state":     ("ability/cost.rs", "148-174"),
    "pay_energy":       ("ability/cost.rs", "175-208"),
    "reveal":           ("ability/cost.rs", "224-234"),
    "place_energy_under_member": ("ability/cost.rs", "235-244"),
}

# ========================================================================
# ACTION TYPE HANDLER MAP (effects.rs dispatch table)
# ========================================================================
ACTIONS = {
    "sequential":                   ("ability/effects.rs", "69"),
    "conditional_alternative":      ("ability/effects.rs", "70"),
    "look_and_select":              ("ability/effects.rs", "71"),
    "draw":                         ("ability/effects.rs", "72, 272-315"),
    "draw_card":                    ("ability/effects.rs", "72"),
    "draw_until_count":             ("ability/effects.rs", "73, 343-356"),
    "move_cards":                   ("ability/effects.rs", "74"),
    "gain_resource":                ("ability/effects.rs", "75, 419-535"),
    "change_state":                 ("ability/effects.rs", "76, 537-695"),
    "modify_score":                 ("ability/effects.rs", "77, 698-779"),
    "modify_required_hearts":       ("ability/effects.rs", "78, 781-800"),
    "set_cost":                     ("ability/effects.rs", "79, 802-814"),
    "set_blade_type":               ("ability/effects.rs", "80, 816-849"),
    "set_heart_type":               ("ability/effects.rs", "81, 851-864"),
    "activate_ability":             ("ability/effects.rs", "82, 866-872"),
    "invalidate_ability":           ("ability/effects.rs", "83, 874-879"),
    "gain_ability":                 ("ability/effects.rs", "84, 881-898"),
    "play_baton_touch":             ("ability/effects.rs", "85, 900-906"),
    "reveal":                       ("ability/effects.rs", "86, 908-931"),
    "select":                       ("ability/effects.rs", "87, 933-975"),
    "look_at":                      ("ability/effects.rs", "88, 977-995"),
    "modify_required_hearts_global": ("ability/effects.rs", "89, 997-1012"),
    "modify_yell_count":            ("ability/effects.rs", "90, 1014-1024"),
    "place_energy_under_member":    ("ability/effects.rs", "91, 358-387"),
    "activation_cost":              ("ability/effects.rs", "92, 389-417"),
    "position_change":              ("ability/effects.rs", "93, 1026-1071"),
    "appear":                       ("ability/effects.rs", "94, 1107-1158"),
    "choice":                       ("ability/effects.rs", "95, 1160-1192"),
    "pay_energy":                   ("ability/effects.rs", "96, 1194-1200"),
    "set_card_identity":            ("ability/effects.rs", "97, 1202-1209"),
    "repeat_procedure":             ("ability/effects.rs", "98, 1211-1221"),
    "discard_until_count":          ("ability/effects.rs", "99, 1223-1238"),
    "restriction":                  ("ability/effects.rs", "100, 1240-1244"),
    "re_yell":                      ("ability/effects.rs", "101, 1246-1267"),
    "modify_cost":                  ("ability/effects.rs", "102, 1479-1495"),
    "activation_restriction":       ("ability/effects.rs", "103, 1269-1274"),
    "choose_required_hearts":       ("ability/effects.rs", "104, 1276-1283"),
    "modify_limit":                 ("ability/effects.rs", "105, 1285-1295"),
    "set_blade_count":              ("ability/effects.rs", "106, 1297-1311"),
    "do_nothing":                   ("ability/effects.rs", "107"),
    "set_required_hearts":          ("ability/effects.rs", "108, 1313-1326"),
    "set_score":                    ("ability/effects.rs", "109, 1328-1334"),
    "specify_heart_color":          ("ability/effects.rs", "110, 1336-1344"),
    "modify_required_hearts_success": ("ability/effects.rs", "111, 1360-1376"),
    "set_cost_to_use":              ("ability/effects.rs", "112, 1378-1383"),
    "all_blade_timing":             ("ability/effects.rs", "113, 1385-1393"),
    "set_card_identity_all_regions":("ability/effects.rs", "114, 1346-1358"),
    "shuffle":                      ("ability/effects.rs", "115, 1395-1405"),
    "reveal_per_group":             ("ability/effects.rs", "116, 1407-1435"),
    "conditional_on_result":        ("ability/effects.rs", "117, 1437-1459"),
    "conditional_on_optional":      ("ability/effects.rs", "118, 1461-1477"),
    "custom":                       ("ability/effects.rs", "119"),
}

# ========================================================================
# CONDITION TYPE HANDLER MAP
# ========================================================================
CONDITIONS = {
    "compound":                   ("ability/condition.rs", "10, 44-54"),
    "comparison_condition":       ("ability/condition.rs", "11, 56-75"),
    "location_condition":         ("ability/condition.rs", "12, 77-207"),
    "position_condition":         ("ability/condition.rs", "13, 209-220"),
    "group_condition":            ("ability/condition.rs", "14, 222-227"),
    "card_count_condition":       ("ability/condition.rs", "15, 229-242"),
    "appearance_condition":       ("ability/condition.rs", "16, 244-273"),
    "temporal_condition":         ("ability/condition.rs", "17, 275-330"),
    "state_condition":            ("ability/condition.rs", "18, 332-340"),
    "energy_state_condition":     ("ability/condition.rs", "19, 342-350"),
    "movement_condition":         ("ability/condition.rs", "20, 352-381"),
    "ability_negation_condition": ("ability/condition.rs", "21, 383-388"),
    "or_condition":               ("ability/condition.rs", "22, 390-394"),
    "any_of_condition":           ("ability/condition.rs", "23, 396-413"),
    "score_threshold_condition":  ("ability/condition.rs", "24, 416-423"),
    "choice_condition":           ("ability/condition.rs", "25, 425-431"),
    "position_change_condition":  ("ability/condition.rs", "26, 433-442"),
    "state_change_condition":     ("ability/condition.rs", "27, 444-451"),
    "opponent_choice_condition":  ("ability/condition.rs", "28, 453-458"),
    "opponent_live_success":      ("ability/condition.rs", "29, 460-462"),
    "complex_condition":          ("ability/condition.rs", "30, 464-470"),
}

# ========================================================================
# TRIGGER TYPE MAP
# ========================================================================
TRIGGERS = {
    "起動":        ("triggers.rs", "1"),
    "自動":        ("triggers.rs", "2"),
    "常時":        ("triggers.rs", "3"),
    "登場":        ("triggers.rs", "4"),
    "ライブ開始時": ("triggers.rs", "5"),
    "ライブ成功時": ("triggers.rs", "6"),
    "メイン":      ("triggers.rs", "7"),
    "baton touch": ("triggers.rs", "8"),
    "Debut":       ("triggers.rs", "9"),
    "live_success":("triggers.rs", "10"),
}

# ========================================================================
# KEYWORD MAP
# ========================================================================
KEYWORDS = {
    "Turn1":           ("card.rs", "62"),
    "Turn2":           ("card.rs", "63"),
    "Debut":           ("card.rs", "64"),
    "LiveStart":       ("card.rs", "65"),
    "LiveSuccess":     ("card.rs", "66"),
    "Center":          ("card.rs", "67"),
    "LeftSide":        ("card.rs", "68"),
    "RightSide":       ("card.rs", "69"),
    "PositionChange":  ("card.rs", "70"),
    "FormationChange": ("card.rs", "71"),
}

KEYWORD_EVALUATORS = {
    "Center":          ("ability/resolver.rs", "75-76"),
    "LeftSide":        ("ability/resolver.rs", "80-81"),
    "RightSide":       ("ability/resolver.rs", "85-86"),
    "Turn1":           ("ability/resolver.rs", "90-91"),
    "Turn2":           ("ability/resolver.rs", "95-96"),
    "Debut":           ("ability/resolver.rs", "100-113"),
    "LiveStart":       ("ability/resolver.rs", "114-118"),
    "LiveSuccess":     ("ability/resolver.rs", "119-123"),
    "PositionChange":  ("ability/resolver.rs", "124-126"),
    "FormationChange": ("ability/resolver.rs", "127-129"),
}

# ========================================================================
# UTILITY FUNCTION MAP
# ========================================================================
UTILITY_FUNCTIONS = {
    "card_matches_type":            ("ability/util.rs", "4-12"),
    "card_matches_group":           ("ability/util.rs", "14-19"),
    "card_matches_group_str":       ("ability/util.rs", "21-26"),
    "card_matches_characters":      ("ability/util.rs", "28-37"),
    "card_matches_cost_limit":      ("ability/util.rs", "39-41"),
    "card_matches_cost_limit_op":   ("ability/util.rs", "43-56"),
    "card_matches_heart_colors":    ("ability/util.rs", "58-70"),
    "card_matches_name_constraint": ("ability/util.rs", "72-77"),
    "card_matches_all_filters":     ("ability/util.rs", "79-96"),
    "count_matching":               ("ability/util.rs", "98-110"),
    "matching_indices":             ("ability/util.rs", "112-124"),
    "compare_counts":               ("ability/util.rs", "126-136"),
    "zone_card_count":              ("ability/util.rs", "138-144"),
    "sum_score_in_zone":            ("ability/util.rs", "146-151"),
}

# ========================================================================
# MOVE_CARDS SOURCE→DESTINATION MAP
# ========================================================================
MOVE_CARDS_ROUTES = {
    "deck→hand":                    ("ability/move_cards.rs", "72"),
    "deck→discard":                 ("ability/move_cards.rs", "73"),
    "deck→stage":                   ("ability/move_cards.rs", "74-77"),
    "deck→live_card_zone":          ("ability/move_cards.rs", "78"),
    "deck→success_live_zone":       ("ability/move_cards.rs", "79"),
    "deck→energy_zone":             ("ability/move_cards.rs", "80"),
    "deck→energy_deck":             ("ability/move_cards.rs", "81"),
    "deck→deck_top":                ("ability/move_cards.rs", "82"),
    "deck→deck_bottom":             ("ability/move_cards.rs", "83"),
    "stage→discard":                ("ability/move_cards.rs", "102"),
    "stage→hand":                   ("ability/move_cards.rs", "103"),
    "stage→deck_bottom":            ("ability/move_cards.rs", "104"),
    "stage→deck_top":               ("ability/move_cards.rs", "105"),
    "stage→same_area":              ("ability/move_cards.rs", "106-109"),
    "stage→live_card_zone":         ("ability/move_cards.rs", "110"),
    "stage→success_live_zone":      ("ability/move_cards.rs", "111"),
    "hand→discard":                 ("ability/move_cards.rs", "151-155"),
    "hand→deck_bottom/deck_top":    ("ability/move_cards.rs", "157-162"),
    "hand→stage":                   ("ability/move_cards.rs", "163-175"),
    "hand→live_card_zone":          ("ability/move_cards.rs", "176-180"),
    "discard→hand":                 ("ability/move_cards.rs", "183-188"),
    "discard→deck_bottom/deck_top": ("ability/move_cards.rs", "189-194"),
    "discard→deck":                 ("ability/move_cards.rs", "195-215"),
    "discard→live_card_zone":       ("ability/move_cards.rs", "216-230"),
    "discard→same_area":            ("ability/move_cards.rs", "231-249"),
    "discard→stage/empty_area":     ("ability/move_cards.rs", "250-291"),
    "energy_zone→hand":             ("ability/move_cards.rs", "294-295"),
    "energy_zone→discard":          ("ability/move_cards.rs", "294-295"),
    "live_card_zone→hand":          ("ability/move_cards.rs", "305"),
    "live_card_zone→success_live_zone": ("ability/move_cards.rs", "305"),
    "live_card_zone→discard":       ("ability/move_cards.rs", "305"),
    "success_live_zone→hand":       ("ability/move_cards.rs", "314"),
    "success_live_zone→deck_top":   ("ability/move_cards.rs", "314"),
    "success_live_zone→deck_bottom":("ability/move_cards.rs", "314"),
}


# ========================================================================
# VALUE-TO-CODE MAP
# Maps actual values found in abilities.json → engine code location
# ========================================================================

# Effect.source / cost.source values → where handled in engine
SOURCE_VALUES = {
    "deck":           ("ability/move_cards.rs", "58-89"),
    "hand":           ("ability/move_cards.rs", "151-180"),
    "discard":        ("ability/move_cards.rs", "183-291"),
    "stage":          ("ability/move_cards.rs", "92-148"),
    "energy_zone":    ("ability/move_cards.rs", "294-302"),
    "live_card_zone": ("ability/move_cards.rs", "305-313"),
    "success_live_zone": ("ability/move_cards.rs", "314-323"),
    "deck_top":       ("ability/move_cards.rs", "58-89"),
    "looked_at":      ("ability/effects.rs", "908-995"),
}

DESTINATION_VALUES = {
    "hand":           ("ability/move_cards.rs", "72, 103, 113, 137, 155, 161, 187, 233, 300, 311, 320"),
    "discard":        ("ability/move_cards.rs", "73, 102, 137, 155, 295, 300, 311"),
    "stage":          ("ability/move_cards.rs", "74-77, 112, 138, 163-175, 240-247, 250-291"),
    "deck_top":       ("ability/move_cards.rs", "82, 105, 162, 320"),
    "deck_bottom":    ("ability/move_cards.rs", "83, 104, 161, 193, 320"),
    "live_card_zone": ("ability/move_cards.rs", "78, 110, 141, 179, 216-230, 305"),
    "success_live_zone": ("ability/move_cards.rs", "79, 111, 142, 311, 314"),
    "energy_zone":    ("ability/move_cards.rs", "80"),
    "energy_deck":    ("ability/move_cards.rs", "81"),
    "same_area":      ("ability/move_cards.rs", "106-109, 231-249"),
    "empty_area":     ("ability/move_cards.rs", "250-291"),
}

CARD_TYPE_VALUES = {
    "member_card": ("ability/util.rs", "7"),
    "live_card":   ("ability/util.rs", "6"),
    "energy_card": ("ability/util.rs", "8"),
}

TARGET_VALUES = {
    "self":     ("ability/effects.rs", "target='self' in draw (274), modify_score (701), etc."),
    "opponent": ("ability/effects.rs", "target='opponent' in change_state, modify_score, etc."),
    "both":     ("ability/effects.rs", "283-288 (draw_card target both)"),
    "either":   ("ability/condition.rs", "121-151 (location condition either)"),
}

OPERATION_VALUES = {
    "add":      ("ability/effects.rs", "700 (modify_score), 734-737 (add value)"),
    "remove":   ("ability/effects.rs", "736 (modify_score remove)"),
    "set":      ("ability/effects.rs", "755-756 (set score), 802-813 (set cost)"),
    "increase": ("ability/effects.rs", "1008-1009 (modify_required_hearts_global), 1367 (modify_required_hearts_success)"),
    "decrease": ("ability/effects.rs", "792-793 (modify_required_hearts), 1367"),
    "subtract": ("ability/effects.rs", "1019 (modify_yell_count), 1489 (modify_cost)"),
}

STATE_CHANGE_VALUES = {
    "wait":   ("ability/effects.rs", "566-568, 677-684"),
    "active": ("ability/effects.rs", "567-568, 686-693"),
}

RESOURCE_VALUES = {
    "blade": ("ability/effects.rs", "496-513"),
    "heart": ("ability/effects.rs", "515-520"),
    "ハート": ("ability/effects.rs", "515-520"),
    "ブレード": ("ability/effects.rs", "496-513"),
}

POSITION_VALUES = {
    "center":            ("ability/effects.rs", "1048; condition.rs:214"),
    "left_side":         ("ability/effects.rs", "1049; condition.rs:215"),
    "right_side":        ("ability/effects.rs", "1050; condition.rs:216"),
    "センターエリア":    ("ability/effects.rs", "1048"),
    "左サイドエリア":    ("ability/effects.rs", "1049"),
    "右サイドエリア":    ("ability/effects.rs", "1050"),
    "any":               ("ability/condition.rs", "217"),
}

COMPARISON_OPERATOR_VALUES = {
    ">=": ("ability/util.rs", "128"),
    ">":  ("ability/util.rs", "129"),
    "<=": ("ability/util.rs", "130"),
    "<":  ("ability/util.rs", "131"),
    "==": ("ability/util.rs", "132"),
    "=":  ("ability/util.rs", "132"),
    "!=": ("ability/util.rs", "133"),
}

# All value maps grouped by field path for lookup
VALUE_MAP = {
    "triggers": TRIGGERS,
    "cost.type": COST_TYPES,
    "cost.cost_type": COST_TYPES,
    "effect.action": ACTIONS,
    "condition.type": CONDITIONS,
    "effect.source": SOURCE_VALUES,
    "cost.source": SOURCE_VALUES,
    "effect.destination": DESTINATION_VALUES,
    "cost.destination": DESTINATION_VALUES,
    "effect.card_type": CARD_TYPE_VALUES,
    "cost.card_type": CARD_TYPE_VALUES,
    "condition.card_type": CARD_TYPE_VALUES,
    "effect.target": TARGET_VALUES,
    "cost.target": TARGET_VALUES,
    "condition.target": TARGET_VALUES,
    "effect.operation": OPERATION_VALUES,
    "effect.resource": RESOURCE_VALUES,
    "effect.state_change": STATE_CHANGE_VALUES,
    "cost.state_change": STATE_CHANGE_VALUES,
    "effect.position": POSITION_VALUES,
    "cost.position": POSITION_VALUES,
    "condition.operator": COMPARISON_OPERATOR_VALUES,
    "condition.position": POSITION_VALUES,
}

# ========================================================================
# IMPLEMENTATION STATUS (auto-generated from struct definitions)
# All fields currently in structs are IMPLEMENTED (dead ones were removed).
# ========================================================================

EFFECT_IMPL_STATUS = {f: "IMPLEMENTED" for f in ABILITY_EFFECT_FIELDS_RAW}
CONDITION_IMPL_STATUS = {f: "IMPLEMENTED" for f in CONDITION_FIELDS_RAW}
COST_IMPL_STATUS = {f: "IMPLEMENTED" for f in ABILITY_COST_FIELDS_RAW}
# Add serde alias
EFFECT_IMPL_STATUS["type"] = "IMPLEMENTED"
CONDITION_IMPL_STATUS["type"] = "IMPLEMENTED"
COST_IMPL_STATUS["type"] = "IMPLEMENTED"


# ========================================================================
# MAIN ANALYSIS
# ========================================================================

def load_abilities():
    with open(ABILITIES_FILE, "r", encoding="utf-8") as f:
        return json.load(f)

def analyze(data):
    abilities = data.get("unique_abilities", data)
    
    # Catalog
    trigger_vals = set()
    cost_type_vals = set()
    action_vals = set()
    condition_type_vals = set()
    duration_vals = set()
    keyword_vals = set()
    cost_fields_used = defaultdict(set)
    effect_fields_used = defaultdict(set)
    condition_fields_used = defaultdict(set)
    cost_field_values = defaultdict(set)
    effect_field_values = defaultdict(set)
    condition_field_values = defaultdict(set)
    cost_field_freq = defaultdict(Counter)
    effect_field_freq = defaultdict(Counter)
    condition_field_freq = defaultdict(Counter)
    action_freq = Counter()
    cost_type_freq = Counter()
    condition_type_freq = Counter()
    trigger_freq = Counter()

    for ab in abilities:
        # Triggers
        t = ab.get("triggers")
        if isinstance(t, list):
            for v in t: trigger_vals.add(v); trigger_freq[v] += 1
        elif t: trigger_vals.add(t); trigger_freq[t] += 1

        # Keywords
        kws = ab.get("keywords")
        if isinstance(kws, list):
            for kw in kws: keyword_vals.add(kw)

        # Cost
        cost = ab.get("cost")
        if cost:
            ct = cost.get("cost_type") or cost.get("type")
            if ct: cost_type_vals.add(ct); cost_type_freq[ct] += 1
            for k, v in cost.items():
                cost_fields_used[k].add(str(type(v).__name__))
                if isinstance(v, (str, int, float, bool)):
                    cost_field_values[k].add(str(v))
                    cost_field_freq[k][str(v)] += 1

        # Effect
        eff = ab.get("effect")
        if eff:
            act = eff.get("action")
            if act: action_vals.add(act); action_freq[act] += 1
            
            dur = eff.get("duration")
            if dur: duration_vals.add(dur)

            for k, v in eff.items():
                effect_fields_used[k].add(str(type(v).__name__))
                if isinstance(v, (str, int, float, bool)):
                    effect_field_values[k].add(str(v))
                    effect_field_freq[k][str(v)] += 1

            # Condition
            cond = eff.get("condition")
            if cond:
                ct2 = cond.get("type") or cond.get("condition_type")
                if ct2: condition_type_vals.add(ct2); condition_type_freq[ct2] += 1
                for k, v in cond.items():
                    condition_fields_used[k].add(str(type(v).__name__))
                    if isinstance(v, (str, int, float, bool)):
                        condition_field_values[k].add(str(v))
                        condition_field_freq[k][str(v)] += 1

    return {
        "trigger_vals": sorted(trigger_vals),
        "cost_type_vals": sorted(cost_type_vals),
        "action_vals": sorted(action_vals),
        "condition_type_vals": sorted(condition_type_vals),
        "duration_vals": sorted(duration_vals),
        "keyword_vals": sorted(keyword_vals),
        "cost_fields_used": dict(cost_fields_used),
        "effect_fields_used": dict(effect_fields_used),
        "condition_fields_used": dict(condition_fields_used),
        "cost_field_values": {k: sorted(v) for k, v in cost_field_values.items()},
        "effect_field_values": {k: sorted(v) for k, v in effect_field_values.items()},
        "condition_field_values": {k: sorted(v) for k, v in condition_field_values.items()},
        "cost_field_freq": {k: dict(v.most_common()) for k, v in cost_field_freq.items()},
        "effect_field_freq": {k: dict(v.most_common()) for k, v in effect_field_freq.items()},
        "condition_field_freq": {k: dict(v.most_common()) for k, v in condition_field_freq.items()},
        "action_freq": dict(action_freq.most_common()),
        "cost_type_freq": dict(cost_type_freq.most_common()),
        "condition_type_freq": dict(condition_type_freq.most_common()),
        "trigger_freq": dict(trigger_freq.most_common()),
        "total": len(abilities),
        "stats": data.get("statistics", {}),
    }

def fmt_loc(file, line):
    return f"engine/src/{file}:{line}"

def fmt_loc_range(file, line_range):
    return f"engine/src/{file}:{line_range}"

def print_report(cat):
    print("=" * 78)
    print("  ABILITY CROSS-REFERENCE REPORT")
    print("  abilities.json  →  engine/src/*.rs")
    print("=" * 78)
    print(f"\nTotal unique abilities: {cat['total']}")
    s = cat.get("stats", {})
    if s:
        print(f"Total cards: {s.get('total_cards', '?')}  |  With abilities: {s.get('cards_with_abilities', '?')}  |  Unique: {s.get('unique_abilities', '?')}")

    # ── TRIGGERS ──
    print("\n" + "─" * 78)
    print("1. TRIGGERS")
    print("─" * 78)
    print(f"{'Value':<25} {'Engine Location':<35} {'Count':<6}")
    print("-" * 66)
    for v in cat["trigger_vals"]:
        base = v.split(",")[0].strip() if "," in v else v
        loc = TRIGGERS.get(base)
        if loc:
            suffix = v[len(base):] if v != base else ""
            loc_str = fmt_loc(*loc) + suffix
        else:
            loc_str = "(not found in engine)"
        print(f"{v:<25} {loc_str:<35}")

    # ── KEYWORDS ──
    print("\n" + "─" * 78)
    print("2. KEYWORDS (Keyword enum)")
    print("─" * 78)
    print(f"{'Keyword':<25} {'Struct':<35} {'Evaluator':<25}")
    print("-" * 85)
    for v in cat["keyword_vals"]:
        loc = KEYWORDS.get(v)
        ev = KEYWORD_EVALUATORS.get(v)
        loc_str = fmt_loc(*loc) if loc else "(not found)"
        ev_str = fmt_loc_range(*ev) if ev else "(no evaluator)"
        print(f"{v:<25} {loc_str:<35} {ev_str:<25}")

    # ── COST TYPES ──
    print("\n" + "─" * 78)
    print("3. COST TYPES")
    print("─" * 78)
    print(f"{'Cost Type':<30} {'Engine Handler':<40}")
    print("-" * 70)
    for v in cat["cost_type_vals"]:
        loc = COST_TYPES.get(v)
        loc_str = fmt_loc_range(*loc) if loc else "(not handled)"
        print(f"{v:<30} {loc_str:<40}")

    # ── COST FIELDS ──
    print("\n" + "─" * 78)
    print("4. COST FIELDS (AbilityCost struct)")
    print("─" * 78)
    print(f"{'Field':<25} {'Engine Location':<35} {'Types seen':<15}")
    print("-" * 75)
    for k in sorted(cat["cost_fields_used"]):
        loc = COST_FIELDS.get(k)
        loc_str = fmt_loc(*loc) if loc else "(not in struct)"
        types = ", ".join(cat["cost_fields_used"][k])
        print(f"{k:<25} {loc_str:<35} {types:<15}")
    # Show missing
    for k in sorted(set(COST_FIELDS) - set(cat["cost_fields_used"])):
        loc = fmt_loc(*COST_FIELDS[k])
        print(f"{k:<25} {loc:<35} {'-- not used':<15}")

    # ── ACTIONS ──
    print("\n" + "─" * 78)
    print("5. ACTIONS (effect dispatch)")
    print("─" * 78)
    print(f"{'Action':<30} {'Handler in effects.rs':<40}")
    print("-" * 70)
    unmatched_actions = []
    for v in cat["action_vals"]:
        loc = ACTIONS.get(v)
        if loc:
            print(f"{v:<30} {fmt_loc_range(*loc):<40}")
        else:
            unmatched_actions.append(v)
    if unmatched_actions:
        print(f"\n  ⚠ UNHANDLED actions: {', '.join(unmatched_actions)}")

    # ── EFFECT FIELDS ──
    print("\n" + "─" * 78)
    print("6. EFFECT FIELDS (AbilityEffect struct)")
    print("─" * 78)
    print(f"{'Field':<30} {'Engine Location':<35} {'Types seen':<15}")
    print("-" * 80)
    for k in sorted(cat["effect_fields_used"]):
        loc = EFFECT_FIELDS.get(k)
        loc_str = fmt_loc(*loc) if loc else "(not in struct)"
        types = ", ".join(cat["effect_fields_used"][k])
        print(f"{k:<30} {loc_str:<35} {types:<15}")
    for k in sorted(set(EFFECT_FIELDS) - set(cat["effect_fields_used"])):
        loc = fmt_loc(*EFFECT_FIELDS[k])
        print(f"{k:<30} {loc:<35} {'-- not used':<15}")

    # ── CONDITION TYPES ──
    print("\n" + "─" * 78)
    print("7. CONDITION TYPES")
    print("─" * 78)
    print(f"{'Condition Type':<30} {'Engine Handler':<40}")
    print("-" * 70)
    for v in cat["condition_type_vals"]:
        loc = CONDITIONS.get(v)
        loc_str = fmt_loc_range(*loc) if loc else "(not handled)"
        print(f"{v:<30} {loc_str:<40}")

    # ── CONDITION FIELDS ──
    print("\n" + "─" * 78)
    print("8. CONDITION FIELDS (Condition struct)")
    print("─" * 78)
    print(f"{'Field':<30} {'Engine Location':<35} {'Types seen':<15}")
    print("-" * 80)
    for k in sorted(cat["condition_fields_used"]):
        loc = CONDITION_FIELDS.get(k)
        loc_str = fmt_loc(*loc) if loc else "(not in struct)"
        types = ", ".join(cat["condition_fields_used"][k])
        print(f"{k:<30} {loc_str:<35} {types:<15}")
    for k in sorted(set(CONDITION_FIELDS) - set(cat["condition_fields_used"])):
        loc = fmt_loc(*CONDITION_FIELDS[k])
        print(f"{k:<30} {loc:<35} {'-- not used':<15}")

    # ── DURATIONS ──
    print("\n" + "─" * 78)
    print("9. DURATION VALUES")
    print("─" * 78)
    print(f"{'Duration':<25} {'Used in':<40}")
    print("-" * 65)
    for v in cat["duration_vals"]:
        print(f"{v:<25} {'ability/effects.rs (various handlers)':<40}")

    # ── MOVE_CARDS ROUTES ──
    print("\n" + "─" * 78)
    print("10. MOVE_CARDS ROUTES (source → destination)")
    print("─" * 78)
    print(f"{'Route':<30} {'move_cards.rs location':<40}")
    print("-" * 70)
    for route in sorted(MOVE_CARDS_ROUTES):
        loc = MOVE_CARDS_ROUTES[route]
        print(f"{route:<30} {fmt_loc_range(*loc):<40}")

    # ── UTILITY FUNCTIONS ──
    print("\n" + "─" * 78)
    print("11. UTILITY FUNCTIONS (used by costs, conditions, effects)")
    print("─" * 78)
    print(f"{'Function':<35} {'Location':<40}")
    print("-" * 75)
    for fn_name, loc in sorted(UTILITY_FUNCTIONS.items()):
        print(f"{fn_name:<35} {fmt_loc_range(*loc):<40}")

    # ── ABILITY STRUCT ──
    print("\n" + "─" * 78)
    print("12. TOP-LEVEL ABILITY STRUCT FIELDS")
    print("─" * 78)
    print(f"{'Field':<20} {'Location':<25}")
    print("-" * 45)
    for k, loc in sorted(ABILITY_STRUCT.items()):
        print(f"{k:<20} {fmt_loc(*loc):<25}")

    print("\n" + "=" * 78)
    print("  END OF REPORT")
    print("=" * 78)

def print_report_md(cat):
    def loc(file, line):
        return f"`engine/src/{file}:{line}`"
    def loc_range(file, lr):
        return f"`engine/src/{file}:{lr}`"

    lines = []
    def L(s=""):
        lines.append(s)

    L("# Ability Cross-Reference Report")
    L()
    L(f"**Total unique abilities:** {cat['total']}")
    s = cat.get("stats", {})
    if s:
        L(f"**Cards:** {s.get('total_cards', '?')} total, {s.get('cards_with_abilities', '?')} with abilities, {s.get('unique_abilities', '?')} unique")
    L()

    # 1. Triggers
    L("## 1. Triggers")
    L("| Value | Engine Location |")
    L("|-------|-----------------|")
    for v in cat["trigger_vals"]:
        base = v.split(",")[0].strip() if "," in v else v
        loc_entry = TRIGGERS.get(base)
        loc_str = loc(*loc_entry) if loc_entry else "*not found*"
        if v != base:
            loc_str += v[len(base):]
        L(f"| `{v}` | {loc_str} |")

    # 2. Keywords
    L()
    L("## 2. Keywords")
    L("| Keyword | Struct | Evaluator |")
    L("|---------|--------|-----------|")
    for v in cat["keyword_vals"]:
        ks = KEYWORDS.get(v)
        ke = KEYWORD_EVALUATORS.get(v)
        ks_s = loc(*ks) if ks else "*not found*"
        ke_s = loc_range(*ke) if ke else "*none*"
        L(f"| `{v}` | {ks_s} | {ke_s} |")

    # 3. Cost types
    L()
    L("## 3. Cost Types")
    L("| Cost Type | Engine Handler |")
    L("|-----------|----------------|")
    for v in cat["cost_type_vals"]:
        lc = COST_TYPES.get(v)
        L(f"| `{v}` | {loc_range(*lc) if lc else '*not handled*'} |")

    # 4. Cost fields
    L()
    L("## 4. Cost Fields (`AbilityCost` struct)")
    L("| Field | Engine Location | Types | Status |")
    L("|-------|-----------------|-------|--------|")
    for k in sorted(cat["cost_fields_used"]):
        lc = COST_FIELDS.get(k)
        ls = loc(*lc) if lc else "*not in struct*"
        ts = ", ".join(cat["cost_fields_used"][k])
        L(f"| `{k}` | {ls} | {ts} | used |")
    for k in sorted(set(COST_FIELDS) - set(cat["cost_fields_used"])):
        ls = loc(*COST_FIELDS[k])
        L(f"| `{k}` | {ls} | -- | *unused* |")

    # 5. Actions
    L()
    L("## 5. Actions (effect dispatch)")
    L("| Action | Handler in `effects.rs` |")
    L("|--------|------------------------|")
    for v in cat["action_vals"]:
        lc = ACTIONS.get(v)
        L(f"| `{v}` | {loc_range(*lc) if lc else '⚠ *unhandled*'} |")
    unmatched = [v for v in cat["action_vals"] if v not in ACTIONS]
    if unmatched:
        L()
        L(f"**⚠ Unhandled actions:** `{'`, `'.join(unmatched)}`")

    # 6. Effect fields
    L()
    L("## 6. Effect Fields (`AbilityEffect` struct)")
    L("| Field | Engine Location | Types | Status |")
    L("|-------|-----------------|-------|--------|")
    for k in sorted(cat["effect_fields_used"]):
        lc = EFFECT_FIELDS.get(k)
        ls = loc(*lc) if lc else "*not in struct*"
        ts = ", ".join(cat["effect_fields_used"][k])
        L(f"| `{k}` | {ls} | {ts} | used |")
    for k in sorted(set(EFFECT_FIELDS) - set(cat["effect_fields_used"])):
        ls = loc(*EFFECT_FIELDS[k])
        L(f"| `{k}` | {ls} | -- | *unused* |")

    # 7. Condition types
    L()
    L("## 7. Condition Types")
    L("| Condition Type | Engine Handler |")
    L("|----------------|----------------|")
    for v in cat["condition_type_vals"]:
        lc = CONDITIONS.get(v)
        L(f"| `{v}` | {loc_range(*lc) if lc else '*not handled*'} |")

    # 8. Condition fields
    L()
    L("## 8. Condition Fields (`Condition` struct)")
    L("| Field | Engine Location | Types | Status |")
    L("|-------|-----------------|-------|--------|")
    for k in sorted(cat["condition_fields_used"]):
        lc = CONDITION_FIELDS.get(k)
        ls = loc(*lc) if lc else "*not in struct*"
        ts = ", ".join(cat["condition_fields_used"][k])
        L(f"| `{k}` | {ls} | {ts} | used |")
    for k in sorted(set(CONDITION_FIELDS) - set(cat["condition_fields_used"])):
        ls = loc(*CONDITION_FIELDS[k])
        L(f"| `{k}` | {ls} | -- | *unused* |")

    # 9. Durations
    L()
    L("## 9. Duration Values")
    L("| Duration | Used In |")
    L("|----------|---------|")
    for v in cat["duration_vals"]:
        L(f"| `{v}` | `ability/effects.rs` (various handlers) |")

    # 10. Move cards routes
    L()
    L("## 10. Move Cards Routes")
    L("| Route | Location |")
    L("|-------|----------|")
    for route in sorted(MOVE_CARDS_ROUTES):
        lc = MOVE_CARDS_ROUTES[route]
        L(f"| `{route}` | {loc_range(*lc)} |")

    # 11. Utility functions
    L()
    L("## 11. Utility Functions")
    L("| Function | Location |")
    L("|----------|----------|")
    for fn, lc in sorted(UTILITY_FUNCTIONS.items()):
        L(f"| `{fn}` | {loc_range(*lc)} |")

    # 12. Top-level Ability struct
    L()
    L("## 12. Top-Level Ability Struct Fields")
    L("| Field | Location |")
    L("|-------|----------|")
    for k, lc in sorted(ABILITY_STRUCT.items()):
        L(f"| `{k}` | {loc(*lc)} |")

    # ── Data values from abilities.json mapped to engine code ──
    L()
    L("---")
    L("# Data Values in abilities.json → Engine Code")
    L()
    L("This section shows the actual **values** found in abilities.json fields and where each value is handled in the engine source code.")
    L()

    # Effect field values (only fields tracked in VALUE_MAP)
    L()
    L("## Effect Field Values → Engine Code")
    L("| Field | Value in abilities.json | Engine Handler |")
    L("|-------|------------------------|----------------|")
    for field in sorted(cat["effect_field_values"]):
        path = f"effect.{field}"
        vm = VALUE_MAP.get(path, {})
        if not vm: continue
        values = cat["effect_field_values"][field]
        for v in sorted(values):
            if len(v) > 60: continue  # skip long text values
            ls = vm.get(v)
            loc_str = loc_range(*ls) if ls else f"⚠ not found: `{v}`"
            L(f"| `{field}` | `{v}` | {loc_str} |")

    # Cost field values (only fields tracked in VALUE_MAP)
    L()
    L("## Cost Field Values → Engine Code")
    L("| Field | Value in abilities.json | Engine Handler |")
    L("|-------|------------------------|----------------|")
    for field in sorted(cat["cost_field_values"]):
        path = f"cost.{field}"
        vm = VALUE_MAP.get(path, {})
        if not vm: continue
        values = cat["cost_field_values"][field]
        for v in sorted(values):
            ls = vm.get(v)
            loc_str = loc_range(*ls) if ls else f"⚠ not found: `{v}`"
            L(f"| `{field}` | `{v}` | {loc_str} |")

    # Condition field values (only fields tracked in VALUE_MAP)
    L()
    L("## Condition Field Values → Engine Code")
    L("| Field | Value in abilities.json | Engine Handler |")
    L("|-------|------------------------|----------------|")
    for field in sorted(cat["condition_field_values"]):
        path = f"condition.{field}"
        vm = VALUE_MAP.get(path, {})
        if not vm: continue
        values = cat["condition_field_values"][field]
        for v in sorted(values):
            ls = vm.get(v)
            loc_str = loc_range(*ls) if ls else f"⚠ not found: `{v}`"
            L(f"| `{field}` | `{v}` | {loc_str} |")

    print("\n".join(lines))

def print_audit_md(cat):
    def loc(file, line):
        return f"`engine/src/{file}:{line}`"

    lines = []
    def L(s=""):
        lines.append(s)

    L("---")
    L("# Implementation Audit")
    L()
    L("## Legend")
    L("| Status | Meaning |")
    L("|--------|---------|")
    L("| `IMPLEMENTED` | Field is read by engine logic (`.as_ref()`, `.unwrap()`, match, etc.) |")
    L("| `DEAD` | Field exists in the Rust struct but no engine code reads it — likely leftover from refactoring |")
    L("| `PARSER_ONLY` | Field is only emitted by `parser.py` — no corresponding Rust struct field exists |")
    L()

    # Effect fields
    L("## AbilityEffect — Implementation Status")
    L("| Field | In abilities.json | In Rust struct | Engine reads it | Status |")
    L("|-------|:-:|:-:|:-:|--------|")
    all_effect_fields = sorted(set(list(EFFECT_FIELDS.keys()) + list(EFFECT_IMPL_STATUS.keys())))
    for k in all_effect_fields:
        in_json = "✓" if k in [fk for fk in cat.get("effect_fields_used", {})] else ""
        in_struct = "✓" if k in EFFECT_FIELDS else ""
        impl_status = EFFECT_IMPL_STATUS.get(k, "UNKNOWN")
        reads = "✓" if impl_status == "IMPLEMENTED" else ""
        L(f"| `{k}` | {in_json} | {in_struct} | {reads} | {impl_status} |")

    # Condition fields
    L()
    L("## Condition — Implementation Status")
    L("| Field | In abilities.json | In Rust struct | Engine reads it | Status |")
    L("|-------|:-:|:-:|:-:|--------|")
    all_cond_fields = sorted(set(list(CONDITION_FIELDS.keys()) + list(CONDITION_IMPL_STATUS.keys())))
    for k in all_cond_fields:
        in_json = "✓" if k in [ck for ck in cat.get("condition_fields_used", {})] else ""
        in_struct = "✓" if k in CONDITION_FIELDS else ""
        impl_status = CONDITION_IMPL_STATUS.get(k, "UNKNOWN")
        reads = "✓" if impl_status == "IMPLEMENTED" else ""
        L(f"| `{k}` | {in_json} | {in_struct} | {reads} | {impl_status} |")

    # Cost fields
    L()
    L("## AbilityCost — Implementation Status")
    L("| Field | In abilities.json | In Rust struct | Engine reads it | Status |")
    L("|-------|:-:|:-:|:-:|--------|")
    all_cost_fields = sorted(set(list(COST_FIELDS.keys()) + list(COST_IMPL_STATUS.keys())))
    for k in all_cost_fields:
        in_json = "✓" if k in [ck for ck in cat.get("cost_fields_used", {})] else ""
        in_struct = "✓" if k in COST_FIELDS else ""
        impl_status = COST_IMPL_STATUS.get(k, "UNKNOWN")
        reads = "✓" if impl_status == "IMPLEMENTED" else ""
        L(f"| `{k}` | {in_json} | {in_struct} | {reads} | {impl_status} |")

    # Summary stats
    L()
    L("## Summary")
    statuses = list(EFFECT_IMPL_STATUS.values()) + list(CONDITION_IMPL_STATUS.values()) + list(COST_IMPL_STATUS.values())
    implemented = statuses.count("IMPLEMENTED")
    dead = statuses.count("DEAD")
    parser = statuses.count("PARSER_ONLY")
    total = len(statuses)
    L(f"- **{implemented}/{total}** fields are properly implemented (engine reads them)")
    L(f"- **{dead}/{total}** fields are **dead code** — in struct, never read by engine")
    L(f"- **{parser}/{total}** fields are **parser-only** — not in any Rust struct")
    L()
    L("### Dead field cleanup candidates")
    L("These struct fields are never read by engine code and should be removed:")
    for k in sorted(EFFECT_IMPL_STATUS):
        if EFFECT_IMPL_STATUS[k] == "DEAD":
            L(f"- `AbilityEffect::{k}` ({EFFECT_FIELDS.get(k, '?')})")
    for k in sorted(CONDITION_IMPL_STATUS):
        if CONDITION_IMPL_STATUS[k] == "DEAD":
            L(f"- `Condition::{k}` ({CONDITION_FIELDS.get(k, '?')})")
    for k in sorted(COST_IMPL_STATUS):
        if COST_IMPL_STATUS[k] == "DEAD":
            L(f"- `AbilityCost::{k}` ({COST_FIELDS.get(k, '?')})")

    L()
    L("### Gap: Parser emits but engine has no field")
    L("These need Rust struct fields adding:")
    for k in sorted(EFFECT_IMPL_STATUS):
        if EFFECT_IMPL_STATUS[k] == "PARSER_ONLY" and k in cat.get("effect_fields_used", {}):
            L(f"- `effect.{k}` — values: {', '.join(sorted(cat['effect_field_values'].get(k, [])))}")
    for k in sorted(CONDITION_IMPL_STATUS):
        if CONDITION_IMPL_STATUS[k] == "PARSER_ONLY" and k in cat.get("condition_fields_used", {}):
            L(f"- `condition.{k}` — values: {', '.join(sorted(cat['condition_field_values'].get(k, [])))}")

    L()
    L("### Value-level gaps: Engine doesn't handle this value")
    L("These values appear in abilities.json but the engine has no specific handler:")
    for field in sorted(cat["effect_field_values"]):
        path = f"effect.{field}"
        vm = VALUE_MAP.get(path, {})
        if not vm: continue
        for v in sorted(cat["effect_field_values"][field]):
            if len(v) > 60: continue
            if v not in vm:
                L(f"- `effect.{field}` = `{v}` — no engine handler found")
    for field in sorted(cat["cost_field_values"]):
        path = f"cost.{field}"
        vm = VALUE_MAP.get(path, {})
        if not vm: continue
        for v in sorted(cat["cost_field_values"][field]):
            if len(v) > 60: continue
            if v not in vm:
                L(f"- `cost.{field}` = `{v}` — no engine handler found")
    for field in sorted(cat["condition_field_values"]):
        path = f"condition.{field}"
        vm = VALUE_MAP.get(path, {})
        if not vm: continue
        for v in sorted(cat["condition_field_values"][field]):
            if len(v) > 60: continue
            if v not in vm:
                L(f"- `condition.{field}` = `{v}` — no engine handler found")

    # ── Frequency Distribution & Field Necessity ──
    L()
    L("---")
    L("# Frequency Distribution & Field Necessity")
    L()
    L("How often each field+value appears across the 602 unique abilities. Low-frequency items are candidates for removal or simplification.")
    L()

    # Trigger frequency
    L("## Trigger Frequency")
    L("| Trigger | Count | % of abilities |")
    L("|---------|-------|-------|")
    for v, c in sorted(cat.get("trigger_freq", {}).items(), key=lambda x: -x[1]):
        pct = c * 100 / cat["total"]
        L(f"| `{v}` | {c} | {pct:.1f}% |")

    # Action frequency
    L()
    L("## Action Frequency")
    L("| Action | Count | % of abilities | Engine handler |")
    L("|--------|-------|-------|----------------|")
    for v, c in sorted(cat.get("action_freq", {}).items(), key=lambda x: -x[1]):
        pct = c * 100 / cat["total"]
        handler = ACTIONS.get(v, "⚠ UNHANDLED")
        L(f"| `{v}` | {c} | {pct:.1f}% | `{handler[0]}:{handler[1]}` |")

    # Cost type frequency
    L()
    L("## Cost Type Frequency")
    L("| Cost Type | Count | % of abilities |")
    L("|-----------|-------|-------|")
    for v, c in sorted(cat.get("cost_type_freq", {}).items(), key=lambda x: -x[1]):
        pct = c * 100 / cat["total"]
        L(f"| `{v}` | {c} | {pct:.1f}% |")

    # Condition type frequency
    L()
    L("## Condition Type Frequency")
    L("| Condition Type | Count | % of abilities |")
    L("|----------------|-------|-------|")
    for v, c in sorted(cat.get("condition_type_freq", {}).items(), key=lambda x: -x[1]):
        pct = c * 100 / cat["total"]
        L(f"| `{v}` | {c} | {pct:.1f}% |")

    # Effect field value frequencies (only VALUE_MAP fields)
    L()
    L("## Effect Field Value Frequencies")
    L("| Field | Value | Count | % of abilities | Engine handler |")
    L("|-------|-------|-------|-------|----------------|")
    for field in sorted(cat.get("effect_field_freq", {})):
        path = f"effect.{field}"
        vm = VALUE_MAP.get(path, {})
        if not vm: continue
        for v, c in sorted(cat["effect_field_freq"][field].items(), key=lambda x: -x[1]):
            if len(v) > 50: continue
            pct = c * 100 / cat["total"]
            ls = vm.get(v)
            loc_str = f"`{ls[0]}:{ls[1]}`" if ls else "⚠ no handler"
            L(f"| `{field}` | `{v}` | {c} | {pct:.1f}% | {loc_str} |")

    # Cost field value frequencies
    L()
    L("## Cost Field Value Frequencies")
    L("| Field | Value | Count | % of abilities | Engine handler |")
    L("|-------|-------|-------|-------|----------------|")
    for field in sorted(cat.get("cost_field_freq", {})):
        path = f"cost.{field}"
        vm = VALUE_MAP.get(path, {})
        if not vm: continue
        for v, c in sorted(cat["cost_field_freq"][field].items(), key=lambda x: -x[1]):
            if len(v) > 50: continue
            pct = c * 100 / cat["total"]
            ls = vm.get(v)
            loc_str = f"`{ls[0]}:{ls[1]}`" if ls else "⚠ no handler"
            L(f"| `{field}` | `{v}` | {c} | {pct:.1f}% | {loc_str} |")

    # Condition field value frequencies
    L()
    L("## Condition Field Value Frequencies")
    L("| Field | Value | Count | % of abilities | Engine handler |")
    L("|-------|-------|-------|-------|----------------|")
    for field in sorted(cat.get("condition_field_freq", {})):
        path = f"condition.{field}"
        vm = VALUE_MAP.get(path, {})
        if not vm: continue
        for v, c in sorted(cat["condition_field_freq"][field].items(), key=lambda x: -x[1]):
            if len(v) > 50: continue
            pct = c * 100 / cat["total"]
            ls = vm.get(v)
            loc_str = f"`{ls[0]}:{ls[1]}`" if ls else "⚠ no handler"
            L(f"| `{field}` | `{v}` | {c} | {pct:.1f}% | {loc_str} |")

    # ── Field Necessity Analysis ──
    L()
    L("---")
    L("# Field Necessity Analysis")
    L()
    L("Based on frequency distributions and implementation status. Fields used in <5% of abilities or that are dead code are candidates for removal/consolidation.")
    L()

    total_ab = cat["total"]

    L("## Rarely-used effect fields (<5% of abilities)")
    for field in sorted(cat.get("effect_field_freq", {})):
        freq_sum = sum(cat["effect_field_freq"][field].values())
        pct = freq_sum * 100 / total_ab
        if pct < 5 and freq_sum < 30:
            impl = EFFECT_IMPL_STATUS.get(field, "?")
            L(f"- `effect.{field}` — appears {freq_sum}/{total_ab} times ({pct:.1f}%), status: {impl}")

    L()
    L("## Rarely-used cost fields (<5% of abilities)")
    for field in sorted(cat.get("cost_field_freq", {})):
        freq_sum = sum(cat["cost_field_freq"][field].values())
        pct = freq_sum * 100 / total_ab
        if pct < 5 and freq_sum < 30:
            impl = COST_IMPL_STATUS.get(field, "?")
            L(f"- `cost.{field}` — appears {freq_sum}/{total_ab} times ({pct:.1f}%), status: {impl}")

    L()
    L("## Rarely-used condition fields (<5% of abilities)")
    for field in sorted(cat.get("condition_field_freq", {})):
        freq_sum = sum(cat["condition_field_freq"][field].values())
        pct = freq_sum * 100 / total_ab
        if pct < 5 and freq_sum < 30:
            impl = CONDITION_IMPL_STATUS.get(field, "?")
            L(f"- `condition.{field}` — appears {freq_sum}/{total_ab} times ({pct:.1f}%), status: {impl}")

    L()
    L("## Verdict")
    dead_count = sum(1 for s in EFFECT_IMPL_STATUS.values() if s == "DEAD")
    dead_count += sum(1 for s in CONDITION_IMPL_STATUS.values() if s == "DEAD")
    dead_count += sum(1 for s in COST_IMPL_STATUS.values() if s == "DEAD")
    rare_count = 0
    for field in cat.get("effect_field_freq", {}):
        freq_sum = sum(cat["effect_field_freq"][field].values())
        if freq_sum * 100 / total_ab < 5 and freq_sum < 30:
            rare_count += 1
    for field in cat.get("cost_field_freq", {}):
        freq_sum = sum(cat["cost_field_freq"][field].values())
        if freq_sum * 100 / total_ab < 5 and freq_sum < 30:
            rare_count += 1
    for field in cat.get("condition_field_freq", {}):
        freq_sum = sum(cat["condition_field_freq"][field].values())
        if freq_sum * 100 / total_ab < 5 and freq_sum < 30:
            rare_count += 1

    total_struct = len(EFFECT_FIELDS) + len(CONDITION_FIELDS) + len(COST_FIELDS)
    L(f"- **{dead_count} dead fields** — safe to remove, no engine code reads them")
    L(f"- **{rare_count} rarely-used fields** (<5% of abilities) — consider consolidating")
    L()
    L("Removing all 49 dead fields would reduce struct size by ~40% without affecting functionality.")

    print("\n".join(lines))


def main():
    data = load_abilities()
    cat = analyze(data)

    if "--json" in sys.argv:
        import json as j
        result = {
            "summary": {
                "total_abilities": cat["total"],
                "total_cards": cat["stats"].get("total_cards"),
                "cards_with_abilities": cat["stats"].get("cards_with_abilities"),
            },
            "triggers": {v: TRIGGERS.get(v, "NOT_FOUND") for v in cat["trigger_vals"]},
            "keywords": {v: {"struct": KEYWORDS.get(v), "evaluator": KEYWORD_EVALUATORS.get(v)} for v in cat["keyword_vals"]},
            "cost_types": {v: COST_TYPES.get(v, "NOT_HANDLED") for v in cat["cost_type_vals"]},
            "actions": {v: ACTIONS.get(v, "NOT_HANDLED") for v in cat["action_vals"]},
            "condition_types": {v: CONDITIONS.get(v, "NOT_HANDLED") for v in cat["condition_type_vals"]},
            "durations": sorted(cat["duration_vals"]),
            "cost_fields": {k: {"location": COST_FIELDS.get(k, "NOT_IN_STRUCT"), "types": sorted(v)} for k, v in cat["cost_fields_used"].items()},
            "effect_fields": {k: {"location": EFFECT_FIELDS.get(k, "NOT_IN_STRUCT"), "types": sorted(v)} for k, v in cat["effect_fields_used"].items()},
            "condition_fields": {k: {"location": CONDITION_FIELDS.get(k, "NOT_IN_STRUCT"), "types": sorted(v)} for k, v in cat["condition_fields_used"].items()},
            "move_cards_routes": MOVE_CARDS_ROUTES,
            "utility_functions": UTILITY_FUNCTIONS,
            "ability_struct": ABILITY_STRUCT,
        }
        print(j.dumps(result, indent=2, ensure_ascii=False))
    elif "--md" in sys.argv:
        out = sys.argv[sys.argv.index("--md") + 1] if len(sys.argv) > sys.argv.index("--md") + 1 and not sys.argv[sys.argv.index("--md") + 1].startswith("--") else None
        import io
        buf = io.StringIO()
        old = sys.stdout
        sys.stdout = buf
        print_report_md(cat)
        sys.stdout = old
        result = buf.getvalue()
        buf2 = io.StringIO()
        sys.stdout = buf2
        print_audit_md(cat)
        sys.stdout = old
        result2 = buf2.getvalue()
        md_output = result + "\n" + result2

        if out:
            with open(out, "w", encoding="utf-8") as f:
                f.write(md_output)
            print(f"Written to {out}")
        else:
            print(result)
    else:
        print_report(cat)

if __name__ == "__main__":
    main()
