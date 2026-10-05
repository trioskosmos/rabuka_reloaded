#!/usr/bin/env bash
# tools/zz_report.sh <label> <dir>
# Re-prints the per-suite counts captured by zz_measure_suites.sh, so a report
# can be written without re-running anything.  BOTH shapes are labelled:
#   case-summary : the harness's own "N cases passed, M cases failed" line
#   ^FAIL: lines : the per-assertion line count.  These disagree by 1.4-4x and
#                  NEITHER is a defect count.
set -u
LBL="$1"; DIR="$2"
for f in "$DIR"/*.log; do
  b=$(basename "$f" .log)
  case "$b" in *.build) continue;; esac
  echo "== $b"
  grep -aE '=+ *[A-Za-z0-9_ ]*: *[0-9]+ cases passed, [0-9]+ cases failed(, [0-9]+ crashed)? *=+' "$f" | tail -1 | sed 's/^/   case-summary : /'
  printf '   ^FAIL: lines : %s\n' "$(grep -ac '^FAIL:' "$f")"
  printf '   ^ok  cases   : %s\n' "$(grep -acE '^ok[: ]' "$f")"
  printf '   ^FAILED case : %s   ^CRASH: %s\n' "$(grep -acE '^FAILED' "$f")" "$(grep -ac '^CRASH' "$f")"
done
