"""Auto-compiler: scans abilities.json, discovers field types, generates bytecode + Rust decoder."""

import json, struct, re, hashlib
from pathlib import Path

# ── Value vocabulary tables for type inference ──
ZONES = {
    "hand",
    "stage",
    "center",
    "left",
    "right",
    "discard",
    "waitroom",
    "energy",
    "energy_zone",
    "deck",
    "deck_top",
    "deck_bottom",
    "success_zone",
    "live_card_zone",
    "success_live_zone",
    "energy_deck",
    "empty_area",
    "same_area",
    "under_member",
    "looked_at",
    "revealed_cards",
    "selected_cards",
    "resolution",
    "exclusion_zone",
    "deck_or_discard",
}
PLAYERS = {"self", "opponent", "both", "owner"}
CARD_TYPES = {
    "card",
    "any",
    "member_card",
    "member",
    "live_card",
    "live",
    "energy_card",
    "energy",
    "event_card",
    "event",
    "character_card",
    "character",
    "baton_touch_card",
    "baton_touch",
    "climax_card",
    "climax",
}
RESOURCES = {
    "heart",
    "blade",
    "yell",
    "shield",
    "energy",
    "hearts",
    "blades",
    "heart_icon",
    "blade_icon",
}
HEARTS = {
    "heart01",
    "smile",
    "pink",
    "heart02",
    "pure",
    "green",
    "heart03",
    "cool",
    "blue",
    "heart04",
    "active",
    "orange",
    "heart05",
    "natural",
    "purple",
    "heart06",
    "elegant",
    "red",
}
STATES = {
    "rest",
    "rested",
    "stand",
    "stood",
    "stand_up",
    "reverse",
    "reversed",
    "wait",
    "waited",
}
DURATIONS = {
    "turn_end",
    "this_turn",
    "live_end",
    "until_end_of_live",
    "permanent",
    "always",
    "until_used",
    "next_turn",
}
OPERATORS = {"=", "==", "!=", ">", ">=", "<", "<="}

VOCAB = {
    "zone": ZONES,
    "player": PLAYERS,
    "card_type": CARD_TYPES,
    "resource": RESOURCES,
    "heart": HEARTS,
    "state": STATES,
    "duration": DURATIONS,
    "operator": OPERATORS,
}

# ── Encoding tables ──
ZONE_ENCODE = {
    v: i
    for i, v in enumerate(
        [
            "hand",
            "stage",
            "center",
            "left",
            "right",
            "discard",
            "waitroom",
            "energy",
            "energy_zone",
            "deck",
            "deck_top",
            "deck_bottom",
            "success_zone",
            "live_card_zone",
            "success_live_zone",
            "energy_deck",
            "empty_area",
            "same_area",
            "under_member",
            "looked_at",
            "revealed_cards",
            "selected_cards",
            "resolution",
            "exclusion_zone",
            "deck_or_discard",
        ]
    )
}
PLAYER_ENCODE = {"self": 0, "opponent": 1, "both": 2, "owner": 3}
CARD_TYPE_ENCODE = {
    v: i
    for i, v in enumerate(
        [
            "card",
            "member_card",
            "live_card",
            "energy_card",
            "event_card",
            "character_card",
            "baton_touch_card",
            "climax_card",
        ]
    )
}
RESOURCE_ENCODE = {"heart": 0, "blade": 1, "yell": 2, "shield": 3, "energy": 4}
HEART_ENCODE = {
    v: i
    for i, v in enumerate(["smile", "pure", "cool", "active", "natural", "elegant"])
}
STATE_ENCODE = {"rest": 0, "stand": 1, "reverse": 2, "wait": 3}
DURATION_ENCODE = {
    "this_turn": 0,
    "turn_end": 0,
    "until_end_of_live": 1,
    "live_end": 1,
    "permanent": 2,
    "always": 2,
    "until_used": 3,
    "next_turn": 4,
}
OPERATOR_ENCODE = {"=": 0, "==": 0, "!=": 1, ">": 2, ">=": 3, "<": 4, "<=": 5}


def norm(v):
    if v is None:
        return ""
    return str(v).strip().lower().replace(" ", "_").replace("-", "_")


def encode(val, ftype, strs):
    if ftype == "str_list":
        items = []
        if isinstance(val, list):
            items = [str(x).strip() for x in val if x is not None and str(x).strip()]
        elif val is not None and str(val).strip():
            items = [str(val).strip()]
        out = bytearray([len(items) & 0xFF])
        for it in items:
            out.extend(struct.pack("<H", strs.idx(it)))
        return bytes(out)
    if val is None:
        if ftype == "str_idx":
            return struct.pack("<H", 0xFFFF)
        if ftype == "u16":
            return struct.pack("<H", 0xFFFF)
        # All scalar/enum optypes use 0xFF as the "absent" sentinel so the
        # decoder can distinguish an unset field (→ None) from a real value.
        return b"\xff"
    if isinstance(val, list):
        if not val:
            return struct.pack("<H", 0xFFFF) if ftype == "str_idx" else b"\xff"
        val = val[0]
    s = norm(val)
    if ftype == "u8":
        if isinstance(val, str):
            ct = CARD_TYPE_ENCODE.get(s)
            if ct is not None:
                return bytes([ct])
        return bytes([int(val) & 0xFF])
    if ftype == "i8":
        return struct.pack("<b", int(val))
    if ftype == "u16":
        return struct.pack("<H", int(val))
    if ftype == "bool":
        return bytes([1 if val else 0])
    if ftype == "zone":
        return bytes([ZONE_ENCODE.get(s, 0)])
    if ftype == "player":
        return bytes([PLAYER_ENCODE.get(s, 0)])
    if ftype == "card_type":
        return bytes([CARD_TYPE_ENCODE.get(s, 0)])
    if ftype == "resource":
        return bytes([RESOURCE_ENCODE.get(s, 0)])
    if ftype == "heart":
        return bytes([HEART_ENCODE.get(s, 0)])
    if ftype == "state":
        return bytes([STATE_ENCODE.get(s, 0)])
    if ftype == "duration":
        return bytes([DURATION_ENCODE.get(s, 0)])
    if ftype == "operator":
        return bytes([OPERATOR_ENCODE.get(s, 3)])
    if ftype == "str_idx":
        return struct.pack("<H", strs.idx(val))
    return b"\x00"


class BC:
    def __init__(self):
        self.data = bytearray()

    def u8(self, v):
        self.data.append(v & 0xFF)

    def u16(self, v):
        self.data.extend(struct.pack("<H", v))

    def __len__(self):
        return len(self.data)

    def __bytes__(self):
        return bytes(self.data)


class StringTable:
    def __init__(self):
        self._strings, self._index = [], {}

    def idx(self, s):
        if not s:
            return 0xFFFF
        if isinstance(s, list):
            s = s[0] if s else ""
        s = str(s).strip()
        if not s:
            return 0xFFFF
        if s not in self._index:
            self._index[s] = len(self._strings)
            self._strings.append(s)
        return self._index[s]

    def __iter__(self):
        return iter(self._strings)

    def __len__(self):
        return len(self._strings)


# ── Condition compiler ──
COND_OPCODES = {
    "card_count_condition": 0x40,
    "location_condition": 0x41,
    "comparison_condition": 0x42,
    "group_condition": 0x43,
    "movement_condition": 0x44,
    "temporal_condition": 0x45,
    "appearance_condition": 0x46,
    "state_condition": 0x47,
    "energy_state_condition": 0x48,
    "position_condition": 0x49,
    "or_condition": 0x4A,
    "and_condition": 0x4B,
    "end_condition": 0x4C,
    "highest_cost_on_stage_condition": 0x4D,
    "state_change_condition": 0x4E,
    "card_blade_condition": 0x4F,
    "all_cost_comparison_condition": 0x50,
    "ability_filter_condition": 0x51,
    "has_moved": 0x52,
    "not_moved": 0x53,
    "opponent_live_success": 0x54,
    "no_excess_heart": 0x55,
}
COND_FIELDS = {
    "card_count_condition": [
        ("zone", "location"),
        ("operator", "operator"),
        ("u8", "count"),
        ("u8", "card_type"),
        ("str_idx", "group_names"),
        ("player", "target"),
    ],
    "location_condition": [
        ("zone", "location"),
        ("u8", "card_type"),
        ("bool", "exclude_self"),
        ("player", "target"),
    ],
    "comparison_condition": [
        ("zone", "location"),
        ("str_idx", "comparison_type"),
        ("str_idx", "aggregate"),
        ("operator", "operator"),
        ("u16", "count"),
        ("player", "target"),
        ("resource", "resource_type"),
        ("u8", "card_type"),
        ("str_idx", "group_names"),
    ],
    "group_condition": [
        ("str_idx", "group_names"),
        ("u8", "count"),
        ("operator", "operator"),
    ],
    "movement_condition": [
        ("str_idx", "movement"),
        ("zone", "location"),
        ("player", "target"),
        ("u16", "cost_limit"),
        ("operator", "cost_limit_operator"),
        ("bool", "baton_touch_trigger"),
        ("u8", "min_baton_touch_count"),
        ("bool", "exclude_self"),
        ("str_list", "group_names"),
        ("card_type", "card_type"),
        ("str_list", "characters"),
        ("zone", "baton_touch_source"),
        ("str_idx", "comparison_type"),
        ("operator", "operator"),
        ("bool", "self_effect_only"),
        ("bool", "energy_placed"),
        ("zone", "area_direction"),
        ("bool", "self_target"),
        ("zone", "source"),
        ("zone", "destination"),
        ("state", "from_state"),
        ("state", "to_state"),
    ],
    "temporal_condition": [("u8", "count"), ("operator", "operator")],
    "appearance_condition": [("zone", "location"), ("u8", "count")],
    "state_condition": [
        ("state", "state"),
        ("operator", "operator"),
        ("bool", "value"),
    ],
    "energy_state_condition": [("operator", "operator"), ("u8", "count")],
    "position_condition": [("zone", "location")],
    "highest_cost_on_stage_condition": [],
    "state_change_condition": [("state", "state_change")],
    "card_blade_condition": [("operator", "operator"), ("u8", "count")],
    "card_count_condition": [
        ("zone", "location"),
        ("operator", "operator"),
        ("u8", "count"),
        ("u8", "card_type"),
        ("str_idx", "group_names"),
        ("player", "target"),
    ],
    "all_cost_comparison_condition": [("operator", "operator"), ("u16", "count")],
    "ability_filter_condition": [("str_idx", "text")],
    "has_moved": [("zone", "position"), ("str_idx", "group_names")],
    "not_moved": [],
    "opponent_live_success": [("bool", "no_excess_heart")],
    "no_excess_heart": [],
}
COND_COMPARISON_TYPE = {"cost": 0, "power": 1, "count": 2, "level": 3, "hand_count": 4}
COND_AGGREGATE = {"total": 0, "average": 1, "max": 2, "min": 3}


def compile_condition(cond, bc, strs):
    if not isinstance(cond, dict):
        return
    t = cond.get("type", "")
    if t in ("compound", "or_condition"):
        opc = (
            COND_OPCODES["or_condition"]
            if cond.get("operator", "and") == "or" or t == "or_condition"
            else COND_OPCODES["and_condition"]
        )
        bc.u8(opc)
        for sc in cond.get("conditions", []):
            compile_condition(sc, bc, strs)
        bc.u8(COND_OPCODES["end_condition"])
        return
    fields = COND_FIELDS.get(t)
    if fields is None:
        return
    bc.u8(COND_OPCODES[t])
    for ftype, fname in fields:
        val = cond.get(fname)
        bc.data.extend(encode(val, ftype, strs))


# ── Cost compiler ──
COST_OPCODES = {
    "move_cards_cost": 0x80,
    "tap": 0x81,
    "rest": 0x82,
    "energy_cost": 0x83,
    "discard_cost": 0x84,
    "place_energy_under_member_cost": 0x85,
    "pay_energy_cost": 0x86,
    "change_state_cost": 0x87,
    "sequential_cost": 0x88,
    "reveal_cost": 0x89,
    "choice_condition": 0x8A,
}


def compile_cost(cost, bc, strs):
    if isinstance(cost, list):
        for c in cost:
            compile_cost(c, bc, strs)
        return
    if not isinstance(cost, dict):
        return
    t = cost.get("type", "")
    if t == "move_cards":
        bc.u8(0x80)
        bc.data.extend(encode(cost.get("source", "stage"), "zone", strs))
        bc.data.extend(encode(cost.get("destination", "discard"), "zone", strs))
        bc.data.extend(encode(cost.get("card_type", "member_card"), "card_type", strs))
        bc.u8(1 if cost.get("self_cost") else 0)
        bc.data.extend(encode(cost.get("count"), "u8", strs))
        bc.u8(1 if cost.get("optional") else 0)
        bc.u8(1 if cost.get("any_number") else 0)
        bc.data.extend(encode(cost.get("group_names"), "str_list", strs))
        bc.data.extend(encode(cost.get("characters"), "str_list", strs))
    elif t == "tap":
        bc.u8(0x81)
    elif t == "rest":
        bc.u8(0x82)
        bc.data.extend(encode(cost.get("count", 1), "u8", strs))
    elif t == "energy":
        bc.u8(0x83)
        bc.data.extend(encode(cost.get("energy", cost.get("count", 1)), "u8", strs))
        bc.u8(0)
    elif t == "discard":
        bc.u8(0x84)
        bc.data.extend(encode(cost.get("count"), "u8", strs))
        bc.data.extend(encode(cost.get("card_type", "card"), "card_type", strs))
        bc.u8(1 if cost.get("optional") else 0)
        bc.u8(1 if cost.get("any_number") else 0)
        bc.data.extend(encode(cost.get("group_names"), "str_list", strs))
        bc.data.extend(encode(cost.get("characters"), "str_list", strs))
    elif t == "place_energy_under_member":
        bc.u8(0x85)
        bc.data.extend(encode(cost.get("count", 1), "u8", strs))
    elif t == "pay_energy":
        bc.u8(0x86)
        bc.data.extend(encode(cost.get("energy", cost.get("count", 1)), "u8", strs))
        bc.u8(1 if cost.get("optional") else 0)
    elif t == "change_state":
        bc.u8(0x87)
        bc.data.extend(encode(cost.get("state_change", "rest"), "state", strs))
        bc.u8(1 if cost.get("optional") else 0)
        bc.u8(1 if cost.get("self_cost") else 0)
    elif t == "sequential_cost":
        costs = cost.get("costs", [])
        bc.u8(0x88)
        bc.u8(len(costs))
        for sc in costs:
            compile_cost(sc, bc, strs)
    elif t == "reveal":
        bc.u8(0x89)
        bc.data.extend(encode(cost.get("source", "hand"), "zone", strs))
        bc.data.extend(encode(cost.get("card_type", "card"), "card_type", strs))
        bc.data.extend(encode(cost.get("count", 1), "u8", strs))
    elif t == "choice_condition":
        opts = cost.get("options", [])
        bc.u8(0x8A)
        bc.u8(len(opts))
        for opt in opts:
            compile_cost(opt, bc, strs)


# ── Opaque string prefix for condition comparison_type/aggregate ──
def first_str(v):
    if v is None:
        return None
    if isinstance(v, list):
        return v[0] if v else None
    return v


# ── Main compiler ──
EFFECT_OPCODES = {
    s: i + 1
    for i, s in enumerate(
        [
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
        ]
    )
}
# Compound opcodes  Emust be at fixed values 0x60+ for the hand-coded vm.rs handlers
EFFECT_OPCODES["compound_sequential"] = 0x60
EFFECT_OPCODES["compound_conditional"] = 0x61
EFFECT_OPCODES["compound_conditional_alt"] = 0x62
EFFECT_OPCODES["compound_choice"] = 0x65
EFFECT_OPCODES["compound_look_at"] = 0x70
EFFECT_OPCODES["compound_select_cards"] = 0x71


def compile_one(eff, bc, strs, field_map, is_sub=False):
    if not isinstance(eff, dict):
        return
    a = eff.get("action", "")

    # Conditional wrapper
    cond = eff.get("condition")
    if (
        isinstance(cond, dict)
        and not is_sub
        and a
        not in (
            "conditional_alternative",
            "conditional_on_optional",
            "conditional_on_result",
        )
    ):
        bc.u8(0x61)
        cb = BC()
        compile_condition(cond, cb, strs)
        bc.u16(len(cb))
        bc.data.extend(cb.data)
        body = BC()
        compile_one(eff, body, strs, field_map, True)
        bc.u16(len(body))
        bc.data.extend(body.data)
        alt = eff.get("alternative")
        if isinstance(alt, dict):
            ab = BC()
            compile_one(alt, ab, strs, field_map, True)
            bc.u16(len(ab))
            bc.data.extend(ab.data)
        else:
            bc.u16(0)
        return

    if a == "conditional_on_optional":
        bc.u8(0x63)
        bc.u8(1 if eff.get("optional") else 0)
        return
    if a == "conditional_on_result":
        bc.u8(0x64)
        return
    if a == "choice":
        bc.u8(0x65)
        bc.data.extend(encode(eff.get("count", 1), "u8", strs))
        bc.u16(strs.idx(first_str(eff.get("group_names"))))
        for ck in ("choice_condition", "alternative_condition"):
            c = eff.get(ck)
            if isinstance(c, dict):
                cb = BC()
                compile_condition(c, cb, strs)
                bc.u16(len(cb))
                bc.data.extend(cb.data)
            else:
                bc.u16(0)
        bc.u8(1 if eff.get("alternative_count_type") == "any_number" else 0)
        opts = eff.get("options", [])
        bc.u8(len(opts))
        for opt in opts:
            compile_one(opt, bc, strs, field_map, True)
        return
    if a == "sequential":
        actions = eff.get("actions", [])
        bc.u8(0x60)
        bc.u8(len(actions))
        for act in actions:
            compile_one(act, bc, strs, field_map, True)
        return
    if a == "look_and_select":
        look = eff.get("look_action", {})
        select = eff.get("select_action", {})
        bc.u8(0x70)
        bc.data.extend(encode(look.get("count", 1), "u8", strs))
        bc.data.extend(encode(look.get("source", "deck_top"), "zone", strs))
        bc.data.extend(encode(look.get("target", "self"), "player", strs))
        bc.u8(0x71)
        bc.data.extend(encode(select.get("count", 1), "u8", strs))
        bc.data.extend(encode(select.get("destination", "hand"), "zone", strs))
        bc.u8(1 if select.get("discard_remaining") else 0)
        return
    if a == "conditional_alternative":
        bc.u8(0x62)
        return

    opcode = EFFECT_OPCODES.get(a)
    if opcode is None:
        return
    bc.u8(opcode)
    for ftype, fname in field_map.get(a, []):
        bc.data.extend(encode(eff.get(fname), ftype, strs))


def compile_all(abilities):
    """Store each `unique_abilities[i]` entry as a compact *binary JSON* slice.

    Binary JSON = a tagged tree (see `read_value` in vm.rs) with all strings
    (object keys AND string values) interned into a single `STRINGS` table and
    referenced by 2-byte indices. Because field names are no longer repeated as
    text on every ability, the blob is far smaller than the text JSON (no
    per-ability key duplication, no whitespace, no UTF-8 quoting) — yet the
    decoder reconstructs the *exact same* ``serde_json::Value`` the text loader
    would, and then runs the identical ``from_value`` + ``populate_from_json`` +
    draw-fix post-processing.

    This is fully data-driven: the codec is generic over any JSON shape, so a
    new action type or field needs ZERO encoder/decoder changes.

    The top-level ``cards`` field is skipped: it is the card_no→ability mapping
    consumed only by the loader (not an ``Ability`` field) and is large, so
    dropping it shrinks the blob with zero effect on the decoded ``Ability``.
    """
    # Top-level keys that are loader metadata, not part of `Ability`, and are
    # large/redundant in the binary.
    SKIP_KEYS = {"cards"}

    strings = []  # interned strings (UTF-8)
    string_idx = {}  # str -> u16 index

    def intern(s):
        if s not in string_idx:
            if len(strings) >= 0x10000:
                # Fallback: pathological case, just re-emit. Should never happen
                # for our vocabulary (keys + a bounded set of ability text).
                return 0xFFFF
            string_idx[s] = len(strings)
            strings.append(s)
        return string_idx[s]

    # Keys whose array values are vectors of effect objects in the bytecode
    # schema (see read_effect_vec_value / read_effect_vec_boxed_value).
    EFFECT_LIST_KEYS = ("actions", "options", "effect_steps")

    # Keys whose dict value is a Condition object (read_condition_value), plus
    # the array key whose elements are Conditions ("conditions").
    CONDITION_KEYS = (
        "condition",
        "alternative_condition",
        "result_condition",
        "choice_condition",
        "activation_condition_parsed",
        "cause",
    )

    # Condition "type" string -> variant tag. Mirrors the `#[serde(tag = "type")]`
    # Condition enum in engine/src/core/card.rs, including every alias.
    COND_TO_VARIANT_TAG = {
        "compound": 0,
        "or_condition": 0,
        "card_count_condition": 1,
        "location_condition": 1,
        "comparison_condition": 2,
        "both_condition": 2,
        "all_cost_comparison_condition": 2,
        "highest_cost_on_stage_condition": 2,
        "movement_condition": 3,
        "not_moved": 3,
        "has_moved": 3,
        "group_condition": 4,
        "appearance_condition": 5,
        "temporal_condition": 6,
        "state_condition": 7,
        "energy_state_condition": 7,
        "state_change_condition": 7,
        "resource_condition": 8,
        "card_blade_condition": 8,
        "ability_filter_condition": 9,
        "score_threshold_condition": 10,
        "choice_condition": 11,
        "position_change_condition": 11,
        "complex_condition": 12,
        "position_condition": 13,
        "opponent_choice_condition": 14,
        "opponent_live_success": 15,
        "no_excess_heart": 16,
        "otherwise_condition": 17,
        "action_success_condition": 17,
        "custom": 17,
        "any_of_condition": 18,
        "all_revealed_match_heart_color": 19,
    }

    ACTION_TO_VARIANT_TAG = {
        # Empty action: no-action/no-type dicts that sit directly inside an
        # effect vector (filter options) decode as variant 3 (SelectTarget),
        # whose EffectFilter carries the option's fields.
        "": 3,
        "move_cards": 1,
        "discard_card": 1,
        "discard_until_count": 1,
        "place_energy_under_member": 1,
        "re_yell": 1,
        "shuffle": 1,
        "play_baton_touch": 1,
        "double_baton_touch": 1,
        "draw": 2,
        "draw_card": 2,
        "draw_until_count": 2,
        "select": 3,
        "select_cards": 3,
        "select_number": 3,
        "choose_target_player": 3,
        "look": 4,
        "look_at": 4,
        "reveal": 4,
        "reveal_effect": 4,
        "reveal_per_group": 4,
        "reveal_until_live_card": 4,
        "reveal_until_chosen_card": 4,
        "look_and_select": 4,
        "modify_score": 5,
        "modify_required_hearts": 6,
        "modify_required_hearts_global": 6,
        "modify_required_hearts_success": 6,
        "gain_resource": 7,
        "pay_energy": 7,
        "change_state": 8,
        "set_card_identity": 8,
        "set_card_identity_all_regions": 8,
        "gain_ability": 9,
        "gain_ability_from_source": 9,
        "invalidate_ability": 9,
        "suppress_ability_trigger": 9,
        "activate_ability": 9,
        "sequential": 10,
        "sequential_cost": 10,
        "choice": 10,
        "choice_condition": 10,
        "repeat_procedure": 10,
        "conditional_alternative": 10,
        "conditional_on_optional": 10,
        "conditional_on_result": 10,
        "restriction": 11,
        "activation_restriction": 11,
        "modify_limit": 11,
        "all_blade_timing": 11,
        "reduce_live_card_set_limit": 11,
        "position_change": 12,
        "rotation": 12,
        "set_cost": 13,
        "set_cost_to_use": 13,
        "modify_cost": 13,
        "activation_cost": 13,
        "set_blade_type": 13,
        "set_blade_count": 13,
        "set_heart_type": 13,
        "specify_heart_color": 13,
        "choose_required_hearts": 13,
        "perform_yell": 13,
        "modify_yell_count": 13,
        "custom": 14,
        "do_nothing": 14,
        "action_by": 14,
        "opponent_action": 14,
    }

    # Cost objects (AbilityCost) carry `type` instead of `action`. The values are
    # valid action names (ActionType::from_str covers every one), so the alias is
    # identity — sequential_cost/choice_condition stay distinct ActionType variants
    # (SequentialCost/ChoiceCondition), they are NOT folded into sequential/choice.
    # Their sub-effects live in `costs`/`options`, which the decoder reads as
    # `actions` (the compound list). NOTE: `choice_condition` is also a Condition
    # variant name, but in the current abilities.json it only appears as a cost type
    # — verified by scan, and the bytecode deep-compare test guards this corpus.
    COST_TYPE_TO_ACTION = {
        "move_cards": "move_cards",
        "pay_energy": "pay_energy",
        "reveal": "reveal",
        "change_state": "change_state",
        "place_energy_under_member": "place_energy_under_member",
        "sequential_cost": "sequential_cost",
        "choice_condition": "choice_condition",
    }

    def enc_val(
        v, out: bytearray, in_effect_vec: bool = False, is_condition: bool = False
    ):
        if v is None:
            out.append(0x00)
        elif isinstance(v, bool):
            out.append(0x02 if v else 0x01)
        elif isinstance(v, int):
            out.append(0x03)
            out.extend(struct.pack("<q", v))
        elif isinstance(v, float):
            out.append(0x04)
            out.extend(struct.pack("<d", v))
        elif isinstance(v, str):
            out.append(0x06)
            out.extend(struct.pack("<H", intern(v)))
        elif isinstance(v, list):
            out.append(0x07)
            out.extend(struct.pack("<I", len(v)))
            for item in v:
                enc_val(item, out, in_effect_vec, is_condition)
        elif isinstance(v, dict):
            if is_condition:
                # Condition object: encode as TAG_OBJECT_VARIANT with the
                # Condition variant tag. The "type" key is the tag itself, so it
                # is dropped before counting the remaining fields (the decoder
                # dispatches by tag byte). Unknown types fall back to TAG_OBJECT.
                t = v.get("type") or ""
                # None if the type string is unknown; 0 is a valid tag (Compound).
                vtag = COND_TO_VARIANT_TAG.get(t)
                # `or_condition` is the Compound variant with OR semantics: the
                # JSON oracle (condition_populate_from_json) forces operator="or"
                # when the type alias is used. Replicate that at compile time so
                # the decoded bytecode matches the oracle.
                if t == "or_condition" and "operator" not in v:
                    v["operator"] = "or"
                if vtag is not None:
                    v.pop("type", None)
                    out.append(0x09)  # TAG_OBJECT_VARIANT
                    out.append(vtag)
                else:
                    out.append(0x08)  # TAG_OBJECT (serde fallback; keeps "type")
                out.extend(struct.pack("<I", len(v)))
                for k, val in v.items():
                    out.extend(struct.pack("<H", intern(str(k))))
                    enc_val(
                        val,
                        out,
                        k in EFFECT_LIST_KEYS,
                        k in CONDITION_KEYS or k == "conditions",
                    )
            elif "action" not in v:
                # Cost objects (AbilityCost) use `type`/`zone` instead of
                # `action`/`source`, and compound costs use `costs`/`options`
                # instead of `actions`. Alias them here so costs flow through the
                # same TAG_OBJECT_VARIANT effect decoder as normal effects,
                # eliminating the runtime normalize_cost_keys serde path.
                t = v.get("type")
                if t in COST_TYPE_TO_ACTION:
                    v["action"] = COST_TYPE_TO_ACTION[t]
                    v.pop("type", None)
                    if "source" not in v and "zone" in v:
                        v["source"] = v.pop("zone")
                    if "actions" not in v:
                        for k in ("costs", "options"):
                            if k in v:
                                v["actions"] = v.pop(k)
                                break
                elif in_effect_vec:
                    # Filter option with neither action nor type (direct element
                    # of an effect vector): fabricate an empty action so it
                    # decodes as variant 3 (SelectTarget), whose EffectFilter
                    # carries the option's filter fields (group_names, card_type,
                    # card_property, ...).
                    v["action"] = ""
            if not is_condition:
                action = v.get("action", "")
                vtag = ACTION_TO_VARIANT_TAG.get(action, 0)
                if vtag:
                    out.append(0x09)  # TAG_OBJECT_VARIANT
                    out.append(vtag)
                else:
                    out.append(0x08)  # TAG_OBJECT
                out.extend(struct.pack("<I", len(v)))
                for k, val in v.items():
                    out.extend(struct.pack("<H", intern(str(k))))
                    enc_val(
                        val,
                        out,
                        k in EFFECT_LIST_KEYS,
                        k in CONDITION_KEYS or k == "conditions",
                    )
        else:
            out.append(0x00)

    def enc_entry(entry, out: bytearray):
        # Object with `cards` (loader-only mapping) stripped.
        out.append(0x08)
        out.extend(struct.pack("<I", sum(1 for k in entry if k not in SKIP_KEYS)))
        for k, val in entry.items():
            if k in SKIP_KEYS:
                continue
            out.extend(struct.pack("<H", intern(str(k))))
            enc_val(
                val,
                out,
                k in EFFECT_LIST_KEYS,
                k in CONDITION_KEYS or k == "conditions",
            )

    offsets, bytecode = [], bytearray()

    # Extract card→ability pairs BEFORE encoding (so card_no values get interned)
    card_ability_pairs = []
    for idx, entry in enumerate(abilities):
        for card_entry in entry.get("cards", []):
            # cards field format: "card_no | Ability Name"
            card_no = card_entry.split(" | ")[0] if " | " in card_entry else card_entry
            str_idx = intern(card_no)
            card_ability_pairs.append((str_idx, idx))

    for entry in abilities:
        offsets.append(len(bytecode))
        enc_entry(entry, bytecode)
    offsets.append(len(bytecode))

    # ── Phase 3: Reorder strings by frequency, rewrite with u8 indices ──
    bytecode, offsets, strings, card_ability_pairs = compact_bytecode(
        bytes(bytecode), offsets, strings, card_ability_pairs
    )

    return bytes(bytecode), offsets, strings, card_ability_pairs


def compact_bytecode(bytecode, offsets, strings, card_ability_pairs):
    """Reorder strings by frequency and rewrite bytecode with u8+escape indices."""
    freq = [0] * len(strings)

    def count_one(bc, pos):
        """Count string references in a single value starting at pos. Return new pos."""
        if pos >= len(bc):
            return pos
        tag = bc[pos]
        pos += 1
        if tag in (0x00, 0x01, 0x02):
            return pos
        elif tag == 0x03:
            return pos + 8
        elif tag == 0x04:
            return pos + 8
        elif tag == 0x06:
            if pos + 2 > len(bc):
                return len(bc)
            idx = bc[pos] | (bc[pos + 1] << 8)
            pos += 2
            if idx < len(freq):
                freq[idx] += 1
            return pos
        elif tag == 0x07:
            if pos + 4 > len(bc):
                return len(bc)
            n = bc[pos] | (bc[pos + 1] << 8) | (bc[pos + 2] << 16) | (bc[pos + 3] << 24)
            pos += 4
            for _ in range(n):
                pos = count_one(bc, pos)
            return pos
        elif tag == 0x08:
            if pos + 4 > len(bc):
                return len(bc)
            n = bc[pos] | (bc[pos + 1] << 8) | (bc[pos + 2] << 16) | (bc[pos + 3] << 24)
            pos += 4
            for _ in range(n):
                if pos + 2 > len(bc):
                    return len(bc)
                kidx = bc[pos] | (bc[pos + 1] << 8)
                pos += 2
                if kidx < len(freq):
                    freq[kidx] += 1
                pos = count_one(bc, pos)
            return pos
        elif tag == 0x09:
            pos += 1  # skip variant tag byte
            if pos + 4 > len(bc):
                return len(bc)
            n = bc[pos] | (bc[pos + 1] << 8) | (bc[pos + 2] << 16) | (bc[pos + 3] << 24)
            pos += 4
            for _ in range(n):
                if pos + 2 > len(bc):
                    return len(bc)
                kidx = bc[pos] | (bc[pos + 1] << 8)
                pos += 2
                if kidx < len(freq):
                    freq[kidx] += 1
                pos = count_one(bc, pos)
            return pos
        return pos

    # Count frequencies by walking each ability slice
    for i in range(len(offsets) - 1):
        s, e = offsets[i], offsets[i + 1]
        if s < e:
            count_one(bytecode, s)

    # Build reorder map: most frequent strings get indices 0..253
    indexed = list(range(len(strings)))
    indexed.sort(key=lambda i: (-freq[i], i))
    new_idx = [0] * len(strings)
    for new_pos, old_pos in enumerate(indexed):
        new_idx[old_pos] = new_pos

    new_strings = [strings[old] for old in indexed]

    # Remap card_ability_pairs
    new_pairs = []
    for str_idx, ability_idx in card_ability_pairs:
        new_pairs.append((new_idx[str_idx], ability_idx))

    # Rewrite bytecode with new indices using u8+escape encoding
    new_bytecode = bytearray()
    new_offsets = []

    def write_idx(out, idx):
        if idx < 0xFE:
            out.append(idx)
        else:
            out.append(0xFE)
            out.extend(struct.pack("<H", idx))

    def rewrite_one(bc, pos, out):
        """Rewrite a single value from old bytecode at pos into out. Return new pos."""
        if pos >= len(bc):
            return pos
        tag = bc[pos]
        pos += 1
        out.append(tag)
        if tag in (0x00, 0x01, 0x02):
            return pos
        elif tag == 0x03:
            out.extend(bc[pos : pos + 8])
            return pos + 8
        elif tag == 0x04:
            out.extend(bc[pos : pos + 8])
            return pos + 8
        elif tag == 0x06:
            if pos + 2 > len(bc):
                return len(bc)
            old_idx = bc[pos] | (bc[pos + 1] << 8)
            pos += 2
            write_idx(out, new_idx[old_idx])
            return pos
        elif tag == 0x07:
            if pos + 4 > len(bc):
                return len(bc)
            n = bc[pos] | (bc[pos + 1] << 8) | (bc[pos + 2] << 16) | (bc[pos + 3] << 24)
            pos += 4
            out.extend(struct.pack("<I", n))
            for _ in range(n):
                pos = rewrite_one(bc, pos, out)
            return pos
        elif tag == 0x08:
            if pos + 4 > len(bc):
                return len(bc)
            n = bc[pos] | (bc[pos + 1] << 8) | (bc[pos + 2] << 16) | (bc[pos + 3] << 24)
            pos += 4
            out.extend(struct.pack("<I", n))
            for _ in range(n):
                if pos + 2 > len(bc):
                    return len(bc)
                old_kidx = bc[pos] | (bc[pos + 1] << 8)
                pos += 2
                mapped = new_idx[old_kidx]
                write_idx(out, mapped)
                pos = rewrite_one(bc, pos, out)
            return pos
        elif tag == 0x09:
            vtag = bc[pos]
            pos += 1
            out.append(vtag)
            if pos + 4 > len(bc):
                return len(bc)
            n = bc[pos] | (bc[pos + 1] << 8) | (bc[pos + 2] << 16) | (bc[pos + 3] << 24)
            pos += 4
            out.extend(struct.pack("<I", n))
            for _ in range(n):
                if pos + 2 > len(bc):
                    return len(bc)
                old_kidx = bc[pos] | (bc[pos + 1] << 8)
                pos += 2
                mapped = new_idx[old_kidx]
                write_idx(out, mapped)
                pos = rewrite_one(bc, pos, out)
            return pos
        return pos

    for i in range(len(offsets) - 1):
        s, e = offsets[i], offsets[i + 1]
        new_offsets.append(len(new_bytecode))
        if s < e:
            rewrite_one(bytecode, s, new_bytecode)
    new_offsets.append(len(new_bytecode))

    print(
        f"Bytecode: {len(bytecode)} -> {len(new_bytecode)} bytes ({100 * (1 - len(new_bytecode) / len(bytecode)):.1f}% smaller)"
    )
    top_n = min(254, len(indexed))
    top_freq = sum(freq[indexed[j]] for j in range(top_n))
    total_freq = sum(freq)
    print(
        f"Strings: {len(strings)} unique, top {top_n} cover {top_freq}/{total_freq} refs ({100 * top_freq / total_freq:.1f}%)"
    )

    return new_bytecode, new_offsets, new_strings, new_pairs


# ── Rust code generation ──
def generate_abilities_gen(bytecode, offsets, strings, card_ability_pairs, build_dir):
    # Emit the concatenated binary-JSON as a byte array (24-byte rows).
    hex_lines = []
    for i in range(0, len(bytecode), 24):
        chunk = bytecode[i : i + 24]
        hex_lines.append("    " + ", ".join(f"0x{b:02x}" for b in chunk) + ",")

    str_lits = ", ".join(json.dumps(s, ensure_ascii=False) for s in strings)
    pair_strs = ", ".join(f"{s},{a}" for s, a in card_ability_pairs)

    src = f"""// Auto-generated by compile_abilities.py
//
// Each ability in `unique_abilities` is stored as a compact *binary JSON* slice
// (tagged tree with interned strings). `get_ability(idx)` decodes
// `BYTECODE[OFFSETS[idx]..OFFSETS[idx+1]]` back into the SAME `serde_json::Value`
// the text loader would produce, then runs the identical post-processing
// (`from_value::<Ability>` + `populate_from_json` + draw-count fix). Field names
// are interned into `STRINGS` so the blob is far smaller than text JSON, while
// remaining fully data-driven: new action types / fields need zero decoder
// changes.

pub const NUM_ABILITIES: usize = {len(offsets) - 1};

pub const BYTECODE: &[u8] = &[
{chr(10).join(hex_lines)}
];

/// Byte offsets into `BYTECODE`. `OFFSETS[i]..OFFSETS[i+1]` is the binary-JSON
/// slice for `unique_abilities[i]`. Stored as `u32` to support large ROM data.
pub const OFFSETS: &[u32] = &[{", ".join(str(o) for o in offsets)}];

/// Interned strings: object keys and string values. Indexed by the 2-byte
/// `u16` references inside `BYTECODE`.
pub const STRINGS: &[&str] = &[{str_lits}];

/// Card_no → ability index pairs. Each entry is (card_no_string_index, ability_index).
/// Generated from the `cards` field of `unique_abilities`. Used at load time by
/// `CardLoader::build_abilities_map_shared` to build the card_no → Vec<AbilityRef>
/// mapping without parsing abilities.json into a `serde_json::Value`.
///
/// Format: flat array of [str_idx, ability_idx, str_idx, ability_idx, ...]
pub const CARD_ABILITY_PAIRS: &[u16] = &[{pair_strs}];
"""
    (build_dir / "abilities_gen.rs").write_text(src, encoding="utf-8")
    # The crate compiles `src/ability/abilities_gen.rs`, so mirror the artifact
    # there as well. (Kept in sync so a regen is a single command.)
    engine_dir = Path(__file__).parent.parent / "engine" / "src" / "ability"
    if engine_dir.exists():
        (engine_dir / "abilities_gen.rs").write_text(src, encoding="utf-8")


# ── Main ──
def main():
    root = Path(__file__).parent
    with open(root / "abilities.json", encoding="utf-8") as f:
        data = json.load(f)
    abilities = data["unique_abilities"]
    print(f"Compiling {len(abilities)} abilities...")

    bytecode, offsets, strings, card_ability_pairs = compile_all(abilities)
    build_dir = root / "build"
    build_dir.mkdir(parents=True, exist_ok=True)

    (build_dir / "abilities.bin").write_bytes(bytecode)
    print(f"\n  abilities.bin: {len(bytecode)} bytes ({len(bytecode) / 1024:.1f}KB)")
    print(f"  interned strings: {len(strings)}")
    print(f"  card→ability pairs: {len(card_ability_pairs)}")

    generate_abilities_gen(bytecode, offsets, strings, card_ability_pairs, build_dir)
    print(f"  Avg: {len(bytecode) / len(abilities):.1f} bytes/ability")

    # Write generation manifest for reproducibility tracking
    abilities_json_path = root / "abilities.json"
    abilities_hash = hashlib.sha256(abilities_json_path.read_bytes()).hexdigest()[:16]
    bytecode_hash = hashlib.sha256(bytecode).hexdigest()[:16]

    git_hash = "unknown"
    try:
        import subprocess

        result = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            capture_output=True,
            text=True,
            cwd=str(root.parent),
            timeout=5,
        )
        if result.returncode == 0:
            git_hash = result.stdout.strip()
    except Exception:
        pass

    manifest = {
        "schema": "compiled_abilities.v1",
        "compiler": "cards/compile_abilities.py",
        "engine_commit": git_hash,
        "input": {
            "source": "cards/abilities.json",
            "sha256": abilities_hash,
            "unique_abilities": len(abilities),
        },
        "output": {
            "bytecode_bytes": len(bytecode),
            "interned_strings": len(strings),
            "card_ability_pairs": len(card_ability_pairs),
            "sha256": bytecode_hash,
        },
    }
    (build_dir / "generation_manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )
    print(f"  manifest: generation_manifest.json")


if __name__ == "__main__":
    main()
