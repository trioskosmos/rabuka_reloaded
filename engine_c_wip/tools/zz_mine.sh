#!/bin/sh
# Measurement harness -- FINAL. Every summary shape observed in this tree:
#  A "==== N cases passed, M cases failed ===="  (with optional ", K crashed")
#  B "N cases, A assertions, M failures"
#  C "tests passed : P" + "tests failed (...) : F"
#  D "<stem>: A assertions, M failures"  and  "A assertions, M failures, K known gaps"
#  E "N checks failed"   and   "N tests ok, M failed, K crashed (...)"
#  F "N ok, M FAILED, K CRASHED, K SETUPBUG"
# Takes the LAST matching line in the log for each shape, so a per-case line
# ("7 assertion(s), 0 failure(s)") never masquerades as the suite summary.
# BUILD_RED comes ONLY from a compiler/linker diagnostic, never from a bare
# "Error 1", which make also emits for an ordinary red suite.
printf '%-30s %6s %6s %6s %6s %-7s %-12s %s\n' SUITE PASS FAIL CARET RESULT VERDICT STATUS
for T in "$@"; do
  f="zzlogs/$T.out"
  make --no-print-directory t-nogate T="$T" >"$f" 2>&1
  rc=$?
  if grep -qE '(^|[^_a-zA-Z])error: |undefined reference to|No rule to make' "$f"; then
    st=BUILD_RED
  elif [ ! -s "$f" ]; then st=EMPTY
  else st=run; fi
  P="-"; F="-"; SH=none
  n() { echo "$1" | grep -oE '[0-9]+' | head -1; }
  a=$(grep -oE '==== [0-9]+ cases passed, [0-9]+ cases failed' "$f" | tail -1)
  b=$(grep -oE '[0-9]+ cases, [0-9]+ assertions, [0-9]+ failures' "$f" | tail -1)
  c=$(grep -oE 'tests passed[[:space:]]*: [0-9]+' "$f" | tail -1)
  d=$(grep -oE '^[0-9]+ assertions, [0-9]+ failures' "$f" | tail -1)
  d2=$(grep -oE "^[a-z_0-9]+: [0-9]+ assertions, [0-9]+ failures" "$f" | tail -1)
  e=$(grep -oE '^[0-9]+ checks failed' "$f" | tail -1)
  e2=$(grep -oE '^[0-9]+ tests ok, [0-9]+ failed' "$f" | tail -1)
  f2=$(grep -oE '[0-9]+ ok, [0-9]+ FAILED, [0-9]+ CRASHED' "$f" | tail -1)
  if [ -n "$a" ]; then
    P=$(echo "$a" | grep -oE '^=+ [0-9]+' | n)
    F=$(echo "$a" | grep -oE '[0-9]+ cases failed' | n); SH=A
  elif [ -n "$b" ]; then
    P=$(n "$b"); F=$(echo "$b" | awk -F', ' '{print $3}' | n); SH=B
  elif [ -n "$c" ]; then
    P=$(echo "$c" | awk -F': ' '{print $2}')
    F=$(grep -oE 'tests failed[^:]*: [0-9]+' "$f" | tail -1 | awk -F': ' '{print $2}'); SH=C
  elif [ -n "$e2" ]; then
    P=$(echo "$e2" | cut -d' ' -f1); F=$(echo "$e2" | cut -d' ' -f3); SH=E
  elif [ -n "$f2" ]; then
    P=$(echo "$f2" | cut -d' ' -f1); F=$(echo "$f2" | cut -d' ' -f3); SH=F
  elif [ -n "$d" ]; then
    P=$(echo "$d" | cut -d' ' -f1); F=$(echo "$d" | cut -d' ' -f3); SH=D
  elif [ -n "$d2" ]; then
    P=$(n "$d2"); F=$(echo "$d2" | awk -F', ' '{print $2}' | n); SH=D
  elif [ -n "$e" ]; then F=$(n "$e"); SH=E; fi
  [ "$P" = "-" ] && [ "$F" != "-" ] && P="?"
  caret=$(grep -c '\^FAIL' "$f")
  v=$(grep -oE '^(PASSED|FAILED):' "$f" | tail -1 | tr -d ':')
  [ -z "$v" ] && v=NONE
  res=$(grep -oE '[0-9]+ result line\(s\)' "$f" | tail -1 | grep -oE '[0-9]+')
  [ -z "$res" ] && res="-"
  printf '%-30s %6s %6s %6s %6s %-7s %-12s %s\n' "$T" "$P" "$F" "$caret" "$res" "$v" "$st/$SH" "$rc"
done
