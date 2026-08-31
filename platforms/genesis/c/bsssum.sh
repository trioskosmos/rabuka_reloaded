#!/bin/bash
cd "$(dirname "$0")"
m68k-linux-gnu-nm -S obj/*.o 2>/dev/null | awk '$3=="b"||$3=="B"{tot+=strtonum("0x"$2)} END{print "total BSS bytes =", tot}'
m68k-linux-gnu-nm -S obj/*.o 2>/dev/null | awk '$3=="b"||$3=="B"{print strtonum("0x"$2), $4}' | sort -rn | head -15
