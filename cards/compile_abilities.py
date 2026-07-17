"""Auto-compiler: scans abilities.json, discovers field types, generates bytecode + Rust decoder."""

import json, struct, re
from pathlib import Path
from collections import defaultdict

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
    if val is None:
        if ftype == "str_idx":
            return struct.pack("<H", 0xFFFF)
        if ftype == "u16":
            return struct.pack("<H", 0)
        return b"\x00"
    if isinstance(val, list):
        if not val:
            return struct.pack("<H", 0xFFFF) if ftype == "str_idx" else b"\x00"
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
            (infer_type(fn, list(vals)), fn) for fn, vals in sorted(fields.items())
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
        ("zone", "location"),
        ("u8", "card_type"),
        ("u8", "count"),
        ("operator", "operator"),
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
        bc.data.extend(encode(cost.get("count", 1), "u8", strs))
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
        bc.data.extend(encode(cost.get("count", 1), "u8", strs))
        bc.data.extend(encode(cost.get("card_type", "card"), "card_type", strs))
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


def compile_all(abilities, field_map):
    strs = StringTable()
    offsets, bytecode = [], bytearray()
    for entry in abilities:
        offsets.append(len(bytecode))
        eff, cost = entry.get("effect", {}), entry.get("cost")
        w = BC()
        if cost:
            compile_cost(cost, w, strs)
        if isinstance(eff, dict):
            compile_one(eff, w, strs, field_map)
        bytecode.extend(w.data)
    offsets.append(len(bytecode))
    return bytes(bytecode), offsets, strs


# ── Rust code generation ──
def rust_name(s):
    return "".join(w.capitalize() for w in s.split("_"))


def generate_abilities_gen(bytecode, offsets, strs, build_dir, field_map):
    all_ops = [(a, EFFECT_OPCODES[a], "effect") for a in sorted(EFFECT_OPCODES.keys())]
    seen = set()
    for ctype, code in COND_OPCODES.items():
        if code not in seen:
            all_ops.append((ctype, code, "condition"))
            seen.add(code)
    for ctype, code in COST_OPCODES.items():
        if code not in seen:
            all_ops.append((ctype, code, "cost"))
            seen.add(code)
    all_ops.sort(key=lambda x: x[1])

    hex_lines = []
    for i in range(0, len(bytecode), 24):
        chunk = bytecode[i : i + 24]
        hex_lines.append("    " + ", ".join(f"0x{b:02x}" for b in chunk) + ",")

    src = f"""// Auto-generated
pub const NUM_ABILITIES: usize = {len(offsets) - 1};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {{
{chr(10).join(f"    {rust_name(n)} = {c}," for n, c, _ in all_ops)}
}}

impl TryFrom<u8> for Opcode {{
    type Error = UnknownOpcode;
    fn try_from(v: u8) -> Result<Self, Self::Error> {{
        match v {{
{chr(10).join(f"            {c} => Ok(Self::{rust_name(n)})," for n, c, _ in all_ops)}
            _ => Err(UnknownOpcode(v)),
        }}
    }}
}}

pub struct UnknownOpcode(pub u8);

pub const BYTECODE: &[u8] = &[
{chr(10).join(hex_lines)}
];

pub const OFFSETS: &[u16] = &[{", ".join(str(o) for o in offsets)}];

pub const STRINGS: &[&str] = &[{", ".join(f'"{s}"' for s in strs)}];
"""
    (build_dir / "abilities_gen.rs").write_text(src, encoding="utf-8")


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
        },
        "DrawUntilCount": {"count": "target_count"},
        "SelectTarget": {"count": "target_count"},
        "GainResource": {
            "count": "value",
            "baton_touch_trigger": "_skip",
            "conditional": "_skip",
            "max": "_skip",
            "max_repeats": "repeat_limit",
            "parenthetical": "_skip",
            "per_unit_source": "_skip",
            "target": "_skip",
        },
        "ModifyScore": {
            "count": "value",
            "conditional": "_skip",
            "parenthetical": "_skip",
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
        },
        "CustomOp": {
            "count": "value",
            "ability_filter": "_skip",
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
            "blade_type": "_skip",
            "duration": "_skip",
        },
        "MiscOp": {"count": "value"},
        "LookReveal": {"count": "count"},
        "ChangeState": {
            "count": "_skip",
            "max": "_skip",
            "parenthetical": "_skip",
            "position_compare": "_skip",
        },
        "AbilityOp": {
            "count": "_skip",
            "max": "_skip",
            "parenthetical": "_skip",
            "source_location": "location",
        },
        "RestrictionOp": {"count": "_skip", "destination": "_skip", "target": "_skip"},
        "PositionOp": {
            "count": "_skip",
            "parenthetical": "_skip",
            "position_compare": "_skip",
        },
        "ModifyScore": {"count": "value", "parenthetical": "_skip"},
        "ModifyHearts": {
            "count": "value",
            "parenthetical": "_skip",
            "max_repeats": "repeat_limit",
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


# ── Main ──
def main():
    root = Path(__file__).parent
    with open(root / "abilities.json", encoding="utf-8") as f:
        data = json.load(f)
    abilities = data["unique_abilities"]
    print(f"Compiling {len(abilities)} abilities...")

    field_map = scan_abilities(abilities)
    print(f"Discovered {len(field_map)} action types:")
    for a in sorted(field_map.keys(), key=lambda x: EFFECT_OPCODES.get(x, 999)):
        fields = field_map[a]
        print(
            f"  0x{EFFECT_OPCODES.get(a, 0):02x} {a}: {[f[0] + ':' + f[1] for f in fields]}"
        )

    bytecode, offsets, strs = compile_all(abilities, field_map)
    build_dir = root / "build"
    build_dir.mkdir(parents=True, exist_ok=True)

    (build_dir / "abilities.bin").write_bytes(bytecode)
    print(f"\n  abilities.bin: {len(bytecode)} bytes ({len(bytecode) / 1024:.1f}KB)")

    generate_abilities_gen(bytecode, offsets, strs, build_dir, field_map)
    generate_vm_gen(build_dir, field_map)
    print(f"  Strings: {len(strs)}, Avg: {len(bytecode) / len(abilities):.1f} bytes")


if __name__ == "__main__":
    main()
