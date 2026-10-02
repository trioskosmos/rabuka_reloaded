#!/bin/bash
# engine_c/platforms/cdi/build_cdi.sh — cross-build the CD-i module (SCC68070).
# Requires: m68k-linux-gnu-gcc (apt install gcc-m68k-linux-gnu binutils-m68k-linux-gnu)
# Usage:   build_cdi.sh [host]   (pass "host" to use the PC-filesystem read fallback)
#
# Run it with the WSL/bash that has the m68k toolchain on PATH. The tree root is
# derived from this script's own location, so the build follows the checkout
# instead of a hardcoded absolute path.
set -e
W="${ENGINE_C_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
if [ ! -f "$W/Makefile" ]; then
    echo "build_cdi.sh: no engine_c_wip tree at $W (set ENGINE_C_ROOT)" >&2
    exit 1
fi
cd "$W"
OUT="$W/platforms/cdi/output"
mkdir -p "$OUT"
HOSTFLAG=""
if [ "$1" = "host" ]; then HOSTFLAG="-DCDI_HOST"; fi

for tool in m68k-linux-gnu-gcc m68k-linux-gnu-size m68k-linux-gnu-objcopy; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "build_cdi.sh: $tool not found on PATH." >&2
        echo "              apt install gcc-m68k-linux-gnu binutils-m68k-linux-gnu" >&2
        exit 1
    fi
done

# Generate gen_data.bin (offset tables) — streamed from storage on CD-i.
python3 tools/gen_gen_data_bin.py

# The engine source list. This MIRRORS the host Makefile's SRC list, minus
# src/main.c (the CD-i entry point is cdi_main.c). It used to be a stale
# hand-copied subset missing 16 files, so the build had undefined references at
# link time for every symbol those files define -- rb_setup_*, rb_executor_*,
# rb_desc_state_verb_ja, RBKA_CARD_ABILITY_PAIRS, RBKA_BYTECODE, and so on.
#
# generated/gen_data.c IS in the list even though this target streams
# gen_data.bin instead: src/core/card.c indexes RBKA_CARD_ABILITY_PAIRS by name
# and src/ability/vm.c reads RBKA_STRINGS_FNV / RBKA_STRINGS_TABLE_LEN, and
# gen_data.c is the only file that defines them. The tables this variant exists
# to keep out of the ROM (RBKA_OFFSET_DELTAS, RBKA_STRINGS_OFFSETS) are dead on
# this target -- everything reads them through g_offset_deltas /
# g_strings_offsets -- so -fdata-sections + --gc-sections drop them from the
# image.
#
# generated/gen_data_rom.c is NOT in the list: it defines the same rb_load_gen_data
# and the same three g_* pointers as generated/gen_data_cdi.c, and compiling both
# is a multiple-definition link error. See gen_data_rom.c's PLATFORM VARIANT
# CONTRACT.
SRC="src/ability/enums.c src/ability/types.c src/ability/vm.c src/ability/condition.c src/ability/choice.c src/ability/ability_queue.c src/ability/dynamic_count.c src/ability/util.c src/ability/cost.c src/ability/compound.c src/ability/resolver.c src/ability/effects/executor.c src/ability/effects/ability_effects.c src/ability/effects/custom.c src/ability/effects/move.c src/ability/effects/move_missing.c src/ability/effects/rules.c src/ability/effects/look.c src/ability/effects/state.c src/ability/effects/ability.c src/ability/effects/misc.c src/ability/effects/draw.c src/ability/effects/score.c src/ability/log.c src/ability/debug.c src/ability/abilities_gen.c src/ability/describe.c src/game/deck_parser.c src/game/deck_builder.c src/game/game_setup.c src/core/card.c src/core/data.c src/core/alloc.c src/core/game_state_core.c src/core/modifiers.c src/core/stats_pipeline.c src/core/game_state_abilities.c src/core/tracking.c src/core/zones.c src/bytecode_data.c src/core/generated/bytecode_blob.c src/core/generated/gen_data.c src/core/generated/gen_data_cdi.c src/turn/phase.c src/turn/live.c src/turn/triggers.c src/engine.c"
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

# `set -e` used to abort here on a failed link, so the three diagnostic lines
# below never ran and the linker output -- the only thing that says WHICH
# symbol is missing -- was thrown away in /tmp. Capture it, print it, fail.
LINKLOG="$OUT/cdilink.txt"
LINKRC=0
m68k-linux-gnu-gcc -nostdlib -Tplatforms/cdi/cdi.ld -Wl,--gc-sections \
  -Wl,--start-group $OBJ "$OUT/crt0.o" "$OUT/cdi_lib.o" "$OUT/cdi_main.o" -lgcc -Wl,--end-group \
  -o "$OUT/rabuka_cdi.elf" 2>"$LINKLOG" || LINKRC=$?
echo "LINK_RC=$LINKRC"
echo "cdilink.txt contents:"; cat "$LINKLOG"
if [ "$LINKRC" -ne 0 ]; then
  echo "build_cdi.sh: LINK FAILED (rc=$LINKRC). Undefined references above are" >&2
  echo "the whole story: every 'undefined reference to' is a symbol no file in" >&2
  echo "the SRC list defines." >&2
  exit "$LINKRC"
fi
echo "elf present?"; ls -l "$OUT/rabuka_cdi.elf" 2>&1
m68k-linux-gnu-size "$OUT/rabuka_cdi.elf"
m68k-linux-gnu-objcopy -O binary "$OUT/rabuka_cdi.elf" "$OUT/rabuka_cdi.bin"
ls -l "$OUT/rabuka_cdi.bin"
