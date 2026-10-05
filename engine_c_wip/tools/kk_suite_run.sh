#!/bin/bash
# kk_suite_run.sh -- run named C suites, capturing stdout and stderr SEPARATELY
# as raw bytes (never through a PowerShell pipeline) so the suite summary can be
# decoded by BOM. Prints, per suite: build status, CASE COUNT, ^FAIL count, and
# the last summary lines. A suite that builds but prints NO case count is
# reported as BROKEN, not green.
set -u
cd /c/Users/trios/OneDrive/Documents/rabuka_reloaded/engine_c_wip || exit 9
OUT=/c/Users/trios/AppData/Local/Temp/kk_suite
mkdir -p "$OUT"
for T in "$@"; do
  echo "########## SUITE $T"
  make t-nogate T="$T" > "$OUT/$T.out" 2> "$OUT/$T.err"
  rc=$?
  echo "  make_exit=$rc"
  # Build/link failure signature: make reports it as "Error 1", not "error:".
  if grep -qE 'Error [0-9]+' "$OUT/$T.out" "$OUT/$T.err" 2>/dev/null; then
    echo "  BUILD_OR_LINK_ERROR: yes"
    grep -aE 'Error [0-9]+|undefined reference|collect2' "$OUT/$T.out" "$OUT/$T.err" 2>/dev/null | head -5 | sed 's/^/    /'
  else
    echo "  BUILD_OR_LINK_ERROR: no"
  fi
  echo "  bytes_out=$(wc -c < "$OUT/$T.out")  bytes_err=$(wc -c < "$OUT/$T.err")"
  echo "  DONE"
done
echo "########## ALL_DONE"
