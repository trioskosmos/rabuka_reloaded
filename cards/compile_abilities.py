"""Auto-compiler: scans abilities.json, discovers field types, generates bytecode + Rust decoder."""

import json, struct, re
from pathlib import Path
from collections import defaultdict


def _parse_list_fields():
    """Return the set of EffectKind/Condition field names whose Rust type is a
    Vec-of-strings. These are encoded/decoded as `str_list` so multi-element
    lists (e.g. heart_colors) round-trip losslessly."""
    fields = set()
    try:
        content = (
            Path(__file__).parent / ".." / "engine" / "src" / "core" / "card.rs"
        ).read_text(encoding="utf-8")
    except OSError:
        return fields
    for enum in ("pub enum EffectKind", "pub enum Condition"):
        ei = content.find(enum)
        if ei < 0:
            continue
        ee = content.find("};", ei) + 2
        cur = None
        for line in content[ei:ee].split("\n"):
            m = re.match(r"    (\w+) \{", line)
            if m:
                cur = m.group(1)
            m2 = re.match(r"        (\w+): (.+),", line)
            if m2 and cur:
                t = m2.group(2)
                if "Vec<String>" in t or "Vec<ArcStr" in t:
                    fields.add(m2.group(1))
    return fields


LIST_FIELDS = _parse_list_fields()

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


# ── Field type inference ──
def infer_type(name, values):
    ints = [v for v in values if isinstance(v, int) and not isinstance(v, bool)]
    if ints and all(v == 0 or v == 1 for v in ints):
        bools = [v for v in values if isinstance(v, bool)]
        if len(bools) > len(ints):
            return "bool"
    if any(isinstance(v, bool) for v in values):
        return "bool"
    if ints:
        return "u16" if any(v > 255 for v in ints) else "u8"
    strs = [norm(v) for v in values if isinstance(v, str) and v]
    if strs:
        for vtype, vocab in VOCAB.items():
            if all(s in vocab for s in strs):
                return vtype
        for s in strs:
            if s in ZONES:
                return "zone"
    return "str_idx"


# ── Scan abilities.json ──
def scan_abilities(abilities):
    action_fields = defaultdict(lambda: defaultdict(set))
    compound_actions = {
        "sequential",
        "choice",
        "conditional",
        "conditional_alternative",
        "conditional_on_optional",
        "conditional_on_result",
        "look_and_select",
    }

    def scan(eff, is_sub=False):
        if not isinstance(eff, dict):
            return
        a = eff.get("action", "")
        if not a:
            return
        if a in compound_actions:
            for k, v in eff.items():
                if k in (
                    "action",
                    "condition",
                    "alternative",
                    "actions",
                    "look_action",
                    "select_action",
                    "options",
                    "choice_condition",
                    "alternative_condition",
                    "choice_modifier",
                    "primary_effect",
                    "alternative_effect",
                    "optional_action",
                    "conditional_action",
                    "followup_action",
                    "result_condition",
                    "gained_effect",
                    "trigger_event",
                    "text",
                    "activation_condition_parsed",
                    "quoted_text",
                ):
                    continue
                if isinstance(v, (dict, list)):
                    if isinstance(v, list) and v and isinstance(v[0], str):
                        action_fields[a][k].add(v[0])
                    continue
                action_fields[a][k].add(v)
            for sk in ("actions", "options"):
                for sub in eff.get(sk, []):
                    scan(sub, True)
            for sk in (
                "look_action",
                "select_action",
                "primary_effect",
                "alternative_effect",
                "optional_action",
                "conditional_action",
                "followup_action",
                "result_condition",
                "gained_effect",
            ):
                sub = eff.get(sk)
                if isinstance(sub, dict):
                    scan(sub, True)
            return
        for k, v in eff.items():
            if k in ("action", "condition", "alternative", "text", "continuation"):
                continue
            if isinstance(v, (dict, list)):
                if isinstance(v, list) and v and isinstance(v[0], str):
                    action_fields[a][k].add(v[0])
                continue
            action_fields[a][k].add(v)

    for entry in abilities:
        eff = entry.get("effect")
        if isinstance(eff, dict):
            scan(eff)

    result = {}
    for a, fields in action_fields.items():
        field_list = [
            (
                "str_list" if fn in LIST_FIELDS else infer_type(fn, list(vals)),
                fn,
            )
            for fn, vals in sorted(fields.items())
        ]
        result[a] = field_list
    return result


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
            out.append(0x08)
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
    return bytes(bytecode), offsets, strings, card_ability_pairs


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

    # scan_abilities is kept for diagnostic reporting of the action-type
    # vocabulary; it no longer drives the decoder.
    field_map = scan_abilities(abilities)
    print(f"Discovered {len(field_map)} action types:")
    for a in sorted(field_map.keys(), key=lambda x: EFFECT_OPCODES.get(x, 999)):
        fields = field_map[a]
        print(
            f"  0x{EFFECT_OPCODES.get(a, 0):02x} {a}: {[f[0] + ':' + f[1] for f in fields]}"
        )

    bytecode, offsets, strings, card_ability_pairs = compile_all(abilities)
    build_dir = root / "build"
    build_dir.mkdir(parents=True, exist_ok=True)

    (build_dir / "abilities.bin").write_bytes(bytecode)
    print(f"\n  abilities.bin: {len(bytecode)} bytes ({len(bytecode) / 1024:.1f}KB)")
    print(f"  interned strings: {len(strings)}")
    print(f"  card→ability pairs: {len(card_ability_pairs)}")

    generate_abilities_gen(bytecode, offsets, strings, card_ability_pairs, build_dir)
    print(f"  Avg: {len(bytecode) / len(abilities):.1f} bytes/ability")


if __name__ == "__main__":
    main()
