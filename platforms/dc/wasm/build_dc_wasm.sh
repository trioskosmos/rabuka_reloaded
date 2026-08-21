#!/bin/bash
# Full Dreamcast build: runtime + SJIS table + engine + shell + link.
#
# NOTE: this recompiles EVERYTHING including the ~20MB wasm2c-generated
# engine (rabuka_wasm.o, ~4-5 min). If you only changed dc_main.c use
# relink_dc.sh instead -- but remember relink_dc.sh does NOT recompile
# rabuka_wasm.o, so any engine/wasm change REQUIRES this script.
# (Symptom of forgetting: native test passes but the DC shows old
# behaviour, e.g. the AI's deck stayed dk:0 after the deck-alignment fix.)
set -e
source /root/kos/environ.sh >/dev/null
cd /root/dcbuild

CFLAGS="-O2 -DWASM_RT_USE_MMAP=0 -DWASM_RT_MEMCHECK_BOUNDS_CHECK -DWASM_RT_INSTALL_SIGNAL_HANDLER=0 -Istub"

echo "=== [1/4] runtime + sjis table objects ==="
kos-cc $CFLAGS -c wasm-rt-impl.c -o wasm-rt-impl.o
kos-cc $CFLAGS -c wasm-rt-mem-impl.c -o wasm-rt-mem-impl.o
kos-cc $CFLAGS -c sjis_table.c -o sjis_table.o
echo "runtime OK"

echo "=== [2/4] glue object ==="
kos-cc $CFLAGS -c dc_main.c -o dc_main.o
echo "glue OK"

echo "=== [3/4] engine (20MB generated C) - this takes a while ==="
time kos-cc $CFLAGS -c rabuka_wasm.c -o rabuka_wasm.o
echo "engine OK"

echo "=== [4/4] link ELF ==="
kos-cc -O2 -o rabuka_dc.elf dc_main.o sjis_table.o rabuka_wasm.o wasm-rt-impl.o wasm-rt-mem-impl.o
sh-elf-size rabuka_dc.elf || true
echo "ALL DONE"
