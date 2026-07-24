#!/usr/bin/bash
set -e
KOS_BASE=/opt/toolchains/dc/rust/kos
KOS_PORTS=/opt/toolchains/dc/rust/kos-ports
PATH=/opt/toolchains/dc/rust/sh-elf/bin:/opt/toolchains/dc/rust/kos/utils/build_wrappers:/opt/toolchains/dc/rust/bin:/usr/bin:/bin
BLD=/opt/toolchains/dc/rust/build_target
OUT=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/output_dc
SRC=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/dc

echo "=== Compile entry.c ==="
sh-elf-gcc -c "$SRC/entry.c" -o "$BLD/entry.o" -I$KOS_BASE/include -I$KOS_BASE/kernel/arch/dreamcast/include
echo "Compiled entry.o: $(stat --format=%s "$BLD/entry.o") bytes"

echo "=== Link ELF ==="
sh-elf-gcc "$BLD/entry.o" "$BLD/sh-elf/release/librabuka_dc.a" \
    -Wl,--gc-sections -T$KOS_BASE/utils/ldscripts/shlelf.xc -nodefaultlibs \
    -L$KOS_BASE/lib/dreamcast -L$KOS_BASE/addons/lib/dreamcast -L$KOS_PORTS/lib \
    -Wl,--start-group -lkallisti -lm -lc -lgcc -Wl,--end-group \
    -o "$OUT/rabuka_dc.elf"
echo "Linked rabuka_dc.elf: $(stat --format=%s "$OUT/rabuka_dc.elf") bytes"

echo "=== Strip to 1ST_READ.BIN ==="
mkdir -p "$OUT/disc"
sh-elf-objcopy -R .stack -O binary "$OUT/rabuka_dc.elf" "$OUT/disc/1ST_READ.BIN"
echo "Created 1ST_READ.BIN: $(stat --format=%s "$OUT/disc/1ST_READ.BIN") bytes"

echo "=== Key symbols ==="
sh-elf-nm "$OUT/rabuka_dc.elf" | grep -E "rabuka_main|_arch_main|_start|main"
echo "DONE"
