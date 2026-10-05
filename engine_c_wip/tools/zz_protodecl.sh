#!/bin/sh
# Which rb_gsc_* symbols are DEFINED but have no prototype in include/rabuka.h
defs=$(grep -oE '^[A-Za-z_][A-Za-z0-9_ ]*\**\s*\b(rb_gsc_[a-z0-9_]+)\s*\(' src/core/game_state_core.c \
        | grep -oE 'rb_gsc_[a-z0-9_]+' | sort -u)
n=0; m=0
for s in $defs; do
  n=$((n+1))
  if grep -qE "\b$s\s*\(" include/rabuka.h; then
    printf 'DECLARED  %s\n' "$s"
  else
    printf 'MISSING   %s\n' "$s"
    m=$((m+1))
  fi
done
printf '\ntotal defined=%s missing=%s\n' "$n" "$m"
