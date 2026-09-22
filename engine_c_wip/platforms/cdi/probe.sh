#!/bin/bash
set -e
W=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/engine_c
cd "$W"
SRC="src/ability/vm.c src/ability/condition.c src/ability/choice.c src/ability/ability_queue.c src/ability/dynamic_count.c src/ability/util.c src/ability/cost.c src/ability/compound.c src/ability/resolver.c src/ability/effects/move.c src/ability/effects/look.c src/ability/effects/state.c src/ability/effects/ability.c src/ability/effects/misc.c src/ability/effects/draw.c src/ability/effects/score.c src/ability/log.c src/ability/debug.c src/core/card.c src/core/data.c src/core/alloc.c src/core/modifiers.c src/core/stats_pipeline.c src/core/game_state_abilities.c src/core/tracking.c src/core/zones.c src/core/generated/bytecode_blob.c src/core/generated/gen_data.c src/turn/phase.c src/turn/live.c src/turn/triggers.c src/engine.c src/main.c"
mkdir -p /tmp/cdiobj
CFLAGS="-std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated -ffreestanding -nostdlib -m68000 -ffunction-sections -fdata-sections"
for f in $SRC; do
  o=/tmp/cdiobj/$(echo "$f" | tr '/' '_').o
  m68k-linux-gnu-gcc $CFLAGS -c -o "$o" "$f" 2>/tmp/err.txt || { echo "COMPILE FAIL: $f"; head -30 /tmp/err.txt; exit 1; }
done
echo "ALL COMPILED OK"
m68k-linux-gnu-gcc -nostdlib -Wl,--gc-sections -o /tmp/probe.elf /tmp/cdiobj/*.o 2>/tmp/linkerr.txt || true
echo "=== UNDEFINED SYMBOLS (libc needs) ==="
grep -oE "undefined reference to .[a-zA-Z_]+." /tmp/linkerr.txt | sed -E "s/undefined reference to .//; s/.//" | sort -u
