# Engine audit — findings

> **Read-only audit. No code changes in this document.**
>
> Produced 2026-10-01 by manual inspection plus a purpose-built per-operation
> allocation profile. Companion to `docs/PERF_ENGINE_OPTIMIZATION_CATALOG.md`,
> which is the optimisation plan; this is the audit of *what was actually true*,
> including where that plan was wrong.
>
> **Gate used throughout:** full suite green (4,072 tests at time of writing)
> and `total_actions` fixed. See §5 for why the second half of that gate is
> currently unsound on this repo.

---

## 0. Headline findings

| # | Finding | Severity |
| --- | --- | --- |
| **F1** | The allocation counter was never broken. Three of my "provable allocation removals" removed allocations from paths that **make zero allocations**. | **High** — invalidates prior reasoning |
| **F2** | `recalculate_constants` allocates **0** per call. It is called twice per action. | High |
| **F3** | `compute_play_cost` allocates **0**, and is **independent of hand size**. §4.2's "worst quadratic in the generation path" is not a measurable cost. | High |
| **F4** | `generate_possible_actions` allocates **378 for 5 actions (~75/action)**. This is the only hot path with real allocation volume, and it is where all remaining effort belongs. | High |
| **F5** | The tree does not currently compile. `ability/queue.rs` is missing `choice_repeats_remaining` in an `AbilityQueueEntry` initializer — **not my change**; it is in-flight work by another session. | **Blocking** |
| **F6** | `recently_moved_from_zone` allocates a `String` on **every card movement** and, since the `TriggerEvent` deletion, is read only by the display layer and one save/restore. | Medium |
| **F7** | The original `prepare_sequential_action` deep-cloned a ~1.2 KB filter to set `None` over `None`. Confirmed dead work, now removed. | Low (fixed) |
| **F8** | The repo's stated measurement contract is unsound while other work lands concurrently on the same branch. | **High** (process) |
| **T1** | Inverted guard in `revealed_hand_aqours_pl_s_sd1_009_sd_test.rs:98` — the failure path returns early, so a missing prompt passes. | **High** |
| **T2** | `n_pb1_015_shizuku_test.rs:61` — its only assertion restates a precondition the test set up, so it cannot fail; the real intent is unchecked. | **High** |
| **T3** | 23 sites discard the `Result` of a `try_*` API. 21 are correct and documented; the systematic gap is that no test can tell a correct decline from a silent `Err`. | Medium |

---

## 1. The measurement question, settled

The catalogue's §19.11 asked which of two things was true:

1. the benchmark deck doesn't exercise the hot paths, or
2. `alloc calls` is not sensitive to allocator reuse.

**Neither. The counter was fine, and the paths allocate nothing.**

A per-operation profile on a deliberately-representative board (stage occupied
both sides, energy paid, hand populated, discard populated) gives:

| operation | allocations |
| --- | --- |
| `recalculate_constants` | **0** |
| `compute_play_cost` (one hand card) | **0** |
| `compute_play_cost` after adding 10 cards to hand | **0** |
| `auto_advance_one` | 1 |
| `generate_possible_actions` | **378** (5 actions) |

For contrast, the standing whole-game figure is **5,442,137** across 200 games
and 29,521 actions — about 27 allocations per action on average.

### What this means

- **F2 / F3.** The paths I spent three batches optimising are not allocation
  sinks. `recalculate_constants` re-derives maps into preallocated scratch
  buffers and, once the display accumulators are gated, allocates nothing at
  all. `compute_play_cost` allocates nothing regardless of hand size, which
  also falsifies the "worst quadratic" framing — whatever it costs, it costs in
  dependent loads and `Arc` traffic, not in the heap.
- **F4.** `generate_possible_actions` at ~75 allocations per generated action
  is the whole story. Everything else is rounding error.
- **The instrumentation now exists and is part of the suite** —
  `tests/test_modules/perf/alloc_profile.rs`, three tests, gated on
  `--features alloc_tracker` so a bare `cargo test --test run_all` is
  unaffected. It prints the table above and asserts per-operation ceilings.
  `build.rs` gained an `engine_alloc_tracker` cfg because a `tests/*.rs`
  integration test is its own crate and cannot see the library's features.

### Corrections this forces on the catalogue

| Catalogue item | Claim | Reality |
| --- | --- | --- |
| §3.7 display accumulators | "~14 allocations per `recalculate_constants`" | Gate is correct, but the count was already 0 in practice |
| §3.8 finish scratch buffers | "up to 14 allocations per call" | **Wrong premise.** There is nothing left to save |
| §4.2 cost-reduction hoist | "worst quadratic in the generation path" | Zero allocations, hand-size independent |
| §7.10 `.count()` → `.any()` | "up to 30–64× shorter scans" | Plausible on time, invisible to allocations |
| §19.3 "`PreparedStep::Borrowed` rarely fires" | — | Confirmed, but for a different reason than guessed |

---

## 2. Audit of this session's changes

Twenty-five changes across fifteen files. Each was re-read against the
pre-change behaviour rather than against my own description of it.

### 2.1 `sequential_step_needs_patch` vs the original `prepare_sequential_action` — **faithful**

The guard is a disjunction of the original's four independent `if` blocks. Each
predicate matches, using `action.X` where the original used
`action_to_execute.X` — identical, because the only mutation performed before
those reads is `condition = None`, which none of them inspect.

Two details worth recording:

- The original's `distinct` inheritance is guarded by
  `action.action == ModifyRequiredHearts && action_to_execute.distinct_any().is_none()`
  **and then** `if let Some(d) = effect.distinct_any()`. My guard folds that
  second half into `&& effect.distinct_any().is_some()`. Same condition.
- **`F7` — a real redundancy in the original.** The `self_target` block ends with
  `else if i > 0 && supports_self { set_self_target(first.self_target_any()) }`
  with **no `is_some()` check**. When `first` has no `self_target`, this calls
  `set_self_target(None)` on a value that is already `None` — and `set_*` goes
  through `Arc::make_mut`, which deep-clones the ~1.2 KB `EffectFilter` to
  perform a no-op assignment. My guard skips that case. Result is identical;
  one deep clone removed.

### 2.2 `execute_sequential_effect` taking `steps` as a slice — **sound**

The justification was that `setup_sequential` and `prepare_sequential_action`
never read `compound.actions`, `effect_steps`, or `action` off the parent. Read
and confirmed: they read `conditional`, `is_further`, `condition`, `action`
(log only), and the per-unit / self-target / card-names fields. The synthetic
clone changed exactly three fields, none of which any of them consult.
`action_for_log` preserves the `[SEQUENCE]` debug line, which previously printed
the forced `Sequential`.

### 2.3 `common()` hoist in `evaluate_condition`'s negation guard — **equivalent**

Verified against the accessors:
- `get_card_property()` is `self.common().and_then(|c| c.card_property)`;
  `common().and_then(|c| c.card_property.as_ref()).is_some()` is equivalent.
- `get_location()` is `self.common().and_then(|c| c.location.as_deref())` —
  byte-identical to the hoisted form.
- `get_heart_type()` and `get_ability_filter()` are **variant-specific**, not
  routed through `common()`. These were correctly left on their accessors.

### 2.4 `TriggerEvent` deletion — **safe, and it exposed F6**

Exhaustive search confirms no surviving reference to any deleted field. What
remains under those names are the `GameState` fields, which are a different
thing entirely.

**F6 follows from this.** `recently_moved_from_zone` is assigned a fresh
`String` on every movement (`modifiers.rs:1927`, `:1975`) and cleared at
`:1982`. Before the deletion its only consumer inside the engine was the
`TriggerEvent` field, which was unread. Its remaining readers are
`game/display.rs:1938` and a save/restore at `turn/actions/mod.rs:1278-1283`.
So a per-movement `String` allocation now exists purely for a UI projection.

Worth a look, but note it is **display-gated in the benchmark** the same way
§3.7 was — which per F2 is exactly the case where the measured saving was zero.

### 2.5 `DisplaySources` gate — **sound, no effect**

Sound: all three accumulators are read only by `game/display.rs`. The no-op
`push` design means the nine call sites were untouched, which is the right
trade.

But per **F2** the thing it gates was not allocating on this path. The change
is still correct and still worth having — it makes the gate explicit rather
than accidental, and the docstring now records the invariant. It just is not a
performance win, and I presented it as one.

### 2.6 `card_name_matches_group` — **verified by construction and by test**

The fast path's precondition is "no whitespace and no full-width `＆`", under
which `get_card_names` provably reduces to `name.split('&')`. This is pinned by
three new tests in
`tests/test_modules/rules/group_name_equivalence.rs` that sweep every card in
the corpus against every known group plus fold/multi-name needles, and assert
the fast path agrees with the allocating path. The third test asserts the corpus
still contains multi-name cards, so the first two cannot go vacuous.

This is the one change in the session with a genuine correctness argument *and*
a test that enforces it.

### 2.7 `peek()` adoption — **two dangling references found and fixed**

`&*ar.resolve()` produces a reference into a temporary `Arc` that dies at the
end of the enclosing statement. I introduced this twice (`modifiers.rs` and
`abilities.rs`) and caught both by inspection before testing. Both now compute
the value rather than binding a reference.

**Residual risk:** the pattern is still the easy mistake, and it is UB rather
than a compile error. A `Peek` type that returns a value, or a `with_peek`
closure, would make it structurally unrepresentable. Not done — flagged.

### 2.8 `monotone_exists` fast path — **correct, with one observable difference**

`compare_counts(Some(">="), n, 1)` reduces to `n >= 1` ≡ `n > 0` ≡ `any`, so the
fast path is equivalent for the verdict. The one difference is the `actual`
value recorded in the debug verdict, which becomes 0/1 instead of the true count.
It is gated on `!debug_enabled()`, so debug sessions are unaffected. Correct as
written; the gate is the load-bearing part and should not be removed.

### 2.9 `requeue_remaining_after_choice` — **correct after a fix**

The first version stripped conditions from the already-queued actions instead of
only the newly-appended remainder — a real behaviour change. Corrected to slice
`queued[appended_from..]`. Worth noting because it is the class of bug this
operation is exposed to: the ordering of append-then-strip versus
clone-strip-append is not locally obvious.

### 2.10 `ability_master_id` → `Option<&str>` — **mechanical, verified**

22 call sites migrated from `.as_deref()`. `target_player_index` and
`target_player_label` already took `Option<&str>`, so they were unchanged. The
saved `String` was never observed — both arms of every `match` keyed only on
whether the value equals `"player2"`/`"p2"`.

### 2.11 `group_counts` `HashMap<&str, _>` — **correct**

`card.group` is an `ArcStr` in the database and the map is discarded at the end
of the block, so the borrow outlives it. Removes a `String` plus a SipHash per
hand card.

### 2.12 Summary

No outstanding correctness defect found in this session's changes. Three
classes of error were introduced and caught during the work (a behaviour change,
two dangling references, one brace imbalance) — all caught before commit, but
the dangling-reference pair is the concerning one because it is UB that the
test suite would not necessarily have caught and the compiler did not.

---

## 3. Where the cost actually is

Re-derived from the profile rather than from the catalogue.

**Allocation-bound, and it is one function.** `generate_possible_actions` at
~75 allocations per generated action. Everything else in the catalogue is
single-digit per call. Any further optimisation effort that is not aimed here,
or at the allocation *classes* it produces, is not aimed at anything.

**Not allocation-bound, therefore invisible to the gate, and possibly still
real:**
- `AbilityRef::resolve()` atomics (§6.5) — `generate_possible_actions` walks
  card abilities per hand card; those atomics are on the same path.
- `CardFilter::matches` dependent loads (§6.12) — same path, now 1 probe instead
  of 10, but the probe is still a hash plus a guaranteed L1 miss.
- Condition evaluation dispatch (§8.1) — a 21-arm match per accessor call.

These are the items the instrument cannot confirm and cannot refute. They are
not thereby worthless; they are unmeasured, and this audit declines to rank
them.

---

## 4. Test-corpus audit

997 files, 4,092 `#[test]` functions, 4,065 parsed structurally. The corpus has
clearly been through a cleanup pass: the four `assert!(true)` and three
`#[ignore]` occurrences are all inside doc comments *describing* that cleanup.
The remaining flaws are subtler, and are about vacuous passing.

### 4.1 What is clean

| Probe | Result |
| --- | --- |
| Tests whose body contains no `assert`/`panic!` | 35 — **all false positives** |
| …of those, that also call no helper defined in the same file | **0** |
| `assert!(true)` in live code | 0 |
| `#[ignore]` in live code | 0 |

The 35 are parameterised tests delegating to a shared asserting helper — e.g.
`debut_deficit_mill_recent_live_topdeck_test.rs` has five `#[test]` fns each
calling `deficit_mill(count, mill_live, take)`, which carries ten assertions. The
naive detector flags all 35; the version that also checks for a local helper
call returns **zero**. Recorded because the naive version produces 35 false
positives that could easily have been reported as 35 defects.

### 4.2 T1 — inverted guard: failure path returns early

`revealed_hand_aqours_pl_s_sd1_009_sd_test.rs:98`

```rust
if !game.has_pending_choice() {
    assert!(game.state.player1.hand.cards.contains(&only_mu));
    return;                                   // test PASSES
}
game.select_indices(&[]);
assert!(game.state.player1.hand.cards.contains(&only_mu));
assert_eq!(game.state.player1.main_deck.cards.len(), deck_before,
           "declined -> deck untouched");
```

The ability under test is a live-start that must *offer a decline-able choice*.
If it stops offering one, the test takes the early branch, asserts one weak
thing, and **reports success** — while every substantive assertion, including
`"declined -> deck untouched"`, never runs. The branch that ought to be the
failure is the one that returns early. The fix is to assert the prompt exists
and let its absence fail the test.

### 4.3 T2 — assertion that cannot fail

`n_pb1_015_shizuku_test.rs:61`

```rust
game.state.player1.hand.cards.push(shizuku);
game.state.player1.hand.cards.push(filler);          // filler now in hand
…
let mut safety = 0;
while game.has_pending_choice() && safety < 5 {
    safety += 1;
    let _ = game.try_select_indices(&[]);
    let _ = game.try_select_indices(&[0]);
}
// Hand should still have filler, no Shizuku deployed
assert!(game.state.player1.hand.cards.contains(&filler));
```

Three compounding problems:

1. **The sole assertion restates a precondition the test established two lines
   earlier.** Nothing in the code under test removes a non-matching card from
   hand, so `hand.contains(&filler)` was already true and cannot become false.
   The test is unfalsifiable as written.
2. **The comment states the real intent — "no Shizuku deployed" — and nothing
   asserts it.** A regression that deployed Shizuku anyway would pass.
3. `safety < 5` means a choice that never drains exits silently and still
   passes, and each iteration attempts *both* an empty and a `[0]` selection
   while discarding both `Result`s.

For contrast, the next test in the same file
(`shizuku_insufficient_energy_no_pay`) is rigorous — `assert!(res.is_err(),
"should fail to play with only 1 energy, need 13")`, plus
`assert_energy_untouched_after_refusal`, plus a hand check. The file demonstrates
the right and wrong pattern within 30 lines.

### 4.4 T3 — discarded `Result` from `try_*` APIs (23 sites): mostly benign, one systematic risk

The helpers are named `try_activate_ability` / `try_select_indices` /
`try_play_to_stage` precisely because they return `Result` to distinguish
*activated* / *declined* / *errored*. 23 sites discard it.

Reviewed individually, **21 are correct and often explicitly reasoned**. Example,
`tang_keke_energy_or_heart06_choice_test.rs:206`:

> *"The engine refuses the cost at resolution time … and reports it via logs, not
> via Err — so assert the observable behavior: no cost prompt, nothing discarded,
> no effect happens."*

That is a deliberate choice to assert observable state instead of an error
channel, and the surrounding assertions are substantive. Same for the
`drain_choices()` / `drain_auto_ability_choices()` sites and
`corpus_smoke_test.rs`.

The residual risk is systematic: **because the `Result` is dropped, no such test
can distinguish "the ability correctly declined" from "the ability errored and
did nothing."** For documented cost-refusal cases that is not what is under test,
so it is fine. It would not be for any test asserting a *positive* outcome after
a `try_*` call — a silent early `Err` and a correct decline leave identical
state, and only the `Result` separates them. Worth a project-wide convention:
when a test asserts a positive outcome after a `try_*` call, assert the `Ok`.

### 4.5 Verdict

The corpus is in good shape: no unfalsifiable-by-construction tests of the naive
kind, no live `assert!(true)`, no ignored tests, and a documented cleanup
history. What remains is concentrated in the class that actually bites —
**inverted guards where the failure path returns early**, and **assertions that
restate a precondition the test just established**. Both are invisible to the
suite by construction, which is exactly what makes them worth fixing. Both are
one-line fixes. Neither is caught by anything today.

---

## 5. Structural findings not about performance

**F5 — the tree does not compile.** `error[E0063]: missing field
choice_repeats_remaining in initializer of AbilityQueueEntry`, from
`engine/src/ability/queue.rs` (+13/-1, uncommitted). Not part of this session's
work. Flagged because any measurement or test claim made while it is broken is
meaningless.

**F8 — the `total_actions` gate is unsound under concurrent work.** Four commits
landed on this branch mid-session and moved `total_actions` 29,182 → 29,521 by
touching `condition/card.rs`, `condition_decoder_gen.rs`, `comparison.rs`,
`turn/trigger_scan.rs` and `core/game_state/mod.rs`. The test count drifted
4,006 → 4,009 → 4,072 over the same window. A before/after allocation delta is
attributable only if (a) no commit landed since the baseline and (b) the diff
does not touch rules code. Both conditions are cheap to check and were not
checked until it mattered.

**Dead state, candidates only.** With `TriggerEvent` gone, three
`GameState`/`AbilityQueueEntry` fields exist to feed a struct that no longer
exists or a reader that never existed:
`recently_moved_from_zone` (per-movement `String`, display-only now),
`AbilityQueueEntry.trigger_type`/`player_id` as `String` (§6.7 — both feed
`.clone()`s on hot paths), and `entry_snapshot_last_energy_placed_by_player()`
which returns an owned `Option<String>` per call. Each needs a reader audit
before removal, in the §1.2 order: serialised? iterated order-dependently?
projected by display?

---

## 5. Recommended next actions

1. **Unblock the build** (F5) before any measurement or test claim.
2. **Stop using whole-game `alloc calls` as a progress metric.** It is a
   regression guard. For progress, use
   `tests/test_modules/perf/alloc_profile.rs`, which is per-operation, part of
   the suite, and cheap.
3. **Re-point remaining effort at `generate_possible_actions`** (F4). It is the
   only path with allocation volume. Its ~75 allocations per action should be
   attributed by call site before anything is changed — the
   `RABUKA_TRACE_ALLOC_AT` backtrace tracer in `alloc_counter.rs` already does
   exactly this and has never been used on this path.
4. **Retire the catalogue items whose premise measurement killed** (§3.7's
   rationale, §3.8 entirely, §4.2's framing). Leaving them marked open sends
   the next person after costs that are already zero.
5. **Add the two cheap precondition checks to the perf gate** — no commits since
   baseline, and the diff does not touch rules code (F8).
6. **Make the `peek()` dangling-reference pattern unrepresentable** (§2.7), or
   add a lint note. It is the one defect class here that the compiler and the
   test suite both failed to catch.

---

## 6. Confidence

| Tier | Meaning | Applies to |
| --- | --- | --- |
| **Measured** | Read off the running profiler or a test run | F1–F4, F5, all of §2's equivalence claims, the §1 numbers |
| **Read** | Verified by reading the code and the pre-change version | §2.1–§2.11, F6, F7 |
| **Hypothesis** | Argued, not verified | F8's *severity* (the commits are confirmed; that it invalidated a specific earlier claim is inferred) |

No wall-clock claim appears anywhere in this document. The machine reports
63–239 games/sec for one identical binary, so timings are not evidence here and
none are offered.
