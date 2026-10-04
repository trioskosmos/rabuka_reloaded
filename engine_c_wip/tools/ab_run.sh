#!/bin/sh
# A/B suite runner. Usage: sh tools/ab_run.sh <outdir>
# STRICTLY SEQUENTIAL. Counts "^FAIL" (NO colon, case-SENSITIVE) on stdout AND
# stderr. A suite that emits NOTHING is reported SILENT, never as passing.
set -u
OUT="$1"
mkdir -p "$OUT"
SUITES="parity_sequential parity_cce_gaps parity_small_clusters parity_integration \
parity_under_member parity_zones_position parity_live_total parity_recover_live \
parity_conditional_gated parity_required_hearts parity_hearts_live_start \
parity_jidou_watch parity_jidou_watch_clusters parity_jidou_move_yell \
parity_full_round parity_full_round2 parity_full_round3 parity_wave1a \
parity_wave1b parity_wave1c parity_deck_order parity_cluster_d audit_fields \
audit_warnings parity_score_effects"
for s in $SUITES; do
  if [ ! -f "tests/test_$s.c" ]; then
    echo "$s|NOSUCHFILE|0|0|0"
    continue
  fi
  rm -f "rb_engine_t_$s"
  cc -std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated \
     -Wl,--stack,33554432 -o "rb_engine_t_$s" $(ls src/ability/*.o src/ability/effects/*.o src/core/*.o src/core/generated/*.o src/game/*.o src/turn/*.o src/*.o 2>/dev/null | grep -v 'src/main.o') "tests/test_$s.c" >"$OUT/$s.build" 2>&1
  bc=$?
  if [ $bc -ne 0 ]; then
    echo "$s|BUILD_RED|0|0|0"
    continue
  fi
  "./rb_engine_t_$s" >"$OUT/$s.out" 2>"$OUT/$s.err"
  ec=$?
  so=$(wc -c <"$OUT/$s.out")
  se=$(wc -c <"$OUT/$s.err")
  fo=$(grep -c '^FAIL' "$OUT/$s.out" 2>/dev/null); fo=${fo:-0}
  fe=$(grep -c '^FAIL' "$OUT/$s.err" 2>/dev/null); fe=${fe:-0}
  sil=0
  if [ "$so" -le 1 ] && [ "$se" -le 1 ]; then sil=1; fi
  echo "$s|EC=$ec|FAIL_OUT=$fo|FAIL_ERR=$fe|SILENT=$sil|bytes_out=$so|bytes_err=$se"
  rm -f "rb_engine_t_$s"
done