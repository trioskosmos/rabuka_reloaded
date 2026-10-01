# Allocation list — ranked optimisation targets

> **Findings only. Someone else does the fixes.**
>
> Source: `RABUKA_TRACE_ALLOC_AT=40` on `sim_bench --deck "5CP3Z idou" --seed 1
> --policy random --jobs 1`, i.e. the per-call-site backtrace tracer in
> `alloc_counter.rs` armed around a single action. Command and raw output are
> reproducible; see §4.
>
> Companion to `docs/ENGINE_AUDIT_FINDINGS.md` (correctness + measurement audit)
> and `docs/PERF_ENGINE_OPTIMIZATION_CATALOG.md` (optimisation plan).

---

## 1. The headline: the documented measurement command is wrong

`docs/PERF_ENGINE_OPTIMIZATION_CATALOG.md` §1 and
`docs/PERF_ENGINE_COMBINED.md` §1 both specify:

```
cargo build --release --features "profiling alloc_tracker" --bin sim_bench
```

**`profiling` makes the profiler the single largest allocator in the engine.**
Measured on the same 200-game run, same seed:

| build | `alloc calls` | vs clean |
| --- | --- | --- |
| `--features "profiling alloc_tracker"` | 5,442,137 | +902,530 |
| `--features "alloc_tracker"` | **4,539,607** | — |

**16.6% of every allocation counted under the documented command is the profiler
measuring itself.** The backtrace trace makes it unambiguous — the top entry in
the polluted build, by a wide margin:

```
95x  9296B  <std::thread::local::LocalKey<…>>::with::
             <<rabuka_engine::timer::Timer as Drop>::drop::{closure#0}>
```

95 of 305 allocations in one action, 31%. `timer.rs:14-19` already documents this
failure mode — *"made the profiler the single largest allocator in the engine —
about 42% of every allocation counted in a `--features profiling,alloc_tracker`
build"* — and fixed the `Vec` key to an `Arc` slice. That reduced it. It did not
eliminate it, and the fix's own docstring says the number should not be trusted.

### Consequences

1. **The recorded baseline of 10,228,233 is not a property of the engine.** At
   4,539,607 for the same workload, the documented figure is roughly double, and
   the excess is instrumentation.
2. **Every allocation delta derived under that command is measured against an
   inflated, instrument-dependent baseline.** This is why three consecutive
   optimisation batches in the previous session measured *exactly* flat
   (5,442,137 to seven significant figures, three times): they were each ~1% of
   a number that is 17% profiler overhead, riding on a baseline that moves
   whenever the profiler's own cost changes.
3. **Fix the command before using it for anything:**
   `--features alloc_tracker` only. `profiling` is for flamegraphs, not for
   allocation counts. The two are not composable for this purpose.

---

## 2. Ranked targets — one action, 212 allocations, 16,527 bytes, 58 call sites

Percentages are of the 212 allocations for this action. This is a single action
mid-game on one deck; treat it as a *shape*, not a universal distribution.

| # | Call site | Allocs | Bytes | % | What it is |
| --- | --- | --- | --- | --- | --- |
| **1** | `ability::vm::get_ability` | **30** | 4,530 | 14.1% | Bytecode → `Ability` decode |
| **2** | `ability_store::AbilityRef::resolve` | **26** | 1,714 | 12.3% | The `Arc` handed to callers |
| 3 | `GameState::trigger_auto_abilities_for_player_with_event` | 19 | 38 | 9.0% | TAS scan (after the `TriggerEvent` fix) |
| 4 | `TurnEngine::player_perform_live` | 15 | 578 | 7.1% | Live-phase performance |
| 5 | `TurnEngine::check_live_success` | 10 | 682 | 4.7% | Live success check |
| 6 | `RawVec<AbilityFilterBranch>::grow_one` | 8 | 1,024 | 3.8% | A `Vec` growing 8× — should be `SmallVec` |
| 7 | `TurnEngine::compute_allocations` | 10 | 384 | 4.7% | Resource/score accumulation |
| 8 | `ability::vm::strip_bytecode_header` | 9 | 432 | 4.2% | Bytecode header parse, per call |
| 9 | `ConditionContext::new` + `evaluate_condition` | 12 | 96 | 5.7% | Condition context build + evaluate |
| 10 | `PlayerId as From<String>>::from` | 6 | 720 | 2.8% | `String` → `Arc<str>` |
| 11 | `core::card::EffectKind::from_action` | 3 | 29 | 1.4% | **the `to_lowercase()`** |
| 12 | `RawVec<MemberContribution>::grow_one` | 2 | 832 | 0.9% | 416 B/alloc — large vectors |
| 13 | `GameState::push_movement_event` | 12 | 23 | 5.7% | Movement log |
| 14 | `GameState::enqueue_printed_ability` | 2 | 17 | 0.9% | **the `format!("{}_{}", …)` identity** |
| 15 | `AbilityResolver::new` | 1 | 4 | 0.5% | The ~1.9 KB resolver box |
| 16 | `hashbrown RawTable<(i16, ModifierEntry)>::reserve_rehash` | 1 | 52 | 0.5% | `heart_modifiers` inner map |

### 2.1 The dominant pair: 26% of the action is ability decoding

`#1` + `#2` = **56 allocations, 6,244 bytes** for a single action. Both are the
ability decode path, and together they are the largest thing in the list.

**The key observation is that `AbilityRef::resolve` should allocate *zero*
times.** On std it is documented as a decode-slot cache read:

```rust
// ability/ability_store.rs:45-50
pub fn resolve(&self) -> Arc<Ability> {
    #[cfg(not(target_has_atomic = "ptr"))]
    { *self.decoded_slots()[self.0 as usize].get().expect("ability bytes not decoded") }
```

That path is an `OnceLock::get` plus an `Arc::clone` — **neither allocates.** An
`Arc::clone` is an atomic increment; only `Arc::new` allocates.

So 26 allocations at ~66 bytes each means **26 `Arc::new` calls, i.e. 26 decode
misses per action.** The cache is not holding. Candidate causes, in order of
likelihood:

- the slot index `self.0` is derived per *call site* rather than per ability, so
  each site has its own slot and the cache is cold per site;
- `decoded_slots()` is re-derived per call and the `OnceLock` chain is not the
  same object each time;
- the `#[cfg]` split means the desktop path is silently taking the `no_std`
  branch, which is `Arc::new(self.decode())` — a full decode and a fresh
  allocation on *every* call.

**This is the single highest-value thing to investigate**, and it is
distinguishable with a one-line counter: count `Arc::new` in `resolve` versus
`OnceLock` hits. If the hit rate is low, the decode cache — recorded as a
completed win in the combined doc — is not actually working on desktop, and
`get_ability`'s 30 allocations are the same bug seen from the other side.

### 2.2 `#3` — the TAS scan, 19 tiny allocations

19 allocations totalling **38 bytes** — so ~2 bytes each, i.e. small `SmallVec`
spills and similar. This is the code I changed this session (`TriggerEvent`
deletion, batch passed as `&[i16]`). The `SmallVec::from_slice` I introduced is
one of them and allocates whenever a batch exceeds 4 cards. Worth a look given
the density: 19 allocations for one scan is high, and the byte count says none
of them are doing real work.

### 2.3 `#6` and `#12` — `Vec`s that should be `SmallVec`

`RawVec<AbilityFilterBranch>::grow_one` firing **8 times for 1,024 bytes**
means a vector started empty and grew repeatedly. `RawVec<MemberContribution>`
allocating **416 bytes per push** means a vector of large elements built by
`push` from empty. Both are the codebase's established `SmallVec` idiom applied
elsewhere but missing here. `MemberContribution` at 416 B/element especially
warrants a `SmallVec<[MemberContribution; N]>` — it is the same pattern as the
zone storage in `core/player.rs`, which is already correct.

### 2.4 `#11` — `EffectKind::from_action`'s `to_lowercase()`, confirmed

3 allocations, 29 bytes. `core/card.rs:1263-1268` starts with
`let a = action.to_lowercase();` before a 60-arm match, on a fieldless enum whose
`to_str()` already returns a canonical `&'static str`. Small, but it is a
confirmed real allocation on a per-action path, and the fix is a match on
`action` with both spellings as arms.

### 2.5 `#14` — the ability identity string, confirmed

`enqueue_printed_ability` allocating twice. This is
`format!("{}_{}", card_no, ability.full_text)` — a `String` built from up to
150 bytes of Japanese ability text, once per enqueue, and the consumer then
parses the index back out of it. Catalogue §9.9; needs a `(card_id,
ability_index)` pair instead, and carries a latent collision bug worth reporting
separately.

### 2.6 `#16` — `heart_modifiers` inner map, confirmed touched

One `reserve_rehash` on `RawTable<(i16, ModifierEntry)>`. This is direct evidence
for catalogue §6.1-A2: the nested `HashMap<HeartColor, ModifierEntry>` is
actively rehashing during play, not just being read. It remains a wire-format
change (§19.9) so it is not a quick win — but this confirms the cost is real
rather than theoretical.

---

## 3. What the trace does *not* show

Worth stating, because it bounds the list:

- **No `Choice::to_frontend_json` / `Choice::serialize` in this action.** The
  polluted trace had a final block with **118 allocations** across those two
  (`62x Choice::serialize` + `56x Choice::to_frontend_json`) — the single
  largest block in that run. It is absent here because this action had no
  choice prompt. **For any action that offers a choice, JSON-serialising the
  `Choice` is ~118 allocations and is likely the top target.** Sample a choice
  action to confirm; it is a different optimisation from everything above.
- **No `generate_possible_actions`.** This trace covers execution only. The
  per-operation profile in `tests/test_modules/perf/alloc_profile.rs` puts
  generation at **378 allocations for 5 actions (~75/action)**, which is a
  different and larger budget than this 212-allocation execution. Both deserve
  attention; they are not the same code.
- **One action, one deck, one policy.** The 18–24-scans-per-turn estimate
  (catalogue §2.1) means trigger-scan cost is very sensitive to which action you
  land on. Arm the tracer at several indices before ranking TAS work.

---

## 4. Reproducing

```
cargo build --release --features alloc_tracker --bin sim_bench

$env:RABUKA_ALLOC_TRACK="1"
$env:RABUKA_TRACE_ALLOC_AT="40"     # action index within game 0
& C:\rust_targets\release\sim_bench.exe --deck "5CP3Z idou" --games 1 --seed 1 --policy random --jobs 1 --alloc
```

Baseline for comparison, no tracer:

```
& C:\rust_targets\release\sim_bench.exe --deck "5CP3Z idou" --games 200 --seed 1 --policy random --jobs 1 --alloc
```

**Note the target directory is `C:\rust_targets`, not `engine\target`** — find it
with `cargo metadata --no-deps --format-version 1 | ConvertFrom-Json |
Select-Object -ExpandProperty target_directory`.

Caveat: `RABUKA_TRACE_ALLOC_AT` captures a backtrace per allocation, so it is
for *attribution* only. Never use a traced build for a count — the tracer is
itself an allocator.

---

## 5. Summary of what to work on, in order

1. **Fix the measurement command** (§1). Everything else is measured against it.
   One-line change, and it invalidates the recorded baseline.
2. **Investigate `AbilityRef::resolve` allocating 26×/action** (§2.1). If the
   decode-slot cache is not hitting on desktop, the single biggest documented
   win in the project is not actually in effect. One counter decides it.
3. **Sample a choice-bearing action** (§3). `Choice` JSON serialisation is ~118
   allocations when it happens — likely the top target for prompt-heavy decks.
4. **`generate_possible_actions`, ~75 allocations per action** — larger per-action
   budget than execution, and untouched by the tracer above.
5. **SmallVec conversions** (§2.3) — mechanical, no design questions.
6. **`EffectKind::from_action`, `enqueue_printed_ability`** (§2.4, §2.5) —
   small, confirmed, easy.
