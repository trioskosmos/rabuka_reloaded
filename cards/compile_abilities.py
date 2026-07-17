"""
compile_abilities.py

Build-time compiler: reads abilities.json, outputs:
  build/abilities.bin         — raw bytecode (~7KB)
  build/abilities_gen.rs      — generated Rust source (opcode enum, offsets, bytecode blob)
  build/vm_gen.rs             — generated Rust VM decoder
  build/abilities_disasm.txt  — human-readable disassembly (debug builds)

Usage: python compile_abilities.py
"""

import json, struct, sys
from pathlib import Path
import textwrap

# ─────────────────────────────────────────────────────────────
# Opcode table — hex values live HERE, not in generated Rust
# Mapping: JSON action/condition name -> byte opcode
# The Python build script is the ONLY place these mappings exist.
# The generated Rust source uses named constants derived from
# JSON names (e.g., Opcode::DrawCard for "draw_card").
# ─────────────────────────────────────────────────────────────

# Effect action -> opcode
EFFECT_OPCODES = {
    "draw_card": 0x01,
    "move_cards": 0x02,
    "gain_resource": 0x03,
    "modify_score": 0x04,
    "change_state": 0x05,
    "position_change": 0x06,
    "modify_required_hearts": 0x07,
    "modify_cost": 0x08,
    "set_blade_type": 0x09,
    "set_blade_count": 0x0A,
    "set_heart_type": 0x0B,
    "gain_ability": 0x0C,
    "restriction": 0x0D,
    "choose_target_player": 0x0E,
    "place_energy_under_member": 0x0F,
    "draw_until_count": 0x10,
    "modify_yell_count": 0x11,
    "invalidate_ability": 0x12,
    "suppress_ability_trigger": 0x13,
    "activate_ability": 0x14,
    "play_baton_touch": 0x15,
    "modify_required_hearts_global": 0x16,
    "gain_ability_from_source": 0x17,
    "set_card_identity": 0x18,
    # Compound effects
    "sequential": 0x60,
    "conditional": 0x61,
    "conditional_alternative": 0x62,
    "conditional_on_optional": 0x63,
    "conditional_on_result": 0x64,
    # Sub-effects
    "look_at": 0x70,
    "select_cards": 0x71,
}

# Condition type -> opcode
CONDITION_OPCODES = {
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

# Cost type -> opcode
COST_OPCODES = {
    "move_cards_cost": 0x80,
    "tap": 0x81,
    "rest": 0x82,
    "energy": 0x83,
    "discard": 0x84,
    "place_energy_under_member_cost": 0x85,
    "pay_energy": 0x86,
    "change_state_cost": 0x87,
    "sequential_cost": 0x88,
    "reveal": 0x89,
    "choice_condition": 0x8A,
}

# All opcodes combined for the reverse map
ALL_OPCODES = {}
ALL_OPCODES.update(EFFECT_OPCODES)
ALL_OPCODES.update(CONDITION_OPCODES)
ALL_OPCODES.update(COST_OPCODES)
OPCODE_TO_NAME = {v: k for k, v in ALL_OPCODES.items()}

# ─────────────────────────────────────────────────────────────
# Zone/type/color encoding tables
# These map JSON string values to compact u8 representations.
# ─────────────────────────────────────────────────────────────

ZONE = {
    "hand": 0,
    "hand_cards": 0,
    "stage": 1,
    "center": 2,
    "stage_center": 2,
    "left": 3,
    "left_side": 3,
    "stage_left": 3,
    "right": 4,
    "right_side": 4,
    "stage_right": 4,
    "discard": 5,
    "discard_pile": 5,
    "waitroom": 6,
    "energy": 7,
    "energy_zone": 8,
    "deck": 9,
    "deck_top": 10,
    "deck_bottom": 11,
    "success_zone": 12,
    "live_card_zone": 13,
    "success_live_zone": 14,
    "success_live_card_zone": 14,
    "energy_deck": 15,
    "empty_area": 16,
    "same_area": 17,
    "under_member": 18,
    "under": 18,
    "looked_at": 19,
    "revealed_cards": 20,
    "selected_cards": 21,
    "resolution": 22,
    "resolution_zone": 22,
    "exclusion_zone": 23,
    "deck_or_discard": 24,
}

RESOURCE = {
    "heart": 0,
    "hearts": 0,
    "heart_icon": 0,
    "blade": 1,
    "blades": 1,
    "blade_icon": 1,
    "yell": 2,
    "shield": 3,
}

HEART = {
    "heart01": 0,
    "smile": 0,
    "pink": 0,
    "heart02": 1,
    "pure": 1,
    "green": 1,
    "heart03": 2,
    "cool": 2,
    "blue": 2,
    "heart04": 3,
    "active": 3,
    "orange": 3,
    "heart05": 4,
    "natural": 4,
    "purple": 4,
    "heart06": 5,
    "elegant": 5,
    "red": 5,
}

CARD_TYPE = {
    "card": 0,
    "any": 0,
    "member_card": 1,
    "member": 1,
    "live_card": 2,
    "live": 2,
    "energy_card": 3,
    "energy": 3,
    "event_card": 4,
    "event": 4,
    "character_card": 5,
    "character": 5,
    "baton_touch_card": 6,
    "baton_touch": 6,
    "climax_card": 7,
    "climax": 7,
}

PLAYER = {"self": 0, "opponent": 1, "both": 2, "owner": 3}
COMPARE = {"=": 0, "==": 0, "!=": 1, ">": 2, ">=": 3, "<": 4, "<=": 5}
DURATION = {
    "turn_end": 0,
    "this_turn": 0,
    "live_end": 1,
    "until_end_of_live": 1,
    "permanent": 2,
    "always": 2,
    "until_used": 3,
    "next_turn": 4,
}
STATE = {
    "rest": 0,
    "rested": 0,
    "stand": 1,
    "stood": 1,
    "stand_up": 1,
    "reverse": 2,
    "reversed": 2,
    "wait": 3,
    "waited": 3,
}


def _encode(v, table, default=0):
    if v is None:
        return default
    if isinstance(v, str):
        return table.get(v.strip().lower().replace(" ", "_").replace("-", "_"), default)
    return int(v) & 0xFF


def z(v):
    return _encode(v, ZONE, 9)


def r(v):
    return _encode(v, RESOURCE)


def h(v):
    return _encode(v, HEART)


def ct(v):
    return _encode(v, CARD_TYPE)


def p(v):
    return _encode(v, PLAYER)


def op(v):
    return _encode(v, COMPARE)


def dur(v):
    return _encode(v, DURATION)


def st(v):
    return _encode(v, STATE)


def normalize_group_names(v):
    if v is None:
        return None
    if isinstance(v, list):
        return v[0] if v else None
    if isinstance(v, str):
        return v
    return None


# ─────────────────────────────────────────────────────────────
# Bytecode writer
# ─────────────────────────────────────────────────────────────


class BC:
    """Bytecode accumulator — produces an instruction stream."""

    def __init__(self):
        self.data = bytearray()

    def u8(self, v):
        self.data.append(v & 0xFF)

    def u16(self, v):
        self.data.extend(struct.pack("<H", v))

    def i8(self, v):
        self.data.extend(struct.pack("<b", v))

    def __len__(self):
        return len(self.data)

    def __bytes__(self):
        return bytes(self.data)


# ─────────────────────────────────────────────────────────────
# String table — shared across all abilities
# ─────────────────────────────────────────────────────────────


class StringTable:
    """Compact string table for group names, character names, etc."""

    def __init__(self):
        self._strings = []
        self._index = {}

    def idx(self, s):
        if s is None or s == "":
            return 0xFFFF
        s = s.strip()
        if s not in self._index:
            self._index[s] = len(self._strings)
            self._strings.append(s)
        return self._index[s]

    def get(self, i):
        return self._strings[i] if i < len(self._strings) else None

    def __len__(self):
        return len(self._strings)

    def __iter__(self):
        return iter(self._strings)


# ─────────────────────────────────────────────────────────────
# Condition compiler
# ─────────────────────────────────────────────────────────────


def compile_condition(cond, bc: BC, strs: StringTable):
    if not isinstance(cond, dict):
        return

    t = cond.get("type", "")

    if t == "compound":
        op_type = cond.get("operator", "and")
        sub = cond.get("conditions", [])
        if op_type == "or":
            bc.u8(CONDITION_OPCODES["or_condition"])
        else:
            bc.u8(CONDITION_OPCODES["and_condition"])
        for sc in sub:
            if isinstance(sc, dict):
                compile_condition(sc, bc, strs)
        bc.u8(CONDITION_OPCODES["end_condition"])
        return

    if t == "or_condition":
        bc.u8(CONDITION_OPCODES["or_condition"])
        for sc in cond.get("conditions", []):
            if isinstance(sc, dict):
                compile_condition(sc, bc, strs)
        bc.u8(CONDITION_OPCODES["end_condition"])
        return

    if t == "card_count_condition":
        bc.u8(CONDITION_OPCODES["card_count_condition"])
        bc.u8(z(cond.get("location", "stage")))
        bc.u8(op(cond.get("operator", ">=")))
        bc.u8(int(cond.get("count", 1)))
        bc.u8(ct(cond.get("card_type", "card")))
        bc.u16(strs.idx(normalize_group_names(cond.get("group_names"))))
        bc.u8(p(cond.get("target", "self")))
        return

    if t == "location_condition":
        bc.u8(CONDITION_OPCODES["location_condition"])
        bc.u8(z(cond.get("location", "stage")))
        bc.u8(ct(cond.get("card_type", "card")))
        bc.u8(1 if cond.get("exclude_self") else 0)
        bc.u8(p(cond.get("target", "self")))
        return

    if t == "comparison_condition":
        bc.u8(CONDITION_OPCODES["comparison_condition"])
        bc.u8(z(cond.get("location", "hand")))
        ct_map = {"cost": 0, "power": 1, "count": 2, "level": 3, "hand_count": 4}
        bc.u8(ct_map.get(cond.get("comparison_type", "cost"), 0))
        agg_map = {"total": 0, "average": 1, "max": 2, "min": 3}
        bc.u8(agg_map.get(cond.get("aggregate", "total"), 0))
        bc.u8(op(cond.get("operator", "=")))
        bc.u16(int(cond.get("count", cond.get("cost_total", 0))))
        return

    if t == "group_condition":
        bc.u8(CONDITION_OPCODES["group_condition"])
        bc.u16(strs.idx(normalize_group_names(cond.get("group_names"))))
        bc.u8(int(cond.get("count", 1)))
        bc.u8(op(cond.get("operator", ">=")))
        return

    if t == "movement_condition":
        bc.u8(CONDITION_OPCODES["movement_condition"])
        bc.u8(z(cond.get("location", "stage")))
        bc.u8(ct(cond.get("card_type", "card")))
        bc.u8(int(cond.get("count", 1)))
        bc.u8(op(cond.get("operator", ">=")))
        return

    if t == "temporal_condition":
        bc.u8(CONDITION_OPCODES["temporal_condition"])
        bc.u8(int(cond.get("count", 1)))
        bc.u8(op(cond.get("operator", ">=")))
        return

    if t == "appearance_condition":
        bc.u8(CONDITION_OPCODES["appearance_condition"])
        bc.u8(z(cond.get("location", "stage")))
        bc.u8(int(cond.get("count", 1)))
        return

    if t == "state_condition":
        bc.u8(CONDITION_OPCODES["state_condition"])
        bc.u8(st(cond.get("state", "rest")))
        bc.u8(op(cond.get("operator", "==")))
        bc.u8(1 if cond.get("value", True) else 0)
        return

    if t == "energy_state_condition":
        bc.u8(CONDITION_OPCODES["energy_state_condition"])
        bc.u8(op(cond.get("operator", ">=")))
        bc.u8(int(cond.get("count", 1)))
        return

    if t == "position_condition":
        bc.u8(CONDITION_OPCODES["position_condition"])
        bc.u8(z(cond.get("location", "stage")))
        return

    if t == "highest_cost_on_stage_condition":
        bc.u8(CONDITION_OPCODES["highest_cost_on_stage_condition"])
        return

    if t == "state_change_condition":
        bc.u8(CONDITION_OPCODES["state_change_condition"])
        bc.u8(st(cond.get("state_change", "rest")))
        return

    if t == "card_blade_condition":
        bc.u8(CONDITION_OPCODES["card_blade_condition"])
        bc.u8(op(cond.get("operator", ">=")))
        bc.u8(int(cond.get("count", 1)))
        return

    if t == "all_cost_comparison_condition":
        bc.u8(CONDITION_OPCODES["all_cost_comparison_condition"])
        bc.u8(op(cond.get("operator", ">=")))
        bc.u16(int(cond.get("count", 0)))
        return

    if t == "ability_filter_condition":
        bc.u8(CONDITION_OPCODES["ability_filter_condition"])
        bc.u16(strs.idx(cond.get("text", "")))
        return

    if t == "has_moved":
        bc.u8(CONDITION_OPCODES["has_moved"])
        bc.u8(z(cond.get("position", "center")))
        bc.u16(strs.idx(normalize_group_names(cond.get("group_names"))))
        return

    if t == "not_moved":
        bc.u8(CONDITION_OPCODES["not_moved"])
        return

    if t == "opponent_live_success":
        bc.u8(CONDITION_OPCODES["opponent_live_success"])
        bc.u8(1 if cond.get("no_excess_heart") else 0)
        return

    if t == "no_excess_heart":
        bc.u8(CONDITION_OPCODES["no_excess_heart"])
        return


# ─────────────────────────────────────────────────────────────
# Cost compiler
# ─────────────────────────────────────────────────────────────


def compile_cost(cost, bc: BC, strs: StringTable):
    if not cost:
        return
    if isinstance(cost, list):
        for c in cost:
            compile_cost(c, bc, strs)
        return
    if not isinstance(cost, dict):
        return

    t = cost.get("type", "move_cards")

    if t == "move_cards":
        bc.u8(COST_OPCODES["move_cards_cost"])
        bc.u8(z(cost.get("source", "stage")))
        bc.u8(z(cost.get("destination", "discard")))
        bc.u8(ct(cost.get("card_type", "member_card")))
        bc.u8(1 if cost.get("self_cost") else 0)
        bc.u8(int(cost.get("count", 1)))
        return

    if t == "tap":
        bc.u8(COST_OPCODES["tap"])
        return

    if t == "rest":
        bc.u8(COST_OPCODES["rest"])
        bc.u8(int(cost.get("count", 1)))
        return

    if t == "energy":
        bc.u8(COST_OPCODES["energy"])
        bc.u8(int(cost.get("energy", cost.get("count", 1))))
        bc.u8(0)
        return

    if t == "discard":
        bc.u8(COST_OPCODES["discard"])
        bc.u8(int(cost.get("count", 1)))
        bc.u8(ct(cost.get("card_type", "card")))
        return

    if t == "place_energy_under_member":
        bc.u8(COST_OPCODES["place_energy_under_member_cost"])
        bc.u8(int(cost.get("count", 1)))
        return

    if t == "pay_energy":
        bc.u8(COST_OPCODES["pay_energy"])
        bc.u8(int(cost.get("energy", cost.get("count", 1))))
        bc.u8(1 if cost.get("optional") else 0)
        return

    if t == "change_state":
        bc.u8(COST_OPCODES["change_state_cost"])
        bc.u8(st(cost.get("state_change", "rest")))
        bc.u8(1 if cost.get("optional") else 0)
        bc.u8(1 if cost.get("self_cost") else 0)
        return

    if t == "sequential_cost":
        costs = cost.get("costs", [])
        bc.u8(COST_OPCODES["sequential_cost"])
        bc.u8(len(costs))
        for sc in costs:
            if isinstance(sc, dict):
                compile_cost(sc, bc, strs)
        return

    if t == "reveal":
        bc.u8(COST_OPCODES["reveal"])
        bc.u8(z(cost.get("source", "hand")))
        bc.u8(ct(cost.get("card_type", "card")))
        bc.u8(int(cost.get("count", 1)))
        return

    if t == "choice_condition":
        options = cost.get("options", [])
        bc.u8(COST_OPCODES["choice_condition"])
        bc.u8(len(options))
        for opt in options:
            if isinstance(opt, dict):
                compile_cost(opt, bc, strs)
        return


# ─────────────────────────────────────────────────────────────
# Effect compiler
# ─────────────────────────────────────────────────────────────


def compile_effect(eff, bc: BC, strs: StringTable, is_sub=False):
    if not isinstance(eff, dict):
        return

    action = eff.get("action", "")

    # ── Wrapper effects (conditions around other effects) ──
    cond = eff.get("condition")
    if isinstance(cond, dict) and not is_sub:
        bc.u8(EFFECT_OPCODES["conditional"])
        # Emit condition length prefix so the VM can skip conditions
        cond_bytes = BC()
        compile_condition(cond, cond_bytes, strs)
        bc.u16(len(cond_bytes))
        bc.data.extend(cond_bytes.data)
        body = BC()
        compile_effect(eff, body, strs, is_sub=True)
        bc.u16(len(body))
        bc.data.extend(body.data)
        alt = eff.get("alternative")
        if isinstance(alt, dict):
            alt_body = BC()
            compile_effect(alt, alt_body, strs, is_sub=True)
            bc.u16(len(alt_body))
            bc.data.extend(alt_body.data)
        else:
            bc.u16(0)
        return

    if action == "conditional_alternative":
        cond = eff.get("condition")
        if isinstance(cond, dict):
            bc.u8(EFFECT_OPCODES["conditional_alternative"])
            compile_condition(cond, bc, strs)
            primary = eff.get("primary_effect") or eff.get("effect")
            if isinstance(primary, dict):
                body = BC()
                compile_effect(primary, body, strs, is_sub=True)
                bc.u16(len(body))
                bc.data.extend(body.data)
            else:
                bc.u16(0)
            alt = eff.get("alternative_effect")
            if isinstance(alt, dict):
                alt_body = BC()
                compile_effect(alt, alt_body, strs, is_sub=True)
                bc.u16(len(alt_body))
                bc.data.extend(alt_body.data)
            else:
                bc.u16(0)
        return

    if action == "conditional_on_optional":
        bc.u8(EFFECT_OPCODES["conditional_on_optional"])
        bc.u8(1 if eff.get("optional") else 0)
        return

    if action == "conditional_on_result":
        bc.u8(EFFECT_OPCODES["conditional_on_result"])
        return

    # ── Sequential (list of sub-effects) ──
    if action == "sequential":
        actions = eff.get("actions", [])
        bc.u8(EFFECT_OPCODES["sequential"])
        bc.u8(len(actions))
        for act in actions:
            compile_effect(act, bc, strs, is_sub=True)
        return

    # ── Look and select ──
    if action == "look_and_select":
        look = eff.get("look_action", {})
        select = eff.get("select_action", {})
        bc.u8(EFFECT_OPCODES["look_at"])
        bc.u8(int(look.get("count", 1)))
        bc.u8(z(look.get("source", "deck_top")))
        bc.u8(p(look.get("target", "self")))
        bc.u8(EFFECT_OPCODES["select_cards"])
        bc.u8(int(select.get("count", 1)))
        bc.u8(z(select.get("destination", "hand")))
        bc.u8(1 if select.get("discard_remaining") else 0)
        return

    # ── Simple effects ──
    if action == "draw_card":
        bc.u8(EFFECT_OPCODES["draw_card"])
        bc.u8(int(eff.get("count", 1)))
        bc.u8(z(eff.get("source", "deck")))
        return

    if action == "move_cards":
        bc.u8(EFFECT_OPCODES["move_cards"])
        bc.u8(int(eff.get("count", 1)))
        bc.u8(z(eff.get("source", "hand")))
        bc.u8(z(eff.get("destination", "hand")))
        bc.u8(ct(eff.get("card_type", "card")))
        bc.u8(p(eff.get("target", "self")))
        return

    if action == "gain_resource":
        bc.u8(EFFECT_OPCODES["gain_resource"])
        bc.u8(r(eff.get("resource", "heart")))
        bc.u8(int(eff.get("count", 1)))
        heart = (
            eff.get("heart_color") or (eff.get("heart_colors") or [None])[0]
            if isinstance(eff.get("heart_colors"), list) and eff.get("heart_colors")
            else None
        )
        bc.u8(h(heart) if heart else 0)
        bc.u8(dur(eff.get("duration", "turn_end")))
        bc.u16(
            strs.idx(
                str(eff.get("characters", [""])[0])
                if isinstance(eff.get("characters"), list) and eff.get("characters")
                else None
            )
        )
        return

    if action == "modify_score":
        bc.u8(EFFECT_OPCODES["modify_score"])
        bc.i8(int(eff.get("value", eff.get("count", 0))))
        bc.u8(1 if eff.get("per_unit") else 0)
        bc.u8(p(eff.get("target", "self")))
        return

    if action == "change_state":
        bc.u8(EFFECT_OPCODES["change_state"])
        bc.u8(st(eff.get("state_change", "rest")))
        bc.u8(p(eff.get("target", "self")))
        return

    if action == "position_change":
        bc.u8(EFFECT_OPCODES["position_change"])
        bc.u8(p(eff.get("target", "self")))
        return

    if action == "modify_required_hearts":
        bc.u8(EFFECT_OPCODES["modify_required_hearts"])
        bc.i8(int(eff.get("value", eff.get("count", 0))))
        bc.u8(p(eff.get("target", "self")))
        return

    if action == "modify_required_hearts_global":
        bc.u8(EFFECT_OPCODES["modify_required_hearts_global"])
        bc.i8(int(eff.get("value", eff.get("count", 0))))
        return

    if action == "modify_cost":
        bc.u8(EFFECT_OPCODES["modify_cost"])
        bc.i8(int(eff.get("value", eff.get("count", 0))))
        bc.u8(p(eff.get("target", "self")))
        return

    if action in ("set_blade_type",):
        bc.u8(EFFECT_OPCODES["set_blade_type"])
        bc.u8(0)
        return

    if action == "set_blade_count":
        bc.u8(EFFECT_OPCODES["set_blade_count"])
        bc.u8(int(eff.get("value", eff.get("count", 0))))
        return

    if action == "set_heart_type":
        bc.u8(EFFECT_OPCODES["set_heart_type"])
        bc.u8(h(eff.get("value", "smile")))
        return

    if action == "gain_ability":
        bc.u8(EFFECT_OPCODES["gain_ability"])
        bc.u16(int(eff.get("value", 0)))
        bc.u8(dur(eff.get("duration", "turn_end")))
        return

    if action == "gain_ability_from_source":
        bc.u8(EFFECT_OPCODES["gain_ability_from_source"])
        bc.u16(int(eff.get("value", 0)))
        return

    if action == "restriction":
        bc.u8(EFFECT_OPCODES["restriction"])
        return

    if action == "choose_target_player":
        bc.u8(EFFECT_OPCODES["choose_target_player"])
        bc.u8(p(eff.get("target", "self")))
        return

    if action == "place_energy_under_member":
        bc.u8(EFFECT_OPCODES["place_energy_under_member"])
        bc.u8(int(eff.get("count", 1)))
        return

    if action == "draw_until_count":
        bc.u8(EFFECT_OPCODES["draw_until_count"])
        bc.u8(int(eff.get("count", 1)))
        bc.u8(z(eff.get("source", "deck")))
        return

    if action == "modify_yell_count":
        bc.u8(EFFECT_OPCODES["modify_yell_count"])
        bc.i8(int(eff.get("value", eff.get("count", 0))))
        return

    if action == "invalidate_ability":
        bc.u8(EFFECT_OPCODES["invalidate_ability"])
        return

    if action == "suppress_ability_trigger":
        bc.u8(EFFECT_OPCODES["suppress_ability_trigger"])
        return

    if action == "activate_ability":
        bc.u8(EFFECT_OPCODES["activate_ability"])
        return

    if action == "play_baton_touch":
        bc.u8(EFFECT_OPCODES["play_baton_touch"])
        return

    if action == "set_card_identity":
        bc.u8(EFFECT_OPCODES["set_card_identity"])
        bc.u16(strs.idx(eff.get("value", "")))
        return

    if action == "choice":
        return


# ─────────────────────────────────────────────────────────────
# Main compilation
# ─────────────────────────────────────────────────────────────


def compile_all(abilities):
    strs = StringTable()
    offsets = []
    bytecode = bytearray()
    disasm = []
    debug_names = []

    for i, entry in enumerate(abilities):
        offsets.append(len(bytecode))
        eff = entry.get("effect", {})
        cost = entry.get("cost")

        w = BC()
        if cost:
            compile_cost(cost, w, strs)
        if isinstance(eff, dict):
            compile_effect(eff, w, strs)
        bytecode.extend(w.data)

        name = entry.get("triggerless_text", "") or entry.get("full_text", "")
        name = (
            name.replace("{{", "")
            .replace("}}", " ")
            .replace("|", ": ")
            .replace("  ", " ")
            .strip()[:80]
        )
        debug_names.append(name)
        disasm.append(disassemble_one(entry, len(w.data)))

    offsets.append(len(bytecode))
    return bytes(bytecode), offsets, disasm, debug_names, strs


def disassemble_one(entry, byte_len):
    eff = entry.get("effect", {})
    parts = []
    if isinstance(eff, dict):
        parts.append(eff.get("action", "?"))
        for k in ("count", "source", "destination", "card_type", "resource", "target"):
            if eff.get(k):
                parts.append(f"{k}={eff[k]}")
        if isinstance(eff.get("action"), list):
            parts.append(f"actions={len(eff['action'])}")
        if eff.get("action") == "sequential":
            parts.append(f"sub={len(eff.get('actions', []))}")
        if isinstance(eff.get("condition"), dict):
            parts.append(f"cond={eff['condition'].get('type', '?')}")
    cost = entry.get("cost")
    if cost:
        if isinstance(cost, dict):
            parts.append(f"cost={cost.get('type', '?')}")
        elif isinstance(cost, list):
            parts.append(f"cost=[{len(cost)}]")
    parts.append(f"({byte_len}B)")
    return ", ".join(parts)


# ─────────────────────────────────────────────────────────────
# Rust code generation
# ─────────────────────────────────────────────────────────────


def rust_name(json_name):
    return "".join(word.capitalize() for word in json_name.split("_"))


def generate_rust(bytecode, offsets, disasm, names, strs, build_dir):
    build_dir.mkdir(parents=True, exist_ok=True)

    all_ops = sorted(ALL_OPCODES.items(), key=lambda x: x[1])
    enum_variants = []
    variant_to_json = {}
    for json_name, code in all_ops:
        variant = rust_name(json_name)
        enum_variants.append(f"    {variant} = {code},")
        variant_to_json[variant] = json_name

    hex_chunks = []
    for i in range(0, len(bytecode), 24):
        chunk = bytecode[i : i + 24]
        hex_chunks.append("    " + ", ".join(f"0x{b:02x}" for b in chunk) + ",")

    offset_list = ", ".join(str(o) for o in offsets)

    disasm_lines = []
    for i, (n, d) in enumerate(zip(names, disasm)):
        disasm_lines.append(f"    /// [{i:03d}] {n}")
        disasm_lines.append(f"    /// {d}")

    tryfrom_arms = []
    for json_name, code in all_ops:
        variant = rust_name(json_name)
        tryfrom_arms.append(f"            {code} => Ok(Self::{variant}),")

    string_table = ", ".join(f'"{s}"' for s in strs)

    rust_source = f"""// Auto-generated by compile_abilities.py — DO NOT EDIT
// Source: cards/abilities.json
// Built: {len(bytecode)} bytes of bytecode, {len(offsets) - 1} unique abilities, {len(strs)} string table entries

pub const NUM_ABILITIES: usize = {len(offsets) - 1};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {{
{chr(10).join(enum_variants)}
}}

impl TryFrom<u8> for Opcode {{
    type Error = UnknownOpcode;

    fn try_from(v: u8) -> Result<Self, Self::Error> {{
        match v {{
{chr(10).join(tryfrom_arms)}
            _ => Err(UnknownOpcode(v)),
        }}
    }}
}}

impl Opcode {{
    pub const fn json_name(self) -> &'static str {{
        match self {{
{chr(10).join(f'            Self::{rust_name(jn)} => "{jn}",' for jn, _ in all_ops)}
        }}
    }}
}}

#[derive(Debug)]
pub struct UnknownOpcode(pub u8);

impl core::fmt::Display for UnknownOpcode {{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{
        write!(f, "unknown opcode: 0x{{:02x}}", self.0)
    }}
}}

pub const BYTECODE: &[u8] = &[
{chr(10).join(hex_chunks)}
];

pub const OFFSETS: &[u16] = &[{offset_list}];

pub const STRINGS: &[&str] = &[{string_table}];

#[cfg(debug_assertions)]
pub const DEBUG_NAMES: &[&str] = &[
{chr(10).join(f'    r#"{e}"#,' for e in names)}
];

#[cfg(debug_assertions)]
pub const DEBUG_DISASM: &[&str] = &[
{chr(10).join(f'    r#"{e}"#,' for e in disasm)}
];
"""
    (build_dir / "abilities_gen.rs").write_text(rust_source, encoding="utf-8")


# ─────────────────────────────────────────────────────────────
# VM decoder generation — maps opcodes to decode functions
# The generated code is included directly into vm.rs, so it
# shares the same scope (read_u8, etc.).
# ─────────────────────────────────────────────────────────────

# ─────────────────────────────────────────────────────────────
# EffectKind variant field lists — ALL fields, for codegen
# Fields not provided by bytecode get Default::default()
# Fields with Box<Vec<String>> need Box::default()
# ─────────────────────────────────────────────────────────────

# Parse card.rs for Condition variant fields (used for default constructors)
_CONDITION_FIELDS_CACHE = None


def _get_condition_fields():
    global _CONDITION_FIELDS_CACHE
    if _CONDITION_FIELDS_CACHE is not None:
        return _CONDITION_FIELDS_CACHE
    import re

    root = Path(__file__).parent.parent / "engine/src/core"
    with open(root / "card.rs", encoding="utf-8") as f:
        content = f.read()
    idx = content.find("pub enum Condition")
    end = content.find("};", idx) + 2
    block = content[idx:end]
    variants = {}
    current_var = None
    for line in block.split("\n"):
        m = re.match(r"    (\w+) \{", line)
        if m:
            current_var = m.group(1)
            variants[current_var] = []
            continue
        m = re.match(r"        (\w+):", line)
        if m and current_var:
            variants[current_var].append(m.group(1))
        if line.strip() == "}," and current_var:
            current_var = None
    _CONDITION_FIELDS_CACHE = variants
    return variants


# For each EffectKind variant, list ALL its field names.
# Fields that are Box<Vec<String>> are marked with "boxvec".
# This is used by the vm_gen.rs generator to produce constructors.
EFFECTKIND_ALL_FIELDS = {
    "DrawCards": [
        "source",
        "target",
        "destination",
        "target_count",
        "card_type",
        "dynamic_count",
        "card_names",
        "location",
        "exclude_self",
        "per_unit",
        "per_unit_count",
        "per_unit_type",
        "per_unit_heart_colors",
        "per_unit_location",
        "position",
        "state",
        "heart_colors",
        "trigger_type",
        "original_value",
        "action_by",
    ],
    "MoveCards": [
        "source",
        "target",
        "destination",
        "count",
        "card_type",
        "target_count",
        "characters",
        "exclude_characters",
        "group_names",
        "exclude_group_names",
        "cost_limit",
        "cost_limit_operator",
        "cost_limit_min",
        "cost_limit_max",
        "card_names",
        "placement_order",
        "shuffle",
        "any_number",
        "discard_remaining",
        "multiple_targets",
        "exclude_self",
        "exclude_selected",
        "exclude_by_name_source",
        "name_constraint",
        "name_constraint_source",
        "ability_filter",
        "ability_filter_triggers",
        "or_ability_filters",
        "card_property",
        "original_value",
        "source_position",
        "exclude_position",
        "allow_occupied_stage",
        "target_from_selection",
        "group_reference",
        "cost_from_revealed",
        "self_target",
        "per_group",
        "per_group_count",
        "state",
        "negation",
        "location",
        "activation_position",
        "exclude_heart_colors",
        "filter_targets_by_heart_colors",
        "cost_total",
        "cost_total_operator",
        "need_heart_total",
        "need_heart_operator",
        "need_heart_color",
        "distinct",
        "state_change",
        "self_cost",
        "dynamic_count",
        "or_card_types",
        "position",
        "cost_reference",
        "cost_offset",
        "all",
        "energy_count",
        "heart_colors",
        "baton_touch_trigger",
        "target_member",
        "same_unit_name",
        "action_by",
        "activation_condition_parsed",
        "quoted_text",
    ],
    "SelectTarget": [
        "source",
        "target",
        "destination",
        "target_count",
        "card_type",
        "characters",
        "exclude_characters",
        "group_names",
        "exclude_group_names",
        "cost_limit",
        "cost_limit_operator",
        "cost_limit_min",
        "cost_limit_max",
        "card_names",
        "exclude_self",
        "exclude_selected",
        "placement_order",
        "distinct",
        "name_constraint",
        "name_constraint_source",
        "ability_filter",
        "ability_filter_triggers",
        "or_ability_filters",
        "card_property",
        "original_value",
        "negation",
        "self_target",
        "per_unit",
        "per_unit_count",
        "per_unit_type",
        "per_unit_heart_colors",
        "per_unit_location",
        "optional",
        "location",
        "state",
        "activation_position",
        "group_reference",
        "multiple_targets",
        "question",
        "answers",
        "choice_maker",
        "choice_type",
        "choice_options",
        "filter_targets_by_heart_colors",
        "heart_colors",
        "cost_total",
        "cost_total_operator",
        "or_card_types",
        "action_by",
        "require_all_heart_colors",
        "heart_color_count",
        "options",
        "per_group",
        "per_group_count",
        "reveal",
        "any_number",
        "discard_remaining",
    ],
    "LookReveal": [
        "source",
        "target",
        "destination",
        "card_type",
        "characters",
        "exclude_characters",
        "group_names",
        "exclude_group_names",
        "cost_limit",
        "cost_limit_operator",
        "cost_limit_min",
        "cost_limit_max",
        "card_names",
        "exclude_self",
        "distinct",
        "name_constraint",
        "name_constraint_source",
        "ability_filter",
        "ability_filter_triggers",
        "or_ability_filters",
        "card_property",
        "original_value",
        "negation",
        "self_target",
        "per_unit",
        "per_unit_count",
        "per_unit_type",
        "per_unit_heart_colors",
        "per_unit_location",
        "location",
        "group_reference",
        "dynamic_count",
        "heart_colors",
        "reveal",
        "filter_targets_by_heart_colors",
        "activation_position",
        "state",
        "optional",
        "blind",
        "is_reveal",
        "picker",
        "multiple_targets",
        "options",
        "resource_on_select",
        "require_all_heart_colors",
        "heart_color_count",
    ],
    "ModifyScore": [
        "source",
        "target",
        "destination",
        "operation",
        "value",
        "duration",
        "card_type",
        "group_names",
        "per_unit",
        "per_unit_count",
        "per_unit_type",
        "per_unit_location",
        "per_unit_heart_colors",
        "location",
        "effect_constraint",
        "self_target",
        "heart_colors",
        "exclude_self",
        "target_count",
        "repeat_limit",
        "filter_targets_by_heart_colors",
        "cost_total",
        "cost_total_operator",
        "distinct",
        "position",
        "activation_position",
        "card_names",
        "card_property",
        "state",
        "negation",
        "max_repeats",
        "need_heart_operator",
        "need_heart_total",
    ],
    "ModifyHearts": [
        "operation",
        "value",
        "duration",
        "heart_colors",
        "group_names",
        "per_unit",
        "per_unit_count",
        "per_unit_heart_colors",
        "location",
        "timing_condition",
        "original_value",
        "original_count",
        "original_operator",
        "exclude_self",
        "self_target",
        "exclude_heart_colors",
        "repeat_limit",
        "card_type",
        "target_count",
        "filter_targets_by_heart_colors",
        "cost_total",
        "cost_total_operator",
        "group_reference",
        "negation",
        "replace_all",
        "position",
        "all",
        "per_unit_type",
        "distinct",
    ],
    "GainResource": [
        "resource",
        "heart_colors",
        "heart_colors_from_selected_card",
        "sign",
        "operation",
        "value",
        "energy_count",
        "dynamic_count",
        "optional",
        "duration",
        "position",
        "any_number",
        "per_unit",
        "per_unit_count",
        "per_unit_type",
        "per_unit_heart_colors",
        "per_unit_location",
        "location",
        "group_names",
        "card_type",
        "cost_limit",
        "cost_limit_operator",
        "characters",
        "exclude_characters",
        "exclude_group_names",
        "self_target",
        "target_from_selection",
        "card_property",
        "original_value",
        "negation",
        "filter_targets_by_heart_colors",
        "activation_position",
        "state",
        "heart_type",
        "target_count",
        "all",
        "same_name",
        "exclude_self",
        "group_reference",
        "trigger_type",
        "distinct",
        "heart_color",
        "action_by",
        "activation_condition_parsed",
        "multiple_targets",
        "repeat_limit",
        "timing_condition",
        "require_all_heart_colors",
        "heart_color_count",
    ],
    "ChangeState": [
        "source",
        "target",
        "destination",
        "state_change",
        "card_type",
        "cost_limit",
        "cost_limit_operator",
        "cost_from_revealed",
        "optional",
        "self_cost",
        "characters",
        "exclude_characters",
        "group_names",
        "exclude_group_names",
        "blade_limit",
        "blade_limit_operator",
        "per_unit",
        "per_unit_count",
        "per_unit_type",
        "per_unit_heart_colors",
        "per_unit_location",
        "location",
        "distinct",
        "exclude_self",
        "self_target",
        "identities",
        "all_regions",
        "card_names",
        "negation",
        "cost_total",
        "cost_total_operator",
        "card_property",
        "original_value",
        "name_constraint",
        "name_constraint_source",
        "filter_targets_by_heart_colors",
        "group_reference",
        "activation_position",
        "ability_filter",
        "ability_filter_triggers",
        "or_ability_filters",
        "exclude_heart_colors",
        "heart_colors",
        "all",
        "position",
        "state",
        "action_by",
        "activation_condition_parsed",
    ],
    "AbilityOp": [
        "source",
        "target",
        "destination",
        "ability_gain",
        "ability_gain_trigger",
        "gained_effect",
        "ability_text",
        "target_trigger",
        "source_card",
        "suppressed_trigger",
        "card_type",
        "group_names",
        "exclude_group_names",
        "characters",
        "exclude_characters",
        "cost_limit",
        "cost_limit_operator",
        "location",
        "trigger_filter",
        "trigger_type",
        "duration",
        "self_target",
        "exclude_self",
        "effect_type",
        "use_limit",
        "triggers",
        "activation_condition_parsed",
        "option",
        "all",
        "activation_position",
        "heart_colors",
        "dynamic_count",
    ],
    "CompoundEffect": [
        "source",
        "target",
        "destination",
        "repeat_limit",
        "options",
        "choice_type",
        "choice_options",
        "question",
        "answers",
        "choice_maker",
        "alternative_effect",
        "optional",
        "target_count",
        "group_names",
        "heart_colors",
        "exclude_self",
        "duration",
        "position",
        "all",
        "activation_position",
        "card_type",
        "trigger_type",
        "activation_condition_parsed",
        "original_value",
        "shuffle",
        "distinct",
        "group_reference",
        "per_unit",
        "per_unit_count",
        "per_unit_type",
        "alternative_count_type",
    ],
    "RestrictionOp": [
        "restriction_type",
        "restricted_destination",
        "delayed",
        "timing",
        "treat_as",
        "timing_condition",
        "phase",
        "non_stackable",
        "operation",
        "card_type",
        "location",
        "effect_type",
        "replaces_event",
        "choice_based",
        "trigger_type",
        "trigger_filter",
        "duration",
        "self_target",
        "exclude_self",
        "group_names",
        "exclude_group_names",
        "characters",
        "exclude_characters",
    ],
    "PositionOp": [
        "source",
        "target",
        "destination",
        "position",
        "target_member",
        "source_position",
        "exclude_position",
        "allow_occupied_stage",
        "optional",
        "card_type",
        "group_names",
        "exclude_group_names",
        "characters",
        "exclude_characters",
        "cost_limit",
        "cost_limit_operator",
        "energy_count",
        "dynamic_count",
        "any_number",
        "cost_from_revealed",
        "exclude_self",
        "multiple_targets",
        "self_target",
        "state",
        "activation_position",
        "group_reference",
    ],
    "MiscOp": [
        "source",
        "target",
        "destination",
        "operation",
        "value",
        "card_type",
        "group_names",
        "exclude_group_names",
        "characters",
        "exclude_characters",
        "cost_limit",
        "cost_limit_operator",
        "location",
        "duration",
        "heart_colors",
        "heart_type",
        "heart_selection",
        "blade_type",
        "self_target",
        "exclude_self",
        "choice",
        "lose_blade_hearts",
        "dynamic_count",
        "per_unit",
        "per_unit_count",
        "per_unit_type",
        "per_unit_heart_colors",
        "per_unit_location",
        "repeat_limit",
        "identities",
        "all_regions",
        "timing",
        "treat_as",
        "effect_constraint",
        "original_value",
        "original_count",
        "original_operator",
        "original_cost",
        "blade_limit",
        "blade_limit_operator",
        "negation",
        "activation_position",
        "target_count",
        "group_reference",
        "parenthetical",
        "quoted_text",
        "same_unit_name",
        "alternative_count_type",
        "per_group",
        "per_group_count",
        "resource_icon_count",
        "cost_total",
        "cost_total_operator",
        "cost_reference",
        "cost_offset",
        "blind",
        "picker",
        "all",
        "sign",
        "heart_color_count",
        "require_all_heart_colors",
        "energy_count",
        "placement_order",
        "ref_value",
        "ref_offset",
        "id",
        "card_names",
        "character_effects",
        "or_card_types",
        "options",
        "position",
        "ability_filter",
    ],
    "CustomOp": [
        "action_by",
        "opponent_action",
        "effect_type",
        "replaces_event",
        "choice_based",
        "card_type",
        "group_names",
        "exclude_group_names",
        "characters",
        "exclude_characters",
        "identities",
        "all_regions",
        "question",
        "answers",
        "choice_maker",
        "options",
        "location",
        "duration",
        "self_target",
        "exclude_self",
        "original_value",
        "timing",
        "treat_as",
        "trigger_type",
        "trigger_filter",
        "activation_condition_parsed",
        "use_limit",
        "triggers",
    ],
}

# For each effect opcode, describe how to decode it.
# Format: (EffectKind_variant, action_string, [(optype, var_name, ek_field_name)])
# opcode -> (variant, action_str, [(operand_type, read_variable, effectkind_field)])
# All non-specified EffectKind fields get Default::default()
# Operand types: "u8", "i8", "u16", "zone", "player", "bool", "card_type", "resource", "heart", "state", "duration", "operator", "str_idx"
EFFECT_DECODE_MAP = {
    "draw_card": (
        "DrawCards",
        "draw_card",
        [
            ("u8", "target_count", "target_count"),
            ("zone", "source", "source"),
        ],
    ),
    "move_cards": (
        "MoveCards",
        "move_cards",
        [
            ("u8", "count", "count"),
            ("zone", "source", "source"),
            ("zone", "destination", "destination"),
        ],
    ),
    "gain_resource": (
        "GainResource",
        "gain_resource",
        [
            ("u8", "count", "value"),
        ],
    ),
    "modify_score": (
        "ModifyScore",
        "modify_score",
        [
            ("i8", "value", "value"),
            ("bool", "per_unit", "per_unit"),
            ("player", "target", "target"),
        ],
    ),
    "change_state": (
        "ChangeState",
        "change_state",
        [
            ("state", "state_change", "state_change"),
            ("player", "target", "target"),
        ],
    ),
    "position_change": (
        "PositionOp",
        "position_change",
        [
            ("player", "target", "target"),
        ],
    ),
    "modify_required_hearts": (
        "ModifyHearts",
        "modify_required_hearts",
        [
            ("i8", "value", "value"),
        ],
    ),
    "modify_cost": (
        "CustomOp",
        "modify_cost",
        [
            ("i8", "value", "_unused"),
            ("player", "target", "_unused"),
        ],
    ),
    "set_blade_type": ("CustomOp", "set_blade_type", []),
    "set_blade_count": (
        "MiscOp",
        "set_blade_count",
        [
            ("u8", "value", "value"),
        ],
    ),
    "set_heart_type": (
        "MiscOp",
        "set_heart_type",
        [
            ("heart", "value", "heart_type"),
        ],
    ),
    "gain_ability": ("AbilityOp", "gain_ability", []),
    "restriction": ("RestrictionOp", "restriction", []),
    "choose_target_player": (
        "SelectTarget",
        "choose_target_player",
        [
            ("player", "target", "target"),
        ],
    ),
    "place_energy_under_member": (
        "MoveCards",
        "place_energy_under_member",
        [
            ("u8", "count", "count"),
        ],
    ),
    "draw_until_count": (
        "DrawCards",
        "draw_until_count",
        [
            ("u8", "target_count", "target_count"),
            ("zone", "source", "source"),
        ],
    ),
    "modify_yell_count": (
        "ModifyScore",
        "modify_yell_count",
        [
            ("i8", "value", "value"),
        ],
    ),
    "invalidate_ability": ("AbilityOp", "invalidate_ability", []),
    "suppress_ability_trigger": ("AbilityOp", "suppress_ability_trigger", []),
    "activate_ability": ("AbilityOp", "activate_ability", []),
    "play_baton_touch": ("MoveCards", "play_baton_touch", []),
    "modify_required_hearts_global": (
        "ModifyHearts",
        "modify_required_hearts_global",
        [
            ("i8", "value", "value"),
        ],
    ),
    "gain_ability_from_source": ("AbilityOp", "gain_ability_from_source", []),
    "set_card_identity": ("ChangeState", "set_card_identity", []),
}


def _assign_expr(optype, src_var, field_name):
    """Generate Rust expression to assign a decoded value to an EffectKind field."""
    conv = {
        "u8": f"Some({src_var} as u32)",
        "i8": f"Some({src_var} as u32)",
        "u16": f"Some({src_var} as u32)",
        "bool": f"Some({src_var})",
        "zone": f"Some({src_var}.into())",
        "player": f"Some({src_var}.into())",
        "card_type": f"Some({src_var}.into())",
        "resource": f"Some({src_var}.into())",
        "heart": f"Some({src_var}.into())",
        "state": f"Some({src_var}.into())",
        "duration": f"Some({src_var}.into())",
        "operator": f"Some({src_var}.into())",
        "str_idx": f"{src_var}.map(|s| s.into())",
    }
    return conv.get(optype, f"Some({src_var})")


def _default_field_val(field_name):
    """Generate default value for an EffectKind field.
    Default::default() works for both Option<T> and Box<Vec<String>>."""
    return "Default::default()"


def generate_vm_rs(build_dir):
    build_dir.mkdir(parents=True, exist_ok=True)
    lines = [
        "// Auto-generated by compile_abilities.py — DO NOT EDIT",
        "",
    ]

    # ── Default constructor for each Condition variant ──
    cond_fields = _get_condition_fields()
    for variant in sorted(cond_fields.keys()):
        all_fields = cond_fields[variant]
        fn_name = f"default_condition_{variant[0].lower()}{variant[1:]}"
        lines.append(f"fn {fn_name}() -> Condition {{")
        lines.append(f"    Condition::{variant} {{")
        for f in all_fields:
            lines.append(f"        {f}: Default::default(),")
        lines.append("    }")
        lines.append("}")
        lines.append("")

    # ── Default constructor for each EffectKind variant ──
    variant_used = set()
    for json_name, (variant, action, fields) in EFFECT_DECODE_MAP.items():
        variant_used.add(variant)

    for variant in sorted(variant_used):
        all_fields = EFFECTKIND_ALL_FIELDS.get(variant, [])
        fn_name = f"default_{variant[0].lower()}{variant[1:]}"
        lines.append(f"fn {fn_name}() -> EffectKind {{")
        lines.append(f"    EffectKind::{variant} {{")
        for f in all_fields:
            lines.append(f"        {f}: {_default_field_val(f)},")
        lines.append("    }")
        lines.append("}")
        lines.append("")

    # ── decode_effect_kind ──
    lines.append(
        "fn decode_effect_kind(op: Opcode, cursor: &mut &[u8]) -> Option<Box<EffectKind>> {"
    )
    lines.append("    match op {")
    for json_name, (variant, action, ek_fields) in sorted(EFFECT_DECODE_MAP.items()):
        op = rust_name(json_name)
        fn_name = f"default_{variant[0].lower()}{variant[1:]}"
        lines.append(f"        Opcode::{op} => {{")
        for optype, var_name, ek_field in ek_fields:
            if not var_name.startswith("_"):
                lines.append("            " + _read_expr(optype, var_name) + ";")
        lines.append(f"            let mut ek = {fn_name}();")
        field_assigns = []
        for optype, var_name, ek_field in ek_fields:
            if not ek_field.startswith("_"):
                field_assigns.append(
                    (ek_field, _assign_expr(optype, var_name, ek_field))
                )
        # Use if let to set variant-specific fields
        # Use `field: ref mut alias` to avoid shadowing local variables
        fields_list = ", ".join(f"{f}: ref mut _bc_{f}" for f, _ in field_assigns)
        if field_assigns:
            lines.append(
                f"            if let EffectKind::{variant} {{ {fields_list}, .. }} = &mut ek {{"
            )
            for f, expr in field_assigns:
                lines.append(f"                *_bc_{f} = {expr};")
            lines.append("            }")
        lines.append("            Some(Box::new(ek))")
        lines.append("        }")
    lines.append("        _ => None,")
    lines.append("    }")
    lines.append("}")
    lines.append("")

    # ── decode_simple_effect (advance cursor, return action string) ──
    lines.append(
        "fn decode_simple_effect(op: Opcode, cursor: &mut &[u8]) -> &'static str {"
    )
    lines.append("    match op {")
    for json_name, (variant, action, ek_fields) in sorted(EFFECT_DECODE_MAP.items()):
        op = rust_name(json_name)
        lines.append(f"        Opcode::{op} => {{")
        for optype, var_name, ek_field in ek_fields:
            # In decode_simple_effect, we advance cursor but don't use values
            lines.append("            " + _read_expr(optype, "_" + var_name) + ";")
        lines.append(f'            "{action}"')
        lines.append("        }")
    lines.append('        _ => "",')
    lines.append("    }")
    lines.append("}")
    lines.append("")

    (build_dir / "vm_gen.rs").write_text("\n".join(lines), encoding="utf-8")


def _read_expr(optype, name):
    exprs = {
        "u8": f"let {name} = read_u8(cursor)",
        "i8": f"let {name} = read_i8(cursor)",
        "u16": f"let {name} = read_u16(cursor)",
        "bool": f"let {name} = read_u8(cursor) != 0",
        "zone": f"let {name} = decode_zone(read_u8(cursor))",
        "player": f"let {name} = decode_player(read_u8(cursor))",
        "card_type": f"let {name} = decode_card_type(read_u8(cursor))",
        "resource": f"let {name} = decode_resource(read_u8(cursor))",
        "heart": f"let {name} = decode_heart(read_u8(cursor))",
        "state": f"let {name} = decode_state(read_u8(cursor))",
        "duration": f"let {name} = decode_duration(read_u8(cursor))",
        "operator": f"let {name} = decode_operator(read_u8(cursor))",
        "str_idx": f"let {name} = read_str(cursor)",
    }
    return exprs.get(optype, f"let {name} = read_u8(cursor)")


def generate_disassembly(bytecode, offsets, names, disasm, build_dir):
    lines = []
    for i in range(len(offsets) - 1):
        start = offsets[i]
        end = offsets[i + 1]
        lines.append(f"#{i:03d} [{start:04x}-{end:04x}] ({end - start}B): {names[i]}")
        lines.append(f"    {disasm[i]}")
        lines.append("")
    (build_dir / "abilities_disasm.txt").write_text("\n".join(lines), encoding="utf-8")


# ─────────────────────────────────────────────────────────────
# Entry point
# ─────────────────────────────────────────────────────────────


def main():
    root = Path(__file__).parent
    with open(root / "abilities.json", encoding="utf-8") as f:
        data = json.load(f)

    abilities = data["unique_abilities"]
    print(f"Compiling {len(abilities)} unique abilities...")

    bytecode, offsets, disasm, names, strs = compile_all(abilities)

    build_dir = root / "build"
    build_dir.mkdir(parents=True, exist_ok=True)

    (build_dir / "abilities.bin").write_bytes(bytecode)
    print(
        f"  abilities.bin          {len(bytecode):>8} bytes  ({len(bytecode) / 1024:.1f}KB)"
    )

    generate_rust(bytecode, offsets, disasm, names, strs, build_dir)
    print(f"  abilities_gen.rs       generated")

    generate_vm_rs(build_dir)
    print(f"  vm_gen.rs              generated")

    generate_disassembly(bytecode, offsets, names, disasm, build_dir)
    print(f"  abilities_disasm.txt   generated")
    print(f"  String table entries:  {len(strs)}")
    if bytecode:
        print(f"  Average bytes/ability: {len(bytecode) / len(abilities):.1f}")
        print(
            f"  Compressed from {len(json.dumps(data, ensure_ascii=False)) / 1024:.0f}KB JSON -> {len(bytecode) / 1024:.1f}KB bytecode  ({len(json.dumps(data, ensure_ascii=False)) / max(len(bytecode), 1):.0f}x reduction)"
        )


if __name__ == "__main__":
    main()
