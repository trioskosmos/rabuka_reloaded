#!/bin/sh
# choice.c measurement runner. STRICTLY SEQUENTIAL.
# Usage: sh tools/choice_measure.sh <label>
# Emits per suite: label|suite|BUILD_RED|OK|EC|F fail-lines|C cases|SILENT
# Counts are case-SENSITIVE with \r? (harness writes CRLF) on stdout AND stderr.
set -u
LBL="$1"
OUT="choicemeas_${LBL}"
mkdir -p "$OUT"
SUITES="parity_cce_gaps parity_sequential parity_small_clusters parity_under_member \
parity_integration parity_search_reorder parity_jidou_watch parity_jidou_watch_clusters \
parity_jidou_move_yell parity_zones_position parity_zone_source parity_live_total \
parity_recover_live parity_restriction parity_wave1a parity_wave1b parity_wave1c \
parity_full_round parity_full_round2 parity_full_round3 parity_choice_routing \
parity_queue_resume parity_int_gaps mechanics alias_ownership"
for s in $SUITES; do
  if [ ! -f "tests/test_$s.c" ]; then echo "$LBL|$s|NOSUCHFILE|0|0|0"; continue; fi
  rm -f "rb_engine_t_$s"
  cc -std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated \
     -Wl,--stack,33554432 -o "rb_engine_t_$s" $(ls src/ability/*.o src/ability/effects/*.o src/core/*.o src/core/generated/*.o src/game/*.o src/turn/*.o src/*.o 2>/dev/null | grep -v 'src/main.o') "tests/test_$s.c" >"$OUT/$s.build" 2>&1
  bc=$?
  if [ $bc -ne 0 ]; then echo "$LBL|$s|BUILD_RED|0|0|0"; continue; fi
  "./rb_engine_t_$s" >"$OUT/$s.out" 2>"$OUT/$s.err"
  ec=$?
  so=$(wc -c <"$OUT/$s.out"); se=$(wc -c <"$OUT/$s.err")
  f=$( (cat "$OUT/$s.out"; cat "$OUT/$s.err") | grep -c '^FAIL:' )
  cs=$( (cat "$OUT/$s.out"; cat "$OUT/$s.err") | grep -cE '[A-Za-z0-9_]+: [0-9]+ (cases|assertion\(s\))' )
  sil=0; if [ "$so" -le 1 ] && [ "$se" -le 1 ]; then sil=1; fi
  echo "$LBL|$s|OK|EC=$ec|FAIL=$f|CASE=$cs|SILENT=$sil"
  rm -f "rb_engine_t_$s"
done
