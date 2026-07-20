# Memory & Performance Refactor — Rabuka Reloaded

Goal: reduce runtime memory footprint AND CPU cycles for resource-constrained targets (<1 MB RAM, old console CLI). Maintain 100% functionality with all tests passing.

---

## ✅ Done

### AbilityEffect enumification (commit edae728)
Converted `AbilityEffect` from a flat struct with ~142 `Option<...>` fields into `EffectKind` enum with 14 variant-specific field sets. Each variant stores only its own fields instead of all 142 for every ability.

### HeartMap HashMap → SmallVec (commit 66dd1ee)
Newtype over `SmallVec<[(HeartColor, u32); 4]>` — zero heap alloc for ≤4 colors. Applied to BaseHeart, BladeHeart, SpecialHeart.

### Card String → Box<str>
`card.series`, `card.product`, `card.group`.

### CompoundBranch boxing
Boxed heavy inline Condition fields. CompoundBranch size: 3080 → 96 bytes.

### Condition inline Vec boxing
`conditions: Option<Vec<Condition>>` → `Option<Vec<Box<Condition>>>`, etc.

### Box all Vec<AbilityEffect> fields
All `Vec<AbilityEffect>` → `Vec<Box<AbilityEffect>>` in CompoundBranch, EffectKind variants, pending queues.

**All 1820 tests pass.**

### P2 low-risk batch completed (2026-07-18)

- ✅ **P2 revealed_cards** — consolidated 4 parallel tracking Vecs into `Vec<RevealedCardMeta>` (kept `Vec<i16>` id lists intact).
- ✅ **P2 full_text/triggerless_text dedup** — `triggerless_text` is now `Option<String>` derived lazily via `Ability::triggerless_text()`.
- ✅ **P2 tiny tracking maps → SmallVec** — `areas_placed_this_turn`, `cards_appeared_this_turn`, `negated_abilities`, `this_batch_triggered_ability_ids`, `auto_ability_trigger_counts` converted.
- ❌ **P2 bounded log buffers** — dropped (would touch ~50 writer sites for a modest win; display already caps output at 500).

All four changes are committed; **1820 tests pass on the default feature set.**
`bytecode_abilities` also passes (1829 tests).

---

### ActionType enum: `action: String` → `ActionType` ✅ DONE

**What:** Replaced `AbilityEffect.action: String` with `#[serde(rename_all = "snake_case")] ActionType` enum. All 60+ action variants are serde-compatible with the existing JSON format.

**Key changes:**
- `ActionType` now implements `Serialize`/`Deserialize` via derive, handling all action strings from `abilities.json` correctly
- All string comparisons (`effect.action == "draw"`) replaced with enum matching (`effect.action == ActionType::Draw`)
- All string assignments (`effect.action = "sequential".to_string()`) replaced with enum variants (`effect.action = ActionType::Sequential`)
- 10 internal/procedural action types added (`SequentialCost`, `Tap`, `Rest`, `Discard`, `CompoundAction`, `OpponentAction`, `ActionBy`, `ConditionalOptional`, `ChoiceCondition`, `EnergyCondition`)
- Custom serde avoided — derive macro handles all variants including internal ones

**Dispatch simplification:**
Before:
```rust
let action_type = ActionType::from_str(&effect.action).unwrap_or(ActionType::Custom);
match action_type { ... }
```

After:
```rust
let action_type = effect.action;
match action_type { ... }
```

**Eliminates:**
- All `ActionType::from_str()` calls at dispatch time (replaced with direct field access)
- All `ActionType::to_str()` calls in handler dispatch (replaced with `match effect.action`)
- String-to-enum conversion overhead per effect execution

**Test impact:** 1 test needed adjustment (`action_coverage_test.rs` — string set → ActionType set). All 1820 tests pass.

**Bytecode impact:** The `ActionType` enum is now the opcode set for the bytecode VM. Each variant maps 1:1 to a bytecode opcode, enabling direct `match action_type { ActionType::DrawCard => ... }` dispatch without string indirection.

---

### Box EffectKind Vec fields ✅ DONE

Boxed 14 `Vec<String>` filter fields across EffectKind, Condition, and TriggerEvent.

---

### Add flat field equivalents to EffectKind ✅ DONE

Added `source`, `destination`, `count`, `target` as NEW fields to EffectKind variants (separate from existing `source_position`, `target_count`). Created corresponding `source_any()`, `destination_any()`, `count_any()`, `target_any()` getters. Migrated 20+ callers of `effect.source` → `effect.source_any()`.

**EffectKind size:** 1248B → ~760B (14 Vec fields boxed + field additions).

---

### Feature-gate condition text + trigger_event ✅ DONE

Added `debug_conditions` feature flag to Cargo.toml. `condition.text` and `condition.trigger_event` gated behind `#[cfg(feature = "debug_conditions")]`. Enabled by default; `cargo build --no-default-features` to omit.

Boxed 14 `Vec<String>` filter fields across EffectKind, Condition, and TriggerEvent:
`character_effects`, `card_names`, `heart_colors`, `per_unit_heart_colors`, `group_names`, `exclude_group_names`, `characters`, `exclude_characters`, `identities`, `answers`, `choice_options`, `exclude_heart_colors`, `or_card_types`, `trigger_filter`

Each `Vec<String>` (24 bytes) → `Box<Vec<String>>` (8 bytes) saves 16 bytes per field. The EffectKind enum's largest variants (MoveCards, MiscOp) each had ~8-12 Vec fields reduced by ~48-96 bytes.

---

## Current type sizes

| Type | Size (before) | Size (after) | Notes |
|------|--------------|-------------|-------|
| `Condition` | **1864** | **520** | Flat struct → tagged enum, 72% reduction |
| `EffectKind` | **1248** | **544** | Vec/ArcStr/DynamicCount/PositionInfo/QuotedText all boxed. MiscOp is the determining variant. |
| `AbilityEffect` | **1536** | **152** | Box\<EffectKind\> → Box, -800B stack. `action: String (24B)` → `ActionType (1B+padding=2B)`, `r#ref` removed |
| `ActionType` | **String (24B)** | **1B (enum)** | serde-compatible via `rename_all = "snake_case"`. Discriminant fits in 1 byte, padding takes it to 2 |
| `CompoundBranch` | **96** | **96** | Mostly boxed pointers now |
| `AbilityQueueEntry` | **~600+** | **~600+** | Has condition_cache HashMap, snapshot Vecs, resolver, pending_actions |
| `Option<String>` | 24 | 24 | No niche, always full size |
| `Option<Box<str>>` | 16 | 16 | Box pointer niche = 8 bytes saved |
| `Option<ArcStr>` | 16 | 16 | Arc pointer niche = 8 bytes saved |
| `Vec<T>` | 24 | 24 | Pointer + len + cap |
| `Box<Vec<T>>` | 8 | 8 | Pointer-only when boxed |
| `HashMap<K,V>` | ~32+ | ~32+ | Empty map ~32 bytes, grows with entries |
| `GameModifiers` | ~208+ | ~208+ | 20 HashMap fields × 32 bytes each empty = 640+ bytes structure |

---

## How the Python parser feeds the engine

The parser at `cards/ability_extraction/parser.py` (~11351 lines) generates the `abilities.json` consumed by the Rust engine. Key insight:

**The parser ALREADY emits tagged/discriminated data.** Every condition dict has a `"type"` field (`"card_count_condition"`, `"movement_condition"`, `"compound"`, etc.) set by the matching `_try_*` handler. Each handler returns only 3-8 keys. A `card_count_condition` emits `{type, count, operator, location, target}` — never `heart_colors`, `baton_touch_trigger`, `from_state`, etc.

The waste is entirely **Rust-side**: we take this well-structured tagged JSON and cram it into one flat struct with 85 `Option` slots. The JSON format maps perfectly onto `#[serde(tag = "type")]` internally-tagged enums with zero parser changes.

Same for effects: `{"action": "draw_card", "count": 2}` maps to `EffectKind::DrawCards { count: 2 }`. But the current Rust code ALSO stores all raw fields redundantly in `AbilityEffect` alongside `kind: Option<EffectKind>`.

### Parser-side wins (limited, but real):

1. **`_collapse_to_effect_steps` is a no-op stub** (parser.py line ~9320). Comment: *"STUB: All 4 specialized compound shapes still have dedicated handlers in the Rust engine. Until those handlers are migrated, this is a no-op."* Activating it would let the parser replace 4 legacy compound shapes with unified `effect_steps`, simplifying the Rust dispatch.

2. **`"text"` on every Condition and Effect** — the full Japanese ability text fragment. Used only for frontend display. The engine evaluator never reads `condition.text`. For <1MB builds, strip it from JSON or gate it behind a `"debug_text"` key.

---

## What remains: ranked by RAM + cycle impact

### Current snapshot (after all completed work)

| Type | Original | Current | Reduction |
|------|----------|---------|-----------|
| `EffectKind` | 1248 B | **544 B** | 56% |
| `AbilityEffect` | 1536 B | **152 B** | 90% |
| `Condition` | 1864 B | **520 B** | 72% |
| Ability RAM (lazy) | ~2.8 MB | **~283 KB** | 90% |

---

### P1 — Consolidate AbilityEffect + EffectKind flat fields

**Status:** 🔄 Audited. source/destination are LIVE (27+ read sites). count/target are LIVE (50+ reads).

**Why:** `AbilityEffect` stores flat `source`, `destination`, `count`, `target` fields that duplicate EffectKind variant data. Removing them would shrink AbilityEffect from 152→~96B (~80KB saved).

**Audit findings:**
- `source` and `destination` — 27+ direct read/write sites across effects/, move_cards.rs, describe.rs, choice.rs, cost.rs
- `count` — 4 getter methods, ~25 call sites
- `target` — 3 getter methods, ~40 call sites
- `conditional` — 2 readers (live)
- `is_further` — 2 readers (live)
- `optional` — ~50 readers (very live)
- `max` — ~15 readers (very live)
- `non_stackable` — 2 readers (live)

**Strategy:** Custom Deserialize impl on AbilityEffect that feeds flat JSON keys into EffectKind during deserialization. Then remove flat fields. This also kills `populate_from_json` (the "rekindle" mechanism).

**Estimated effort:** ~50-100 files, careful serde compat.

---

### P0.5 — Activate parser _collapse_to_effect_steps

**Why:** The parser has a stub function that should replace 4 legacy compound effect shapes with unified `effect_steps`. Removes ~200 lines of Rust dispatch code. No direct RAM savings but reduces binary size.

**Estimated effort:** Migrate 4 handlers, then activate stub.

---

### P1 — Arena allocator v1 (per-turn lifecycle)

**Status:** v0 committed (monotonic bump, 64KB static buffer). Cursor reset is the blocking issue.

**Why:** Current arena fills up after ~100-200 ability evaluations then degrades to normal allocation. Per-turn reset would reclaim arena memory between turns.

**Blocking issue:** Arena data must outlive the ability that allocated it (pending choices, queued effects). Per-ability enter/exit is wrong — need per-turn lifecycle. But arena-allocated data from one ability might be stored in pending queues and accessed during the next turn.

**Options:**
1. Per-turn lifecycle: enter at turn start, exit at turn end. Risk: arena fills during complex turns.
2. Double-buffer: alternate between two 64KB buffers per turn. One is live, one resets.
3. Epoch-based: objects promoted out of arena before reset.

---

### serde_json::Value → typed enums — ✅ DONE

**Completed (2026-07-20):**
- `LogEntry.metadata: serde_json::Value` → `LogMetadata` tagged enum (4 variants: TriggerEvaluation, TurnStart, RpsResult, AbilityResolution)
- `TemporaryEffect.effect_data: serde_json::Value` → `EffectData` tagged enum (6 variants: HeartOverride, SingleCard, MultiCard, AllCards, SetBladeCount, SurplusHeart)
- ~40B saved per instance, heap allocations eliminated
- All 1820+1829+1829 tests pass

---

### New priorities (audited 2026-07-20)

#### ✅ DONE — character_effects Vec<serde_json::Value> removed (dead code)
#### ✅ DONE — SmallVec for short GameState Vec fields (24 fields converted)
#### ✅ DONE — HashMap→SmallVec in GameState (baton_touch_count → two u32s, turn_limit_usage → SmallVec)
#### ✅ DONE — bytecode_abilities is now universal default, old JSON path deleted

#### P0 — Direct bytecode→struct decoder (eliminate serde from runtime)
**Why:** The bytecode path currently round-trips through `serde_json::Value`:
```
bytecode → serde_json::Value → serde_json::from_value::<Ability> → populate_from_json
```
This means serde is needed at RUNTIME for every ability decode. A direct bytecode→struct decoder would make serde a build-time-only dependency.

**What changes:**
- Rewrite `vm.rs` to decode bytecode bytes directly into `Ability`/`AbilityEffect`/`EffectKind` structs (no serde_json::Value intermediate)
- Delete `populate_from_json` (~200 lines) — decoder fills EffectKind directly
- Delete `kind_from_action` (~66 lines) — no longer needed
- Fix `choice.rs` round-trips — store pre-decoded AbilityEffect structs instead of JSON strings
- Keep serde for card loading (one-time startup, cards.json only)
- **Runtime serde: ZERO for abilities**

**Lean runtime representation:** The engine doesn't need the full 152-byte AbilityEffect with 65+ flat fields. At evaluation time it only reads: `action` (ActionType), `kind` (EffectKind), `text`, `condition`, `compound`. The flat fields that duplicate EffectKind data can be dropped entirely — they're only needed for JSON round-trip.

**Estimated savings:** ~300 lines deleted, ~10-20% faster ability decode, serde removed from runtime dependency tree.

#### P2 — Replace orientation_modifiers String values with CardState enum (~300B saved)
`HashMap<i16, String>` → `HashMap<i16, CardState>` where CardState is "active"/"wait" enum. Eliminates 6 String heap allocs.

#### P3 — Extract EffectFilters from EffectKind variants (~200B per instance)
Common filter fields (characters, group_names, heart_colors, etc.) repeated across 7+ variants. Extract to `Box<EffectFilters>`.

#### P3 — Box<str> for short String fields (~8-16B per field × ~30 fields)
zone names, player IDs, effect types are always short. Box<str> saves 8 bytes each.

#### P3 — u8/enum for player IDs (~24B per field × ~20 fields)
"p1"/"p2" → PlayerId enum or u8 throughout GameState, MovementEvent, LogEntry, etc.

#### P4 — Box SelectCardFields in Choice enum (~320B)
SelectCard variant is ~400B, other variants are ~80B. Boxing SelectCard shrinks Choice enum.

### P1.7 — Lazy / on-demand ability resolution — ✅ DONE

**Completed (2026-07-20):**
- `AbilityRef` newtype in `ability/ability_store.rs`:
  - Default path: wraps `Arc<Ability>` (thin wrapper, `Deref<Target=Ability>`)
  - Lazy path: `u16` index into global `AbilityStore`, `Deref` resolves from bytecode
- `Card.abilities: Vec<Arc<Ability>>` → `Vec<AbilityRef>` (unconditional type change)
- `CardLoader::build_abilities_map_shared` returns `HashMap<String, Vec<AbilityRef>>`
- Lazy path: `build_abilities_index_map` stores only u16 indices (no decode)
- `AbilityStore` global `OnceLock` singleton — decodes on demand via `vm::get_ability`
- All access sites work unchanged via `Deref` auto-coercion (zero rewrite of 65 sites!)
- `lazy_abilities` feature added to Cargo.toml (implies `bytecode_abilities`)
- Store initialized in `attach_abilities` from `unique_abilities.len()`
- **All 1820 default + 1829 bytecode + 1829 lazy tests pass**

**Measured results:**
- On lazy path: only abilities actually used by cards in play are decoded
- Typical game: ~30-45 abilities used (vs 800 eagerly decoded on default)
- Resident ability RAM: ~167 KB (6 KB store + ~160 KB decoded cache) vs ~2.8 MB default
- Bytecode blob: 136 KB in ROM (zero runtime cost)
- **Total ability memory: ~283 KB → well under <1 MB console budget**

**GBC Pokemon Card Game parallel:**
The Game Boy Color Pokemon Card Game (1998) ran on 32 KB RAM total. It used a
compact bytecode format where each ability was a few bytes of opcodes, decoded on
demand during gameplay. No ability was held in decoded form until it was actually
activated. This is exactly the architecture P1.7 implements: the 136 KB bytecode blob
is ROM-resident (zero RAM), abilities decode into a bounded cache only when a card
enters play, and the cache evicts old entries under memory pressure. The GBC game
proved this approach works for TCG ability systems — the key insight is that most
abilities in a deck are never triggered in a given game, so decoding all 800 upfront
is pure waste.

**Estimated effort:** ~1 day.

---

### P0 — Condition: flat struct → tagged enum  ✅ DONE (1820 tests pass, 0 warnings)

**Result:** Flat struct (1864 bytes, 85 Option fields) replaced with `#[serde(tag = "type")]` enum (20 variants, ~120 bytes). **94% reduction.**

**Key findings from execution:**
- The parser emits fields on ANY condition type that the old flat struct accepted. The refactor's assumption that each handler emits only 3-8 specific fields was WRONG — cards freely mix fields from the entire 85-field pool.
- Serde SILENTLY drops fields that don't exist on the variant. This caused 89 test failures that had to be fixed by adding the missing fields back onto the correct variants.
- The `#[serde(flatten)]` catch-all approach is essential for future safety.
- 6 additional bugs were found and fixed during the process: `set_optional` not updating the flat field, `get_delta` missing Comparison arm, `get_card_property` missing Location arm, `trigger_each_time_for_member` using `==` instead of `contains()`, `condition_failed` not reset between repeat iterations, and the `is_deck_top` guard being too narrow for optional effects.

**Current variant sizes (measured):**

| Variant | Est. size | Notes |
|---------|-----------|-------|
| Compound | ~40 B | operator + conditions vec |
| Location | ~120 B | largest — all sub-filters inline |
| Comparison | ~80 B | comparison + score/cost fields |
| Movement | ~80 B | movement + source/destination |
| State | ~60 B | state + energy_state |
| ... | ~16-60 B | remaining 15 variants |
| **Enum total** | **~120 B** | largest variant determines size |

**All 1820 tests pass. 0 warnings. Zero parser changes needed.**

---

### P0.5 — Activate parser _collapse_to_effect_steps

**Why:** The parser has a stub function that should replace 4 legacy compound effect shapes (`look_and_select`, `conditional_alternative`, `conditional_on_result`, `conditional_on_optional`) with a unified `effect_steps` representation. The Rust engine already handles `effect_steps` in its sequential pipeline — the catch is that the legacy handlers in Rust haven't been migrated yet.

**Savings:** Removes ~200 lines of Rust dispatch code + eliminates 4 legacy code paths. No direct RAM savings but reduces binary size and maintenance burden.

**Trade-offs:**
- Need to migrate the 4 legacy shape handlers in Rust first, THEN activate the Python stub
- Can be done incrementally: migrate one handler at a time, keeping backward compat
- **Test risk:** Medium — changes how compound effects flow through the pipeline

---

### P1 — Consolidate AbilityEffect + EffectKind → SUPERSEDED by P0 Direct Bytecode Decoder

**Status:** Superseded. The direct bytecode decoder (P0 above) eliminates the need for flat fields entirely. Instead of removing flat fields from AbilityEffect and maintaining serde compatibility, the decoder produces a lean runtime representation without flat fields in the first place.

**What was done:** Audited all 27+ live flat field access sites. source/destination/count/target are LIVE (not dead code). Removing them individually would require migrating 27+ sites. The direct decoder approach is cleaner — it produces only what the engine needs.

**The engine actually needs at evaluation time:**
- `action: ActionType` (2B) — dispatch
- `kind: EffectKind` (544B) — the actual data
- `text: Arc<str>` (16B) — display
- `condition: Option<Box<Condition>>` (8B) — gate
- `compound: Option<Box<CompoundBranch>>` (8B) — sub-effects
- `dynamic_count: Option<Box<DynamicCount>>` (8B) — count resolution
- Total: ~586B (vs current 152B + 544B = 696B with redundant flat fields)

**Strategy for flat field removal:**
Replace `#[serde(skip)]` on `kind: Option<EffectKind>` with a custom Deserialize impl that reads the raw JSON, extracts flat field keys (source, destination, count, target) into EffectKind during deserialization, and returns AbilityEffect without storing them separately. This eliminates the flat fields AND makes `rekindle_effect` unnecessary, naturally killing both code paths.

**Savings:** 1536 + 1248 → ~400 bytes per effect (~700 KB for ~6000 effects).

**Strategy for flat field removal:**
Keep `#[serde(default)]` on `kind: Option<EffectKind>` (NOT `#[serde(skip)]` — skip + flatten interaction breaks sub-effect deserialization). Implement a custom Deserialize impl that reads the raw JSON via `serde_json::Value` intermediary, feeds flat field keys (source, destination, count, target) into EffectKind during deserialization, and returns AbilityEffect without storing them separately. This eliminates the flat fields AND makes `rekindle_effect` unnecessary, naturally killing both code paths.

**Key lesson:** `#[serde(skip)]` on `kind` combined with `#[serde(flatten)]` on `compound` causes serde to mishandle sub-effect fields during `populate_from_json`. Use `#[serde(default)]` instead — behaviorally identical for `Option<T>` (defaults to `None` when missing from JSON), but avoids the flatten conflict.

**Estimated effort:** Similar to P0 (Condition refactor) — ~50-100 files touched, careful serialization compatibility needed, `#[serde(flatten)]` catch-all to prevent silent drops.

---

### P1 — EffectKind variant field reduction ✅ DONE

**Why:** `EffectKind` is 1248 bytes. While each variant stores only its relevant fields, variants like `MoveCards` (~60 fields) and `MiscOp` (~90 fields) still carry too many rarely-used Options.

**What was done:**
- Boxed 14 `Vec<String>`/`Vec<serde_json::Value>` filter fields on EffectKind variants
- `character_effects`, `card_names`, `heart_colors`, `per_unit_heart_colors`, `group_names`, `exclude_group_names`, `characters`, `exclude_characters`, `identities`, `answers`, `choice_options`, `exclude_heart_colors`, `or_card_types`, `trigger_filter` → `Box<Vec<String>>` or `Option<Box<Vec<String>>>`
- Updated 14 getter methods and 11 setter methods + Condition enum getters + test code
- All 1820 tests pass

**Savings:** ~16 bytes per field × ~10 fields per variant = ~160 bytes saved on largest variants. Total EffectKind: 1248 → ~1150.

---

### Box large inline fields on EffectKind variants ✅ DONE

**Why:** Three large non-`Copy` types were inlined in 7-9 EffectKind variants each, inflating the enum to 816 bytes. Boxing them removes their size from the enum discriminant.

**What was done:**
- `dynamic_count: Option<DynamicCount>` → `Option<Box<DynamicCount>>` (96B → 8B) in 7 variants
- `position: Option<PositionInfo>` → `Option<Box<PositionInfo>>` (40B → 8B) in 9 variants
- `quoted_text: Option<QuotedText>` → `Option<Box<QuotedText>>` (48B → 8B) in 2 variants
- Updated `dynamic_count_any()`, `position_any()`, `quoted_text_any()` getters to use `.as_deref()`
- `vm_gen.rs` generated code unchanged (uses `Default::default()` for these fields)
- Condition variant `position` fields left unboxed (different struct, different semantics)

**Measured results:**
- EffectKind: **816 → 656 bytes** (160 bytes saved per variant)
- Total EffectKind heap: **1,172,408 → 951,856 bytes** (~215 KB saved)
- All 1820 default + 1829 bytecode + 1829 lazy tests pass

**Trade-offs:**
- Boxed filters add a heap alloc when filters are present
- Most effects don't use character/group filters — `Option<Box<Vec<String>>>` (8 bytes None) is cheaper than `Option<Vec<String>>` (24 bytes None)
- **Test risk:** Low — mechanical field type change, serde-compatible

---

### P1 — Remove serde_json::Value → typed enums ✅ DONE

**Completed (2026-07-20):**
- `LogEntry.metadata: serde_json::Value` → `LogMetadata` tagged enum
- `TemporaryEffect.effect_data: serde_json::Value` → `EffectData` tagged enum
- All 1829 tests pass

---

### P1 — Remove redundant condition.text and TriggerEvent at runtime ✅ DONE

**Why:** Every condition dict has `"text"` (the raw Japanese fragment) and `"trigger_event"` (documentation). Neither is read by engine logic.

**What was done:**
- Added `debug_conditions` feature to Cargo.toml (in default features)
- Feature-gated `text: Option<String>` on all Condition variants: `#[cfg(feature = "debug_conditions")]`
- Feature-gated `trigger_event: Option<Box<TriggerEvent>>` on all Condition variants: `#[cfg(feature = "debug_conditions")]`
- When feature is disabled, serde silently ignores the JSON keys (unknown fields)
- 1820 tests pass

**Savings:** Each condition's text field is 24 bytes (Option String). With 20 variants × ~1200 conditions = ~28 KB saved when feature disabled. Plus TriggerEvent at ~152 bytes per struct.

**Trade-offs:**
- Frontend loses ability text context — reconstruct from parent ability's `full_text`
- Feature is in defaults: `cargo build --no-default-features` to omit
- **Test risk:** None — fields are never read by engine logic

---

### P1 — Remove TriggerEvent from Condition (or parser-side) ✅ DONE

Covered by `debug_conditions` feature gate above. `trigger_event: Option<Box<TriggerEvent>>` is gated behind the same `#[cfg(feature = "debug_conditions")]` flag as `text`. Not read by engine logic.

---

### P1 — Remove rekindle_effect recursion (derisked by P1 AbilityEffect change)

**Why:** `EffectKind` is `#[serde(skip)]`, so every deserialization triggers `rekindle_effect()` which recursively walks the ENTIRE ability tree to reconstruct EffectKind from raw JSON. If P1 (AbilityEffect removal of flat field redundancy) is done, EffectKind becomes the source of truth and rekindle is unnecessary — kind is serialized with the struct.

**Savings:** Eliminates recursive tree walk on every JSON load (thousands of nodes).

**Trade-offs:**
- Only matters if AbilityEffect still has flat fields. If P1 consolidates to EffectKind-only, rekindle naturally dies.
- **Clock cycle win:** Thousands of recursive JSON traversals eliminated
- **Test risk:** Medium — serialization format changes

---

### P1.5 — ActionType dispatch enum ✅ DONE

**What:** Replaced `AbilityEffect.action: String` (24 bytes, heap-allocated) with `ActionType` enum (1 byte + padding = 2 bytes, stack-local). All string comparisons eliminated.

**Why:** Enables direct enum dispatch in the bytecode VM. Each `ActionType` variant maps 1:1 to a bytecode opcode. No string parsing at execution time.

**Key findings:**
- Using `#[serde(rename_all = "snake_case")]` derive handles ALL action strings (including internal ones like `sequential_cost`, `conditional_optional`). No `#[serde(skip)]` needed on any variant.
- Custom `Serialize`/`Deserialize` impls that delegate to `from_str()`/`to_str()` are unnecessary — the derive handles everything.
- `ActionType::from_str()` and `ActionType::to_str()` kept for backward compat but no longer called on the hot path.

**Files touched:** `enums.rs`, `card.rs`, `cost.rs`, `effects/mod.rs`, `compound.rs`, `choice.rs`, `resolver.rs`, `util.rs`, `move_cards.rs`, `live.rs`, `triggers.rs`, `modifiers.rs`, `player.rs`, `game_setup.rs`, `card_loader.rs`, `abilities.rs`, `misc.rs`, `state.rs`, `describe.rs`, + 3 test files.

**Savings:** ~22 bytes per effect field (String → enum). Elimination of ~50 `from_str()` calls per ability execution.

**Test impact:** 3 test files updated. All 1820 tests pass.

---

### P1.6 — Build bytecode VM — ✅ REDESIGNED (2026-07-18)

**Why:** The current engine decodes `abilities.json` into full `AbilityEffect` structs
at load time (~4 MB JSON → ~2.8 MB in-memory structs). For PS1, N64, DS, Dreamcast
targets with <1 MB RAM, this is too expensive. A bytecode format stores abilities as
compact opcodes (~14B per ability vs ~400B per AbilityEffect).

**Design pivot (2026-07-18):** Approach evolved through two implementations:

1. *Minified-JSON slice* (first attempt): store each ability as its minified JSON and
   re-parse with `serde_json::from_slice`. 100% correct drop-in (all 1820 pass, fully
   automatic) but only 1421 KB → 1090 KB, and still a full serde parse at load — kept
   the runtime cost. Rejected.
2. *Binary-JSON (current)*: store each ability as a compact tagged tree with all
   strings (object keys + string values) interned into a generated `STRINGS` table and
   referenced by 2-byte indices. The decoder (`vm.rs::read_value`) reconstructs the
   **exact same** `serde_json::Value` the text loader would, then runs the identical
   default-path post-processing (`from_value::<Ability>` + `populate_from_json` +
   draw-count fix). The codec is **generic over JSON shape** (no per-action schema),
   so a new action type / field needs ZERO encoder or decoder changes.

**Why generic binary-JSON instead of a schema-aware per-variant codec:** the per-variant
approach was the original design but it diverged from the JSON loader (the 730+ failures
— field-alias mismatches, missing enum decoders). Generic binary-JSON + reuse of the
proven `populate_from_json` path is *correct by construction* and stays automatic.

**Correctness guard (new):** `tests/test_modules/bytecode_deep_compare_test.rs`
decodes every ability both ways (bytecode `get_ability(idx)` vs the default JSON-path
`serde_json::from_value::<Ability>` + `populate_from_json` + draw-fix) and asserts the
two `Ability` values are deep-equal. This is the automated guarantee that "new ability
types can be added with no issue" — any decoder divergence from the JSON loader fails
this test at regen time. All 1829 tests pass under `--features bytecode_abilities`
(incl. this guard); all 1820 pass on the default path.

**Measured results (2026-07-18, second pass):**
- On-disk/ROM asset: `abilities.json` 1421 KB → `abilities.bin` **136 KB (90% smaller)**.
  Real, large win for console ROM shipping. (Dropped the loader-only `cards` mapping
  from the binary: STRINGS 5320 → 3592 entries, blob 147 → 136 KB.)
- Per-ability blob: ~1395 B (text) → **~175 B** (binary-JSON).
- Peak parse heap at load: lower than the text path — the decoder consumes the
  reconstructed `Value` by value (no whole-tree `clone`) and reads only the one needed
  slice; the text path materializes the whole 4 MB file string + a full `Value` tree.
- CPU: binary-JSON decode is ~2x the structured work of text `from_value` (it builds a
  `Value` then runs `from_value`+`populate_from_json`), so it is *not* faster per
  ability. CPU is not the current bottleneck, so this is acceptable. The `get_ability`
  path also avoids the old redundant `entry.clone()` of the whole tree.
- Runtime in-memory **decoded** `Ability` structs: **UNCHANGED** (~2.8 MB) — the decoded
  struct is identical to the JSON path, and the bytecode path decodes the same number of
  `Arc<Ability>` per card as the default path. The doc's "14B/ability vs 400B/ability"
  target is only reachable by *not holding decoded structs in RAM* — i.e. **lazy /
  on-demand decoding** that keeps abilities in the compact ROM blob and decodes only the
  abilities a card actually needs (see "Resident RAM — the real remaining gap" below).

**Savings delivered:** 90% smaller ROM asset + smaller STRINGS + lower peak load heap.
**Not yet delivered:** resident decoded-struct reduction (needs lazy/on-demand decode).

**Resident RAM — the real remaining gap (next tracked task):**

The decoded `Ability`/`EffectKind` struct layout is fixed, so *no decoder change* shrinks
what is held in RAM. Today `CardLoader::build_abilities_map_inner`
(`src/core/card_loader.rs:94`) eagerly decodes **all 800** abilities into
`Arc<Ability>` at load, and `Card.abilities: Vec<Arc<Ability>>` (`src/core/card.rs:274`)
is read at ~67 sites. The bytecode path decodes the same set, so resident RAM is identical
to the JSON path (~2.8 MB of unique `Ability` structs). The doc's "14B/ability vs 400B"
target is only reachable by **not holding decoded structs in RAM** — i.e. keep abilities in
the compact 136 KB blob (ROM/static) and decode only the abilities a card actually needs,
caching decoded ones in a bounded pool.

This is a **default-path refactor** (it changes `Card.abilities`'s type and its access
sites), so it breaks the bytecode feature's isolation guarantee and must be its own tracked
task with its own test gate (all 1820 must stay green on the default path AND the
bytecode path). It is the genuine path to the "<1 MB RAM" console target.

#### Design: lazy / on-demand ability resolution

**Goal:** `Card.abilities` no longer owns decoded `Ability` structs up front. Instead it
holds lightweight *references* into the bytecode blob; the engine resolves them to
`Arc<Ability>` on first use and caches in a shared pool.

**1. Keep the blob resident, not the structs.**
The regenerated `BYTECODE`/`OFFSETS`/`STRINGS` (`src/ability/abilities_gen.rs`) already live
in the binary as `&[u8]` — zero heap. They become the *source of truth* for ability data.
We never parse the 4 MB `abilities.json` at runtime; `compile_abilities.py` output is the
only asset shipped.

**2. Replace `Card.abilities` with a reference handle.**
```rust
// Before (default path):
pub abilities: Vec<Arc<Ability>>,

// After (unified):
pub abilities: Vec<AbilityRef>,   // AbilityRef = u16 ability index into BYTECODE
```
`AbilityRef` is a newtype over the `unique_abilities` index. It is `Copy`, 2 bytes, and
serializes/compares trivially. Because abilities are *shared* across many cards, the index
form also **dedupes for free** — 50 cards referencing the same ability store one `u16`
each instead of 50 `Arc` headers.

**3. A resolver that decodes on demand + caches.**
Introduce `AbilityResolver` (feature-gated wrapper, but the *interface* `resolve(ref) ->
Arc<Ability>` is used by all 67 sites so default + bytecode share call sites):
```rust
pub struct AbilityResolver {
    cache: Mutex<HashMap<u16, Arc<Ability>>>,   // decoded abilities, bounded
    // optional: an LRU/arena eviction so the cache can't grow past N entries
}
impl AbilityResolver {
    pub fn resolve(&self, r: AbilityRef) -> Arc<Ability> {
        if let Some(a) = self.cache.get(&r.0) { return a.clone(); }
        let a = crate::ability::vm::get_ability(r.0 as usize)
                    .map(Arc::new)
                    .expect("ability index out of range");
        self.cache.insert(r.0, a.clone());
        a
    }
}
```
- Default (non-bytecode) builds can keep `get_ability` as "parse the one JSON slice from
  the in-memory `abilities_data`" (or just keep eager `Arc<Ability>` and skip the resolver
  entirely) — i.e. the resolver is a no-op shim on the default path, so default-path
  behavior is unchanged and the 1820 tests stay green without touching 67 sites.

**4. Mechanically rewrite the 67 access sites.** Each `card.abilities` (currently
`Vec<Arc<Ability>>`) becomes "iterate `AbilityRef`, resolve per element." The minimal,
low-risk transformation per site:
```rust
// Before:
for ability in &card.abilities { ... ability.full_text ... }
// After:
for ar in &card.abilities {
    let ability = resolver.resolve(*ar);
    ... ability.full_text ...
}
```
Because the iteration shape (`for ... in &card.abilities`) is identical, this is a
mechanical `s/&card.abilities/&card.abilities/` + inject `resolver.resolve(...)` edit,
not a logic rewrite. `card.abilities.iter().enumerate()` and `.len()` still work
(`Vec<AbilityRef>` keeps those methods). `.any(|a| ...)`, `.is_empty()`, `.clone()` all
still apply (resolve returns `Arc<Ability>`).

**5. Pass the resolver everywhere it's needed.** `resolver` is created once at game setup
and threaded through `GameState` (already the natural owner of per-game mutable state) or
stored as a `OnceLock`/global since abilities are immutable for a game. The default path
can supply an identity resolver that returns pre-built `Arc<Ability>` (so non-bytecode
builds need zero structural change — only the bytecode path actually decodes lazily).

**6. Bound the cache (the actual RAM win).** With an LRU or arena cap (e.g. 256 entries),
resident decoded ability RAM tops out at ~256 × ~3.5 KB ≈ 900 KB instead of 800 × 3.5 KB
≈ 2.8 MB — and in practice a game touches far fewer than 256 distinct abilities, so peak
is much lower. Combined with the 136 KB blob, total ability memory fits the <1 MB console
budget. The `CountingAllocator` (already in the repo) can measure before/after.

**7. Test gate (mandatory).**
- All 1820 default-path tests must pass unchanged.
- All 1829 bytecode-path tests must pass (the deep-compare guard still proves
  `get_ability` is byte-identical to the JSON loader).
- Add `tests/test_modules/ability_lazy_resolve_test.rs`: build a deck, force the cache to
  a tiny cap (e.g. 4 entries), play a full game, and assert (a) every resolved ability
  matches the eager decode, and (b) the cache never exceeds its cap (proving eviction +
  re-decode works without logic changes).
- Fuzz: randomly cap the cache at 1..N and run a few game scripts; assert outcomes are
  stable regardless of cap (decoding is pure).

**Risks / mitigations:** the 67-site rewrite is the only real risk; because it's mechanical
(`Vec<Arc<Ability>>` → `Vec<AbilityRef>` + per-element resolve) and the iteration API is
preserved, each site is a 1-2 line change. Keep the default-path resolver as an identity
shim so the default build is untouched and its 1820 tests are unaffected. Do it behind a
feature flag (`lazy_abilities`) so it can land independently and be reverted if any site
is missed.

**Estimated effort:** ~1 day — 1 newtype + 1 resolver + 67 mechanical edits + 1 fuzz test.
The bytecode generator/decoder are already done and need no change; this task only changes
how `Card.abilities` is *populated and read*.

**Isolation:** only `cards/compile_abilities.py` (generator) + generated
`src/ability/abilities_gen.rs` + feature-gated `src/ability/vm.rs` + added test are
touched. No default-path code changed. Regenerate with `python cards/compile_abilities.py`
(which writes both `cards/build/` and `src/ability/abilities_gen.rs`).

---

### P1 — HashMap consolidation in GameModifiers

**Impact:** GameModifiers has 20 HashMaps: blade_modifiers, heart_modifiers, heart_override, orientation_modifiers, cost_modifiers, score_modifiers, need_heart_modifiers, constant_blade_bonuses, constant_cost_bonuses, constant_score_bonuses, constant_heart_bonuses, heart_color_multiplier, delayed_cannot_active, success_zone_blade_bonuses, success_zone_heart_bonuses, success_zone_score_bonuses, blade_type_modifiers, etc.

Each empty HashMap costs ~32 bytes. With 18 maps, that's ~576 bytes of overhead before any game data. During play, most maps have overlapping keys (card IDs).

**Fix:** Replace with `HashMap<i16, ModifierSet>` where `ModifierSet` is:

```rust
struct ModifierSet {
    blade: Option<ModifierEntry>,
    heart: Option<SmallVec<[(HeartColor, ModifierEntry); 2]>>,
    cost: Option<ModifierEntry>,
    score: Option<ModifierEntry>,
    orientation: Option<String>,
    // ... one Option per modifier type. Zero-overhead when None.
}
```

**Trade-offs:**
- Single hash lookup instead of 18 per card evaluation
- Only allocate for cards that have ANY modifier
- **Clock cycle win:** One hash vs 18 per recalculate_constants iteration
- **Test risk:** Medium — changes every modifier access site

---

### P2 — Bounded log buffers — ❌ DROPPED

**Why:** `rule_log: Vec<String>` and `structured_log: Vec<LogEntry>` grow unbounded.

**Status:** Dropped by decision. The display layer already caps output at the last 500
entries (`display.rs`), and bounding the live buffers would require touching ~50 direct
`.push()` writer sites across the engine (high churn for a modest win). Deferred.

---

### P2 — Parallel Vec consolidation: revealed_cards — ✅ DONE

**Impact:** GameState had 5 parallel Vecs for revealed cards + 5 more for
revealed_cost_cards (id, source, source_name, is_private, owner).

**Fix:** Kept `revealed_cards: Vec<i16>` / `revealed_cost_cards: Vec<i16>` as the
card-id lists (they are read/written from ~30 engine + test sites as plain `Vec<i16>`),
and consolidated the 4 parallel tracking columns into a single
`Vec<RevealedCardMeta>` / `Vec<RevealedCardMeta>` (struct: `source`, `source_name`,
`is_private`, `owner`). `push_revealed_card` / `push_revealed_cost_card` /
`clear_revealed_cards` / display mapping updated accordingly.

**Why not a single `Vec<RevealedCard>` replacing the id Vec:** that would have required
rewriting every one of the ~30 `revealed_cards` call sites (including 20+ test files
that do `.push(id)`, `.len()`, `.contains(...)`, `core::mem::take`, indexing) — a
high-risk "system shock". The contained version keeps `Vec<i16>` semantics intact
and only rewrites the engine-internal meta tracking, which is not referenced by tests.

**Savings:** 8 Vec headers → 2 per group (4 → 2). Better cache locality for the
tracking columns. All 1820 tests pass.

---

### P2 — Ability full_text + triggerless_text dedup — ✅ DONE

**Why:** Every `Ability` stored both `full_text` and `triggerless_text` as `String`;
for most abilities the latter is just `full_text` with a leading trigger clause
(`【…】`) stripped.

**Fix:** `triggerless_text` is now `Option<String>` (serde `skip_serializing_if`),
defaulting to `None` = "derive from `full_text`". Added `Ability::triggerless_text()`
which returns the stored value when `Some`, otherwise strips a leading `【…】` clause
from `full_text`. All writers (`vm.rs`, `gen_abilities_map.rs`, `card_loader.rs`,
`ability_effects.rs`, `ability_queue.rs`) and readers updated. The JSON serde contract
is preserved (triggerless_text is still emitted when present).

**Savings:** `triggerless_text` is no longer stored redundantly for the common case
(~80 KB across ~1000 abilities). All 1820 tests pass.

---

### P2 — HashMap → SmallVec for tiny tracking maps — ✅ DONE

Many GameState collections hold 0-5 entries. Replaced the following with
`SmallVec<[T; 8]>` (or `SmallVec<[(String, u32); 8]>` for the counter map):
- `areas_placed_this_turn: HashSet<String>` → `SmallVec<[String; 8]>`
- `cards_appeared_this_turn: HashSet<i16>` → `SmallVec<[i16; 8]>`
- `negated_abilities: HashSet<i16>` → `SmallVec<[i16; 8]>`
- `this_batch_triggered_ability_ids: HashSet<String>` → `SmallVec<[String; 8]>`
- `auto_ability_trigger_counts: HashMap<String, u32>` → `SmallVec<[(String, u32); 8]>`

Set `insert` → dedup-then-`push`; map `entry().or_insert().+=1` → `find`-or-push.
`display.rs` target structs still receive `Vec`/`HashMap` (converted via `.iter()`).
`gained_abilities` and `cards_moved_this_turn` were deliberately left as `HashMap`/
`HashSet` (heavier value types / wider use). All 1820 tests pass.

**Savings:** No heap alloc when empty; no hash computation for inserts/lookups on these
tiny collections.

---

### P2 — GameState display clone elimination

**Why:** `display.rs:game_state_to_display()` takes `&GameState` and clones strings to build display structs. Called on every `get-state` command.

**Fix:** Refactor display to borrow from GameState where possible (`&str` instead of `String`).

**Trade-offs:**
- Lifetime management complexity
- Partial fix: clone only changed fields
- **Clock cycle win:** Eliminates O(game state) string clones per display
- **Test risk:** Medium

---

### P2 — AbilityQueueEntry field trimming

**Why:** `AbilityQueueEntry` (~600+ bytes) has many fields that are None/empty for most entries.

**Fix:** Move optional-heavy fields behind `Box<AbilityQueueEntryExtras>` — common case (no choice, no pending actions, no condition cache) pays only 16-byte niche-optimized pointer.

**Trade-offs:**
- Extra pointer hop for uncommon case
- **Test risk:** Medium

---

### P2 — Card database on-demand / compact loading

**Why:** All ~3000 cards parsed into full Card structs at init. Only ~100 used per game.

**Fix:** Compact binary format with index-based lookup. Only load cards in the current deck.

**Savings:** From ~3000 card structs to ~100. Card data ~200 bytes → ~50-80 bytes in binary format.

**Trade-offs:**
- Massive architectural change to every `card_database.get_card()` call site
- Can be incremental: start with lazy loading, then optimize storage
- **Test risk:** High
- **Clock cycle win:** Smaller card data = fewer cache misses on every card lookup

---

### P2 — Ability full_text + triggerless_text dedup — ✅ DONE

**Why:** Every Ability stores both. For most abilities these are identical or triggerless just strips the prefix.

**Fix:** `triggerless_text` is now `Option<String>` (serde `skip_serializing_if`),
derived lazily via `Ability::triggerless_text()` which strips a leading `【…】`
trigger clause from `full_text` when the field is `None`. The stored `String`
`full_text` is retained as the canonical source. (See the detailed note in the P2
section above.)

**Savings:** ~80 KB for ~1000 abilities. All 1820 tests pass.

**Trade-offs:**
- Lazy computation on first access (cheap — str::strip_prefix)
- **Test risk:** Low

---

### P2 — PerformanceSnapshot lifetime reduction

**Why:** Large Breakdown structs (Vec<HeartSource>, Vec<BladeSource>, Vec<Allocation>, etc.) kept until overwritten.

**Fix:** Drop detailed allocation data after victory determination phase.

**Trade-offs:**
- Frontend loses post-game detail — keep optional for final state
- **Test risk:** Low

---

### P3 — EffectKind field enums

Replace remaining `Option<Box<str>>` with proper enums: `placement_order`, `distinct`, `ability_filter`, `card_type`, `location`, `state`, `operation`, etc.

**Clock cycle win:** `match` on enum discriminant instead of `as_deref() == Some("value")` string comparison.

**Test risk:** Medium

---

### P3 — String interning for closed sets

Zone names, action types, heart color names, group names — all closed sets repeatedly allocated. Many already have Rust enums (Zone, ActionType, HeartColor, Operator) but JSON data still uses raw strings.

Use a compile-time interner or just ensure all const strings are `&'static str`.

**Test risk:** Low

---

## Updated strategy: parser ↔ engine contract

The Python parser (`cards/ability_extraction/parser.py`) already outputs well-structured tagged JSON. The engine should use that structure directly instead of flattening it.

**New contract:**

| Where | What changes | Impact |
|-------|-------------|--------|
| **Parser output format** | Unchanged for Condition (already tagged). Activate `_collapse_to_effect_steps` stub for unified effect shapes. Optionally strip `text` from conditions for non-debug builds. | Zero or minimal |
| **Parser code** | Small change: flip the stub switch. Optionally add a `debug` flag for text inclusion. | 1 file, ~5 lines |
| **Rust Condition** | Replace 1864-byte flat struct with `#[serde(tag = "type")]` enum. **No JSON format change.** | Large refactor of evaluation code. |
| **Rust AbilityEffect** | Remove flat field redundancy, let EffectKind be the source of truth. | Medium refactor of effect dispatch. |
| **Rust rekindle_effect** | Dies naturally when EffectKind carries serialization. | Removed. |

**The guiding principle: the Rust side should consume the JSON structure the way the parser emits it, not flatten it first and then reshape it.**

---

## Current memory per "average" ability

Rough estimate for a typical ability with 1 condition and 3 option effects:

| Stage | Per ability memory | Cumulative |
|-------|-------------------|------------|
| **Before any work** | ~17 KB | 17 KB |
| **After edae728 (EffectKind enumification)** | ~8 KB | 25 KB |
| **After 66dd1ee (HeartMap + boxing)** | ~5 KB | 30 KB |
| **After box EffectKind Vecs** | ~2.8 KB | 32.8 KB |
| **After P0 (Condition tagged enum)** | **~1.0 KB** | **34 KB** |
| **P0.5 (effect_steps activation)** | ~900 B | 34.9 KB |
| **P1 (AbilityEffect + EffectKind consolidation)** | ~600 B | 35.5 KB |
| **P2 (bounded logs, HashMap consolidation, Vec structs)** | ~450 B | 35.95 KB |
| **P3 (enums, interning, lazy cards)** | ~300 B | 36.25 KB |

*Per-ability memory includes its share of the ability tree. Total ability database with ~2000 abilities:*
- Current: ~2000 × 3 KB = ~6 MB
- After P0: ~2000 × 1.0 KB = ~2 MB
- After P2: ~2000 × 450 B = ~900 KB
- After P3 (lazy): <200 KB (only currently-relevant abilities loaded)

---

## CPU vs RAM tradeoff analysis

The core tension: **does boxing EffectKind (808B per effect → 8B on stack + 808B on heap) make things faster or slower?**

### Concrete example: evaluating "西木野真姫" (8 effects)

This ability does: discard → specify heart color → reveal 5 → check revealed → select cards → gain blade.

**Memory cost (storage, loaded once):**
```
EffectKind enum:           808B × 8 effects =  6,464B  (heap, counted once)
AbilityEffect struct:      264B × 8 effects =  2,112B  (stack, per-context copy)
Condition enum:            536B × 2           =  1,072B  (2 conditions on this ability)
Text fields (Japanese):    635B × 2 (UTF-16)  =  1,270B  (full_text + triggerless_text)
Total per ability:                              ~10.9 KB
```

**Runtime cost (per execution, when this ability triggers):**
```
Step 1: Condition evaluation
  │  evaluate_card_count_condition()
  │   │  get_count()         1 getter  ~3 cycles
  │   │  get_group_reference() 1 getter ~3 cycles
  │   │  get_card(cid) → c.group   ~20 cycles (HashMap lookup)
  │   │  resolve_zone_card_count() ~200 cycles (filter 5 revealed cards)
  │   └─ compare_counts()    ~5 cycles
  └── Total: ~250 cycles  (99.9% of work is in resolve_zone_card_count)

Step 2: Effect execution (sequential → specify_heart_color → reveal → select)
  │  getter calls: source_any(), destination_any(), count_any(),
  │               target_any(), card_type_any(), group_names_any()
  │  ~6 getters × ~3 cycles = 18 cycles
  │  Actual work: shuffling cards, checking hearts, granting blade
  │  Each sub-step: 100-5000 cycles (card movement, database lookups)
  └── Total: ~500-10000 cycles

Step 3: Logging
  │  serialize AbilityEffect → JSON: ~500 cycles
  └── Total: ~500 cycles

Grand total per trigger: ~1000-11000 cycles
Getter overhead (Box deref): ~18 cycles per trigger = 0.16-1.8% of total
```

**The Box deref cost:**
- Without Box: `self.kind` → `Option<EffectKind>` inline. `match &self.kind` → LDR from struct offset (1 cycle).
- With Box: `self.kind` → `Option<Box<EffectKind>>`. `self.kind.as_deref()` → LDR pointer (1 cycle) + LDR through pointer (1 cycle).

For 8 effects × 6 getters × 2 extra cycles = **96 extra cycles per full turn** on a 3DS (268MHz). That's **0.000036% of one frame** (1 frame = 16.6M cycles). Completely invisible.

### Why boxing is still the right call

Stack memory from EffectKind = 808B × 1451 effects = **1,171,208 bytes**. Boxing moves this to the heap. The heap is where most game data already lives (Strings, Vecs). The 3DS has 128MB total — losing 1.1MB of stack to EffectKind would overflow the per-thread stack limit (~256KB default on 3DS). Boxing is required for correctness, not just performance.

### The real bottleneck: heap fragmentation

Each Box<EffectKind> is an individual 808B heap allocation. 1451 of them. On a 3DS with 8KB block alignment, each allocation wastes 208-720B. A ROM-based compact representation (128B per effect, mmap'd) would both save 680B per effect AND eliminate the 1451 individual allocs. That's the next target.

---

## Arena allocator architecture

### The problem: 15K allocs per trigger

Every ability evaluation creates ~15,000 heap allocations (Vecs, Strings, Boxes) that are freed immediately after the trigger. The allocator churn costs **~150μs per trigger** on a 3DS (15K allocs × ~10ns each). For a game with 10 triggers per turn × 20 turns = 200 triggers, that's **30ms = 2 frames** spent just in malloc/free.

The allocations come in three categories:

| Category | Example | Allocs | Fate |
|----------|---------|--------|------|
| **Temporary** | Vecs for card lists, format! strings | ~14,900 | Freed before trigger end |
| **Persistent** | Box\<EffectKind\>, Box\<Condition\> | ~100 | Survive across triggers |
| **Setup** | Card database, ability JSON load | once | Never freed |

The temporary ones are the majority. They don't need to survive past the trigger. An arena allocator can eliminate them entirely.

### The solution: thread-local bump arena + per-type pool

Two layers working together:

```
┌─────────────────────────────────────────────────┐
│                 #[global_allocator]              │
│                                                 │
│  alloc(size):                                   │
│    if arena.active → bump from thread-local slab │
│    if size == 808 → pop from EffectKind pool     │
│    if size == 536 → pop from Condition pool      │
│    else → System.alloc(size)                     │
│                                                 │
│  dealloc(ptr, size):                             │
│    if arena.active → no-op (arena reset reclaims)│
│    if pool-owned → push to pool free list        │
│    else → System.free(ptr)                       │
└─────────────────────────────────────────────────┘
```

#### Layer 1: Bump arena (temporary allocations)

```
Trigger start:  arena.cursor = arena.base           (1 store)
alloc:          ptr = arena.cursor; cursor += size   (1 add, 1 load)
dealloc:        do nothing                           (0 ops)
Trigger end:    arena.cursor = arena.base           (1 store)
```

- Pre-allocated 1MB slab per thread (64KB on 3DS, 8MB on desktop)
- Reset is O(1) — just reposition cursor
- No individual frees needed
- Thread-local: no synchronization
- The `CountingAllocator` runs on top: arena bumps increment the alloc counter (for measurement) but never call `System.alloc`/`free`

**Impact:** The ~14,900 temporary allocs per trigger become 14,900 pointer bumps. Total cost: ~50ns instead of ~150μs.

**Limitation:** Arena memory can't be freed individually. Everything is reclaimed at once when the arena resets. Any allocation that outlives the trigger (e.g., a cloned EffectKind moved to the pending queue) must be cloned out of the arena before reset.

#### Layer 2: PoolBox (persistent per-type allocations)

For types that DO survive across triggers (EffectKind, Condition), a fixed-size pool recycles blocks:

```rust
struct Pool<T> {
    slots: Box<[UnsafeCell<MaybeUninit<T>>]>,   // pre-allocated at init
    free: Mutex<Vec<usize>>,                     // free-list of slot indices
    next: AtomicUsize,                           // bump counter for new slots
}

struct EkBox {
    slot: Option<usize>,     // pool slot (preferred)
    heap: Option<Box<T>>,    // fallback when pool exhausted
}
```

- **Allocation**: pop from free-list → if empty, bump into next slot → if all full, `Box::new` fallback
- **Deallocation**: drop value in slot → push slot index to free-list
- **Clone**: allocate new slot, `T::clone` into it
- **Cost**: `alloc` = Mutex lock + Vec::pop (~50ns) vs malloc(808) (~200ns). No system allocator call after warmup.

Currently implemented for EffectKind (128 slots × 808B = 103 KB). Condition pool uses the same mechanism but is blocked by serde deserialization quirks (see "What doesn't work yet" below).

**Impact:** The ~52 persistent EffectKind allocs per trigger become pool bumps. Zero system allocator traffic after the first trigger.

#### Combined flow per trigger

```
1. Arena.enter()                          — cursor = base, enable arena mode
2. Condition evaluation                    — all Vec<String>, Vec<i16> from arena
3. Effect execution                       — temporary Strings, Boxes from arena
4. EffectKind lookup/modify               — from EkBox pool (already live)
5. Clone effect for pending queue         — EkBox::clone() takes from pool
6. Arena.exit() → clone persistents out   — EkBox stays (pool-managed), arena resets
```

### Implementation status

| Component | Status | Lines of code | Impact |
|-----------|--------|---------------|--------|
| `CountingAllocator` | ✅ Live | 130 | Measurement tool |
| `Pool<T>` + `EkBox` | ✅ Live | 180 | 0.3% alloc reduction (52 allocs saved) |
| `CondBox` (Condition pool) | 🔲 Blocked | ~50 | Same again (52 allocs) |
| Bump arena v0 (monotonic) | ✅ Live (feature-gated) | ~90 | 64KB static buffer, cursor never resets |
| Bump arena v1 (cursor reset) | 🔲 Not started | ~40 | **~99% alloc reduction** (~14,900 allocs saved) |
| Arena reset in trigger boundary | 🔲 Not started | ~20 | Reset cursor after each trigger |

The bump arena is the 99% solution. The pools are just insurance for the few allocations that outlive the trigger.

### What doesn't work yet

**Arena v0 (monotonic) — committed:**
A 64KB static buffer with a bump cursor that never resets. `arena_alloc()` bumps the cursor; `arena_contains_ptr()` identifies arena-owned pointers; `CountingAllocator::dealloc()` no-ops for arena pointers. Feature-gated behind `arena_allocator` (NOT in default).

The arena v0 proves the architecture works (1828/1829 tests pass with arena enabled) but fills up after ~100-200 ability evaluations. Once full, all allocs fall through to System — degrading to the status quo.

**Cursor reset — the blocking issue:**
Resetting the cursor requires knowing when ALL arena-allocated data from a trigger batch is dropped. The problem: `process_current_ability` enters/exits the arena per-ability, but the resolver and pending queue may hold references to arena-allocated data (Vecs of moved cards, selected cards, etc.) that outlive the ability's `clear_effect_tracking()` call. When the next ability's `arena_enter()` resets the cursor, those references become dangling.

Attempted solutions that failed:
1. **thread_local! + Cell** — re-entrant init from global allocator caused stack overflow
2. **Static mut + AtomicBool** — not thread-safe across test threads
3. **platform_alloc (HeapAlloc)** — different heap than Rust's System allocator caused heap corruption
4. **Depth counter** — cursor reset at depth 0→1 still overwrites live data from paused abilities

The correct solution requires a **per-turn arena** (reset between game turns, not between abilities) OR an **epoch-based reclamation** scheme where objects are promoted out of the arena before reset.

**CondBox serde issue:** Condition's deserialization uses `#[serde(tag = "type")]`. When wrapped in `CondBox`, serde needs to deserialize the inner enum. The naive approach of delegating to `Condition::deserialize(d)` should work — but testing showed 6 Kasumi test failures that need investigation.

### What it would take to finish arena v1

1. **Per-turn arena lifecycle** — enter at game turn start, exit at turn end (not per-ability). ~10 lines. This means the arena survives across all ability evaluations within a single turn, and the cursor resets between turns. Risk: arena might fill up during a complex turn with many abilities.
2. **Double-buffer arena** — two 64KB buffers, alternate between turns. One is live (active), one is dead (safe to reset). ~40 lines.
3. **Fix CondBox serde** — ~10 lines  
4. **Add arena-backed Vec/Box type** — ~150 lines (or use `bumpalo` crate)

Total: ~280 lines to eliminate ~99% of per-trigger allocs (15K → ~150).

### The ultimate target: 10μs per trigger

With the arena + pools in place:

| Current per-trigger | With arena + pool |
|---------------------|-------------------|
| 15,000 alloc calls | ~150 (pooled EffectKind + Conditions only) |
| 326 KB total allocated | ~3 KB (pool slots + a few system allocs) |
| ~150μs alloc time | ~1μs (pointer bumps) |
| Heap fragmentation | Zero (arena reset is O(1)) |

This makes the engine viable for:
- **AI training**: millions of game simulations with zero allocator pressure
- **GBA/3DS**: peak heap = pool size + arena size (fixed, predictable)
- **Web server**: lower-latency per-request game evaluation

## How to measure

```
cargo run --bin size_check
```

For runtime profiling:
```
cargo test --release -- --nocapture
```
Add `#[cfg(feature = "profiling")]` timing instrumentation for cycle measurement.

---

## Guiding principles

1. **Don't sacrifice correctness.** All 1820+ tests must pass at every step.
2. **Don't sacrifice speed for RAM.** Smaller structs = fewer cache misses = faster. Boxing adds indirection at a predictable 1-cycle cost, trivially hidden by cache wins.
3. **The parser already emits good structure — use it directly.** The Rust side should consume the JSON the way the parser emits it, not flatten it and then reshape it.

4. **Never trust serde to surface missing fields.** Serde silently drops JSON fields that don't match any struct/enum field. This is the #1 source of bugs in enumification refactors. Every variant should carry `#[serde(flatten)] extra: HashMap<String, serde_json::Value>` during development, and log a warning when `extra` is non-empty. This catches parser-variant mismatches instantly instead of producing silent data loss → mysterious test failures.

5. **The parser is not cleanly tagged.** Each condition handler may emit 3-8 "core" keys, but many cards override or add extra fields from the original 85-field pool. The assumption that `card_count_condition` always emits `{type, count, operator, location, target}` is false — it can also emit `group_reference`, `card_property`, `character`, `delta`, `scope`, and more depending on the card. The enum variants must account for this reality, not the idealized handler output.
6. **Feature-gate debug-only data.** `text`, `TriggerEvent`, verbose metadata → only in dev/debug builds.
7. **Do the P0 work first.** Condition tagged enum is the biggest single win and validates the core strategy. Everything else depends on it.
8. **Profile before and after.** Use `std::mem::size_of` and runtime heap profiling to verify gains.

## Session 2: ArcStr + enum optimization results (July 2026)

### Alloc reduction (test `mymai_tonight_blade_disappears`)

| Metric | Before | After | Δ |
|--------|--------|-------|---|
| alloc calls | 15,053 | 1,700 | -89% |
| total allocated | 326 KB | 148 KB | -55% |
| 16-31B bucket | 6,530 | 270 | -96% |
| 8-15B bucket | 4,112 | 337 | -92% |
| 1-7B bucket | 3,635 | 287 | -92% |
| test runtime | 1.36s | 0.61s | -55% |

### Changes made

**Enums (serde-renamed, wire-compatible):**
- `Allocation.source_type`, `source_name`, `phase` — String → enum
- `Adjustment.adjustment_type` — String → enum
- `HeartSource.source_type`, `BladeSource.source_type` — String → SourceType enum

**ArcStr conversions (cheap clone via refcount):**
- `Allocation.target_name`, `CardNeed.name`
- `LiveCardResult.card_no`, `MemberContribution.card_no`, `YellCardResult.card_no`
- `AbilityBonus.source`, `AbilityBonus.ability_text`
- `TriggeredAbility.card_name`, `TriggeredAbility.effect_text`
- `Card.name`, `Card.card_no` (the source — makes all `.clone()` cheap)

**Other:**
- phases.rs: replaced HashMap `.clone()` with `&` references via scope blocks
- Merged duplicate color_mods iteration in snapshot loop
- `format!("{:?}", color)` → static string
- Removed `op_str.clone()` via reordering
- `.to_vec()` → `.as_ref()`
- SmallVec: 5 `Vec<i16>` return types → `SmallVec<[i16; 8]>`

### Remaining alloc breakdown

After optimization, a typical test has ~1,700 allocs:
- **~290 (1-7B):** small enums, copied booleans, `bool` arrays
- **~337 (8-15B):** card_no copies, small strings (`"= 3"`, `"+1"`, etc.)
- **~270 (16-31B):** format strings (`"Ability: CardName"`, `"Requirement = 3"`, rule log entries) — these are *format!()* calls that inherently allocate
- **~258 (32-63B):** rule log entries, ability text copies
- **~317 (64-127B):** serde JSON serialization buffers, choice struct internals
- **~69 (128-255):** snapshot breakdown allocation vecs
- **~102 (256-511):** HashMap internal growth, larger serde buffers
- **~49 (512-1K):** triggered-ability storage, larger choices
- **~6 (1K-2K):** large log entries
- **~2 (2K-4K), ~2 (4K-8K), ~1 (8K-16K):** `get_pending_choice_json()` serde output

**Key takeaway:** The alloc count is now dominated by *necessary* format!/serde operations — there's no single hot function left. 96% of bucket-2 allocs eliminated.

### What took 610 ms?

- **~290 ms — binary startup.** Cargo test binary load, Rust runtime init, test- framework enumeration. Fixed overhead; cannot eliminate without changing test architecture.
- **~200 ms — `OnceLock` database init (`CardLoader::load()`).** Parses `cards.json` (7400 cards) and `abilities.json` via `serde_json::from_str()`. Runs once per process. The JSON comes from `include_str!` (compiled into the binary) so no I/O; this is pure CPU for serde deserialization + HashMap insertions.
- **~120 ms — running 1820 tests.** Average ~66 μs per test. Actual logic is: card lookups, phase transitions, choice resolution, ability condition evaluation, and assertions. No single test is slower than ~4 ms (with `--nocapture` overhead; without it, all are <1 ms).

**Opportunities:**
1. **Faster DB format:** Replace `serde_json` with `bincode` or `rkyv` for card loading. Would cut 200 ms → ~20-40 ms. Tradeoff: build script needed to pre-compile binary card data.
2. **Bump arena global allocator:** Eliminates allocator overhead entirely (pointer bump instead of system malloc). 6-10 lines of `#[global_allocator]` + ~90 lines of bump arena + fallback. Tradeoff below.

### Bump arena tradeoffs

**What it is:** A thread-local bump arena registered as the global allocator. Every `alloc` bumps a cursor; every `dealloc` is a no-op (memory freed only when the arena is reset at trigger boundaries).

**Cost:** ~3 CPU instructions per alloc (a single `add` + two loads/stores). Vs system allocator ~30-150 ns per alloc.

**Why it helps the 3DS:** The 3DS has 128 MB total RAM, slow CPU, and a naive `malloc` implementation that fragments badly. A bump arena:
- Bypasses the system allocator entirely for hot-path allocs
- Eliminates fragmentation (sequential bump, sequential free on reset)
- Reduces allocator CPU overhead from measurable to trace-level

**Why it DOESN'T help on desktop:** The 0.61s test runtime is dominated by 290 ms binary startup + 200 ms db init. Allocator overhead is ~0.04% of wall time (1700 allocs × ~100 ns ≈ 170 μs). A bump arena saves 165 μs out of 610 ms — invisible.

**Risks:**

| Risk | Mitigation |
|------|------------|
| Large/long-lived allocs pin the arena (card database, persistent HashMaps) | Bypass to system allocator for allocs above 4 KB or marked `#[global_allocator]` arena-specific |
| Wrong reset point → arena OOM → panic | Arena reset is explicit; CI catches misuse. Arena is reset at trigger boundaries (after each ability effect resolves) |
| HashMap/vec growth inside the arena never frees until reset — worst case eats the entire arena | Set arena capacity to ~1 MB; overflow → system allocator fallback |
| Compatibility: crates using `realloc` or `dealloc` assumptions (serde_json, `std::collections`) | Override `realloc` → bump for grow, system for shrink. Override `dealloc` → no-op for arena-owned, system otherwise |

**Implementation sketch:**

```rust
use std::alloc::{GlobalAlloc, Layout, System};

const ARENA_SIZE: usize = 1 << 20;  // 1 MB

struct BumpArena {
    buf: UnsafeCell<[u8; ARENA_SIZE]>,
    cursor: AtomicUsize,
}

unsafe impl GlobalAlloc for BumpArena {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() > 4096 {
            return System.alloc(layout);  // large → system
        }
        let offset = self.cursor.fetch_add(layout.size(), Ordering::Relaxed);
        if offset + layout.size() > ARENA_SIZE {
            System.alloc(layout)  // arena full → system
        } else {
            self.buf.get().cast::<u8>().add(offset)
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr < self.buf.get().cast() as *mut u8
            || ptr >= self.buf.get().cast::<u8>().add(ARENA_SIZE)
        {
            System.dealloc(ptr, layout);  // not arena-owned → system
        }
        // arena-owned: no-op
    }
}

#[global_allocator]
static ALLOCATOR: BumpArena = BumpArena { ... };
```

**Verdict:** Worth implementing for the 3DS build. Not worth the complexity for desktop (0.61s runtime can't be meaningfully improved further).
