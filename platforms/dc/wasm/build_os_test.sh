#!/bin/bash
set -e
source /root/kos/environ.sh >/dev/null
cd /root/dcbuild/optbuild
CFLAGS="-Os -DWASM_RT_USE_MMAP=0 -DWASM_RT_MEMCHECK_BOUNDS_CHECK -DWASM_RT_INSTALL_SIGNAL_HANDLER=0 -Istub"
echo "=== engine (-Os) ==="
kos-cc $CFLAGS -c rabuka_wasm.c -o rabuka_os.o 2>&1 | grep -c warning || true
test -f rabuka_os.o
echo "=== link ==="
kos-cc -Os -o rabuka_os.elf ../dc_main.o rabuka_os.o sjis_table.o ../wasm-rt-impl.o ../wasm-rt-mem-impl.o
echo LINK_OK
sh-elf-size rabuka_os.elf
