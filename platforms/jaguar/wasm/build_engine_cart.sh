#!/bin/bash
# Rabuka Jaguar cartridge — wasm2c engine pipeline (endian-safe: wasm2c 1.0.41
# accesses linear memory via wasm_rt_memcpy byte copies, so m68k BE is fine).
#
#   rust -> wasm32 (jaguar feature: 512KB heap, 22-page linear memory)
#        -> wasm2c -> m68k-unknown-linux-gnu-gcc (freestanding, XIP from cart)
#        -> Univ.bin header + raw + $FF pad -> .j64 -> BigPEmu
#
# Reuses: platforms/jaguar/wasm shell (jag_main.c, boot_jw.S, font.S,
# jag_libc.c, jag_setjmp.S, link_jagwasm.ld) and /root/dcbuild/jagwasm.
set -e
GCC=m68k-unknown-linux-gnu-gcc
OBJCOPY=m68k-unknown-linux-gnu-objcopy
REPO=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded
DIR=$REPO/platforms/jaguar/wasm
BJL=$REPO/research/jaguar/new_bjl/bin
WASM_SRC=/mnt/c/rust_targets/wasm32-jag/wasm32-unknown-unknown/release/rabuka_wasm.wasm
WORK=/root/dcbuild/jagwasm
OUT=/mnt/c/Emulators/BigPEmu

mkdir -p "$WORK"; cd "$WORK"
cp -f "$DIR"/jag_main.c "$DIR"/boot_jw.S "$DIR"/font.S "$DIR"/jag_libc.c \
      "$DIR"/jag_setjmp.S "$DIR"/light8x8.fnt "$DIR"/gpu_blob.bin \
      "$DIR"/link_jagwasm.ld .
cp -f "$DIR"/runtime/wasm-rt* .
mkdir -p stub/sys && touch stub/sys/mman.h
cp -f "$WASM_SRC" rabuka_wasm.wasm

CFLAGS="-m68000 -ffreestanding -fno-builtin -nostdlib -fno-pic -fno-pie \
-DNDEBUG -Os -DWASM_RT_USE_MMAP=0 -DWASM_RT_MEMCHECK_BOUNDS_CHECK \
-DWASM_RT_INSTALL_SIGNAL_HANDLER=0 -DWASM_RT_STACK_DEPTH_COUNT=1 -Istub"

echo "=== [1/4] runtime + shell objects ==="
$GCC $CFLAGS -c wasm-rt-impl.c     -o wasm-rt-impl.o
$GCC $CFLAGS -c wasm-rt-mem-impl.c -o wasm-rt-mem-impl.o
$GCC $CFLAGS -c jag_main.c         -o jag_main.o
$GCC $CFLAGS -c jag_libc.c         -o jag_libc.o
$GCC $CFLAGS -c boot_jw.S          -o boot_jw.o
$GCC $CFLAGS -c font.S             -o font.o
$GCC $CFLAGS -c jag_setjmp.S       -o jag_setjmp.o
echo "runtime OK"

echo "=== [2/4] engine (~20MB generated C, m68k -Os: be patient) ==="
time $GCC $CFLAGS -fno-tree-vectorize -c rabuka_wasm.c -o rabuka_wasm.o
echo "engine OK"

echo "=== [3/4] link + raw ==="
$GCC -m68000 -nostdlib -nostartfiles -no-pie -Wl,-T,link_jagwasm.ld \
  -o rabuka_wamr_jag.elf boot_jw.o jag_main.o jag_libc.o jag_setjmp.o font.o \
  wasm-rt-impl.o wasm-rt-mem-impl.o rabuka_wasm.o -lgcc
$OBJCOPY -O binary rabuka_wamr_jag.elf rabuka_wamr_jag.raw
m68k-unknown-linux-gnu-size rabuka_wamr_jag.elf || true

echo "=== [4/4] cart (.j64: Univ header @\$800000 + code @\$802000 + \$FF pad) ==="
bzcat "$BJL/allff.bin.bz2" > allff.bin
cat "$BJL/Univ.bin" rabuka_wamr_jag.raw allff.bin > "$OUT/rabuka_jag.j64"
truncate -s 4M "$OUT/rabuka_jag.j64"
cp "$OUT/rabuka_jag.j64" "$REPO/platforms/jaguar/output/rabuka_jag.j64"
cp rabuka_wamr_jag.elf "$REPO/platforms/jaguar/output/"
echo "DONE: $OUT/rabuka_jag.j64 ($(stat -c %s "$OUT/rabuka_jag.j64") bytes)"
