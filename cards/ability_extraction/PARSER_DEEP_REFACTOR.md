# Parser Deep Refactoring — Findings & Plan

## Current State
- 2313 engine tests pass (7 new bp7-007 tests)
- 22 parser tests pass
- 0 custom actions (was 1)
- Validation: 0 issues

---

## DONE this session
- Phase 1-7: deduplicate, unify, break up, remove dead code
- Phase A: parser_utils overlap cleanup
- Phase B: _walk sub-walker dedup
- Phase C: merge validation systems
- P0: fix bp6-003 regression (Rust: respect card_type from parsed effect)
- Fix last custom action (appear-to-empty-slot)
- 7 edge case gameplay tests for bp7-007

---

## Remaining

### P1: `_try_per_unit` breakdown (376 lines) — HIGHEST ROI
Location: parser.py ~6323

Does 6 jobs in one function:
1. Detect per-unit marker (につき/ごとに)
2. Extract count + unit type from text
3. Classify per_unit_type (member/card/live_card_zone/energy_deck/etc)
4. Parse group names + heart colors + card_property from per-unit text
5. Split sequential patterns (e.g. "Aを得る。Bを失う" per unit)
6. Propagate per-unit config into sub-actions

Refactor into: `_detect_per_unit(text)`, `_extract_per_unit_type(text)`, `_split_per_unit_sequential(text, info)`, `_propagate_per_unit(eff, info)`

### P2: `parse_action` 460-line monolith
Location: parser.py:2587-3053

Does 21 things. Steps 1-18 are unconditional field extraction before dispatch.
Refactor into `_extract_common_fields(text, action)` + short dispatch.

### P3: `_try_conditional` 129 lines with 15+ special cases
Location: parser.py ~8474

Generic conditional handler with accumulated if-blocks for:
yell count, baton touch, preceding_moved, OR conditions, etc.
Refactor: extract special cases into post-parse enrichers.

### P4: `_walk` sub-walker grouping
11 sub-walkers → group into 4-5 logical groups.
e.g. `_walk_propagate_identity` (same_name/distinct/group_names),
`_walk_propagate_targeting` (exclude_self/position).

### P5: `_fill_defaults_move_cards` zone-only dest heuristic
Current workaround: text-based "メンバーのいないエリア" check.
Refactor: set `unresolved_source: true` instead of demoting to custom.
