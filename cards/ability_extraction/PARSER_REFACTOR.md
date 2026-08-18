# Parser Refactoring Plan — Complete

All three phases done. 2306 engine tests + 22 parser tests pass.

---

## Phase A: Clean up parser_utils/parser.py overlap ✅
- Removed `extract_cost_operator()` from parser.py (dead `より大きい` pattern)
- Removed local `POSITION_KEYWORDS` from parser.py (imported from parser_utils)
- Renamed `check_exclude_self` → `_check_exclude_self_broad` in parser_utils

## Phase B: Deduplicate _walk sub-walkers ✅
- Removed `same_name` propagation from `_walk_propagate_text_context_fields`
  (superset handled by `_walk_propagate_flags`)
- Removed operator/count inference from `_propagate_context`
  (already handled by `_walk_set_defaults` during `_walk` traversal)

## Phase C: Merge validation systems ✅
- Deleted `_validate_output()` from extract_card_abilities.py (~240 lines)
- Added 3 new rules to `_validate_semantic()`: or_location, heart_content, state_change
- `_validate_semantic` now uses `_json_has` recursive tree walking for all checks
- Import `_validate_semantic` in extract_card_abilities.py

## Phase 8: Standardize _ACTION_RULES format — NOT DONE (33 tuple entries remain)
- Low priority: mechanical work, no behavioral change
