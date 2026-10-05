#!/bin/sh
nm -g src/core/game_state_core.o | grep -oE 'rb_gsc_[a-z0-9_]+' | sort -u > zzlogs/gsc_nm.txt
grep -oE '^[A-Za-z_][A-Za-z0-9_ ]*\**\s*\b(rb_gsc_[a-z0-9_]+)\s*\(' src/core/game_state_core.c \
  | grep -oE 'rb_gsc_[a-z0-9_]+' | sort -u > zzlogs/gsc_re.txt
echo "nm-defined: $(wc -l < zzlogs/gsc_nm.txt)   regex-found: $(wc -l < zzlogs/gsc_re.txt)"
echo "=== in nm but NOT matched by my regex ==="
comm -23 zzlogs/gsc_nm.txt zzlogs/gsc_re.txt
echo "=== of those, declared in rabuka.h? ==="
comm -23 zzlogs/gsc_nm.txt zzlogs/gsc_re.txt | while read -r s; do
  if grep -qE "\b$s[[:space:]]*\(" include/rabuka.h; then echo "DECLARED $s"; else echo "MISSING  $s"; fi
done