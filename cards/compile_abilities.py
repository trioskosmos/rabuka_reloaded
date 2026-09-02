"""Auto-compiler: scans abilities.json, discovers field types, generates bytecode + Rust decoder."""

import json
import struct
import hashlib
import zlib
from pathlib import Path

from build_lib import (
    StringTable,
    write_len,
    read_len,
    compress_with_header,
    write_generation_manifest,
    write_string_blob,
    delta_encode_offsets,
    sha256_short,
)


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


def compile_all(abilities):
    """Store each `unique_abilities[i]` entry as a compact *binary JSON* slice.

    Binary JSON = a tagged tree with all strings (object keys AND string values)
    interned into a single `STRINGS` table and referenced by 2-byte indices.
    """
    SKIP_KEYS = {"cards"}

    strings = StringTable()

    EFFECT_LIST_KEYS = ("actions", "options", "effect_steps")
    CONDITION_KEYS = (
        "condition",
        "alternative_condition",
        "result_condition",
        "choice_condition",
        "activation_condition_parsed",
        "cause",
    )

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
        "modify_yell_source": 14,
        "custom": 14,
        "do_nothing": 14,
        "action_by": 14,
        "opponent_action": 14,
    }

    COST_TYPE_TO_ACTION = {
        "move_cards": "move_cards",
        "pay_energy": "pay_energy",
        "reveal": "reveal",
        "change_state": "change_state",
        "place_energy_under_member": "place_energy_under_member",
        "sequential_cost": "sequential_cost",
        "choice_condition": "choice_condition",
    }

    def enc_val(v, out: bytearray, in_effect_vec: bool = False, is_condition: bool = False):
        if v is None:
            out.append(0x00)
        elif isinstance(v, bool):
            out.append(0x02 if v else 0x01)
        elif isinstance(v, int):
            out.append(0x03)
            if v < 0:
                out.append(0xFF)
                out.extend(struct.pack("<I", v & 0xFFFFFFFF))
            elif v <= 0xFD:
                out.append(v & 0xFF)
            elif v <= 0xFFFF:
                out.append(0xFE)
                out.extend(struct.pack("<H", v))
            else:
                out.append(0xFF)
                out.extend(struct.pack("<I", v))
        elif isinstance(v, float):
            out.append(0x04)
            out.extend(struct.pack("<d", v))
        elif isinstance(v, str):
            out.append(0x06)
            out.extend(struct.pack("<H", strings.intern(v)))
        elif isinstance(v, list):
            out.append(0x07)
            write_len(out, len(v))
            for item in v:
                enc_val(item, out, in_effect_vec, is_condition)
        elif isinstance(v, dict):
            if is_condition:
                t = v.get("type") or ""
                vtag = COND_TO_VARIANT_TAG.get(t)
                if t == "or_condition" and "operator" not in v:
                    v["operator"] = "or"
                if t in ("has_moved", "not_moved") and "movement" not in v:
                    v["movement"] = t
                if vtag is not None:
                    v.pop("type", None)
                    out.append(0x09)
                    out.append(vtag)
                else:
                    out.append(0x08)
                write_len(out, len(v))
                for k, val in v.items():
                    out.extend(struct.pack("<H", strings.intern(str(k))))
                    enc_val(val, out, k in EFFECT_LIST_KEYS, k in CONDITION_KEYS or k == "conditions")
            elif "action" not in v:
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
                    v["action"] = ""
            if not is_condition:
                action = v.get("action", "")
                vtag = ACTION_TO_VARIANT_TAG.get(action, 0)
                if vtag:
                    out.append(0x09)
                    out.append(vtag)
                else:
                    out.append(0x08)
                write_len(out, len(v))
                for k, val in v.items():
                    out.extend(struct.pack("<H", strings.intern(str(k))))
                    enc_val(val, out, k in EFFECT_LIST_KEYS, k in CONDITION_KEYS or k == "conditions")
        else:
            out.append(0x00)

    def enc_entry(entry, out: bytearray):
        out.append(0x08)
        write_len(out, sum(1 for k in entry if k not in SKIP_KEYS))
        for k, val in entry.items():
            if k in SKIP_KEYS:
                continue
            out.extend(struct.pack("<H", strings.intern(str(k))))
            enc_val(val, out, k in EFFECT_LIST_KEYS, k in CONDITION_KEYS or k == "conditions")

    card_ability_pairs = []
    for idx, entry in enumerate(abilities):
        for card_entry in entry.get("cards", []):
            card_no = card_entry.split(" | ")[0] if " | " in card_entry else card_entry
            str_idx = strings.intern(card_no)
            card_ability_pairs.append((str_idx, idx))

    offsets, bytecode = [], bytearray()
    for entry in abilities:
        offsets.append(len(bytecode))
        enc_entry(entry, bytecode)
    offsets.append(len(bytecode))

    bytecode, offsets, strings_list, card_ability_pairs = compact_bytecode(
        bytes(bytecode), offsets, strings.get_strings(), card_ability_pairs
    )

    return bytes(bytecode), offsets, strings_list, card_ability_pairs


def compact_bytecode(bytecode, offsets, strings, card_ability_pairs):
    """Reorder strings by frequency and rewrite bytecode with u8+escape indices."""
    freq = [0] * len(strings)

    def count_one(bc, pos):
        if pos >= len(bc):
            return pos
        tag = bc[pos]
        pos += 1
        if tag in (0x00, 0x01, 0x02):
            return pos
        elif tag == 0x03:
            if pos >= len(bc):
                return len(bc)
            b = bc[pos]
            if b <= 0xFD:
                return pos + 1
            elif b == 0xFE:
                return pos + 3
            elif b == 0xFF:
                return pos + 5
            return pos + 9
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
            n, pos = read_len(bc, pos)
            for _ in range(n):
                pos = count_one(bc, pos)
            return pos
        elif tag == 0x08:
            n, pos = read_len(bc, pos)
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
            pos += 1
            n, pos = read_len(bc, pos)
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

    for i in range(len(offsets) - 1):
        s, e = offsets[i], offsets[i + 1]
        if s < e:
            count_one(bytecode, s)

    indexed = list(range(len(strings)))
    indexed.sort(key=lambda i: (-freq[i], i))
    new_idx = [0] * len(strings)
    for new_pos, old_pos in enumerate(indexed):
        new_idx[old_pos] = new_pos

    new_strings = [strings[old] for old in indexed]

    new_pairs = []
    for str_idx, ability_idx in card_ability_pairs:
        new_pairs.append((new_idx[str_idx], ability_idx))

    new_bytecode = bytearray()
    new_offsets = []

    def write_idx(out, idx):
        if idx < 0xFE:
            out.append(idx)
        else:
            out.append(0xFE)
            out.extend(struct.pack("<H", idx))

    def rewrite_one(bc, pos, out):
        if pos >= len(bc):
            return pos
        tag = bc[pos]
        pos += 1
        out.append(tag)
        if tag in (0x00, 0x01, 0x02):
            return pos
        elif tag == 0x03:
            if pos >= len(bc):
                return pos
            b = bc[pos]
            if b <= 0xFD:
                out.append(b)
                return pos + 1
            elif b == 0xFE:
                out.append(b)
                out.extend(bc[pos + 1 : pos + 3])
                return pos + 3
            elif b == 0xFF:
                out.append(b)
                out.extend(bc[pos + 1 : pos + 5])
                return pos + 5
            out.extend(bc[pos : pos + 9])
            return pos + 9
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
            n, pos = read_len(bc, pos)
            write_len(out, n)
            for _ in range(n):
                pos = rewrite_one(bc, pos, out)
            return pos
        elif tag == 0x08:
            n, pos = read_len(bc, pos)
            write_len(out, n)
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
            n, pos = read_len(bc, pos)
            write_len(out, n)
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


def generate_abilities_gen(bytecode, offsets, strings, card_ability_pairs, build_dir):
    assert (build_dir / "abilities.bin.z").exists(), (
        "run main() first: abilities.bin.z must exist before generating abilities_gen.rs"
    )

    blob_bytes, str_offsets = write_string_blob(strings, build_dir / "abilities_strings.bin")
    offsets_hex_str = ", ".join(str(o) for o in str_offsets)
    pair_strs = ", ".join(f"{s},{a}" for s, a in card_ability_pairs)

    offset_deltas = delta_encode_offsets(offsets)
    delta_strs = ", ".join(str(d) for d in offset_deltas)

    CHUNK_CAP = 30000
    snes_chunks, snes_locs, snes_sizes = [], [], []
    cur, cur_start = bytearray(), 0
    for i in range(len(offsets) - 1):
        d = offsets[i + 1] - offsets[i]
        if len(cur) + d > CHUNK_CAP and len(cur) > 0:
            snes_chunks.append(bytes(cur))
            snes_sizes.append(len(cur))
            cur = bytearray()
            cur_start = 0
        snes_locs.append((len(snes_chunks), cur_start, d))
        cur += bytecode[offsets[i] : offsets[i + 1]]
        cur_start += d
    if len(cur) > 0 or not snes_chunks:
        snes_chunks.append(bytes(cur))
        snes_sizes.append(len(cur))
    assert all(s <= 0xFFFF for s in snes_sizes), "chunk too big for u16 offset"

    snes_chunk_decls = "\n".join(
        f'extern "C" {{ pub static BYTECODE_C{ci}: [u8; {len(c)}]; }}' for ci, c in enumerate(snes_chunks)
    )
    snes_loc_strs = ", ".join(f"({c},{s},{l})" for c, s, l in snes_locs)
    snes_slice_arms = "\n".join(
        f'        {ci} => unsafe {{ &BYTECODE_C{ci}[start..start + len] }},' for ci in range(len(snes_chunks))
    )

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

#[cfg(all(not(feature = "snes"), not(feature = "gba")))]
pub const COMPRESSED_BYTECODE: &[u8] = include_bytes!("../../../cards/build/abilities.bin.z");
#[cfg(all(not(feature = "snes"), not(feature = "gba")))]
pub const DECOMPRESSED_LEN: usize = {len(bytecode)};
#[cfg(feature = "gba")]
pub const BYTECODE: &[u8] = include_bytes!("../../../cards/build/abilities.bin");

/// Per-ability byte lengths (deltas between consecutive absolute offsets).
/// `OFFSET_DELTAS[i]` is the size of the binary-JSON slice for
/// `unique_abilities[i]`. Absolute positions are rebuilt as a running prefix
/// sum (see `offset_of` in vm.rs). Stored as `u16` because slice lengths are
/// small; this is half the size of the old absolute u32 offset table.
#[cfg(not(feature = "snes"))]
pub const OFFSET_DELTAS: &[u16] = &[{delta_strs}];

/// SNES/16-bit path: the bytecode is split into <=64KB chunks (a single larger
/// object cannot exist on a 16-bit target). Data lives in ROM via these extern
/// symbols; `ABILITY_LOCS[i]` = (chunk_idx:u8, start:u16, len:u16) for ability i.
#[cfg(feature = "snes")]
{snes_chunk_decls}
#[cfg(feature = "snes")]
pub const ABILITY_LOCS: &[(u8, u16, u16)] = &[{snes_loc_strs}];

/// Return the bytecode slice for ability `idx`'s chunk at (start, len). 16-bit
/// safe: `start`/`len` are within a single <=64KB chunk.
#[cfg(feature = "snes")]
pub fn bytecode_slice(ci: u8, start: usize, len: usize) -> &'static [u8] {{
    match ci {{
{snes_slice_arms}
        _ => &[],
    }}
}}

/// Interned strings: object keys and string values. Stored as a single blob
/// with u32 offsets to save the 16-byte per-entry fat pointer overhead of
/// `&[&str]` (saves ~68KB for 5695 strings). Indexed by the 2-byte `u16`
/// references inside `BYTECODE` via `get_string(idx)`.
pub const STRINGS_BLOB: &[u8] = include_bytes!("../../../cards/build/abilities_strings.bin");
pub const STRINGS_OFFSETS: &[u32] = &[{offsets_hex_str}];
#[inline]
pub fn get_string(idx: usize) -> Option<&'static str> {{
    if idx + 1 >= STRINGS_OFFSETS.len() {{ return None; }}
    let start = STRINGS_OFFSETS[idx] as usize;
    let end = STRINGS_OFFSETS[idx + 1] as usize;
    unsafe {{ Some(core::str::from_utf8_unchecked(&STRINGS_BLOB[start..end])) }}
}}

/// Card_no -> ability index pairs. Each entry is (card_no_string_index, ability_index).
/// Generated from the `cards` field of `unique_abilities`. Used at load time by
/// `CardLoader::build_abilities_map_shared` to build the card_no -> Vec<AbilityRef>
/// mapping without parsing abilities.json into a `serde_json::Value`.
///
/// Format: flat array of [str_idx, ability_idx, str_idx, ability_idx, ...]
pub const CARD_ABILITY_PAIRS: &[u16] = &[{pair_strs}];
"""
    (build_dir / "abilities_gen.rs").write_text(src, encoding="utf-8")
    engine_dir = Path(__file__).parent.parent / "engine" / "src" / "ability"
    if engine_dir.exists():
        (engine_dir / "abilities_gen.rs").write_text(src, encoding="utf-8")

    c_lines = ['/* Auto-generated by compile_abilities.py: snes BYTECODE chunk data */']
    for ci, c in enumerate(snes_chunks):
        hexs = ", ".join(f"0x{b:02x}" for b in c)
        c_lines.append(f"const unsigned char BYTECODE_C{ci}[{len(c)}] = {{{hexs}}};")
    c_src = "\n".join(c_lines) + "\n"
    (build_dir / "bytecode_data.c").write_text(c_src, encoding="utf-8")
    (Path(__file__).parent.parent / "platforms" / "snes" / "bytecode_data.c").write_text(c_src, encoding="utf-8")
    print(f"  bytecode_data.c: {len(snes_chunks)} chunks")


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
    MAGIC = b"RBKA"
    VERSION = 1
    header = MAGIC + VERSION.to_bytes(4, "little")
    compressed = zlib.compress(header + bytes(bytecode), level=9)
    (build_dir / "abilities.bin.z").write_bytes(compressed)
    (build_dir / "abilities.bin").write_bytes(header + bytes(bytecode))
    print(f"\n  abilities.bin: {len(bytecode) + len(header)} bytes ({(len(bytecode)+len(header)) / 1024:.1f}KB) with header")
    print(f"  compressed: {len(compressed)} bytes ({len(compressed) / 1024:.1f}KB) ({100*(1-len(compressed)/(len(bytecode)+len(header))):.1f}% smaller)")
    print(f"  header: magic={MAGIC!r} version={VERSION}")
    print(f"  interned strings: {len(strings)}")
    print(f"  card->ability pairs: {len(card_ability_pairs)}")

    generate_abilities_gen(bytecode, offsets, strings, card_ability_pairs, build_dir)
    print(f"  Avg: {len(bytecode) / len(abilities):.1f} bytes/ability")

    abilities_json_path = root / "abilities.json"
    abilities_hash = sha256_short(abilities_json_path)
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

    write_generation_manifest(
        build_dir,
        "compiled_abilities.v1",
        "cards/compile_abilities.py",
        "cards/abilities.json",
        abilities_hash,
        len(abilities),
        len(bytecode) + len(header),
        len(compressed),
        "abilities",
        {
            "interned_strings": len(strings),
            "card_ability_pairs": len(card_ability_pairs),
            "sha256": bytecode_hash,
            "compressed_sha256": hashlib.sha256(compressed).hexdigest()[:16],
        },
    )
    print(f"  manifest: generation_manifest.json")


if __name__ == "__main__":
    main()