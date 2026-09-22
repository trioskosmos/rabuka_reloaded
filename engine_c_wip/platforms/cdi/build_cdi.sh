#!/bin/bash
# engine_c/platforms/cdi/build_cdi.sh — cross-build the CD-i module (SCC68070).
# Requires: m68k-linux-gnu-gcc (apt install gcc-m68k-linux-gnu binutils-m68k-linux-gnu)
# Usage:   build_cdi.sh [host]   (pass "host" to use the PC-filesystem read fallback)
set -e
W=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/engine_c
cd "$W"
OUT="$W/platforms/cdi/output"
mkdir -p "$OUT"
HOSTFLAG=""
if [ "$1" = "host" ]; then HOSTFLAG="-DCDI_HOST"; fi

# Generate gen_data.bin (offset tables) — streamed from storage on CD-i.
python3 tools/gen_gen_data_bin.py

SRC="src/ability/vm.c src/ability/condition.c src/ability/choice.c src/ability/ability_queue.c src/ability/dynamic_count.c src/ability/util.c src/ability/cost.c src/ability/compound.c src/ability/resolver.c src/ability/effects/move.c src/ability/effects/look.c src/ability/effects/state.c src/ability/effects/ability.c src/ability/effects/misc.c src/ability/effects/draw.c src/ability/effects/score.c src/ability/log.c src/ability/debug.c src/core/card.c src/core/data.c src/core/alloc.c src/core/modifiers.c src/core/stats_pipeline.c src/core/game_state_abilities.c src/core/tracking.c src/core/zones.c src/core/generated/gen_data_cdi.c src/turn/phase.c src/turn/live.c src/turn/triggers.c src/engine.c"
OBJ=""
CFLAGS="-std=c11 -Os -Wall -Iinclude -Isrc -Isrc/core/generated -ffreestanding -nostdlib -m68000 -fno-builtin -D_FORTIFY_SOURCE=0 -ffunction-sections -fdata-sections -fno-unwind-tables -fno-asynchronous-unwind-tables -fno-exceptions -fmerge-all-constants $HOSTFLAG"
for f in $SRC; do
  o="$OUT/$(echo "$f" | tr '/' '_').o"
  m68k-linux-gnu-gcc $CFLAGS -c -o "$o" "$f"
  OBJ="$OBJ $o"
done
# platform objects
m68k-linux-gnu-gcc $CFLAGS -c -o "$OUT/crt0.o" platforms/cdi/crt0.S
m68k-linux-gnu-gcc $CFLAGS -c -o "$OUT/cdi_lib.o" platforms/cdi/cdi_lib.c
m68k-linux-gnu-gcc $CFLAGS -c -o "$OUT/cdi_main.o" platforms/cdi/cdi_main.c

m68k-linux-gnu-gcc -nostdlib -Tplatforms/cdi/cdi.ld -Wl,--gc-sections \
  -Wl,--start-group $OBJ "$OUT/crt0.o" "$OUT/cdi_lib.o" "$OUT/cdi_main.o" -lgcc -Wl,--end-group \
  -o "$OUT/rabuka_cdi.elf" 2>/tmp/cdilink.txt
echo "LINK_RC=$?"
echo "cdilink.txt contents:"; cat /tmp/cdilink.txt
echo "elf present?"; ls -l "$OUT/rabuka_cdi.elf" 2>&1
m68k-linux-gnu-size "$OUT/rabuka_cdi.elf"
m68k-linux-gnu-objcopy -O binary "$OUT/rabuka_cdi.elf" "$OUT/rabuka_cdi.bin"
ls -l "$OUT/rabuka_cdi.bin"
