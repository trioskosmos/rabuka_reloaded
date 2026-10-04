#!/bin/sh
# Baseline / after measurement for the E1 assignment.
# Counts ^FAIL (NO colon -- there are five other prefixes, FAIL: is the verdict
# runner's canonical form and is NOT the assertion-failure prefix), case
# sensitively, across stdout AND stderr. Exit code is deliberately ignored.
#
# usage: sh tools/e1_suites.sh <label>
set -u
LABEL="${1:-run}"
SUITES="parity_live_total parity_live_total2 parity_recover_live surplus_heart \
live_success_rules opponent_live_success_flow b8_live_timing \
parity_under_member parity_jidou_watch parity_jidou_watch_clusters \
parity_jidou_move_yell parity_jidou_life parity_sequential parity_integration \
parity_small_clusters parity_cce_gaps parity_zones_position parity_full_round \
parity_full_round2 parity_full_round3 parity_wave1a parity_wave1b parity_wave1c \
parity_required_hearts parity_hearts_live_start parity_card_filters \
parity_effect_conditions audit_fields mechanics"

OUT="e1_suites_${LABEL}.txt"
: > "$OUT"
TOT=0
for s in $SUITES; do
  if [ ! -f "tests/test_$s.c" ]; then
    echo "$s ABSENT" >> "$OUT"
    continue
  fi
  rm -f rb_engine_t_$s
  gcc -std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated \
      '-Wl,--stack,33554432' -o rb_engine_t_$s $(ls src/*/*.o src/*/*/*.o src/*.o 2>/dev/null | grep -v '/main\.o$') \
      tests/test_$s.c > e1_build_$s.log 2>&1
  if [ ! -f rb_engine_t_$s.exe ] && [ ! -f rb_engine_t_$s ]; then
    n=$(grep -c '^FAIL' e1_build_$s.log 2>/dev/null || echo 0)
    echo "$s BUILD_FAIL $n" >> "$OUT"
    TOT=$((TOT + n))
    continue
  fi
  ./rb_engine_t_$s.exe > e1_run_$s.log 2>&1
  n=$(cat e1_run_$s.log | grep -c '^FAIL' || true)
  v=$(cat e1_run_$s.log | grep -c '^PASSED' || true)
  f=$(cat e1_run_$s.log | grep -c '^FAILED' || true)
  echo "$s FAIL=$n PASSED_VERDICT=$v FAILED_VERDICT=$f" >> "$OUT"
  TOT=$((TOT + n))
  rm -f rb_engine_t_$s.exe rb_engine_t_$s
done
echo "LABEL=$LABEL TOTAL_FAIL=$TOT" >> "$OUT"
echo "LABEL=$LABEL TOTAL_FAIL=$TOT"