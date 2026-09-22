#!/bin/bash
# engine_c/platforms/genesis/build_genesis.sh
# Cross-build a Sega Mega Drive / Genesis ROM from the engine_c port.
# Requires the prebuilt m68k-elf toolchain under engine_c/toolchains/m68k-gcc.
set -e
W="$(cd "$(dirname "$0")/../.." && pwd)"
TC="$W/toolchains/m68k-gcc/m68k-elf/bin"
OUT="$W/platforms/genesis/output"
mkdir -p "$OUT"

if [ ! -x "$TC/m68k-elf-gcc" ]; then
    echo "m68k-elf toolchain not found at $TC" >&2
    echo "Download a Windows m68k-elf-gcc (e.g. iratahack/m68k-elf-gcc) and extract there." >&2
    exit 1
fi

# Embed the data blobs into the ROM as read-only sections (.romdata).
# Use bare filenames so objcopy emits the simple _binary_<name>_start symbols
# that genesis_main.c references.
cp "$W/src/cards.bin" "$OUT/cards.bin"
cp "$W/src/abilities_strings.bin" "$OUT/abilities_strings.bin"
( cd "$OUT" && \
  "$TC/m68k-elf-objcopy" -I binary -O elf32-m68k --rename-section .data=.romdata \
      cards.bin cards_bin.o && \
  "$TC/m68k-elf-objcopy" -I binary -O elf32-m68k --rename-section .data=.romdata \
      abilities_strings.bin abstr_bin.o )

CFLAGS="-m68000 -std=c11 -Os -Wall -I$W/include -I$W/src -I$W/src/core/generated -ffreestanding -nostdlib -fno-builtin -fno-unwind-tables -fno-asynchronous-unwind-tables -fno-exceptions -fmerge-all-constants -ffunction-sections -fdata-sections -DRB_ROM_STRINGS"

SRC="src/ability/vm.c src/ability/condition.c src/ability/choice.c src/ability/ability_queue.c src/ability/dynamic_count.c src/ability/util.c src/ability/cost.c src/ability/compound.c src/ability/resolver.c src/ability/effects/move.c src/ability/effects/look.c src/ability/effects/state.c src/ability/effects/ability.c src/ability/effects/misc.c src/ability/effects/draw.c src/ability/effects/score.c src/ability/log.c src/ability/debug.c src/core/card.c src/core/data.c src/core/alloc.c src/core/modifiers.c src/core/stats_pipeline.c src/core/game_state_abilities.c src/core/tracking.c src/core/zones.c src/core/generated/bytecode_blob.c src/core/generated/gen_data.c src/turn/phase.c src/turn/live.c src/turn/triggers.c src/engine.c"

OBJ=""
for f in $SRC; do
    o="$OUT/$(echo "$f" | tr '/' '_').o"
    "$TC/m68k-elf-gcc" $CFLAGS -c -o "$o" "$W/$f"
    OBJ="$OBJ $o"
done
# platform objects
"$TC/m68k-elf-gcc" $CFLAGS -c -o "$OUT/crt0.o"        "$W/platforms/genesis/crt0.S"
"$TC/m68k-elf-gcc" $CFLAGS -c -o "$OUT/genesis_lib.o"  "$W/platforms/genesis/genesis_lib.c"
"$TC/m68k-elf-gcc" $CFLAGS -c -o "$OUT/vdp.o"         "$W/platforms/genesis/vdp.c"
"$TC/m68k-elf-gcc" $CFLAGS -c -o "$OUT/genesis_main.o" "$W/platforms/genesis/genesis_main.c"

"$TC/m68k-elf-gcc" -nostdlib -T"$W/platforms/genesis/genesis.ld" -Wl,--gc-sections \
    -o "$OUT/rabuka_genesis.elf" $OBJ "$OUT/crt0.o" "$OUT/genesis_lib.o" \
    "$OUT/vdp.o" "$OUT/genesis_main.o" "$OUT/cards_bin.o" "$OUT/abstr_bin.o" -lgcc

"$TC/m68k-elf-objcopy" -O binary "$OUT/rabuka_genesis.elf" "$OUT/rabuka_genesis.bin"
ls -l "$OUT/rabuka_genesis.bin"

# Pad to 2 MB and write the Sega checksum (sum of 16-bit words from 0x200).
python3 "$W/platforms/genesis/pad_checksum.py" "$OUT/rabuka_genesis.bin" 0x200000
cp "$OUT/rabuka_genesis.bin" "$OUT/rabuka_genesis.md"
echo "Built: $OUT/rabuka_genesis.bin (.md)"
