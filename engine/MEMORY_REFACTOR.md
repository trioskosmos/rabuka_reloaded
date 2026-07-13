# Memory Refactor — Rabuka Reloaded

Goal: reduce runtime memory footprint for a potential 3DS port (128 MB budget).

---

## ✅ Done (commit edae728 — AbilityEffect enumification)

**Big structural change:** Converted `AbilityEffect` from a flat struct with ~142 `Option<...>` fields into `EffectKind` enum with 14 variant-specific field sets. Each variant stores only its own fields (MoveCards ~60, DrawCards ~20, GainResource ~15, etc.) instead of all 142 for every ability.

- Added 30+ missing variant fields (`cost_reference`, `cost_offset`, `distinct`, `heart_color`, `replace_all`, `all`, `action_by`, `per_unit_type`, etc.)
- Added `shared_populate_nested()` in card_loader.rs to recursively build EffectKind for ALL nested sub-effects (conditions, options, gained effects, etc.)
- Fixed `clone_from` on `Option<&Vec<String>>` → `set_card_names(names.clone())`
- Fixed `any_number` re-prompt infinite loop (max cap guard)
- **All 1820 tests pass.**

---

## ✅ Done (commit 66dd1ee — More optimisations)

### HeartMap — replaced `HashMap<HeartColor, u32>` with `SmallVec`
- Newtype over `SmallVec<[(HeartColor, u32); 4]>` — zero heap alloc for ≤4 colors
- Applied to `BaseHeart`, `BladeHeart`, `SpecialHeart`
- Updated ~15 source files, ~15 test files

### `Card` string fields — `String` → `Box<str>`
- `card.series`, `card.product`, `card.group`

### `CompoundBranch` — boxed heavy inline fields
- `alternative_condition: Option<Condition>` → `Option<Box<Condition>>`
- `result_condition: Option<Condition>` → `Option<Box<Condition>>`
- **CompoundBranch size: 3080 → 96 bytes** (−2984)

### `Condition` — boxed inline Vecs
- `conditions: Option<Vec<Condition>>` → `Option<Vec<Box<Condition>>>`
- `options: Option<Vec<AbilityEffect>>` → `Option<Vec<Box<AbilityEffect>>>`

### `AbilityEffect` — indirect savings
- Size dropped 5248 → **1536 bytes** (CompoundBranch shrunk)

---

## ✅ Done (current session — Box all Vec<AbilityEffect>)

### Boxed all inline `Vec<AbilityEffect>` fields
| Field | Before | After | Impact |
|-------|--------|-------|--------|
| `CompoundBranch.actions` | `Option<Vec<AbilityEffect>>` | `Option<Vec<Box<AbilityEffect>>>` | 1536→8 bytes per element |
| `AbilityEffect.effect_steps` | `Option<Vec<AbilityEffect>>` | `Option<Vec<Box<AbilityEffect>>>` | 1536→8 bytes per element |
| `EffectKind::SelectTarget.options` | `Option<Vec<AbilityEffect>>` | `Option<Vec<Box<AbilityEffect>>>` | 1536→8 bytes per element |
| `EffectKind::LookReveal.options` | `Option<Vec<AbilityEffect>>` | `Option<Vec<Box<AbilityEffect>>>` | 1536→8 bytes per element |
| `EffectKind::CompoundEffect.options` | `Option<Vec<AbilityEffect>>` | `Option<Vec<Box<AbilityEffect>>>` | 1536→8 bytes per element |
| `EffectKind::MiscOp.options` | `Option<Vec<AbilityEffect>>` | `Option<Vec<Box<AbilityEffect>>>` | 1536→8 bytes per element |
| `EffectKind::CustomOp.options` | `Option<Vec<AbilityEffect>>` | `Option<Vec<Box<AbilityEffect>>>` | 1536→8 bytes per element |

**Cascade fixes**: Updated ~40 usage sites across 8 files (compound.rs, cost.rs, choice.rs, card_loader.rs, effects/mod.rs, effects/misc.rs, describe.rs, resolver.rs).

Note: struct sizes don't change (Vec header is always 24 bytes), but heap allocation per element drops from 1536 to 8 bytes. A choice with 5 options goes from ~7.5 KB inline to ~40 bytes inline + 7.5 KB heap.

### Bonus fixes
- `pending_repeat_actions: Vec<AbilityEffect>` → `Vec<Box<AbilityEffect>>`
- `pending_deferred_costs: Vec<AbilityEffect>` → `Vec<Box<AbilityEffect>>`

**All 1820 tests pass.**

---

## Current type sizes

| Type | Size | Notes |
|------|------|-------|
| `Condition` | **1864** | ~40 `Option<String>` fields (24 bytes each when None) |
| `EffectKind` | **1248** | Largest variant sets size (MoveCards ~85 fields, MiscOp ~90) |
| `AbilityEffect` | **1536** | Still has `CompoundBranch` + `EffectKind` embedded |
| `CompoundBranch` | **96** | Mostly boxed pointers now |
| `Option<String>` | 24 | No niche, always full size |
| `Option<Box<str>>` | 16 | Box pointer niche = 8 bytes saved |
| `Vec<T>` inline in struct | 24 | Pointer + len + cap, elements on heap |
| `Vec<Box<T>>` | 24 | Same header, but `Box<T>` is pointer-sized |

---

## ❌ What remains: ranked by memory impact

### P1 — `Condition` `Option<String>` → `Option<Box<str>>`
**Impact: ~55 fields × 8 bytes = 440 bytes saved per Condition instance.**

Every `Condition` has 1864 bytes — roughly half is `Option<String>` "air" (24 bytes each when None). Changing to `Option<Box<str>>` cuts each to 16 bytes.

Fields in `Condition` (struct at line 6196):
`text`, `location`, `operator`, `card_type`, `target`, `group_reference`, `state`, `position_compare`, `area_direction`, `temporal_scope`, `cost_limit_operator`, `baton_touch_source`, `movement_state`, `energy_state`, `comparison_target`, `comparison_source`, `movement`, `temporal`, `phase`, `phase_target`, `comparison_type`, `appearance_source`, `card_property`, `resource_type`, `activation_position`, `unit`, `from_state`, `heart_type`, `to_state`, `aggregate`, `heart_source`, `ability_filter`, `cost_total_operator`, `scope`, `source`, `destination`, `cost_reference_character`, `cost_reference_operator`, `cost_reference_type`, `reference_card`

Fields in `TriggerEvent` (struct at line 6385):
`event_type`, `tense`, `location`, `source_character`, `source_group`, `ability_filter`, `phase`, `phase_target`, `recurrence`, `source`, `destination`, `from_state`, `to_state`

Fields in `CostComparison` (struct at line 6411):
`operator`, `relative_to`, `cost_limit_operator`

### P1 — `EffectKind` `Option<String>` → `Option<Box<str>>`
**Impact: ~50 fields across variants.** Same 8-byte saving per field.

Fields shared across MoveCards, SelectTarget, LookReveal, MiscOp, etc:
`card_type`, `placement_order`, `state`, `distinct`, `location`, `name_constraint`, `name_constraint_source`, `ability_filter`, `card_property`, `cost_limit_operator`, `cost_total_operator`, `need_heart_operator`, `need_heart_color`, `state_change`, `source_position`, `exclude_position`, `group_reference`, `per_unit_type`, `per_unit_location`, `activation_position`, `choice_type`, `choice_maker`, `question`, `picker`, `cost_reference`, `action_by`, `target_member`, `trigger_type`, etc.

### P2 — `Condition` enumification
**Impact: eliminates 90%+ of `Option<String>` fields entirely.**

Natural variants:
- `ComparisonCondition` — `comparison_type`, `comparison_target`, `operator`, `count`
- `StateCondition` — `state`, `operator`, `count`
- `GroupCondition` — `group_names`, `target`, `location`
- `CharacterCondition` — `characters`, `target`, `location`
- `CompoundCondition` — `conditions: Vec<Box<Condition>>`, `operator`
- `MovementCondition` — `movement`, `source`, `destination`
- `PhaseCondition` — `phase`, `phase_target`
- `EnergyCondition` — `energy_state`, `operator`, `count`
- `HeartCondition` — `heart_type`, `heart_source`, `operator`, `count`
- ... etc.

### P3 — `EffectKind` field enums
Small closed-set strings → enums. E.g. `placement_order` is always `"ascending"` / `"descending"` / `"random"`.

### P3 — `Card` remaining `Option<String>` → `Option<Box<str>>`
`img`, `unit`, `_img` — minor.

---

## Current memory per "average" ability

Rough estimate for a typical ability with 1 condition and 3 option effects:

**Before any work:** ~17 KB
**After edae728 (enumification):** ~8 KB
**After 66dd1ee (HeartMap + boxing):** ~5 KB
**After P0 (box EffectKind Vecs):** ~3 KB
**After P1 (Box<str>):** ~2.5 KB
**After P2 (Condition enum):** ~1.5 KB

---

## How to measure

```
cargo run --bin size_check
```
