# Ability Documentation Scripts

Tools for analyzing `cards/abilities.json` and tracking test coverage for card abilities.

## Quick Start

```bash
# Single command — regenerates everything:
python ability_docs_scripts/generate_report.py
```

That's it. Two output files are produced:
- `FULL_REPORT.md` — comprehensive coverage report with tables and gaps
- `untested_card_ids.txt` — plain list of untested card IDs for test writing

## What It Analyzes

The report covers:
- **Coverage by action type** — which effect actions (move_cards, draw_card, etc.) have tests
- **Coverage by trigger type** — 登場, ライブ開始時, 常時, etc.
- **Coverage by action+cost pairing** — e.g. `move_cards + pay_energy`
- **Untested key combinations** — which unique sets of JSON keys have zero test coverage
- **Top coverage gaps** — the 30 largest untested combos
- **Complete schema reference** — every key combination that appears in the data

## Test Coverage Method

Each ability in `abilities.json` has a `cards` array listing which card IDs use it.
The script scans test `.rs` files in `engine/tests/test_modules/` for quoted strings
matching those card IDs (e.g. `"PL!-sd1-005-SD"`). If any card ID from an ability
appears in any test file, that ability is considered "tested".

This is a heuristic — a card being referenced doesn't guarantee thorough testing —
but it's an effective first-pass filter for finding gaps.

## Typical Workflow

1. Run `python ability_docs_scripts/generate_report.py`
2. Open `FULL_REPORT.md` and look at **Top Coverage Gaps** and **Untested Key Combinations**
3. Use `untested_card_ids.txt` to find cards to write tests for
4. Check `ABILITY_DOCUMENTATION.md` for the ability schema
5. Write tests in `engine/tests/test_modules/`
6. Re-run `generate_report.py` to verify coverage improved

## Individual Scripts

The analysis is also available as separate scripts for CI or modular use:

| Script | Description |
|--------|-------------|
| `analyze_abilities.py` | Field usage stats (cost keys, effect keys by type/action) |
| `cross_reference_tests.py` | Action-to-test-file mapping |
| `find_untested_abilities.py` | Per-ability untested report with card IDs |
| `key_combo_analysis.py` | Key-combination-level coverage analysis |

These produce individual output files in `archive/`.

## Reference Files

| File | Description |
|------|-------------|
| `ABILITY_DOCUMENTATION.md` | Exhaustive schema reference (cost types, effect actions, conditions, zones, triggers, execution pipeline, edge cases) |
| `abilities_summary.md` | High-level stats (card counts, trigger breakdown) |
