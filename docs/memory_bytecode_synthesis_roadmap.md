# Memory × Bytecode × Serde: Unified Roadmap

A synthesis of:
- `engine/MEMORY_REFACTOR.md` — RAM + CPU reduction for resource-constrained targets
- `docs/bytecode_serde_ram_optimization.md` — zero-serde ability decode + bytecode size

**Purpose:** merge both efforts into one north star, re-baseline against the *current, verified*
code state (2026-08-01), and lay out the concrete next phases. Every "done" claim below was
confirmed against the source; every number was re-measured from the current tree.

---

> ## ⛔ TESTING POLICY — THE ONLY MECHANISM. READ BEFORE DOING ANYTHING. ⛔
>
> **There is exactly ONE verification loop for all work in this roadmap:**
>
> 1. **Run the parser:** `python cards/ability_extraction/extract_card_abilities.py`
>    (from the repo root — this single command chains `compile_abilities.py`, both
>    decoder generators, and `validate_schema.py`).
> 2. **Run the tests:** `cargo test` (from `engine/`, NOT the workspace root).
>
> **Nothing else. Ever.**
>
> - **NEVER run `cargo check`.** It is not a valid verification step for this repo and it
>   is effectively **banned**. Do not run it, do not rely on it, do not "just check".
> - **NEVER run** `cargo build`, `cargo run`, `cargo bench`, `cargo clippy`, `cargo fmt`,
>   size probes, `RABUKA_ALLOC_TRACK` runs, `size_check` binaries, or any other tool to
>   "verify" a change. All of those are **off the table** for this work.
> - The parser → `cargo test` loop is the **only** signal that tells you a change is correct.
> - If `cargo test` (after running the parser) is green, the phase is done. If it is not
>   green, fix the code and re-run — **do not reach for any other tool**.
> - This policy is **mandatory**, not a suggestion. Violating it (e.g. treating `cargo check`
>   as a substitute) means the work is not verified, period.

---

## 1. Verified current state (re-baseline)

### Measured type sizes (historical baseline; measured once for the re-baseline, not a step to re-run)

| Type | Size | Notes |
|------|------|-------|
| `Condition` | **400 B** | tagged enum, 20 variants. doc said ~520; now 400 |
| `EffectKind` | **192 B** | via `EffectFilter` sub-struct boxed into each variant |
| `EffectFilter` | **544 B** | lives on heap; the ~192 B enum is a pointer+small fields |
| `AbilityEffect` | **136 B** | flat fields + `Box<CompoundBranch>` + `Option<EkBox>` |
| `Ability` | **112 B** | |
| `AbilityCost` | **136 B** | transparent newtype over `AbilityEffect` |
| `CompoundBranch` | **96 B** | all sub-effects boxed |
| `ActionType` | **1 B** | enum, 1 byte |
| `PositionInfo` / `DynamicCount` / `QuotedText` | 40 / 96 / 48 B | direct decoders done |
| `AbilityQueueEntry` | **2536 B** | **not in either doc** — biggest surprise (see Phase 5) |

### Remaining `serde_json::from_value` calls in the ability decode path

| Location | Type decoded | Status |
|----------|--------------|--------|
| `read_condition_value` | `Condition` (20 variants, internally tagged) | **GONE** (A1, 694913d4) |
| `decode_ability_cost` | `AbilityEffect` inside `AbilityCost` | **GONE** (A2) |
| `decode_ability_effect_from_object` | `AbilityEffect` (TAG_OBJECT fallback) | **GONE** (A3, 06aece4e) |

The ability decode path is fully direct and bytecode-only. The remaining
`serde_json::from_value` calls are all in the JSON oracle/test path (`kind_from_action`'s
`dynamic_count` fallback in `card.rs`, `qa_test_suite.rs`) — **gated behind `json_path_test`
(R4, `420eae75`)**, compiled only for the deep-compare oracle, off for `ds`/`no_std`.

### Serde/JSON infrastructure (ability-path; gated by R4 — compiled only for the oracle)

| Symbol | Location | ~Lines | Status |
|--------|----------|--------|--------|
| `populate_from_json` | `vm.rs` | ~115 | `#[cfg(json_path_test)]` |
| `condition_populate_from_json` | `vm.rs` | ~54 | `#[cfg(json_path_test)]` |
| `normalize_cost_keys` + `recursive_normalize_cost_value` | `vm.rs` | ~35 | `#[cfg(json_path_test)]` |
| `kind_from_action` (JSON twin of `build_*`) | `card.rs` | ~345 | `#[cfg(json_path_test)]` |
| `Deserialize` derives | `card.rs` etc. | — | needed for card/deck loading + oracle |

> These **ability-path** symbols remain only for the deep-compare oracle
> (`tests/test_modules/bytecode_deep_compare_test.rs`), the bytecode↔JSON equality guard.
> The hot path never touches them; `ds`/`no_std` builds compile without them.
> Note: serde/serde_json are *also* used in production **outside** the ability path
> (`card_loader.rs`, `deck_parser.rs`, `web_server.rs`, DS `DECKS_JSON`) — see §8.

### Memory refactor state

| Item | Status |
|------|--------|
| Lazy/decode-on-demand abilities (`AbilityRef` = u16) | DONE (`RESOLVED_ABILITIES` cache removed, R5) |
| Compact cards (`compact_cards` + `compact_card_data` blob) | DONE (blob wired on 3DS only — see §8) |
| Compact GameState (`compact_state`) | DONE |
| HeartMap → `SmallVec<[(HeartColor, u8); 4]>` | DONE (already u8) |
| `arena_allocator` subsystem | **REMOVED** (R2, `70eabeb7`) |
| `EkBox` pool (128 slots) | LIVE, wired into `AbilityEffect.kind` (kept, R6) |
| `CondBox` pool (64 slots) | **REMOVED** (unwired, R6) |
| P3 enums (`card_type`, `orientation`, `zone`) | DONE |
| P3 enums (`operation`, etc.) | PARTIAL — **O4: deferred** |

---

## 2. The unified north star

Both documents converge on one principle:

> **The Python compiler already knows the shape of every ability. It should encode that
> shape (variant tags + aliases) into the bytecode so the Rust decoder never has to
> re-derive structure from string keys or `serde_json::Value`.**

- The **bytecode doc** chases this for `EffectKind` (done). It stops there.
- The **memory doc** chases the same spirit for RAM (pools, arena, lazy decode), but its
  arena/CondBox work is *blocked by serde*.

Track A (zero-serde decode) is complete. The next phase of work is **removal-first**
(Track R): delete or gate the systems that no longer pay for themselves before optimizing
what remains. `Condition` and `AbilityCost` no longer decode through serde, so the
ability-path JSON decode system is now only kept for the oracle.

### Guiding principle (2026-08-02): remove before optimize

> **Prefer deleting whole systems/resources over squeezing existing ones.** Every
> optimization (boxing a struct, shrinking a field, resetting the arena) keeps the
> system alive and adds code. The cheapest byte is the one never shipped, and the
> cheapest CPU cycle is the one never spent. So the priority order is:
>
> 1. **Delete** systems that no longer pay for themselves (dead encoders, blocked
>    features, debug/test-only paths, caches whose RAM cost outweighs their CPU win).
> 2. **Gate** resource-heavy systems behind features so `ds`/`no_std` builds ship
>    without them (serde JSON oracle, debug instrumentation, `debug_conditions`).
> 3. **Consolidate** duplicate paths (JSON decode vs bytecode decode) down to one.
> 4. Only then **optimize** what remains — and only with a measured win.

Track A is complete; the remaining "squeezing" phases (B1/B2/B3, C1) are **deferred**
(§5) in favour of the removal-first Track R (§4).

### Constraint honored throughout

Verification is **parser → `cargo test` only** (see the TESTING POLICY banner at the top).
`cargo test` from `engine/` must stay green at every step — in particular
`bytecode_deep_matches_json_path`, which guarantees bytecode decode == JSON decode. The
JSON decode path stays available (it is the oracle); only the production hot path goes
direct. **Never** use `cargo check` or any other tool to verify a phase.

---

## 3. Track A — Finish zero-serde decode (continuation of the bytecode doc)

> **Status: DONE.** All three `serde_json::from_value` calls in the ability decode path
> are gone (A1 `694913d4`, A2 + A3 `06aece4e`). `vm_gen.rs` deleted. Sections A1–A3 below
> are kept as the historical record of how it was done.

### Phase A1 — Condition direct decoder (unblocks vm.rs:374)

**Goal.** Kill the `serde_json::from_value::<Condition>` call and the `serde_json::Value`
tree reconstruction in `read_condition_value`.

**Why.** Conditions are the most frequent nested decode in the engine (every effect's
`condition`, plus `alternative_condition`, `result_condition`, `choice_condition`,
`activation_condition_parsed`, and compound/recursive sub-conditions). Each decode
currently builds a full `serde_json::Map` + `Value` + runs serde tag dispatch + then
`condition_populate_from_json` walks it *again*. That is two full JSON walks per condition.

**How.** Mirror the EffectKind pattern exactly:

1. **Compiler** (`cards/compile_abilities.py` `enc_val`): add `COND_TO_VARIANT_TAG` (20
   variants, 1 byte each) and emit `TAG_OBJECT_VARIANT` + tag for any dict whose `type`
   maps to a condition tag and that has no `action`. Keep `TAG_OBJECT` as the fallback for
   unknown dicts.
2. **Decoder** (`vm.rs::read_condition_value`): on `TAG_OBJECT_VARIANT`, read the tag, then
   dispatch to `decode_condition_direct(bc, variant, count)`.
3. **Generated code** (`condition_decoder_gen.rs`, new file, generated by a new/extended
   `generate_effect_decoder.py`): a `ConditionLocals` accumulator struct + a
   `decode_condition_field(bc, key, locals)` dispatcher (one arm per field, reusing the
   *existing* readers — `read_arc_str_value`, `read_u8_value`, `read_bool_value`,
   `read_opt_str_vec_value`, `read_position_value`, `read_effect_state_value`,
   `read_condition_value`, etc.) + per-variant `build_*` constructors.
4. **Cleanup:** delete `condition_populate_from_json` from the hot path (keep it compiled
   for the test oracle if it is still reachable from the JSON path).

**Field-type trap.** Unlike `EffectKind`, `Condition` has several *differently-typed*
homonyms across variants (`state: Option<EffectState>` vs `state: Option<CardState>`;
`distinct: Option<Box<DistinctInfo>>`; `card_type: Option<ConditionCardType>`;
`operator: Option<ArcStr>`; `ability_filter: Option<AbilityFilter>`). A single
`ConditionLocals` field type must be the widest/superset type per key name, or the decoder
must be per-variant. Prefer per-key superset accumulation with a `build_*` per variant that
casts/narrows — same shape the `EffectKindLocals`/`build_filter` split already uses, so the
generator pattern transfers.

**Files:** `cards/compile_abilities.py`, `cards/generate_effect_decoder.py` (new condition
emission), new `engine/src/ability/condition_decoder_gen.rs`, `engine/src/ability/vm.rs`.

**Test gate:** parser → `cargo test` from `engine/` (deep-compare proves parity; it runs under
default features, which include `bytecode_abilities`).

**Effort/risk:** ~1–2 days. High-touch (20 variants × ~30 fields) but mechanical; the
deep-compare test catches every field miss. This was the largest single piece of Track A.

---

### Phase A2 — Cost direct decoder (unblocks vm.rs:886)

**Goal.** Kill the `from_value` inside `decode_ability_cost` and the
`normalize_cost_keys`/`collect_json_map` pair.

**Key insight — don't write a new decoder, alias in the compiler.** `AbilityCost` is
`AbilityCost(pub AbilityEffect)`, and cost objects are already shaped like effects. The
only reason they take the JSON path is that costs carry `type`/`zone` instead of
`action`/`source`. Fix it where the shape is known — in `enc_val`:

```python
# inside enc_val, before ACTION_TO_VARIANT_TAG lookup:
if "action" not in v:
    t = v.get("type", "")
    if t in COST_TYPE_TO_ACTION:          # move_cards→move_cards, pay_energy→pay_energy, reveal→reveal
        v["action"] = COST_TYPE_TO_ACTION[t]
    if "source" not in v and "zone" in v:
        v["source"] = v["zone"]           # zone→source alias
```

Then cost dicts flow through the *existing* `TAG_OBJECT_VARIANT` + `decode_ability_effect_direct`
path with **zero new decoder code**. `decode_ability_cost` becomes:

```rust
fn decode_ability_cost(bc: &mut BcReader) -> Option<Option<Box<AbilityCost>>> {
    let tag = bc.read_u8()?;
    match tag {
        TAG_NULL => Some(None),
        TAG_OBJECT_VARIANT => {
            let variant = bc.read_u8()?;
            let inner = decode_ability_effect_direct(bc, variant)?;
            Some(Some(Box::new(AbilityCost(inner))))
        }
        TAG_OBJECT => { /* keep serde fallback temporarily, then delete */ }
        _ => None,
    }
}
```

**Cleanup:** delete `collect_json_map`, `recursive_normalize_cost_value`, and the runtime
`normalize_cost_keys` call. Keep `pub fn normalize_cost_keys` compiled (the deep-compare
test calls it on raw JSON). Verify the 3 real cost types (`move_cards`, `pay_energy`,
`reveal`) cover all costs in `abilities.json` (checked: yes).

**Files:** `cards/compile_abilities.py` (~10 lines), `engine/src/ability/vm.rs` (~30).

**Test gate:** parser → `cargo test`. **Effort/risk:** 2–4 h, Low.

---

### Phase A3 — Delete the TAG_OBJECT effect fallback + dead JSON decode

**Goal.** Remove the third `from_value` (`vm.rs:1003`, `decode_ability_effect_from_object`)
and the 2,300-line dead `vm_gen.rs`.

**How.**
1. After A1/A2, verify no real ability hits the `TAG_OBJECT` branch of
   `read_effect_value`/`read_effect_vec_value` (add a debug counter / test sweep over all
   abilities: every decoded effect must come from `TAG_OBJECT_VARIANT`).
2. Delete `decode_ability_effect_from_object` and the `TAG_OBJECT` arms in
   `read_effect_value`, `read_effect_vec_value`, `read_condition_value`, `decode_ability_cost`.
3. `vm_gen.rs` is generated-but-never-included (`engine/src/ability/vm_gen.rs`; no
   `include!`, no `mod`). Delete the file and remove the `generate_vm_gen` call in
   `compile_abilities.py` (or keep the generator but stop shipping output — prefer delete).
4. `populate_from_json`, `kind_from_action`, and the `Deserialize` derives remain **only**
   because the deep-compare test uses them. Leave them compiled for now; gate later (Phase 7).

**Files:** `engine/src/ability/vm.rs`, `engine/src/ability/vm_gen.rs` (delete),
`cards/compile_abilities.py`.

**Test gate:** parser → `cargo test`. **Effort/risk:** 1–2 h, Low (after verification).

---

## 4. Track R — Remove systems and resources (new priority)

Under the "remove before optimize" principle. Each phase deletes or gates a whole system,
not a micro-tweak. Ordered by **value ÷ risk**: pure zero-risk deletions first, then the
biggest shipped-code cuts, then feature-gating, then the sweep.

### Phase R1 — Delete the dead opcode encoder (`compile_one`/`compile_condition`/`compile_cost`)

> **Status: DONE (2026-08-02, `af05a47e`).** Deleted the opcode encoder cluster
> (`compile_one`/`compile_condition`/`compile_cost` + `COND_OPCODES`/`COND_FIELDS`/
> `COST_OPCODES`/`EFFECT_OPCODES` tables, ~377 lines) and the obsolete opcode-vs-schema
> checks in `validate_schema.py`. Byte-identical regen (112121 B) + 1934/1934 green.

**Goal.** Remove the superseded opcode-based encoder in `cards/compile_abilities.py`
(verified: `compile_one`/`compile_condition`/`compile_cost` have **no non-recursive
caller** — `enc_val` is the only encoder). Also delete the opcode tables they alone use
(`COND_OPCODES`, `COND_FIELDS`, `COST_OPCODES`, `EFFECT_OPCODES`). ~300 lines of pure
deletion.

**How.**
1. Delete `compile_one`, `compile_condition`, `compile_cost` and the opcode tables that
   have no other reader.
2. Sweep `vm.rs` for opcode-era readers that no encoder emits (anything beyond the
   `TAG_OBJECT_VARIANT` + direct readers). Delete unreachable arms.
3. Confirm `compile_abilities.py` re-runs to byte-identical output (`cards/build/*.bin`,
   `abilities_gen.rs`, `condition_decoder_gen.rs`).

**Files:** `cards/compile_abilities.py`, `engine/src/ability/vm.rs`.
**Gate:** parser → `cargo test` (byte-identical regen is proven by `cargo test` after
re-running the parser). **Effort/risk:** 1–2 h, Low.

---

### Phase R2 — Remove the `arena_allocator` subsystem

> **Status: DONE (2026-08-02, `70eabeb7`).** Deleted `arena.rs` (81 lines), the
> `arena_allocator` feature, the arena hooks in `ability_store.rs`/`move_cards.rs`/
> `game_state/abilities.rs`/`game_state/mod.rs`, the `ARENA_LIVE_BYTES` metric, and the
> README feature-table rows. 1934/1934 green.

**Goal.** Delete the arena feature entirely: `engine/src/arena.rs`, the `arena_allocator`
feature, the arena hooks in `alloc_counter.rs`, and the arena bypass calls added in
`ability_store.rs` / `move_cards.rs`.

**Why.** The feature is **off by default**, its headline goal (B1 per-turn reset) is
**blocked** (game-state grows into the bump — unsafe to reset), and the feature build has
a **pre-existing flaky allocator panic**. It currently adds complexity and a maintenance
surface for zero shipped benefit. Removing it deletes a whole blocked subsystem.

**How.** Remove the feature + module + hooks; delete the `ARENA_LIVE_BYTES` metric; leave
`alloc_counter` as pure system-alloc accounting. Re-check the `ds` DS build (which has its
own bump allocator in `platforms/ds`) — the engine arena is independent of it.

**Files:** `engine/Cargo.toml`, `engine/src/arena.rs` (delete), `engine/src/lib.rs`,
`engine/src/alloc_counter.rs`, `engine/src/ability/ability_store.rs`,
`engine/src/ability/move_cards.rs`.
**Gate:** parser → `cargo test` (proves no dangling cfg; no test asserts arena stats).
**Effort/risk:** 2–3 h, Low (dead path).

---

### Phase R3 — Strip debug/alloc instrumentation from production builds

> **Status: DONE (2026-08-02, `f46d1908`).** `alloc_tracker` out of default (counting
> allocator no longer wraps every alloc/free); `debug_conditions` kept — its fields are
> load-bearing. 1934/1934 green.

**Goal.** Remove resource-holding instrumentation from shipped targets. **Done (2026-08-02):
`alloc_tracker` removed from default** — the `CountingAllocator` `#[global_allocator]`
(lib.rs:28-30) no longer wraps every malloc/free in atomics + a 14-bucket histogram on
production builds. 1934/1934 green.

**Why `debug_conditions` stays (corrected).** The original plan to flip `debug_conditions`
out of default is **wrong** — its two fields are **load-bearing gameplay state**, not debug
payloads:
- `Condition.text` is the **condition-cache key** and the `same_as_prev` dedup key in
  `compound.rs:207-249`. Gating it off made every condition collapse to key `""`, corrupting
  compound-evaluation caching → **46 gameplay test failures** (verified: 1888/1934).
- `Condition.trigger_event` is the **fallback source** for `source`/`destination`/
  `from_state`/`to_state`/`self_effect_only` (card.rs:3335-3372) and is read directly by
  `condition/state.rs:454`, `condition.rs:329`, `game_state/abilities.rs:325`.

So `debug_conditions` must stay in default; the field-level strip it was chasing is not a
removal, it's a redesign (a cache key must exist regardless).

**Remaining scope (all safe, gated-not-default already or pure deletion):**
- `ds_debug` (`ds_print`/`nds_println`) — already not in default; leave.
- `ABILITY_DEBUG`/`debug.rs`/`log.rs` — runtime-gated by an `AtomicBool`, cheap; leave.
- The `alloc_counter.rs` module (233 lines) stays compiled only under `alloc_tracker`
  (dev-only now).

**Files (done):** `engine/Cargo.toml` (default features).
**Gate:** parser → `cargo test` — **1934/1934 green.**
**Effort/risk:** done, Low.

---

### Phase R4 — Gate the ability-path JSON decode (drop serde from the ability system)

> **Status: DONE (2026-08-02, `420eae75`).** `json_path_test` feature added (in default;
> off for `ds`/`no_std` which use `--no-default-features`). `normalize_cost_keys`,
> `recursive_normalize_cost_value`, `populate_from_json`, `condition_populate_from_json`,
> and `kind_from_action` are `#[cfg(feature = "json_path_test")]`; the deep-compare oracle
> runs only under both `bytecode_abilities` + `json_path_test`. 1934/1934 green.

**Goal.** Feature-gate the ability-path JSON decode (`populate_from_json`,
`condition_populate_from_json`, `normalize_cost_keys` + `recursive_normalize_cost_value`,
`kind_from_action`, the ability `Deserialize` derives) behind a dev-only `json_path_test`
feature so `ds`/`no_std` builds compile without that decode *system*.

**Why.** The hot path is 100% direct; these symbols are now used only by
`bytecode_deep_compare_test.rs` (the oracle). Keeping them compiled costs binary size and
RAM on every target that never needs them.

**Scope note (corrected).** serde/serde_json are **not** ability-only: `card_loader.rs`
(`load_cards_from_strs`), `deck_parser.rs`, `web_server.rs`, and the DS `DECKS_JSON` parse
all call `serde_json` in production. R4 gates **only** the ability-path symbols. Dropping
serde from `ds`/`no_std` *entirely* requires reworking card/deck loading too — track that
as a follow-up, not part of R4.

**How.**
1. Add `json_path_test` (dev-only) feature to `engine/Cargo.toml`; put `serde`/`serde_json`
   behind it for `ds`/`no_std` targets (keep them on default/dev for the oracle).
2. `#[cfg(feature = "json_path_test")]` on `populate_from_json`, `kind_from_action`,
   `normalize_cost_keys`, `condition_populate_from_json`, and the ability `Deserialize`
   derives. Prefer `cfg_attr` on derives.
3. Verify: parser → `cargo test` (the deep-compare oracle still runs green under default
   features). No feature-flag cargo runs beyond the policy loop are performed.
4. Optional follow-up: move the oracle test itself behind the feature.

**Files:** `engine/Cargo.toml`, `engine/src/ability/vm.rs`, `engine/src/core/card.rs`,
`engine/tests/test_modules/bytecode_deep_compare_test.rs`.

**Gate:** parser → `cargo test` (deep-compare green on default; the serde drop from
`ds`/`no_std` is verified by the policy loop, not by extra feature-flag cargo runs).
**Effort/risk:** ~half a day, Low–Medium (feature plumbing, not logic; loader serde stays).

---

### Phase R5 — Remove the `RESOLVED_ABILITIES` ability cache

> **Status: DONE (2026-08-02, `928ec7a3`).** Removed the `OnceLock<Mutex<HashMap>>` cache,
> its imports, the cache hit/insert, and the duplicate `no_std` decode branch — `resolve()`
> now always decodes on demand (single path). 1934/1934 green.

**Goal.** Restore the documented no-cache design: `AbilityRef::resolve()` decodes fresh and
the caller drops the `Arc` — reclaiming the ~120 KB cache and its arena bypass wiring.

**Why.** The cache was re-added (eb9d4ff9) for 3DS MP perf, but contradicts the memory
doc's "zero leaked RAM" design and is a persistent allocation we had to bypass around.

**How.** Delete the `OnceLock<Mutex<HashMap<u16, Arc<Ability>>>>` and cache hit/insert; keep
the decode-on-demand path. If decode cost proves load-bearing on the 3DS MP flow, cap the
cache to a small LRU (removal of unbounded growth).

**Files:** `engine/src/ability/ability_store.rs`.
**Gate:** parser → `cargo test` (full suite green; no separate perf runs — the policy loop
is the only verification). **Effort/risk:** 1–2 h, Low; **but** verify the 3DS MP flow it
was added for.

---

### Phase R6 — Pools and dependency sweep: keep only what pays

> **Status: DONE (2026-08-02, `13ce4337`).** Deleted the unwired `CondBox` pool (64 slots,
> zero users) and the dead `once_cell` dependency (zero references in engine + platforms;
> removed from `psp`/`ds`/`wii` features and `platforms/ds/Cargo.toml`). `EkBox` kept
> (wired into `AbilityEffect.kind`). 1934/1934 green.

**Goal.** Decide `EkBox`/`CondBox` pools on measured benefit; remove what doesn't pay.
`CondBox` is defined but not wired — either wire it (only if it moves RAM materially) or
delete it.

**Deps (corrected).** `once_cell` is **dead everywhere** (zero references in engine and
platforms) → delete it and the `ds`/`psp`/`wii` feature links. `rmp-serde` **is** used
(`game_state` save/load, 3DS `cards.bin`, `card_loader` msgpack) → **keep**, don't drop.
`uuid`/`actix`/`tokio`/`bytes` are `server`-only, already optional → leave.

**How.** Judge pools on code footprint and necessity, not on profiling runs. `CondBox` is
unwired → delete it. `EkBox` stays only if it is genuinely wired and paying; otherwise
delete. Remove the dead `once_cell` dep. This is the "remove resources" sweep after the
system removals.

**Files:** `engine/src/core/pool.rs`, `engine/Cargo.toml`.
**Gate:** parser → `cargo test`. **Effort/risk:** half a day, Low.

---

## 5. Track O — Optimization (squeezing) — deferred

Deferred under "remove before optimize". Revisit **only** after Track R, and only with a
measured win. Kept for reference; B1 stays blocked per the note below.

| Phase | Old goal | Status / why deferred |
|-------|----------|------------------------|
| O1 (was B1) | Arena per-turn reset | **BLOCKED** — game-state grows into the bump (measured 2–22 KB live); unsafe to reset. Removal (R2) supersedes it. |
| O2 (was B3) | Box `AbilityResolver` → shrink `AbilityQueueEntry` 2.5 KB → ~100 B | Deferred — micro-opt; only if profiles show queue residency is meaningful. |
| O3 (was B2) | Wire `CondBox` pool into `Condition` | Deferred — see R6 (pool decision). |
| O4 (was C1) | P3 enum conversions (`operation` etc.) | Deferred — byte-level squeeze, lowest value. |

### Phase B1 — Arena v1: cursor reset (the 99% alloc win) *(historical — kept as record; its RABUKA_ALLOC_TRACK/measurement steps are NOT to be re-run)*

> **Status (2026-08-01): BLOCKED — per-turn reset is unsafe with the current
> global-allocator arena.** The safe subset was shipped instead (arena bypass for
> persistent allocations + live-bytes metric). Default suite: **1934/1934 green**.
> Note: the `arena_allocator` feature build also has a **pre-existing flaky failure**
> (a latent capacity-overflow panic in the global allocator — `NonNull::new_unchecked`
> on a huge layout request — exposed by the arena's different heap layout), unrelated
> to Track A/B work.

**Why.** Arena v0 is a monotonic 64 KB bump that fills after ~100–200 ability evaluations
and then falls back to `System`. The docs measured **~15,000 allocs / ~326 KB per trigger**
before the ArcStr/enum work; even now, the remaining ~1,700 allocs/test are mostly
temporaries the arena could absorb. Cursor reset turns pointer-bump into a one-store reset.

**Why it's blocked — measured, not just suspected.** Because `arena_alloc` is the **global
allocator** while `process_current_ability` is active, every allocation made during ability
resolution lands in the bump — *including persistent game-state growth*. Measured with a
live-bytes counter (`ARENA_LIVE_BYTES` in `alloc_counter.rs`): even after bypassing the
ability cache (`RESOLVED_ABILITIES`) and the `Vec::leak` at `move_cards.rs:1169`,
**2–22 KB of arena allocations are never freed at test end** (modifier `HashMap` growth,
tracking maps, queue buffers that grow during resolution). A per-turn reset overwrites that
live data → dangling game-state pointers. The original docs' claim that "the arena serves
only small temporaries after Track A" does not hold for game-state containers.

**Safe subset shipped (this commit):**
- `arena::arena_bypass_enter/exit()` + check in `arena_alloc` — a thread-local opt-out so
  persistent allocations never enter the bump.
- Wired into `AbilityRef::resolve()` (decoded abilities are cached in `RESOLVED_ABILITIES`
  and held in queue entries, so they must be system-allocated) and the `Vec::leak` in
  `move_cards.rs`.
- `ARENA_LIVE_BYTES` metric printed in the `RABUKA_ALLOC_TRACK` report, so future arena work
  can verify the bump holds only temporaries.

**Path forward (to unblock the reset):** either (a) shrink the arena window so it excludes
game-state mutation — enter/exit only around the pure decode/evaluate phases of effect
resolution, letting persistent containers grow on the system allocator; or (b) route every
persistent game-state growth site through the bypass (invasive, many sites). Both are larger
than B1's original estimate; re-scope before attempting.

**How.** *(original plan, superseded by the blocker above)*
1. Move `arena_enter`/`arena_exit` (`abilities.rs:1235` / `game_state/mod.rs:613`) from the
   per-ability `process_current_ability` to the turn boundary (game setup/turn start/end).
2. Implement double-buffer: two 64 KB static buffers; `arena_enter(turn_parity)` selects the
   active buffer; `arena_exit()` at turn end resets the inactive one. ~40 lines in `arena.rs`.
3. Keep `arena_allocator` feature-gated, out of default.
4. Measure with `RABUKA_ALLOC_TRACK` + `CountingAllocator` (already wired via the global
   allocator). Gate: alloc count per test drops materially; all tests pass with the feature on.

**Risk.** A turn with many abilities could exceed 64 KB. Mitigate with a large→System
fallback (already present: `layout.size() > 4096` bypass) and optionally a 256 KB buffer.

**Files:** `engine/src/arena.rs`, `engine/src/core/game_state/abilities.rs`,
`engine/src/core/game_state/mod.rs`.

**Test gate:** full suite on default (green) + `--features arena_allocator` (pre-existing
flaky failures, separate from this phase).

**Effort/risk:** 0.5–1 day, Medium → **blocked**; safe subset shipped, reset needs re-scope.

---

### Phase B2 — Wire `CondBox` into `Condition` *(historical, deferred — see O3/R6)*

**Why.** `Condition` is 400 B; nested conditions are `Box<Condition>` (heap alloc per
condition). A 64-slot pool recycles them. The docs abandoned this because serde
deserialization of `#[serde(tag = "type")]` through the pool failed 6 Kasumi tests.

**After A1, the serde blocker is gone**: `decode_condition_direct` constructs `Condition`
by hand, so `read_condition_value` can return `CondBox` directly.

**How.**
1. Change `read_condition_value` to return `CondBox`; change the `Condition`-holding fields
   (`AbilityEffect.condition`, `CompoundBranch.alternative_condition`/`result_condition`,
   `Condition` variants' `condition`/`cause`/`conditions` elements) to `CondBox` where the
   pool pays off (top-level conditions).
2. Add `Deserialize` to the `make_pool_box!` macro (deserialize → `CondBox::new`) so the
   deep-compare test oracle still compiles, and keep `Serialize` (already there).

**Trade-off.** The pool is a `Mutex<Vec<usize>>` global — fine for the single-threaded
console and test threads (EkBox already proves it). But pool leaks between tests are
impossible to detect (static). Cap it and prefer the arena for the bulk.

**Test gate:** full suite. **Effort/risk:** 0.5–1 day, Medium.

---

### Phase B3 — Shrink `AbilityQueueEntry` (2536 B → ~96 B + heap) *(historical, deferred — see O2)*

**Why.** This is now the single largest per-queue-entry struct and is **missing from both
docs**. The 2,536 B is dominated by `resolver: Option<AbilityResolver>` inlined (holds
`current_ability: Option<Ability>`, `current_effect: Option<AbilityEffect>`, `pipeline`,
`step_state`, `spawn_context`, etc.), plus `pending_actions: Vec<AbilityEffect>` and two
`SmallVec` snapshots.

**How.**
1. `resolver: Option<AbilityResolver>` → `Option<Box<AbilityResolver>>`. The resolver is
   only non-None while an entry is actively resolving; idle entries (most of the queue)
   drop ~2.4 KB each. One alloc per active resolution — amortized, and the resolver already
   lives across choice round-trips so it is a single alloc per ability, not per step.
2. Optional follow-up: `pending_actions: Vec<AbilityEffect>` → `Vec<Box<AbilityEffect>>`
   only if profiles justify the extra indirection (136 B → 8 B per pending action).

**Files:** `engine/src/ability_queue.rs`, `engine/src/ability/resolver.rs` (no change to
resolver logic, only the field type + `Box::new` at construction).

**Test gate:** full suite. **Effort/risk:** 1–2 h, Low.

---

### Track C — Remaining trims (historical; re-mapped into Track R / Track O)

The old C1–C4 trim list no longer stands alone — each item moved into the removal track or
the deferred optimization track:

| Old item | Now |
|----------|-----|
| C1. P3 enum conversions (`operation` etc.) | **O4** — deferred, lowest value. |
 | C2. `debug_conditions` for `no_std` (strip `text`/`trigger_event`) | **R3** — DONE: `alloc_tracker` stripped from default. The `text`/`trigger_event` fields stay (load-bearing: cache key + source/state fallbacks). |
| C3. Dead `compile_one`/`compile_condition`/`compile_cost` encoder | **R1** — delete the dead opcode encoder. |
| C4. `kind_from_action` + `Deserialize` feature-gating (ability-path serde) | **R4** — gate the ability-path JSON decode. |

---

## 6. Suggested execution order

| Order | Phase | Type | Commit message | Status |
|-------|-------|------|----------------|--------|
| 1 | R1 Delete dead opcode encoder | removal | `refactor: delete dead compile_one/compile_condition/compile_cost encoder` | DONE `af05a47e` |
| 2 | R2 Remove `arena_allocator` subsystem | removal | `refactor: remove blocked arena_allocator subsystem` | DONE `70eabeb7` |
| 3 | R3 Strip debug/alloc instrumentation | removal | `refactor: drop counting allocator from default builds` | DONE `f46d1908` |
| 4 | R4 Gate ability-path JSON decode (`json_path_test`) | removal | `refactor: gate ability-path json decode behind json_path_test` | DONE `420eae75` |
| 5 | R5 Remove `RESOLVED_ABILITIES` cache | removal | `refactor: remove RESOLVED_ABILITIES cache, decode on demand` | DONE `928ec7a3` |
| 6 | R6 Pools + dep sweep (CondBox, `once_cell`) | removal | `refactor: remove unwired CondBox pool and dead once_cell dep` | DONE `13ce4337` |
| 7+ | O1–O4 squeezing (was B1/B2/B3/C1) | deferred | — | only if measured |

Each phase: **run the parser (`python cards/ability_extraction/extract_card_abilities.py`) → `cargo test` from `engine/` →
commit if green.** This is the **only** verification loop (see TESTING POLICY at top).
The deep-compare guard (`bytecode_deep_matches_json_path`) runs inside the normal
`cargo test` — no extra feature-flag runs, no `cargo check`, no probes, no benchmarks.

---

## 7. What success looks like

After Track A: **zero `serde_json::from_value` in the ability decode path**, zero
intermediate `serde_json::Value` trees during decode, ~600 lines of serde/JSON
infrastructure removed from the hot path, 2,310 lines of dead generated code gone, and the
ability-path serde one feature-gate away from being dropped. **Achieved.**

After Track R: the engine ships **fewer systems, not smarter ones** — **all achieved**:
- Dead opcode encoder gone (R1); blocked `arena_allocator` subsystem gone (R2).
- Counting allocator gone from default builds (R3); `debug_conditions` kept — its fields are load-bearing.
- Ability-path JSON decode gated behind `json_path_test`, off for `ds`/`no_std` (R4).
- Ability cache removed — decode on demand (R5); unwired CondBox + dead `once_cell` deleted (R6).
- Net: smaller binaries, less resident RAM, fewer code paths to maintain — achieved by
  deletion, not by micro-optimization. All green via the parser → `cargo test` loop.

Track O (squeezing) is **deferred**: arena per-turn reset stays blocked (game-state grows
into the bump — measured 2–22 KB live), and the resolver/CondBox/P3 trims are only worth
doing if profiling after Track R shows they move a real number.

---

## 8. Verified bytecode state + remaining serde (2026-08-02)

### The ability decode path is properly bytecode now — no fallbacksVerified in the current tree:

- `get_ability(idx)` (`vm.rs:102`) slices the embedded `BYTECODE` blob by `OFFSETS` and
  decodes **only** via `decode_ability` → `decode_ability_effect_direct` /
  `condition_decoder_gen` — the generated field-dispatch decoders. **No serde, no JSON.**
- The generic `read_value` (serde_json tree) decoder is **gone** (A3); `TAG_OBJECT` /
  `TAG_OBJECT_VARIANT` are handled by the direct decoders.
- No lazy JSON fallback on decode failure: `get_ability` returns `Err(DecodeError)`
  (no fallback to `from_value`).
- `AbilityRef::resolve()` decodes on demand, no cache (R5), no arena (R2).
- The only serde ability decode left (`populate_from_json` / `kind_from_action` /
  `normalize_cost_keys` / `condition_populate_from_json`) is `#[cfg(json_path_test)]` —
  compiled solely for the deep-compare oracle, off for `ds`/`no_std` (R4).

**Verdict: yes — the ability system is properly bytecode, with zero production
fallbacks or bad conversions in the decode path.**

### Remaining serde in production (NOT ability decode)

serde/`serde_json` now ships only for **deck parsing** and the web server, not card loading:

| Site | What it deserializes | Who uses it | Status |
|------|----------------------|-------------|--------|
| `card_loader.rs:39,42` | `Vec<Card>` / `HashMap<String, Card>` from JSON | desktop bins, tests | **GONE** — routed to embedded blob (R8) |
| `card_loader.rs` | all cards from `cards_gen.rs` `CARD_BLOB` | desktop + DS, zero serde | **DONE (R8)** |
| `deck_parser.rs:37,40` | `Vec<Card>` from baked deck JSON | DS deck loading | **remains** |
| `web_server.rs` | API request/response structs | `server` feature only | keeps serde (API contract) |
| `qa_test_suite.rs:2159,2467` | `Choice` structs (test-only) | `qa_test_suite` | fine (test-only) |
| `card_binary.rs` blob | compact `cards.bin` | desktop (R8) + 3DS | **DONE (R8)** |

**R8 — card loading is bytecode/blob now (DONE, `a5848110`).** `compact_card_data` is in
default; `load_cards_from_file`/`load_cards_from_strs` decode all cards from the embedded
`CARD_BLOB` (zero serde), verified by `test_blob_matches_json` over **all 2280 cards**
(field-for-field, including `group` derivation and `Some(0)` score/cost via presence bits
0x08/0x10). Fixes that shipped with it:
- `parse_header`/offset-table reads were `u8` but the format is `u32` (broken blob decoder).
- Blob header compacted: `num_cards` u32→u16, and the u32 offset table (9,124 B) replaced
  with a u8 per-card **length table** (2,280 B) + prefix-sum decode — **~6.8 KB smaller**
  (539 KB → 532 KB). `strtab_len` and string indices stay u16/u32 (string table has 5,675
  strings, max string 687 B).
- Blob `group` now mirrors `map_series_to_group` (multi-line series → empty group), matching
  the JSON deserializer exactly (this was the bring_love multiname regression).
- `score`/`cost` presence flags so `Some(0)` is preserved.

**Remaining (smaller):**
1. **`deck_parser.rs:37,40`** still `serde_json::from_str::<Vec<Card>>` on the baked DS deck
   JSON. Could route through the blob's `resolve_deck_indices`/`find_card_index_by_no`.
2. **`web_server.rs`** keeps serde (it IS the API contract) — out of scope; `server`-only.
3. **`qa_test_suite.rs`** `Choice` from_value is test-only — fine.

**Note on build size:** enabling `compact_card_data` in default embeds the ~532 KB blob
(`cards_gen.rs`, ~3.7 MB source) in every build — the cost of dropping card serde. Tests:
**1934/1934 green**.
