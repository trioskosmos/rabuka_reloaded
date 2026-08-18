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

## Phase 3: Break `_fill_defaults` into helpers ✅ DONE
- [x] Extract `_fill_defaults_move_cards()` (~125 lines)
- [x] Extract `_fill_defaults_count_and_refine()` (~90 lines)
- `_fill_defaults` reduced from 498 to ~260 lines

## Phase 4: Break `_extract_generic_fields` into helpers ✅ DONE
- [x] Extract `_extract_comparison_fields()` (~68 lines)
- [x] Extract `_extract_resource_fields()` (~49 lines)
- `_extract_generic_fields` reduced from 366 to ~250 lines

## Phase 5: Break `_process_pre_fix` into named fix functions ✅ DONE
- [x] Extract `_fix_sequential_chain()` (~63 lines)
- [x] Extract `_fix_condition_enrichment()` (~70 lines)
- `_process_pre_fix` reduced from 476 to ~340 lines

## Phase 6: Break `_process_post_fixes` into named fix functions ✅ DONE
- [x] Extract `_fix_conditional_on_result()` (~163 lines)
- `_process_post_fixes` reduced from ~338 to ~175 lines

## Phase 7: Remove dead code ✅ DONE
- [x] Remove duplicate re_yell registration (was at ~2250-2279)
- [x] Remove dead `locals().get("dur_effect")` check in parse_effect

## Phase 8: Standardize _ACTION_RULES format — NOT DONE (33 tuple entries remain)
- [ ] Convert 33 tuple-format entries to ActionRule objects
- [ ] Remove try/except TypeError arity workaround in dispatch
- Low priority: mechanical work, no behavioral change
