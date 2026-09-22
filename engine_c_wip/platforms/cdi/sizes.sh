#!/bin/bash
cd /mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/engine_c/platforms/cdi/output
printf "%8s %s\n" KB FILE
for f in src_*.o cdi_*.o crt0.o; do
  m68k-linux-gnu-size "$f" 2>/dev/null | awk 'NR==2{printf "%8d %s\n", int($4/1024), fname}' fname="$f"
done | sort -rn | head -25
