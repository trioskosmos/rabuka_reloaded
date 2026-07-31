# Bytecode, Serde & RAM Optimization

## Overview
Audit findings and plan for reducing serde overhead, shrinking bytecode, and minimizing RAM usage in the ability system.

## Current Priority: ZERO SERDE for ability decode path

After Phases 1-6, the bytecode decoder still reconstructs `serde_json::Value` objects for 6 complex types, then uses `serde_json::from_value()` to populate them. This is the last serde bottleneck in the hot path.

### Remaining serde calls in ability decode (vm.rs):
| Type | from_value() calls | Complexity |
|------|-------------------|------------|
| `Condition` | 1 (line 374) | HIGH — ~40 enum variants, internally tagged |
| `AbilityEffect` | 2 (lines 841, 958) | HIGH — 15 fields + flattened CompoundBranch |
| `PositionInfo` | 2 (lines 437, 453) | LOW — untagged enum, 2 variants |
| `DynamicCount` | 1 (line 478) | LOW — struct, 4 fields |
| `EffectState` | 2 (lines 498, 514) | LOW — enum, few variants |
| `QuotedText` | 1 (line 650) | LOW — struct, 2 fields |

### Goal
Eliminate ALL `serde_json::from_value()` calls from vm.rs. Remove `#[derive(Deserialize)]` from AbilityEffect, Condition, PositionInfo, DynamicCount, EffectState, QuotedText. Delete `populate_from_json`, `condition_populate_from_json`, `decode_ability_effect_from_object`, `collect_json_map`, `normalize_cost_keys`, `kind_from_action`.

---

## Phase 1: Eliminate Runtime JSON Round-Trips (choice.rs / compound.rs)

**Problem:** `conditional_choice: Option<String>` stores three different types as serialized JSON strings, then deserializes them back at runtime via `serde_json::from_str`.

| Serialize site | Type stored | Deserialize site |
|---|---|---|
| `choice.rs:2143` | `Vec<Box<AbilityEffect>>` | `choice.rs:2109` |
| `compound.rs:912` | `AbilityEffect` | `choice.rs:2948` |
| `effects/draw.rs:582` | `Vec<String>` | `compound.rs:948` |
| `effects/misc.rs:3344` | `Vec<Box<AbilityEffect>>` | `choice.rs:2109` |
| `move_cards.rs:1456` | `Vec<String>` | `compound.rs:948,975` |
| `look.rs:405,696` | `Vec<String>` | `compound.rs:948,975` |
| `effects/misc.rs:222` | `Vec<String>` | `compound.rs:948` |

### Fix

Replace `conditional_choice: Option<String>` on `AbilityQueueEntry` (`ability_queue.rs:61`) with a tagged enum:

```rust
pub enum ConditionalChoice {
    Strings(Vec<String>),
    Effects(Vec<Box<AbilityEffect>>),
    Effect(AbilityEffect),
}
```

- [x] Define `ConditionalChoice` enum in `ability_queue.rs`
- [x] Change field type on `AbilityQueueEntry`
- [x] Update all serialize sites (choice.rs, compound.rs, move_cards.rs, look.rs, effects/misc.rs, effects/draw.rs)
- [x] Update all deserialize sites (choice.rs:2109,2948, compound.rs:948,975)
- [x] Remove `serde_json::to_string` / `serde_json::from_str` calls
- [x] `cargo test`

---

## Phase 2: Downsize Integer Types (card.rs)

**Problem:** Every numeric field uses `u32` (4 bytes) or `i32` (4 bytes) when the real values never exceed ~36. Changing to `u8`/`i8` saves 2-6 bytes per field (with niche optimization).

### Fields to change: `u32` -> `u8`

All `Option<u32>` fields named any of:
- `count`, `target_count` (max 15)
- `cost_limit`, `cost_limit_min`, `cost_limit_max` (max 12)
- `per_unit_count`, `per_group_count` (max 3)
- `cost_total` (max 36)
- `need_heart_total` (max 15)
- `energy_count` (max 12)
- `heart_color_count` (max 6)
- `repeat_limit`, `max_repeats` (max 5)
- `value` (max 15)
- `original_count`, `original_cost` (max 12)
- `blade_limit`, `resource_icon_count` (max 6)
- `use_limit` (max 3)
- `turn_number` (max 20)
- `min_baton_touch_count` (max 3)
- `min_count` (max 15)
- `calculation_value` (max 10)
- `or_card_types` values — already `Vec<String>`, no change needed

**Affected types:** `EffectKind` (all 14 variants), `Condition` (Location, Comparison, Movement, Group, Appearance, Temporal, State, Resource, AbilityFilter, ScoreThreshold, AllRevealedMatchHeartColor), `TriggerEvent`, `CostComparison`, `LocationSubChecks`, `AbilityEffect`, `Ability`, `DynamicCount`, `HeartIcon`

### Fields to change: `i32` -> `i8`

- `EffectKind::MoveCards::cost_offset` (max +/-5)
- `EffectKind::MiscOp::cost_offset` (max +/-5)
- `EffectKind::MiscOp::ref_offset` (max +/-5)

### Fields to change: `u32` -> `u8` (non-Option)

- `Card::cost` (max 12)
- `Card::blade` (max 6)
- `Card::score` (max 15)
- `HeartIcon::count` (max 5)

### Note on HeartMap

`SmallVec<[(HeartColor, u32); 4]>` — all values are heart counts (max 5). Changing to `u8` touches the `HeartMap` Index/IndexMut impls, serde custom impls, and every call site. Do this separately.

### Steps

- [x] Change `Ability` struct: `use_limit: Option<u32>` -> `Option<u8>`
- [x] Change `AbilityEffect` struct: `count: Option<u32>` -> `Option<u8>`
- [x] Change all `EffectKind` variant fields (bulk find-replace per type above)
- [x] Change all `Condition` variant fields
- [x] Change `TriggerEvent`, `CostComparison`, `LocationSubChecks`, `DynamicCount`, `HeartIcon`
- [x] Change `Card` numeric fields
- [x] Update `i32` -> `i8` for cost_offset, ref_offset
- [x] Update all getter macros: `u32_getter!` returns `Option<u8>` now (rename to `u8_getter!` or parameterize)
- [x] Update all setter macros
- [x] Update handler code that does arithmetic on these fields (cast/compare)
- [x] `cargo test`

### Estimated RAM savings

~113 fields across EffectKind variants. Each `Option<u32>` is 8 bytes (niche-optimized). Each `Option<u8>` is 2 bytes. Savings: ~113 * 6 = ~678 bytes per `EffectKind` instance. With 800 cached abilities, ~540 KB saved. EkBox pool (128 slots) saves ~76 KB.

---

## Phase 3: Bytecode Size Reduction — Single-Byte Field Indices

**Problem:** Every JSON key in the bytecode is a u16 index into `STRINGS`. Common keys like `"action"`, `"source"`, `"target"`, `"card_type"` appear hundreds of times across 800 abilities.

### Fix

Reserve indices 0-255 for ~60 most common field names. Encode as u8 tag byte. Only uncommon field names use u16.

| Byte value | Meaning |
|---|---|
| `0x00-0xFD` | u8 index into first 254 string slots |
| `0xFE` | Next byte is u16 index (little-endian) |
| `0xFF` | Next 2 bytes are u16 index (for future expansion) |

### Steps

- [x] Profile `STRINGS` table: count frequency of each string across all 800 abilities
- [x] Select top 60 field names for u8 encoding
- [x] Update `compile_abilities.py`: reorder STRINGS so common names are first 254 slots, emit u8 for them
- [x] Update `vm.rs` BcReader: read key index as u8, check for 0xFE escape
- [x] Update `vm.rs` object decode loop
- [x] Regenerate `abilities_gen.rs`
- [x] `cargo test`

### Estimated savings

Each ability has ~10-15 fields. Each key index saves 1 byte (u16->u8). ~800 abilities * ~12 fields * ~80% common = ~7700 bytes (~7.5 KB). Modest but compounds with other savings.

---

## Phase 4: Bytecode — Encode EffectKind Variant Tags

**Problem:** Bytecode stores the full JSON shape (`{"action":"gain_resource",...}`). At runtime: bytecode -> `serde_json::Value` -> `serde_json::from_value::<AbilityEffect>()` -> `populate_from_json()` walks the same JSON again to build `EffectKind`. Two full passes.

### Fix

The Python compiler already maps `action` -> `EffectKind` variant. Emit a variant tag byte after the object header:

```
TAG_OBJECT count
  [u8 variant_tag, u8 action_key_idx, ...]
```

The Rust decoder reads the tag, creates the correct `EffectKind` variant directly. Fields are deserialized straight into the variant — no `populate_from_json()` needed.

### Steps

- [x] Add variant tag mapping in `compile_abilities.py` (action string -> tag byte 0x01-0x0E)
- [x] Emit variant tag as first field in each ability effect object
- [x] Update `vm.rs` decoder: read variant tag, dispatch to variant-specific field reader
- [x] Write variant-specific field readers that populate `EffectKind` directly
- [x] Remove `populate_from_json()` call from bytecode decode path
- [x] Keep `populate_from_json()` for JSON fallback / tests
- [x] Regenerate `abilities_gen.rs`
- [x] `cargo test`

### Estimated savings

Eliminates one full JSON tree walk per ability decode. Reduces per-ability decode cost by ~40-50%. No binary size change (same data, different layout).

---

## Phase 5: Direct Binary Decoder (Kill `serde_json::Value`)

**Problem:** The bytecode decoder reconstructs a `serde_json::Value` tree (heap allocations for every string, array, object), then `serde_json::from_value` does dynamic type checking to fill struct fields.

### Fix

Write a hand-coded binary decoder that reads bytecode directly into `AbilityEffect` + `EffectKind` structs. The decoder:

1. Reads the variant tag (Phase 4)
2. For each field, reads the key index, matches it against a static dispatch table for the current variant
3. Reads the value directly into the correct struct field using the tag type
4. No intermediate `serde_json::Value` allocation

The Python compiler generates a field-name -> field-offset lookup table per EffectKind variant. The Rust decoder uses this for O(1) dispatch instead of string matching.

### Steps

- [x] Define `FieldId` enum mapping string names to numeric IDs (shared between Python compiler and Rust decoder)
- [x] Update `compile_abilities.py` to emit FieldId-based encoding instead of string key indices
- [x] Write `decode_ability_effect_direct(bc, variant_tag) -> AbilityEffect` in vm.rs
- [x] Write variant-specific decoders for each EffectKind variant (auto-generated)
- [ ] Write `decode_condition_direct(bc) -> Condition` (same approach)
- [x] Replace `serde_json::from_value` calls with direct decoder
- [ ] Remove `#[derive(Deserialize)]` from `Ability`, `AbilityEffect`, `EffectKind`, `Condition` (keep `Serialize`)
- [x] `cargo test`

### Estimated savings

- Eliminates ALL intermediate `serde_json::Value` heap allocations during decode
- Eliminates serde derive overhead from compile time
- Reduces per-ability decode cost by ~60-80% vs current approach
- Binary size unchanged

---

## Phase 6: Reduce EffectKind Field Duplication

**Problem:** The `EffectKind` enum has 14 variants with ~50 fields duplicated across 8-14 variants each. This makes the enum 544 bytes per variant (determined by MiscOp). Each field also has `#[serde(default)]` attributes adding to compile time.

### Fix

Extract shared fields into an `EffectFilter` sub-struct:

```rust
pub struct EffectFilter {
    pub card_type: Option<CardType>,
    pub group_names: Option<Box<Vec<String>>>,
    pub exclude_self: Option<bool>,
    // ... ~71 shared fields
}
```

Each `EffectKind` variant holds `filter: Option<Box<EffectFilter>>` + only variant-specific fields.

### Steps

- [x] Define `EffectFilter` struct with all shared fields
- [x] Add `filter: Option<Box<EffectFilter>>` to each `EffectKind` variant
- [x] Remove shared fields from variants
- [x] Update direct decoder (Phase 5) to populate filter separately
- [x] Update all getter methods
- [x] Update all setter methods
- [x] Update `generate_effect_decoder.py` to produce `build_filter(ek)` + decoder arms for filter fields
- [x] `cargo test`

### Estimated RAM savings

Enum shrinks from 544 -> ~140 bytes (largest variant with only unique fields). ~400 bytes saved per EffectKind. With 800 cached abilities + EkBox pool: ~390 KB saved.

---

## Phase 7: Zero Serde — Eliminate All serde_json::from_value from Ability Decode

**Problem:** After Phases 4-6, the bytecode decoder still reconstructs `serde_json::Value` trees for complex sub-objects (Condition, AbilityEffect, PositionInfo, DynamicCount, EffectState, QuotedText), then calls `serde_json::from_value()` to populate them. This means every ability decode still heap-allocates JSON trees and runs full serde deserialization for these types.

### Step 1: Write direct decoders for simple types (LOW risk)

Port these simple types first — few fields, few variants:

- [ ] `decode_position_info_direct(bc) -> Option<PositionInfo>` — untagged enum, 2 variants (Single/All)
- [ ] `decode_dynamic_count_direct(bc) -> Option<Box<DynamicCount>>` — struct, 4 fields
- [ ] `decode_effect_state_direct(bc) -> Option<Box<EffectState>>` — enum, few variants
- [ ] `decode_quoted_text_direct(bc) -> Option<Box<QuotedText>>` — struct, 2 fields
- [ ] Replace `serde_json::from_value()` calls with direct decoders in vm.rs
- [ ] Remove `#[derive(Deserialize)]` from PositionInfo, DynamicCount, EffectState, QuotedText
- [ ] Add these types to `generate_effect_decoder.py` or write decoder by hand
- [ ] `cargo test`

### Step 2: Port AbilityEffect to direct decode (MEDIUM risk)

- [ ] Write `decode_ability_effect_direct(bc) -> AbilityEffect` that reads ALL fields directly (no serde)
- [ ] This already partially exists — extend to populate AbilityEffect + CompoundBranch fields
- [ ] Remove `serde_json::from_value::<AbilityEffect>()` calls (vm.rs lines 841, 958)
- [ ] Remove `#[derive(Deserialize)]` from AbilityEffect and CompoundBranch
- [ ] Delete `populate_from_json()` — only called from old path
- [ ] Delete `decode_ability_effect_from_object()` — TAG_OBJECT fallback
- [ ] Delete `collect_json_map()` — only called by above
- [ ] `cargo test`

### Step 3: Port Condition to direct decode (HIGH risk)

- [ ] Write `decode_condition_direct(bc) -> Condition` — ~40 variants, internally tagged
- [ ] Port each Condition variant: Location, Comparison, Movement, Group, Appearance, Temporal, State, Resource, AbilityFilter, ScoreThreshold, AllRevealedMatchHeartColor, etc.
- [ ] Port `LocationSubChecks`, `TriggerEvent`, `CostComparison` as sub-decoders
- [ ] Replace `serde_json::from_value::<Condition>()` call (vm.rs line 374)
- [ ] Delete `condition_populate_from_json()` — only called from read_condition_value
- [ ] Remove `#[derive(Deserialize)]` from Condition and all its sub-types
- [ ] `cargo test`

### Step 4: Port cost decode + clean up (LOW risk)

- [ ] Write `decode_ability_cost_direct(bc)` for TAG_OBJECT cost objects
- [ ] Delete `normalize_cost_keys()` / `recursive_normalize_cost_value()` — only needed for JSON path
- [ ] Delete `kind_from_action()` — only called by old JSON path
- [ ] Remove TAG_OBJECT fallback from `decode_ability_effect()` — all effects now use TAG_OBJECT_VARIANT
- [ ] Remove `#[derive(Deserialize)]` from AbilityCost
- [ ] Delete `populate_from_json`, `condition_populate_from_json`, `decode_ability_effect_from_object`, `collect_json_map`
- [ ] Remove ~300 lines of serde infrastructure from vm.rs and card.rs
- [ ] `cargo test`

### Summary of deletions after all steps

| Function/Attribute | Lines | Step |
|---|---|---|
| `serde_json::from_value::<PositionInfo>` | — | 1 |
| `serde_json::from_value::<DynamicCount>` | — | 1 |
| `serde_json::from_value::<EffectState>` | — | 1 |
| `serde_json::from_value::<QuotedText>` | — | 1 |
| `#[derive(Deserialize)]` on PositionInfo, DynamicCount, EffectState, QuotedText | — | 1 |
| `serde_json::from_value::<AbilityEffect>` (2 calls) | — | 2 |
| `#[derive(Deserialize)]` on AbilityEffect, CompoundBranch | — | 2 |
| `populate_from_json()` | ~60 | 2 |
| `decode_ability_effect_from_object()` | ~30 | 2 |
| `collect_json_map()` | ~20 | 2 |
| `serde_json::from_value::<Condition>` | — | 3 |
| `condition_populate_from_json()` | ~50 | 3 |
| `#[derive(Deserialize)]` on Condition + 30 sub-types | — | 3 |
| `normalize_cost_keys()` / `recursive_normalize_cost_value()` | ~35 | 4 |
| `kind_from_action()` | ~80 | 4 |
| `decode_ability_cost` (JSON fallback) | ~30 | 4 |
| TAG_OBJECT fallback path | ~20 | 4 |
| `#[derive(Deserialize)]` on AbilityCost | — | 4 |
| **Total** | **~400+ lines** | |

### Estimated savings

- Eliminates ALL `serde_json::Value` heap allocations during ability decode
- Eliminates serde derive proc-macro overhead (~38 types × compile time)
- Reduces per-ability decode cost by ~60-80% vs current approach
- Enables removing serde from the `no_std` / embedded build

## Summary Table

| Phase | What | RAM savings | Status |
|---|---|---|---|
| 1 | Kill JSON round-trips in choice.rs | Removes runtime alloc+parse | **DONE** |
| 2 | Downsize u32->u8 | ~540 KB | **DONE** |
| 3 | u8 field indices | ~7.5 KB binary | **DONE** |
| 4 | EffectKind variant tags | Infrastructure | **DONE** |
| 5 | Direct binary decoder for EffectKind | Eliminates Value allocs | **DONE** |
| 6 | EffectFilter sub-struct | ~390 KB | **DONE** |
| 7.1 | Direct decoders: PositionInfo, DynamicCount, EffectState, QuotedText | — | **NEXT** |
| 7.2 | Direct decoder: AbilityEffect | — | TODO |
| 7.3 | Direct decoder: Condition (~40 variants) | — | TODO |
| 7.4 | Cost decode + cleanup (~400 lines deleted) | — | TODO |

**Achieved savings:** ~540 KB RAM, ~29 KB binary, EffectKind 544→~140 bytes.
**Current priority:** Phase 7 — eliminate ALL serde_json::from_value from the ability decode path.

---

## Execution Order

| Phase | Risk | Commit message | Status |
|---|---|---|---|
| 1 | Low | `refactor: replace conditional_choice JSON string with tagged enum` | DONE |
| 2 | Medium | `refactor: downsize u32/i32 fields to u8/i8 in ability types` | DONE |
| 3 | Low | `perf: single-byte indices for common bytecode field names` | DONE |
| 4 | Medium | `perf: encode EffectKind variant tags in bytecode` | DONE |
| 5 | High | `perf: direct binary decoder for EffectKind` | DONE |
| 6 | High | `refactor: extract EffectFilter sub-struct from EffectKind` | DONE |
| 7.1 | Low | `perf: direct decoders for PositionInfo, DynamicCount, EffectState, QuotedText` | NEXT |
| 7.2 | Medium | `perf: direct decoder for AbilityEffect, remove serde from effect path` | TODO |
| 7.3 | High | `perf: direct decoder for Condition, remove serde from condition path` | TODO |
| 7.4 | Medium | `perf: direct cost decoder, delete all serde infrastructure (~400 lines)` | TODO |

Each phase: make changes -> `cargo test` -> commit if green.
