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
| `Condition` | **1864** | **~120** | Flat struct → tagged enum, 94% reduction |
| `EffectKind` | **1248** | **~1150** | 14 Vec fields boxed — further consolidation planned |
| `AbilityEffect` | **1536** | **1536** | Not yet consolidated — next target |
| `CompoundBranch` | **96** | **96** | Mostly boxed pointers now |
| `AbilityQueueEntry` | **~600+** | **~600+** | Has condition_cache HashMap, snapshot Vecs, resolver, pending_actions |
| `Option<String>` | 24 | 24 | No niche, always full size |
| `Option<Box<str>>` | 16 | 16 | Box pointer niche = 8 bytes saved |
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

## ❌ What remains: ranked by RAM + cycle impact

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

### P1 — Consolidate AbilityEffect + EffectKind

**Status:** 🔄 Step 5 done (Box filter sets). Steps 1-4 require careful handling of field name semantics.

**Why:** `AbilityEffect` (1536 bytes) and `EffectKind` (1248 bytes) store the SAME data redundantly.
- `effect.source` flat field vs `EffectKind::MoveCards.source_position` vs `EffectKind::DrawCards.sources` — three DIFFERENT names for the same concept.
- 137 `_any()` getter methods exist because flat fields and EffectKind can diverge.
- `rekindle_effect` exists because EffectKind is reconstructed from flat JSON.

**Lesson from Condition refactor:** This is NOT a simple field removal. The field NAMES are inconsistent across EffectKind variants (`source_position` vs `source`, `target_count` vs `count`, etc.). Before removing flat fields, the EffectKind fields need to be RENAMED to a consistent convention that mirrors the JSON output (e.g., all variants use `source`, `count`, `target`). Then the flat fields can be removed from AbilityEffect and the getters collapse into single-match arms.

**Progress:**
1. ✅ Added flat field equivalents (`source`, `destination`, `count`, `target`) to EffectKind as NEW fields — keeping existing `source_position` and `target_count` intact (they represent DIFFERENT concepts: zone vs stage position, card count vs target count). Created `source_any()`, `destination_any()`, `count_any()`, `target_any()` getters.
2. 🔲 Remove flat fields from `AbilityEffect` (`action`, `source`, `destination`, `count`, `target`) — requires custom Deserialize for AbilityEffect to feed flat JSON fields into EffectKind during deserialization instead of storing them as struct fields
3. 🔲 Simplify 137 `_any()` getters — no flat field fallback needed once flat fields are removed
4. 🔲 Kill `rekindle_effect` — EffectKind becomes the single source of truth  
5. ✅ **Box filter sets** on large EffectKind variants — DONE. 14 Vec fields boxed

**Strategy for flat field removal:**
Replace `#[serde(skip)]` on `kind: Option<EffectKind>` with a custom Deserialize impl that reads the raw JSON, extracts flat field keys (source, destination, count, target) into EffectKind during deserialization, and returns AbilityEffect without storing them separately. This eliminates the flat fields AND makes `rekindle_effect` unnecessary, naturally killing both code paths.

**Savings:** 1536 + 1248 → ~400 bytes per effect (~700 KB for ~6000 effects).

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

**Trade-offs:**
- Boxed filters add a heap alloc when filters are present
- Most effects don't use character/group filters — `Option<Box<Vec<String>>>` (8 bytes None) is cheaper than `Option<Vec<String>>` (24 bytes None)
- **Test risk:** Low — mechanical field type change, serde-compatible

---

### P1 — Remove serde_json::Value → typed enums

**Why:** `TemporaryEffect.effect_data: Option<serde_json::Value>` and `LogEntry.metadata: Option<serde_json::Value>`. `serde_json::Value` is a heap-allocated JSON tree. Every temporary effect and log entry pays this cost.

**Fix:** Replace with `Option<EffectMetadata>` where EffectMetadata is a compact typed enum:

```rust
enum EffectMetadata {
    HeartColorOverride { color: HeartColor, amount: u32 },
    BladeBonus { target: i16, amount: i32 },
    ScoreBonus { amount: i32 },
    CostMod { target: i16, delta: i32 },
    AbilityGain { ability_key: String },
    // ...
    None,
}
```

**Savings:** `serde_json::Value` is ~72 bytes minimum + heap alloc per nested value. Typed enum: ~24-40 bytes, stack-local.

**Trade-offs:**
- Needs to enumerate all metadata forms — but they're internal to the engine (effects know what they emit)
- More rigid, harder to extend mid-game
- **Test risk:** Medium

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

### P2 — Bounded log buffers

**Why:** `rule_log: Vec<String>` and `structured_log: Vec<LogEntry>` grow unbounded. Each LogEntry has `text: String`, `metadata: Option<serde_json::Value>`. After 20+ turns, thousands of entries.

**Fix:** Replace with `VecDeque` capped at 200 entries.

**Trade-offs:**
- Lose infinite history. 200 entries is enough for any practical game play.
- **Test risk:** Low

---

### P2 — Parallel Vec consolidation: revealed_cards

**Impact:** GameState has 5 parallel Vecs for revealed cards + 5 more for revealed_cost_cards:
- `revealed_cards: Vec<i16>`, `revealed_card_sources: Vec<Option<i16>>`, `revealed_card_source_names: Vec<Option<String>>`, `revealed_card_is_private: Vec<bool>`, `revealed_card_owners: Vec<Option<u8>>`

**Fix:** Replace with single `Vec<RevealedCard>` struct.

**Savings:** 10 Vec headers → 2. String deduplication. Better cache locality.

**Trade-offs:**
- Mechanical change, low risk
- **Clock cycle win:** Adjacent fields per card instead of 5 separate arrays = fewer cache misses

---

### P2 — HashMap → SmallVec for tiny tracking maps

Many GameState HashMaps hold 0-5 entries:
- `areas_placed_this_turn`, `cards_appeared_this_turn`, `negated_abilities`, `this_batch_triggered_ability_ids`, `gained_abilities`, `auto_ability_trigger_counts`, etc.

**Fix:** Replace with `SmallVec<[(K, V); 4]>` — linear search for <10 entries is faster than hashing.

**Trade-offs:**
- O(n) lookup instead of O(1), but n < 10 means no hash computation overhead
- **Clock cycle win:** No heap alloc when empty. No hash computation for inserts/lookups.
- **Test risk:** Medium

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

### P2 — Ability full_text + triggerless_text dedup

**Why:** Every Ability stores both. For most abilities these are identical or triggerless just strips the prefix.

**Fix:** Store one `text: Box<str>`, compute triggerless on demand (lazily cached).

**Savings:** ~80 KB for ~1000 abilities.

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
