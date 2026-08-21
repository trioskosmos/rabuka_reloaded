#!/bin/bash
# ============================================================
#  Build Rabuka Jaguar cartridge via the wasm2c pipeline:
#    wasm (prebuilt by build_jaguar.bat step 1, Windows cargo)
#    -> wasm2c -> m68k-unknown-linux-gnu-gcc (-Os, big-endian)
#    -> .j64 cartridge -> C:\Emulators\BigPEmu
#
#  Run from WSL:  bash build_jag_wasm.sh
# ============================================================
set -e

GCC="m68k-unknown-linux-gnu-gcc"
OBJCOPY="m68k-unknown-linux-gnu-objcopy"

DIR=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/jaguar/wasm
WASM=/mnt/c/rust_targets/wasm32-unknown-unknown/release/rabuka_wasm.wasm
OUT=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/jaguar/output
BJL=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/research/jaguar/new_bjl/bin
DEST=/mnt/c/Emulators/BigPEmu
WORK=/root/dcbuild/jagwasm

mkdir -p "$OUT" "$DEST" "$WORK"
cd "$WORK"

echo "============================================"
echo " [1/6] wasm2c transpile (module name matters:"
echo "       file must be called rabuka_wasm.wasm)"
echo "============================================"
cp -f "$WASM" rabuka_wasm.wasm
/root/wabt-1.0.41/bin/wasm2c rabuka_wasm.wasm -o rabuka_wasm.c

echo "============================================"
echo " [2/6] copy sources + runtime"
echo "============================================"
cp -f "$DIR"/jag_main.c "$DIR"/jag_libc.c "$DIR"/jag_setjmp.S \
      "$DIR"/boot_jw.S "$DIR"/font.S "$DIR"/link_jagwasm.ld .
cp -f "$DIR"/light8x8.fnt "$DIR"/gpu_blob.bin .
mkdir -p stub/sys
cp -f "$DIR"/runtime/*.h "$DIR"/runtime/*.c "$DIR"/runtime/*.inc .
cp -f "$DIR"/runtime/stub/sys/mman.h stub/sys/

echo "============================================"
echo " [3/6] compile shell + runtime (m68k, -Os, BE)"
echo "============================================"
CFLAGS="-m68000 -ffreestanding -fno-builtin -nostdlib -fno-pic -fno-pie -DNDEBUG -Os \
  -DWASM_RT_USE_MMAP=0 -DWASM_RT_MEMCHECK_BOUNDS_CHECK \
  -DWASM_RT_INSTALL_SIGNAL_HANDLER=0 -DWABT_BIG_ENDIAN=1 \
  -ffunction-sections -fdata-sections -I. -Istub"

$GCC $CFLAGS -c jag_main.c     -o jag_main.o
$GCC $CFLAGS -c jag_libc.c     -o jag_libc.o
$GCC $CFLAGS -c jag_setjmp.S   -o jag_setjmp.o
$GCC $CFLAGS -c boot_jw.S      -o boot_jw.o
$GCC $CFLAGS -c font.S         -o font.o
$GCC $CFLAGS -c wasm-rt-impl.c    -o wasm-rt-impl.o
$GCC $CFLAGS -c wasm-rt-mem-impl.c -o wasm-rt-mem-impl.o

echo "============================================"
echo " [4/6] compile engine (~18MB generated C)"
echo "============================================"
time $GCC $CFLAGS -c rabuka_wasm.c -o rabuka_wasm.o 2>cc_engine.log || { tail -20 cc_engine.log; exit 1; }

echo "============================================"
echo " [5/6] link ELF at \$802000 (XIP) + gc-sections"
echo "============================================"
$GCC -m68000 -nostdlib -nostartfiles -no-pie -Wl,-T,link_jagwasm.ld -Wl,--gc-sections \
  -o rabuka_jag.elf boot_jw.o jag_main.o jag_libc.o jag_setjmp.o font.o \
  wasm-rt-impl.o wasm-rt-mem-impl.o rabuka_wasm.o -lgcc 2>link.log \
  && echo "       link OK" || { echo "LINK FAILED:"; tail -30 link.log; exit 1; }
m68k-unknown-linux-gnu-size rabuka_jag.elf

echo "============================================"
echo " [6/6] wrap .j64 + deploy to BigPEmu"
echo "============================================"
$OBJCOPY -O binary rabuka_jag.elf rabuka.raw
echo "       raw rom: $(stat -c %s rabuka.raw) bytes"
bzcat "$BJL/allff.bin.bz2" > allff.bin
cat "$BJL/Univ.bin" rabuka.raw allff.bin > "$OUT/rabuka_wasm.j64"
truncate -s 4M "$OUT/rabuka_wasm.j64"
cp -f "$OUT/rabuka_wasm.j64" "$DEST/rabuka.j64"
ls -la "$DEST/rabuka.j64"

echo
echo "BUILD DONE: C:\\Emulators\\BigPEmu\\rabuka.j64"
