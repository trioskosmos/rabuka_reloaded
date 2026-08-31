#!/bin/bash
cd "$(dirname "$0")"
for o in obj/*.o; do
  m68k-linux-gnu-nm -S --size-sort "$o" 2>/dev/null
done | awk '$3=="b" || $3=="B" {print $2, $1, $4}' | sort -rn | head -30
