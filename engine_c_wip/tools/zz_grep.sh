#!/bin/sh
for T in "$@"; do
  echo "########## $T"
  grep -nE 'cases|assertion|failure|^FAIL|^PASSED|^FAILED|error:|cases passed' "zzlogs/$T.out" | tail -12
done
