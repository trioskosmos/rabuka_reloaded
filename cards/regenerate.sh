#!/usr/bin/env bash
# Regenerate all ability pipeline artifacts from source data.
# Run from the repository root: bash cards/regenerate.sh
#
# This script updates:
#   cards/abilities.json          (parser output)
#   cards/build/abilities.bin     (bytecode)
#   cards/build/abilities_gen.rs  (generated Rust constants)
#   cards/build/generation_manifest.json
#   engine/src/ability/abilities_gen.rs (checked-in copy)
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Step 1: Extract abilities from cards.json ==="
cd "$REPO_ROOT"
python cards/ability_extraction/extract_card_abilities.py

echo ""
echo "=== Step 2: Compile abilities to bytecode + Rust ==="
python cards/compile_abilities.py

echo ""
echo "=== Step 3: Validate schema ↔ compiler ↔ engine ==="
python cards/validate_schema.py

echo ""
echo "=== Step 4: Verify tests pass ==="
cargo test --features bytecode_abilities -- bytecode 2>&1 | tail -5

echo ""
echo "=== Done. Regeneration complete. ==="
