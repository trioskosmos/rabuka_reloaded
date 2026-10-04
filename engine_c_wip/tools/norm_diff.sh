#!/bin/bash
# Normalised (pointer + long-number masked) diff of two suites' streams.
B="$1"; F="$2"; shift 2
for s in "$@"; do
  echo "=== $s stdout"
  diff <(sed -E 's/0x[0-9a-fA-F]{6,}/PTR/g; s/[0-9]{6,}/NUM/g' "$B/$s.out") \
       <(sed -E 's/0x[0-9a-fA-F]{6,}/PTR/g; s/[0-9]{6,}/NUM/g' "$F/$s.out") | head -14
  echo "=== $s stderr"
  diff <(sed -E 's/0x[0-9a-fA-F]{6,}/PTR/g; s/[0-9]{6,}/NUM/g' "$B/$s.err") \
       <(sed -E 's/0x[0-9a-fA-F]{6,}/PTR/g; s/[0-9]{6,}/NUM/g' "$F/$s.err") | head -14
done