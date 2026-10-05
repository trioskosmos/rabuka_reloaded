#!/bin/sh
# baseline/after runner for the six decode-layer suites + friends.
# Usage: sh zzm6_run.sh <tag>
# Writes zz_mine.<tag>.<suite>.{out,err} as RAW BYTES (sh 2> , never PowerShell).
tag="$1"
[ -n "$tag" ] || { echo "usage: zzm6_run.sh <tag>"; exit 2; }
suites="parity_card_score parity_group_names decode_skew decode_fallback artifacts_consistent describe_golden parity_enums vm_decode parity_x_class parity_fw_effects parity_condition_and_effect parity_card_identity parity_sequential parity_small_clusters mechanics harness_self decode_bytecode_shape null_effect_guard"
mkdir -p "zzs/$tag"
for s in $suites; do
  if [ ! -f "tests/test_$s.c" ]; then
    echo "SUITE $s: NO tests/test_$s.c" >> "zzs/$tag/summary.txt"
    continue
  fi
  sh -c "make --no-print-directory t-nogate T=$s >zzs/$tag/$s.out 2>zzs/$tag/$s.err"
  echo "=== SUITE $s (exit $?)" >> "zzs/$tag/summary.txt"
  # case counts: every distinct summary shape, grepped by shape not by one key
  grep -aiE '^[A-Za-z].*[0-9]+ *(checks?|cases?|tests?|cards?|abilities|effects|pairs|rows|checks)/' "zzs/$tag/$s.out" "zzs/$tag/$s.err" 2>/dev/null >> "zzs/$tag/summary.txt"
  grep -aoE '(RED|GREEN|YELLOW): [0-9]+ checks, [0-9]+ failures' "zzs/$tag/$s.out" "zzs/$tag/$s.err" 2>/dev/null | head -5 >> "zzs/$tag/summary.txt"
  grep -acE '^\^FAIL' "zzs/$tag/$s.out" "zzs/$tag/$s.err" 2>/dev/null >> "zzs/$tag/summary.txt"
  grep -aE '^FAILED:|^PASSED:|^INFO:|^VERDICT' "zzs/$tag/$s.out" "zzs/$tag/$s.err" 2>/dev/null | head -8 >> "zzs/$tag/summary.txt"
done
echo "done $tag"