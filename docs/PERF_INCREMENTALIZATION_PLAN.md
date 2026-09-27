# Performance plan: incrementalising the engine

Status: analysis complete, no optimisation landed that I can defend as a win.
Baseline commit: `f5107b71`. Bench: `sim_bench`, `5CP3Z idou`, random policy.

This document exists because the question "why isn't fullspeed random play faster"
has been asked repeatedly and answered wrongly three times (see §6). It records
what was measured, what was ruled out, and what work would actually pay.

---

## 1. How to reproduce every number here

```powershell
cd engine
# timing tree
cargo build --release --features profiling --bin sim_bench
.\target\release\sim_bench.exe --deck "5CP3Z idou" --games 200 --seed 1 --policy random --jobs 1
# full untruncated report -> engine/timer_report.txt
# timings on stderr, allocations per call path, size histogram

# allocation counts + per-region allocation counts
cargo build --release --features "profiling alloc_tracker" --bin sim_bench
$env:RABUKA_ALLOC_TRACK="1"
.\target\release\sim_bench.exe --deck "5CP3Z idou" --games 200 --seed 1 --policy random --jobs 1 --alloc

# throughput
cargo build --release --bin sim_bench
.\target\release\sim_bench.exe --deck "5CP3Z idou" --games 500 --repeat 3 --seed 1 --policy random --jobs 1
```

**Always use `--jobs 1` for profiling.** With more workers the global `TIMERS`
mutex is on the hot path and the numbers are not comparable (this also used to
OOM outright — see §6.1).

### Measurement caveat that invalidates most of this work

This machine runs at ~38% background load (IDE, Discord, a second agent building).
Run-to-run throughput varies **±20%** on a 1000-game run. A paired, interleaved
A/B over 6 iterations still produced a ±25% spread. **Do not accept a throughput
claim from a single run, and do not accept a before/after comparison taken at
different times.** Either interleave A/B in one session or use a deterministic
proxy (allocation counts, cycle counts, timer totals).

---

## 2. The measured baseline

200 games, 148 executed actions/game, `--jobs 1`. Depth-1 instrumented total
**1,380 ms**; ~1,190 ms of that is `execute_and_settle`, ~280 ms is
`generate_possible_actions`.

| depth-1 region | share |
|---|---|
| `execute_and_settle` | 79.8% |
| `generate_possible_actions` | 20.2% |

Inside `execute_and_settle`, `advance_phase` is **~33% of all engine time**
(measured across both call parents — it is under-reported if you only look at
one):

| `advance_phase` arm | calls | total | avg |
|---|---|---|---|
| `first_performance` | 3,017 | 98.8 ms | 32.7 µs |
| `live_card_set` | 3,017 | 83.7 ms | 27.7 µs |
| `second_performance` | 3,017 | 78.3 ms | 25.9 µs |
| `live_victory` | 3,088 | 69.8 ms | 22.6 µs |
| `active` | 6,812 | 68.9 ms | 10.1 µs |
| `draw` | 6,812 | 45.4 ms | 6.7 µs |
| `energy` | 8,582 | 6.2 ms | 0.7 µs |

`execute_main_phase_action` (~900 ms) is almost entirely three ability leaves:

| leaf | calls | avg | allocs/call |
|---|---|---|---|
| `exec::play_member_to_stage` | 11,016 | 30.8 µs | 162 |
| `exec::resume_with_choice` | 11,222 | 25.4 µs | 149 |
| `exec::use_ability` | 2,718 | **62.7 µs** | 387 |

**Allocation volume:** 7,125,545 allocations per 200 games = **35,628/game**,
3.2 MB churned per game, **~132 allocations per action**.

**Allocation size distribution — the key clue:**

```
1-7B   : 1,406,466  (20%)
8-15B  : 2,195,483  (31%)     <-- together, 51% are <=15 bytes
16-31B : 1,336,372  (19%)
32-63B :   785,425
64-127B:   747,894
128-255:   182,495
512-1K :   205,843
```

Half of all allocations are 15 bytes or smaller. That is not data structures.
That is `String` churn on values like `"p1"`, `"stage"`, `"self"`, and small
integers being formatted into strings.

---

## 3. The actual answer: no hotspot, and that is the problem

There is no single dominant cost. The largest block is 20%. That is why three
rounds of micro-optimisation produced nothing measurable.

The structural reason: **the engine re-derives everything from scratch on every
action, ~150 times per game.** Each action pays for:

1. A full rebuild of the legal-action list (20.2% of engine time).
2. Four full board scans for auto-triggers (`trig::auto_scan`, 4x per card play).
3. `recalculate_constants` — **22,032 calls per 200 games ≈ 110 per game**,
   from **28 separate call sites**, at 2.59 µs each.
4. Re-evaluation of every ability condition from live state.
5. Re-execution of the performance phases' heart/blade/yell bookkeeping.

The board is tiny — 3 stage slots, ~7-card hand — so each of these is cheap in
isolation. There is no hot loop. The cost is that none of it is incremental.

### Is the allocator the limit? Probably not.

~132 allocations per action at a typical Windows alloc+free cost of 25–30 ns is
**~4 µs of the ~18.7 µs per action — roughly 20%**. That is an *estimate from
arithmetic*, not a measurement: the direct experiment (sampling the allocator's
own time inside the `CountingAllocator` hook) is implemented in
`alloc_counter.rs` but has not produced a clean run yet, and mimalloc could not
be built here (`vcruntime.h` is not installed — the MSVC C++ workload is
incomplete). **Treat "the allocator is 20%" as unconfirmed.** Real, but not the
wall.

---

## 4. Work items, ranked by measured size

### 4.1 Performance phases — 177 ms (12.8%), 121–136 allocs/call
`execute_performance_phase`, 3,017 x 2 calls per 200 games (30/game), 32.7 µs
and 136 allocations each. The densest non-resolution region in the engine and
completely unexplored.

Concrete known waste at `turn/phases.rs:321-356`:
- `nhm_flat` is `collect()`ed into a heap `Vec` on every call.
- `player_id` is `String`-cloned **twice** (lines 336/338 and 356) purely to
  satisfy borrows. My `PlayerId` change already makes these refcount bumps, but
  the double-clone structure should still go — take one `&PlayerId` and pass it.
- `cannot_live_players.contains(&player_id)` and `seat_index_by_id(&player_id)`
  both take `&str` and force an owned copy.

First step is instrumentation, not optimisation: subdivide
`execute_performance_phase` the same way `resolve_ability` was subdivided, then
attack whatever dominates. Do not guess here.

### 4.2 Action-list generation — 20.2%, 31.8 allocs/call
`generate_possible_actions` → `generate_main_phase_actions` (5.2 µs, ~22
`Action`/`ActionParameters` structs per call).

The decision logic is **free**: `calculate_play_cost_reduction` is 0.16 µs and
`area_candidates_for` is 0.06 µs per hand card. All 5.2 µs is construction.

Two concrete, mechanical options (not yet tried):
- **`Box<ActionParameters>`.** `ActionParameters` is ~180 bytes inline, so
  `Action` is ~240 bytes. `Vec<Action>` of 22 is ~5 KB built, moved and dropped
  per call. Boxing the parameters shrinks `Action` to ~64 bytes: ~4x less
  memcpy on a path that is nothing but memcpy. Read sites keep working via
  deref coercion; only construction sites change.
- **Hoist the per-hand-card `Arc<Vec<AreaInfo>>`.** One `Arc` + one `Vec` per
  hand card (14 allocs/call for a 7-card hand). Whether the areas can be shared
  across cards depends on how much `effective_cost` varies per card — measure
  first.

**Caching the action list between actions will not help this benchmark.** The
bench calls `generate_possible_actions` 1:1 with `execute_and_settle`, and every
action mutates the hand/stage/energy, so the list genuinely changes every time.
It would help the bot (which re-queries) but not the training loop. I proposed
this earlier; on re-examination it does not pay here.

### 4.3 `recalculate_constants` dirty flag — 57 ms (4.1%), 110 calls/game
Called from 28 sites; 2.59 µs each; every call re-walks all stage abilities,
re-decodes (cached) and rebuilds `ConditionContext` + score sources.

Phase 1 explicitly deferred this: *"conditions read live state (energy, zones)
that a naive version counter misses (51 tests)."* That is a test-design problem,
not an engine problem. The workable shape:
- A `state_generation: Cell<u64>` bumped by the zone-mutating helpers.
- Recompute when the generation moved since the last call **or** when any
  `mods` bucket was touched.
- The 51 tests are the gate: they exercise the live-read behaviour, so they
  become the specification of what must invalidate. Expect to add explicit
  invalidation to some effect handlers.

### 4.4 Trigger-scan coalescing — 52 ms, 4 scans per play
`trig::auto_scan` is cheap per call (0.84–1.09 µs) but runs 4x per card play
(self+opponent, before and after placement). Two of the four are adjacent with
no intervening mutation in some paths — worth checking whether they can be
coalesced rather than cached.

### 4.5 Remove remaining 2-byte `String` churn
`Player.id` is done (`PlayerId(Arc<str>)`, §6.3). The same pattern still exists
for:
- `AbilityQueueEntry.player_id: String` — copied on every enqueue.
- `GameState.deck_emptied_by_effect`, `pending_success_replacement_player_id`,
  `ReplacementEffect.player_id` — all `Option<String>` holding a player id.
- `gs.player_prefix()` — still returns `String`; a `_id()` variant returning
  `&PlayerId` exists and should be used where the result is only compared.

Each is worth ~1 allocation per occurrence per resolution.

### 4.6 Condition evaluation string interning
Conditions compare against `&str` literals (`"stage"`, `"self"`, zone names).
`Zone` is already an enum; the string comparisons around it are pure overhead.
This is the largest remaining unknown — the condition evaluator has not been
sub-instrumented, so its share is unmeasured.

---

## 5. Sequencing

1. **Sub-instrument `execute_performance_phase`** (§4.1). Biggest unexplored
   block; cheap to do with the tooling that now exists.
2. **`Box<ActionParameters>`** (§4.2). Mechanical, compiler-checked, no
   semantic risk. Measure with allocation counts, which are deterministic.
3. **`recalculate_constants` dirty flag** (§4.3). Highest risk, most tests
   affected — needs its own branch and a full-suite gate.
4. Remaining `String` churn (§4.5), then condition interning (§4.6).

Do not start at 3 before 1 and 2 are done — 3 is the one that can consume a
week, and it only pays 4%.

---

## 6. Things already tried, and what they proved

Recording these so they are not re-litigated.

### 6.1 `timer.rs` global `CALL_STACK` — **real bug, fixed**
`engine/src/timer.rs` held the label stack as one global `Mutex<Vec<&str>>`. With
2+ workers the interleaved push/pop mints a call-path key neither thread ever
took, so `TIMERS` grows combinatorially: **2 workers → 3.9 GB → OOM abort; 12
workers → instant death.** Now `thread_local!`. Verified 12 workers = 23.3 MB.
This also explains why no one had ever profiled a multi-worker run.

Related: the `%` column summed *inclusive* rows to ~300%, and `sim_bench` never
called `print_results` at all — so the canonical production-path harness could
not emit a profile. Both repo benchmarks (`profile_target`,
`benches/performance.rs`) additionally call `TurnEngine::execute_main_phase_action`
directly, bypassing `record_action_boundary`, so they measure a path production
never takes.

### 6.2 `push_ability_result` gate — correct, negligible
Built a card-name `String`, a `canonical_trigger()` `String`, a full ability-text
clone and six more `String`s inside `LogMetadata`, then checked
`logging_enabled()` at the *bottom* and discarded all of it. Gate moved to the
top. **−0.7% of allocations** — it is on the failure path, not the hot one.

### 6.3 `Player.id` → `PlayerId(Arc<str>)` — large change, small win
109 compile errors → 0, all bins and tests build, **3712 tests pass**.
Allocations **7,295,325 → 7,125,545 = −2.3%**. Serde output is byte-identical
(the `rc` feature is enabled). A newtype was required because std has no
`Arc<str> == &str` impl, so all 75+ comparison sites would otherwise break.

Two of those collections had to stay `String`: `cannot_activate_members` and
`constant_cannot_activate_members` hold *either* a player id *or* a card-id
string and are discriminated by `parse::<i16>()`. Only `cannot_live_players` is
player-ids-only and became `Vec<PlayerId>`.

`trigger_auto_ability` / `trigger_auto_ability_by_index` / `add_replacement_effect`
now take `impl Into<PlayerId>` rather than `String`, so callers pass a shared id
instead of allocating a throwaway copy — this is the pattern to follow for the
remaining `String` player-id fields (§4.5).

### 6.4 Hypotheses killed by measurement — do not revisit
| Hypothesis | Measurement | Verdict |
|---|---|---|
| Ability re-decode per resolution is the cost | `vm::get_ability` fires **17 times** in a whole run; `ability_store` has a per-slot `OnceLock<Arc<Ability>>` | dead — ~0% |
| The byte-at-a-time full-board state hash | `record_action_boundary` 0.42 µs x ~93k calls | 2% |
| The 4x-per-play trigger scan is O(board) expensive | 0.87–1.09 µs per scan | 2.7% |
| `recalculate_constants` (the missing dirty flag) dominates | 2.0–2.8 µs across 28 sites | 4.1% |
| Memory pressure | 19.8 MB flat from 1 to 12 workers | dead |
| `log::trace!!` args evaluate when the logger is uninitialised | `log`'s `MAX_LOG_LEVEL_FILTER` defaults to `Off` (0) | dead — args are skipped |
| A faster allocator is a big win | mimalloc cannot build here (`vcruntime.h` missing) | unmeasured |

### 6.5 Parallelism is not the bottleneck
433.9 gps at 1 worker → **2200.3 gps at 12 workers = 5.07x**, which is ~72% of
what 6 physical cores can deliver (SMT gives ~1.3x, not 2x). Single-threaded
throughput has roughly doubled since the 221 gps figure in
`docs/PERF_BASELINE_PLAN.md`. Memory is flat at ~20 MB across all worker counts.

---

## 7. Tooling added (all in this change, uncommitted)

`sim_bench` can now actually profile itself, which it never could before:

- `timer.rs`: per-thread call-path stack; depth-1 share column with the
  inclusive-double-count caveat printed; **per-call-path allocation counts**;
  full untruncated report written to `engine/timer_report.txt`.
- `alloc_counter.rs`: public `alloc_count()` / `alloc_buckets()` so regions can
  be attributed; sampled allocator self-timing (implemented, not yet producing
  a clean reading).
- `game_setup.rs`, `bin_common.rs`, `ability/vm.rs`, `core/game_state/abilities.rs`,
  `ability/gates.rs`, `ability/resolver.rs`, `ability/effects/mod.rs`,
  `turn/phases.rs`: `Timer::start` probes down the ability-execution path.
  All are `#[cfg]`-gated and cost nothing when `profiling` is off.

**The instrumented regions are the map for §4.** Keep them; they are how the next
person finds the next thing.
