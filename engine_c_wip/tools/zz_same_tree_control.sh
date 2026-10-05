#!/usr/bin/env bash
# tools/zz_same_tree_control.sh <label>
#
# The eight-agent tree CANNOT be snapshotted across a rebuild, so a wall-clock
# "before" from earlier in the session is NOT a valid baseline. This does the
# only comparison a regression claim may rest on: a SAME-TREE control.
#
#   arm A (control) : the fix's own hunks are reverted in place, the affected
#                     .o files are DELETED (make will not rebuild a .o whose
#                     mtime looks newer than its .c), and the suite is rebuilt
#                     and run.
#   arm B (fixed)   : the hunks are restored, the same .o files are deleted,
#                     and the suite is rebuilt and run again.
#
# Everything else in the tree is untouched and identical between the two arms.
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
LBL="${1:?label}"
OUT="zzctl_$LBL"
mkdir -p "$OUT"

SUITES="parity_zone_source parity_zones_position parity_cce_gaps parity_int_gaps alias_ownership mechanics parity_queue_resume"
CFLAGS="-std=c11 -O2 -Iinclude -Isrc -Isrc/core/generated -Wl,--stack,33554432"

run_arm () {
  arm="$1"
  rm -f src/ability/choice.o src/turn/live.o src/ability/ability_queue.o \
        src/engine.o src/test_game.o
  # Deleting the .o is not enough -- they have to be REBUILT from the sources
  # as they stand in this arm, or the link picks up whichever objects were left
  # over and the arm measures nothing. `make` on OBJ_LIB is the only rebuild.
  make $(sed -n 's/^OBJ_LIB := //p' Makefile) > "$OUT/$arm.build.log" 2>&1 \
    || { echo "$arm: OBJECT REBUILD INVALID"; return 1; }
  for T in $SUITES; do
    cc $CFLAGS -o ./zzc_${arm}_$T $(find src -name '*.o' | sort | grep -v '/main\.o$' | grep -v '/web_server\.o$' | tr '\n' ' ') \
       tests/test_$T.c > "$OUT/$arm.$T.build.log" 2>&1 || { echo "$arm $T: BUILD INVALID"; continue; }
    ./rb_verdict ./zzc_${arm}_$T $T > "$OUT/$arm.$T.log" 2>&1 || true
  done
}

python tools/zz_unpatch.py
# Give OneDrive/AV a moment to release the handles the unpatch just closed;
# a transient PermissionError here is what silently invalidates the arm.
sleep 3
run_arm control
sleep 3
python tools/zz_repatch.py
sleep 3
run_arm fixed

for T in $SUITES; do
  echo "== $T"
  for arm in control fixed; do
    f="$OUT/$arm.$T.log"
    [ -f "$f" ] || { echo "   $arm: NO LOG"; continue; }
    printf '   %-8s ' "$arm"
    grep -aoE '[0-9]+ cases passed, [0-9]+ cases failed(, [0-9]+ crashed)?' "$f" | tail -1 | tr -d '\n'
    printf '  ^FAIL:%s  ^ok:%s' "$(grep -ac '^FAIL:' "$f")" "$(grep -acE '^ok[: ]' "$f")"
    echo
  done
done
rm -f ./zzc_*_*.exe
