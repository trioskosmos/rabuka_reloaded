# Engine performance: the one document

> **One document.** This merges `PERF_ENGINE_COMBINED.md` (the measured
> record) and `PERF_ENGINE_OPTIMIZATION_CATALOG.md` (the open-item
> catalogue) on 2026-10-08. The catalogue's per-item analyses — ~60
> items across Categories A–M, each with Where / Cost / Fix / Scale /
> Risk / Gate — are recoverable from git history (the commit before the
> merge); this document keeps the contract, the measured profile, the
> landed record, and the open list, and drops the landed detail.
>
> Engine *correctness* work is not here. See `docs/ABILITY_PIPELINE.md`,
> `docs/TEST_AUDIT_PLAN.md`, and `engine/tests/TEST_QUALITY.md`.

## 1. The measurement contract — read before trusting any number

Engine speed is measured on `sim_bench` with the **random** policy: a
real policy is tuned against the engine, so improvements to it can be
engine slowdowns wearing a disguise. Random play has no such coupling.

    cargo build --release --bin sim_bench
    .\target\release\sim_bench.exe --deck "5CP3Z idou" --games 500 --repeat 3 --seed 1 --policy random --jobs 1

**Gate on allocation counts, not games per second.** The machine runs
other work in the background; the noise floor is **±12% and
time-varying** (same binary, same seed, same `total_actions`: 78,003 ·
78,477 in a quiet period; 72,547 · 64,733 later). Every speed claim
below ~10% on this box is unreliable. Allocation counts are exact: two
runs of the same binary gave *exactly* 10,228,233 allocations and
29,648 actions, every time.

    cargo build --release --features "profiling alloc_tracker" --bin sim_bench
    $env:RABUKA_ALLOC_TRACK="1"
    .\target\release\sim_bench.exe --deck "5CP3Z idou" --games 200 --seed 1 --policy random --jobs 1 --alloc

**`total_actions` is the trajectory gate.** If it moves, the work being
measured is no longer the same work, and the allocation delta means
nothing. Re-baseline after any commit that can change game outcomes.

**The bench IS deterministic across processes.** Two runs of the same
binary with the same arguments produce byte-identical `total_actions`,
and the only iterations of `card_database.cards` are in the bot and
`web_server.rs`, which the random-policy sweep never executes. A plain
two-process before/after is sound *provided `total_actions` matches*.

**`profiling` costs 30–40%** (45.0k aps with it vs ~75k without on
2026-10-08), so `mean_aps` must always be quoted from a non-profiling
build. Each `Timer::start` costs ~0.44 µs — a function whose total is
near that cannot be resolved by per-phase timing; measure the skip rate
or end-to-end `mean_aps` instead. Timer guards **nest**, so a phase
breakdown read off inclusive rows is wrong by construction; the
self-time table in `timer_report.txt` is the one that sums exactly.

**The engine depends on `HashMap` iteration order — not "might", does.**
Swapping `compat::HashMap` to hashbrown/FxHash changed engine behaviour
(a scripted link match went 28 s → 90 s) while
`loopback_full_match_stays_in_lockstep` still passed — that test only
checks the two link sides agree with each other, not against a golden.
Consequences, both permanent:

- Any change to `compat::HashMap`, or to code that iterates a map and
  *acts on the order*, is a **gameplay change**. Gate it on engine
  outcomes (fixed-seed win-rate over a large `--games` count), not on
  tests passing.
- Adding a **side table** or removing an **allocation** is safe.
  Changing a **hash** or an **iteration order** is not.

**Arm-order bias is real.** Whichever arm runs FIRST in a pair pays
cold-cache and first-touch costs; a single on/off pair reported +25%,
+42% and +21% for one unchanged build. `sim_bench --ab <flag>` plays
off, on, off, on inside one process so each arm gets one cold and one
warm run — that collapsed the same change to +3.0% and +1.0%, i.e.
noise.

**Allocation counts are a reproducibility gate, not a ranking signal.**
`Vec::new()` and `HashMap::default()` do not allocate until first push;
three "provable allocation removals" were removing allocations from
paths that make zero. Rank candidates with the `--features profiling`
self-time table instead.

## 2. Where the time actually goes (measured 2026-10-08)

Profile: `sim_bench --features profiling -- --secs 10 --deck aqours_cup`,
random policy, `--jobs 1`. Full report in `engine/timer_report.txt`.

**Shape:** depth-1 self time splits `execute_and_settle` ~75% /
`generate_possible_actions` ~25%. There is no single hotspot — the
largest *exclusive* cost centre is 16.8% of self time, and the profile
is a long tail of microsecond-scale derivations. That flatness is the
finding: the engine re-derives rather than caches, and every individual
derivation is cheap.

**Top self-time rows** (self ms / self µs per call / calls):

| call path | self ms | µs/call | calls |
| --- | --- | --- | --- |
| `generate_possible_actions → generate_main_phase_actions` | **1690.2** | **7.22** | 234,083 |
| `execute_and_settle → exec::resume_with_choice` | 524.2 | 4.55 | 115,331 |
| `execute_and_settle` (own frame) | 483.2 | 0.62 | 785,493 |
| `exec::play_member_to_stage` | 470.4 | 3.84 | 122,621 |
| `advance_phase::live_victory` | 354.5 | 8.45 | 41,935 |
| `recalculate_constants` (via play_member_to_stage) | 316.7 | 2.58 | 122,621 |
| `record_action_boundary` | 280.6 | 0.42 | 669,382 |
| `recalculate_constants` (via live_card_set) | 248.9 | 3.82 | 65,172 |
| `generate_pending_choice_actions` | 233.3 | 2.02 | 115,331 |
| `execute_performance_phase` (first) | 226.1 | 6.26 | 36,129 |
| `generate_live_card_set_actions` | 210.9 | 0.52 | 407,533 |
| `recalculate_constants` (via active) | 199.2 | 2.50 | 79,586 |
| `execute_performance_phase` (second) | 187.1 | 5.18 | 36,129 |
| `exec::use_ability` | 179.6 | 5.63 | 31,877 |

**The #1 sink is `generate_main_phase_actions`: 7.22 µs/call self,
234k calls, 1690 ms — 16.8% of all self time.** Its per-call budget
(itemized 2026-10-08, ~7.4 µs real after subtracting instrumentation):

- **Per-call fixed costs:** `distinct_stage_groups` (was 15 card-resolve
  + group-match lookups; now 3 — landed), 3 `Arc::from(&'static str)`
  for the area names (now a `LazyLock` — landed), `is_action_prohibited`
  (usually empty), the Pass action.
- **Per hand card (~5 avg):** `calculate_play_cost_reduction` (0.15 µs,
  1 alloc), `area_candidates_for` (0.08 µs), `has_double_baton`
  (ability walk), `baton_touch_protected` group tests, and
  `available_areas: Arc::new(Vec<AreaInfo>)` — 2 allocations + 3
  atomic `ArcStr` clones per hand card.
- **Per generated action (up to 3 per hand card):** `make_action_params`
  (~15-field `ActionParameters`), `area_label_en/ja`, and — landed —
  `stage_area_index` was a string parse of "left"/"center"/"right" for
  every action; it is now the slot index.
- **Stage + waitroom ability scan:** 3 slots × abilities with the full
  gate chain, plus a scan of **every waitroom card × every ability** on
  every call, to find the few that activate from discard — now answered
  by a load-time index (§3, 2026-10-08).

The remaining ~4 µs is the enumeration itself — every hand card × every
stage area × every activatable ability with cost and gate evaluation —
which is the job. The function is called ~52× per game, so it is the
#1 hotspot by call count × per-call fixed cost, not because any single
line is slow.

**`recalculate_constants` is NOT a hotspot** (4.1% ceiling, measured).
`check_timing`'s own frame (the eight repair steps) costs **more** than
the recalculation it wraps (6.01 vs 3.52 µs/call), and the repair steps
are hot, not a rarely-taken safety net: skipping all five behind an
ab-flag was measured VOID at 168,347 vs 141,807 actions — they fire
constantly under random play, and 5 tests fail with them off.

## 3. Landed — do not re-propose

Each of these was gated on the allocation count with `total_actions`
held fixed, and (where it could touch a predicate) on a fixed-seed
win-rate comparison.

**Allocation removals (the big ones):** gate tables as `const` fn-pointer
arrays (−587,291 allocs) · `AbilityEffect.kind` as `Arc<EffectKind>`
(pool deleted) · `EffectOwner` **accessors** borrow instead of returning
owned effects (−480,908) · two full-`Card` deep clones off the hot path
(−330,241) · per-call `HashSet`s in `recalculate_constants` replaced by
three-zone `contains` scans (−78,764) · condition clone skipped when no
overlay patch applies · the `describe_condition_actual` dead `String`
build gated behind `log::buffer_len()` · `AbilityQueueEntry.id` deleted ·
`opponent_id` returning `PlayerId` · `normalized_steps` /
`prepare_sequential_action` borrow (together −34,624, ~0.6% — the
per-call-site estimates had predicted ~10× more).

**Category A (work performed unconditionally to feed a disabled
feature) — all landed.** `push_cond_verdict`, `push_effect_verdict`,
`pay_ability_cost`'s `cost_desc`, `rule_log_activated`, the 21 direct
`gs.rule_log` writes, and the display-only accumulators (via the
`DisplaySources<T>` enum, whose no-op `push` sits beside the existing
`ui_text` helper).

**`CardDatabase` load-time indexes** (the §7.2/§15.1 keystone project,
landed): `auto_abilities`, `constant_cards`, `trigger_indices`,
`suppress_cards`, and the dense `live_ids` / `energy_ids` bitsets — all
built once at load by `build_ability_indexes` (one `resolve()` per
ability, shared by the JSON and blob loaders). Card ids are dense from
0, so the bitsets are array reads with no hashing.

**2026-10-08 (this session, `generate_main_phase_actions` fixed costs):**
`distinct_stage_groups` single pass with a matched-bitset (3 card
resolutions instead of 15) · `card_matches_group_str_card` — a
card-resolved matching variant whose `set_card_identity` walk `peek()`s
the ability cache instead of paying an `Arc::clone` atomic per ability
· `INTERNED_AREA_NAMES` as a `LazyLock<[ArcStr; 3]>` static (no_std
keeps the per-call construction) · `stage_area_index` from the loop's
slot index instead of `parse::<MemberArea>()`. Measured on aqours_cup,
`--secs 10`, three runs: 122,835 / 121,840 / 119,701 aps vs a
pre-change mean of 116,650 (**+4.1%**), with `total_actions`
byte-identical before/after (49,237 on the fixed-seed check) and the
outcome histogram unchanged — the offered action sets are identical.

**2026-10-08 (waitroom discard-activation index):**
`generate_main_phase_actions` scanned every waitroom card × every
ability on every call to find the few abilities that activate from
discard. That predicate is a pure function of the printed ability, so
it is now indexed at load: `AbilityIndexes.discard`, built in the same
`build_ability_indexes` pass (shared by the JSON and blob loaders),
stored as `CardDatabase.discard_abilities: HashMap<i16, Vec<u16>>`,
copied by `create_copy`, exposed as `discard_ability_indices(card_id)`.
The waitroom scan probes the index first — most cards carry no
discard-activating ability and skip on one HashMap lookup, without
resolving any ability. Gates: `total_actions` byte-identical before/after
(31,551 on the 200-game fixed-seed check; 984,519 on the 2000-game × 3
sweep), 4354/4354 tests. Single-thread aps unchanged within noise — the
scan was ~1–2% of the function, as estimated.

**2026-10-08 (`--secs` parallelism restored):** `--secs` was added in
`fbc72ae4` with `jobs` forced to 1 ("time-bounding parallel chunks
would need a shared deadline"). Before that commit `--secs` was silently
ignored, so `--secs 10 --jobs 128` fell back to the games path and
parallelized — the regression the jobs sweep exposed. The parallel path
now shares one deadline: workers stride through the game-index space
(worker `j` takes indices `j, j+jobs, j+2*jobs, …`), so every seed
pair stays unique across workers. Games mode is untouched (fixed-chunk
split, deterministic `total_actions`).

**Parallelism (§6) is the largest single result** and is not engine
work: `--jobs 128` buys 6.4×.

## 4. What is left

Statuses are as of 2026-10-08. Several items have landed since the
catalogue was written (2026-09-30) — the in-tree code comments still
reference the old catalogue section numbers (§3.x–§15.x), and those
sections are recoverable from git history. **Verify an item's status
against the source before starting it.**

### 4.1 (landed 2026-10-08) the waitroom discard-activation scan

Landed as a `CardDatabase`-level index — see §3. The
`GameState`-level cache variant (invalidation hooks on the ~25 waitroom
mutation sites) was rejected in favour of the load-time index, which has
no invalidation at all: "does any ability of this card activate from
discard" is a pure function of the printed abilities.

### 4.2 The mechanical batch (Phase 1 — all provably outcome-neutral)

Condensed from the catalogue's Phase 1; items marked ✓ landed. Est.
impact is the catalogue's *estimate* (currency: allocations per action),
not a measurement — the catalogue's own §19.3 correction is that
per-call-site Scale figures were worth ~10× less than estimated.

| item | est. | status |
| --- | --- | --- |
| `waited_stage_cards` → `SmallVec<[i16;3]>` | ~3 | ✓ landed |
| `find_printed_ability` double-resolve | ~1 | open (1 line) |
| `ChoiceRoute` stringification ×2 | ~2 | open (trivial) |
| `requeue_remaining_after_choice` → `into_iter()` | ~32 × steps | open |
| lazy `apply_modify_cost_to_ability_cost` clone | ~32/no-cost ability | open |
| `execute_modify_required_hearts_standard` → slice | 1+N/heart mod | open |
| `post_cost_position_gate` → no `Vec` | 1/resolution | open |
| `ResumeSnapshot::capture` — 10 lookups → 1 | 2/choice answer | open |
| `ResolveSnapshot` / `Zone::from_str` hoists | 0 (branches) | open |
| `process_player_abilities_depth`'s `HashMap` → `SmallVec` | 1–2/drain | open |
| `set_indices_waited/active` buffers | 3/multi-energy payment | open |
| `performer_owned_ids` → three-zone scan | 1/performer | open |
| `check_invalid_live_cards` → `SmallVec` | 2/`check_timing` | open |
| `zone_card_ids`/`_occupied` → iterators | 2/zone query | open |
| `get_zone_cards` and friends → borrow | 1/zone eval | open |
| `counted_by_any().to_vec()` → borrow | 1+N/card/count | open |
| `Active` phase: parse once, `SmallVec` not `HashSet` | 1/Active phase | open |
| `execute_performance_phase`'s `nhm_flat` + partition | 3/performer | open |
| `ArcStr::from(&str)` single-alloc | 1/decode | ✓ landed at the hot site (the area names, via `LazyLock`); other sites unaudited |
| `player_prefix()` → `player_prefix_id()` at 12 sites | 1/log line | open |
| `can_activate_effect` → `Option<&Ability>` pointer token | ~50% of condition evals | open (also Phase 2.7) |
| hoist `condition.common()` | 0 (branches) | open |
| `.count()` → `.any()` for monotone location clauses | 0 (scan length) | open |
| hoist `filter_subset` out of per-card loops | 0 | open |
| `count_side` guard for the weighted second pass | 1 zone filter | open |
| `ConditionContext` only when a condition exists | ~8/card play | open |
| `entry_positions` carries the owner (safe half) | 18 scans/entry | open |
| remaining log-argument bindings (§3.9–§3.12: `hand_cost_matching_indices`, the six condition-layer name lists, the `stats_pipeline` `HashMap` clone, `provide_choice_result`/`SelectionContext` clones) | several | open |
| finish the `recalculate_constants` scratch set | up to 14/call | open, **one field at a time** |

### 4.3 The large items (Phase 2)

| item | est. impact | effort | gate |
| --- | --- | --- | --- |
| **`ability_master_id()` → `Option<&str>`** — 199 call sites × 1 alloc + up to 12 zone scans on the fallback; the largest single allocation count in the catalogue | high | low | `total_actions` + a debug-assert old-vs-new player-resolution diff over one full run |
| **`get_card_names` allocation-free + `normalize_name` borrowing** — 4 allocs × 67 call sites × per-candidate | high | medium | exhaustive corpus equivalence test |
| `store_pending_choice` gating | ~20/choice offer | low | `total_actions` + grep log assertions |
| `filter_subset` / `CardFilter::from_effect` single-bind | 200–300 dependent loads; 10 probes → 1 | medium | `total_actions` + full suite |
| `execute_move_cards` borrow its 8 `String`s | ~9/MoveCards | low | `total_actions` |
| `Option<&Ability>` in `AbilityRef` (the remaining ~50 tight-loop `resolve()` sites) | ~200–300 atomics/action | small | measure at `--jobs 1` **and** `--jobs 128` |
| `EffectOwner` **writes** are deep clones (§5.4) — the accessors were fixed, the writes were not | ~1.7 KB per write | high | needs a new corpus harness; the `resolver.rs:102-119` comment records that this area has produced bugs before |

### 4.4 The deliberate projects (Phase 3 — each needs its own gate and sign-off; do not batch)

1. **Interning the closed vocabulary** (`EffectFilter` is 73% fat
   pointers; ~26 `Box`es under each `Condition`). Try the `no_std`
   version first, where the serde caveat disappears and a 63% reduction
   in `EffectFilter` is the difference between fitting in 1 MB or not;
   then the desktop version behind a hand-written `Serialize`/`Deserialize`.
2. **`Option<Box<CompoundBranch>>` → `Option<…>`** — still deferred
   deliberately: ~60 sites across 15 files *and* a `serde(flatten)`
   wire-format change. Do it only after the clone-site removals (§5.2/5.3,
   landed) have captured their fraction of the benefit.
3. **Replacing the derived-identity string with a `(card, index)`
   pair** — medium-high effort, *can* change outcomes, and carries a
   latent collision bug worth reporting independently.
4. **Stage-scoped caching of `recalculate_constants`** — only the
   *local, no-invalidation* versions (the stage half: `Stage.stage` is
   written through a small enumerable set, unlike `active_energy_count`'
   ~12 raw writers). Leave `active_energy_count` alone: its 4.1%
   ceiling did not move, and that number governs the whole idea.

### 4.5 Build configuration (Phase 4 — all unmeasured on this box; the wall-clock instrument is unreliable here)

`[profile.bench]` (free, and it is a measurement bug) → the three
cross-crate `#[inline]`s → `bench_no_log` (decide the canonical command
explicitly) → `panic = "abort"` (gate on `bot_arena`) → `lto = "thin"`
alone → PGO if the machine ever becomes measurable. **`lto = "fat"` +
`codegen-units = 1` was measured and is not worth it** (+2–4%
single-threaded, inside noise, appeared to cost throughput at high
`--jobs`; deliberately uncommitted).

## 5. Closed — do not retry without re-measuring

| idea | recorded result |
| --- | --- |
| `lto = "fat"` + `codegen-units = 1` | +2–4% single-threaded, inside noise; appeared to cost throughput at high `--jobs`. Deliberately uncommitted. |
| Pooling allocator (`fast_alloc`) | **Landed 2026-10-08: +12%.** jobs=1: 139,760 / 139,077 aps vs 124,386 baseline; jobs=128: 850,719 / 865,023 / 892,168 vs 774,200; `total_actions=328173` byte-identical; 4354/4354 tests pass *with the pool active*. The previous "2.8% slower" measurement was on a pool whose `FreeLists` was `#[derive(Clone, Copy)]` = 310 bytes, and `Cell::get()`/`Cell::set()` copied the whole struct on every alloc and every dealloc — ~1.5 KB per alloc+dealloc pair × 7.3M pairs ≈ 11 GB of copying, which dwarfed whatever the pool saved. The fix is an 8-byte pointer cell (`Cell<*const FreeLists>`) mutating through a raw pointer, with the struct allocated through `System.alloc` at first use so `Box::new` cannot re-enter and stack-overflow on the first allocation. Enable with `--features fast_alloc`. |
| `Condition` `Box<T>` → `Arc<T>` | **Negative**: −683 allocations, −0.007%. The inference is the useful part — condition cloning was never the bottleneck; `EffectFilter` underneath it is. |
| Ability re-decode at call time | Dead on desktop (`OnceLock` per slot; only 17 `get_ability` calls). **Still live on `no_std`** — no cache there. |
| State hashing to short-circuit repeats | Dead. Ablation showed repeats are already cheap. |
| 4× trigger scan cost | Measured 2.7%, "not the lever" — but the model behind the number is wrong by ~an order of magnitude on the live phase (it omits the implicit scan in `process_current_ability` and the 18–24 scans the live phase issues per turn). Do not quote 2.7% × any factor. |
| `recalculate_constants` dominates | No. 4.1% ceiling, and the ceiling did not move. |
| Skip `recalculate_constants` when no 常時 is derivable | **Closed 2026-10-05, built sound over five attempts and lost.** A `CardDatabase` index over printed abilities failed 68 tests (`gain_ability` grants 常時 that never appear in `card.abilities`); the sound five-source version measured 1.8% slower (only 16% of calls skip); a later sound-and-free gate measured +0.2% — noise. The pass is already cheap when there is nothing to derive. |
| 自動 ability index over printed 常時 / empty-queue short-circuit in `process_pending_auto_abilities` | Closed. Both measured losses (23,674 vs 26,402 and 25,908 vs 26,402 aps): a `HashMap` probe per card costs more than iterating `card.abilities` directly once the card turns out to *have* a 自動. |
| Memory pressure | Dead. No growth while games run; all resident card data ~2.4 MB. The allocator share is *CPU time*, not memory. |
| Logging args (the `log` macro) | Dead, already — the macro is lazy. The bugs were the `let` bindings the args depend on (Category A, all landed). |
| Action-list caching | Dead for this benchmark: `generate_possible_actions` is called 1:1 with `execute_and_settle`. |
| Boxing `ActionParameters` | **Net loss**: saves ~0.5% of memcpy, adds ~22 allocations per call (~1.5M per 200-game run). |
| Dirty-flag caching of `recalculate_constants` | 4.1% ceiling; `active_energy_count` is written from ~12 raw sites plus a self-referential read of `mods`' own output that any version-counter scheme must prove terminates. |
| RAM reduction | Not the lever. |
| GPU | No, and not close: unpredictable branching, pointer chasing, 7M host allocations per run. The only theoretical path is batching N independent games into a struct-of-arrays lockstep layout — a rewrite of all 2011 abilities. If the training loop runs a neural policy, batch *that* instead. |

## 6. Parallelism — the largest single result

Not engine work; worth more than everything in §3 combined. Machine:
Ryzen 5 5600H, 6 physical cores / 12 logical. In games mode
`total_actions` is identical at every setting (984,519 on the
2000-game × 3 aqours_cup sweep); in secs mode the played count varies
with machine speed, which is inherent to a wall-clock budget.

Current numbers (`--deck aqours_cup --secs 10 --seed 1 --policy
random`, 2026-10-08, waitroom index + parallel-secs in):

| `--jobs` | gps | aps | speedup |
| --- | --- | --- | --- |
| 1 | 759 | 124,386 | 1.00x |
| 2 | 1,486 | 244,296 | 1.96x |
| 4 | 2,651 | 434,139 | 3.49x |
| 6 | 3,351 | 550,006 | 4.42x |
| 12 | 4,407 | 723,730 | 5.82x |
| 64 | 4,754 | 779,913 | 6.27x |
| **128** | 4,841 | **794,200** | **6.39x** |
| 256 | 4,825 | 793,119 | 6.38x |

**Run the benchmark at `--jobs 128`, not `--jobs 12`** — +10% over 12
threads, and it cuts run-to-run spread. Oversubscribing a 12-thread box
by 10× costs nothing here because each game is ~1.3 ms and the whole
live state is L1-resident; throughput stays flat from 64 to 256
threads.

Three findings worth more than the ratios:

- **2 jobs = 1.96x, not 2.0x.** A single game already keeps a core ~90%
  busy, so there is only ~10% stall headroom. This closes the
  "run several games interleaved per thread for ILP" idea — the ceiling
  is ~10%, and the OS already gives that for free via process sharding.
  There is no intra-game parallelism to exploit.
- **The plateau is ~6.4x on 6 physical cores** (SMT included). The CPU
  is done. Every further gain has to come from making a single action
  cheaper (§2, §4) or from a batch/GPU rewrite (the SoA project in
  §4.3).
- **SMT is worth 1.32x** (6→12 threads: 550k → 724k aps), more than
  expected for a branch-heavy, allocation-bound workload but nowhere
  near 2x.

**The one real multithreading cost: `Arc` refcount contention.**
`GameState.card_database` is an `Arc<CardDatabase>` shared by every
worker, cloned on every card play plus once per ability resolution — an
atomic increment on a single cache line from 12 cores, tens of
thousands of times a game. Invisible at `--jobs 1`, it appears only at
the top of the thread range, which is why three profiling rounds that
all mandated `--jobs 1` never saw it. The card database is read-only
for the whole run: `Box::leak` it once at startup and store a `&'static`,
or thread the reference through. Zero refcount, zero atomics. The same
applies to `PlayerId(Arc<str>)`.

## 7. Checked and found already correct

Recorded so nobody re-audits them. Each was read, not assumed.

| item | verdict |
| --- | --- |
| Zone storage | All inline, zero-allocation in steady state (~292 B/player). Leave alone. |
| Effect dispatch / condition dispatch | Dense-enum jump tables. No fallthrough, no linear scan, no arm-count problem. |
| `recalculate_constants` recursion | `refresh_yell_sources` calls it exactly 2×/play, gated on `restored == restored_all`. Correct shape. |
| `scratch_*` buffer idiom | Correctly applied to the three `exp_*` maps. |
| `Timer` instrumentation | `profiling` off by default; 0 `Instant::now()` outside `#[cfg(feature = "profiling")]`. |
| `push_ability_result` | Correctly gated before any of its ~7 `String`s are built — the counter-example that proves the Category A bugs were bugs, not the codebase's normal state. |
| `zone_for_card` | Returns an owned `String` after 12 scans, but its only caller `push_ability_result` is gated *before* the call. Already correct. |
| `EffectOwner` accessors | `owning_effect` / `answering_effect` already borrow (the *writes* are §4.3). |
| `describe_condition_actual` | Correctly gated; `evaluate_condition_debug` is `serde_support`-gated to the debug endpoint. |
| Gate tables | Landed as `const` fn-pointer arrays, correctly `#[inline]`. |
| `PlayerId` comparisons | `Arc<str>` `PartialEq` is a 2-byte `memcmp`, not a pointer compare. Nothing to win. |
| `ConditionContext` stage-first probe | `Player::contains_card` already probes the 3-slot stage first and short-circuits; the literal fix would be a no-op or a regression. |
| `miniz_oxide` | One `OnceLock` inflate of ~92 KB per process, before timing starts. |
| `describe.rs` | The only semi-hot caller is `choice.rs:2690` on the re-prompt branch. Not a target. |

## 8. Lessons that were load-bearing

Recorded because the wrong numbers were sequencing advice.

- **Arm-order bias** (§1): whichever arm runs first pays cold-cache and
  first-touch costs. A reverted change that leaves its scaffolding
  behind is worse than the change — the `check_timing` batching
  experiment (§3 of the old combined doc) shipped an intermediate state
  that ran an *extra* full validation per settle and was a 36%
  regression before it was caught.
- **Per-call-site `Scale` estimates are the least trustworthy numbers in
  the old catalogue.** A `Box<AbilityEffect>` deep clone really is ~4
  allocations, but the *frequency* of reaching a site dominates, and
  frequency was inferred from source structure rather than counted.
  §5.2/§5.3 predicted ~128 allocations per sequential ability and
  measured ~0.6% total. Get a counter before the refactor, not after.
- **A guard that looks obviously safe can be a gameplay bug.** The
  proposed `TriggerEvent` early-exit guard would have silently suppressed
  real triggers: `trigger_auto_abilities_for_player_with_event` scans the
  board regardless of the event's contents, and a default `TriggerEvent`
  is not a no-op. The identical predicate at `turn/actions/mod.rs:1464`
  guards a *different* call.
- **The repair steps inside `check_timing` are hot, not a safety net**
  (§2). Every empty-input short-circuit aimed at them has nothing to
  skip.
- **Allocation count is a reproducibility gate, not a ranking signal.**
  Three batches of real, test-passing optimisations measured exactly
  5,442,137 alloc calls — twice — because the benchmark does not
  exercise the paths, or the counter cannot see the win. Rank with the
  self-time table.
- **The `total_actions` gate is only valid on a stable tree.** Other
  commits landing mid-run moved it 29,648 → 29,341 and confounded three
  separate measurements.
- **A faster hasher is a gameplay change** (§1), and the fat-LTO
  `--jobs` regression is a code-layout effect (fewer CGUs ⇒ less freedom
  to cluster hot functions ⇒ more L2/L3 contention between workers), not
  a codegen/thread-count interaction — those are a build-time parameter
  and a runtime flag and cannot interact.
