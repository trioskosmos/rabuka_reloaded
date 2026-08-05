# Rabuka Reloaded — Unified Memory & Bytecode Optimization Guide

> **One document to rule them all.** This supersedes the following historical docs
> (kept in git for history but no longer maintained separately):
> - `docs/memory_optimization_plan.md` (previous pass)
> - `docs/memory_bytecode_synthesis_roadmap.md` (Track A/R synthesis)
> - `docs/bytecode_serde_ram_optimization.md` (bytecode/serde RAM)
> - `engine/MEMORY_REFACTOR.md` (original refactor log)

**Status: live.** Current baseline (verified 2026-08-04): `cargo test -p rabuka_engine`
= **1936 tests green**, `parser.py` regenerates bytecode deterministically, `no_std`
lib builds (**all console feature combos — DS/GBA/PS1/PSP/Wii/DC**) compile clean,
3DS lib compiles.

---

## ⛔ Testing policy (the only verification loop) ⛔

1. `python cards/ability_extraction/parser.py` (regenerates abilities.json + bytecode)
2. `cargo test -p rabuka_engine` from the repo root

That is the *entire* correctness signal. `cargo check` is a convenient compile-only
pre-filter during editing but is **not** a substitute for the parser → test loop.

---

## 1. What "memory" means here

The game targets consoles with tiny RAM (GBA 288KB, PS1 2MB, DS 4MB, 3DS 128MB,
PSP 32MB, Wii 24MB, DC 16MB) plus desktop/web/AI training. The engine must ship a full
card game (~2280 cards, ~800 abilities) in that envelope. Two orthogonal resources:

- **ROM/static size** — `abilities.bin` (bytecode), `cards_gen.rs` card blob, embedded
  deck JSON. Costs flash/disk, not RAM.
- **Resident RAM** — decoded `Card`/`Ability`/`Condition` structs, `GameState`,
  `GameModifiers`, logs. This is the scarce resource.

## 2. What is already done (verified in tree)

### 2a. Bytecode ability system (ROM + decode path)
- Abilities compile to a tagged binary blob (`abilities.bin`, ~79KB, ~98 B/ability).
- `AbilityRef` = `u16` index; `resolve()` decodes **on demand** from the embedded
  blob, no cache, no serde, no arena (R2/R5 done).
- Direct binary decoders for `EffectKind`, `Condition`, `PositionInfo`,
  `DynamicCount`, `EffectState`, `QuotedText`, `AbilityEffect`, `AbilityCost`
  (Track A). Zero `serde_json::from_value` in the ability decode path.
- `json_path_test` feature gates the JSON oracle (`populate_from_json`,
  `kind_from_action`, `normalize_cost_keys`, `condition_populate_from_json`) used
  only by `bytecode_deep_compare_test.rs` — the bytecode↔JSON equality guard.
- Card loading is bytecode/blob too (`compact_card_data`): all 2280 cards decode
  from the embedded `CARD_BLOB` (zero serde), header compacted (u32→u16 card count,
  u8 length table), ~6.8KB smaller.

### 2b. Struct sizes (measured on the dev 64-bit build via `size_budget_test`)
| Struct | Size | Notes |
|--------|------|-------|
| `Condition` | **400 B** | tagged enum; largest variant dominates |
| `EffectKind` | **192 B** | `EffectFilter` boxed (544B on heap when present) |
| `AbilityEffect` | **136 B** | flat fields + `Box<CompoundBranch>` + `Option<EkBox>` |
| `Ability` | **112 B** | |
| `AbilityCost` | **136 B** | transparent newtype over `AbilityEffect` |
| `AbilityQueueEntry` | **632 B** | resolver boxed (was 2536B) |
| `AbilityResolver` | **1888 B** | |
| `GameState` | **10448 B** | under 13000B budget |
| `Player` | **648 B** | |
| `Card` | **328 B** | `compact_cards` gates display-only fields |
| `GameModifiers` | **904 B** | |
| `ModifierEntry` | **4 B** | `{ additive: i16, set: i16 }` |

### 2c. Integer narrowing completed
All `u32`/`i32`/`usize` struct fields in the core engine (excluding bots, web server,
tests, generated code) are now the smallest native type:

**GameState** (`core/game_state/mod.rs`):
- `mulligan_selected_indices` → `SmallVec<[u8;2]>`, `live_card_selected_indices` → `SmallVec<[u8;3]>`
- `scratch_entry_positions` → `HashMap<i16, Option<u8>>`
- `last_vacated_stage_area` → `Option<u8>`
- `depth_first_cutoff` → `Option<u16>`
- scratch buffers `scratch_exp_blade/cost/score` → `HashMap<i16,i16>`,
  `scratch_exp_heart` → `HashMap<i16, HashMap<String,i16>>`

**Energy zone** — `active_energy_count` + full API (`active_count`, `set_active_count`,
`add_active`, `sub_active`, `can_pay_energy`, `pay_energy`, `pay_energy_count`) → `u8`.
Rippled through ~40 call sites; removed pre-existing `as usize` widenings.

**Ability queue** — `QueueState` entry_index, `current_index`, `cost_paid_index` → `u8`.

**Modifiers** (`core/game_modifiers.rs`):
- `ModifierEntry { additive, set }` → `i16` with **saturating** accumulation
- All `constant_*_bonuses` maps → `HashMap<i16, i16>`; `constant_heart_bonuses` inner → i16
- `constant_global_need_heart` / `constant_score_sources` → `Vec<(i16, String, i16)>`
- `success_zone_*_bonuses` → `HashMap<i16, i16>` (and inner heart maps)
- `p1/p2_constant_total_score_bonus` → `i16` (+ `calculate_live_score` param → i16)

**Effect data** (`core/types.rs`): `CardEffectItem.amount`, `CardEffectItemRef.amount`,
`EffectData` amounts, `AbilityApplication.amount`, `Adjustment.value` → `i16`.

### 2d. Dead code removed (this + previous passes, ~2300 lines)- `cards/build/vm_gen.rs` (5500-line merge-conflict artifact)
- Duplicate generated `cards/build/{abilities_gen,cards_gen}.rs` (gitignored now)
- `ds_debug` feature + 38 dead cfg blocks
- Dead functions: `buffer_log`, `cache_size`, `init_ability_store`,
  `resolve_deck_indices`, `trace_effect_start/end`, `get_trace_node`,
  `expire_live_end_effects`, `get_triggerable_abilities`, `mark_energy_as_wait`,
  `player_set_live_cards`, `snapshot_to_rule_log`, `count_stage_members_with_trigger`,
  `trigger_and_process_auto_abilities`, `evaluate_position_change_condition`,
  `test_q30_*`, `AbDebug::print_cost`, `Zone::label`,
  `record_turn_limited_ability_use`, `has_turn_limited_ability_been_used`,
  `get_str`, `get_str_from_blob`, `card_name_by_no`, `fmt_hearts`, `fmt_heart_vec`
- Dead variants/fields: `AbilityLogItem::KeyValue`, `DecodeError::SerdeFailed`,
  `StepState.looked_at_total_count`
- Duplicate `_prohibition_destination_blocks` free fn
- Stale `#[allow]`s + 3DS dead `tprintln!`/`c2d`/`COL_PINK`
- Opcode encoder (`compile_one`/`compile_condition`/`compile_cost`), arena subsystem,
  `RESOLVED_ABILITIES` cache, `CondBox` pool, `once_cell` dep (earlier tracks)

### 2e. Console feature-combo fixes (this pass)
The engine `psp`/`wii`/`dc` feature definitions were missing `compact_card_data`
(and `dc` was missing `bytecode_abilities`), so `load_two_decks`'s blob branch
referenced the gated `card_binary`/`vm` modules and those combos failed to compile.
Fixed to match DS/GBA/PS1: `psp`/`wii`/`dc` now all enable `compact_card_data`
(+ `bytecode_abilities` for `dc`). All six console combos now build clean.

---

## 3. Remaining opportunities (ranked)

### R1 — `game_state_history: Vec<u64>` (loop detection) — resolved
Uncapped by decision: `check_permanent_loop` pushes every distinct state hash with no
trimming, so any state repeat (however long between occurrences) is caught.
The `max_state_history_size` field and `DEFAULT_HISTORY_SIZE` constant were removed.

### R2 — `deck_parser.rs` serde path — JSON→bytecode completion
`load_two_decks` has a `serde_support` branch that `serde_json::from_str`s the
`include_str!` deck JSON files; the `no_std` branch already decodes per-deck blobs.
- The `DECK_CARD_FILES` `include_str!` JSON strings (~16 files) ship in the binary on
  serde builds. Could route serde builds through the blobs too and drop the JSON
  include_str entirely, or gate the JSON behind a feature. **Modest ROM win.**
- `parse_all_decks`/`parse_deck_file` are desktop/AI-only (main.rs, harness.rs,
  web_server) — fine as-is.

### R3 — `Condition` is 400B (largest struct)
The enum is dominated by the largest variant (Location, ~30 `Option` fields, mostly
`Option<ArcStr>` = 16B each with niche). The `debug_conditions` feature already gates
`text`/`trigger_event`. Further reduction options:
- **Box `location_sub_checks`-style heavy sub-structs** — already boxed where noted.
- **Coalesce same-typed `Option<ArcStr>` fields** that are mutually exclusive per
  card (e.g. `source`/`destination`/`target` rarely all set) — **high churn, low
  value**; each is independently read by the evaluator.
- **`Operation`/`state`/`placement_order` enums** (O4 in the old roadmap) — the
  remaining P3 enum conversions. `operation: Option<ArcStr>` appears on EffectKind
  variants; converting to an enum saves ~14B/variant and enables `match` dispatch.
  **Medium value, medium churn.**
**Recommendation:** do the P3 enum conversions (`operation`, `state`, `placement_order`)
as the next struct-level win; skip the mutually-exclusive-field coalescing.

### R4 — `EffectFilter` heap footprint
`EffectFilter` is 544B, boxed per effect (`Option<Box<EffectFilter>>`). ~2280 cards ×
filter-bearing effects. On a full deck load this is the largest per-card heap consumer
after `Condition`. Options:
- **Lazy `EffectFilter`**: only construct it when a filter is actually read. Many
  effects carry an empty/default filter that the evaluator never touches.
- **Intern shared filters**: identical filter sets (same card_type/group combos)
  dedupe via Arc — could reuse `ArcStr`-style interning.
**Recommendation:** profile first; `EffectFilter` is already boxed so it costs 8B on
the enum + 544B on heap per filter-bearing effect.

### R5 — Transient choice structs (low value, skip unless profiling shows residency)
`ChoiceBuilder.count`, `SelectionContext.count`, `LiveSuccessOption.card_index`,
`AutoAbilityOption.queue_index` are `usize`. They're built per-choice and dropped;
narrowing to u8 touches every `.len()` comparison for ~2 bytes each. **Skip.**

### R6 — Web server / bots (out of scope for console)
`web_server.rs` keeps serde (it IS the API). `bot/` uses u32/usize (iterations,
rollout_depth, observation sizes) — only compiled for desktop/AI. **Skip.**

### R7 — Remaining `as i16`/`as u8` casts
A few casts remain at modifier-map boundaries (values from `effect.value_any()` etc.
are `u8` internally but the local accumulators are `i16`/`i32`). These are correct and
bounded; no action needed unless a cast sites audit finds an overflow path. The
saturating arithmetic in `ModifierEntry` prevents silent wraps.

---

## 4. The deferred "150KB RAM" north star (historical targets, current status)

| Consumer | Old target | Status |
|----------|-----------|--------|
| Card data | 1.4KB/120 cards packed | PARTIAL — blob in ROM, deck-load only; packed `PackedCard` deferred |
| Ability structs in RAM | 0 (decode on demand) | DONE |
| Bytecode IS the effect | 0 materialized structs | DEFERRED (still materializes `AbilityEffect`) |
| GameState | 2.5KB fixed arrays | DONE-ish (compact_state caps logs; fields narrowed) |
| String data | 0 runtime | DEFERRED (ArcStr refcounted, not indexed) |
| **Total** | **~150KB** | ~450–600KB today |

## 5. Arena / pools — explicitly closed
- `arena_allocator` subsystem **removed** (blocked: game-state grows into the bump).
- `CondBox` pool **removed** (unwired); `EkBox` kept (wired into `AbilityEffect.kind`).

---

## 6. How to measure

```bash
# Struct sizes (regression budget test)
cargo test -p rabuka_engine --test run_all size_budget -- --nocapture

# Parser + full test (the only verification loop)
python cards/ability_extraction/parser.py
cargo test -p rabuka_engine
```

---

## 7. Current verified sizes (2026-08-04, dev 64-bit)

```
  AbilityQueueEntry          632 B   (budget 700)
  AbilityResolver           1888 B   (budget 2200)
  GameState                10448 B   (budget 13000)
  Player                     648 B   (budget 900)
  Card                       328 B   (budget 400)
  Ability                    112 B   (budget 160)
  AbilityEffect              136 B   (budget 200)
  GameModifiers              904 B   (budget 1200)
  PerformanceSnapshot        344 B   (budget 500)
  LogEntry                   232 B   (budget 300)
  MemberContribution         104 B   (budget 160)
  MovementEvent               40 B   (budget 60)
  Allocation                  32 B   (budget 40)
  AbilityApplication          32 B   (budget 40)
  PositionChangeEvent         40 B   (budget 64)
  Adjustment                  56 B   (budget 64)
  AbilityBonus                40 B   (budget 40)
```

---

## 8. Explicitly NOT doing (decisions)

- **`#[repr(u8)]` on fieldless enums** — measured no-op (Rust already packs them).
- **u16 repack of `just_completed_ability_key`** — the gained-ability sentinel
  `10000 + gidx` exceeds u16 (12-bit card_id + ability_idx > 16).
- **JSON-path decode oracle deletion** — kept as the bytecode↔JSON regression net
  (`bytecode_deep_compare_test.rs`).
- **`bytecode_abilities` feature-gate removal** — compile-only churn; every consumer
  enables it.
- **Vendored PSP FFI** (`platforms/psp/vendored/`) — third-party.
- **Arena per-turn reset** — blocked (measured 2–22KB live game-state growth into the
  bump); superseded by removal (R2).
- **`debug_conditions` field strip** — `text`/`trigger_event` are load-bearing
  (condition-cache key + source/state fallbacks).
- **Bounded log buffers** — display already caps at 500; ~50 writer sites for a modest win.
- **HashMap consolidation in GameModifiers** — heterogeneous types/lifecycles; already
  well-encapsulated.
- **`resolve_deck_indices`** — deleted as dead; per-deck blobs decode directly.
