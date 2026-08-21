#!/bin/bash
# Minimal boot test -> .rom (raw cart image, no Univ stub needed if BigPEmu
# loads .rom at $800000... but keep Univ+code layout so offset 0x2000=$802000).
set -e
GCC="m68k-unknown-linux-gnu-gcc"
OBJCOPY="m68k-unknown-linux-gnu-objcopy"
DIR=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/jaguar/wasm
OUT=/mnt/c/Emulators/BigPEmu
BJL=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/research/jaguar/new_bjl/bin
WORK=/root/dcbuild/jagtest
mkdir -p "$WORK"; cd "$WORK"
cp -f "$DIR"/boot_test.c "$DIR"/boot_jw.S "$DIR"/font.S "$DIR"/link_jagwasm.ld .
CFLAGS="-m68000 -ffreestanding -fno-builtin -nostdlib -fno-pic -fno-pie -DNDEBUG -Os"
$GCC $CFLAGS -c boot_test.c -o boot_test.o
$GCC $CFLAGS -c boot_jw.S  -o boot_jw.o
$GCC $CFLAGS -c font.S     -o font.o
$GCC -m68000 -nostdlib -nostartfiles -no-pie -Wl,-T,link_jagwasm.ld \
  -o boot_test.elf boot_jw.o boot_test.o font.o || { echo LINK FAIL; exit 1; }
$OBJCOPY -O binary boot_test.elf boot_test.raw
bzcat "$BJL/allff.bin.bz2" > allff.bin
cat "$BJL/Univ.bin" boot_test.raw allff.bin > "$OUT/boot_test.rom"
truncate -s 1M "$OUT/boot_test.rom"
echo "DONE: $OUT/boot_test.rom ($(stat -c %s "$OUT/boot_test.rom") bytes)"
