#!/usr/bin/env bash
# tools/zz_same_tree_control2.sh <label>
# Same-tree control for the ROUTE fix (fix2): revert fix2's three hunks in
# place, rebuild, run; restore, rebuild, run. Fix1 stays LANDED in both arms --
# it is already independently measured, and keeping it in both arms is what
# makes this arm differ ONLY by fix2.
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
LBL="${1:?label}"
OUT="zzctl_$LBL"
mkdir -p "$OUT"

SUITES="parity_zones_position parity_zone_source parity_sequential parity_cce_gaps parity_int_gaps alias_ownership mechanics parity_queue_resume parity_small_clusters parity_integration"
CFLAGS="-std=c11 -O2 -Iinclude -Isrc -Isrc/core/generated -Wl,--stack,33554432"

run_arm () {
  arm="$1"
  rm -f src/ability/choice.o src/ability/effects/misc.o src/turn/live.o \
        src/ability/ability_queue.o src/engine.o src/test_game.o
  make $(sed -n 's/^OBJ_LIB := //p' Makefile) > "$OUT/$arm.build.log" 2>&1 \
    || { echo "$arm: OBJECT REBUILD INVALID"; return 1; }
  for T in $SUITES; do
    # OneDrive/AV intermittently holds the freshly-linked .exe and gcc fails
    # with no diagnostic at all. One retry, and the failure is reported with
    # the build log rather than as a silent "BUILD INVALID".
    cc $CFLAGS -o ./zzc2_${arm}_$T $(find src -name '*.o' | sort | grep -v '/main\.o$' | grep -v '/web_server\.o$' | tr '\n' ' ') \
       tests/test_$T.c > "$OUT/$arm.$T.build.log" 2>&1 \
    || { sleep 4; cc $CFLAGS -o ./zzc2_${arm}_$T $(find src -name '*.o' | sort | grep -v '/main\.o$' | grep -v '/web_server\.o$' | tr '\n' ' ') \
       tests/test_$T.c >> "$OUT/$arm.$T.build.log" 2>&1; } \
    || { echo "$arm $T: BUILD INVALID"; continue; }
    ./rb_verdict ./zzc2_${arm}_$T $T > "$OUT/$arm.$T.log" 2>&1 || true
  done
}

python tools/zz_unpatch2.py
sleep 3
run_arm control
sleep 3
python tools/zz_repatch2.py
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
rm -f ./zzc2_*.exe
