# Parser Notes — debt, history, and status

**File**: `cards/ability_extraction/parser.py` (~13.5K lines)
Consolidates the former `PARSER_REFACTOR.md`, `PARSER_DEEP_REFACTOR.md`, and
`PARSER_DEBT.md` (all completed sessions; kept in git history).

## Remaining debt (live)

### Phase 8 — Standardize `_ACTION_RULES` format — DONE (stale entry)
All registrations already go through the ActionRule-normalizing
`_register_action` (`__post_init__` arity normalization, E2a session); the
dispatch loop has no TypeError workaround. Nothing to do.

### `_process_pre_fix` FIX-block triage (2026-08-24 session)
Empirically verified by removal + regen byte-diff, one block at a time:

| Block | Verdict | Evidence |
|---|---|---|
| FIX 6 (opponent_action flatten) | **DEAD — removed** | 0 output change; no producer emits the wrapper |
| FIX 2 (each_time → conditional_on_optional) | LOAD-BEARING | 2 abilities change without it |
| FIX 3 (conditional_on_optional cleanup) | LOAD-BEARING | 4 abilities change; `optional` strip prevents engine double-prompting. positive/negative renames have no producer (dead but harmless) |
| FIX 7/7b (ability_filter backfill) | LOAD-BEARING | 2 abilities |
| FIX 9 (result_condition card_property) | LOAD-BEARING | 1 ability |
| FIX 9b (followup self_target/self_cost) | LOAD-BEARING | 2 abilities |
| FIX 10–15, N | presumed live (same pattern); untested | — |

Conclusion: these blocks are compensations for shapes that *handlers emit*.
Dissolving them (Phase 5) means fixing each producing handler first — the
blocks themselves are correct code.

## Fundamental structural issues (from the deep-refactor review)
1. **`_fill_defaults` re-extracts fields `parse_action` already set**
   Re-extracts source, destination, cost_limit, optional, max, position,
   group_names, heart_colors. Reader can't tell which function sets which field.
   *Fix*: move all extraction into `parse_action`; `_fill_defaults` only sets
   action-type-specific defaults (draw→deck/hand, shuffle→move_cards).
2. **`_fill_defaults_move_cards` is ~131 lines of source inference**
   Complex source→destination inference that belongs in the dispatch table or
   `parse_action` itself.
3. **`_walk` + `_propagate_context` = two full tree walks**
   `_walk` (11 sub-walkers) runs during normalization; `_propagate_context`
   (~240 lines) runs after `_process_pre_fix`. Overlapping work; merging requires
   understanding the timing dependency (propagate needs pre_fix output).
4. **`_process_pre_fix` is ~340 lines of compensating patches**
   See triage table above: most are load-bearing; dissolution = producer fixes.
5. **Double/triple extraction of the same fields**
   `extract_source`, `extract_destination`, `extract_card_type`, etc. are called
   3–4 times on the same text across `parse_action`, `_fill_defaults`, `_walk`.

## Completed work (history)

### Session: extract script single-owner parsing (2026-08-24)
- extract_card_abilities.py now calls parser's real `parse_ability()`;
  deleted the divergent weaker inline copy
- Added `normalize_multiline()` (old normalize collapsed `\n` choice bullets)
- Hardened condition back-fill: effect text only, only when no condition
  exists, leading-gate only (was re-scanning cost text and double-gating)
- Generic 「プレイに際し…コストはNになる」 handler (`_try_play_time_cost_set`)
  replaced the LL-bp7-001-specific override
- Deleted dead code: FieldExtractor, _DEBUG_LOG plumbing, segment_clauses
  Stage-A IR (+tests), compile_abilities vocab/encode block, typo'd patterns

### Session: dedup + helper extraction (PARSER_DEBT)
- Extracted shared helpers: `detect_exclude_self()`, `extract_heart_colors_from_text()`,
  `detect_duration_code()`, `extract_cost_operator()`; replaced all inline copies
- Unified dual-path condition enrichment → `_enrich_condition_common()`
- Broke up god-functions into named helpers:
  - `_fill_defaults` 498 → ~260 lines (`_fill_defaults_move_cards`, `_fill_defaults_count_and_refine`)
  - `_extract_generic_fields` 366 → ~250 (`_extract_comparison_fields`, `_extract_resource_fields`)
  - `_process_pre_fix` 476 → ~340 (`_fix_sequential_chain`, `_fix_condition_enrichment`)
  - `_process_post_fixes` ~338 → ~175 (`_fix_conditional_on_result`)
- Removed dead code (duplicate re_yell registration, dead `dur_effect` check)

### Session: overlap cleanup (PARSER_REFACTOR)
- Removed `parser_utils`/`parser.py` overlaps (dead cost pattern, local POSITION_KEYWORDS)
- Deduplicated `_walk` sub-walkers (removed superseded propagation paths)

### Session: validation merge + deep refactor (PARSER_DEEP_REFACTOR)
- Deleted `_validate_output()` from extract_card_abilities.py (~240 lines);
  merged into single `_validate_semantic()` with recursive tree walking
- `_try_per_unit` 350 → 25 lines; `parse_action` reduced by 130 lines
