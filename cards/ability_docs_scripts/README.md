# Ability Documentation Scripts

Tools for analyzing `cards/abilities.json` and tracking test coverage for card abilities.

## Quick Start

```bash
# Single command — regenerates coverage docs (canonical):
python ../test_inventory.py
# (or from repo root: python cards/test_inventory.py)

# Invert abilities.json — map JSON back to source texts (parser introspection):
python ability_docs_scripts/invert_abilities.py
```

## What It Analyzes

The canonical report (`engine/tests/TEST_COVERAGE.md` + `docs/ABILITY_MATRIX.md` + `TEST_INVENTORY.json`) covers:
- **Coverage by action type** — which effect actions have tests
- **Coverage by trigger type** — 登場, ライブ開始時, 常時, etc.
- **Coverage by action+cost pairing** — e.g. `move_cards + pay_energy`
- **Untested key combinations** — unique JSON key combos with zero test coverage
- **Top coverage gaps** — the 30 largest untested combos
- **Complete schema reference** — every key combination in the data

The inverted index (`INVERTED_ABILITIES_CONDENSED.md`, `INVERTED_ABILITIES_ABSTRACT.md`) maps parsed JSON structures back to the raw Japanese texts that produce them — useful for finding parser gaps where the same JSON hides different meanings.

## Scripts

| Script | Description | Status |
|--------|-------------|--------|
| `../test_inventory.py` | **Canonical** coverage inventory — generates `TEST_COVERAGE.md`, `ABILITY_MATRIX.md`, `TEST_INVENTORY.json/.md` | **Use this** |
| `analyze_abilities.py` | Field usage stats (cost/effect keys) | Kept (parser introspection, not coverage) |
| `invert_abilities.py` | JSON-to-text inversion (`INVERTED_ABILITIES*.md`, `--card/--diff/--query`) | Kept (parser introspection) |

## Output Files

_Generated outputs are **not checked in** — regenerate with the scripts above._

| File | Produced by | Description |
|------|-------------|-------------|
| `../../engine/tests/TEST_COVERAGE.md` + `../../docs/ABILITY_MATRIX.md` + `TEST_INVENTORY.json/.md` | `../test_inventory.py` (checked in, CI-checked) | Canonical coverage inventory |
| `ABILITY_DOCUMENTATION.md` → canonical copy at `../ABILITY_DOCUMENTATION.md` | analyze_abilities.py era tools | Exhaustive schema reference |
| `INVERTED_ABILITIES_CONDENSED.md` / `_ABSTRACT.md` / `.md` | `invert_abilities.py` | JSON → source text inversion for parser introspection |

## Test Coverage Method

Each ability in `abilities.json` has a `cards` array listing which card IDs use it.
The report scripts scan test `.rs` files in `engine/tests/test_modules/` for quoted strings
matching those card IDs (e.g. `"PL!-sd1-005-SD"`). If any card ID from an ability
appears in any test file, that ability is considered "tested".

This is a heuristic — a card being referenced doesn't guarantee thorough testing —
but it's an effective first-pass filter for finding gaps.

## Typical Workflow

1. Run `python cards/test_inventory.py` (from repo root)
2. Open `docs/ABILITY_MATRIX.md` for **Gaps to prioritize** or `engine/tests/TEST_INVENTORY.md` for per-ability depth
3. Use `engine/tests/TEST_COVERAGE.md` gap tables to pick target cards
4. Check `INVERTED_ABILITIES*.md` / `python ability_docs_scripts/invert_abilities.py --card PL!…` for parser introspection
5. Write tests in `engine/tests/test_modules/`
6. Re-run `python cards/test_inventory.py` and verify `python cards/test_inventory.py --check` passes
