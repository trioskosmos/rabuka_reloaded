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

    ACTION_TO_VARIANT_TAG = {
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
        "choice": 10,
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

    def enc_val(v, out: bytearray):
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
                enc_val(item, out)
        elif isinstance(v, dict):
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
                enc_val(val, out)
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
            enc_val(val, out)

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
<<<<<<< Updated upstream
    # The crate compiles `src/ability/abilities_gen.rs`, so mirror the artifact
    # there as well. (Kept in sync so a regen is a single command.)
    engine_dir = Path(__file__).parent.parent / "engine" / "src" / "ability"
    if engine_dir.exists():
        (engine_dir / "abilities_gen.rs").write_text(src, encoding="utf-8")
=======


def generate_vm_gen(build_dir, field_map):
    import re

    card_rs = Path(__file__).parent.parent / "engine/src/core/card.rs"
    with open(card_rs, encoding="utf-8") as f:
        content = f.read()

    # Parse Condition enum fields
    ci = content.find("pub enum Condition")
    ce = content.find("};", ci) + 2
    cf = {}
    cv = None
    for line in content[ci:ce].split("\n"):
        m = re.match(r"    (\w+) \{", line)
        if m:
            cv = m.group(1)
            cf[cv] = {}
        m2 = re.match(r"        (\w+): (.+),", line)
        if m2 and cv:
            cf[cv][m2.group(1)] = m2.group(2).strip()
        if line.strip() == "}," and cv:
            cv = None

    # Parse EffectKind enum fields
    ei = content.find("pub enum EffectKind")
    ee = content.find("};", ei) + 2
    ekf = {}
    ev = None
    for line in content[ei:ee].split("\n"):
        m = re.match(r"    (\w+) \{", line)
        if m:
            ev = m.group(1)
            ekf[ev] = {}
        m2 = re.match(r"        (\w+): (.+),", line)
        if m2 and ev:
            ekf[ev][m2.group(1)] = m2.group(2).strip()
        if line.strip() == "}," and ev:
            ev = None

    FIELD_ALIAS = {
        "DrawCards": {
            "count": "target_count",
            "baton_touch_trigger": "_skip",
            "parenthetical": "_skip",
            "activation_position": "_skip",
            "answers": "_skip",
            "duration": "_skip",
            "group_names": "_skip",
            "multiple_targets": "_skip",
            "optional": "_skip",
            "per_unit_heart_colors": "_skip",
            "per_unit_location": "_skip",
            "position_compare": "_skip",
            "trigger_type": "_skip",
        },
        "DrawUntilCount": {
            "count": "target_count",
            "activation_position": "_skip",
            "answers": "_skip",
            "duration": "_skip",
            "group_names": "_skip",
            "multiple_targets": "_skip",
            "optional": "_skip",
            "per_unit_heart_colors": "_skip",
            "per_unit_location": "_skip",
            "position_compare": "_skip",
            "trigger_type": "_skip",
        },
        "SelectTarget": {
            "count": "target_count",
            "activation_position": "_skip",
            "answers": "_skip",
            "characters": "_skip",
            "choice_options": "_skip",
            "cost_limit": "_skip",
            "cost_limit_operator": "_skip",
            "duration": "_skip",
            "effect_steps": "_skip",
            "exclude_characters": "_skip",
            "exclude_group_names": "_skip",
            "filter_targets_by_heart_colors": "_skip",
            "group_reference": "_skip",
            "heart_color_count": "_skip",
            "heart_colors": "_skip",
            "multiple_targets": "_skip",
            "or_card_types": "_skip",
            "per_unit_heart_colors": "_skip",
            "per_unit_location": "_skip",
            "position": "_skip",
            "question": "_skip",
            "require_all_heart_colors": "_skip",
        },
        "GainResource": {
            "count": "value",
            "baton_touch_trigger": "_skip",
            "conditional": "_skip",
            "max": "_skip",
            "max_repeats": "repeat_limit",
            "parenthetical": "_skip",
            "per_unit_source": "_skip",
            "target": "_skip",
            "activation_position": "_skip",
            "answers": "_skip",
            "characters": "_skip",
            "exclude_characters": "_skip",
            "exclude_group_names": "_skip",
            "filter_targets_by_heart_colors": "_skip",
            "group_reference": "_skip",
            "heart_color_count": "_skip",
            "heart_colors_from_selected_card": "_skip",
            "heart_type": "_skip",
            "multiple_targets": "_skip",
            "or_card_types": "_skip",
            "per_unit_heart_colors": "_skip",
            "per_unit_location": "_skip",
            "require_all_heart_colors": "_skip",
            "same_name": "_skip",
            "state": "_skip",
            "target_count": "_skip",
            "target_from_selection": "_skip",
            "timing_condition": "_skip",
            "trigger_type": "_skip",
        },
        "ModifyHearts": {
            "count": "value",
            "baton_touch_trigger": "_skip",
            "conditional": "_skip",
            "max": "_skip",
            "max_repeats": "repeat_limit",
            "non_stackable": "_skip",
            "parenthetical": "_skip",
            "target": "_skip",
            "activation_position": "_skip",
            "exclude_heart_colors": "_skip",
            "group_reference": "_skip",
            "original_count": "_skip",
            "original_operator": "_skip",
            "per_unit_heart_colors": "_skip",
            "per_unit_type": "_skip",
            "position": "_skip",
            "replace_all": "_skip",
            "timing_condition": "_skip",
        },
        "ModifyScore": {
            "count": "value",
            "conditional": "_skip",
            "parenthetical": "_skip",
            "max_repeats": "repeat_limit",
            "activation_position": "_skip",
            "card_names": "_skip",
            "card_property": "_skip",
            "cost_total": "_skip",
            "cost_total_operator": "_skip",
            "distinct": "_skip",
            "effect_constraint": "_skip",
            "filter_targets_by_heart_colors": "_skip",
            "heart_colors": "_skip",
            "need_heart_operator": "_skip",
            "need_heart_total": "_skip",
            "negation": "_skip",
            "per_unit_heart_colors": "_skip",
            "per_unit_location": "_skip",
            "position": "_skip",
            "repeat_limit": "_skip",
            "state": "_skip",
            "target_count": "_skip",
        },
        "CustomOp": {
            "count": "value",
            "ability_filter": "_skip",
            "blade_type": "_skip",
            "conditional": "_skip",
            "cost_limit": "_skip",
            "cost_limit_operator": "_skip",
            "destination": "_skip",
            "non_stackable": "_skip",
            "operation": "_skip",
            "original_count": "_skip",
            "original_operator": "_skip",
            "per_unit": "_skip",
            "per_unit_count": "_skip",
            "per_unit_location": "_skip",
            "per_unit_type": "_skip",
            "source": "_skip",
            "target": "_skip",
            "value": "_skip",
            "duration": "_skip",
            "original_value": "_skip",
            "exclude_self": "_skip",
            "group_names": "_skip",
            "location": "_skip",
            "card_type": "_skip",
            "self_target": "_skip",
        },
        "MiscOp": {
            "count": "value",
            "activation_position": "_skip",
            "blade_limit": "_skip",
            "blade_limit_operator": "_skip",
            "blade_type": "_skip",
            "card_names": "_skip",
            "characters": "_skip",
            "choice": "_skip",
            "cost_total": "_skip",
            "cost_total_operator": "_skip",
            "cost_reference": "_skip",
            "cost_offset": "_skip",
            "distinct": "_skip",
            "effect_constraint": "_skip",
            "energy_count": "_skip",
            "exclude_characters": "_skip",
            "exclude_group_names": "_skip",
            "group_names": "_skip",
            "group_reference": "_skip",
            "heart_color_count": "_skip",
            "heart_colors": "_skip",
            "heart_selection": "_skip",
            "heart_type": "_skip",
            "identities": "_skip",
            "id": "_skip",
            "lose_blade_hearts": "_skip",
            "location": "_skip",
            "negation": "_skip",
            "operation": "_skip",
            "options": "_skip",
            "or_card_types": "_skip",
            "original_cost": "_skip",
            "original_count": "_skip",
            "original_operator": "_skip",
            "original_value": "_skip",
            "parenthetical": "_skip",
            "per_group": "_skip",
            "per_group_count": "_skip",
            "per_unit": "_skip",
            "per_unit_count": "_skip",
            "per_unit_heart_colors": "_skip",
            "per_unit_location": "_skip",
            "per_unit_type": "_skip",
            "picker": "_skip",
            "placement_order": "_skip",
            "position": "_skip",
            "ref_offset": "_skip",
            "ref_value": "_skip",
            "repeat_limit": "_skip",
            "require_all_heart_colors": "_skip",
            "resource_icon_count": "_skip",
            "same_unit_name": "_skip",
            "sign": "_skip",
            "target_count": "_skip",
            "timing": "_skip",
            "treat_as": "_skip",
        },
        "ChangeState": {
            "count": "_skip",
            "max": "_skip",
            "parenthetical": "_skip",
            "position_compare": "_skip",
            "ability_filter": "_skip",
            "ability_filter_triggers": "_skip",
            "activation_position": "_skip",
            "card_names": "_skip",
            "characters": "_skip",
            "exclude_characters": "_skip",
            "exclude_group_names": "_skip",
            "exclude_heart_colors": "_skip",
            "filter_targets_by_heart_colors": "_skip",
            "group_reference": "_skip",
            "heart_colors": "_skip",
            "identities": "_skip",
            "name_constraint": "_skip",
            "name_constraint_source": "_skip",
            "negation": "_skip",
            "or_ability_filters": "_skip",
            "or_card_types": "_skip",
            "original_value": "_skip",
        },
        "AbilityOp": {
            "count": "_skip",
            "max": "_skip",
            "parenthetical": "_skip",
            "source_location": "location",
            "activation_position": "_skip",
            "activation_condition_parsed": "_skip",
            "all": "_skip",
            "cost_limit": "_skip",
            "cost_limit_operator": "_skip",
            "dynamic_count": "_skip",
            "effect_type": "_skip",
            "heart_colors": "_skip",
            "option": "_skip",
            "trigger_filter": "_skip",
            "trigger_type": "_skip",
            "triggers": "_skip",
            "use_limit": "_skip",
        },
        "RestrictionOp": {
            "count": "_skip",
            "destination": "_skip",
            "target": "_skip",
            "characters": "_skip",
            "choice_based": "_skip",
            "effect_type": "_skip",
            "exclude_characters": "_skip",
            "exclude_group_names": "_skip",
            "group_names": "_skip",
            "non_stackable": "_skip",
            "operation": "_skip",
            "replaces_event": "_skip",
            "restricted_destination": "_skip",
            "timing": "_skip",
            "timing_condition": "_skip",
            "trigger_filter": "_skip",
            "trigger_type": "_skip",
        },
        "PositionOp": {
            "count": "_skip",
            "parenthetical": "_skip",
            "position_compare": "_skip",
            "any_number": "_skip",
            "cost_from_revealed": "_skip",
            "cost_limit": "_skip",
            "cost_limit_operator": "_skip",
            "dynamic_count": "_skip",
            "energy_count": "_skip",
            "exclude_characters": "_skip",
            "exclude_group_names": "_skip",
            "group_names": "_skip",
            "group_reference": "_skip",
            "multiple_targets": "_skip",
            "state": "_skip",
        },
        "LookReveal": {
            "count": "_skip",
            "activation_position": "_skip",
            "blind": "_skip",
            "card_names": "_skip",
            "characters": "_skip",
            "cost_limit": "_skip",
            "cost_limit_operator": "_skip",
            "distinct": "_skip",
            "dynamic_count": "_skip",
            "exclude_characters": "_skip",
            "exclude_group_names": "_skip",
            "filter_targets_by_heart_colors": "_skip",
            "group_names": "_skip",
            "group_reference": "_skip",
            "heart_color_count": "_skip",
            "heart_colors": "_skip",
            "is_reveal": "_skip",
            "multiple_targets": "_skip",
            "name_constraint": "_skip",
            "name_constraint_source": "_skip",
            "negation": "_skip",
            "optional": "_skip",
            "options": "_skip",
            "or_ability_filters": "_skip",
            "or_card_types": "_skip",
            "original_value": "_skip",
            "per_unit_heart_colors": "_skip",
            "per_unit_location": "_skip",
            "picker": "_skip",
            "require_all_heart_colors": "_skip",
            "resource_on_select": "_skip",
            "reveal": "_skip",
            "self_target": "_skip",
            "state": "_skip",
        },
        "CompoundEffect": {
            "count": "_skip",
            "activation_position": "_skip",
            "activation_condition_parsed": "_skip",
            "alternative_count_type": "_skip",
            "card_type": "_skip",
            "distinct": "_skip",
            "group_reference": "_skip",
            "heart_colors": "_skip",
            "original_value": "_skip",
            "parenthetical": "_skip",
            "per_unit": "_skip",
            "per_unit_count": "_skip",
            "per_unit_type": "_skip",
            "position": "_skip",
            "shuffle": "_skip",
            "trigger_type": "_skip",
        },
    }

    lines = ["// Auto-generated"]

    # Default condition constructors
    for v in sorted(cf.keys()):
        fn = f"default_condition_{v[0].lower()}{v[1:]}"
        lines.append(f"fn {fn}() -> Condition {{")
        lines.append(f"    Condition::{v} {{")
        for fname in sorted(cf[v].keys()):
            lines.append(f"        {fname}: Default::default(),")
        lines.append("    }")
        lines.append("}")
        lines.append("")

    # Default EffectKind constructors
    action_to_variant = {
        "draw_card": "DrawCards",
        "draw_until_count": "DrawCards",
        "move_cards": "MoveCards",
        "gain_resource": "GainResource",
        "modify_score": "ModifyScore",
        "change_state": "ChangeState",
        "position_change": "PositionOp",
        "modify_required_hearts": "ModifyHearts",
        "modify_required_hearts_global": "ModifyHearts",
        "modify_cost": "CustomOp",
        "set_blade_type": "CustomOp",
        "set_blade_count": "MiscOp",
        "set_heart_type": "MiscOp",
        "gain_ability": "AbilityOp",
        "gain_ability_from_source": "AbilityOp",
        "restriction": "RestrictionOp",
        "choose_target_player": "SelectTarget",
        "place_energy_under_member": "MoveCards",
        "modify_yell_count": "ModifyScore",
        "invalidate_ability": "AbilityOp",
        "suppress_ability_trigger": "AbilityOp",
        "activate_ability": "AbilityOp",
        "play_baton_touch": "MoveCards",
        "set_card_identity": "ChangeState",
        "pay_energy": "GainResource",
        "look_at": "LookReveal",
        "select": "SelectTarget",
        "select_cards": "SelectTarget",
        "select_number": "SelectTarget",
        "reveal": "LookReveal",
        "reveal_until_live_card": "LookReveal",
        "do_nothing": "CustomOp",
        "perform_yell": "MiscOp",
        "specify_heart_color": "MiscOp",
        "re_yell": "MiscOp",
        "reduce_live_card_set_limit": "RestrictionOp",
        "discard_until_count": "MoveCards",
        "repeat_procedure": "CompoundEffect",
        "conditional_alternative": "CompoundEffect",
    }
    used_ek = set()
    for a in EFFECT_OPCODES:
        v = action_to_variant.get(a)
        if v:
            used_ek.add(v)
    used_ek.add("CompoundEffect")
    used_ek.add("MoveCards")
    used_ek.add("ChangeState")

    for v in sorted(used_ek):
        fn = f"default_{v[0].lower()}{v[1:]}"
        lines.append(f"fn {fn}() -> EffectKind {{")
        lines.append(f"    EffectKind::{v} {{")
        for fname in sorted(ekf.get(v, {}).keys()):
            ft = ekf[v][fname]
            if ft.startswith("Option<Box<"):
                lines.append(f"        {fname}: None,")
            elif ft.startswith("Box<"):
                lines.append(f"        {fname}: Box::default(),")
            else:
                lines.append(f"        {fname}: Default::default(),")
        lines.append("    }")
        lines.append("}")
        lines.append("")

    # decode_ability_effect — builds AbilityEffect directly from bytecode,
    # setting both EffectKind type tag AND convenience fields
    lines.append(
        "fn decode_effect_kind(op: Opcode, cursor: &mut &[u8]) -> Option<Box<EffectKind>> {"
    )
    lines.append("    match op {")
    for action in sorted(EFFECT_OPCODES.keys()):
        opname = rust_name(action)
        variant = action_to_variant.get(action)
        if variant is None:
            continue
        fn = f"default_{variant[0].lower()}{variant[1:]}"
        fields = field_map.get(action, [])
        lines.append(f"        Opcode::{opname} => {{")
        vars_read = []
        for ftype, fname in fields:
            vname = fname.replace("-", "_")
            lines.append("            " + _read_expr(ftype, vname) + ";")
            vars_read.append((ftype, vname, fname))
        lines.append(f"            let mut ek = {fn}();")
        used = set()
        ek_assigns = []
        for ftype, vname, fname in vars_read:
            sfname = FIELD_ALIAS.get(variant, {}).get(fname, fname)
            if sfname == "_skip":
                continue
            if sfname in used:
                continue
            used.add(sfname)
            ft = ekf.get(variant, {}).get(sfname)
            if ft:
                expr = _assign(ft, vname, ftype)
                if expr:
                    ek_assigns.append((sfname, expr))
        if ek_assigns:
            fl = ", ".join(f"{f}: ref mut _bc_{f}" for f, _ in ek_assigns)
            lines.append(
                f"            if let EffectKind::{variant} {{ {fl}, .. }} = &mut ek {{"
            )
            for f, e in ek_assigns:
                lines.append(f"                *_bc_{f} = {e};")
            lines.append("            }")
        lines.append("            Some(Box::new(ek))")
        lines.append("        }")
    lines.append("        _ => None,")
    lines.append("    }")
    lines.append("}")
    lines.append("")

    # action_for_op
    lines.append("fn action_for_op(op: Opcode) -> &'static str {")
    lines.append("    match op {")
    for action in sorted(EFFECT_OPCODES.keys()):
        lines.append(f'        Opcode::{rust_name(action)} => "{action}",')
    lines.append('        _ => "",')
    lines.append("    }")
    lines.append("}")
    lines.append("")

    # decode_operator_from_str
    lines.append("fn decode_operator_from_str(s: &str) -> Operator {")
    lines.append(
        '    match s { ">=" => Operator::Gte, "<=" => Operator::Lte, ">" => Operator::Gt, "<" => Operator::Lt, "=" => Operator::Eq, _ => Operator::Eq }'
    )
    lines.append("}")
    lines.append("")

    # decode_cond_card_type
    lines.append("fn decode_cond_card_type(v: u8) -> ConditionCardType {")
    lines.append(
        "    match v { 1 => ConditionCardType::MemberCard, 2 => ConditionCardType::LiveCard, 3 => ConditionCardType::EnergyCard, _ => ConditionCardType::MemberCard }"
    )
    lines.append("}")
    lines.append("")

    # decode_condition
    cond_variant = {
        "card_count_condition": "Location",
        "location_condition": "Location",
        "comparison_condition": "Comparison",
        "group_condition": "Group",
        "movement_condition": "Movement",
        "temporal_condition": "Temporal",
        "appearance_condition": "Appearance",
        "state_condition": "State",
        "energy_state_condition": "State",
        "position_condition": "PositionCond",
        "highest_cost_on_stage_condition": "ScoreThreshold",
        "state_change_condition": "State",
        "card_blade_condition": "Resource",
        "all_cost_comparison_condition": "Comparison",
        "ability_filter_condition": "AbilityFilter",
        "has_moved": "Movement",
        "not_moved": "Movement",
        "opponent_live_success": "OpponentLiveSuccess",
        "no_excess_heart": "NoExcessHeart",
    }

    lines.append("pub fn decode_condition(cursor: &mut &[u8]) -> Condition {")
    lines.append("    if cursor.is_empty() { return default_condition_alwaysTrue(); }")
    lines.append("    let op_val = cursor[0];")
    lines.append("    match op_val {")
    for ctype in sorted(COND_FIELDS.keys()):
        code = COND_OPCODES[ctype]
        v = cond_variant[ctype]
        fn = f"default_condition_{v[0].lower()}{v[1:]}"
        fields = COND_FIELDS[ctype]
        lines.append(f"        {code} => {{")
        lines.append("            let _ = read_u8(cursor);")
        vr = []
        for ftype, fname in fields:
            vname = fname.replace("-", "_")
            lines.append("            " + _read_expr(ftype, vname) + ";")
            vr.append((ftype, vname, fname))
        lines.append(f"            let mut c = {fn}();")
        assigns = []
        for ftype, vname, fname in vr:
            ft = cf.get(v, {}).get(fname)
            if ft:
                expr = _cond_assign(ft, vname, ftype, fname)
                if expr:
                    assigns.append((fname, expr))
        if assigns:
            fl = ", ".join(f"{f}: ref mut _bc_{f}" for f, _ in assigns)
            lines.append(
                f"            if let Condition::{v} {{ {fl}, .. }} = &mut c {{"
            )
            for f, e in assigns:
                lines.append(f"                *_bc_{f} = {e};")
            lines.append("            }")
        lines.append("            c")
        lines.append("        }")
    lines.append("        0x4A | 0x4B => {")
    lines.append("            let _ = read_u8(cursor);")
    lines.append('            let op_str = if op_val == 0x4A { "or" } else { "and" };')
    lines.append("            let mut conditions = Vec::new();")
    lines.append("            loop {")
    lines.append("                if cursor.is_empty() || cursor[0] == 0x4C {")
    lines.append(
        "                    if !cursor.is_empty() { let _ = read_u8(cursor); }"
    )
    lines.append("                    break;")
    lines.append("                }")
    lines.append("                conditions.push(Box::new(decode_condition(cursor)));")
    lines.append("            }")
    lines.append(
        "            if conditions.is_empty() { default_condition_alwaysTrue() }"
    )
    lines.append(
        "            else if conditions.len() == 1 { *conditions.into_iter().next().unwrap() }"
    )
    lines.append("            else { let mut c = default_condition_compound();")
    lines.append(
        "                if let Condition::Compound { operator: ref mut _bc_o, conditions: ref mut _bc_cond, .. } = &mut c {"
    )
    lines.append(
        "                    *_bc_o = Some(op_str.into()); *_bc_cond = Some(conditions);"
    )
    lines.append("                } c")
    lines.append("            }")
    lines.append("        }")
    lines.append("        _ => default_condition_alwaysTrue(),")
    lines.append("    }")
    lines.append("}")
    lines.append("")

    (build_dir / "vm_gen.rs").write_text("\n".join(lines), encoding="utf-8")


def _read_expr(ftype, vname):
    m = {
        "u8": f"let {vname} = read_u8(cursor)",
        "u16": f"let {vname} = read_u16(cursor)",
        "i8": f"let {vname} = read_i8(cursor)",
        "bool": f"let {vname} = read_u8(cursor) != 0",
        "zone": f"let {vname} = decode_zone(read_u8(cursor))",
        "player": f"let {vname} = decode_player(read_u8(cursor))",
        "card_type": f"let {vname} = decode_card_type(read_u8(cursor))",
        "resource": f"let {vname} = decode_resource(read_u8(cursor))",
        "heart": f"let {vname} = decode_heart(read_u8(cursor))",
        "state": f"let {vname} = decode_state(read_u8(cursor))",
        "duration": f"let {vname} = decode_duration(read_u8(cursor))",
        "operator": f"let {vname} = decode_operator(read_u8(cursor))",
        "str_idx": f"let {vname} = read_str(cursor)",
    }
    return m.get(ftype, f"let {vname} = read_u8(cursor)")


def _assign(ft, src_var, optype):
    """Generate assignment expression. Returns None if not possible."""
    if ft is None:
        return None
    is_opt = ft.startswith("Option<")
    inner = ft[7:-1] if is_opt else ft
    opt_ret = optype == "str_idx"

    def wr(expr):
        return f"Some({expr})" if is_opt else expr

    if inner == "ArcStr":
        if opt_ret:
            return (
                f"{src_var}.map(|s| s.into())"
                if is_opt
                else f"{src_var}.map_or(Default::default(), |s| s.into())"
            )
        return wr(f"{src_var}.into()")
    if inner == "u32":
        if opt_ret:
            return (
                f"{src_var}.map(|s| s.parse().ok().unwrap_or(0))"
                if is_opt
                else f"{src_var}.map_or(0, |s| s.parse().ok().unwrap_or(0))"
            )
        return wr(f"{src_var} as u32")
    if inner == "bool":
        return wr(f"{src_var}")
    if inner == "String":
        if opt_ret:
            return (
                f"{src_var}.map(|s| s.to_string())"
                if is_opt
                else f"{src_var}.map_or(String::new(), |s| s.to_string())"
            )
        return wr(f"{src_var}.to_string()")
    if "Box<Vec<" in inner or "Vec<" in inner:
        bwrap = "Box::new(" if "Box<" in inner else ""
        bclose = ")" if "Box<" in inner else ""
        if opt_ret:
            if is_opt:
                return f"{src_var}.map(|s| {bwrap}vec![s.to_string()]{bclose})"
            return f"{src_var}.map_or(Default::default(), |s| {bwrap}vec![s.to_string()]{bclose})"
        return wr(f"{bwrap}vec![{src_var}.to_string()]{bclose}")
    if inner == "Operator":
        return wr(f"decode_operator_from_str({src_var})")
    if inner == "ConditionCardType":
        return wr(f"decode_cond_card_type({src_var})") if not opt_ret else None
    return None


def _cond_assign(ft, src_var, optype, fname):
    if ft is None:
        return None
    is_opt = ft.startswith("Option<")
    inner = ft[7:-1] if is_opt else ft
    opt_ret = optype == "str_idx"

    def wr(expr):
        return f"Some({expr})" if is_opt else expr

    if inner == "ArcStr":
        if opt_ret:
            return (
                f"{src_var}.map(|s| s.into())"
                if is_opt
                else f"{src_var}.map_or(Default::default(), |s| s.into())"
            )
        return wr(f"{src_var}.into()")
    if inner == "u32":
        if opt_ret:
            return (
                f"{src_var}.map(|s| s.parse().ok().unwrap_or(0))"
                if is_opt
                else f"{src_var}.map_or(0, |s| s.parse().ok().unwrap_or(0))"
            )
        return wr(f"{src_var} as u32")
    if inner == "bool":
        return wr(f"{src_var}")
    if inner == "String":
        if opt_ret:
            return (
                f"{src_var}.map(|s| s.to_string())"
                if is_opt
                else f"{src_var}.map_or(String::new(), |s| s.to_string())"
            )
        return wr(f"{src_var}.to_string()")
    if "Box<Vec<" in inner or "Vec<" in inner:
        bwrap = "Box::new(" if "Box<" in inner else ""
        bclose = ")" if "Box<" in inner else ""
        if opt_ret:
            if is_opt:
                return f"{src_var}.map(|s| {bwrap}vec![s.to_string()]{bclose})"
            return f"{src_var}.map_or(Default::default(), |s| {bwrap}vec![s.to_string()]{bclose})"
        return wr(f"{bwrap}vec![{src_var}.to_string()]{bclose}")
    if inner == "Operator":
        return wr(f"decode_operator_from_str({src_var})")
    if inner == "ConditionCardType":
        return wr(f"decode_cond_card_type({src_var})") if not opt_ret else None
    if inner in ("PositionInfo", "DistinctType", "PlacementOrder"):
        return None
    return None
>>>>>>> Stashed changes


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
