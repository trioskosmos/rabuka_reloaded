#!/bin/sh
tag="$1"; shift
suites="$*"
[ -n "$tag" ] || { echo "usage: zzm6_sweep.sh <tag> <suites...>"; exit 2; }
mkdir -p "zzs/$tag"
for s in $suites; do
  if [ -f "tests/test_$s.c" ]; then
    sh -c "make --no-print-directory t-nogate T=$s >zzs/$tag/$s.out 2>zzs/$tag/$s.err"
    echo "$s exit=$?"
  else
    echo "$s MISSING"
  fi
done