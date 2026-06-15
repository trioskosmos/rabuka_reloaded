# Ability Documentation Scripts

Tools for analyzing `cards/abilities.json` and tracking test coverage for card abilities.

## Quick Start

```bash
# Single command — regenerates everything:
python ability_docs_scripts/generate_report.py

# Invert abilities.json — map JSON back to source texts:
python ability_docs_scripts/invert_abilities.py
```

## What It Analyzes

The main report (`FULL_REPORT.md`) covers:
- **Coverage by action type** — which effect actions have tests
- **Coverage by trigger type** — 登場, ライブ開始時, 常時, etc.
- **Coverage by action+cost pairing** — e.g. `move_cards + pay_energy`
- **Untested key combinations** — unique JSON key combos with zero test coverage
- **Top coverage gaps** — the 30 largest untested combos
- **Complete schema reference** — every key combination in the data

The inverted index (`INVERTED_ABILITIES_CONDENSED.md`, `INVERTED_ABILITIES_ABSTRACT.md`) maps parsed JSON structures back to the raw Japanese texts that produce them — useful for finding parser gaps where the same JSON hides different meanings.

## Scripts

| Script | Description |
|--------|-------------|
| `generate_report.py` | Main report generator, produces `FULL_REPORT.md` and `untested_card_ids.txt` |
| `analyze_abilities.py` | Field usage stats (cost keys, effect keys by type/action) |
| `cross_reference_tests.py` | Action-to-test-file mapping |
| `find_untested_abilities.py` | Per-ability untested report with card IDs |
| `key_combo_analysis.py` | Key-combination-level coverage analysis |
| `invert_abilities.py` | JSON-to-text inversion: produces `INVERTED_ABILITIES_CONDENSED.md` and `INVERTED_ABILITIES_ABSTRACT.md` |

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

1. Run `python ability_docs_scripts/generate_report.py`
2. Open `FULL_REPORT.md` and look at **Top Coverage Gaps** and **Untested Key Combinations**
3. Use `untested_card_ids.txt` to find cards to write tests for
4. Check `ABILITY_DOCUMENTATION.md` for the ability schema
5. Write tests in `engine/tests/test_modules/`
6. Re-run `generate_report.py` to verify coverage improved
