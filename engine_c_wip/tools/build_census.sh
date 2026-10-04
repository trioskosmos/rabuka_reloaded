#!/bin/sh
# Build the fixer-wave census tool against the snapshot's objects. STRICTLY SEQUENTIAL.
set -eu
OBJS=$(ls src/ability/*.o src/ability/effects/*.o src/core/*.o src/core/generated/*.o src/game/*.o src/turn/*.o src/*.o 2>/dev/null | grep -v 'src/main\.o$')
rm -f rb_census_fx tools/census_fx.o
cc -std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated -c -o tools/census_fx.o tools/census_fx.c || exit 1
cc -std=c11 -O2 -Wl,--stack,33554432 -o rb_census_fx $OBJS tools/census_fx.o || exit 1
test -x rb_census_fx || { echo "census binary MISSING"; exit 1; }
echo "census built $(date +%s)"