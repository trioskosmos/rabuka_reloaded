# Bytecode, Serde & RAM Optimization

## Overview
Audit findings and plan for reducing serde overhead, shrinking bytecode, and minimizing RAM usage in the ability system.

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

- [ ] Profile `STRINGS` table: count frequency of each string across all 800 abilities
- [ ] Select top 60 field names for u8 encoding
- [ ] Update `compile_abilities.py`: reorder STRINGS so common names are first 254 slots, emit u8 for them
- [ ] Update `vm.rs` BcReader: read key index as u8, check for 0xFE escape
- [ ] Update `vm.rs` object decode loop
- [ ] Regenerate `abilities_gen.rs`
- [ ] `cargo test`

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

- [ ] Add variant tag mapping in `compile_abilities.py` (action string -> tag byte 0x01-0x0E)
- [ ] Emit variant tag as first field in each ability effect object
- [ ] Update `vm.rs` decoder: read variant tag, dispatch to variant-specific field reader
- [ ] Write variant-specific field readers that populate `EffectKind` directly
- [ ] Remove `populate_from_json()` call from bytecode decode path
- [ ] Keep `populate_from_json()` for JSON fallback / tests
- [ ] Regenerate `abilities_gen.rs`
- [ ] `cargo test`

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

- [ ] Define `FieldId` enum mapping string names to numeric IDs (shared between Python compiler and Rust decoder)
- [ ] Update `compile_abilities.py` to emit FieldId-based encoding instead of string key indices
- [ ] Write `decode_ability_effect_direct(bc, variant_tag) -> AbilityEffect` in vm.rs
- [ ] Write variant-specific decoders for each EffectKind variant
- [ ] Write `decode_condition_direct(bc) -> Condition` (same approach)
- [ ] Replace `serde_json::from_value` calls with direct decoder
- [ ] Remove `#[derive(Deserialize)]` from `Ability`, `AbilityEffect`, `EffectKind`, `Condition` (keep `Serialize`)
- [ ] `cargo test`

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
    // ... ~50 shared fields
}
```

Each `EffectKind` variant holds `filter: Option<Box<EffectFilter>>` + only variant-specific fields.

### Prerequisites

Phase 4+5 should be done first — with direct binary decoder, serde is gone from the decode path, so restructuring the enum is safe.

### Steps

- [ ] Define `EffectFilter` struct with all shared fields
- [ ] Add `filter: Option<Box<EffectFilter>>` to each `EffectKind` variant
- [ ] Remove shared fields from variants
- [ ] Update direct decoder (Phase 5) to populate filter separately
- [ ] Update all getter methods
- [ ] Update `filter_subset()` / `CardFilter::from_effect()`
- [ ] Update all handler code
- [ ] `cargo test`

### Estimated RAM savings

Enum shrinks from 544 -> ~140 bytes (largest variant with only unique fields). ~400 bytes saved per EffectKind. With 800 cached abilities + EkBox pool: ~390 KB saved.

---

## Summary Table

| Phase | What | RAM savings | Bytecode savings | Complexity | Status |
|---|---|---|---|---|---|
| 1 | Kill JSON round-trips in choice.rs | Removes runtime alloc+parse | — | Low | **DONE** |
| 2 | Downsize u32->u8 | ~540 KB | — | Medium (mechanical) | **DONE** |
| 3 | u8 field indices | — | ~7.5 KB | Low | TODO |
| 4 | EffectKind variant tags | — | — | Medium | TODO |
| 5 | Direct binary decoder | Eliminates Value allocs | — | High | TODO |
| 6 | EffectFilter sub-struct | ~390 KB | — | High | TODO |

**Combined estimated savings:** ~930 KB RAM, ~7.5 KB binary, ~60-80% faster ability decode.

## Execution Order

| Phase | Risk | Commit message | Status |
|---|---|---|---|
| 1 | Low | `refactor: replace conditional_choice JSON string with tagged enum` | DONE |
| 2 | Medium | `refactor: downsize u32/i32 fields to u8/i8 in ability types` | DONE |
| 3 | Low | `perf: single-byte indices for common bytecode field names` | TODO |
| 4 | Medium | `perf: encode EffectKind variant tags in bytecode` | TODO |
| 5 | High | `perf: direct binary decoder, eliminate serde_json::from_value` | TODO |
| 6 | High | `refactor: extract EffectFilter sub-struct from EffectKind` | TODO |

Each phase: make changes -> `cargo test` -> commit if green.
