#!/bin/bash
# run one or more suites, print case-count line + ^FAIL count + verdict
# usage: bash tools/zzrun.sh stem [stem...]
cd "$(dirname "$0")/.."
for T in "$@"; do
  out="zzlogs/ZZ_$T.txt"
  mkdir -p zzlogs
  make t-nogate T="$T" >"$out" 2>&1
  # summarise: every line that looks like a summary, plus FAIL counts
  printf '### %s\n' "$T"
  if ! grep -qE '[0-9]+ assertions?, [0-9]+ failures?' "$out"; then
    printf '  NO-SUMMARY (build/run problem)\n'
  fi
  grep -hoE '[0-9]+ assertions?, [0-9]+ failures?' "$out" | sort -u | sed 's/^/  SUM /'
  printf '  ^FAIL %s\n' "$(grep -c '^FAIL' "$out")"
  grep -E '^(PASSED|FAILED):' "$out" | tail -2 | sed 's/^/  /'
done