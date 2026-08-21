#!/bin/bash
set -e
cd /root/dcbuild/jagwasm
cp -f '/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/jaguar/wasm/native_probe.c' .
gcc -O1 -I. -Istub -DNDEBUG -DWASM_RT_USE_MMAP=0 -DWASM_RT_MEMCHECK_BOUNDS_CHECK \
    -DWASM_RT_INSTALL_SIGNAL_HANDLER=0 -o native_probe native_probe.c \
    rabuka_wasm.c wasm-rt-impl.c wasm-rt-mem-impl.c -lm 2>probe_build.log \
    || { tail -20 probe_build.log; exit 1; }
timeout 120 ./native_probe
