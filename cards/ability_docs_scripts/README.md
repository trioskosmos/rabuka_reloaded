# Ability Documentation Scripts

Tools for analyzing `cards/abilities.json` and tracking test coverage for card abilities.

## Quick Start

```bash
# Single command — regenerates coverage docs (canonical):
python ../test_inventory.py
# (or from repo root: python cards/test_inventory.py)

# Legacy aliases (deprecated shims → test_inventory.py):
python ../coverage_report.py
python ability_docs_scripts/generate_report.py  # now -> test_inventory

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
| `../coverage_report.py` | Alias for `test_inventory.py` | Deprecated shim |
| `generate_report.py` | Old `FULL_REPORT.md` generator | Deprecated shim → `test_inventory.py` |
| `cross_reference_tests.py` | Old action→test mapping | Deprecated shim |
| `find_untested_abilities.py` | Old untested report | Deprecated shim |
| `find_unique_untested.py` | Old unique-pattern finder | Deprecated shim |
| `key_combo_analysis.py` | Old key-combo analysis | Deprecated shim |
| `analyze_abilities.py` | Field usage stats (cost/effect keys) | Kept (parser introspection, not coverage) |
| `invert_abilities.py` | JSON-to-text inversion (`INVERTED_ABILITIES*.md`, `--card/--diff/--query`) | Kept (parser introspection) |

## Output Files

| File | Description |
|------|-------------|
| `FULL_REPORT.md` | Comprehensive coverage report with tables and gaps |
| `untested_card_ids.txt` | Plain list of untested card IDs for test writing |
| `ABILITY_DOCUMENTATION.md` | Exhaustive schema reference |
| `abilities_summary.md` | High-level stats (card counts, trigger breakdown) |
| `INVERTED_ABILITIES_CONDENSED.md` | JSON → source text substrings with `(xN)` counts |
| `INVERTED_ABILITIES_ABSTRACT.md` | Abstracted JSON (values → placeholders) with variable breakdown |

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
