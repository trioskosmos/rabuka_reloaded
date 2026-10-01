#!/usr/bin/env bash
# Regenerate the C engine's ability artifacts, IN ORDER, in one command.
#
# THIS IS THE ONLY SUPPORTED WAY TO REGENERATE THE C ABILITY ARTIFACTS.
# Running cards/compile_abilities.py on its own is NOT enough and leaves the
# engine silently skewed -- see the incident notes below.
#
# WHY THIS SCRIPT EXISTS
# ----------------------
# The ability data reaches the C engine through THREE layers:
#
#   layer 1  cards/abilities.json                     (source, committed)
#            -> cards/compile_abilities.py
#   layer 2  cards/build/{abilities.bin,
#                          abilities.bin.z,
#                          abilities_strings.bin,     <-- now TRACKED in git
#                          abilities_gen.rs,          <-- GITIGNORED (.gitignore:112)
#                          bytecode_data.c,           <-- GITIGNORED (.gitignore:126)
#                          generation_manifest.json}
#   layer 3  engine_c_wip/src/core/generated/gen_data.{c,h}      <-- COMMITTED
#                          engine_c_wip/src/core/generated/bytecode_blob.c <-- COMMITTED
#
# The C engine reads the OFFSETS from layer 3 (compiled in) and the STRING
# TABLE from layer 2 (read at runtime from ../cards/build/). Rebuilding layer 2
# without rebuilding layer 3 produces a stale-offsets-plus-new-table pair.
# Because the interned string pool is UNSEPARATED, that pair is not an error --
# it is a silent byte splice of neighbouring entries. This happened TWICE:
#
#   2026-09-30  227 of 6033 offsets wrong by up to 311 bytes, 458 of 985
#               abilities decoding at least one string wrong.
#   2026-10-01  1408 defective string references (111 in key position) across
#               494 of 985 abilities. The pool's total length was UNCHANGED
#               between the two runs, so a length check alone would NOT have
#               caught it: `cost_limit` -> `cost_limitc`, `left_side` ->
#               `eft_side`, `resource_type` -> `ent_orderchoi`.
#
# The fix has four independent halves, so that no single forgotten step can
# reintroduce a skewed engine:
#
#   1. THIS SCRIPT, so the order is one command and layer 2 cannot be rebuilt
#      in isolation;
#   2. cards/build/abilities_strings.bin is TRACKED IN GIT, so the bytes the
#      committed offsets point at are versioned with the offsets themselves.
#      `git status` now shows the drift instead of hiding it;
#   3. tools/verify_artifacts.py, an INDEPENDENT checker that recomputes every
#      digest from the bytes on disk. This script runs it at the end and exits
#      non-zero on any disagreement, so a partial regeneration fails here
#      instead of at some later test run;
#   4. rb_verify_generated_tables() in src/ability/vm.c, which recomputes the
#      per-string FNV-1a digests AND a whole-table digest from the blob actually
#      loaded at runtime and compares them against digests committed in
#      gen_data.c. tests/test_artifacts_consistent.c asserts on it, so the skew
#      is also caught by the ordinary test target.
#
# usage:  bash tools/regen_all.sh            # everything, in order + verify
#         bash tools/regen_all.sh --c-only   # layers 2->3 only (see caveat)
#         bash tools/regen_all.sh --verify   # verify only, regenerate nothing
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EC="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO="$(cd "$EC/.." && pwd)"
PY="${PY:-python3}"
export PYTHONPATH="$REPO/cards${PYTHONPATH:+:$PYTHONPATH}"

MODE="${1:-}"
case "$MODE" in
  --verify)
    echo "== verifying only (regenerating nothing) =="
    ( cd "$EC" && "$PY" tools/verify_artifacts.py )
    exit $?
    ;;
  --c-only) ;;
  "") ;;
  *) echo "unknown argument: $MODE (expected --verify or --c-only)"; exit 2 ;;
esac

if [ "$MODE" != "--c-only" ]; then
  echo "== layer 1->2: cards/compile_abilities.py =="
  # This rewrites abilities_strings.bin IN PLACE. If anything below then fails,
  # the working tree is left skewed -- which is exactly the state this script
  # exists to prevent, so the verification at the end is not optional.
  ( cd "$REPO" && "$PY" cards/compile_abilities.py )
else
  echo "== layer 1->2: SKIPPED (--c-only) =="
  echo "   WARNING: cards/build/ was NOT rebuilt. If you just changed"
  echo "   cards/abilities.json, drop --c-only or the C tables will be"
  echo "   regenerated from stale Rust data."
fi

echo "== layer 2->3a: gen_from_rs.py (offsets + string digests) =="
# gen_from_rs.py reads the .rs AND the .bin it indexes, and REFUSES to emit if
# they disagree, so it cannot itself produce a skewed pair. It now also records
# RBKA_STRINGS_TABLE_FNV, a digest of the whole pool as one byte run, which
# catches drift that leaves every individual string length unchanged.
( cd "$EC" && "$PY" tools/gen_from_rs.py \
    "$REPO/cards/build/abilities_gen.rs" \
    "$REPO/cards/build/abilities_strings.bin" )

echo "== layer 2->3b: gen_bytecode.py (combined bytecode blob) =="
( cd "$EC" && "$PY" tools/gen_bytecode.py )

echo "== verifying the result (tools/verify_artifacts.py) =="
# Independent of the generators above: it re-reads every artifact from disk and
# recomputes every digest. Non-zero exit here fails the whole script.
( cd "$EC" && "$PY" tools/verify_artifacts.py )

echo
echo "Done. artifacts/abilities_strings.bin is TRACKED, so commit it TOGETHER"
echo "with gen_data.c and bytecode_blob.c -- one commit, three files. If you"
echo "committed only the C tables, the next agent's checkout is skewed again."
