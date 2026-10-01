#!/usr/bin/env bash
# Build + run engine_c_wip tests in a private tree (audit_zones agent).
#
#   bash tools/build_audit_zones.sh                  -> build+run tests/test_audit_zones.c
#   bash tools/build_audit_zones.sh t test_a.c test_b.c ... -> build+run those tests
#
# WHY NOT `make`: the Makefile has no rule for tests/test_audit_zones.c (the
# Makefile is not this agent's to edit), and include/rabuka.h is mid-edit by a
# peer so the working tree does not currently compile against
# src/ability/condition.c. Every file EXCEPT the two this agent owns
# (src/core/zones.c, src/core/data.c) is therefore overlaid from the last
# COMMITTED revision, into the THROWAWAY tree only. `git show` is read-only:
# the working tree is never touched -- no stash, no checkout, no commit.
set -uo pipefail
AGENT=audit_zones
EC="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO="$(cd "$EC/.." && pwd)"
BUILD_ROOT=/c/Users/trios/AppData/Local/Temp/kilo/rb_isobuild
ROOT="$BUILD_ROOT/$AGENT.$$.$(date +%s)"
TREE="$ROOT/engine_c"

MODE="${1:-audit}"; shift || true
if [ "$MODE" = "t" ]; then TESTS=("$@"); else TESTS=(tests/test_audit_zones.c); fi

rm -rf "$TREE"; mkdir -p "$TREE"
( cd "$EC" && find src include tools -type f \
    \( -name '*.c' -o -name '*.h' -o -name '*.py' -o -name '*.S' \) -print0 \
  | xargs -0 -I{} cp --parents -f "{}" "$TREE/" )
cp -f "$EC/Makefile" "$TREE/"
mkdir -p "$TREE/tests"
( cd "$EC" && find tests -type f \
    \( -name '*.c' -o -name '*.h' -o -name '*.txt' -o -name '*.json' \) -print0 \
  | xargs -0 -I{} cp --parents -f "{}" "$TREE/" )
mkdir -p "$ROOT/cards/build"
cp -f "$REPO/cards/build/cards.bin" "$ROOT/cards/build/"
cp -f "$REPO/cards/build/abilities_strings.bin" "$ROOT/cards/build/"
# Some suites call rb_load("src") with no fallback, so they need the blobs there
# too. The tree is thrown away afterwards.
cp -f "$REPO/cards/build/cards.bin" "$TREE/src/" 2>/dev/null || true
cp -f "$REPO/cards/build/abilities_strings.bin" "$TREE/src/" 2>/dev/null || true

OVERLAY=$(git -C "$REPO" ls-tree -r --name-only HEAD -- engine_c_wip \
          | grep -E '\.(c|h)$' \
          | grep -vE 'engine_c_wip/(src/core/zones\.c|src/core/data\.c)$')
# BASELINE=1 also reverts this agent's two files to HEAD, which is how the
# before/after blast-radius numbers are measured.
if [ "${BASELINE:-0}" = "1" ]; then
  OVERLAY="$OVERLAY
$(git -C "$REPO" ls-tree -r --name-only HEAD -- engine_c_wip \
   | grep -E 'engine_c_wip/src/core/(zones|data)\.c$')"
fi
while read -r f; do
  rel="${f#engine_c_wip/}"
  case "$rel" in tests/*) continue ;; esac
  mkdir -p "$TREE/$(dirname "$rel")"
  git -C "$REPO" show "HEAD:$f" > "$TREE/$rel"
done <<< "$OVERLAY"

cd "$TREE"
CF="-std=gnu11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated"
make CFLAGS="$CF" >/dev/null 2>&1 || { echo "BUILD FAILED"; exit 1; }

LIB=$(ls src/ability/*.o src/ability/effects/*.o src/game/*.o src/core/*.o \
          src/core/generated/*.o src/turn/*.o src/*.o 2>/dev/null | grep -v 'src/main.o')
RC=0
for t in "${TESTS[@]}"; do
  base=$(basename "$t" .c)
  if ! gcc $CF -o "rb_$base.exe" $LIB "$t" src/test_game.c 2>&1 | grep -E 'error' ; then :; fi
  if [ ! -x "rb_$base.exe" ]; then echo "LINK FAILED: $t"; RC=1; continue; fi
  echo "########## $t"
  "./rb_$base.exe" > "rb_$base.out" 2>&1
  rc=$?
  echo "exit=$rc"
  grep -E 'cases,|tests,|assertions|passed|FAILED|ALL .*PASSED|RED' "rb_$base.out" | tail -4
  echo "  [summary] FAIL lines: $(grep -c 'FAIL' "rb_$base.out")  exit=$rc"
  [ $rc -ne 0 ] && RC=$rc
done
rm -rf "$ROOT"
exit $RC
