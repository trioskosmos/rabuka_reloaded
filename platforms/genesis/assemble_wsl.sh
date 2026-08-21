#!/bin/bash
# Assemble all LLVM-emitted m68k .s files with GNU as (mature relaxation),
# then link the Genesis ROM. Bypasses LLVM's broken MCAssembler::relaxOnce.
set -e
export PATH=/usr/bin:/bin:/root/.cargo/bin
TARGET_DIR=/root/gen-target-asm
OUT=$TARGET_DIR/linked
mkdir -p "$OUT"
cd "$OUT"

echo "=== [1/3] assemble all .s with GNU as ==="
i=0
fail=0
while IFS= read -r f; do
    o="$OUT/$(basename "$f" .s).o"
    if ! /usr/bin/m68k-linux-gnu-as -march=68000 "$f" -o "$o" 2>>"$OUT/as_errors.log"; then
        echo "AS FAIL: $f"
        fail=$((fail+1))
    fi
    i=$((i+1))
done < <(/usr/bin/find $TARGET_DIR/m68k-unknown-none-elf/release/deps /root/gen-target-asm/m68k-unknown-none-elf/release/build -name '*.s' 2>/dev/null)
echo "assembled $i files, $fail failures"

echo "=== [2/3] link ==="
cd /mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/genesis
m68k-linux-gnu-gcc -m68000 -nostdlib -nostartfiles -no-pie \
    -Wl,-T,link.ld -o "$OUT/rabuka_genesis_asm" "$OUT"/*.o -lgcc 2>"$OUT/link_err.log" \
  && echo LINK_OK || { echo LINK_FAIL; tail -20 "$OUT/link_err.log"; exit 1; }

echo "=== [3/3] size + rom ==="
m68k-linux-gnu-size "$OUT/rabuka_genesis_asm"
m68k-linux-gnu-objcopy -O binary "$OUT/rabuka_genesis_asm" "$OUT/rabuka_genesis_asm.bin"
ls -la "$OUT"
