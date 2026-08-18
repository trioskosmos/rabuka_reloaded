# Parser Refactoring Plan — Beyond Phase 8

Three high-impact refactorings that reduce real pain in the parser.

---

## 1. Merge Validation Systems

**Problem**: Two near-duplicate validation systems exist:
- `_validate_semantic()` in parser.py (581 lines, 27 regex rules + 4 structural checks)
- `_validate_output()` in extract_card_abilities.py (241 lines, 10 inline checks)

7 checks are duplicated (same_name, distinct_name, lose_resource, per_group, baton_touch, non_stackable, card_property).

**Key difference**: `_validate_semantic` checks top-level only; `_validate_output` recursively walks to depth 10. `_validate_semantic` filters quotes/parens; `_validate_output` doesn't.

**Plan**:
1. Merge all 10 checks from `_validate_output` into `_validate_semantic`'s rule table (add 3 new rules: or_location, heart_content, state_change)
2. Make all rules use recursive tree walking (not just top-level)
3. Keep the quote/paren filtering from `_validate_semantic`
4. Delete `_validate_output` from extract_card_abilities.py, call `_validate_semantic` instead
5. Verify: `python extract_card_abilities.py` output should show same or fewer gaps

**Risk**: Low — validation only reports, doesn't modify output.

---

## 2. Deduplicate _walk Sub-Walkers

**Problem**: Multiple sub-walkers do overlapping work:
- `_walk_set_defaults` (line 10109) and `_propagate_context` (line 10531) both infer operators and count from text
- `_walk_propagate_text_context_fields` (line 9784) and `_walk_propagate_flags` (line 10184) both propagate `same_name`

**Plan**:
1. Remove `same_name` propagation from `_walk_propagate_flags` (keep it in `_walk_propagate_text_context_fields` which runs first)
2. Merge the operator/count inference from `_propagate_context` into `_walk_set_defaults` — make `_walk_set_defaults` the single source of truth
3. Simplify `_propagate_context` to only handle context-specific fields (heart_colors, target, group_names) that the walker can't do
4. Verify: extract → tests pass

**Risk**: Medium — must ensure no field propagation is lost.

---

## 3. Clean Up parser_utils/parser.py Overlap

**Problem**:
- `extract_cost_operator` in parser.py (line 553) fully reimplements `extract_operator` from parser_utils.py (line 216)
- `POSITION_KEYWORDS` duplicated between parser_utils.py (line 556) and parser.py (line 161)
- `check_exclude_self` (parser_utils) vs `detect_exclude_self` (parser.py) — confusingly different

**Plan**:
1. Replace `extract_cost_operator` in parser.py with import from parser_utils
2. Remove local `POSITION_KEYWORDS` from parser.py, import from parser_utils
3. Rename `check_exclude_self` in parser_utils to `_check_exclude_self_broad` (it's only used by FieldExtractor internally) to avoid confusion with `detect_exclude_self`
4. Verify: extract → tests pass

**Risk**: Low — import changes only.

---

## Execution Order

1. Phase A: Clean up parser_utils overlap (lowest risk, clears confusion)
2. Phase B: Deduplicate _walk sub-walkers (medium risk, structural improvement)
3. Phase C: Merge validation systems (low risk, largest line reduction)
