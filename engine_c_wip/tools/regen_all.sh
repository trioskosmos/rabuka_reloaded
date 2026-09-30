#!/usr/bin/env bash
# Regenerate the C engine's ability artifacts, IN ORDER, in one command.
#
# WHY THIS SCRIPT EXISTS
# ----------------------
# The ability data reaches the C engine through THREE layers, and on
# 2026-09-30 two of them were allowed to drift apart:
#
#   layer 1  cards/abilities.json                     (source, committed)
#            -> cards/compile_abilities.py
#   layer 2  cards/build/{abilities.bin,
#                          abilities.bin.z,
#                          abilities_strings.bin,     <-- GITIGNORED (.gitignore:189)
#                          abilities_gen.rs,          <-- GITIGNORED (.gitignore:112)
#                          bytecode_data.c,           <-- GITIGNORED (.gitignore:126)
#                          generation_manifest.json}
#   layer 3  engine_c_wip/src/core/generated/gen_data.{c,h}      <-- COMMITTED
#                          engine_c_wip/src/core/generated/bytecode_blob.c <-- COMMITTED
#
# The C engine reads the OFFSETS from layer 3 (compiled in) and the STRING
# TABLE from layer 2 (read at runtime from ../cards/build/). Rebuilding layer 2
# without rebuilding layer 3 produces a stale-offsets-plus-new-table pair.
# Because the interned string pool is UNSEPARATED, that pair is not an error —
# it is a silent byte splice of neighbouring entries. Measured on 2026-09-30:
# 227 of 6033 offsets wrong by up to 311 bytes, 156 strings carrying U+FFFD,
# 105 identifier keys spliced, and 458 of 985 abilities decoding at least one
# string wrong. The pool's total length was UNCHANGED, so a length check alone
# would not have caught it.
#
# The fix has two halves:
#   * this script, so the order is a single command and layer 2 cannot be
#     rebuilt in isolation; and
#   * rb_verify_generated_tables() in src/ability/vm.c, which recomputes an
#     FNV-1a digest of every interned string from the blob actually loaded at
#     runtime and compares it against a digest committed in gen_data.c. If
#     anyone rebuilds layer 2 without this script again, the engine now says
#     so by name instead of decoding garbage.
#
# usage:  bash tools/regen_all.sh          # everything, in order
#         bash tools/regen_all.sh --c-only # layers 2->3 only, skip compile_abilities
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EC="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO="$(cd "$EC/.." && pwd)"
PY="${PY:-python3}"

C_ONLY=0
[ "${1:-}" = "--c-only" ] && C_ONLY=1

if [ "$C_ONLY" -eq 0 ]; then
  echo "== layer 1->2: cards/compile_abilities.py =="
  ( cd "$REPO" && "$PY" cards/compile_abilities.py )
else
  echo "== layer 1->2: SKIPPED (--c-only) =="
  echo "   WARNING: cards/build/ was NOT rebuilt. If you just changed"
  echo "   cards/abilities.json, drop --c-only or the C tables will be"
  echo "   regenerated from stale Rust data."
fi

echo "== layer 2->3a: gen_from_rs.py (offsets + string digests) =="
# gen_from_rs.py reads the .rs AND the .bin it indexes, and REFUSES to emit if
# they disagree, so it cannot itself produce a skewed pair.
( cd "$EC" && "$PY" tools/gen_from_rs.py \
    "$REPO/cards/build/abilities_gen.rs" \
    "$REPO/cards/build/abilities_strings.bin" )

echo "== layer 2->3b: gen_bytecode.py (combined bytecode blob) =="
( cd "$EC" && "$PY" tools/gen_bytecode.py )

echo "== verifying the result =="
"$PY" - "$REPO" <<'PYEOF'
import hashlib, pathlib, re, sys
repo = pathlib.Path(sys.argv[1])
ec = repo / "engine_c_wip"
def arr(text, name):
    m = re.search(r"(?<![A-Z_])" + name + r"\[\d+\]\s*=\s*\{(.*?)\}\s*;", text, re.S)
    if m: return [int(x) for x in re.findall(r"\d+", m.group(1))]
    m = re.search(name + r":\s*.*?=\s*&\[(.*?)\]\s*;", text, re.S)
    return [int(x) for x in re.findall(r"\d+", m.group(1))]
gc = (ec / "src/core/generated/gen_data.c").read_text(encoding="utf-8")
rs = (repo / "cards/build/abilities_gen.rs").read_text(encoding="utf-8", errors="replace")
table = (repo / "cards/build/abilities_strings.bin").read_bytes()
ok = True
for name in ("RBKA_STRINGS_OFFSETS", "RBKA_OFFSET_DELTAS", "RBKA_CARD_ABILITY_PAIRS"):
    c = arr(gc, name); r = arr(rs, name.rsplit("_", 1)[-1] if name.startswith("RBKA_") else name)
    if c != r:
        print(f"  MISMATCH {name}: {len(c)} committed vs {len(r)} current"); ok = False
if arr(gc, "RBKA_STRINGS_OFFSETS")[-1] != len(table):
    print("  MISMATCH: string table length"); ok = False
bc = (ec / "src/core/generated/bytecode_blob.c").read_text(encoding="utf-8")
n = len(re.findall(r"\d+", re.search(r"RBKA_BYTECODE\[\]\s*=\s*\{(.*?)\}\s*;", bc, re.S).group(1)))
if n != sum(arr(gc, "RBKA_OFFSET_DELTAS")):
    print(f"  MISMATCH: blob {n} bytes vs sum(OFFSET_DELTAS) {sum(arr(gc,'RBKA_OFFSET_DELTAS'))}"); ok = False
print("  gen_data.c and bytecode_blob.c are consistent with cards/build/"
      if ok else "  STILL SKEWED -- do not commit")
sys.exit(0 if ok else 1)
PYEOF

echo
echo "Done. Commit gen_data.c AND bytecode_blob.c together with the"
echo "cards/build regeneration, or the skew comes straight back."
