#!/bin/sh
set -u
cc -std=c11 -O2 -Iinclude -Isrc -Isrc/core/generated -o dump_cost \
  tools/dump_cost.c \
  $(ls src/ability/*.o src/ability/effects/*.o src/core/*.o src/core/generated/*.o src/game/*.o src/turn/*.o src/*.o 2>/dev/null | grep -v 'src/main.o')