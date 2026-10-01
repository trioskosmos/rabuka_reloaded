#!/usr/bin/env python3
"""Verify that the generated ability artifacts are mutually consistent.

WHY THIS EXISTS
---------------
The ability data reaches the C engine through three layers:

  1  cards/abilities.json                       (source, committed)
  2  cards/build/{abilities.bin, abilities.bin.z, abilities_strings.bin,
                  abilities_gen.rs, bytecode_data.c, generation_manifest.json}
  3  engine_c_wip/src/core/generated/{gen_data.c, gen_data.h,
                                      bytecode_blob.c}          (committed)

The C engine compiles the OFFSETS from layer 3 and reads the STRING TABLE from
layer 2 at runtime. Those two were allowed to drift apart, twice. Because the
interned string pool is UNSEPARATED, a few bytes of drift does not raise an
error -- it returns a byte splice of two neighbouring entries:

    cost_limit      -> cost_limitc
    left_side       -> eft_side
    invalidate_ability -> stiming_conditioni
    resource_type   -> ent_orderchoi

Worse, the drift can leave the pool's TOTAL LENGTH unchanged, so a length check
alone does not catch it.

This script is the independent checker. It recomputes every digest from the
bytes actually on disk and compares them against the committed C tables, so a
stale or partially-regenerated artifact set is a non-zero exit rather than a
silently wrong engine.

It is deliberately standalone -- it does NOT regenerate anything -- so it can be
run at any time, including by a build gate or a reviewer.

usage:  python3 tools/verify_artifacts.py [--quiet]
exit:   0 = consistent, 1 = skewed (details on stdout), 2 = artifact missing
"""
import hashlib
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
EC = ROOT / "engine_c_wip"
BUILD = ROOT / "cards" / "build"
GEN = EC / "src" / "core" / "generated"


def fnv1a(data: bytes) -> int:
    """FNV-1a 32. Must match rb_fnv1a32() in src/ability/vm.c exactly."""
    h = 0x811C9DC5
    for b in data:
        h = ((h ^ b) * 0x01000193) & 0xFFFFFFFF
    return h


def c_array(text: str, name: str) -> list[int]:
    """Pull a C integer array literal out of a generated source file.

    Handles the sized form (`const unsigned char X[1234] = {...}`), the unsized
    form (`const unsigned char X[] = {...}`, used for RBKA_BYTECODE), and hex
    literals as well as decimal.
    """
    m = re.search(r"(?<![A-Z_])" + re.escape(name) + r"\[\d*\]\s*=\s*\{(.*?)\}\s*;", text, re.S)
    if m:
        return [int(x, 16) if x.lower().startswith("0x") else int(x, 10)
                for x in re.findall(r"0[xX][0-9A-Fa-f]+|\d+", m.group(1))]
    m = re.search(re.escape(name) + r":\s*.*?=\s*&\[(.*?)\]\s*;", text, re.S)
    if m:
        return [int(x) for x in re.findall(r"\d+", m.group(1))]
    raise SystemExit(f"could not find array {name}")


def rs_array(text: str, name: str) -> list[int]:
    m = re.search(re.escape(name) + r":\s*&\[u\d+\]\s*=\s*&\[(.*?)\]\s*;", text, re.S)
    if not m:
        raise SystemExit(f"could not find {name} in abilities_gen.rs")
    return [int(x) for x in re.findall(r"\d+", m.group(1))]


def main() -> int:
    quiet = "--quiet" in sys.argv
    problems: list[str] = []
    notes: list[str] = []

    def bad(msg: str) -> None:
        problems.append(msg)

    rs_path = BUILD / "abilities_gen.rs"
    bin_path = BUILD / "abilities_strings.bin"
    gc_path = GEN / "gen_data.c"
    gh_path = GEN / "gen_data.h"
    bc_path = GEN / "bytecode_blob.c"
    js_path = ROOT / "cards" / "abilities.json"

    for p in (rs_path, bin_path, gc_path, gh_path, bc_path):
        if not p.exists():
            print(f"FATAL: required artifact missing: {p}")
            print("       Run: bash tools/regen_all.sh")
            return 2

    rs = rs_path.read_text(encoding="utf-8", errors="replace")
    gc = gc_path.read_text(encoding="utf-8", errors="replace")
    gh = gh_path.read_text(encoding="utf-8", errors="replace")
    bc = bc_path.read_text(encoding="utf-8", errors="replace")
    table = bin_path.read_bytes()

    offsets = c_array(gc, "RBKA_OFFSET_DELTAS")
    str_off = c_array(gc, "RBKA_STRINGS_OFFSETS")
    fnv = c_array(gc, "RBKA_STRINGS_FNV")
    pairs = c_array(gc, "RBKA_CARD_ABILITY_PAIRS")
    m = re.search(r"RBKA_STRINGS_TABLE_LEN\s*=\s*(\d+)", gc)
    table_len = int(m.group(1)) if m else None
    m = re.search(r"RBKA_NUM_ABILITIES\s*=\s*(\d+)", gc)
    num_ab = int(m.group(1)) if m else None

    n_str = len(str_off) - 1
    bytecode = c_array(bc, "RBKA_BYTECODE")

    # ── 1. shapes agree with the header ───────────────────────────────────
    m = re.search(r"RBKA_NUM_STRING_OFFSETS\s+(\d+)", gh)
    if not m or int(m.group(1)) != len(str_off):
        bad(f"gen_data.h RBKA_NUM_STRING_OFFSETS={m.group(1) if m else '?'} "
            f"but gen_data.c has {len(str_off)} offsets")
    if len(fnv) != n_str:
        bad(f"RBKA_STRINGS_FNV has {len(fnv)} entries but there are {n_str} "
            f"interned strings -- the digest table and the offset table disagree")
    m = re.search(r"RBKA_NUM_ABILITIES_C\s+(\d+)", gh)
    if m and num_ab is not None and int(m.group(1)) != num_ab:
        bad(f"gen_data.h RBKA_NUM_ABILITIES_C={m.group(1)} vs gen_data.c {num_ab}")
    m = re.search(r"RBKA_NUM_CARD_ABILITY_PAIRS\s+(\d+)", gh)
    if m and int(m.group(1)) != len(pairs) // 2:
        bad(f"gen_data.h RBKA_NUM_CARD_ABILITY_PAIRS={m.group(1)} vs "
            f"gen_data.c {len(pairs) // 2}")
    if len(offsets) != (num_ab or -1):
        bad(f"RBKA_OFFSET_DELTAS has {len(offsets)} entries, "
            f"RBKA_NUM_ABILITIES is {num_ab}")
    if len(pairs) % 2:
        bad("RBKA_CARD_ABILITY_PAIRS has an odd length")

    # ── 2. the bytecode blob is the size the deltas claim ──────────────────
    if len(bytecode) != sum(offsets):
        bad(f"bytecode_blob.c holds {len(bytecode)} bytes but "
            f"sum(RBKA_OFFSET_DELTAS) is {sum(offsets)}")

    # ── 3. offsets are well formed and end at the recorded length ───────────
    if not str_off or str_off[0] != 0:
        bad(f"RBKA_STRINGS_OFFSETS[0] is {str_off[0] if str_off else '?'}, expected 0")
    for i in range(n_str):
        if str_off[i] > str_off[i + 1]:
            bad(f"RBKA_STRINGS_OFFSETS is not monotonic at index {i} "
                f"({str_off[i]} > {str_off[i + 1]})")
            break
    if str_off and table_len is not None and str_off[-1] != table_len:
        bad(f"RBKA_STRINGS_TABLE_LEN={table_len} but the last offset is "
            f"{str_off[-1]}")
    if table_len is not None and len(table) != table_len:
        bad(f"string table on disk is {len(table)} bytes but gen_data.c "
            f"recorded {table_len}")

    # ── 4. THE LOAD-BEARING CHECK: every interned string's digest ──────────
    # Length-only checks are not enough (see module docstring), so recompute
    # FNV-1a for every string straight from the bytes on disk.
    mismatched = []
    for i in range(n_str):
        if str_off[i + 1] > len(table):
            bad(f"string #{i} runs past the end of the table "
                f"({str_off[i]}..{str_off[i + 1]} > {len(table)})")
            break
        seg = table[str_off[i]:str_off[i + 1]]
        if fnv1a(seg) != (fnv[i] if i < len(fnv) else None):
            mismatched.append(i)
    if mismatched:
        bad(f"{len(mismatched)} of {n_str} interned strings do not match the "
            f"digest recorded in gen_data.c (first: #{mismatched[0]})")
        notes.append("  -> gen_data.c and cards/build/abilities_strings.bin "
                     "are from DIFFERENT generator runs. "
                     "FIX: bash tools/regen_all.sh")

    # Whole-table digest: one comparison that catches ANY byte-level drift,
    # including drift that preserves every individual string length. Falls back
    # to "not recorded" on tables generated before this check existed.
    m = re.search(r"RBKA_STRINGS_TABLE_FNV\s*=\s*(\d+)", gc)
    if m:
        if int(m.group(1)) != fnv1a(table):
            bad(f"whole-table digest mismatch: gen_data.c records "
                f"{m.group(1)}, the file on disk hashes to {fnv1a(table)}")
    else:
        notes.append("  NOTE: gen_data.c has no RBKA_STRINGS_TABLE_FNV; "
                     "re-run tools/regen_all.sh to add the whole-table digest")

    # ── 5. gen_data.c must match the .rs that produced it ──────────────────
    try:
        rs_off = rs_array(rs, "STRINGS_OFFSETS")
        if rs_off != str_off:
            bad(f"gen_data.c STRINGS_OFFSETS ({len(str_off)}) != "
                f"abilities_gen.rs ({len(rs_off)})")
        rs_deltas = rs_array(rs, "OFFSET_DELTAS")
        if rs_deltas != offsets:
            bad(f"gen_data.c OFFSET_DELTAS ({len(offsets)}) != "
                f"abilities_gen.rs ({len(rs_deltas)})")
        rs_pairs = rs_array(rs, "CARD_ABILITY_PAIRS")
        if rs_pairs != pairs:
            bad(f"gen_data.c CARD_ABILITY_PAIRS ({len(pairs)}) != "
                f"abilities_gen.rs ({len(rs_pairs)})")
        m = re.search(r"NUM_ABILITIES:\s*usize\s*=\s*(\d+)", rs)
        if m and num_ab is not None and int(m.group(1)) != num_ab:
            bad(f"abilities_gen.rs NUM_ABILITIES={m.group(1)} vs "
                f"gen_data.c {num_ab}")
    except SystemExit as e:
        bad(str(e))

    # ── 6. the manifest must describe the source we actually compiled ──────
    man_path = BUILD / "generation_manifest.json"
    if man_path.exists() and js_path.exists():
        man = man_path.read_text(encoding="utf-8", errors="replace")
        m = re.search(r'"sha256":\s*"([0-9a-f]{16})"', man)
        if m:
            actual = hashlib.sha256(js_path.read_bytes()).hexdigest()[:16]
            if actual != m.group(1):
                bad(f"cards/abilities.json hashes to {actual} but "
                    f"generation_manifest.json was written for {m.group(1)} -- "
                    f"the manifest is from an older source")
    else:
        notes.append("  NOTE: generation_manifest.json or abilities.json "
                     "missing; skipped the source-freshness check")

    # ── report ─────────────────────────────────────────────────────────────
    if not quiet:
        print(f"  abilities            {num_ab}")
        print(f"  interned strings     {n_str}")
        print(f"  card-ability pairs   {len(pairs) // 2}")
        print(f"  string table bytes   {len(table)}")
        print(f"  bytecode bytes       {len(bytecode)}")
        for n in notes:
            print(n)

    if problems:
        print("STILL SKEWED -- the generated ability artifacts do not agree:")
        for p in problems:
            print(f"  * {p}")
        print("FIX: bash tools/regen_all.sh   (regenerates cards/build AND the "
              "committed C tables in one ordered run)")
        return 1

    if not quiet:
        print("  gen_data.c, bytecode_blob.c, abilities_gen.rs and "
              "abilities_strings.bin are consistent")
    return 0


if __name__ == "__main__":
    sys.exit(main())
