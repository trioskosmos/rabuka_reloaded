#!/bin/bash
# Build the engine_c -> Sega Genesis ROM (runs under WSL).
set -e
cd "$(dirname "$0")"

E="/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/engine_c"
CC=m68k-linux-gnu-gcc
CFLAGS="-O2 -std=c11 -ffreestanding -nostdinc -Wall -I libc -I $E/include -I $E/src -I $E/src/core/generated -DRB_ROM_STRINGS"

mkdir -p obj output

echo "== packing ROM data =="
python3 pack.py

ENG_SRC="ability/vm.c ability/condition.c ability/choice.c ability/ability_queue.c \
ability/dynamic_count.c ability/util.c ability/cost.c ability/compound.c \
ability/resolver.c ability/effects/move.c ability/effects/look.c ability/effects/state.c \
ability/effects/ability.c ability/effects/misc.c ability/effects/draw.c ability/effects/score.c \
ability/log.c core/card.c core/data.c core/alloc.c core/modifiers.c core/stats_pipeline.c \
core/game_state_abilities.c core/tracking.c core/zones.c core/generated/bytecode_blob.c \
core/generated/gen_data.c turn/phase.c turn/live.c turn/triggers.c engine.c"

OBJS=""
for s in $ENG_SRC; do
  o="obj/$(basename "$s" .c).o"
  echo "  CC $s"
  $CC $CFLAGS -c "$E/src/$s" -o "$o"
  OBJS="$OBJS $o"
done

for s in crt0.s romdata.s sys.c console.c genesis_main.c; do
  o="obj/$(basename "$s" .c).o"; [ "${s##*.}" = "s" ] && o="obj/$(basename "$s" .s).o"
  echo "  CC $s"
  $CC $CFLAGS -c "$s" -o "$o"
  OBJS="$OBJS $o"
done

echo "== linking =="
$CC -T link_genesis.ld -nostdlib -lgcc -o output/rabuka_genesis_c.elf $OBJS
m68k-linux-gnu-objcopy -O binary output/rabuka_genesis_c.elf output/rabuka_genesis_c.bin
m68k-linux-gnu-size output/rabuka_genesis_c.elf
echo "== built output/rabuka_genesis_c.bin =="
ls -l output/rabuka_genesis_c.bin
