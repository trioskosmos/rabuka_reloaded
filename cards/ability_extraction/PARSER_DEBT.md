# Parser Tech Debt — Progress Tracker

**File**: `cards/ability_extraction/parser.py` (~12,600 lines)
**Baseline**: 2306 engine tests + 22 parser tests pass

---

## Phase 1: Extract shared helpers + replace inline copies ✅ DONE
- [x] Add `detect_exclude_self(text) -> bool` helper
- [x] Add `extract_heart_colors_from_text(text) -> list` helper
- [x] Add `detect_duration_code(text) -> str|None` helper
- [x] Replace 6+ cost_limit_operator inline copies → `extract_cost_operator()`
- [x] Replace 9+ exclude_self inline copies → `detect_exclude_self()`
- [x] Replace 6+ heart_colors extraction copies → `extract_heart_colors_from_text()`
- [x] Remove debug print at ~line 5291

## Phase 2: Unify parse_condition dual-path enrichment ✅ DONE
- [x] Extract `_enrich_condition_common(d, text)` helper
- [x] Replace handler path enrichment block → `_enrich_condition_common(result, text)`
- [x] Replace fallthrough path enrichment block → `_enrich_condition_common(condition, text)`

## Phase 3: Break `_fill_defaults` into helpers — DEFERRED
- [ ] Extract `_fill_defaults_source_dest(action, text)`
- [ ] Extract `_fill_defaults_card_type(action, text)`
- [ ] Extract `_fill_defaults_resource(action, text)`
- [ ] Extract `_fill_defaults_per_unit(action, text)`
- [ ] Extract `_fill_defaults_misc(action, text)`

## Phase 4: Break `_extract_generic_fields` into helpers — DEFERRED
- [ ] Extract `_extract_target_location_fields(d, text)`
- [ ] Extract `_extract_card_resource_fields(d, text)`
- [ ] Extract `_extract_comparison_fields(d, text)`
- [ ] Extract `_extract_temporal_fields(d, text)`

## Phase 5: Break `_process_pre_fix` into named fix functions — DEFERRED
- [ ] Extract each numbered FIX block into `_fix_N_name(eff, fix_stats)` functions

## Phase 6: Break `_process_post_fixes` into named fix functions — DEFERRED
- [ ] Extract heart_colors stripping, cost card_property, primary_effect cleanup,
  conditional_on_result restructuring into separate functions

## Phase 7: Remove dead code ✅ DONE
- [x] Remove duplicate re_yell registration (was at ~2250-2279)
- [x] Remove dead `locals().get("dur_effect")` check in parse_effect

## Phase 8: Standardize _ACTION_RULES format — DEFERRED
- [ ] Convert tuple-format entries to ActionRule objects
- [ ] Remove try/except TypeError arity workaround
