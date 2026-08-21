#!/bin/bash
# Rebuild shell + sjis table, link, package, deploy.
set -e
source /root/kos/environ.sh >/dev/null
cd /root/dcbuild
CFLAGS="-O2 -DWASM_RT_USE_MMAP=0 -DWASM_RT_MEMCHECK_BOUNDS_CHECK -DWASM_RT_INSTALL_SIGNAL_HANDLER=0 -Istub"
kos-cc $CFLAGS -c dc_main.c -o dc_main.o
kos-cc $CFLAGS -c sjis_table.c -o sjis_table.o
kos-cc -O2 -o rabuka_dc.elf dc_main.o sjis_table.o rabuka_wasm.o wasm-rt-impl.o wasm-rt-mem-impl.o
/root/sh-elf/bin/sh-elf-strip rabuka_dc.elf -o rabuka_stripped.elf
cd /root/mkdcdisc/build
./mkdcdisc -e /root/dcbuild/rabuka_stripped.elf -n RABUKA -o /root/dcbuild/rabuka.cdi
cp /root/dcbuild/rabuka.cdi /mnt/c/Emulators/Flycast/games/rabuka.cdi
echo DEPLOYED
