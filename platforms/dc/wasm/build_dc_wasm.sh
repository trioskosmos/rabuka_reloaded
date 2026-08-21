#!/bin/bash
# Build the Rabuka Dreamcast port: wasm2c-transpiled engine + KOS glue.
set -e
source /root/kos/environ.sh >/dev/null
cd /root/dcbuild

CFLAGS="-O2 -DWASM_RT_USE_MMAP=0 -DWASM_RT_MEMCHECK_BOUNDS_CHECK -DWASM_RT_INSTALL_SIGNAL_HANDLER=0 -Istub"

echo "=== [1/4] runtime objects ==="
kos-cc $CFLAGS -c wasm-rt-impl.c -o wasm-rt-impl.o
kos-cc $CFLAGS -c wasm-rt-mem-impl.c -o wasm-rt-mem-impl.o
echo "runtime OK"

echo "=== [2/4] glue object ==="
kos-cc $CFLAGS -c dc_main.c -o dc_main.o
echo "glue OK"

echo "=== [3/4] engine (20MB generated C) - this takes a while ==="
time kos-cc $CFLAGS -c rabuka_wasm.c -o rabuka_wasm.o
echo "engine OK"

echo "=== [4/4] link ELF ==="
kos-cc -O2 -o rabuka_dc.elf dc_main.o rabuka_wasm.o wasm-rt-impl.o wasm-rt-mem-impl.o
sh-elf-size rabuka_dc.elf || true
echo "ALL DONE"
