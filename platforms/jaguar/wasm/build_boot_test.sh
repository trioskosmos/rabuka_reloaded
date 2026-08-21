#!/bin/bash
# Minimal boot test: boot_jw.S + font + tiny main, NO wasm engine.
set -e
GCC="m68k-unknown-linux-gnu-gcc"
OBJCOPY="m68k-unknown-linux-gnu-objcopy"
DIR=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/jaguar/wasm
OUT=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/jaguar/output
BJL=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/research/jaguar/new_bjl/bin
WORK=/root/dcbuild/jagtest
mkdir -p "$OUT" "$WORK"; cd "$WORK"
cp -f "$DIR"/boot_test.c "$DIR"/jag_setjmp.S "$DIR"/boot_jw.S "$DIR"/font.S "$DIR"/link_jagwasm.ld .
cp -f "$DIR"/light8x8.fnt "$DIR"/gpu_blob.bin .

CFLAGS="-m68000 -ffreestanding -fno-builtin -nostdlib -fno-pic -fno-pie -DNDEBUG -Os"
$GCC $CFLAGS -c boot_test.c   -o boot_test.o
$GCC $CFLAGS -c boot_jw.S     -o boot_jw.o
$GCC $CFLAGS -c font.S        -o font.o

$GCC -m68000 -nostdlib -nostartfiles -no-pie -Wl,-T,link_jagwasm.ld \
  -o boot_test.elf boot_jw.o boot_test.o font.o 2>link.log \
  && echo "link OK" || { tail -20 link.log; exit 1; }
m68k-unknown-linux-gnu-size boot_test.elf
m68k-unknown-linux-gnu-readelf -h boot_test.elf | grep Entry

$OBJCOPY -O binary boot_test.elf boot_test.raw
bzcat "$BJL/allff.bin.bz2" > allff.bin
cat "$BJL/Univ.bin" boot_test.raw allff.bin > "$OUT/boot_test.j64"
truncate -s 4M "$OUT/boot_test.j64"
echo "DONE: $OUT/boot_test.j64"
