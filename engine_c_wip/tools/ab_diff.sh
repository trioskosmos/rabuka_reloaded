#!/bin/bash
# Diff the ^FAIL verdict text between two ab_run.sh output dirs, per suite.
A="$1"; B="$2"
for s in parity_sequential parity_cce_gaps parity_small_clusters parity_integration \
parity_under_member parity_zones_position parity_live_total parity_recover_live \
parity_conditional_gated parity_required_hearts parity_hearts_live_start \
parity_jidou_watch parity_jidou_watch_clusters parity_jidou_move_yell \
parity_full_round parity_full_round2 parity_full_round3 parity_wave1a \
parity_wave1b parity_wave1c parity_deck_order parity_cluster_d audit_fields \
audit_warnings parity_score_effects; do
  cat "$A/$s.out" "$A/$s.err" 2>/dev/null | grep '^FAIL' | sed 's/0x[0-9a-fA-F]\{6,\}/PTR/g' | sort > /tmp/a.txt
  cat "$B/$s.out" "$B/$s.err" 2>/dev/null | grep '^FAIL' | sed 's/0x[0-9a-fA-F]\{6,\}/PTR/g' | sort > /tmp/b.txt
  if cmp -s /tmp/a.txt /tmp/b.txt; then :; else
    echo "=== $s  (< only-in-baseline, > only-in-fix)"
    diff /tmp/a.txt /tmp/b.txt | grep -E '^[<>]' | sed 's/^/  /'
  fi
done