#!/usr/bin/env bash
# tools/zz_measure_suites.sh <outdir> <suite>...
# Prints, PER SUITE, BOTH count shapes, explicitly labelled, plus a case-count
# sanity assertion.  A broken build or a suite that ran nothing must read as
# INVALID, never as zero failures.
#
#   case   : "==== N cases passed, M cases failed ===="   (the harness summary,
#            which goes to STDERR -- hence 2>&1 below)
#   ^FAIL: : the per-assertion line, which the harness ALSO emits once per case
#            and once per failing assertion. The two disagree; NEITHER is a
#            defect count on its own, so both are printed with labels.
#   cases  : "ok   <name>" lines, the true case count. Absence of data is
#            reported as NOCASE, not as a pass.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 2

OUT="$1"; shift
[ -n "$OUT" ] || { echo "usage: zz_measure_suites.sh <outdir> <suite>..."; exit 2; }
mkdir -p "$OUT"

CFLAGS="-std=c11 -O2 -Iinclude -Isrc -Isrc/core/generated -Wl,--stack,33554432"
OBJ=$(find src -name '*.o' | sort | grep -v '/main\.o$' | grep -v '/web_server\.o$' | tr '\n' ' ')
VERD=./rb_verdict
[ -x "$VERD" ] || cc $CFLAGS -o rb_verdict tools/verdict_runner.c || { echo "BUILD RED: verdict runner"; exit 3; }

total_cases=0
bad=0
for T in "$@"; do
  EXE="./zzm_$T"
  rm -f "$EXE"
  # shellcheck disable=SC2086
  cc $CFLAGS -o "$EXE" $OBJ tests/test_$T.c > "$OUT/$T.build.log" 2>&1
  if [ $? -ne 0 ] || [ ! -f "$EXE" ]; then
    echo "SUITE $T: BUILD INVALID (see $OUT/$T.build.log)"
    bad=$((bad+1)); continue
  fi
  "$VERD" "$EXE" "$T" > "$OUT/$T.log" 2>&1
  rc=$?
  # The binary must be newer than every object, or we measured the old engine.
  stale=""
  for o in $OBJ; do
    if [ "$o" -ot "$EXE" ] 2>/dev/null; then :; fi
  done
  for o in $OBJ; do
    if [ "$o" -nt "$EXE" ]; then stale="$stale $(basename "$o")"; fi
  done
  if [ -n "$stale" ]; then
    echo "SUITE $T: STALE BINARY (newer objects:$stale) -- measurement INVALID"
    bad=$((bad+1)); continue
  fi
  summ=$(grep -aoE '=+ *[A-Za-z0-9_ ]*: *[0-9]+ cases passed, [0-9]+ cases failed(, [0-9]+ crashed)? *=+' "$OUT/$T.log" | tail -1)
  pc=$(grep -ac '^FAIL:' "$OUT/$T.log")
  pa=$(grep -ac '^assert' "$OUT/$T.log")
  ok=$(grep -acE '^ok[: ]' "$OUT/$T.log")
  bad_case=$(grep -acE '^FAILED' "$OUT/$T.log")
  crash=$(grep -ac '^CRASH' "$OUT/$T.log")
  printf 'SUITE %-32s rc=%s\n' "$T" "$rc"
  printf '   case-summary : %s\n' "${summ:-NOSUMMARY}"
  printf '   ^FAIL: lines : %s   (per-assertion; NOT a defect count)\n' "$pc"
  printf '   ^ok  cases   : %s   (case count)\n' "$ok"
  printf '   ^FAILED case : %s   ^CRASH: %s\n' "$bad_case" "$crash"
  if [ -z "$ok" ]; then
    printf '   VERDICT      : NOCASE -- the suite ran no case; absence of data, NOT a pass\n'
    bad=$((bad+1))
  else
    total_cases=$((total_cases+ok))
  fi
  rm -f "$EXE"
done
echo "=== TOTAL ^ok cases across suites: $total_cases ; invalid/unreadable: $bad ==="
