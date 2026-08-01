# Memory × Bytecode × Serde: Unified Roadmap

A synthesis of:
- `engine/MEMORY_REFACTOR.md` — RAM + CPU reduction for resource-constrained targets
- `docs/bytecode_serde_ram_optimization.md` — zero-serde ability decode + bytecode size

**Purpose:** merge both efforts into one north star, re-baseline against the *current, verified*
code state (2026-08-01), and lay out the concrete next phases. Every "done" claim below was
confirmed against the source; every number was re-measured from the current tree.

---

## 1. Verified current state (re-baseline)

### Measured type sizes (from current source, `cargo run --release`)

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
| `vm.rs:374` `read_condition_value` | `Condition` (20 variants, internally tagged) | **BLOCKED ITEM** |
| `vm.rs:886` `decode_ability_cost` | `AbilityEffect` inside `AbilityCost` | **BLOCKED ITEM** |
| `vm.rs:1003` `decode_ability_effect_from_object` | `AbilityEffect` (TAG_OBJECT fallback) | **dead-or-near-dead** |

Only these three calls stand between the engine and a fully direct binary decoder. The
`kind: EffectKind` path is already direct (`decode_ability_effect_direct` +
`effect_decoder_gen.rs`). Phases 7.1–7.2 of the bytecode doc are effectively done; the
doc's remaining 7.3/7.4 map to the three calls above.

### Serde/JSON infrastructure that can be deleted from the hot path

| Symbol | Location | ~Lines |
|--------|----------|--------|
| `populate_from_json` | `vm.rs:1173` | ~115 |
| `condition_populate_from_json` | `vm.rs:1290` | ~54 |
| `collect_json_map` | `vm.rs:912` | ~16 |
| `normalize_cost_keys` + `recursive_normalize_cost_value` | `vm.rs:934-968` | ~35 |
| `decode_ability_effect_from_object` | `vm.rs:976` | ~52 |
| `kind_from_action` (JSON twin of `build_*`) | `card.rs:1031-1376` | ~345 |
| `vm_gen.rs` (dead generated code, **not `include!`d anywhere**) | `src/ability/vm_gen.rs` | **2310** |

> Note: `populate_from_json`, `normalize_cost_keys`, `kind_from_action`, and the
> `Deserialize` derives **must stay compiled** for the deep-compare test
> (`tests/test_modules/bytecode_deep_compare_test.rs`), which is the documented
> bytecode↔JSON equality guard. The goal is to remove them from the *hot path*, not from
> the tree. They can be feature-gated (`ds`/`no_std`) later for the tightest builds.

### Memory refactor state

| Item | Status |
|------|--------|
| Lazy/decode-on-demand abilities (`AbilityRef` = u16) | DONE |
| Compact cards (`compact_cards` + `compact_card_data` blob) | DONE |
| Compact GameState (`compact_state`) | DONE |
| HeartMap → `SmallVec<[(HeartColor, u8); 4]>` | DONE (already u8) |
| Arena v0 (monotonic 64 KB bump, `arena_allocator`) | LIVE |
| Arena v1 (cursor reset) — the **99% alloc win** | **BLOCKED** (game-state grows into the arena; per-turn reset unsafe — see Phase B1). Arena bypass for persistent allocs shipped. |
| `EkBox` pool (128 slots) | LIVE, wired into `AbilityEffect.kind` |
| `CondBox` pool (64 slots) | defined in `pool.rs` but **NOT wired** into `Condition` |
| P3 enums (`card_type`, `orientation`, `zone`) | DONE |
| P3 enums (`operation`, etc.) | PARTIAL (see Phase 6) |

---

## 2. The unified north star

Both documents converge on one principle:

> **The Python compiler already knows the shape of every ability. It should encode that
> shape (variant tags + aliases) into the bytecode so the Rust decoder never has to
> re-derive structure from string keys or `serde_json::Value`.**

- The **bytecode doc** chases this for `EffectKind` (done). It stops there.
- The **memory doc** chases the same spirit for RAM (pools, arena, lazy decode), but its
  arena/CondBox work is *blocked by serde*.

The synthesis: **finish zero-serde first (Track A), then finish the arena/pools (Track B).**
Each phase of Track A removes a reason Track B is blocked. `Condition` and `AbilityCost`
are the last two types that still decode through serde, and they are exactly the types
whose pools/arena wiring is blocked.

### Constraint honored throughout

`cargo test --features bytecode_abilities` must stay green at every step — in particular
`bytecode_deep_matches_json_path`, which guarantees bytecode decode == JSON decode. The
JSON decode path stays available (it is the oracle); only the production hot path goes
direct.

---

## 3. Track A — Finish zero-serde decode (continuation of the bytecode doc)

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

**Test gate:** `cargo test --features bytecode_abilities` (deep-compare proves parity) +
full `run_all` suite.

**Effort/risk:** ~1–2 days. High-touch (20 variants × ~30 fields) but mechanical; the
deep-compare test catches every field miss. This is the largest single piece remaining.

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

**Test gate:** deep-compare + full suite. **Effort/risk:** 2–4 h, Low.

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

**Test gate:** full suite. **Effort/risk:** 1–2 h, Low (after verification).

---

## 4. Track B — Allocation elimination (continuation of the memory doc)

### Phase B1 — Arena v1: cursor reset (the 99% alloc win)

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

### Phase B2 — Wire `CondBox` into `Condition` (unblocked by A1)

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

### Phase B3 — Shrink `AbilityQueueEntry` (2536 B → ~96 B + heap)

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

## 5. Track C — Remaining trims

| Item | Detail | Effort |
|------|--------|--------|
| C1. P3 enum conversions | `EffectKind`/`Condition` fields still `Option<ArcStr>` on closed sets: `operation`, `ability_filter` (has enum but call sites use strings), `placement_order` (done), `distinct` (done). Convert `operation` → enum. Minor. | 2–4 h |
| C2. `debug_conditions` for `no_std` | `text`/`trigger_event` already gated. For `ds`/`no_std`, strip `text` at parser time (memory doc idea) or `--no-default-features` excludes it. Verify the `ds` feature build compiles after Track A (no `serde_json` in hot path). | 2 h |
| C3. Dead `compile_one`/`compile_condition`/`compile_cost` in `compile_abilities.py` | The opcode-based encoder (lines ~401-665) is superseded by `enc_val`. Delete once `decode_condition`-era opcodes are fully gone (after A3 `vm_gen.rs` delete). | 1 h |
| C4. `kind_from_action` + `Deserialize` feature-gating | After A3, gate the JSON oracle behind a `json_path_test` (dev-only) feature so `ds` builds can drop serde from the crate. Doc'd as the endgame "remove serde from no_std". | 1 day |

---

## 6. Suggested execution order

| Order | Phase | Risk | Commit message | Est. |
|-------|-------|------|----------------|------|
| 1 | A2 Cost via compiler alias | Low | `perf: encode cost objects with variant tags, kill serde cost decode` | **DONE** |
| 2 | A3 Delete TAG_OBJECT fallback + dead vm_gen.rs | Low | `refactor: remove TAG_OBJECT effect fallback and dead vm_gen.rs` | **DONE** (06aece4e) |
| 3 | A1 Condition direct decoder | High | `perf: direct condition decoder, remove serde from condition path` | **DONE** (694913d4) |
| 4 | B1 Arena v1 (cursor reset) | Medium | `perf: double-buffer bump arena with per-turn reset` | **BLOCKED** (unsafe; safe subset shipped — bypass + metric) |
| 5 | B3 Box the resolver | Low | `refactor: box AbilityResolver to shrink AbilityQueueEntry 2.5KB→~100B` | 1–2 h — next up |
| 6 | B2 Wire CondBox | Medium | `perf: pool-backed CondBox for decoded conditions` | 0.5–1 d |
| 7 | C1–C4 trims | Low | mixed | 1 d |

Each phase: implement → `cargo test` → `cargo test --features bytecode_abilities` (deep-compare
guard) → commit if green. Measure with `RABUKA_ALLOC_TRACK=1 cargo test -- --nocapture` and
`cargo run --bin size_check`-style probes.

---

## 7. What success looks like

After Track A: **zero `serde_json::from_value` in the ability decode path**, zero
intermediate `serde_json::Value` trees during decode, ~600 lines of serde/JSON
infrastructure removed from the hot path, 2,310 lines of dead generated code gone, and the
`ds`/`no_std` build one feature-gate away from dropping serde entirely. **Achieved.**

After Track B: per-trigger allocs drop from ~1,700 toward the low hundreds (arena absorbs
temporaries, pools recycle `Condition`/`EffectKind`), `AbilityQueueEntry` shrinks ~2.4 KB,
and the arena stops pinning memory between turns. **B1's per-turn reset is blocked** (see
§4) because game-state containers grow into the arena during resolution; the shipped arena
bypass keeps persistent allocations on the system allocator as a prerequisite.

Combined, the two docs' shared goal — a full 800-ability game under the console budget with
bytecode-as-the-effect-source — is reached by finishing the last serde stragglers (A1/A2)
and then completing the allocator story (B1/B2) that serde was blocking.
