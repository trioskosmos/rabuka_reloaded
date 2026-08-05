# Memory & Dead-Code Optimization Plan

Targets: shared `engine/` crate + small verified cleanups in `platforms/3ds/`.
Benchmarks assume 32-bit console targets (GBA/PSP/PS1/DS/3DS/Wii/DC) where
`usize` = 4 bytes. 64-bit desktop numbers are noted where relevant.

Status: **implemented & verified** — `cargo test -p rabuka_engine` (1936 tests) and
`cards/ability_extraction/parser.py` both pass. no_std lib builds (DS/GBA/PSP/PS1
feature combos) compile clean.

Note on "u4": Rust has no native 4-bit integer. Sub-byte values are expressed by
packing multiple fields into one byte (bitfields) or by keeping values in the
smallest native type (`u8`). The existing bytecode blob already packs 2-bit/4-bit
fields (e.g. `type_flags` in `card_binary.rs`); this pass reduced every widened
field to its smallest native type.

---

## 0. Correction: `#[repr(u8)]` on fieldless enums is a NO-OP

Earlier analysis claimed every enum is 4 bytes because no `#[repr(u…)]` exists,
and that `HeartMap` entries shrink 8→2 bytes. **This is wrong, verified by
measurement:**

| Type | size (64-bit host) |
|---|---|
| `enum HeartColor` (12 unit variants, default repr) | **1** |
| `enum HeartColor` with `#[repr(u8)]` | 1 |
| `(HeartColor, u8)` tuple | **2** (both) |
| `SmallVec<[(HeartColor,u8); 4]>` (i.e. `HeartMap`) | 24 (both) |
| `enum WithPayload { A(Box<str>), B, C }` | 24 (both, repr(u8) no change) |

Rust already packs unit-only enums to the minimal discriminant size; adding
`#[repr(u8)]` changes nothing for ≤255 unit variants, and payload enums are
dominated by their payload/alignment. **Phase "add `#[repr(u8)]` everywhere" is
dropped.** The one place a small discriminant could matter is enums with small
payloads stored in dense arrays — none of the hot structures qualify.

### What is `HeartMap`?

`engine/src/core/card.rs:160`:

```rust
pub struct HeartMap(SmallVec<[(HeartColor, u8); 4]>);
```

- A small map of up to 4 `(HeartColor, count)` entries (inline, no heap
  allocation for the common 1–4 entry case).
- Owned by `BaseHeart`, `BladeHeart`, `SpecialHeart` (`card.rs:284–593`), which
  are `Option`al fields on `Card` (`base_heart`, `blade_heart`,
  `special_heart`).
- Actual cost: **16 bytes on 32-bit** (SmallVec header: ptr 4 + cap 4 + inline
  buffer 4×2) and **24 bytes on 64-bit**. The `(HeartColor, u8)` entries are
  already 2 bytes. Not a 2-byte struct.

Implication: `HeartMap` is already near-minimal; no layout change is worth it
here. The per-card win comes from the header (SmallVec overhead), not the
entries.

---

## Phase 1 — Dead code & bytecode-fallback remnants (zero runtime risk)

Pure cleanup; removes an unresolved merge artifact, stale generated duplicates,
a never-enabled feature with 38 dead cfg blocks, 14 dead functions, and stale
warning-suppression attributes.

1. **Delete `cards/build/vm_gen.rs`** — 5,500-line **unresolved merge-conflict
   artifact** (file starts `<<<<<<< Updated upstream`), never compiled, never
   referenced in code. Only referenced in `docs/` and old git logs.

2. **Delete duplicate generated files** — `cards/build/abilities_gen.rs` and
   `cards/build/cards_gen.rs` are **md5-identical** to the canonical compiled
   copies `engine/src/ability/abilities_gen.rs` and
   `engine/src/core/cards_gen.rs`. Keep the `engine/src` copies.

3. **Delete the `ds_debug` feature + its 38 dead `#[cfg(feature="ds_debug")]`
   blocks.** Feature declared at `engine/Cargo.toml:50` (`ds_debug = ["ds"]`),
   enabled by no platform/config anywhere → the blocks never compile:
   - `engine/src/ability/vm.rs` (11 blocks)
   - `engine/src/ability/ability_store.rs:31`
   - `engine/src/core/game_state/abilities.rs` (8 blocks)
   - `engine/src/turn/phases.rs` (12 blocks)
   - `engine/src/turn/triggers.rs` (6 blocks)
   Removes the feature decl + each `#[cfg]` block including its contents.

4. **Delete 14 dead functions** (verified zero callers repo-wide, excluding
   definitions):
   - `AbilityResolver::buffer_log` — `ability/resolver.rs:124` + its
     `#[allow(unused)]`
   - `cache_size` / `init_ability_store` — `ability/ability_store.rs:59,63`
   - `resolve_deck_indices` — `core/card_binary.rs:454`
   - `AbilityResolver::trace_effect_start` / `trace_effect_end` /
     `get_trace_node` — `ability/resolver.rs:1045,1062,1073`
   - `AbilityResolver::expire_live_end_effects` — `ability/choice.rs:148`
   - `GameState::get_triggerable_abilities` — `core/game_state/abilities.rs:2366`
   - `AbilityResolver::mark_energy_as_wait` — `ability/move_cards.rs:2012`
   - `TurnEngine::player_set_live_cards` — `turn/actions.rs:1301`
   - `snapshot_to_rule_log` — `turn/live.rs:2747`
   - `count_stage_members_with_trigger` /
     `trigger_and_process_auto_abilities` — `turn/triggers.rs:186,462`
   - `evaluate_position_change_condition` — `ability/condition/state.rs:922` +
     its `#[allow(dead_code)]`
   - `test_q30_can_play_same_card_multiple_times` — `qa_test_suite.rs:982`
     (call commented out) + its `#[allow(dead_code)]`
   - `AbDebug::print_cost` — `ability/debug.rs:58` (no_std stub) and `:164`
   - `Zone::label` — `ability/enums.rs:100`

5. **Delete dead enum variants / types:**
   - `AbilityLogItem::KeyValue` — `ability/log.rs:30` (never constructed)
   - `DecodeError::SerdeFailed` — `ability/vm.rs:33` (never constructed; the
     serde fallback it described no longer exists) + stale "serde fallback"
     doc comments at `vm.rs:27` and `vm.rs:96–97`

6. **Delete duplicate free function** `_prohibition_destination_blocks` at
   `core/game_state/abilities.rs:4-21` — identical method at `:2182` is the one
   called at `:2297`.

7. **Remove stale `#[allow]`s** (only after confirming each still builds clean
   without it):
   - `game/web_server.rs:167` — `dead_code` on `Room.game_state`, which is
     written and read
   - `ability/resolver.rs:123` — `unused` on the dead `buffer_log` (see #4)
   - `ability/condition/state.rs:921`, `qa_test_suite.rs:981` — on dead fns
   - `turn/phases.rs:37,46` — `unused_macros` on `tdbg!`, which is invoked 51
     times (likely stale)
   - 3DS: `platforms/3ds/src/steps.rs:31` dead `tprintln!` macro +
     `:47` stale `SetupPhase::Testing` allow; `ui/colors.rs:11` dead `c2d`
     helper + `:27` dead `COL_PINK`; `setup.rs:5` blanket `unused_unsafe`
     (verify unsafe still present before removing)

**Runtime impact:** none directly (dead code never ran). Code-size win on all
targets; removes ~5,500-line conflict artifact and ~1.4MB of stale duplicate
generated sources from the repo.

---

## Phase 2 — `usize`/`u32` → `u8`/`u16` field narrowing (REAL RAM win)

`usize` is 4 bytes on all console targets. These fields hold provably small
values; every one saves 3 bytes on console (7 on desktop). This is the actual
highest-value memory change.

### GameState (`core/game_state/mod.rs`)
| Field | Now | → | Bound |
|---|---|---|---|
| `mulligan_selected_indices` (:83) | `SmallVec<[usize;2]>` | `SmallVec<[u8;2]>` | hand idx 0..=6 |
| `live_card_selected_indices` (:84) | `SmallVec<[usize;3]>` | `SmallVec<[u8;3]>` | hand idx 0..=6 |
| `scratch_entry_positions` (:114) | `HashMap<i16, Option<usize>>` | `HashMap<i16, Option<u8>>` | stage slot 0..=2 |
| `last_vacated_stage_area` (:163) | `Option<usize>` | `Option<u8>` | stage slot 0..=2 |
| `activating_ability_index` (:191) | `Option<usize>` | `Option<u8>` | per-card ability idx |
| `depth_first_cutoff` (:206) | `Option<usize>` | `Option<u16>` | queue length |
| `turn_limited_abilities_used` (:82) | `HashMap<(i16,usize,u8),u8>` | `HashMap<(i16,u8,u8),u8>` | per-card ability idx |

**Not changed:** `just_completed_ability_key: Option<u32>` /
`this_batch_triggered_ability_ids: SmallVec<[u32;16]>`. A u32→u16 repack was
proposed but is **unsafe**: `(card_id as u32) << 16 | ability_idx` needs ~12
bits for card_id (2280 cards) + ability_idx bits > 16 total.

### Energy zone (`core/zones.rs:646`)
`EnergyZone.active_energy_count: usize` → `u8` (bounded by
`MAX_ENERGY_CARDS = 12`). Ripples into method signatures
`set_active_count`/`add_active`/`sub_active`/`can_pay_energy`/
`pay_energy_count`/`pay_energy` and casts at `player.rs:307,359`,
`game_setup.rs:1316+`, `move_cards.rs:1960`.

### Ability queue (`ability_queue.rs`)
- `AbilityQueueEntry.ability_index: usize` (:71) → `u8`
- `AbilityQueueEntry.cost_paid_index: usize` (:79) → `u8`
- `QueueState` variant payloads `entry_index: usize` (:54-60) → `u8`
- `AbilityQueue.current_index: usize` (:127) → `u8`

### Ability resolver (`ability/resolver.rs`)
- `current_ability_index: Option<usize>` (:52) → `Option<u8>`
- `selected_count_at_save: Option<usize>` (:66) → `Option<u8>` (0..=4)

### Ability types (`ability/types.rs`)
- `Choice` counts: `SelectHeartColor`/`SelectHeartType`/`SelectLiveSuccess`
  `count: usize` (:154,169,202) → `u8`
- `LiveSuccessOption.card_index` (:239) → `u8`; `AutoAbilityOption.queue_index`
  (:250) → `u8`
- `ChoiceResult` payloads: `CardSelected { indices: Vec<usize> }` (:257) →
  `Vec<u8>`; `AutoAbilitySelected.queue_index` (:262) → `u8`;
  `LiveSuccessSelected.card_index` (:263) → `u8`
- `ExecutionContext::SingleEffect { effect_index }` (:272) → `u8`
- `LookAndSelectStep`: `LookAt.count` (:289) → `u8`; `Select.count` (:293) → `u8`
- `EffectSpawnContext.position: Option<usize>` (:796) → `Option<u8>`
- `StepState.looked_at_total_count` (:1026) → `u8`
- `ZoneSnapshot` counts `hand/stage/waitroom/energy/active_energy/deck_count`
  (:913-918) → `u8`
- `LogMetadata::TriggerEvaluation.ability_index` (`core/types.rs:905`) → `u8`

### Display / wire layer (`game/display.rs`, built transiently, lower priority)
`ability_index: usize` (:59) → `u8`; `activating_ability_index: Option<usize>`
(:462) → `Option<u8>`; `amount: i32` (:75) → `i16`; bonus/set `blade/score/cost:
i32` (:117-157) → `i16`; deck/energy counts `usize` (:195-244) → `u8`;
`mulligan_selected_indices: Vec<usize>` (:558) → `Vec<u8>`.

---

## Phase 3 — `i32` → `i16` in modifiers (RAM win across maps)

`engine/src/core/game_modifiers.rs:36,38`:

```rust
pub struct ModifierEntry {
    pub additive: i32,   // accumulated via repeated add_* / += calls
    pub set: i32,        // absolute override
}
```

8 bytes → **4 bytes per entry**. Stored in ~6 per-player HashMaps
(`blade/heart/cost/score/need_heart_modifiers`), so the win multiplies across
every active card+color pair.

**Required before merging:** audit accumulation bounds. All sources derive from
`u8` game values (`effect.value_or_count(1)`, score totals that are `u8`), and
deltas are small (±1..±3 typical). Confirm no single card can accumulate an
`additive` or `set` beyond ±32767 across stacking effects. Follow-ups in
`core/types.rs`: `CardEffectItem.amount` (:222), `CardEffectItemRef.amount`
(:346), `AbilityApplication.amount` (:848), `Adjustment.value` (:874) → `i16`.

---

## Kept / explicitly not doing

- **JSON-path decode oracle** (`kind_from_action` in `card.rs:1068+`,
  `populate_from_json`/`condition_populate_from_json`/`normalize_cost_keys`/
  `recursive_normalize_cost_value` in `vm.rs:1203-1557`, ~700 lines) — kept by
  decision; it backs `bytecode_deep_compare_test.rs`, the bytecode↔JSON
  regression net. Not dead while that test runs.
- **`bytecode_abilities` feature-gate removal** — every consumer enables it;
  removal is compile-only churn across all platform Cargo.tomls. Not worth it.
- **`#[repr(u8)]` on enums** — see §0; measured no-op.
- **Vendored PSP crate** (`platforms/psp/vendored/`) — third-party FFI.
- **`just_completed_ability_key` u16 repack** — overflow risk (see Phase 2).
- Per-deck GBA blob subsets (`load_two_decks`/`resolve_deck_indices` on 256KB
  ROM) — separate infra work, tracked in
  `docs/memory_bytecode_synthesis_roadmap.md`, not this pass.

---

## Verification

1. `cargo check -p rabuka_engine` (default features)
2. `cargo test -p rabuka_engine` (322 integration test modules + QA suite)
3. `cargo check -p rabuka_engine --no-default-features --features
   "no_std,bytecode_abilities,compact_cards,compact_card_data,compact_state"`
   (DS/GBA console combo)
4. `cargo check -p rabuka_engine --no-default-features --features
   "no_std,bytecode_abilities,compact_cards,compact_card_data,compact_state,
   external_card_data"` (PS1 combo)
5. `cargo check -p rabuka_engine --no-default-features --features
   "no_std,debug_conditions,bytecode_abilities"` (PSP combo)
6. 3DS platform: `cargo check -p rabuka_3ds` if toolchain present

---

## What was actually implemented (this pass)

**Phase 1 (dead code):**
- Deleted `cards/build/vm_gen.rs` (5,500-line merge-conflict artifact),
  duplicate `cards/build/{abilities_gen,cards_gen}.rs`, the `ds_debug` feature +
  its 38 dead cfg blocks, 14+ dead functions, dead enum variants
  (`AbilityLogItem::KeyValue`, `DecodeError::SerdeFailed`), dead helpers
  (`get_str`, `get_str_from_blob`, `card_name_by_no`, `fmt_hearts`,
  `fmt_heart_vec`, `resolve_deck_indices`, `record_turn_limited_ability_use`,
  `has_turn_limited_ability_been_used`), stale `#[allow]`s (web_server, tdbg,
  3DS `tprintln!`/`c2d`/`COL_PINK`/`Testing`), and duplicate
  `_prohibition_destination_blocks`.
- `cards/build/{abilities_gen,cards_gen}.rs` added to `.gitignore` (build
  artifacts; canonical copies live in `engine/src`).

**Phase 2 (field narrowing → smallest native type):**
- GameState: `mulligan_selected_indices`/`live_card_selected_indices` →
  `SmallVec<[u8;N]>`, `scratch_entry_positions` → `HashMap<i16,Option<u8>>`,
  `last_vacated_stage_area` → `Option<u8>`, `depth_first_cutoff` → `Option<u16>`.
- `EnergyZone.active_energy_count` + whole API → `u8` (rippled through ~30
  call sites; removed pre-existing `as usize` widenings where values were natively
  u8).
- Ability queue: `QueueState` entry_index, `current_index`, `cost_paid_index` → `u8`.
- Resolver: `selected_count_at_save` → `Option<u8>`; removed dead trace helpers.
- `LogMetadata::TriggerEvaluation.ability_index` → `u8`.
- **Reverted:** the ability-index chain (`AbilityQueueEntry.ability_index`,
  `current_ability_index`, `turn_limited_abilities_used` keys,
  `activating_ability_index`) stays `usize` — the gained-ability sentinel
  `10000 + gidx` exceeds `u8`.

**Phase 3 (i32 → i16):**
- `ModifierEntry { additive, set }` → `i16` with saturating accumulation; public
  API keeps `i16` args (removed `as i32` casts across callers).
- `CardEffectItem.amount`, `CardEffectItemRef.amount`, `EffectData` amounts,
  `AbilityApplication.amount`, `Adjustment.value`, `success_zone_*_bonuses` maps
  → `i16`; `record_ability_application` signature → `i16`.
- `CardEffectItem`/`AbilityApplication`/`Adjustment`/`EffectData` in
  `types.rs` + display layer narrowed to match.

**Verified:** default build clean, all 1936 tests pass, `parser.py` regenerates
bytecode deterministically, no_std lib combos (DS/GBA/PSP/PS1) compile, 3DS lib
compiles (pre-existing warnings only).
