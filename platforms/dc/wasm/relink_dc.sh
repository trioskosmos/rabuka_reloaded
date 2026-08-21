#!/bin/bash
# Fast rebuild: ONLY the DC shell (dc_main.c) + link. Reuses the existing
# rabuka_wasm.o engine object.
#
# !! If you changed the wasm module (platforms/wasm, engine features, deck
# data via tools/bake_deck_cards.py) you MUST run build_dc_wasm.sh instead
# -- this script does not recompile rabuka_wasm.o, so you would ship the
# old engine with a new shell. That exact mistake shipped an AI player
# with an empty deck (dk:0) even though the fix was already in the tree.
set -e
source /root/kos/environ.sh >/dev/null
cd /root/dcbuild
CFLAGS="-O2 -DWASM_RT_USE_MMAP=0 -DWASM_RT_MEMCHECK_BOUNDS_CHECK -DWASM_RT_INSTALL_SIGNAL_HANDLER=0 -Istub"
kos-cc $CFLAGS -c dc_main.c -o dc_main.o
kos-cc -O2 -o rabuka_dc.elf dc_main.o sjis_table.o rabuka_wasm.o wasm-rt-impl.o wasm-rt-mem-impl.o
echo LINK_DONE
