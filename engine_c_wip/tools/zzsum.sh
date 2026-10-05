#!/bin/bash
# Print the summary block (the lines just before the verdict line) of each zzlog.
cd "$(dirname "$0")/.."
for f in zzlogs/ZZ_*.txt; do
  echo "=== $f"
  awk '/^(FAILED|PASSED): /{exit} {buf[NR%16]=$0} END{}
       {lines[NR]=$0}
       END{}' "$f" >/dev/null
  grep -aB12 -aE '^(FAILED|PASSED): ' "$f" | grep -avE '^(FAILED|PASSED): |^--$' | tail -12
done