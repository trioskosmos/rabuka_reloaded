# Engine Simplification Plan

## Overview
Findings from auditing abilities.json -> bytecode -> engine pipeline for redundant code, duplicate logic, and mergeable abstractions.

---

## Phase 1: Remove Redundant ActionType Variants (engine)

The parser already never emits `draw`, `look`, `reveal_effect`, `conditional_optional`, or `set_card_identity_all_regions`. The engine has dead match arms for these.

### 1a. Remove 6 unused ActionType variants

| Remove | from_str alias to | Parser emits | Engine handler at |
|---|---|---|---|
| `Draw` | `DrawCard` | `draw_card` (117x) | `effects/mod.rs:267` — same handler |
| `Look` | `LookAt` | `look_at` (72x) | `effects/mod.rs:537` — same handler body |
| `RevealEffect` | `Reveal` | `reveal` (9x) | `effects/mod.rs:525` — same handler |
| `ConditionalOptional` | `ConditionalOnOptional` | `conditional_on_optional` (6x) | `effects/mod.rs:929` — duplicate of line 848 |
| `SetCardIdentityAllRegions` | `SetCardIdentity` | `set_card_identity` (1x) | `effects/mod.rs:825` — already subsumed by `execute_set_card_identity_effect` checking `all_regions_any()` |
| `ModifyRequiredHeartsGlobal` | `ModifyRequiredHearts` | `modify_required_hearts_global` (3x) | `effects/mod.rs:570` — calls `execute_modify_required_hearts_standard` which is a subset of `execute_modify_required_hearts` |

**Files:** `engine/src/ability/enums.rs`, `engine/src/ability/effects/mod.rs`
**Steps:** Remove enum variant, alias `from_str` to surviving variant, remove `to_str`/`label` entries, remove duplicate match arm.

### 1b. Remove 5 dead internal-only ActionType variants

These all hit the `log::warn!("Unexpected internal action type")` catch-all. The parser never produces them.

| Remove | Reason |
|---|---|
| `Tap` | Internal-only, no handler |
| `Rest` | Internal-only, no handler |
| `Discard` | Parser produces `DiscardCard`, not `Discard` |
| `ChoiceCondition` | Condition enum repurposed as action, never reaches execute_effect |
| `EnergyCondition` | Same as above |

**Files:** `engine/src/ability/enums.rs`, `engine/src/ability/effects/mod.rs:914-928`
**Steps:** Remove from enum, remove from catch-all match arm.

---

## Phase 2: Remove Redundant Condition Code (engine)

### 2a. Remove `evaluate_or_condition`

- `compound.rs:65-105` (`evaluate_or_condition`) is identical to `evaluate_compound_condition` with `operator == "or"`
- Route all OR compounds through `evaluate_compound_condition`
- Remove `evaluate_or_condition` function

### 2b. Remove dead `ConditionType::OrCondition`

- `enums.rs:511` — exists as metadata only, never a distinct `Condition` enum variant
- `Condition::Compound` with `operator == "or"` serves this role

### 2c. Remove dead `evaluate_position_change_condition`

- `condition/state.rs:919-929` — marked `#[allow(dead_code)]`, never called from `evaluate_condition` dispatch
- Actual position change check happens via `Condition::Movement`

### 2d. Simplify `check_heart_type_all`

- `condition/card.rs:507-598` reimplements `check_heart_type_all_per_card` (lines 603-635) with `.any()` loop
- Rewrite stage-level version as `cards.iter().any(|id| self.check_heart_type_all_per_card(...))`

---

## Phase 3: Engine DRY — Extract Shared Helpers

### 3a. Extract `collect_zone_cards_with_filter`

The pattern "get cards from zone by card_type, apply group/character filter" is copy-pasted across 4 functions in `state.rs`:

- `execute_set_cost` (lines 769-818)
- `execute_modify_cost` (lines 1192-1277)
- `execute_set_blade_count` (lines 1025-1090)
- `execute_set_blade_type` (lines 821-901)

Extract to a shared helper: `fn collect_zone_cards_with_filter(gs, effect) -> Vec<(i16, &Card)>`
**Estimated savings:** ~120 lines

### 3b. Remove `Zone::EnergyZone`

- `enums.rs:18` — cannot be constructed via `from_str` (line 48 maps `"energy_zone"` to `Zone::Energy`)
- Dead in the zone resolution pipeline

### 3c. Remove dead `ActionType` variants from `is_structural` check

- `effects/mod.rs:932-940` — `ConditionalOptional` may still appear in the `is_structural` match

---

## Phase 4: Parser DRY (parser.py only)

### 4a. Extract `extract_positions(text)` helper

Position keyword extraction is duplicated in 4 places:
- `parse_condition` handler path (lines 1371-1381)
- `parse_condition` fallthrough path (lines 1412-1425)
- `parse_action` (lines 1838-1849)
- `_fill_defaults` (lines 5184-5192)

Extract to single helper. **Estimated savings:** ~50 lines

### 4b. Extract `detect_exclude_self(text)` helper

`"このメンバー以外"` -> `exclude_self: True` is detected in 4 places:
- `parse_action` (line 1852)
- `_extract_generic_fields` (line 4722)
- `_fill_defaults` (line 5141)
- `_walk` (line 8761)

### 4c. Remove dead dispatch rules

| Rule | Line | Why dead |
|---|---|---|
| `引いてもよい` | 2062 | Already matched by `引く` (line 2057) + `extract_optional()` |
| Second `もう一度エール` | 2375 | First match at line 2324 breaks the loop |
| Second `ハート.*得る` | 2429 | First match at line 2298 |

---

## Phase 5: Parser Simplification (parser.py + enums.rs)

Requires both parser and engine changes. Do last.

### 5a. `draw_card` -> `move_cards` with `move_type: "draw"`

`draw_card` is always `move_cards` with `source=deck, dest=hand`. Unify with a flag.

### 5b. `draw_until_count` + `discard_until_count` -> `until_count` with `destination`

Same loop pattern, different destination zone.

### 5c. `modify_required_hearts_global` -> `modify_required_hearts` with `scope: "global"`

Already done in Phase 1a (from_str alias). If parser stops emitting `modify_required_hearts_global`, this becomes a pure rename.

---

## Execution Order

| Phase | Commit message | Risk |
|---|---|---|
| 1a | `refactor: remove 6 unused ActionType variants, alias to surviving` | Low — parser never emits them |
| 1b | `refactor: remove 5 dead internal-only ActionType variants` | Low — log::warn catch-all only |
| 2 | `refactor: remove redundant condition evaluation code` | Low — dead code paths |
| 3 | `refactor: extract shared zone-card-collection helper in state.rs` | Medium — refactor of existing logic |
| 4 | `refactor: parser DRY — extract helpers, remove dead rules` | Low — parser only |
| 5 | `refactor: unify draw_card/move_cards and similar action types` | High — cross-cutting |

Each phase: make changes -> `cargo test` -> commit if green.
