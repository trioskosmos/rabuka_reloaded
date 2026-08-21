#!/bin/bash
set -e
source /root/kos/environ.sh >/dev/null
cd /root/dcbuild/optbuild
CFLAGS="-O2 -DWASM_RT_USE_MMAP=0 -DWASM_RT_MEMCHECK_BOUNDS_CHECK -DWASM_RT_INSTALL_SIGNAL_HANDLER=0 -Istub"
echo "=== engine (optimized wasm2c C) ==="
time kos-cc $CFLAGS -c rabuka_wasm.c -o rabuka_wasm.o 2>&1 | tail -1 || true
test -f rabuka_wasm.o
echo "=== sjis ==="
kos-cc $CFLAGS -c sjis_table.c -o sjis_table.o
echo "=== link ==="
kos-cc -O2 -o rabuka_opt.elf ../dc_main.o rabuka_wasm.o sjis_table.o ../wasm-rt-impl.o ../wasm-rt-mem-impl.o
echo LINK_OK
sh-elf-size rabuka_opt.elf
echo "=== baseline for comparison ==="
sh-elf-size ../rabuka_dc.elf
