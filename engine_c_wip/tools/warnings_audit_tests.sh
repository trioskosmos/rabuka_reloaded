#!/bin/sh
# tools/warnings_audit_tests.sh -- cluster W's TEST-build gate.
#
# Compiles every tests/test_*.c with the two classes that predicted the SIGSEGV
# this tree once carried turned into errors. Deliberately ONLY those two: the
# engine build still emits 113 warnings and a blanket -Werror anywhere would
# block every agent in the repo.
#
#   -Werror=array-bounds         gcc 15 routes BOTH the classic out-of-range
#                                 subscript AND a memcpy whose bound exceeds the
#                                 destination to this class (measured: the
#                                 memcpy shape reports -Warray-bounds=2, not
#                                 -Wstringop-overflow).
#   -Werror=maybe-uninitialized  the reading an uninitialised value on a path
#                                 that skipped its writer.
#
# Usage: sh tools/warnings_audit_tests.sh "<baseline>"
# A suite that fails to compile is reported by name and makes the gate fail --
# that IS the point: a suite that will not build is not a suite that passes.
#
# Baseline records are whitespace-separated `file[:line]:row`. Same ratchet shape
# as tools/warnings_audit.sh, for the same reason: `tests/**` is contested, so
# the one known-red source is PINNED with its cluster-W row rather than left to
# make the target red for whoever runs it. A suite that goes green must have its
# pin deleted, and a suite that goes red must add one.
set -u
BASELINE=${1:-}

CC=${CC:-gcc}
CFLAGS_BASE="-std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated"
FLAGS="-Werror=array-bounds -Werror=maybe-uninitialized"

TMP=$(mktemp -d 2>/dev/null || echo /tmp/warn_audit_t_$$)
mkdir -p "$TMP"
trap 'rm -rf "$TMP"' EXIT INT TERM

PINS=0
for pin in $BASELINE; do PINS=$((PINS + 1)); done

n=0; red=0; pinned=0
: > "$TMP/report.txt"
for f in tests/test_*.c; do
    [ -f "$f" ] || continue
    n=$((n + 1))
    # shellcheck disable=SC2086
    if $CC $CFLAGS_BASE $FLAGS -c -o "$TMP/t.o" "$f" > "$TMP/o" 2>&1; then
        # a pinned suite that now builds must lose its pin
        for pin in $BASELINE; do
            case "$pin" in
                "$f":*) printf '  STALE  %-44s pinned but now compiles\n' "$f" \
                        >> "$TMP/report.txt" ;;
            esac
        done
        continue
    fi
    is_pinned=0
    for pin in $BASELINE; do
        case "$pin" in "$f":*) is_pinned=1 ;; esac
    done
    if [ "$is_pinned" -eq 1 ]; then
        pinned=$((pinned + 1))
        for pin in $BASELINE; do
            case "$pin" in
                "$f":*) row=$(echo "$pin" | sed 's/^[^:]*://') ;;
            esac
        done
        printf '  BASELINE %-44s cluster-W row %s\n' "$f" "$row" >> "$TMP/report.txt"
    else
        red=$((red + 1))
        printf '  RED    %-44s\n' "$f" >> "$TMP/report.txt"
        grep 'error:' "$TMP/o" | head -n 2 >> "$TMP/report.txt"
    fi
done
cat "$TMP/report.txt"
echo "warnings-audit-test: $n test sources, $red red, $pinned pinned  (flags: $FLAGS)"
[ "$red" -eq 0 ] || exit 1
exit 0

