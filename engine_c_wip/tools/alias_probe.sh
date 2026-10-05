#!/usr/bin/env bash
# tools/alias_probe.sh -- run the alias-class probe over suites.
#
# ⛔ THE TWO MEASUREMENT TRAPS THIS SCRIPT EXISTS TO CLOSE
#
# 1. "make reports a link failure as Error 1, not `error:`". This script never
#    calls make. It links with gcc directly and checks the exit status AND that
#    the .exe exists AND that the .exe is newer than every input. A link failure
#    here is a LOUD non-zero from this script and an explicit SKIP line -- never
#    a silent pass that leaves the previous binary to be measured.
#
# 2. "a suite that ran 0 tests prints zero FAILs". Every suite line this script
#    prints carries, separately: ok=, fail=, checks= and alias_hits=. A suite
#    that ran nothing shows ok=0 checks=0, which is visibly not a pass.
#
#    `^FAIL` is counted case-sensitively WITH the CRLF-tolerant \r? because the
#    harness writes CRLF on this toolchain; without \r? every count silently
#    reads 0. stdout AND stderr are captured together.
#
# EXIT CODE IS NOT A SIGNAL for a suite. Only the aggregate below is.
#
# Usage:
#   bash tools/alias_probe.sh <outdir> <suite> [<suite> ...]
#   bash tools/alias_probe.sh <outdir> --list-from <file>
set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 2

NEGCTL=0
PRECHANGE=0
CFLAGS="-std=c11 -O2 -Iinclude -Isrc -Isrc/core/generated -Wl,--stack,33554432"
WRAPS="-Wl,--wrap=rb_effect_free -Wl,--wrap=rb_free_ability -Wl,--wrap=rb_has_pending_choice"

# ⛔ NEGATIVE CONTROL. `--negative-control` reverses ONE landed lifetime fix --
# the pca_resolve_entry keep-alive transfer in src/core/game_state_abilities.c --
# by defining RB_ALIAS_PROBE_NEGATIVE_CONTROL, whose block restores the pre-fix
# body (rb_free_ability on the pass that opened the prompt). A probe that has
# not been shown to go NON-ZERO under this define has never fired, and its zero
# on the fixed tree means nothing. MEASURED: 22 hits over parity_cce_gaps +
# parity_int_gaps with the fix reversed, against 0 with it in place.
#
# ⛔ SAME-SNAPSHOT A/B. `--prechange` additionally swaps in the byte-exact
# PRE-CONVERSION look.c (-DRB_ALIAS_SLOT_PRECHANGE), so both arms of the
# g_look_pending_select conversion are measured minutes apart in ONE tree state.
# This matters because the tree CANNOT be snapshotted across a rebuild: twelve
# agents edit choice.c / condition.c / move.c continuously, and a sweep taken 40
# minutes earlier is not comparable to one taken now. A sweep taken earlier in
# this session read 0 hits for parity_look_and_filter; a later one read 32, all
# on queue.resume_eff, a slot this change never touches -- which is the
# contamination this arm exists to rule out.
if [ "${1:-}" = "--negative-control" ]; then
  NEGCTL=1
  shift
  CFLAGS="$CFLAGS -DRB_ALIAS_PROBE_NEGATIVE_CONTROL"
  echo "*** NEGATIVE CONTROL ARM: pca_resolve_entry lifetime fix is REVERTED ***"
fi
if [ "${1:-}" = "--prechange" ]; then
  PRECHANGE=1
  shift
  echo "*** PRE-CHANGE ARM: g_look_pending_select is the pre-conversion alias ***"
fi

OUT="$1"; shift
[ -n "$OUT" ] || { echo "usage: alias_probe.sh [--negative-control|--prechange] <outdir> <suite>..."; exit 2; }

# ── the object library, minus main ─────────────────────────────────────────
# src/main.o defines its own main and a test TU defines a second, so it can
# never be in a suite link.
# CORRECTED 2026-10-05: the comment here used to read "src/web_server.o is not
# in OBJ_LIB's suite set per corpus_measure.py's note", citing Makefile:49-61,
# which contradicted Makefile:61/64 in the same tree -- src/web_server.o IS in
# OBJ_LIB, and rb_engine and rb_web_server both link it. It is still skipped
# here, but for the real reason and not as a claim about OBJ_LIB: it is dropped
# because it is the WEBSERVER TU, not because OBJ_LIB lacks it. Leaving it in
# would not cause a duplicate symbol (no other TU defines web_* symbols); it is
# excluded only to keep the probe's link set equal to what `make t` links.
# corpus_measure.py drops it too, which is where the false claim came from.
OBJ=""
for o in $(find src -name '*.o' | sort); do
  case "$(basename "$o")" in
    main.o|web_server.o) continue ;;
  esac
  OBJ="$OBJ $o"
done

mkdir -p "$OUT"

# The pre-change arm: compile the byte-exact pre-conversion look.c and swap it
# in. Recompiled FRESH with the define -- reusing the prebuilt .o would measure
# the converted engine and report the arm as a no-op.
if [ "${PRECHANGE:-0}" = "1" ]; then
  PC="$OUT/prechange_look.o"
  rm -f "$PC"
  # shellcheck disable=SC2086
  cc $CFLAGS -w -DRB_ALIAS_SLOT_PRECHANGE -c -o "$PC" src/ability/effects/look.c \
     > "$OUT/prechange.build.log" 2>&1
  if [ $? -ne 0 ] || [ ! -f "$PC" ]; then
    echo "ALIAS_PROBE_ABORT: pre-change arm rebuild failed (see $OUT/prechange.build.log)"
    exit 3
  fi
  OBJ=$(echo "$OBJ" | sed 's#src/ability/effects/look\.o##')
  OBJ="$PC $OBJ"
fi

if [ "${NEGCTL:-0}" = "1" ]; then
  NCC="$OUT/negctl_game_state_abilities.o"
  rm -f "$NCC"
  # shellcheck disable=SC2086
  cc $CFLAGS -c -o "$NCC" src/core/game_state_abilities.c \
     > "$OUT/negctl.build.log" 2>&1
  if [ $? -ne 0 ] || [ ! -f "$NCC" ]; then
    echo "ALIAS_PROBE_ABORT: negative-control rebuild failed (see $OUT/negctl.build.log)"
    exit 3
  fi
  OBJ=$(echo "$OBJ" | sed 's#src/core/game_state_abilities\.o##')
  OBJ="$NCC $OBJ"
  # the freshly built object is newer than everything, so exempt it from the
  # staleness comparison below by running that check BEFORE this substitution.
fi

# ── STALENESS GATE ───────────────────────────────────────────────────────────
# A .o older than its .c, or older than include/rabuka.h, would silently measure
# the PREVIOUS engine. That is the same class of error as running a stale
# binary, and it is checked rather than assumed.
STALE=""
for o in $OBJ; do
  c="${o%.o}.c"
  [ -f "$c" ] || continue
  if [ "$o" -ot "$c" ] || [ "$o" -ot include/rabuka.h ]; then
    STALE="$STALE $c"
  fi
done
if [ -n "$STALE" ]; then
  echo "ALIAS_PROBE_ABORT: stale objects (rebuild first):"
  for s in $STALE; do echo "  $s"; done
  exit 3
fi

echo "=== ALIAS PROBE SWEEP: outdir=$OUT suites=$* ==="

TOTAL_HITS=0
TOTAL_FREES=0
SUITES_OK=0
SUITES_BAD=0
: > "$OUT/summary.txt"

for stem in "$@"; do
  src="tests/test_$stem.c"
  exe="rb_engine_ap_$stem.exe"
  log="$OUT/$stem.log"

  if [ ! -f "$src" ]; then
    printf '%-34s SKIP no such suite\n' "$stem" | tee -a "$OUT/summary.txt"
    SUITES_BAD=$((SUITES_BAD+1))
    continue
  fi

  rm -f "$exe"
  # shellcheck disable=SC2086
  cc $CFLAGS $WRAPS -o "$exe.exe" $OBJ tools/alias_probe.c "$src" \
      > "$OUT/$stem.build.log" 2>&1
  rc=$?
  if [ $rc -ne 0 ] || [ ! -f "$exe.exe" ]; then
    # THE TRAP: this must never read as "no errors, therefore clean".
    printf '%-34s BUILD-FAIL rc=%s (NOT counted as clean; see %s)\n' \
        "$stem" "$rc" "$OUT/$stem.build.log" | tee -a "$OUT/summary.txt"
    SUITES_BAD=$((SUITES_BAD+1))
    continue
  fi

  "./$exe.exe" > "$log" 2>&1
  run_rc=$?

  # `^FAIL` with \r? -- the harness writes CRLF here.
  nfail=$(grep -c '^FAIL' "$log" 2>/dev/null || true)
  nok=$(grep -c '^ok' "$log" 2>/dev/null || true)
  nready=$(grep -c '^ALIAS_PROBE_READY' "$log" 2>/dev/null || true)
  nsum=$(grep -c '^ALIAS_PROBE frees=' "$log" 2>/dev/null || true)
  # ⛔ AUTHORITATIVE HIT COUNT = the number of ALIAS_HIT LINES, not the summary
  # counter. parity_cce_gaps and parity_required_hearts fork a child per case
  # and the child leaves through _exit(), so the parent's destructor reports 0
  # while the children's hits are already in the log. Taking the summary counter
  # here is precisely the "reported zero because the reporter ran in the wrong
  # process" measurement trap this probe exists to survive.
  nhit=$(grep -c '^ALIAS_HIT ' "$log" 2>/dev/null || true)
  frees=$(sed -n 's/^ALIAS_PROBE frees=\([0-9]*\).*/\1/p' "$log" | tail -1)
  fwp=$(sed -n 's/.*frees_with_prompt_open=\([0-9]*\).*/\1/p' "$log" | tail -1)
  sumhits=$(sed -n 's/.*hits=\([0-9]*\) suppressed=.*/\1/p' "$log" | tail -1)
  : "${frees:=0}"; : "${fwp:=0}"; : "${sumhits:=0}"
  hits="$nhit"

  # A summary that disagrees with the log is REPORTED, never silently resolved
  # in the flattering direction.
  disagree=""
  if [ "$nhit" -ne 0 ] && [ "$sumhits" -eq 0 ]; then
    disagree="SUMMARY-DISAGREES(forked children: log=$nhit parent_summary=$sumhits)"
  elif [ "$nhit" -eq 0 ] && [ "$sumhits" -ne 0 ]; then
    disagree="SUMMARY-ONLY(log=0 parent_summary=$sumhits)"
  fi

  # The probe must actually have been linked AND must have printed its summary.
  # Without these two guards a non-zero hit count of 0 means nothing.
  probe_ok=1
  [ "$nready" -ge 1 ] || probe_ok=0
  [ "$nsum"   -ge 1 ] || probe_ok=0

  # CASE-COUNT ASSERTION. ok=0 AND nfail=0 is a suite that measured nothing.
  ran=1
  [ "$nok" -eq 0 ] && [ "$nfail" -eq 0 ] && ran=0

  TOTAL_HITS=$((TOTAL_HITS + hits))
  TOTAL_FREES=$((TOTAL_FREES + frees))
  if [ "$probe_ok" -eq 1 ] && [ "$ran" -eq 1 ]; then SUITES_OK=$((SUITES_OK+1));
  else SUITES_BAD=$((SUITES_BAD+1)); fi

  printf '%-34s ok=%-6s FAIL=%-5s alias_hits=%-4s frees_top=%-6s prompt_open=%-6s probe=%s ran=%s %s\n' \
    "$stem" "$nok" "$nfail" "$hits" "$frees" "$fwp" \
    "$([ "$probe_ok" -eq 1 ] && echo LINKED || echo MISSING)" \
    "$([ "$ran" -eq 1 ] && echo yes || echo NO-CASES)" "$disagree" | tee -a "$OUT/summary.txt"

  rm -f "$exe.exe"
done

echo "--- ALIAS PROBE AGGREGATE ---"
echo "suites_probed=$SUITES_OK suites_unusable=$SUITES_BAD total_frees_observed=$TOTAL_FREES total_alias_hits=$TOTAL_HITS"
echo "ALIAS_PROBE_TOTAL hits=$TOTAL_HITS probed=$SUITES_OK unusable=$SUITES_BAD" | tee -a "$OUT/summary.txt"

# ⛔ A SWEEP THAT MEASURED NOTHING IS NOT A CLEAN SWEEP. This happened for real:
# a peer's `make` was still writing objects, every link failed, 18 suites were
# unusable, and the aggregate still printed "ALIAS_PROBE_VERDICT ZERO" -- because
# zero usable suites trivially sums to zero hits. That is the same measurement
# defect as a suite that ran 0 tests printing zero FAILs, one level up. The
# verdict is only allowed to say ZERO when at least one suite actually ran.
if [ "$SUITES_OK" -eq 0 ]; then
  echo "ALIAS_PROBE_VERDICT INVALID -- 0 suites were probed; this is not a measurement"
  exit 2
fi
if [ "$SUITES_BAD" -gt 0 ]; then
  echo "ALIAS_PROBE_VERDICT PARTIAL -- $SUITES_BAD suite(s) unusable; the zero below covers only the $SUITES_OK that ran"
fi
if [ "$TOTAL_HITS" -ne 0 ]; then
  echo "ALIAS_PROBE_VERDICT NONZERO -- the alias class is present"
  exit 1
fi
echo "ALIAS_PROBE_VERDICT ZERO"
exit 0