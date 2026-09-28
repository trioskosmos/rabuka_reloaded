# Engine performance: the one document

> **One document to rule them all.** This supersedes the following, which are
> kept in git for history but are no longer maintained separately:
>
> - `docs/PERF_INCREMENTALIZATION_PLAN.md` — the incrementalization plan and
>   its measured baseline. Its §6 ("Things already tried, and what they
>   proved") is carried forward verbatim in §7 below; it is the most valuable
>   thing in the file and it is why several obvious ideas are closed.
> - `docs/PERF_BASELINE_PLAN.md` — what "baseline" means and how `sim_bench`
>   is built. The measurement contract is carried forward in §2.
> - `docs/memory_optimization_combined.md` — RAM and bytecode. Still accurate
>   for the 150 KB north star and the closed arena/pool decisions; carried
>   forward in §6.
> - `docs/REFACTORING_APPROACH.md` — which remaining items are worth anyone's
>   time, and why. Carried forward in §5 and §6.
>
> Engine *correctness* work is not here. See `docs/ABILITY_PIPELINE.md` and
> `docs/TEST_AUDIT_PLAN.md` for that, and `engine/tests/TEST_QUALITY.md` for
> the generated test-smell report.

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

## 9. Test quality, which is a performance question too

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
