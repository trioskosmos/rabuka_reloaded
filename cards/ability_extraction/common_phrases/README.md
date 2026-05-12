# Common Phrases Analysis

This folder contains tools for analyzing common phrases found in ability texts
from `abilities.json`, and for identifying issues in the parser (`parser.py`).

## Files

- `analyze.py` — Frequency analysis of common phrases, pattern detection, effect type distribution
- `analyze_deep.py` — Deeper analysis of specific parsing issues (do_nothing artifacts, conditional_alternative, これにより, etc.)
- `README.md` — This file

## Key Findings

### Most Common Phrases (Top 15)
| Count | Phrase | Translation |
|-------|--------|-------------|
| 267 | 場合、 | if/when |
| 195 | もよい / てもよい | may (optional) |
| 195 | 置く | place/put |
| 170 | を得る | gain/acquire |
| 154 | 以上 | or more / >= |
| 146 | ライブ終了時まで | until end of live |
| 140 | 手札 | hand |
| 103 | 控え室 | waiting room |
| 87 | 選ぶ | choose/select |
| 85 | 引く | draw |
| 70 | 合計 | total |
| 67 | 手札に加える | add to hand |
| 62 | ステージ | stage |
| 61 | その中から | from among them |
| 61 | 登場 | appear/enter |

## Parser Issues Found & Fixed

### Fixed in parser.py

1. **do_nothing artifacts** (25 -> 2 remaining, both legitimate)
   - `_try_shi_sequential` was splitting condition-containing texts (with `場合、`) on commas,
     creating fragmentary actions. Added condition-marker check to skip such texts.
   - `_try_ability_activation` was splitting on `。` inside `「」` quoted text, splitting
     ability texts at internal periods. Added sentinel-based period protection.
   - `_try_implicit_sequential` period-splitting was also splitting inside `「」`. Added
     the same sentinel protection.
   - `_clean_action_list` was defined in `_normalize_effect_tree` but never called.
     Added call in `_walk` to filter do_nothing from action lists.
   - `_try_kore_niyori_result` produced `primary_effect: do_nothing` when text before
     `これにより` was empty/fragmentary. Added empty-primary-text guard.
   - `parse_action` gain_ability detection was overwritten by the dispatch table. Added
     early return for `is_ability_gain` before dispatch table runs.

2. **debug print** in `parse_cost` (line 2196) — removed.

### Remaining issues (not fixed)

1. **`それらがすべて`** (5 abilities) — Could use dedicated handler.
2. **`代わりに` multi-line case** (1 ability) — Falls through to `sequential` correctly.
3. **Source/destination** — 4 move_cards actions have no destination (edge cases).
4. **`そうした場合` conditional** — Period always precedes it; parsed as `sequential`
   with `conditional: True` flag, which is acceptable.

### How to Run
```bash
python cards/ability_extraction/common_phrases/analyze.py
python cards/ability_extraction/common_phrases/analyze_deep.py
```
