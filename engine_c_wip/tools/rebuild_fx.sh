#!/bin/sh
# Rebuild the objects whose sources this wave touched, from scratch, and report
# the moved mtimes. Avoids the STALE-LINK trap: the object is deleted first, so
# make can never skip the rebuild because a restored source looks older.
set -e
for f in src/ability/condition.o src/ability/choice.o src/ability/cost.o \
         src/core/game_state_abilities.o; do
  rm -f "$f"
  echo "removed $f"
done
make -j1 all 2>&1 | tail -3
for f in src/ability/condition.o src/ability/choice.o src/ability/cost.o \
         src/core/game_state_abilities.o; do
  ls -la --time-style=+%s "$f"
done