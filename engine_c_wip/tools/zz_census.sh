#!/bin/sh
# For each non-static symbol defined in a file, count callers in
# src/ + platforms/ (production) vs tests/ + tools/ (test-only).
F="$1"
nm -g --defined-only "$(echo "$F" | sed 's/\.c$/.o/')" 2>/dev/null \
  | awk '$2=="T"{print $3}' | sort -u > zzlogs/_defs.txt
echo "=== $F : $(wc -l < zzlogs/_defs.txt) exported symbols ==="
printf '%-44s %6s %6s\n' SYMBOL PROD TEST
while read -r s; do
  prod=$(grep -rl --include=*.c --include=*.h "\b$s\b" src platforms 2>/dev/null \
         | grep -v "^$F$" | wc -l)
  tst=$(grep -rl --include=*.c --include=*.h "\b$s\b" tests tools 2>/dev/null | wc -l)
  printf '%-44s %6s %6s\n' "$s" "$prod" "$tst"
done < zzlogs/_defs.txt