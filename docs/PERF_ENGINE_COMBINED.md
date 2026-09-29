# Engine performance: the one document

> **One document.** The four engine-performance documents this consolidated
> (the incrementalization plan, the baseline plan, the memory/bytecode guide and
> the refactoring approach) were deleted after consolidation; recover any of
> them from commit `7d887201`. Their §6 ("Things already tried, and what they
> proved") is carried forward in §7 below, and it is the most valuable part of
> the pile — it is why several obvious ideas are already closed.
>
> Engine *correctness* work is not here. See `docs/ABILITY_PIPELINE.md` and
> `docs/TEST_AUDIT_PLAN.md` for that, and `engine/tests/TEST_QUALITY.md` for
> the generated test-smell report.

## 0. Read this first: the cheap win is not in this file

Everything in §4 is engine work. It took several sessions and bought −14% of
allocations. The parallelism in §10 buys +17% of *wall clock* for a command
line flag. If you are here to make the benchmark finish sooner, do that first
and do not read the rest of this document.

There is no `[profile.release]` in `engine/Cargo.toml`, so the release build
uses cargo defaults (`lto = false`, `codegen-units = 16`). Adding
`lto = "fat"` + `codegen-units = 1` was measured and **is not worth it**: it
gave +2–4% single-threaded, which is inside the noise of §1, and it appeared to
cost throughput at high `--jobs`. It is deliberately not committed. Re-measure
before revisiting, and never accept a +3% throughput claim from this box (§1).

## 1. The measurement contract, and why wall-clock is not it

Engine speed here is measured on `sim_bench` with the **random** policy, which
is the point: a real policy is tuned against the engine, so improvements to
it can be engine slowdowns wearing a disguise. Random play has no such
coupling.

    cargo build --release --bin sim_bench
    .\target\release\sim_bench.exe --deck "5CP3Z idou" --games 500 --repeat 3 --seed 1 --policy random --jobs 1

**Use allocation counts as the gate, not games per second.** The machine
runs other work in the background, and the spread is far worse than the
±20–25% the earlier plan assumed. Measured on identical binaries, same
session:

| | gps |
| --- | --- |
| idle-ish | 452 |
| loaded | 188–280 |
| 5-round interleaved A/B, same binary | 63, 121, 183, 211, 239 |

A single before/after pair on this box means nothing. Allocation counts have
no such problem: two runs of the same binary gave *exactly* 10,228,233
allocations and 29,648 actions, every time. Build with

    cargo build --release --features "profiling alloc_tracker" --bin sim_bench
    $env:RABUKA_ALLOC_TRACK="1"
    .\target\release\sim_bench.exe --deck "5CP3Z idou" --games 200 --seed 1 --policy random --jobs 1 --alloc

and gate on `alloc calls` plus `total_actions`. The action count is the
trajectory: if it moves, the work being measured is no longer the same work,
and the allocation delta means nothing. Three separate measurements this
session were confounded exactly that way, because other commits landed
mid-run and moved `total_actions` from 29,648 to 29,341.

Threads are a free multiplier for the benchmark and are result-neutral:
`--jobs 1/2/4/6` gives 371/664/750/1559 gps with an identical
`total_actions` at every setting. Do not read that as an engine speedup — a
single game is a strictly sequential turn loop, and there is no intra-game
parallelism to exploit without redesigning for speculative action
evaluation.

## 2. Baseline as of 2026-09-28

200 games, `--seed 1`, random policy, single job. The determinism check is
that `total_actions` reads 29,341 and has not moved between comparison runs.

| | before this work | now |
| --- | --- | --- |
| allocations | 10,813,647 | 9,275,472 (−14.2%) |
| bytes allocated | 868.9 MB | 729.9 MB (−16.0%) |
| share of elapsed in the allocator | 21.5% | 17.7% |

**Stale as of 2026-09-29.** The engine was reorganised
(`cf125b15 refactor(engine): reorganize ability effects into per-kind modules`)
after these numbers were taken. Absolute counts moved for that reason alone and
are not comparable to the above: `total_actions` is now **29,875** and
`alloc calls` **7,024,456**. Re-baseline before quoting any absolute figure
here. The *ratios* — the −14.2% of §4, the 17.7% allocator share — were
measured against their own contemporaneous baseline and still stand.

The allocator share re-measured on 2026-09-29 is **21.8%** at 17.2 ns per
allocate/free pair. That number comes from `alloc_counter`'s sampled
self-timing, which only became usable on that date: the `alloc_tracker` feature
did not compile (`AtomicUsize` returned where `u64` was expected) and the
extrapolation never applied its 1-in-64 sample rate, so it reported 0.4%
instead of 21.8%. Both are fixed.

Depth-1 shape is unchanged by all of the below: `execute_and_settle` 81%,
`generate_possible_actions` 19%. `advance_phase` is about a third of engine
time, and within it `first_performance` and `second_performance` are two of
the top four cost centres. Within `execute_and_settle`, the three worst
leaves by allocations-per-call are `use_ability` (399), `current_ability`
(238) and `resume_with_choice` (148).

## 3. There is no hotspot, and that is the finding

The engine re-derives rather than caches. A ~148-action game re-evaluates
stage hearts, trigger arms and condition verdicts from the whole board
hundreds of times. Every individual derivation is cheap — microseconds — and
there are thousands of them. So aggregate time is flat: the plan's own
conclusion, confirmed, and the reason a profile does not point at one line.

That is the whole reason allocation count is the useful proxy. 17.7% of
elapsed is inside the allocator, at ~30 ns per allocate/free pair, and 49% of
allocations are ≥32 B — a picture of many medium structural copies rather
than one hot allocation site.

## 4. Landed, in order of size

Each of these removes a copy. None of them changes a game outcome, and each
was gated on the allocation count with `total_actions` held fixed.

1. **Gate tables as const fn-pointer arrays.** `pre_cost_gates()`,
   `post_cost_gates()` and `effect_gates()` each returned
   `CompositeGate::new(vec![Box::new(..), ..])`, so every ability
   resolution allocated a `Vec` plus one `Box` per gate and every effect
   execution three more. The lists are compile-time constants. The `#[inline]`
   on the old constructors was inlining the *construction*. −587,291
   allocations.
2. **`AbilityEffect.kind`: Mutex-guarded pool box → `Arc<EffectKind>`.**
   `core::pool::EkBox` had a `Clone` that took two `Mutex` lock/unlock pairs
   and then deep-copied the `EffectKind` into a *fresh* pool slot rather than
   sharing, and `Deref`/`DerefMut` were `unsafe` dereferences of pool
   memory. `Arc` makes the clone a refcount bump; the seven mutation sites
   use `Arc::make_mut`, which is copy-free when the effect is uniquely owned,
   which is the case at every build-time site. `core::pool` is now deleted —
   `make_pool_box!` had exactly one instantiation.
3. **`EffectOwner` accessors borrow instead of returning owned effects.**
   `owning_effect()` and `answering_effect()` returned
   `Option<AbilityEffect>`, so every read deep-copied the effect tree: ~1.7 KB
   across four allocations plus the pool round-trip, on ~25 sites in the
   answer-time choice path. The codebase had already made this exact fix on
   the sibling field — `ResolverSession.current_ability` is an
   `Option<Arc<Ability>>` with a comment recording that an owned value was
   "measured 1.2-2.3 us on every one of ~23k resolutions per 500-game run" —
   and never applied it to `EffectOwner`. −480,908.
4. **Two full-`Card` deep clones on the hot path** — one per stage placement,
   one per live card per performance snapshot. 328 B and up to ten heap
   allocations each, to read fields. −330,241.
5. **Per-call ownership `HashSet`s in `recalculate_constants`.** Two sets
   per call over zones holding ~22 ids, built to answer one question each.
   Now a three-zone `contains` scan: no hashing, no allocation. −78,764.
6. **Skip the condition clone when no patch applies.**
   `can_activate_effect` deep-cloned the whole condition tree to patch two
   things into the copy. Both overlays are now asked about first. Plus a
   one-line fix to a gate that was never closed: `log::buffer_len()` returns
   0 when debug is off, so `buffer_len() <= before` was `0 <= 0` and
   `describe_condition_actual` built a `String` on **every condition
   evaluation** in every non-`no_std` build, discarded by `push_verdict`.

## 5. Still worth doing

**`Option<Box<CompoundBranch>>`.** `compound` is an unconditional `Box`, so
every `AbilityEffect` clone pays a heap allocation for the ~95% of effects
with no compound branch. It is ~60 sites across 15 files *and* a
`serde(flatten)` change on the card-data wire format. `test_blob_matches_json`
deep-compares all 2280 cards against JSON and would catch a wire change, but
one allocation is not worth that diff when the 1.2 KB `EffectFilter` half of
a clone is already gone. **Do it deliberately, not in passing.**

**Closed: the effect tree is no longer where the allocations are.** The
`Condition` boxes became `Arc` in the same shape (`Box<T>` → `Arc<T>` is
wire-transparent, unlike `Option<Box<_>>`, because serde serializes both as
the inner value). That measured **−683 allocations, 0.007%** — a negative
result, recorded so nobody later assumes it was a win. The inference is the
useful part: condition cloning was not on the hot path even before the
overlay check removed most of it, so the ~40 `effect.clone()` sites are not
where `use_ability`'s remaining ~380 allocations per call come from. The
clone path is done; look at what `run_effect` and the action-generation path
*build*, not at what the effect tree copies.

**A dense `Vec` index beside `CardDatabase.cards`.** The card ids are dense
and contiguous (0..2280, assigned by a sequential `next_id`), and
`get_card` is called from 356 sites, each a ~7-line hash probe with one
guaranteed L1 miss. A `Vec<Option<Card>>` makes that an index and makes
zone-order iteration a prefetch stream. Expected win is low single digits,
and the working set is L3-resident either way, so this is a layout
improvement more than a speed one.

**Read it against the constraint in §6.**

## 6. The engine depends on `HashMap` iteration order

Not "might" — does. Swapping `compat::HashMap` from `std::collections`
(SipHash-1-3) to `hashbrown` with an FxHash-style hasher, on the theory
that a cheap hasher is a free win on a dense integer key space, changed
engine behaviour: a full scripted link match went from ~28 s to ~90 s,
because the scripted UI picked different actions and the game path changed.
`loopback_full_match_stays_in_lockstep` **still passed** — it only checks
that the two link sides agree with each other, not against a golden — so the
only symptom was a slow test.

Two consequences, both permanent:

- Any change to `compat::HashMap`, or to code that iterates a map and acts
  on the order, is a *gameplay* change. Gate it on engine outcomes, not on
  tests passing.
- Performance measurements must hold `total_actions` fixed for exactly this
  reason. A change that silently alters the game being simulated produces a
  meaningless allocation delta.

The hasher swap was reverted. It is worth retrying with a properly mixed
`finish()`, but only on a machine where the throughput is measurable, and
only with the gameplay gate in place.

Also closed on measurement, and not to be retried: the allocator is not the
problem (`criterion`/a faster allocator bought nothing); action-list caching
does not help this benchmark, which calls `generate_possible_actions` 1:1 with
`execute_and_settle`; boxing `ActionParameters` is now a **net loss** — it
saves ~0.5% of memcpy on a path that is nothing but memcpy but adds ~22
allocations per call, ~1.5M over a 200-game run, and the plan predates the
allocator being measured at 17.7% of elapsed.

## 7. Already tried, and what it proved

Carried forward from `PERF_INCREMENTALIZATION_PLAN.md` §6. These are closed;
reopening one means re-measuring, not re-arguing.

| Idea | Result |
| --- | --- |
| Ability re-decode at call time | Dead. `OnceLock` per bytecode slot, only 17 `get_ability` calls on the desktop build. Still live on `no_std`, where there is no such cache. |
| State hashing to short-circuit repeats | Dead. Ablation showed repeats are already cheap. |
| 4× trigger scan cost | Measured at 2.7%, not the lever. (Undercounted: the live phase issues more scans than this model assumed.) |
| `recalculate_constants` dominates | No. 4.1% total, and the plan's sequencing advice stands — do not start with the dirty flag. |
| Memory pressure | Dead. No growth while games run. |
| Logging args | Dead, already. The args are not the cost. |
| Faster allocator | Dead. |
| Dirty-flag caching of `recalculate_constants` | Deliberately not first. 51 tests, 4.1% ceiling. |

## 8. RAM

Unchanged from `memory_optimization_combined.md`, and orthogonal to
everything above: the allocator share in §2 is malloc/free *CPU time*, not
memory. All resident card data is ~2.4 MB and per-game churn is a few MB, so
there is no pressure to relieve and more RAM would do nothing. The response to
a high allocator share is to allocate less, which is what §4 did.

The 150 KB north star and the "arena / pools explicitly closed" decision
still stand as written there.

## 9. Parallelism: measured 2026-09-29, and it is the largest single result

This is not engine work. It is the thread count, and it is worth more than
everything in §4 combined. Nothing here changes a game outcome — `total_actions`
was identical at every setting measured.

Machine: Ryzen 5 5600H, **6 physical cores / 12 logical**. Interleaved runs,
1000–20000 games, repeated, single session (so §1's noise rules are respected —
the *ratios* are trustworthy even where absolute gps is not).

| `--jobs` | speedup | gps/thread |
| --- | --- | --- |
| 1 | 1.00x | 427 |
| 2 | **1.81x** | 387 |
| 4 | 3.26x | 348 |
| 6 | 4.63x | 330 |
| 12 | 5.86x | 214 |
| **128** | **6.01x** | 22 |

**Run the benchmark at `--jobs 128`, not `--jobs 12`.** It is +17% over 12
threads, and — more usefully — it cuts run-to-run spread from ~30% to ~8%.
Oversubscribing a 12-thread box by 10x costs nothing here because each game is
6.9 ms and the whole live state is L1-resident. Throughput stays flat from 64 to
1536 threads; it does not collapse.

Three findings that follow, and that are worth more than the ratios:

- **2 jobs = 1.81x, not 2.0x.** A single game already keeps a core ~90% busy,
  so there is only ~10% stall headroom. This closes the "run several games
  interleaved per thread for instruction-level parallelism" idea: the ceiling
  is ~10%, and the OS already gives you that for free via process sharding
  (`for i in {1..8}; do sim_bench --jobs 1 --seed $i & done`). There is no
  intra-game parallelism to exploit.
- **6.0x on 6 physical cores is ~100% of the machine.** The CPU is done. Every
  further gain has to come from making a single action cheaper, i.e. §3.
- **SMT is worth 1.27x** (6→12 threads), which is more than expected for a
  branch-heavy, allocation-bound workload but still nowhere near 2x.

### The one real multithreading cost: `Arc` refcount contention

`GameState.card_database` is an `Arc<CardDatabase>` shared by every worker, and
it is cloned on every card play (`turn::phases::handle_play_member_to_stage`)
plus once per ability resolution. That is an atomic increment on a single cache
line from 12 cores, tens of thousands of times a game. It is invisible at
`--jobs 1` and appears only at the top of the thread range — which is why three
profiling rounds, all of which mandated `--jobs 1`, never saw it.

The card database is read-only for the whole run: `Box::leak` it once at
startup and store a `&'static`, or thread the reference through. Zero
refcount, zero atomics. The same applies to `PlayerId(Arc<str>)`, whose
conversion from `String` traded independent allocations for shared refcount
bumps (a good trade — it removed an allocation per comparison — but it moves
traffic onto the contended line).

### GPU: no, and this is not close

A rules interpreter is close to the worst possible GPU shape: unpredictable
branching (which ability fires, in what order), pointer chasing through
`HashMap`/`SmallVec`/`Arc<str>`, and 7M host allocations per run. The only
theoretical path is batching N independent games into a struct-of-arrays
lockstep layout, which is a rewrite of all 2011 abilities, not an optimisation.
If the training loop runs a neural policy, batch *that* instead — the engine
stays on CPU.

## 10. Where the allocations actually are, and why §4's list is a long tail

The original plan stopped at 4.1%–20% blocks and ranked them. The measured
distribution says that ranking cannot work: **the allocations are not
concentrated anywhere.** Three targeted fixes — all verified against the
3,712-test suite — removed 101,089 of 7,125,545 allocations, i.e. **−1.4%**:

1. `AbilityQueueEntry.id` was **dead**. It was read only by
   `has_entry_with_id`, which is called only from a test, and it cost two
   allocations per enqueue (`format!("{:?}", trigger_type)` plus the id string
   itself). The identity is derivable from `card_no` + `ability_index` +
   `trigger_type`, which the entry already stores. Deleted.
2. `build_ability_queue_entry` rebuilt an identical `String` for `player_id` on
   every enqueue. Now the value is moved through when already canonical, with
   the `"player1" → "p1"` mapping preserved exactly. (Removing the mapping
   outright broke 13 tests — some callers really do pass `"player1"`.)
3. `opponent_id` returned an owned `String`; it now returns a `PlayerId`, which
   is an `Arc<str>` refcount bump. Its doc comment claimed it "runs on phase
   transitions, not per action" — **false**, it runs once per card play and
   once per `execute_performance_phase`.

If the two largest *available* dead allocations were worth 0.7% combined, the
remaining ~6.9M are the long tail: `SmallVec` conversions, `HashSet` → linear
scan, `MemberArea` instead of `String`, interning the ~12 `card_no` literals,
`AreaInfo.area` as an enum instead of `ArcStr`. Each is individually safe and
the set might reach 5–8%, but it is dozens of mechanical edits across rules code
for less than the `--jobs 128` flag. Do it as one deliberate project or not at
all.

## 11. Corrections to the figures in the earlier documents

Recorded because the wrong numbers were load-bearing in the sequencing advice.

- **Allocation counts were attributed to the wrong denominator.** The plan
  divided a whole-process counter — which includes the 256-game warmup — by the
  200 measured games. Real figures at the time of that plan: **~15,600
  allocations per game and ~98 per action**, not "35,628 per game" / "132 per
  action". Bytes churn is ~1.33 MB/game, not 3.2 MB.
- **`recalculate_constants` has 24 call sites, not 28** (in `src/`; 25
  counting the test harness). It is called **exactly twice per card play** —
  `110/game` reconciles to `55 play_member_to_stage calls × 2`.
- **The trigger scan is 4x per play only for baton-touch plays**; a plain play
  does 2. The "4x" figure in §7 is an upper bound, not the norm.
- **The size histogram is 51% of allocations ≤15 B**, not "49% ≥32 B". The
  16–31 B bucket is ~19%, so ≥32 B is ~30%.
- **The 51-test dirty-flag claim is an assertion, not a measurement**, and the
  stated cause is wrong. All six live-state dependencies are already
  materialised as counters or sets with 1–2 write funnels each; the real
  obstacle is that `active_energy_count` is written from ~12 sites with raw
  field assignments, plus a self-referential read of `mods`' own output that
  any version-counter scheme must prove terminates. It is *feasible* and it is
  still not worth it: 4.1% ceiling, and the ceiling did not move.

## 12. Test quality, which is a performance question too

A test that silently answers the wrong prompt is a performance bug in the
measurement apparatus, and the same class of bug bit this work repeatedly:
three separate measurements were confounded because the work being measured
had changed underneath them.

The corpus-wide instance of that, and the fix, are in
`engine/tests/TEST_QUALITY.md` under `drain_zero_may_decline` and in
`TestGame::drain_choices`. The short version: 367 drain loops across 130
files were declining *every* prompt, including the ones
`Choice::allow_skip` says must be answered, which parks the ability
mid-resolution and makes the absence assertion that follows pass for the
wrong reason. Nineteen tests were doing exactly that.
