#!/bin/sh
for T in "$@"; do
  echo "########## $T  (bytes=$(wc -c < "zzlogs/$T.out"))"
  tail -6 "zzlogs/$T.out"
  echo "--- last 4 count-bearing lines anywhere ---"
  grep -nE '[0-9]+ (cases|assertions?|failures?|checks)' "zzlogs/$T.out" | tail -4
done
