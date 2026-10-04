#!/bin/sh
# Build + run one suite by stem against the snapshot. STRICTLY SEQUENTIAL.
set -u
s="$1"
OBJS=$(ls src/ability/*.o src/ability/effects/*.o src/core/*.o src/core/generated/*.o src/game/*.o src/turn/*.o src/*.o 2>/dev/null | grep -v 'src/main\.o$')
rm -f "rb_engine_t_$s"
cc -std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated \
   -Wl,--stack,33554432 -o "rb_engine_t_$s" $OBJS "tests/test_$s.c" || { echo "BUILD_RED $s"; exit 1; }
"./rb_engine_t_$s"