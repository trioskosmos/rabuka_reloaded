#!/bin/bash
# Build the engine_c -> Atari Jaguar cartridge (native m68k C,
# runs under WSL). The engine is compiled straight to 68000
# machine code — no wasm2c layer. rb_load_streaming +
# RB_ROM_STRINGS keeps every string in cart ROM (pack.py
# NUL-terminates the blobs and emits their pointer tables
# into ROM), so the whole engine fits the Jaguar's 2 MB DRAM:
#   GameState ~1.04 MB + libc heap 384 KB + stack ~300 KB.
#
#   m68k-unknown-linux-gnu-gcc -> elf -> raw -> Univ.bin
#   header + raw + $FF pad -> .j64 -> BigPEmu
#
# Reuses the proven wasm2c shell assets (gpu_blob.bin,
# font.S, light8x8.fnt) from platforms/jaguar/wasm/.
set -e
cd "$(dirname "$0")"

E="/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/engine_c_wip"
GCC=m68k-unknown-linux-gnu-gcc
OBJCOPY=m68k-unknown-linux-gnu-objcopy
WASM_DIR=../wasm
OUTDIR=output
EMU=/mnt/c/Emulators/BigPEmu

CFLAGS="-m68000 -O2 -std=c11 -ffreestanding -nostdinc -fno-builtin -Wall \
  -I libc -I $E/include -I $E/src -I $E/src/core/generated -DRB_ROM_STRINGS \
  -DRB_PROBE_DROPPED_CAP=256 -DRB_PROBE_KEYS_CAP=128"

mkdir -p obj "$OUTDIR"

echo "== pack ROM data (NUL-terminated blobs + ROM pointer tables) =="
python3 pack.py

echo "== copy proven Jaguar shell assets =="
cp -f "$WASM_DIR/gpu_blob.bin" "$WASM_DIR/font.S" "$WASM_DIR/light8x8.fnt" .

# Full engine library: the Makefile's OBJ_LIB minus the host-only
# files (web_server.c, main.c, test_game.c, choice_renderer.c —
# the last has no PlatformUi consumer on any console target).
ENG_SRC="ability/enums.c ability/types.c ability/vm.c ability/condition.c \
 ability/choice.c ability/ability_queue.c ability/dynamic_count.c \
 ability/util.c ability/cost.c ability/compound.c ability/resolver.c \
 ability/effects/executor.c ability/effects/ability_effects.c \
 ability/effects/custom.c ability/effects/move.c ability/effects/move_missing.c \
 ability/effects/rules.c ability/effects/look.c ability/effects/state.c \
 ability/effects/ability.c ability/effects/misc.c ability/effects/draw.c \
 ability/effects/score.c ability/log.c ability/debug.c \
 ability/abilities_gen.c ability/describe.c \
 game/deck_parser.c game/deck_builder.c game/game_setup.c \
 core/card.c core/data.c core/alloc.c core/game_state_core.c \
 core/modifiers.c core/stats_pipeline.c core/game_state_abilities.c \
 core/tracking.c core/zones.c bytecode_data.c \
 core/generated/bytecode_blob.c core/generated/gen_data.c core/gen_data_rom.c \
 turn/phase.c turn/live.c turn/triggers.c engine.c"

OBJS=""
for s in $ENG_SRC; do
  o="obj/$(basename "$s" .c).o"
  echo "  CC $s"
  $GCC $CFLAGS -c "$E/src/$s" -o "$o"
  OBJS="$OBJS $o"
done

for s in boot_native.S romdata.s sys.c console.c jag_native_main.c font.S; do
  base=$(basename "$s")
  o="obj/${base%.*}.o"
  echo "  CC $s"
  $GCC $CFLAGS -c "$s" -o "$o"
  OBJS="$OBJS $o"
done

echo "== linking =="
$GCC -m68000 -nostdlib -nostartfiles -no-pie -Wl,-T,link_jag_native.ld \
  -o "$OUTDIR/rabuka_jag_native.elf" $OBJS -lgcc
$OBJCOPY -O binary "$OUTDIR/rabuka_jag_native.elf" "$OUTDIR/rabuka_jag_native.raw"
m68k-unknown-linux-gnu-size "$OUTDIR/rabuka_jag_native.elf" || true

echo "== cart (.j64: Univ header @\$800000 + code @\$802000 + \$FF pad) =="
python3 - "$OUTDIR" <<'PYEOF'
import sys
out = sys.argv[1]
raw = open(out + "/rabuka_jag_native.raw", "rb").read()
img = open("Univ.bin", "rb").read() + raw
if len(img) > 4 * 1024 * 1024:
    sys.exit("image too big for a 4 MB cart: %d bytes" % len(img))
img += b"\xff" * (4 * 1024 * 1024 - len(img))
open(out + "/rabuka_jag_native.j64", "wb").write(img)
print("j64: %d bytes (raw %d)" % (len(img), len(raw)))
PYEOF

cp -f "$OUTDIR/rabuka_jag_native.j64" "$OUTDIR/rabuka_jag_native.elf" "$EMU/" 2>/dev/null || true
cp -f "$OUTDIR/rabuka_jag_native.j64" "$OUTDIR/rabuka_jag_native.elf" ../output/ 2>/dev/null || true
echo "DONE: $OUTDIR/rabuka_jag_native.j64"
