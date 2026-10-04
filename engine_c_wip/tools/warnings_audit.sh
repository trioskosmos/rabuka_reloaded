#!/bin/sh
# tools/warnings_audit.sh -- cluster W's bug-predicting-warning gate.
#
# Compiles every engine source with ONLY the bug-predicting classes enabled and
# reports each diagnostic as `file:line:-Wclass`. It is deliberately NOT wired
# into `make all`, `make t` or `make run-all-tests`: nothing that builds today
# can be made to fail by this script, and nothing here blocks an agent.
#
#   usage: sh tools/warnings_audit.sh "<flags>" "<baseline pins>"
#
# Baseline pins are whitespace-separated `file:line:-Wclass:row` records. They
# MUST be whitespace-separated and not newline-separated: the Makefile builds
# WARN_AUDIT_BASELINE with `:=` and backslash continuations, which collapse to a
# single line, so a newline reader sees ONE record and pins nothing. (Measured:
# the first version of this script reported "baseline pins = 1" against eight
# pins and called all eight of them NEW.)
#
# Exit 0 when every diagnostic is pinned, 1 when one is not or when a pin has
# gone stale. A pin that outlives its warning is a lie in the Makefile comment,
# so a landed fix must delete its pin.
set -u

FLAGS=${1:-}
BASELINE=${2:-}

if [ -z "$FLAGS" ]; then
    echo "usage: sh tools/warnings_audit.sh \"<flags>\" \"<pins>\"" >&2
    exit 2
fi

CC=${CC:-gcc}
CFLAGS_BASE="-std=c11 -O2 -Iinclude -Isrc -Isrc/core/generated"

# Every translation unit `make all` builds, minus main.c (a test supplies its
# own main) and minus the platform-only web_server.c.
SRC_LIST=$(sed -n 's/^SRC  *:= *//p' Makefile | tr ' ' '\n' | grep '\.c$' | grep -v '/main\.c$')

TMP=$(mktemp -d 2>/dev/null || echo /tmp/warn_audit_$$)
mkdir -p "$TMP"
trap 'rm -rf "$TMP"' EXIT INT TERM

LOG="$TMP/diag.txt"
: > "$LOG"

# `-fsyntax-only` is NOT enough: -Wrestrict, -Warray-bounds and
# -Wstringop-overflow are emitted by the optimizer, so a syntax-only pass
# reports a clean tree for the one file that needs checking. Measured on
# src/core/data.c: 3 -Wrestrict under -c, ZERO under -fsyntax-only. Compile to a
# throwaway object instead.
for f in $SRC_LIST; do
    [ -f "$f" ] || continue
    # shellcheck disable=SC2086
    $CC $CFLAGS_BASE $FLAGS -c -o "$TMP/$(echo "$f" | tr / _).o" "$f" 2>>"$LOG" || true
done

# `file:line:col: warning: ... [-Wclass]` -> `file:line:-Wclass`
sed -n 's/^\([^:]*\):\([0-9][0-9]*\):[0-9][0-9]*: warning:.*\[-\(W[a-z0-9=-]*\)\]/\1:\2:-\3/p' \
    "$LOG" | sort -u > "$TMP/found.txt"

FOUND=$(wc -l < "$TMP/found.txt" | tr -d ' ')
PINCOUNT=0
for pin in $BASELINE; do PINCOUNT=$((PINCOUNT + 1)); done

echo "warnings-audit: flags = $FLAGS"
echo "warnings-audit: bug-predicting diagnostics found = $FOUND, baseline pins = $PINCOUNT"

if [ "$FOUND" -eq 0 ]; then
    if [ "$PINCOUNT" -ne 0 ]; then
        echo "warnings-audit: FAIL -- every pin is stale (nothing fires any more)."
        for pin in $BASELINE; do
            echo "  STALE    $(echo "$pin" | cut -d: -f1-3)  (cluster W row $(echo "$pin" | cut -d: -f4))"
        done
        exit 1
    fi
    echo "warnings-audit: PASS -- no bug-predicting warning class is present."
    exit 0
fi

NEW=0; STALE=0; STILL=0
: > "$TMP/report.txt"
while IFS= read -r d; do
    [ -n "$d" ] || continue
    row=""
    for pin in $BASELINE; do
        case "$pin" in
            "$d":*) row=$pin; break ;;
        esac
    done
    if [ -n "$row" ]; then
        STILL=$((STILL + 1))
        printf '  BASELINE %-58s cluster-W row %s\n' "$d" "$(echo "$row" | cut -d: -f4)" \
            >> "$TMP/report.txt"
    else
        NEW=$((NEW + 1))
        printf '  NEW      %s\n' "$d" >> "$TMP/report.txt"
    fi
done < "$TMP/found.txt"

for pin in $BASELINE; do
    key=$(echo "$pin" | cut -d: -f1-3)
    if ! grep -q -F -x "$key" "$TMP/found.txt" 2>/dev/null; then
        STALE=$((STALE + 1))
        printf '  STALE    %-58s pinned but no longer emitted -- delete the pin\n' "$key" \
            >> "$TMP/report.txt"
    fi
done

cat "$TMP/report.txt"
echo "warnings-audit: baseline-still-present=$STILL  NEW=$NEW  stale-pins=$STALE"

if [ "$NEW" -gt 0 ]; then
    echo "warnings-audit: FAIL -- $NEW bug-predicting diagnostic(s) outside the baseline."
    echo "warnings-audit: add a cluster-W row to PARITY_CHECKLIST.md and either fix it"
    echo "warnings-audit: or prove it benign in a comment that says WHY. Do not silence."
    exit 1
fi
if [ "$STALE" -gt 0 ]; then
    echo "warnings-audit: FAIL -- $STALE pin(s) no longer fire. The fix landed; delete the pin"
    echo "warnings-audit: from WARN_AUDIT_BASELINE in the Makefile."
    exit 1
fi
echo "warnings-audit: PASS -- $STILL diagnostic(s), all pinned; 0 new."
exit 0
