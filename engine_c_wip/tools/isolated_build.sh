#!/usr/bin/env bash
# Isolated out-of-tree build for engine_c_wip.
#
# Usage:  bash tools/isolated_build.sh <agent-id> [make target/args...]
#   e.g.  bash tools/isolated_build.sh draw t T=parity_draw_until_count
#         bash tools/isolated_build.sh live t T=live_card_zone_movement
#         bash tools/isolated_build.sh x run-all-tests
#
# WHY: several translation agents run concurrently in the SAME working
# directory. `make` writes fixed-path objects (src/**/*.o), so concurrent
# in-tree builds corrupt each other. This script copies the sources into a
# private build root per agent and runs make there, so .o files and binaries
# never collide. Agents still EDIT the real files in engine_c_wip/; only the
# build is relocated. Sources are re-copied on every invocation, so the build
# always reflects current on-disk sources.
#
# NOTE: run artifacts are deliberately NOT copied (tests/baton_touch.log alone
# is 37 GB, plus *.exe and _*.out/_*.err). Only sources, headers, scripts and
# test fixtures are taken.
set -euo pipefail

AGENT="${1:?usage: isolated_build.sh <agent-id> [make args...]}"
shift || true

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EC="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO="$(cd "$EC/.." && pwd)"

# The build root is made UNIQUE PER INVOCATION. Several agents (and parallel
# runs of the same agent) share this directory, and a shared root meant one
# run's `rm -rf` deleted another's .o files mid-make ("No rule to make target
# src/ability/vm.o", "can't create src/turn/phase.o").
BUILD_ROOT="${ISOLATED_BUILD_ROOT:-/c/Users/trios/AppData/Local/Temp/kilo/rb_isobuild}"
ROOT="$BUILD_ROOT/$AGENT.$$.$(date +%s)"
TREE="$ROOT/engine_c"

# Prune stale roots so parallel fan-outs cannot fill the disk. Anything older
# than 90 minutes is abandoned by definition (a build does not take that long).
find "$BUILD_ROOT" -maxdepth 1 -mindepth 1 -type d -mmin +90 -exec rm -rf {} + 2>/dev/null || true

cleanup() { rm -rf "$ROOT"; }
trap cleanup EXIT

rm -rf "$TREE"
mkdir -p "$TREE"

# Engine sources, headers and porting tools.
( cd "$EC" && find src include tools -type f \
    \( -name '*.c' -o -name '*.h' -o -name '*.py' -o -name '*.S' \) -print0 \
  | xargs -0 -I{} cp --parents -f "{}" "$TREE/" )
cp -f "$EC/Makefile" "$TREE/"

# Test sources + scenario fixtures. .log/.out/.err are run artifacts, skipped.
mkdir -p "$TREE/tests"
( cd "$EC" && find tests -type f \
    \( -name '*.c' -o -name '*.h' -o -name '*.txt' -o -name '*.json' \) -print0 \
  | xargs -0 -I{} cp --parents -f "{}" "$TREE/" )

# Card blobs the test binaries load. The Makefile resolves these as
# ../cards/build/*, so they must sit beside the copied tree.
mkdir -p "$ROOT/cards/build"
cp -f "$REPO/cards/build/cards.bin" "$ROOT/cards/build/"
cp -f "$REPO/cards/build/abilities_strings.bin" "$ROOT/cards/build/"

cd "$TREE"
make "$@"
echo "[isolated_build] agent=$AGENT tree=$TREE goal=${*:-all}"