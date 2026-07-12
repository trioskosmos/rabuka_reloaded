# Fix Plan: Ability Enumification Test Failures

## Problem Summary

The "Ability enumification" refactor converted `AbilityEffect` from a flat struct with ~142 optional fields into an `EffectKind` enum with ~14 variant-specific field sets. The root cause of all ~154 test failures is:

**`build_kind_from_action()` wraps raw JSON as `{Tag: json}` and calls `serde_json::from_value()`. Serde silently drops any JSON field that doesn't match the EffectKind variant's fields.**

When abilitiy JSON has field `"heart_color"` (singular) but the GainResource variant only has `heart_colors` (plural Vec), the field is silently dropped and the value is lost. All `_any()` accessors then return `None`.

## 7 Specific Field Mismatches Found

| # | JSON Field | Action | EffectKind Missing In | Used By | Fix |
|---|---|---|---|---|---|
| 1 | `cost_reference` | `move_cards` | MoveCards | Yoshiko | Add field to MoveCards |
| 2 | `cost_offset` | `move_cards` | MoveCards | Yoshiko | Add field to MoveCards |
| 3 | `distinct` | `gain_resource` | GainResource | Edelnote | Add field to GainResource |
| 4 | `heart_color` (singular) | `gain_resource` | GainResource | Tiny Stars | Add field + `heart_color_any()` |
| 5 | `replace_all` | `modify_required_hearts` | ModifyHearts | Hareruya | Add field to ModifyHearts |
| 6 | `max_repeats` (alias) | `repeat_procedure` | CompoundEffect | Natsumi BP5 | Add serde alias |
| 7 | `all` | `move_cards` | MoveCards | Bring the LOVE! | Add field to MoveCards |

## Additional Bugs

### Bug A: `clone_from` in compound.rs:298
```rust
// This does NOTHING — clone_from on Option<&Vec<String>> copies the reference, not the data
action_to_execute.card_names_any().clone_from(&effect.card_names_any());
```
**Fix:** Use `set_card_names()` instead.

### Bug B: Nested EffectKind not populated
The card_loader only walks `compound.actions`, `look_action`, `select_action`, etc. but NOT inside EffectKind variant fields like `options`, `resource_on_select`, `gained_effect`, `alternative_effect`, `opponent_action`, `effect_steps`, and Condition's `options`.
**Fix:** Add recursive `populate_all_nested()` function.

### Bug C: Natsumi `resolve_cost_limit_reference_for_condition` missing
The sequential handler in compound.rs needs to resolve dynamic cost limit references for conditions (needed by Natsumi's conditional_on_result flow).
**Fix:** Add the function (already in move_cards.rs, copy pattern).

## Per-Test-Module Impact

| Module | Tests Failing | Root Cause |
|---|---|---|
| `yoshiko_*` | ~13 files, ~30+ tests | #1, #2: cost_reference/cost_offset missing from MoveCards |
| `natsumi_bp5` | ~12 tests | #6: max_repeats not aliased |
| `edelnote` | ~10 tests | #3: distinct missing from GainResource |
| `b9_more` (Tiny Stars) | ~8 tests | #4: heart_color singular missing + Bug A (clone_from) |
| `bring_love` | ~8 tests | #7: all missing from MoveCards |
| `q127_heart_set_plus` | ~8 tests | #5: replace_all missing from ModifyHearts |
| `bloom_hs` | ~4 tests | #4/#5 indirect |
| `kasumi` | ~6 tests | Activation position/condition — likely kind population (Bug B) |
| `keke_bp5` | ~3 tests | Filter/cost_limit — likely kind population (Bug B) |
| `special_color` | ~6 tests | Position/group filter — likely kind population (Bug B) |
| Other tests | ~50+ tests | Various — generic EffectKind population (Bug B) + accessor fallbacks |

## Fix Implementation Plan

### Step 1: Add missing fields to EffectKind variants
Add these fields to `card.rs` EffectKind enum:
- `MoveCards`: `cost_reference: Option<String>`, `cost_offset: Option<i32>`, `all: Option<bool>`
- `GainResource`: `distinct: Option<String>`, `heart_color: Option<String>`
- `ModifyHearts`: `replace_all: Option<bool>`
- `CompoundEffect`: Add `#[serde(alias = "max_repeats")]` to `repeat_limit`

### Step 2: Update accessor/setter methods
- Add `MoveCards` to match arms of `cost_reference_any()`, `cost_offset_any()`, `all_any()`, `set_cost_reference()`, `set_cost_offset()`, `set_all()`
- Add `GainResource` to match arms of `distinct_any()`, `set_distinct()`; add new `heart_color_any()` accessor + `set_heart_color()` setter
- Add `ModifyHearts` to match arms of `replace_all_any()`, `set_replace_all()`

### Step 3: Fix clone_from bug in compound.rs
Change line 298 to use `action_to_execute.set_card_names(effect.card_names_any().cloned())`

### Step 4: Add recursive populate_all_nested to card_loader.rs
Function that walks ALL EffectKind variant fields with nested AbilityEffects (options, resource_on_select, gained_effect, alternative_effect, opponent_action, effect_steps, condition options) and populates their `kind` field recursively.

### Step 5: Add raw_json fallback
Add `raw_json: Option<serde_json::Value>` to AbilityEffect, set it during card_loader population, and add fallback in all `_any()` accessors to read from raw_json when EffectKind doesn't have the field.

This step fixes ALL remaining field mismatches that may not be covered by Step 1.
