# Parser Tech Debt — Progress Tracker

**File**: `cards/ability_extraction/parser.py` (~12,600 lines)
**Baseline**: 2306 engine tests + 22 parser tests pass

---

## Phase 1: Extract shared helpers + replace inline copies
- [ ] Add `detect_exclude_self(text) -> bool` helper
- [ ] Add `extract_heart_colors_from_text(text) -> list` helper
- [ ] Add `detect_duration_code(text) -> str|None` helper
- [ ] Replace 6+ cost_limit_operator inline copies → `extract_cost_operator()`
- [ ] Replace 9+ exclude_self inline copies → `detect_exclude_self()`
- [ ] Replace 6+ heart_colors extraction copies → `extract_heart_colors_from_text()`
- [ ] Remove debug print at ~line 5291
- [ ] Run: `python ability_extraction/extract_card_abilities.py` → `cargo test --test run_all` → `python test_parser_coverage.py`

## Phase 2: Unify parse_condition dual-path enrichment
- [ ] Extract `_enrich_condition_common(d, text)` helper (scope, position, position_compare, require_position_cards, or_location, heart_content)
- [ ] Replace handler path enrichment block → `_enrich_condition_common(result, text)`
- [ ] Replace fallthrough path enrichment block → `_enrich_condition_common(condition, text)`
- [ ] Run: extract → engine tests → python tests

## Phase 3: Break `_fill_defaults` into helpers
- [ ] Extract `_fill_defaults_source_dest(action, text)` (source/destination normalization)
- [ ] Extract `_fill_defaults_card_type(action, text)` (card_type inference, OR/AND splitting)
- [ ] Extract `_fill_defaults_resource(action, text)` (resource inference, count, operation)
- [ ] Extract `_fill_defaults_per_unit(action, text)` (per_unit_type, blade_limit, dynamic_count)
- [ ] Extract `_fill_defaults_misc(action, text)` (self_target, position, all, multiple_targets, max, any_number, same_name, exclude_group_names, non_stackable, need_heart)
- [ ] Run: extract → engine tests → python tests

## Phase 4: Break `_extract_generic_fields` into helpers
- [ ] Extract `_extract_target_location_fields(d, text)` (characters, groups, targets, locations, exclude_self, exclude_characters, group_names, all_areas, all_members)
- [ ] Extract `_extract_card_resource_fields(d, text)` (card_type, heart_counts, energy, surplus_heart, blade, cost_limit, card_property)
- [ ] Extract `_extract_comparison_fields(d, text)` (comparison_target, comparison_type, aggregate, operator, count, negation, self_target)
- [ ] Extract `_extract_temporal_fields(d, text)` (temporal_scope, distinct, movement, duration, state, phase)
- [ ] Run: extract → engine tests → python tests

## Phase 5: Break `_process_pre_fix` into named fix functions
- [ ] Extract each numbered FIX block into `_fix_N_name(eff, fix_stats)` functions
- [ ] Run: extract → engine tests → python tests

## Phase 6: Break `_process_post_fixes` into named fix functions
- [ ] Extract heart_colors stripping, cost card_property, primary_effect cleanup, conditional_on_result restructuring into separate functions
- [ ] Run: extract → engine tests → python tests

## Phase 7: Remove dead code
- [ ] Remove duplicate re_yell registration (~2306 or ~2357)
- [ ] Remove duplicate gain_resource for ハート (~2407)
- [ ] Remove dead `locals().get("dur_effect")` check in parse_effect
- [ ] Remove unreachable comment after `return None` in parse_condition
- [ ] Run: extract → engine tests → python tests

## Phase 8: Standardize _ACTION_RULES format
- [ ] Convert tuple-format entries to ActionRule objects
- [ ] Remove try/except TypeError arity workaround
- [ ] Run: extract → engine tests → python tests
