# Engine optimization catalogue — the long list

> **Companion to `docs/PERF_ENGINE_COMBINED.md`, not a replacement.** That document
> is the record of what was measured, landed, and *closed*. This one is the
> catalogue of everything still on the table, organised by mechanism so that
> related fixes land together. Read §0 and §1 of the combined doc first: the
> measurement contract and the closed list are load-bearing here and are
> summarised (not restated) in §1 below.
>
> Engine *correctness* work is not here. See `docs/ABILITY_PIPELINE.md`,
> `docs/TEST_AUDIT_PLAN.md`, and `engine/tests/TEST_QUALITY.md`.

## 0. What this document is, and how to read it

> **STATUS UPDATE (2026-09-30).** Phase 1 and part of Phase 2 have been
> implemented and landed; the full suite is green at **4,009 tests** and
> `total_actions` is **unchanged at 29,182** across every measurement, which is
> the trajectory gate §1.1 requires. See §19 for what landed, what was
> deliberately skipped, and **two places where this catalogue turned out to be
> wrong** — one of them a safety bug it recommended.

### 19.1 Landed

| Item | What changed |
| --- | --- |
| §3.1 | `push_cond_verdict` gates internally; `describe_condition_expectation` rendered once, not three times |
| §3.2 | `push_effect_verdict` gates internally; `debug_enabled()` added to the `no_std` log module so console builds get the gate too |
| §3.3 | `pay_ability_cost`'s `cost_desc` gated on the verdict gate |
| §3.5 | `rule_log_activated` gated before building its two `String`s |
| §3.6 | **All 21** direct `gs.rule_log` writes converted to the gated `push_rule_log_fmt` |
| §3.7 | *not landed* — see §19.3 |
| §4.5 | *not landed* — see §19.2, it is unsafe |
| §4.6 | `waited_stage_cards` → `SmallVec<[i16; 3]>` at all 3 sites; callee signature unchanged |
| §4.7 | `performer_owned_ids` `HashSet` → three-zone scan |
| §4.13 | `condition.common()` bound once for the post-dispatch negation guard |
| §5.2 | `normalized_steps_cow` added; routing passes a borrowed step slice instead of a synthetic cloned parent |
| §5.3 | `prepare_sequential_action` returns `PreparedStep` — borrowed when the step needs no strip and no inheritance |
| §5.5 | `apply_modify_cost_to_ability_cost` → `adjusted_cost_after_modify`, clones only when a per-unit `modify_cost` applies |
| §5.8 | `find_printed_ability` no longer re-resolves; `requeue_remaining_after_choice` drops the intermediate `tail` Vec |
| §6.6 | `ArcStr::from(&str)` allocates once instead of twice |
| §6.5 | `AbilityRef::peek()` added; applied at the two hottest read-only sites |
| §6.4 | `normalize_name`'s doc comment corrected — it never avoided allocation |
| §7.10 | `.count()` → `.any()` for monotone `location_condition`, gated on `!debug_enabled()` so the verdict still logs the true count |
| §7.1 | `card_name_matches_group`: allocation-free fast path replacing `get_card_names`, with a new corpus-wide equivalence test |
| §9.1 | `ability_master_id()` → `Option<&str>`; 22 call sites migrated |
| §3.24 | `counted_by_any()` borrowed instead of deep-cloned |

### 19.2 Correction — §4.5 would have been a gameplay bug

§4.5 recommended an early-exit guard on `trigger_auto_abilities_for_player`,
reasoning that the `TriggerEvent` build (a `SmallVec` clone plus an
`Option<String>` clone, ~4 allocations per card play) was "usually discarded".

**That is wrong.** `trigger_auto_abilities_for_player_with_event` scans the
board and enqueues AUTO-triggered abilities **regardless of the event's
contents**. A default `TriggerEvent` is not a no-op, so the guard would have
silently suppressed real triggers. The catalogue's stated justification — that
the predicate is a "known-correct guard already in this codebase" — is a
misreading: the identical predicate at `turn/actions/mod.rs:1464` guards a
*different* call.

**Skipped.** The allocation cost is real, but removing it needs the `TriggerEvent`
to borrow from `GameState`, which the current `&mut self` design cannot express.

### 19.3 Correction — §5.2/§5.3 were worth ~10× less than estimated

§5.2/§5.3 predicted **~128 allocations per sequential ability**. Measured across
the two changes, together they removed **34,624 allocations out of 5.5M (~0.6%)**
on the standard 200-game bench — not the ~1.5% the per-resolution figures implied.

Likely cause, and it is checkable: `PreparedStep::Borrowed` is only chosen when
the step carries **no** condition, because the condition strip is a mutation and
`execute_effect` takes `&AbilityEffect`. If most sequential steps do carry a
condition, the owned path still fires and the win evaporates. The fix for that
is to thread a "condition already evaluated" flag down `execute_effect` so a
borrowed step skips re-evaluation — that also overlaps §8.2's redundant
`can_activate_effect` calls, so it is worth doing as one piece.

**The lesson for the rest of this catalogue:** the per-call-site `Scale` figures
in §5 are the least trustworthy numbers here. A `Box<AbilityEffect>` deep clone
really is ~4 allocations, but the *frequency* of reaching a site is what
dominates, and frequency was inferred from source structure rather than counted.
Any Phase 2/3 item in this document whose case rests on a per-call frequency
should get a counter before the refactor, not after.

### 19.4 Still open

§3.7 display-accumulator gate (needs the `DisplaySources<T>` enum plumbing),
§3.8 the unfinished `recalculate_constants` scratch set, §3.9/§3.10/§3.11 the
remaining log-argument bindings, §4.1/§4.3/§4.4/§4.8/§4.9/§4.10/§4.11/§4.12/§4.14/§4.15,
§5.4 `EffectOwner` provenance, §6.1/§6.2/§6.3/§6.8/§6.9/§6.11/§6.12, §7.3–§7.13,
§8.2 the verdict token, §9.2–§9.9, §10.x, §11.2/§11.3, §13 build config,
§14 threading, §15 the deliberate projects.


Every item below carries the same six fields:

| Field | Meaning |
| --- | --- |
| **Where** | `file:line`, verified against the source as of 2026-09-30 |
| **Cost** | *Why* it is slow, in mechanism terms — allocation, redundant derivation, dependent load, cache miss, branch, syscall |
| **Fix** | The concrete change, in enough detail to implement without re-deriving the analysis |
| **Scale** | Rough impact, expressed in the doc's own currency (allocations per action / per resolution), **derived from call-site counts and code shape — not measured** |
| **Risk** | Effort, blast radius, and whether game outcomes can change |
| **Gate** | The specific check required before landing |

**The single most important thing in this document is that `Scale` is an
estimate and `Gate` is not optional.** On this machine, wall-clock is
unmeasurable (§1 of the combined doc: 63–239 gps for one identical binary
across five rounds). Every number below is a prediction from source reading.
Treat `Scale` as a *ranking* and `Gate` as a *requirement*.

### The 30-second version

Three mechanisms account for most of the opportunity, and none of them is
"a hot loop":

1. **Work performed unconditionally to feed a disabled feature.** The engine
   disables action display and rule logging in the benchmark
   (`engine/src/bin/sim_bench.rs:768-778`) but a large number of sites build the
   string, do the database lookup, or run the format machinery *before* reaching
   the gate. §3. This is by far the largest single block of removable work and
   it is nearly all mechanical.
2. **Loop-invariant work hoisted back inside loops.** The stage is 3 slots and
   the hand is ~7 cards, yet ability sets, filters, group sets and modifier
   lookups are re-walked once per element. §4.
3. **Deep-cloning a ~1.7 KB effect tree on the resolution path.** `AbilityEffect`
   clones cost ~4 allocations; `normalized_steps`, `prepare_sequential_action`
   and the `EffectOwner` writes pay it per step, per repeat, per choice. §5.

The remaining sections cover representation (§6), scans and complexity (§7),
the condition evaluator (§8), the choice round trip (§9), effect execution
(§10), the turn loop (§11), console targets (§12), build configuration (§13),
threading (§14), and the deliberately-large caching projects (§15).

---

## 1. Ground rules — read before touching anything

### 1.1 The measurement contract

Engine speed here is measured on `sim_bench` with the **random** policy,
because a real policy is tuned against the engine and can disguise an engine
slowdown as a policy change.

```
cargo build --release --bin sim_bench
.\target\release\sim_bench.exe --deck "5CP3Z idou" --games 500 --repeat 3 --seed 1 --policy random --jobs 1
```

**Gate on allocation counts, not games per second.**

```
cargo build --release --features "profiling alloc_tracker" --bin sim_bench
$env:RABUKA_ALLOC_TRACK="1"
.\target\release\sim_bench.exe --deck "5CP3Z idou" --games 200 --seed 1 --policy random --jobs 1 --alloc
```

Two runs of the same binary give *exactly* 10,228,233 allocations and 29,648
actions. That determinism is the instrument this whole catalogue is tuned for.

**`total_actions` must be held fixed.** If it moves, the work being measured is
no longer the same work and the allocation delta means nothing. The current
trajectory is **29,875** (as of the 2026-09-29 reorganisation; the combined doc's
older 29,341/29,648 figures are stale). Re-baseline before quoting absolute
numbers.

**The gameplay gate is `total_actions` *plus* a fixed-seed win-rate comparison
over a large `--games` count — not the test suite.** §6 of the combined doc
records a full scripted link match going 28 s → 90 s because
`loopback_full_match_stays_in_lockstep` still passed: that test only checks the
two link sides agree with each other, not against a golden.

### 1.2 The engine depends on `HashMap` iteration order

Not "might" — **does**. Swapping `compat::HashMap` from `std::collections`
(SipHash-1-3) to `hashbrown` with an FxHash-style hasher changed engine
behaviour. The hasher swap was reverted.

Two permanent consequences:

- Any change to `compat::HashMap`, or to code that iterates a map and *acts on
  the order*, is a **gameplay change**. Gate it on engine outcomes.
- Adding a **side table** or removing an **allocation** is safe. Changing a
  **hash** or an **iteration order** is not. Several items below are explicitly
  scoped to stay on the right side of that line, and say so.

A faster hasher is worth retrying *with a properly mixed `finish()`*, but only
on a machine where throughput is measurable, and only behind the gameplay gate.

### 1.3 Closed — do not retry without re-measuring

Reopening one of these means re-measuring, not re-arguing.

| Idea | Recorded result |
| --- | --- |
| `lto = "fat"` + `codegen-units = 1` | +2–4% single-threaded, inside noise; appeared to cost throughput at high `--jobs`. Deliberately uncommitted. |
| Faster / pooling allocator | Dead. `fast_alloc` feature exists and is deliberately **off** (~1.5%, inside noise, and a hand-written global allocator is the highest blast-radius code in a program — the last thing to want on the 3DS/wasm/PSP builds). |
| `Condition` `Box<T>` → `Arc<T>` | **Negative**: −683 allocations, −0.007%. The inference is the useful part — condition cloning was never the bottleneck. |
| Ability re-decode at call time | Dead on desktop (`OnceLock` per slot; only 17 `get_ability` calls). **Still live on `no_std`** — see §12. |
| State hashing to short-circuit repeats | Dead. Ablation showed repeats are already cheap. |
| 4× trigger scan cost | Measured 2.7%, "not the lever" — but the model behind the number is wrong, see §2. |
| `recalculate_constants` dominates | No. 4.1% ceiling, and the ceiling did not move. |
| Memory pressure | Dead. No growth while games run. |
| Logging args (the `log` macro) | Dead, already. The args are not the cost. **But see §3** — the *statements the args depend on* are. |
| Action-list caching | Dead for this benchmark: `generate_possible_actions` is called 1:1 with `execute_and_settle`. |
| Boxing `ActionParameters` | **Net loss**: saves ~0.5% of memcpy, adds ~22 allocations per call (~1.5M per 200-game run). |
| Dirty-flag caching of `recalculate_constants` as a *first* move | 4.1% ceiling; `active_energy_count` is written from ~12 raw sites plus a self-referential read of `mods`' own output. |
| RAM reduction | Not the lever. All resident card data is ~2.4 MB, per-game churn a few MB. The allocator share is *CPU time*, not memory. |

### 1.4 Already landed — do not re-propose

Gate tables as `const` fn-pointer arrays · `AbilityEffect.kind` as
`Arc<EffectKind>` (pool deleted) · `EffectOwner` **accessors** borrow instead of
returning owned effects · two full-`Card` deep clones removed from the hot path ·
per-call `HashSet`s in `recalculate_constants` replaced by three-zone `contains`
scans · condition clone skipped when no overlay patch applies · the
`describe_condition_actual` dead `String` build gated behind
`log::buffer_len()` · `AbilityQueueEntry.id` deleted · `opponent_id` returning
`PlayerId`.

The important boundary: the `EffectOwner` **accessors** were fixed. The
`EffectOwner` **writes** were not — see §5.4.

---

## 2. Instrument before you optimise

Two of the biggest open questions in this catalogue are answerable for the cost
of a few lines, and one of them invalidates a number the project has been
reasoning from.

### 2.1 Count trigger-scan invocations (highest value, ~15 minutes)

The combined doc records the trigger scan as "2× per plain play, 4× for
baton-touch, measured at 2.7% — not the lever", and separately notes the figure
is *undercounted* because the live phase issues more scans than the model
assumed. Both statements are right about the explicit call sites and both miss
the implicit one.

There is an unmodelled scan. `process_current_ability` — which the code's own
comment at `core/game_state/abilities.rs:1890` calls *"the largest self-time
site in the engine"* — runs a full board scan after **every ability that
touched a card**:

```rust
// core/game_state/abilities.rs:2242-2265
if self.recently_moved_cards.is_some()
    || self.last_energy_placed_by_effect()
    || !self.recently_appeared_cards.is_empty()
{
    let event = crate::ability::types::TriggerEvent { /* ...cloned... */ };
    self.set_just_completed(just_completed_key);
    self.trigger_auto_abilities_for_player_with_event(&current_pid, &event);
```

And the live phase issues far more than the model assumes. Inside
`execute_performance_phase` (`turn/phases.rs:317-646`) per performer:

| line | call |
| --- | --- |
| 349 | `check_timing` → `recalculate_constants` + `process_pending_auto_abilities` (1–2 scans) |
| 422–423 | TAS + process |
| 520–521 | conditional: TAS + process (heart check moved live cards) |
| 627–628 | TAS + process (post-snapshot) |
| 631–632 | **TAS on the opponent** — "also scan the opponent's auto-abilities" |
| 634–637 | **TAS + process on both players again** if any draw effect occurred |

That is 3 unconditional scans per performer, 4 with the heart check, 6 with a
draw effect — and `execute_performance_phase` runs twice per turn. Add
`turn/live.rs:1201-1244` (up to 6 more), `live.rs:1657-1666` (2),
`live.rs:379-399` (2, each of which can enter the `batch_rerun` loop), and
`turn/phases.rs:245-246` (2 `trigger_live_start_abilities`, each preceded by
`is_trigger_suppressed` — itself a full board scan).

**Estimated real rate: 18–24 scans per turn from the live phase alone**, plus
one per card-touching ability in the main phase. The 2×/4× model is off by
roughly an order of magnitude on the live phase.

**Do not quote 2.7% × any factor.** That is a re-derivation, not a
measurement. Instead:

```rust
// core/game_state/abilities.rs, at the top of
// trigger_auto_abilities_for_player_with_event
static TAS_CALLS: AtomicU64 = AtomicU64::new(0);
TAS_CALLS.fetch_add(1, Ordering::Relaxed);
```

print it per game alongside `alloc calls`, and let the real number decide
whether §11.1 and §11.2 are worth doing. This changes no behaviour, costs
nothing, and converts an argument into a fact.

### 2.2 Count condition evaluations, separately per call site

`can_activate_effect` has 14 references and **4 live call sites on the
`resolve_ability` path** (§8.2). A per-site counter distinguishes "the
evaluator is slow" from "the evaluator is called 4× for one decision" — two
completely different fixes. The combined doc's inference pointed at the first;
§8.2 argues the second is larger.

### 2.3 Confirm the log/display gates are actually off in the benchmark

`engine/src/bin/sim_bench.rs:768-778`:

```rust
let display = std::env::var("RABUKA_ACTION_DISPLAY")...unwrap_or(false);
let logs    = std::env::var("RABUKA_LOGS")...unwrap_or(false);
rabuka_engine::core::runtime_flags::set_action_display(display);
rabuka_engine::core::runtime_flags::set_logging_enabled(logs);
```

Both default **off**. `ABILITY_DEBUG` also defaults false
(`ability/debug.rs:6`). Most of §3 is only removable *because* these are off.
Re-confirm before landing any of it, and note that **the shipping paths do not
have them off** — the web server, `turn_sim` and the human-facing game never
call the setters, so `logging_enabled()` keeps its `true` default. A change in
§3 therefore improves the benchmark *and* `headless`/training runs, and is a
no-op for the UI.

---

## 3. Category A — work performed unconditionally to feed a disabled feature

**This is the largest block of removable work in the engine, and it is almost
entirely mechanical.** The pattern is always the same:

> An argument is built — `format!`, a `String`, a `Vec`, a database lookup, a
> `format!("{:?}")` of a deep tree — and only *then* reaches a gate that
> discards it. `log::debug!` is lazy. A hand-written `push_*` helper called
> from a `let` statement is not.

The combined doc fixed exactly one instance of this
(`describe_condition_actual` building a `String` on every condition evaluation,
because `log::buffer_len()` returns 0 when debug is off so `0 <= 0` was a gate
that was never closed). **The same bug survives in at least 20 other places,
unguarded.** This is the single most important structural observation in the
catalogue.

### 3.1 `push_cond_verdict` — three full `format!` renders per verdict, four ungated call sites

`ability/condition.rs:301-349`:

```rust
pub fn push_cond_verdict(
    condition: &Condition, extra_actual: &str, passed: bool,
    children: Vec<crate::ability::log::AbilityLogItem>,
) {
    let condition_type = match condition { /* 21 arms */ }.to_string();   // alloc
    let expectation  = describe_condition_expectation(condition);          // alloc (format!-heavy)
    let actual = if !extra_actual.is_empty() {
        extra_actual.to_string()
    } else {
        describe_condition_expectation(condition)                          // SAME CALL AGAIN
    };
    push_verdict(AbilityLogItem::Condition {
        text: describe_condition_expectation(condition),                  // AND A THIRD TIME
        condition_type, expectation, actual, passed, children,
    });
}
```

Two distinct defects:

**(a) Triple evaluation.** `describe_condition_expectation`
(`condition.rs:154-265`) is a nested `format!`/`join`/`to_string` function.
Calling it three times for one verdict is a 3× redundancy even with debug on.

**(b) Four call sites reach it with no gate at all**, so with `ABILITY_DEBUG ==
false` the whole thing is built and dropped by `push_verdict`'s own early return
(`ability/log.rs:79-84`):

- `ability/condition/card/comparison.rs:132-140`
- `ability/condition/compound.rs:99-105` (verified: `push_cond_verdict(condition, &format!("{}/{}", passed_count, conditions.len()), …)`)
- `ability/condition/compound.rs:110`
- `ability/condition/compound.rs:123-129` (verified: `&format!("{}/{} any", …)`)

Compare the *correct* site, `condition.rs:592-600`, which the combined doc
already fixed:

```rust
if crate::ability::log::debug_enabled() && crate::ability::log::buffer_len() <= before {
    let actual = self.describe_condition_actual(condition);
    push_cond_verdict(condition, &actual, final_result, vec![]);
}
```

`evaluate_compound_condition` runs for every `Compound` node, and a typical
3-level compound is the norm. So this is **several wasted `format!`s per
ability condition evaluation**, i.e. several per action.

**Fix.** Hoist the gate into `push_cond_verdict` itself so it cannot be
forgotten:

```rust
pub fn push_cond_verdict(/* … */) {
    if !crate::ability::log::debug_enabled() { return; }   // fixes all four sites at once
    // …and bind `let expectation = describe_condition_expectation(condition);`
    // once, cloning it into `text` rather than calling it three times.
}
```

Also change `condition_type` to a `match` returning `&'static str` literals
rather than `.to_string()` on a matched literal.

**Scale:** 3–5 allocations per verdict, on the most frequent condition shapes.
**Risk:** trivial. **Outcomes:** cannot change — the return value is `()` and
the buffer is debug-only. **Gate:** allocation count, plus one
`set_debug(true)` run diffing the verdict buffer to prove nothing is lost.

### 3.2 `push_effect_verdict` — four `String`s per effect step

`ability/effects/dispatch.rs:21-52` (verified verbatim), called at `dispatch.rs:225`
after **every** dispatch arm:

```rust
pub fn push_effect_verdict(effect: &AbilityEffect) {
    let is_structural = matches!(effect.action, /* 6 variants */);
    if is_structural { return; }
    let val = effect.count.or(effect.value_any()).map(|v| v.to_string()).unwrap_or_default();
    let details = if !val.is_empty() { format!("{} {}", effect.action, val) }
                  else { effect.action.to_string() };
    crate::ability::log::push_verdict(crate::ability::log::AbilityLogItem::Effect {
        text: effect.text.to_string(),
        action: effect.action.to_string(),
        details,
    });
}
```

`execute_effect` is the per-effect-step entry point, so this is 4–5 allocations
per effect step on a path the combined doc puts inside `use_ability`'s 399
allocations per call.

Wider than it looks: the `is_structural` early-out covers 6 variants, but four
**no-op** `ActionType`s (`ModifyYellSource`, `ReplacementAppearWait`,
`CountAsMultiple`, `RepeatProcedure`) fall through to the full build and are
then discarded. Four action types that do literally nothing still pay the full
verdict cost.

**Fix:** `if !crate::ability::log::debug_enabled() { return; }` as the first
statement, before the structural check.

**Scale:** 3–4 allocations per non-structural effect execution.
**Risk:** trivial. **Outcomes:** cannot change. **Gate:** as §3.1.

### 3.3 `pay_ability_cost` builds a cost description unconditionally

`ability/resolver.rs:1110-1125`:

```rust
#[cfg(not(feature = "no_std"))]
let cost_desc = format!("{}: {}→{} {}", cost.action, cost.source.map(|z| z.as_str()).unwrap_or("?"), …);
#[cfg(not(feature = "no_std"))]
push_verdict(AbilityLogItem::Cost {
    text: cost.text.to_string(),
    expectation: cost_desc,
    actual: "支払済".into(),
    …
});
```

`#[cfg(not(feature = "no_std"))]` gates the *existence* of the code on std — not
on `ABILITY_DEBUG`. The desktop benchmark builds all three and discards them on
every costed ability resolution.

**Fix:** wrap the whole block in `if crate::ability::log::debug_enabled() { … }`.
Note `dbg.cost_pay(cost, true)` on the line above already has this shape via
`AbDebug` — copy it.

**Scale:** 2–3 per costed ability. **Risk:** trivial. **Outcomes:** cannot change.

### 3.4 `store_pending_choice` computes a `Vec<String>` + signature before the gate

`ability/resolver.rs:817-837` (verified verbatim):

```rust
pub(crate) fn store_pending_choice(&mut self, gs: &mut GameState) {
    gs.ability_queue.snapshot_requested = true;
    self.capture_choice_owner();                       // load-bearing — keep this
    if let Some(ref choice) = self.awaiting.choice {
        let offered = gs.choice_offered_labels(choice);            // Vec<String>, ~2-8 allocs + N formats
        let sig = choice_offer_sig(&offered, choice.allow_skip()); // String, a write! per option
        let is_new_offer = self.carried.last_offered_sig.as_deref() != Some(sig.as_str());
        if is_new_offer {
            gs.push_choice_offered(choice);   // THIS is gated (mod.rs:1382) — and recomputes `offered`
            self.carried.last_offered_sig = Some(sig);
        }
```

`push_choice_offered` early-returns on `!logging_enabled()`. But
`choice_offered_labels` and `choice_offer_sig` run **before** that gate and are
pure logging work. Choices are the most common action type in a 148-action game,
so this is the densest allocation site in the answer path.

Deeper: `choice_offered_labels` (`core/game_state/mod.rs:1273-1300`) calls
`resolve_index_any_player` per filtered index, and that allocates **twice per
call** (`mod.rs:1251-1269`): `format!("#{idx}")` on the early return *and*
again inside `resolve_card`'s `unwrap_or_else`. A `SelectCard` over 8 candidates
is ~20 allocations, all discarded.

**Fix, three separable steps:**

1. Gate the whole `offered`/`sig` block on `logging_enabled() || ABILITY_DEBUG`,
   placed **after** `capture_choice_owner()`. When logging is off, `last_offered_sig`
   staying `None` is already its meaning.
2. Make `resolve_index_any_player` return `Option<&str>`/`Cow<str>` and compare
   the resolved `Option<i16>` instead of formatting a string to compare it.
3. Pass the already-computed `offered` into `push_choice_offered` instead of
   recomputing it.

**Scale:** ~20 allocations per choice offer. `resume_with_choice` is one of the
three worst leaves by allocations per call (148).
**Risk:** low. **Outcomes:** step 1 changes *log output* (deduped offers may
appear that were suppressed) — not a game outcome, but it will change
`structured_log` contents and can break a golden-log test. Check before landing.
**Gate:** `total_actions` fixed + full suite + grep the log assertions.

### 3.5 `rule_log_activated` computes two `String`s before the gated push

`ability/log.rs:135-143`, called from the `Select` / `DiscardCard` / `MoveCards`
dispatch arms:

```rust
pub(crate) fn rule_log_activated(&self, gs: &mut GameState, label: &str) {
    let pp = self.player_prefix(gs);                            // String + up to 6 zone scans
    let cn = gs.activating_card.and_then(|id| gs.card_database.get_card(id))
        .map(|c| c.name.to_string()).unwrap_or_default();         // String + DB lookup
    gs.push_rule_log_fmt(format_args!("{} {}: {}", pp, cn, label));   // gated — too late
}
```

**Fix:** `if !crate::core::runtime_flags::logging_enabled() { return; }` at the top.
**Scale:** 2 allocations + 6 scans per logged action. **Risk:** trivial.

### 3.6 **21 sites write `gs.rule_log` directly, bypassing the gate entirely**

`GameState::push_rule_log` / `push_rule_log_fmt` (`core/game_state/mod.rs:1492-1506`)
both early-return on `!logging_enabled()`. These 21 sites reach past them to the
raw `Vec` (verified list):

```
ability/effects/ability_ops.rs:291, 378, 739, 769
ability/effects/blade.rs:203
ability/effects/change_state.rs:764
ability/effects/cost.rs:66, 375
ability/effects/deck.rs:481
ability/effects/dispatch.rs:275
ability/effects/energy.rs:591
ability/effects/look.rs:896, 1091
ability/effects/position.rs:90, 108, 1175
ability/effects/required_hearts.rs:372
ability/effects/restriction.rs:43
ability/effects/reyell.rs:58
ability/effects/selection.rs:168, 229
```

Representative — `ability/effects/change_state.rs:759-765`:

```rust
let pp = self.player_prefix(gs);                                            // String + 6 zone scans
let act_name = gs.activating_card.map(|c| self.card_name(c)).unwrap_or_default(); // String + DB lookup
gs.rule_log.push(format!("{} {}: 状態変更→{}", pp, act_name, state_change));       // String
```

**Three allocations plus six zone scans plus a database lookup, all wasted** in
headless. It also defeats `truncate_rule_log`, so `rule_log` grows unbounded in
exactly the runs that disable logging.

**Fix:** mechanical — `gs.rule_log.push(format!(…))` →
`gs.push_rule_log_fmt(format_args!(…))`. 21 edits, zero semantic change.

**Scale:** up to 3 allocations per executed effect. **Risk:** trivial.
**Outcomes:** cannot change. **Gate:** `total_actions` fixed; also assert
`rule_log.len()` no longer grows under `set_training_mode(true)`.

### 3.7 Seven display-only accumulators built on every `recalculate_constants`

`GameModifiers` carries eight `*_sources` fields whose only readers are
`game/display.rs` (plus one display-only push at `turn/phases.rs:604`):

| Field | Written | Read |
| --- | --- | --- |
| `constant_blade_sources` | `modifiers.rs:605` | `display.rs:1710` |
| `constant_heart_sources` | `modifiers.rs:606` | `display.rs:1711` |
| `constant_need_heart_sources` | `modifiers.rs:602` region | `display.rs:1713` |
| `constant_global_need_heart` | `modifiers.rs:602` region | `display.rs:1733` |
| `constant_score_sources` | `mod.rs:201` | `display.rs:581,1514,1517,1723,1990,1992` + `phases.rs:604` |
| `success_zone_blade_sources` | `modifiers.rs:2116` | `display.rs:1714` |
| `success_zone_heart_sources` | `modifiers.rs:2117` | `display.rs:1715` |
| `success_zone_score_sources` | `modifiers.rs:2118` | `display.rs:1716` |

A ready-made gate already exists — `runtime_flags::action_display_enabled()` —
and **is read in 16 places, all in `game/game_setup.rs`**. The layers that do
the re-derivation (`ability/`, `core/game_state/`, `core/game_modifiers.rs`)
never ask. `BonusSource` is 96 bytes inline with two `String`s; `ui_text`
already returns `String::new()` when logging is off, so the *strings* cost
nothing in the benchmark — but **the `Vec` buffer does**: 1 allocation plus
growth reallocs per non-empty vector, per `recalculate_constants`, ×2 per action.

**Fix:**

```rust
let display = crate::core::runtime_flags::action_display_enabled();
if display { self.mods.constant_score_sources.clear(); }   // and the 6 other clears
if display { exp_blade_sources.push(BonusSource { … }); }  // and the 6 other pushes
```

**Scale:** ~7 `Vec` allocations + growth reallocs per `recalculate_constants`,
×2 per action — on the order of 8–16% of the ~98-allocations-per-action budget.
**Risk:** trivial. **Outcomes:** **cannot** change, and the reasoning is
provable: no field is read by `ability/`, `core/`, `turn/` (beyond the
display-only `phases.rs:604`) or `bot/`; the benchmark already sets the flag
false; and the UI paths never call the setter, so they keep the `true` default.
**State this in the commit message** so nobody later "fixes" it by moving a
read into `core/`.

### 3.8 Six of the seven `recalculate_constants` scratch buffers were never added

`core/game_state/modifiers.rs:591-611` — the comment says *"Reuse pre-allocated
scratch buffers to avoid allocation storm"*, takes three, and then allocates
seven more five lines later (verified verbatim):

```rust
let mut exp_blade = core::mem::take(&mut self.scratch_exp_blade);            // 592 ✔ reused
let mut exp_score = core::mem::take(&mut self.scratch_exp_score);            // 594 ✔ reused
let mut exp_heart = core::mem::take(&mut self.scratch_exp_heart);            // 596 ✔ reused
let mut exp_prohibition: Vec<String> = Vec::new();                           // 598 ✘
let mut exp_delayed_gained_effects = SmallVec::new();                        // 599 ✘
let mut expected_gained_texts: HashMap<i16, Vec<String>> = HashMap::default();// 600 ✘ (2 allocs when used)
let mut exp_global_need_heart: Vec<(i16, String, i16)> = Vec::new();         // 602 ✘
let mut exp_blade_sources: Vec<BonusSource> = Vec::new();                     // 605 ✘
let mut exp_heart_sources: Vec<BonusSource> = Vec::new();                     // 606 ✘
let mut exp_global_nh_sources: Vec<BonusSource> = Vec::new();                 // 607 ✘
let mut jyouji_statuses: Vec<ConstantAbilityStatus> = Vec::new();             // 610 ✘
```

Note `jyouji_statuses` at 610 **is** already gated on
`logging_enabled()`/`headless` while its six neighbours are not. That
inconsistency is the argument.

`exp_delayed_gained_effects` is the interesting one: its inline capacity is
inferred from the pushes, and the combined doc's *"Spill >100 → 16 KiB"* datapoint
is the fingerprint of exactly this kind of accumulator. A fresh `SmallVec` that
spills pays a fresh 16 KiB-class allocation **per `recalculate_constants`**,
i.e. twice per action.

This is **not** the dirty-flag project the combined doc declined. It is
finishing an optimisation that was started and left three-quarters done, in the
same style, on the same lines.

**Fix:** add `scratch_exp_prohibition`, `scratch_exp_global_need_heart`,
`scratch_exp_*_sources`, `scratch_jyouji_statuses`,
`scratch_expected_gained_texts` alongside the three that exist, and `mem::take`
them identically.

**Scale:** `recalculate_constants` is called ~110×/game, so ~770 allocations/game
if all seven are populated — up to ~5% of the 15,600/game baseline, from a
mechanical change. **Caveat:** `Vec::new()` does not allocate until first push,
so the real count depends on population. **Measure, do not assume.**
**Risk:** low, and `mem::take` is content-preserving because every use site
starts with `.clear()`. **Audit all 11 use sites for the `.clear()`** before
merging — the doc records a regression in exactly this area. Do it one field at
a time with measurement between. **Outcomes:** cannot change.

### 3.9 `hand_cost_matching_indices` builds a `Vec<String>` and a join for a disabled log

`ability/cost/handlers.rs:397-522`:

```rust
let match_names: Vec<String> = matching_indices.iter()
    .filter_map(|&i| card_db.get_card(pl.hand.cards[i]).map(|c| c.name.to_string()))
    .collect();                                                              // N allocs
…
log::debug!("  ├─ hand[{}] → {} match{}: [{}]", pl.hand.cards.len(), matching_indices.len(), …,
            match_names.join(", "));                                          // N+1 more
```

`match_names` is constructed unconditionally and used **only** as an argument to
`log::debug!`. The macro is lazy; the statement it depends on is not. One
`String` per eligible hand card plus one joined `String`, per hand-discard cost,
discarded whenever the log level is off. (Its value is duplicated by the
`HAND_COST_FILTER` loop ten lines below, so it can simply be deleted.)

Same function, `handlers.rs:413-422`: `HashMap<String, Vec<usize>>` with
`card.group.to_string()` per hand card — a `String` allocation **and** a
SipHash of it, per hand card, for the `same_group_name` cost shape. `card.group`
is an `ArcStr` in the database, so `HashMap<&str, Vec<usize>>` borrowed from the
card DB is allocation-free.

**Fix:** delete `match_names` or move it inside
`if log::log_enabled!(log::Level::Debug)`; change `group_counts` to borrow.
**Scale:** 1+N per hand-discard cost. **Risk:** trivial. **Outcomes:** cannot change.

### 3.10 Six sites in the condition layer format a name list for a disabled log

`ability/condition/card.rs:2607-2621`:

```rust
let names: String = cards.iter()
    .map(|&cid| format!("{}x{}", card_db.get_card(cid).map(|c| c.blade).unwrap_or(0), cid))
    .collect::<Vec<_>>()
    .join(", ");
log::debug!("[BLADE_PIPELINE] card={} cards={:?} names={}", card_id, cards, names);
```

`format!` **per card**, plus a `Vec`, plus a `join`. None of it behind
`ABILITY_DEBUG`, and this runs on every `evaluate_card_blade_condition`. Same
shape at `card.rs:1555-1556`, `:3678`, `:3863-3872`, `:3881`, `:4435`.

The correct gate is already established **in the same file** (`card.rs:256-265`
and `state.rs:256-258` do it for `[TEMP_DIAG]`):

```rust
if crate::ability::debug::ABILITY_DEBUG.load(Ordering::Relaxed) { …build names… log::debug!(…) }
```

**Scale:** `cards.len() + 2` allocations per blade-condition evaluation.
**Risk:** trivial. **Outcomes:** cannot change.

### 3.11 A `HashMap` clone computed for a disabled `log::debug!` argument

`core/stats_pipeline.rs:123-129`:

```rust
let member_original_hearts = db
    .get_card(cid)
    .and_then(|c| c.base_heart.as_deref())
    .map(|bh| bh.hearts.clone());
log::debug!("[HEART_PIPELINE] card={} base={:?} modified={:?}", cid, member_original_hearts, adjusted.hearts);
```

`member_original_hearts` is a **statement**, not inside `format_args!`, so it is
evaluated unconditionally — a `HashMap<HeartColor,u8>` clone, i.e. one
allocation plus five SipHash computations, on every `stage_hearts` call, purely
to be a `Debug` argument to a disabled macro.

**This is the one place in the audit where the "`log` args are not the cost"
rule does not hold.** The file already has the correct pattern 90 lines below
(`stats_pipeline.rs:218-224`). **Fix:** guard it with
`if log::log_enabled!(log::Level::Debug)`.
**Scale:** 1 allocation + 5 hashes per `stage_hearts` call — and
`stage_hearts` is on the heart-need path, which is hot. **Risk:** trivial.

### 3.12 `provide_choice_result` / `SelectionContext` clones feeding a live borrow

Covered in detail in §9.3 — `Choice::SelectCard` is ~24 fields including
8 `String`s and 4 `Option<Vec<_>>`, and it is deep-cloned twice per resume.

### 3.13 The generalisable rule

> **A `let` binding computed for a log argument is not lazy. A `log::debug!`
> argument is. The difference is the whole of this section.**

A mechanical sweep for it: for every `log::debug!`/`trace!` in `engine/src`,
check whether its arguments reference a `let` bound immediately above that does
non-trivial work. There are 857 `format!` sites and 638 `log::*!` sites in
`engine/src`; the intersection of "bound to a local" and "does real work" is
where the remaining money in this category is.

---

## 4. Category B — redundant recomputation and loop-invariant work

The engine re-derives rather than caches. `docs/PERF_ENGINE_COMBINED.md` §3
names this correctly and draws the right conclusion: there is no hotspot,
aggregate time is flat, and the fix is a long tail. **This section is that
long tail, grouped by the shape of the redundancy** — because the shapes
recur, and fixing a shape is worth more than fixing an instance.

### 4.1 Shape: the same value re-derived N times inside one loop

The stage is **3 slots**. The hand is **~7 cards**. These are small, fixed
numbers, and the engine re-derives the same answer once per element anyway.

**`recalculate_constants` re-derives "who owns `card_id`" six times per loop
iteration** with a 3-element scan each — `modifiers.rs:647, 742, 826, 890, 976,
1068`, all reading `self.player1.stage.stage.contains(&card_id)`. The map
`entry_positions` is built at `modifiers.rs:613-624` and **already contains the
answer**; it just stores the slot and not the owner.

Worse, `recalculate_constant_cost_modifiers_with_ids` re-derives it a *seventh*
time with a **different predicate** (`modifiers.rs:1382-1386`):

```rust
let owns = |p: &Player| -> bool { p.stage.stage.contains(&cid) || p.hand.cards.contains(&cid) };
```

— stage **or hand**, where the other six are stage-only. That divergence is
latent, and unifying it **will** change behaviour for a card in hand but not on
stage. Treat a divergence as a bug report, not a tuning knob.

**Fix (safe half, first):** widen `entry_positions`' value type to
`(u8 /*owner 1|2*/, u8 /*slot*/)`. It is keyed by an **explicit card id**, not
a map position, and `scratch_entry_positions` is only iterated for `.clear()`
and `.get()` — so this does not touch iteration order and stays on the right
side of §1.2.

**Fix (semantics half, separately, and last):** reconcile the stage-or-hand
predicate at `modifiers.rs:1382`.

**Scale:** 6 × 3-element scans → 1 map lookup per constant entry; six entries ×
two calls per action puts this around 3%. Small individually, but it is
§1.1's diagnosis in miniature: no hotspot, thousands of cheap derivations.
**Risk:** the widening is trivial. The predicate reconciliation is a
**gameplay-visible change** — gate it per §1.1 (1,000-game fixed-seed action
count **and** win-rate comparison), not on the test suite.

### 4.2 Shape: a whole ability-set walk hoisted back inside a per-card loop

`ability/util.rs:256-336` (`calculate_play_cost_reduction`), §2 of the function:

```rust
// Scan stage cards for cost-reduction auras that apply to card_id
for &stage_id in &stage.stage {
    if stage_id == -1 { continue; }
    if let Some(stage_card) = card_db.get_card(stage_id) {
        if let Some(r) = scan_abilities_for_cost_reduction(
            &stage_card.abilities, card_id, card, card_db, stage,
            hand_count, true, waited_stage_cards) { cost_reduction += r; }
    }
}
```

and `scan_abilities_for_cost_reduction` (`util.rs:405`) begins:

```rust
for ar in abilities {
    let ability = ar.resolve();                                          // Arc::clone = atomic RMW
    if let Some(ref effect) = ability.effect {
        if effect.action != ActionType::ModifyCost
            || effect.operation_any() != Some("subtract")
            || effect.location_any().and_then(Zone::from_str) != Some(Zone::Hand) { continue; }
```

The whole ability walk runs **once per hand card** to answer a question whose
expensive half is invariant. What actually varies per hand card is only:
group membership, `target_card.cost` vs the limit, card type /
`ability_filter == "no_ability"`, and `per_unit_cost_reduction` — which reads
`stage` and `hand_count`, **both loop-invariant**. `hand_count` is computed
*inside* the hand-card loop at `game/game_setup.rs:1767` and is constant across
it.

So the cost is O(hand × stage × abilities) atomic increments plus
O(hand × stage × abilities × 8) dependent loads through a ~1.2 KB
`EffectFilter`, where O(stage × abilities) would do.

**Fix — copy a pattern this codebase already has.** `ability/util.rs:733-793`
(`BatonTouchProtection`) documents the exact shape:

> *"the question depends on the incoming card only through a group-membership
> test, so the ability walk has no business being inside the hand-card loop.
> This type splits the two halves: the walk runs once per stage slot, and the
> per-hand-card step is a group lookup against pre-decoded strings."*

Introduce a `StageCostAura` built once per `generate_main_phase_actions`:

```rust
struct StageCostAura {
    groups: &'static [&'static str],         // the aura's group_names, borrowed
    cost_limit: Option<u8>,
    card_type_only_member: bool,
    no_ability_only: bool,
    per_unit: Option<(PerUnitBinding, u8)>,  // precomputed; hand_count-invariant
    flat_value: u8,
    wait_condition: Option<WaitGate>,        // precomputed verdict + groups
    hand_condition_guard: bool,
}
```

`calculate_play_cost_reduction` becomes `aura.affects(target_card) -> u8` — no
`resolve()`, no ability walk, no `*_any()` chain.

**Scale:** the clearest quadratic in the generation path. Hand is commonly 5–8,
stage 3, abilities-per-card 1–4.
**Risk:** medium effort, **medium risk — gate hard.** This changes cost
reduction, which is *the* number that decides which plays are offered.
**Gate:** `total_actions` **must** be identical, and the full 3,712-test suite
must be green. A `total_actions` move means the change is wrong regardless of
what the tests say.

### 4.3 Shape: the same pure-function value computed at several call sites

`distinct_stage_groups` is computed at **four** sites, two of which fire inside a
single `handle_use_ability`:

| # | site | when |
| --- | --- | --- |
| 1 | `game/game_setup.rs:1697` | every `generate_main_phase_actions` |
| 2 | `turn/actions/mod.rs:416` (`check_mandatory_activation_cost`) | every ability activation |
| 3 | `ability/resolver.rs:1429` (`apply_per_unit_cost_reduction`) | every costed ability resolution |
| 4 | `core/game_state/abilities.rs:2813` (`effective_activation_cost`) | any caller that doesn't pre-supply |

This is the clearest **generate → execute** redundancy in the codebase: the
generator computes `stage_groups` and uses it for every `ability_effective_cost`
gate (`game_setup.rs:2158-2166`); the action is then executed and
`check_mandatory_activation_cost` recomputes the identical value from identical
state. (What each iteration costs is §3.1/§8 — up to 15
`card_matches_group_str` calls, i.e. up to 45 allocations.)

**Fix, two options in increasing order of payoff:**

- *Local:* thread one `u8` through `handle_use_ability` → `trigger_auto_ability`
  → `resolve_ability` → `pay_ability_cost`. Invasive plumbing, but **no
  invalidation reasoning at all** and provably the same value.
- *Better:* cache it on `GameState` beside the other materialised counters.
  `distinct_stage_groups` depends only on the 6 stage slots, which change at a
  small enumerable set of write points (`move_card_from_hand_to_stage`,
  `set_area`, position moves, `recalculate_constants`). The combined doc §11
  called `active_energy_count` the hard case because it has ~12 divergent raw
  writers — **the stage array does not.** This is the easy half of the problem
  the doc called hard.

**Scale:** one full recomputation (with its group-match allocations) removed from
two of four sites.
**Risk:** the local version is none. The cached version is low risk — a stale
cache changes costs, which is gameplay — so gate it strictly.

### 4.4 Shape: eight activation gates evaluated in generation, then again in execution

`turn/actions/mod.rs:505-540` vs `game/game_setup.rs:2117-2186`:

| gate | generation | execution |
| --- | --- | --- |
| `has_trigger(Activation)` | `game_setup.rs:2118` | `actions/mod.rs:510` |
| `activation_location` | `:2087`, `:2121`, `:2125` | `actions/mod.rs:511` → `:345-360` |
| `check_trigger_position` | `:2121` | `actions/mod.rs:390` |
| `check_effect_position` | `:2125` | `actions/mod.rs:391-397` |
| `activates_from_discard` | `:2136` | `actions/mod.rs:570` |
| `use_limit` | `:2141-2145` | `actions/mod.rs:522-529` |
| mandatory energy | `:2167-2172` | `actions/mod.rs:530-535` |
| `validate_mandatory_cost` | `:2177` | `actions/mod.rs:559` |

**Do not delete the execution-time checks.** The doc is explicit
(`ability/util.rs:64-66`): *"The engine must never offer an action it would then
refuse to execute."* Both are needed because an opponent ability, a trigger, or
an out-of-band cost payment can change the state between offer and pick. This is
genuine belt-and-braces, not dead code.

**What is safe is to make the generation loop cheaper.** `game_setup.rs:2097-2116`:

```rust
let activatable: Vec<(usize, crate::Arc<crate::card::Ability>)> = card.abilities.iter()
    .enumerate()
    .map(|(printed_index, ar)| (printed_index, ar.resolve()))          // Arc::clone per ability
    .chain(game_state.gained_card_abilities.get(&card_id)
        .into_iter().flatten().enumerate()
        .map(|(gained_index, gained)| (BASE + gained_index, crate::Arc::new(gained.clone()))))
    .collect();
```

Three costs:

1. `ar.resolve()` is an **atomic increment** per ability, purely to extend a
   lifetime — the loop only takes `&GameState`, so the `Vec` is not needed for
   the borrow checker. 1 heap allocation + N atomics, dropped immediately.
2. `crate::Arc::new(gained.clone())` is a **full deep copy of a `Vec<Ability>`**
   into a fresh `Arc`.
3. The gained-ability arm is set up even when `gained_card_abilities` is empty
   (the common case).

Also `game_setup.rs:2087`: `area_name.parse().unwrap_or(MemberArea::Center)`.
`MemberArea::from_str` (`core/zones.rs:141-152`) has an
`_ => Err(format!("Invalid area: {}", s))` arm, so a parse failure allocates a
`String` to throw away — and `area_name` is always one of three literals from
`stage_positions` (`:2076-2080`), so the parse is pure ceremony.
`MemberArea::from_index(slot)` already exists and returns `Option` with no
allocation.

**Fix:** keep the `AbilityRef` (a `u16`, `Copy`) in a
`SmallVec<[AbilityRef; 4]>` and call `resolve()` *inside* the loop, once, only
for abilities that survive the two cheap gates (`has_trigger`, position) — most
abilities on a card fail one of those. Gate the gained chain behind
`if let Some(gained) = …get(&card_id)`. Replace the parse with
`MemberArea::from_index(slot)`. Change `gained_card_abilities` to
`Vec<Arc<Ability>>` so it is an `Arc` bump instead of a deep copy (this also
removes an allocation from `find_gained_activation`, `turn/actions/mod.rs:443,450`).

**Scale:** modest per activation, but activations are frequent and this is
several allocations each.
**Risk:** the SmallVec/parse/`enumerate` changes are trivially
outcome-neutral; the `Vec<Ability>` → `Vec<Arc<Ability>>` change is
neutral-to-positive and touches only construction sites.

### 4.5 Shape: a `TriggerEvent` built four times per card play, usually discarded

`core/game_state/abilities.rs:436-443`:

```rust
pub fn trigger_auto_abilities_for_player(&mut self, player_id: &str) {
    let event = crate::ability::types::TriggerEvent {
        moved_cards: self.recently_moved_cards.clone().unwrap_or_default(),  // SmallVec clone
        moved_from_zone: self.recently_moved_from_zone.clone(),              // Option<String> = HEAP ALLOC
        ..Default::trigger
    };
    self.trigger_auto_abilities_for_player_with_event(player_id, &event);
}
```

`TriggerEvent` (`ability/types.rs:12-26`) is
`{ moved_cards: SmallVec<[i16;4]>, moved_from_zone: Option<String>, … }`. Building
one costs 1 `SmallVec` clone (heap if >4 cards) plus **1 guaranteed heap
allocation** when `moved_from_zone` is `Some` — which it is after any movement.
That is **4+ allocations per card play** before the scan starts, and the scan
finds nothing in the overwhelming majority of those 4 calls.

The callee's own doc comment (`abilities.rs:577-580`) says it plainly:
*"Called 4x per card play (self + opponent, before and after the placement)."*
The same pattern repeats at `abilities.rs:447-454`, `:1817`, `:2254`, and
`turn/actions/mod.rs:1468-1477`.

**Fix — the cheap half first.** Early-exit on the same predicate
`complete_and_scan` already uses at `turn/actions/mod.rs:1464-1467`:

```rust
pub fn trigger_auto_abilities_for_player(&mut self, player_id: &str) {
    if self.recently_moved_cards.is_none()
        && !self.last_energy_placed_by_effect()
        && self.recently_appeared_cards.is_empty() {
        return;   // nothing to match; the scan would be a no-op
    }
    …
}
```

That predicate is a **known-correct guard already in this codebase**, which is
what makes it low-risk. Deeper (take a `TriggerEventRef<'_>` borrowing from
`self`) fights the borrow checker, which is why the owned event exists — do that
only if the guard is not enough.

**Scale:** ~4 allocations per card play × ~55 plays/game ≈ 220/game, ~1.4% of
15,600.
**Risk:** low. **Outcomes:** cannot change — the guard is the same predicate
already in use.

### 4.6 Shape: three identical three-element `Vec`s

`waited_stage_cards: Vec<i16>` is built at three sites, each a heap allocation
for at most 3 elements, each preceded by up to three `HashMap<i16, String>`
lookups:

- `game/game_setup.rs:1732-1741` (generation)
- `turn/phases.rs:967-975` (live-card set)
- `turn/phases.rs:1242-1252` (member play)

```rust
let waited_stage_cards: Vec<i16> = active_player.stage.stage.iter().copied()
    .filter(|card_id| *card_id != -1
        && game_state.mods.get_orientation_modifier(*card_id) == Some("wait"))
    .collect();
```

**Fix:** `SmallVec<[i16; 3]>`. The callee signature is
`waited_stage_cards: &[i16]`, and `&SmallVec<[i16;3]>` derefs to `&[i16]`, so
**every call site and callee keeps its current signature**. This is a one-token
change at three construction sites and zero signature churn — the cheapest item
in the catalogue.

**Scale:** small but free. 3 allocations per member play plus 2 per live-card set.
**Risk:** trivial. **Outcomes:** cannot change. **Do this first.**

### 4.7 Shape: a `HashSet` where the doc already licensed a linear scan

`turn/phases.rs:547-561`, inside `execute_performance_phase` — one of the two
top cost centres:

```rust
let performer_owned_ids: crate::HashSet<i16> = {
    …
    p.stage.stage.iter()
        .chain(p.live_card_zone.cards.iter())
        .chain(p.success_live_card_zone.cards.iter())
        .copied().filter(|&cid| cid != -1).collect()
};
```

A ~7-element `HashSet` — allocated, hashed, inserted — per performance, used for
two `contains` checks per ability application (`phases.rs:566-568`).

**The combined doc §4 item 5 already did exactly this transformation in
`recalculate_constants` ("Per-call ownership `HashSet`s … Now a three-zone
`contains` scan: no hashing, no allocation. −78,764"). This is the same fix, in
the same codebase, in a function the profiler names as a top-four cost centre,
and it was not applied here.**

**Fix:** three slice `contains` scans. **Scale:** low single digits of
`execute_performance_phase`. **Risk:** none. **Outcomes:** cannot change.

### 4.8 Shape: a value re-derived per card that depends only on the filter

**`filter_subset()` built inside a per-card loop** —
`ability/condition/card.rs:200-211`:

```rust
let mut matching = Vec::new();
for &id in &check_cards {
    let mut filter = condition.filter_subset();          // <-- inside the loop
    filter.heart_colors = condition.get_heart_colors().unwrap_or(&[]);
    let other_filters_match = filter.matches(card_db, id, true) && …
```

`Condition::filter_subset` (`core/card.rs:3794-3821`) is not trivial: it calls
`self.common()` — a **21-arm match** on the `Condition` enum — reads 9
`ConditionCommon` fields, and constructs a 40-field `CardFilter` with
`..Default::default()`. For a 30-card waitroom `location` clause with
`card_property` set that is 30 wasted filter builds. Same at `card.rs:3460-3467`,
which builds a *second* hand-rolled `CardFilter` per card.

The loop-invariant part is only `heart_colors`. **Fix:** hoist the
`filter_subset()` call above the loop and set `heart_colors` once.

**Scale:** ~30× fewer filter builds per zone scan.
**Risk:** trivial. **Outcomes:** cannot change — the filter is a pure function of
the condition.

**Related, same file, genuinely different:** `evaluate_card_count_condition`'s
weighted second pass (`card.rs:2062-2083`) calls `count_cards_with_filters`
**per card** after having already called it once on the whole zone at `:2038` —
so the zone is filtered twice, the second time per-card. Gate it on whether any
card in the corpus is weighted (`PL!-pb2-041-L` is the only one), which is a
property of the *counting card* alone and can be lifted out of the loop. **Fix:
skip the second pass entirely when the weighted set is empty.**

### 4.9 Shape: a `str::parse::<i16>()` per candidate × per entry, for a constant

`turn/phases.rs:110-138` (the `Active` phase):

```rust
if game_state.constant_cannot_activate_members.iter()
    .any(|x| x.parse::<i16>() == Ok(cid)) { return None; }
```

`constant_cannot_activate_members` is a `Vec<String>`; `str::parse` runs the full
integer parser **once per (stage card × member)**, for a value that is
**constant for the whole turn** — nothing in the block mutates it. 3 stage cards
× 6 entries = 18 integer parses per Active phase.

The code comment says *"Compare via parse (no alloc)"* — correct on allocation,
but the parse is the cost.

Also at `phases.rs:160-161`: a `HashSet<i16>` of ~15 elements built only to feed
`mods.tick_delayed_cannot_active_for(&owned)` — one allocation, N hashes, N
inserts, for a set that fits in a `SmallVec`.

**Fix:** parse the member list once into a `SmallVec<[i16; 8]>` at the top of the
`Phase::Active` arm; replace the `HashSet` with `SmallVec<[i16; 16]>` and give
`tick_delayed_cannot_active_for` a slice parameter.
**Risk:** **must preserve current semantics exactly** — player-level entries like
`"p1"` currently fail to parse and never match, and that behaviour is
load-bearing. **Outcomes:** cannot change, *provided* those strings still fail to
parse. **Gate:** full suite + `total_actions` fixed.

### 4.10 Shape: a full board walk whose answer is a property of card data

`turn/trigger_scan.rs:132-154` (`is_trigger_suppressed`):

```rust
player.stage.stage.iter().chain(player.live_card_zone.cards.iter()).copied()
    .filter(|&card_id| card_id != -1)
    .filter_map(|card_id| game_state.card_database.get_card(card_id))
    .any(|card| card.abilities.iter().any(|ability_ref| {
        ability_ref.resolve().effect.as_ref().is_some_and(|effect| {
            effect.action == ActionType::SuppressAbilityTrigger
                && effect.suppressed_trigger_any() == Some(trigger_name)
        })
    }))
```

Every card in the stage + live zone, every ability, every `resolve()` (an atomic
RMW), to answer *"is there a `SuppressAbilityTrigger` naming `live_start`
anywhere on this board"*. It has no relation to the card being played, so it is
repeated per scan with an identical answer.

**`SuppressAbilityTrigger` is a card ability — a fixed property of card data,
not runtime state.** The set of (card, suppressed trigger) pairs on a board can
only change when a card enters or leaves a zone.

**Fix:** the cheapest form is a per-card **trigger bitmask** built at load time
and stored beside `CardDatabase.cards` (wire-transparent, so no `serde` change
and no `test_blob_matches_json` impact), making the predicate a single AND.
**Scale:** ~5–15 atomics + 5–15 `get_card` probes per scan, × ~6 scans per play.
**Risk:** low for the bitmask, **provided** it is built with the identical
predicate at load time. **Outcomes:** cannot change *if* built correctly — but
suppression determines which abilities fire, so a bug here is gameplay.
**Gate:** `total_actions` fixed + full suite.

### 4.11 Shape: the trigger string re-split on every `has_trigger`

`core/card.rs:826`:

```rust
pub fn has_trigger(&self, kind: TriggerKind) -> bool {
    self.triggers.as_deref()
        .is_some_and(|t| crate::triggers::parse_triggers(t).any(|k| k == kind))
}
```

`parse_triggers` (`triggers.rs:81`) is `triggers.split(',').filter_map(TriggerKind::from_token)`,
and `from_token` is a `trim()` plus a 9-arm string match. There are **26
`has_trigger(` call sites**, and `ability_trigger_can_activate` (`util.rs:70-75`)
calls it **up to 4 times**.

**Fix:** parse the trigger string **once** into a `u8` bitmask and answer all
four questions from it:

```rust
impl Ability {
    #[inline] pub fn trigger_mask(&self) -> u8 { /* one split, fold to bits */ }
}
// has_trigger(k) becomes: self.trigger_mask() & bit(k) != 0
```

The TAS scan already short-circuits with a direct `&**t == AUTO` compare
(`abilities.rs:634`), so the mask is only built where a multi-kind question is
actually asked.
**Scale:** up to 4 splits → 1. **Risk:** trivial. **Outcomes:** cannot change —
the fold must produce the same membership set.
**Gate:** a test over all abilities asserting the mask agrees with per-kind
`has_trigger` for all 9 `TriggerKind`s.

### 4.12 Shape: `ConditionContext` construction, four times per resolution

`ability/condition.rs:72-84` (`resolve_self_player`):

```rust
gs.activating_card.and_then(|cid| {
    if gs.player1.contains_card(cid) { Some(&gs.player1) }
    else if gs.player2.contains_card(cid) { Some(&gs.player2) }
    else { None }
})
```

`Player::contains_card` (`core/player.rs:651-658`) is six `Vec::contains` scans
including the two large zones (hand, waitroom — 20–50 elements each). All four
`ConditionContext` constructors call it (`condition.rs:104,111,118,131`), and
`ConditionContext` is built **once per `can_activate_effect`** — i.e. **4× per
resolution** (§8.2) — plus once per TAS pre-filter
(`abilities.rs:673, 814, 908`).

**Fix (trivial, zero risk):** stage-first probe. `gs.activating_card` is almost
always a stage member, so:

```rust
if gs.player1.stage.stage.contains(&cid) { … }
else if gs.player2.stage.stage.contains(&cid) { … }
else if gs.player1.contains_card(cid) { … } // existing fallback
```

That turns the common case into two 3-element scans instead of up to twelve
large-`Vec` scans. Pure reordering of an already-ordered test.

**Do NOT take the tempting shortcut** of using `current_entry().player_id` when
`activating_card` is set. `contains_card` and the entry's `player_id` can
disagree, so that is a **gameplay change** and needs its own gate (a
differential run logging every `(activating_card, contains_card owner, entry
player_id)` triple).

### 4.13 `Zone::from_str` re-parsed on the same string, repeatedly

`Zone::from_str` (`ability/enums.rs:26-122`, macro-generated) is a **30-arm
string match** that rustc compiles to a length-switch plus a `memcmp` chain.
**146 call sites.** Same string, same function, multiple times in one invocation:

- `ability/choice.rs:1008, 1043, 1047, 1063, 1084` — five times in one
  `handle_select_card`.
- `ability/util.rs:2083, 2097` — `count_in_zone` calls it **twice** on the same
  `zone`.
- `ability/condition/card.rs:908, 916, 951` — three times in
  `evaluate_location_condition`.
- `ability/condition/card.rs:26` sites in total.

**Fix:** `let zone = Zone::from_str(zone);` once at the top of each function and
match on the `Copy` enum. **Scale:** 4–5 redundant 30-arm string matches per
card-selection resume. **Risk:** trivial. **Outcomes:** cannot change.

### 4.14 A `HashMap` allocated per queue-drain invocation, for a bounded set

`core/game_state/abilities.rs:1640`:

```rust
fn process_player_abilities_depth(&mut self, raw_player_id: &str) {
    …
    let mut reprocess_counts: HashMap<(i16, usize), u8> = HashMap::default();
```

`process_player_abilities` is called twice per `process_pending_auto_abilities`
(`:1860`, `:1874`), which runs once per `use_ability` and once per ability
completion. The guard is `*count > 5` (`:1723`), so the map is bounded at a
handful of entries by construction.

**Fix:** `SmallVec<[(i16, usize, u8); 8]>` with a linear scan.
**Scale:** 1–2 allocations per drain. **Risk:** trivial. **Outcomes:** cannot change.

### 4.15 `post_cost_position_gate` allocates a `Vec` on every resolution

`ability/gates.rs:167-191`:

```rust
pub fn post_cost_position_gate(resolver, gs, ability) -> GateResult {
    let Some(card_id) = resolver.session.activating_card_id else { return Continue };
    let position = gs.find_card_stage_position(card_id);
    let binding = vec![];
    let keywords = ability.keywords.as_ref().unwrap_or(&binding);
    let position_keywords: Vec<Keyword> = keywords.iter()
        .filter(|k| matches!(k, Center | LeftSide | RightSide))
        .cloned().collect();                                       // <-- HEAP ALLOC
    if !position_keywords.is_empty() && !resolver.check_keywords(gs, &position_keywords, position) { … }
```

`POST_COST_GATES = &[post_cost_position_gate, optional_cost_skip_gate]`
(`gates.rs:258`), so this runs on **every** resolution. The `Vec` is non-empty
for any card with a positional keyword — common. The `vec![]` binding is also
constructed on every call.

**Fix:** no collection is needed — `check_keywords` (`resolver.rs:717-794`) only
pattern-matches the three positional keywords. Inline the loop:

```rust
for k in keywords {
    match k { Center | LeftSide | RightSide => if !check_one(k, position) { return GateResult::stop(…); }, _ => {} }
}
```

**Scale:** 1 allocation per resolution. **Risk:** trivial. **Outcomes:** cannot
change — provably identical, the same three keywords in the same order.

**Related, and NOT safe to simply delete:** `find_card_stage_position` is called
**twice** per resolution (once in `activation_keywords_gate`, `gates.rs:121`;
once here) for the same card. That pair *is* separated by cost payment, which
can move the activating card — which is precisely why the post-cost gate exists.
Gate it, do not remove it.

---

## 5. Category C — deep clones of the effect tree

### 5.1 First, the size of the thing being cloned

This reframes the whole category, and it contradicts the combined doc §5's
inference in a way worth stating plainly.

`EffectFilter` (`core/card.rs:944-1160`) has **164 fields**:

```
Option<ArcStr>      56  ->  56 x 16 B = 896 B   (73% of the struct)
Option<Box<..>>     26  ->  26 x  8 B = 208 B
Option<bool>        41
Option<u8>          22
other (Arc<Condition>, PlacementOrder, i8, Zone, Operator, CardType, …)  18
                            --------------------------------------
                            ~1,234 B
```

`Option<Arc<str>>` is 16 bytes (fat pointer, null-pointer niche), so 56 of them
is 896 bytes. `AbilityEffect` is ~152 bytes and holds a
`Box<CompoundBranch>` (unconditional) plus `kind: Option<Arc<EffectKind>>`
wrapping that ~1.2 KB filter.

So one `effect.clone()` is:

- **~4 heap allocations** (`compound` box, the `EffectKind` filter box, the
  nested effect-step `Vec` + boxes, the `condition: Option<Arc<Condition>>`
  refcount) plus **~1.7 KB of memcpy** (the code's own figure, cited at
  `resolver.rs:308-320` and `modifiers.rs:628`), and
- **~56 atomic refcount increments** from the `ArcStr` fields.

The combined doc's `Condition` `Box<T>` → `Arc<T>` result was **negative**
(−0.007%), and its inference was: *"the clone path is done; the remaining
allocations are in what `run_effect` and the action path **build**, not in what
the effect tree copies."* That inference is **half wrong**, and the wrong half
is the expensive half: the `Condition` boxes were the *top* of the tree, but the
~160-field `EffectFilter` with 30 `Box`es underneath was left in place, and there
are ~26 `effect.clone()` sites on the hot path still paying the full price. The
`Box<T> → Arc<T>` change moved a small part of the tree and left the large part.

### 5.2 `normalized_steps()` deep-clones the whole step tree

`core/card.rs:2452`:

```rust
pub fn normalized_steps(&self) -> Vec<Box<AbilityEffect>> {
    if let Some(ref steps) = self.effect_steps {
        return steps.clone();                                        // ~32 allocs PER STEP
    }
    match self.action {
        ActionType::Sequential => self.compound.actions.clone().unwrap_or_default(),
        ActionType::LookAndSelect => { … }
```

Its only caller is `ability/effects/mod.rs:155` (`normalized_sequential_route`):

```rust
let steps = effect.normalized_steps();
if steps.is_empty() { return None; }
let mut normalized = effect.clone();      // effects/mod.rs:159 — another ~32
normalized.effect_steps = None;
normalized.compound.actions = Some(steps);
normalized.action = ActionType::Sequential;
```

**For a 3-step sequential ability, routing alone costs ~32 (parent) + 3×32
(steps) = ~128 allocations before a single step executes.** Then §5.3 adds
another 3×32.

**Fix:** make `normalized_sequential_route` return a borrowed
`Option<&[Box<AbilityEffect>]>` from `effect.effect_steps` or
`effect.compound.actions`, and add
`execute_sequential_with_steps(&mut self, gs, parent: &AbilityEffect, steps: &[Box<AbilityEffect>])`.
Reading `execute_sequential_effect` (`ability/compound/sequential.rs:201-316`),
the synthesized effect is used only for `.conditional` (`:206`),
`.compound.actions` (`:212`), `.is_further` (`:225`, a log), `.action` (`:223`, a
log), and as the parent argument to `prepare_sequential_action` (`:239`). None of
that requires an owned copy. The `LookAndSelect` branch does need real
construction — put it behind a `Cow<'_, [Box<AbilityEffect>]>` or a small enum so
the two common arms borrow.

**Scale:** ~128 allocations → 0 per sequential ability. Sequential is the single
most common `ActionType` family and there are ~23k resolutions per 500-game run;
this is plausibly the largest single item on the board.
**Risk:** medium effort, low risk. **Outcomes:** cannot change — the synthesized
effect is value-identical to the parent. **Gate:** `total_actions` fixed, plus
instrument to assert no executor ever sees the synthesized effect's `kind` (it
is never read on this path).

### 5.3 `prepare_sequential_action` clones every step, unconditionally

`ability/compound/sequential.rs:117`:

```rust
pub(crate) fn prepare_sequential_action(
    effect: &AbilityEffect, action: &AbilityEffect, i: usize, first: &AbilityEffect,
) -> AbilityEffect {
    let mut action_to_execute = action.clone();      // ~32 allocs, EVERY step, EVERY repeat
    action_to_execute.condition = None;
    …
```

Called from the inner loop at `sequential.rs:239`, inside
`for repeat_idx in 0..repeat_max`. The function's entire job is: strip the
(already-evaluated) condition, and inherit at most 5 parent fields. On the
overwhelmingly common path **none of the inheritance branches fire** and the
clone is a pure round-trip.

**Fix:** two-stage. Add
`fn needs_patch(parent, action, i, first) -> bool` mirroring the existing
predicates; when false, return `Cow::Borrowed(action)` with the condition
stripped at dispatch time.

Note `action_to_execute.condition = None` is itself **load-bearing**:
`execute_effect` → `can_activate_effect` re-reads it. The clean shape is to
thread the strip decision down `execute_effect` as a flag, so the borrowed step
never re-evaluates a condition `route_sequential_step` already evaluated
(`sequential.rs:83-102`) — which is also §8.2.

**Scale:** ~32 × steps × iterations → 0 in the common case.
**Risk:** medium — the `set_*` calls at `:133/:137/:143/:181/:184/:194` and the
`Arc::make_mut` at `:150-158` must all still write when they fire.
**Outcomes:** cannot change, *provided* the condition-strip semantics are
preserved exactly. **Gate:** `total_actions` fixed + a new regression test
asserting the prepared effect is field-identical to today's for every step of a
corpus sweep of sequential abilities.

### 5.4 `EffectOwner` writes are deep clones

`EffectOwner` fields are `Option<AbilityEffect>`
(`ability/resolver.rs:121-135`). The **accessors** were already fixed to borrow
(combined doc §4 item 3) — but the **writes** are still ~32-alloc deep clones:

| site | code |
| --- | --- |
| `ability/effects/look.rs:193, 686, 1386, 1498, 1507` | `self.owner.executing = Some(effect.clone());` |
| `ability/effects/look.rs:962` | `self.owner.parent = Some(effect.clone());` |
| `ability/effects/selection.rs:215` | `self.owner.executing = Some(effect.clone());` |
| `ability/resolver.rs:810` | `self.owner.of_pending_choice = self.owner.executing.clone();` (`capture_choice_owner`) |

`capture_choice_owner` is on **every** choice-offer path, so any look/select/choose
ability pays it every time.

**The principled fix is provenance, not a cheaper copy.** Store the
queue-entry/pending-action index + step index the effect came from and resolve
through the existing chain. `Option<*const AbilityEffect>` is unsound;
`Option<Box<AbilityEffect>>` does not help because every write site holds
`&AbilityEffect` and cannot hand over ownership.

**Scale:** ~32–64 allocations per look/select ability.
**Risk:** **high**, and the doc at `resolver.rs:102-119` records an entire bug
class caused by these fields being ambiguous — do not make them *more*
ambiguous.
**Outcomes:** **this one CAN change outcomes.** `owning_effect`'s doc
(`resolver.rs:296-319`) records that several call sites used to disagree about
which effect a pending choice belonged to, and that was a real bug.
**Gate:** build an exhaustive corpus test that, for every
`SelectCard`/`SelectTarget` prompt the corpus can produce, records
`(owning_effect, answering_effect, target, action, description)` before and
after and requires identical records — then `total_actions`.
**Start instead with the cheap audit:** check whether each `owner.executing = …`
write is read at all before the next write. Several executors write and then
re-enter dispatch, which overwrites it. Dead writes are free to remove.

### 5.5 Unconditional cost clone, even with no `modify_cost` anywhere

`ability/resolver.rs:1389-1406`:

```rust
fn apply_modify_cost_to_ability_cost(&self, gs, cost: &AbilityEffect, ability: &Ability) -> AbilityEffect {
    let mut cost = cost.clone();                    // ~32 allocs, ALWAYS
    if let Some(ref effect) = ability.effect {
        if let Some(mod_cost) = util::find_modify_cost(effect, None, None) { … }
    }
    cost
}
```

Called from `pay_ability_cost` (`resolver.rs:1102`) on **every ability with a
cost**. The clone is needed only inside the `per_unit && subtract` branch.
`find_modify_cost` (`util.rs:122`) is a cheap recursive scan.

**Fix:** look up `mod_cost` **first**, and only then `let mut cost = cost.clone();`.
**Scale:** ~32 allocations → 0 for every ability without a per-unit hand cost
reduction (the vast majority). **Risk:** trivial. **Outcomes:** cannot change.

### 5.6 `pay_cost_move_cards` deep-clones the cost it was handed

`ability/cost/handlers.rs:832-838`:

```rust
// Execute from a FULL clone of the parsed cost. The previous hand-picked
// 7-field copy dropped every card-filter field …
let effect = cost.clone();
self.execute_move_cards(gs, &effect)
```

The comment explains *why* the clone exists — correctness: a prior hand-picked
copy silently dropped `exclude_characters`/`group_names`/`characters`. That
reasoning is about **what is copied**, not about **copying at all**.

**Fix:** change `execute_move_cards(&mut self, gs, effect: &AbilityEffect)` to
take it by reference. It is a synchronous call, so the borrow lives only for the
call. **Check first** whether it stashes a copy in `self.awaiting` — if so, the
clone is load-bearing *there* and can be narrowed to that one site.

**Scale:** ~4 allocations per `move_cards` cost payment. **Risk:** low if it
takes `&`. **Outcomes:** cannot change — by-reference vs by-clone is
semantically identical. This is also a cheap partial answer to the combined
doc's deferred `Option<Box<CompoundBranch>>` item, without the 60-site
wire-format diff, because this site is not on the serialisation path.

### 5.7 `execute_custom` clones twice for `placement_order: any_order`

`ability/effects/dispatch.rs:237-248`:

```rust
if effect.placement_order_any() == Some(PlacementOrder::AnyOrder) {
    let mut routed = effect.clone();               // ~32
    routed.action = …MoveCards;
    …
    self.owner.executing = Some(routed.clone());   // ~32 MORE
    return self.execute_move_cards(gs, &routed);
}
```

**Fix:** `mem::take`/`replace` semantics, or drop the `owner.executing` write if
`execute_move_cards` sets it itself. **Scale:** ~64 → ~32 (or 0).
**Risk:** low, but the `owner.executing` write is observable via
`owning_effect()`, so removing it needs the §5.4 corpus gate.

### 5.8 The remaining `effect.clone()` inventory

All at the same ~32-allocation price:

| site | note |
| --- | --- |
| `ability/condition/conditional_on.rs:40, 135` | |
| `ability/cost/handlers.rs:1399` | `vec![alt_effect.clone()]` → a `Vec<AbilityEffect>` pending action |
| `ability/effects/ability_ops.rs:110, 364` | `set_pending_actions(vec![effect.clone()])` |
| `ability/effects/heart.rs:94` | |
| `ability/effects/position.rs:287, 297, 301, 455` | 3 clones for a `both` target plus a saved copy |
| `ability/effects/replacement.rs:66, 129` | |
| `ability/effects/target.rs:120, 130, 141, 347` | `handle_both_targets` clones 2–3× and saves a 4th |
| `ability/choice.rs:94` | `let mut tail: Vec<AbilityEffect> = remaining.to_vec();` |

**That last one is free.** In `requeue_remaining_after_choice`, called on every
resume that leaves pending actions: `pending` is an owned `Vec` the caller drops
immediately after (`choice.rs:112-122`), so this can be `into_iter()`/`split_off`
— **zero clones, trivially**. **Scale:** ~32 × (remaining steps) per
sequential-with-choice resume. **Risk:** trivial. **Outcomes:** cannot change.

### 5.9 `ability_ops` rebuilds the whole `Ability` field by field

`ability/effects/ability_ops.rs:696-703`:

```rust
full_text: ability.full_text.clone(),
triggerless_text: ability.triggerless_text.clone(),
triggers: ability.triggers.clone(),
cost: ability.cost.clone(),
effect: ability.effect.clone(),        // <-- ~32 allocs
keywords: ability.keywords.clone(),
```

`gain_ability` is one of the most frequently dispatched actions.

**Fix:** if an `&Ability` is in hand, clone it wholesale — the compiler already
does per-field `Clone`, so this is a clarity change — or better, take
`Arc<Ability>` and bump. **Scale:** ~32 → ~1 (or 0). **Risk:** low.

Same file: **`:210`** `Some(c) => (c.card_no.to_string(), c.name.to_string(), c.abilities.clone())`
copies the whole `Vec<AbilityRef>`; **`:227`** `format!("{}_{}", card_no, ability.full_text)`
builds a full-text `String`; **`:134`** `core::mem::take(&mut self.selection.cards).to_vec()`
is a `SmallVec` → `Vec` spill.

### 5.10 A deep clone purely to dodge the borrow checker

`core/game_state/modifiers.rs:2127-2153`:

```rust
let mut entries: Vec<(i16, usize, crate::card::AbilityEffect)> = Vec::new();
for (cid, player_idx, effect) in live_player_zones.iter().flat_map(…) {
    …
    entries.push((*cid, player_idx, (**effect).clone()));    // line 2153
}
for (card_id, player_idx, effect) in &entries { … }
```

A full `AbilityEffect` deep clone — ~4–6 allocations, ~1.7 KB — **per
success-zone constant entry**, to release a `&self.card_database` borrow that
was never blocking: `card_db` is already a local `Arc` at `modifiers.rs:589`.

**Fix:** borrow `&*card_db` and iterate `card.abilities` directly, collecting
only `(i16, usize)` pairs; re-resolve via the existing
`resolve_constant_ability(card_id, ability_idx)` helper that
`recalculate_constants` already uses at `:631`. The `Vec` is ≤6 entries and has
the same shape as `scratch_constant_effect_ids`, which should be reused here
(`modifiers.rs:1313-1317` already has
`collect_reusable_constant_stage_effect_ids`).

**Scale:** ~4–6 allocations per success-zone constant per `recalculate_constants`
×2 per action. Two live cards carrying score constants is **8–12
allocations/action** — on the order of 10% of the budget.
**Risk:** low, and the reasoning is provable — a `clone()` of a value that is only
read is semantically identical, *unless* the original is mutated during the loop,
and the loop body only reads `effect` and writes `self.mods`.
**Outcomes:** none, but this touches live score, the most outcome-sensitive number
in the game. **Gate:** full suite + 1,000-game AI self-play requiring
**bit-identical** results, not just equal action counts.

### 5.11 `Option<Box<CompoundBranch>>` — still deferred, and the cheaper subset of it

The combined doc §5 lists converting `compound` from an unconditional `Box` to
`Option<Box<_>>` as "do it deliberately, not in passing": ~60 sites across 15
files, and a `serde(flatten)` change on the card-data wire format which
`test_blob_matches_json` would catch.

**That assessment still stands, and this catalogue does not overturn it.** But
note that §5.6, §5.3 and §5.2 each remove clone *sites* without touching the
type — a meaningful fraction of the benefit at a fraction of the diff, and
wire-transparent. Do those first; the type change remains available later with
a smaller remaining justification.

### 5.12 `Option<Box<CompoundBranch>>` is not the whole story: `EffectFilter` is 73% fat pointers

The larger, harder version of the same idea is representation rather than
optionality — see §6.4. `compound` is one `Box`; the 164-field `EffectFilter`
with 56 `Option<ArcStr>` is 896 bytes of the ~1,234.

---

## 6. Category D — representation, data structures, and layout

### 6.1 `compat::HashMap` — enumerate the maps, then pick the right ones

`compat.rs:82-84` re-exports `hashbrown::{HashMap, HashSet}`. The engine is
careful here already: zone storage is **entirely inline** and
zero-allocation in steady state (`core/zones.rs`, `core/player.rs:91-113`):

| Zone | Type | Inline capacity |
| --- | --- | --- |
| `Stage.stage` | `[i16; 3]` | — |
| `Stage.under_cards` | `[SmallVec<[i16; 4]>; 3]` | 12 |
| `Hand.cards` | `SmallVec<[i16; 7]>` | 7 |
| `MainDeck.cards` | `SmallVec<[i16; 64]>` | 64 |
| `EnergyDeck.cards` | `SmallVec<[i16; 16]>` | 16 |
| `EnergyZone.cards` | `SmallVec<[i16; 8]>` | 8 + `active_count: u8` |
| `Waitroom.cards` | `SmallVec<[i16; 30]>` | 30 |
| `LiveCardZone` / `SuccessLiveCardZone` | `SmallVec<[i16; 3]>` | 3 each |

~146 `i16` = 292 bytes per player, all inline. **This is genuinely well done and
should be left alone.** The problem is the *maps*, not the zones.

`GameModifiers` (`core/game_modifiers.rs:177-251`) has **22 maps, every one
bounded by 6–14 entries on a real board**. A `HashMap` is 48 bytes of struct
plus a heap table; for 6 entries the table is ~70 bytes allocated on first
insert — so ~118 bytes of storage to hold 6 bytes of information, and every
read pays a **SipHash-1-3 of an `i16`** plus a dependent load.

**A1 — six maps keyed by a dense card id should be arrays.** `orientation_modifiers`
(exactly 6), `blade_type_modifiers` (6), `heart_override` (6), `heart_copy` (6),
`heart_color_multiplier` (6), `score_modifiers` (3). All `HashMap<i16, V>`.
Replace with `Vec<V>` or `[Option<V>; 64]` and reads become `arr[cid as usize]` —
one load, no hash, no dependent miss.

**Why this is safe where §1.2's reversal was not:** the reversal changed `Hash`
and therefore **iteration order**. These six are read by `get(&cid)` and iterated
only in `game/display.rs` (for the UI breakdown). None feeds an
order-dependent accumulation. **But verify per field, do not assume:** grep each
field name and confirm no `for (_, v) in map` in `ability/`, `core/`, or `turn/`.

**A2 — the nested map is worse than the outer one.** `heart_modifiers:
HashMap<i16, HashMap<HeartColor, ModifierEntry>>` costs **two hashes and two
table probes** to read one heart value, and it is read in
`Stage::effective_member_hearts` (`zones.rs:490-540`) for **every heart colour
of every stage member on every `total_hearts()` call**. `HeartColor` is a
fieldless enum with 8 variants (`core/types.rs:83-110`) — so the inner map should
be a `[ModifierEntry; 8]` inline array. This is the single most-read nested map
in the engine and the easiest safe win in this subsection.

**A3 — do NOT convert these, despite the cardinality looking right:**

- `constant_heart_bonuses` **inner** map — string-keyed and iterated
  (`modifiers.rs:511, 516`). §1.2. It *can* become
  `HashMap<i16, Vec<(HeartColor, i16)>>` because `parse_heart_color`
  (`modifiers.rs:1502-1523`) normalises every string to one of 8 values, so the
  inner map has at most 8 distinct keys and the iteration is an order-independent
  per-colour sum. **Gate it.**
- `exp_heart` inner map — same reason.
- `turn_limited_abilities_used: HashMap<(i16,usize,u8), u8>` — tuple key,
  genuinely unbounded.
- `gained_card_abilities` — `Ability` is a big struct; genuinely sparse.
- `state_snapshot_before_change: Option<HashMap<i16, Option<CardOrientation>>>`
  (`change_state.rs:610, 690`) — ≤3 entries, written once, read once.
  `Vec<(i16, Option<CardOrientation>)>` is strictly better. Small win, zero risk.

**A4 — `local_non_stackable: HashSet<String>`** (`modifiers.rs:2121`) is created
**fresh per `recalculate_constants`**, holds only the `effect.text` of the ≤3
live-zone cards being evaluated, and is used purely as a set
(`:2128, 2153, 2164`). A `Vec<&str>` with linear `contains` is both
allocation-free and faster. It also **duplicates** `GameState::local_non_stackable:
HashMap<i16, u8>` (`modifiers.rs:1655`), which already exists and is keyed by the
thing you actually need.

**Scale:** `HashMap<i16,V>` read ~10–15 ns vs `[Option<V>;64]` ~1–2 ns. The read
rate is dominated by `orientation_modifiers` (in `total_blades`, five effect
handlers, `ConditionContext`) and `cost_modifiers` (in `total_cost`, for every
hand card during action generation). Estimate 1.5–3% of wall time, near zero risk.
**Measure separately** for `generate_possible_actions` and `recalculate_constants`
— they have different map mixes, and one number will hide the other.

### 6.2 The `-1` sentinel is the zone representation's one wart, and it costs allocations

`Stage.stage: [i16; 3]` with `-1` for empty. `util::zone_cards` (`util.rs:2066-2088`)
returns the raw `&[i16]` **including the `-1`s** for `Zone::Stage`, and then
`zone_card_ids_occupied` allocates a filtered copy:

```rust
pub fn zone_card_ids_occupied(player: &Player, zone: &str) -> Vec<i16> {
    let mut ids = zone_card_ids(player, zone);                    // allocation 1
    if Zone::from_str(zone) == Some(Zone::Stage) { ids.retain(|&id| id != -1); }
    ids
}
pub fn zone_card_ids(player: &Player, zone: &str) -> Vec<i16> { zone_cards(player, zone).to_vec() }   // allocation 2
```

Two allocations to filter a slice of 3. 9 call sites of `zone_card_ids`, 4 of
`count_in_zone`, 9 of `resolve_indices_to_ids`.

**Fix:** an iterator-returning sibling, and convert the callers:

```rust
pub fn zone_cards_occupied<'a>(p: &'a Player, zone: &str) -> impl Iterator<Item = i16> + 'a {
    let s = zone_cards(p, zone);
    let strip = Zone::from_str(zone) == Some(Zone::Stage);
    s.iter().copied().filter(move |&id| !(strip && id == -1))
}
```

The `-1` filter is preserved exactly, **including** the "raw 3 entries including
sentinels" escape hatch that `zone_card_ids`' own doc comment
(`util.rs:2095-2102`) calls out.
**Scale:** 2 allocations per zone query. **Risk:** trivial. **Outcomes:** cannot change.

### 6.3 `SmallVec` → `Vec` spills on hot paths

208 `to_vec()` sites in `engine/src`. The ones that matter:

**S1 — `counted_by_any().to_vec()`, cloned then dropped (`modifiers.rs:230-233`):**

```rust
let seen_by = effect.counted_by_any()          // -> Option<&Vec<String>>, no alloc
    .map(|v| v.to_vec())                       // 1 Vec + N String allocs
    .unwrap_or_default();
// …five lines later…
if !seen_by.is_empty() && !seen_by.iter().any(|g| card_matches_group_str(…)) { … }
```

The deep clone exists for no reason — `counted_by_any()` borrows from `effect`,
which borrows `card_db` (not `self`), so the borrow lives for the whole block.
**Fix:** keep the borrow.
**Reachability note:** `weighted_zone_count` (`modifiers.rs:256`) is **dead code**,
but `count_as_multiple_weight` (`modifiers.rs:178`) is live and is called from
`ability/condition/card.rs:2080` **once per card in the zone** — so this is
1+N allocations per card per count.
**Scale:** small, pure waste. **Risk:** trivial. **Outcomes:** cannot change.

**S2 — `get_zone_cards` spills a 3-element inline `SmallVec` (`condition/state.rs:246-253`):**

```rust
let cards: Vec<i16> = match *zone_name {
    "success_live_card_zone" => player.success_live_card_zone.cards.iter().copied().collect(),
    "live_card_zone" => player.live_card_zone.cards.to_vec(),       // 3 elements → heap
    _ => continue,
};
```

**Fix:** `let cards: &[i16] = &player.live_card_zone.cards;` — derefs, no copy.
Same at `state.rs:253`, and at `state.rs:868-869` (`recently_moved_cards`,
`entry_trigger_snapshot`, both `SmallVec<[i16;4]>`), `state.rs:122-124`,
`util.rs:2092`, `change_state.rs:53`, `move/mod.rs:723/1585/1893`,
`target.rs:39/341`, `abilities.rs:2575-2581`.
**Why it matters more than its size:** this is the substrate every
`count_cards_in_zone` / zone-counting condition runs through, and the combined
doc establishes that counting paths dominate `recalculate_constants`.
**Scale:** 1 allocation per zone-condition evaluation, on data never on the heap.
**Risk:** trivial.

**S3 — two `Vec<i16>` per `SelectCard` prompt (`abilities.rs:2560-2584`).** Copies a
whole zone (up to 64 for `Deck`) into a fresh `Vec`, then copies it *again* into
`filtered` when `filtered_indices` is `Some`.

**Fix:** iterate `zone_cards(player, zone)` directly and collect only `filtered`.
**⚠ Latent-bug warning:** the `Stage` arm here filters `-1` but `zone_cards` does
**not**. Do not "simplify" this to `zone_cards` wholesale without keeping the
filter — and add a comment recording the difference, because a future reader
will not spot it.
**Scale:** 2 allocations per `SelectCard` — the most common choice type.
**Risk:** medium, precisely because of that discrepancy.

**S4 — `live_card_zone.cards.to_vec()` per success-zone constant
(`modifiers.rs:2432`, `resource.rs:1063-1065`, `ability_ops.rs:233, 341`,
`change_state.rs:53`).** Borrow the slice. **Zero risk.**

### 6.4 `EffectFilter` is 73% fat pointers — interning the closed vocabulary

The single largest *representation* win available, and the hardest.

56 of `EffectFilter`'s 164 fields are `Option<ArcStr>` — **896 of ~1,234 bytes**.
Reading one costs: load the 16-byte fat pointer from the `EffectFilter` (itself
behind an `Arc<EffectKind>`, so already one deref in), then load the `Arc<str>`
header at `*ptr`, then compare bytes. An interned `u16` handle would be: load 2
bytes from a line already in cache, then one indexed load from a small hot
`&'static [&'static str]`.

Those 56 fields draw from a **closed vocabulary**: locations, targets,
positions, phases, durations, trigger kinds, operators, heart colours, group
names, unit names, series names, negation words. Realistically 150–250 distinct
values across the whole 2,280-card corpus.

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct InternedStr(u16);              // 2 bytes
impl InternedStr { pub fn as_str(self) -> &'static str { TABLE.get()[self.0 as usize] } }
```

with `TABLE` a `OnceLock<Vec<&'static str>>` on std and a baked slice on
`no_std`; the bytecode decoder (`ability/vm.rs`) interns on decode.

**Struct-size effect:** 56 × 14 = **784 bytes saved per `EffectFilter`**, a 63%
reduction. At ~2,280 cards × ~1 filter that is a ~3.9 MB reduction *and* a
materially smaller cache footprint.

**⚠ Note the framing.** The combined doc §8 argues RAM is not a problem (~2.4 MB
resident) and is right. But that argument is about *allocation pressure*, not
about **cache residency** — and a flat profile of thousands of cheap derivations
is precisely a cache-residency profile. This is a footprint win, not an
allocation win, and it will not show up in `alloc calls`. That is a reason to
believe it will help, not a reason to expect the allocation gate to confirm it.

**Where this would be WRONG — all four must be respected:**

1. **Serde.** `Option<ArcStr>` serialises transparently as a string;
   `Option<InternedStr>` needs hand-written `Serialize`/`Deserialize` that
   resolve through `TABLE`. `serde_support` is on by default and `EffectFilter`
   crosses the JSON boundary in **both** directions (card data in, `describe` /
   arena out). The wire bytes must be byte-identical or every web/arena test
   breaks. `test_blob_matches_json` deep-compares all 2,280 cards — it will catch
   a mistake, which is good, but it means the diff is large.
2. **Open-ended fields must stay `ArcStr`.** `text`, `full_text`, `unit`,
   `series`, `name`, `triggers` (comma-joined multi-token), `parenthetical`,
   `identities` (arbitrary group names), `card_names` (arbitrary character names),
   `answers`. These are not a closed vocabulary. Only intern the enum-like ones.
3. **Identity-observing code.** `ArcStr: PartialEq` compares **contents**
   (`core/types.rs:54-64`). Interning makes `Eq` a `u16` compare — the same
   answer for a genuinely closed vocabulary, but a **different** answer the moment
   two equal strings get different handles (e.g. one built by `Arc::from` at
   runtime and one from the table). The intern function must be the *only*
   construction path and must **panic or fallible** rather than silently minting
   a duplicate handle for an unseen string. This is a code-review rule, not a
   type-system rule.
4. **Iteration order.** Any `HashMap` keyed by an interned field changes iteration
   order versus one keyed by `ArcStr`. §1.2.

**Scale:** 3–6% of wall time (estimate). **Effort:** **large**.
**Gate:** `total_actions` held fixed. If interning is genuinely
content-preserving, `total_actions` *cannot* move. If it moves, an identity leak
exists and the change is wrong.

### 6.5 `AbilityRef::resolve()` returns an owned `Arc` from 50 tight-loop call sites

`ability/ability_store.rs:45-62`:

```rust
pub fn resolve(&self) -> Arc<Ability> {
    #[cfg(not(target_has_atomic = "ptr"))]
    { *self.decoded_slots()[self.ability_index as usize].get().expect("ability bytes not decoded") }
    #[cfg(target_has_atomic = "ptr")]
    { Arc::new(self.decode()) }
}
```

On std the decode is cached, so a resolve is an index plus an `OnceLock::get()`
plus an `Arc::clone` — **two atomic RMWs on a contended cache line**, per call.
There are **50 `.resolve()` sites**, many inside loops:

| site | loop bound | per iteration |
| --- | --- | --- |
| `modifiers.rs:419-420` | 6 stage × ~3 abilities × 2 calls/action | 1 resolve |
| `modifiers.rs:344-353` | ≤6 stage cards | 1 each |
| `modifiers.rs:1379` | ≤14 stage+hand × ~3 abilities × 2 | 1 each |
| `modifiers.rs:1345-1353` | 6 × ~3 | 1 each |
| `modifiers.rs:202-210` | per card per count | walks `abilities` |
| `ability/util.rs:709-717` | **per card per group-name miss** (§7.1) | 1 per ability |
| `ability/util.rs:894-910` | per card | 1 per ability |
| `ability/condition/card.rs:336, 3358, 3404` | per card per filter | 1 each |
| `core/game_state/abilities.rs:158-168` | per trigger candidate in the enqueue loop | 1 (to read a `u8`) |

**Order of magnitude:** a realistic mid-game board resolves ~90–150 abilities per
`recalculate_constants` × 2, so **~200–300 atomic RMWs per action** on 2–3 shared
cache lines.

**Fix (small, high leverage):** add a borrowed accessor and use it wherever only
a field is read:

```rust
#[inline]
pub fn peek(&self) -> Option<&Ability> { self.decoded_slots()[…].get().map(|a| &**a) }
```

`ability_has_remaining_uses` (`abilities.rs:158-168`) takes a full `Arc` clone
just to read `use_limit: u8`. `card_matches_group_str`'s `SetCardIdentity` arm
takes one per ability per card per group-name miss.

**Scale:** ~0.5% single-threaded; **2–3% at `--jobs 12`**, where the shared-line
penalty is 4× (§14). **Effort:** small. **Outcomes:** cannot change — it is a
read-only accessor.

### 6.6 `ArcStr::from(&str)` allocates twice

`core/types.rs:36-40`:

```rust
impl From<&str> for ArcStr {
    fn from(s: &str) -> Self { ArcStr(Arc::from(s.to_string())) }   // String, then Arc<str>
}
```

`Arc::<str>::from(String)` allocates a new `Arc` header and copies the bytes, then
frees the `String`. `Arc::<str>::from(s)` does it in one. **Two allocations and
two memcpys where one of each suffices.**

This runs once per `ArcStr` field per ability at decode time, so the absolute
number is small — **but the allocation counter in the doc's tables is a
whole-process figure that includes setup, so this inflates the very number people
optimise against.** One line: `ArcStr(Arc::from(s))`. Zero risk. Do it, but do
not expect a benchmark move.

### 6.7 `AbilityQueueEntry` is ~500 bytes inline, and `entries: Vec<…>` memcpy's it

`ability/queue.rs:80-152` — the entry carries `trigger_type: String`, `player_id:
String`, `card_no`, `snapshot_movements: SmallVec<[MovementEvent; 4]>`,
`condition_cache: SmallVec<[(String, bool); 2]>`, plus more. `entries` is a
`Vec<AbilityQueueEntry>`, so every growth realloc memcpy's ~500 bytes per entry.

Two changes, both wire-transparency questions to check:

- `player_id: String` → `PlayerId` (`Arc<str>`). This is the **source** of the
  `String` allocations in §9.1 — `ability_master_id()`'s
  `.map(|e| e.player_id.clone())` is a heap allocation, not an atomic, purely
  because the field is a `String` rather than a `PlayerId`. Fixing the field
  makes the §9.1 fix a `mem::cmp` instead of a `String::clone`. **Check that
  `PlayerId` serialises as a plain string** (the arena protocol round-trips
  `player_id` as a JSON string) before doing this.
- `MovementEvent.cause_player_id: String` → `PlayerId`. `MovementEvent` is 33–40
  bytes **with a `String` in it** (`core/types.rs:955-970`), so
  `SmallVec<[MovementEvent; 8]>` is 336 bytes inline and **every non-empty event
  is one heap allocation**. A 4-card batch move = 4 allocations per movement
  batch per action. Every construction site passes one of two literals, and
  `PlayerId` compares and displays identically.
  `GameState.turn_movements`, `turn_area_movements` and `batch_movements` are all
  affected.

**Scale:** 1–3 per queue operation, plus the `MovementEvent` savings.
**Risk:** check the `serde` shape of both. **Outcomes:** cannot change — `PlayerId`
compares by content.

### 6.8 `Card::box` is an unconditional `Box` for a ~100-byte scalar struct

`core/card.rs:426-430`:

```rust
card_type: r.card_type,                 // 1 B
ability: r.ability,                     // ArcStr, 16 B
box: Some(Box::new(BaseCard { … })),    // 1 heap alloc, always
```

`BaseCard` is ~100 bytes of scalars and smallvecs, always wrapped in `Some`, and
read through `self.box.as_ref().unwrap()` at every `is_member()` / `is_live()` /
`is_energy()` / `get_score()` call. So **2,280 heap allocations at load**, plus a
pointer hop on the most-read accessor path in the engine.

**Fix:** inline it (`pub box: BaseCard`, dropping the `Option`).
**Scale:** 2,280 allocations at load (once) + one pointer hop per card-type check
(46 sites, see §13.2). **Risk:** `box` is `pub` and read in ~20 places; mechanical.
Good news: `card_db.rs` builds `Card` by hand under the `card_db` feature, so a
missed site is a **compile error, not a silent bug**.

### 6.9 `ConditionContext` memcpy's ~800 bytes of snapshot per construction

`ability/condition/state.rs:103-140` (`new_with_self`) clones **11 `SmallVec`s**:

```
recently_moved_cards      SmallVec<[i16; 4]>
revealed_cards            SmallVec<[i16; 8]>
revealed_card_meta        SmallVec<[RevealedCardMeta; 8]>     // 56 B each = 464 B
selected_cards            SmallVec<[i16; 4]>
selected_cards_info       SmallVec<[i16; 4]>
looked_at_cards           SmallVec<[i16; 4]>
awaited_spawns            SmallVec<[AwaitedSpawn; 2]>
turn_movements            SmallVec<[MovementEvent; 4]>
turn_area_movements       SmallVec<[MovementEvent; 2]>
batch_movements           SmallVec<[MovementEvent; 4]>
card_appearances_this_turn SmallVec<[(i16, String, String); 4]>
```

**≈800 bytes of struct memcpy per construction, before a single field is read** —
and most conditions read 1–2 of the 11. It is constructed once per constant
entry per `recalculate_constants` (`modifiers.rs:653`) and once per card per count
(`modifiers.rs:220`), so ~12 per call, ~24 per action, ~19 KB of memcpy per
action.

**Fix, in order of preference:** (1) pass `&GameState` and read through where the
lifetime allows — most consumers only read and `evaluate_condition` takes `&self`;
check first that no consumer mutates `GameState` while holding a context.
(2) Split into a hot `ConditionContext` (3–4 fields) and a
`ConditionContextFull` built on demand. A per-field `OnceCell` would cost more in
branches than it saves.

**Risk: MEDIUM — the highest-risk item in this document.** `ContextSnapshot` is
explicitly a per-resolution snapshot; if any effect path mutates `GameState`
mid-resolution and re-reads the context, removing the copy changes behaviour. And
`state.rs:121` notes that some conditions (e.g. `moves`) already *"consult the
global `gs.batch_movements`"* — i.e. some code paths already bypass the snapshot.
**Gate:** full suite + 1,000-game AI self-play with state-trace comparison. Do it
**after** the zero-risk items have banked.

### 6.10 `Player::contains_card` / `owner_of_card` are O(54) and O(108) linear scans

`core/player.rs:651-658` — six zones: stage(3) + hand(7) + live(3) + success(3) +
energy(8) + waitroom(30) = **up to 54 comparisons**, dominated by the waitroom.
It does *not* check `main_deck`, `energy_deck`, or the exclusion zone.

`GameState::owner_of_card` (`abilities.rs:2723-2731`) — both players, same six
zones: **up to 108 comparisons.** (Its doc comment records that it was introduced
as the single source of truth because `resolve_condition_player` had
*re-implemented* this scan with a **divergent zone list** — so the shape is
already known to be bug-prone.)

**Would a bitset help?** Mathematically yes: 0 vs 1 cache line. But the honest
recommendation is **do not, and here is the reason rather than a shrug.** The
working set is ~292 bytes of `i16` per player — it is already L1-resident. A
bitset adds a *representation* to maintain on every zone mutation (draw, discard,
move, position change, reveal, trigger-spawned card) and every one of those is a
correctness risk, in exchange for converting L1 hits into L1 hits. The
allocation-count gate cannot even see the win. **Spend the effort on §4.1
(reducing the number of calls) rather than on making each call cheaper.**

Where it *does* pay: the fix for §4.12 — probing the **stage first** — turns 12
large-`Vec` scans into two 3-element scans without any new representation. That
is strictly better than a bitset here.

### 6.11 `filter_subset()` — ~50 dependent loads to assemble a flat struct

`core/card.rs:2377-2434` calls ~25 `*_any()` accessors. Every
`filter_*_getter!` expands to `self.kind.as_deref()?.filter()?.field.as_deref()`
— **two dependent loads each**, and `kind` is an `Arc<EffectKind>` (a heap
indirection). So ~50 dependent loads to build a struct of borrowed pointers.

**Callers on the hot path:** `ability/cost/handlers.rs:217` (`validate_leaf_cost_for`,
MoveCards), `:326` (pay/skip gate), `:434`, `:709` (hand / generic zone cost),
`:57`. That is **4–6 `filter_subset()` calls per costed ability resolution**, i.e.
200–300 dependent loads.

The same shape appears in `CardFilter::from_effect` (`util.rs:1711-1780`), which
does **35 independent `self.kind.as_deref()?.filter()?` traversals** — all reading
fields of the *same* `EffectFilter`.

**Fix:** bind once, read many.

```rust
pub fn filter_subset(&self) -> crate::ability::util::CardFilter<'_> {
    let Some(f) = self.kind.as_deref().and_then(EffectKind::filter) else { return CardFilter::default(); };
    // read f.card_type, f.cost_limit, f.characters, …
}
```

This needs non-`_any` siblings on the `filter_*_getter!` macros
(`card.rs:1772-1830`) reading off an `&EffectFilter` — ~22 mechanical macro
invocations — and touches the 3 `filter_subset` definitions (`card.rs:885`, `:2377`,
`:3794`).

**Scale:** zero allocations removed; a real reduction in dependent-load latency on
a very hot path. **Its real value is diagnostic:** right now this is invisible in
an allocation profile, and removing it makes the next round of profiling readable.
**Risk:** medium effort, zero correctness risk. **Outcomes:** cannot change — pure
load reassociation.

### 6.12 `CardFilter::matches` runs up to ten independent hash probes for one card

`ability/util.rs:1585-1690`. The body chains `check_exclude_names`,
`card_matches_type`, `check_group` (which itself calls `card_matches_group_str`
once per group in `groups` and once per entry in `exclude_group_names`),
`card_matches_cost_limit_op` ×3, `card_matches_characters` ×2,
`check_heart_colors` (× `|heart_colors|`), `card_matches_name_fragments`,
`check_ability_filter`, `check_card_property`. Each of those calls
`db.get_card(id)` — a `HashMap<i16, Card>` probe, SipHash plus a dependent L1
miss.

**A fully-filtered card runs 10–15 independent probes of the same key in one
predicate call.**

The combined doc §5 lists the dense-`Vec` card index and notes 356 `get_card`
call sites. It did not notice that **the probe is repeated up to 15 times within
a single call.** The repetition fix is independent of the index fix, strictly
cheaper, and strictly larger.

**Fix (minimal diff):** `let card = db.get_card(id);` at the top of `matches`,
thread `Option<&Card>` into the check helpers, and add `_card(&Card, …)` variants
of `card_matches_type` / `card_matches_cost_limit_op` /
`card_matches_heart_colors`. Keep the `(db, id, …)` wrappers for the other
callers. **Ten probes become one.**

**Scale:** `matches` runs once per (candidate card × filter application), and
filters are applied repeatedly — `resolve_selection` (`util/selection.rs:97-119`)
→ `get_selection_indices` → `matching_indices`, and then
`resolve_from_stage` (`move/mod.rs:1096-1126`) builds the filter a **second
time** and re-runs `matches` over the whole stage on the `Prompt` arm.
**Risk:** low if the borrow is threaded correctly. **Outcomes:** cannot change.

### 6.13 A dense `Vec` index beside `CardDatabase.cards` (already on the list)

The combined doc §5 already proposes this: card ids are dense and contiguous
(0..2280, assigned by a sequential `next_id`), and `get_card` has 356 call sites,
each a ~7-line hash probe with one guaranteed L1 miss. A `Vec<Option<Card>>` makes
that an index and makes zone-order iteration a prefetch stream.

**Expanded with the material gathered here:** the expected win is low single
digits, and the working set is L3-resident either way, so this is a **layout**
improvement more than a speed one. It is also the natural home for two other items
in this document that want the same thing — the precomputed group bitmask
(§7.2) and the `SuppressAbilityTrigger` mask (§4.10) — because all three are
"materialise an immutable card-derived index at load time" and
`CardDatabase::load_or_create` (`core/card.rs:472-503`) **already builds
`normalized_no_to_id` this way**, with a comment recording why. Landing all three
together is a better-shaped project than any of them alone.

**Gate:** `total_actions` fixed. A `Vec` index does not change iteration order of
anything that iterates the *map*, but it does change the *order* of any future
`Vec` iteration — so if `cards` is ever iterated, that is a §1.2 item.

---

## 7. Category E — scans, early exit, and algorithmic complexity

### 7.1 `get_card_names` allocates 3–4 objects per call, inside the innermost filter loop

**This is the single largest allocation-per-candidate site in the engine, and it
is on every group-filtered selection in the game.**

`core/card.rs:681-692` (verified verbatim):

```rust
pub fn get_card_names(&self, card_id: i16) -> Vec<String> {
    if let Some(card) = self.cards.get(&card_id) {
        Self::normalize_name(&card.name)     // card.rs:667 — String alloc; the "fast path" ALLOCATES
            .replace('＆', "&")               // String alloc — UNCONDITIONAL, even with no '＆'
            .split('&')
            .map(|s| s.to_string())           // 1 String per name part
            .collect()                        // Vec alloc
    } else { Vec::new() }
}
```

**Four heap allocations, minimum, unconditionally.** And `normalize_name`'s
doc comment says *"Avoids allocation when no whitespace is present"* while its
fast path is `return name.to_string();` — it allocates anyway.

Its innermost caller, `ability/util.rs:700` inside `card_matches_group_str`:

```rust
|| card_db.get_card_names(card_id).iter().any(|name| …)
```

This is the **fourth** disjunct, reached whenever the cheaper `unit` / `group` /
whitespace checks miss — i.e. the **miss path, which is the common path** when
scanning a zone for a specific group.

`card_matches_group_str` has **67 call sites**. And
`CardFilter::check_group` (`util.rs:1393-1417`) calls it **once per group in
`filter.groups` and once per entry in `filter.exclude_group_names`**. A filter
with 2 accepted groups and 1 excluded group runs it **3× per candidate** = 12
allocations per candidate card, in the `matches` predicate applied to every card
in the zone.

**The amplification, concretely.** `distinct_stage_groups` (`abilities.rs:2786-2806`)
is 5 canonical groups × a 3-slot `any()` = **15 `card_matches_group_str` calls per
invocation**, each potentially 4 allocations = **up to 45 allocations**. It runs at
4 sites (§4.3). A 64-card deck scan with a 2-name group filter is 64 × 2 × 4 =
**512 allocations for one clause**.

**Fix, three layers — do them in this order:**

1. **Make `normalize_name` borrow.** Split into a borrow-returning core plus the
   existing allocating wrapper. `None` when there is no whitespace (the
   overwhelmingly common case) lets hot callers use the borrowed `&str`:

   ```rust
   pub fn normalize_name_ref(name: &str) -> Option<Cow<'_, str>> {
       if !name.bytes().any(|b| b.is_ascii_whitespace()) && !name.contains(char::is_whitespace) { None }
       else { Some(Cow::Owned(name.chars().filter(|c| !c.is_whitespace()).collect())) }
   }
   ```

   34 `normalize_name` sites, several per-card in a loop
   (`core/card.rs:3678-3696` calls it **twice per card**; `:2895` per stage card
   then an O(stage × characters) substring scan).

2. **Make `get_card_names` allocation-free.** All 13 call sites only ever
   *iterate* the result. A borrowed splitter returning
   `impl Iterator<Item = &str>` over whitespace-stripped, `＆`-folded segments
   reproduces the semantics **provided the needle contains no whitespace** (true
   for all `KNOWN_GROUPS`); fall back to the allocating path otherwise.
   The folding must be exact: whitespace removal, `＆` **and** `&` both as
   separators, and the `！`/`µ`/`μ` normalisation that `util.rs:696` and `:700-701`
   apply *after* the split.

3. **Precompute per-card name parts in `CardDatabase`** (see §7.2).

**Scale:** 4 allocations → 0 per group check that reaches the name arm. The
highest-leverage item in this document after §3.
**Risk:** medium — the whitespace and full-width folding is load-bearing (the
`lily white` / `lilywhite` corpus case at `util.rs:650-654`).
**Outcomes:** **can** change if the folding is not byte-identical. A divergence
silently changes which cards a group filter admits — which changes *the game*,
and §1.2 records that this exact class of silent gameplay change has already
bitten this project three times.
**Gate:** an exhaustive corpus test asserting, for every card × every
`KNOWN_GROUPS` entry × every group name in `abilities.json`, that the
optimised predicate equals `get_card_names(id).iter().any(|n| n.contains(g))`.
That test is cheap to write and makes this a zero-risk change. Run the
multi-name-card rules specifically (`q105_*`, `emma bp7-008`, group_reference
tests) plus the full suite with `total_actions` fixed.

### 7.2 Precompute card-derived indexes at load time (the project that unlocks several others)

`CardDatabase` already builds one derived index — `normalized_no_to_id`
(`core/card.rs:488-500`), with a comment calling it an *"R1-adjacent perf fix"*.
Add to it, in the same `load_or_create`, as **side tables in `CardDatabase`
rather than new `Card` fields** (so the wire format and
`test_blob_matches_json` are untouched):

| side table | what it replaces | used by |
| --- | --- | --- |
| `names: HashMap<i16, Box<[ArcStr]>>` | per-call `get_card_names` allocation | `util.rs:700`, `:953`, `:1053`, `core/card.rs:643` |
| `group_mask: HashMap<i16, u8>` over `KNOWN_GROUPS` | the 6-arm `||` chain, for the 5 canonical names | `card_matches_group_str` |
| `identity_aliases: HashMap<i16, SmallVec<[ArcStr; 2]>>` | the `SetCardIdentity` arm of that chain | `util.rs:709-717` |
| `suppress_mask: HashMap<i16, u32>` | `is_trigger_suppressed`'s full board walk | `turn/trigger_scan.rs:132-154` (§4.10) |
| `trigger_mask: HashMap<i16, u32>` | `AbilityRef::resolve()` per group check | `util.rs:710`, `card.rs:336/3358/3404` (§6.5) |
| `cards_by_id: Vec<Option<Card>>` | the 356-site hash probe | `get_card` (§6.13) |

**Why it is provably outcome-neutral:** `CardDatabase` is **immutable during
play** — the only mutation path is `create_copy` (`core/card.rs:462-469`), which
is setup-time only. `cards` / `.insert` / `.create_copy` appear **only** in
`core/card.rs`, nowhere in the turn loop. So these are pure load-time
precomputation of values that are today recomputed per call.

**One caveat that matters:** `create_copy` must copy every new table for the new
id, and any `CardDatabase` constructed outside `load_or_create` (e.g.
`CardDatabase::new()`) must build them empty. A missed site is a **silent wrong
answer**, not a crash — so this is the change that most wants a
`debug_assert`-based invariant check in a `#[cfg(test)]` test.

**Do not skip this because of §7.3.** Four separate implementations of "does
this card match group/type/characters" exist, and a fix to one does **not**
propagate to the others:

| # | Location | Shape |
| --- | --- | --- |
| 1 | `util.rs:679-725` `card_matches_group_str` | canonical, 6-arm `\|\|` chain, falls through to `set_card_identity` |
| 2 | `util.rs:1393-1402` `CardFilter::check_group` | `self.group` then `self.groups`, both via #1 |
| 3 | `util.rs:946-961` `card_matches_characters` | `get_card_names` + `normalize_name` **per name per card** |
| 4 | `core/card.rs:3692-3696` `count_group_cards_in_cards` | `normalize_name` **twice per card**, then #1 |
| 5 | `ability/condition/card/predicates.rs:131-152` | re-implements group-then-type that `CardFilter::matches` already does |

**The perf consequence:** #1/#2 and #3/#4 reach the card database by *different
routes*, so fixing #1 does not fix #3 or #4 and the win is partial. Route all of
them through the same index.

**The correctness consequence, stated and NOT proposed for change:** the `||`
chain at `util.rs:684-717` is six alternatives wide and each has a separate
comment justifying it. A card that matches only under the `set_card_identity` arm
will be found by `card_matches_group_str` but **not** by
`count_group_cards_in_cards`, which only consults `c.group`/`get_card_names`.
That divergence already exists. **Unifying the *semantics* is a gameplay change
and is out of scope** — this catalogue only unifies the *mechanism*.

### 7.3 `card_matches_characters` normalises the filter, per card, per name

`ability/util.rs:946-961`:

```rust
pub fn card_matches_characters(card_db: &CardDatabase, card_id: i16, characters: Option<&[String]>) -> bool {
    match characters {
        Some(names) if !names.is_empty() => {
            let card_names = card_db.get_card_names(card_id);            // 4 allocs
            names.iter().any(|name| {
                let clean_name = CardDatabase::normalize_name(name);     // 1 alloc PER NAME, PER CANDIDATE
                card_names.iter().any(|cn| cn.contains(&clean_name))
            })
        }
        _ => true,
    }
}
```

`clean_name` is loop-invariant in `name` and recomputed for every candidate card.
3 character names × 20 candidates = 60 wasted `String` allocations **plus** 20
`get_card_names` calls (80 allocations).

**Fix:** normalise the `characters` slice **once at filter-construction time**
(hoist into `CardFilter` as `Vec<Cow<str>>`, or normalise in
`filter_from_parts_full`) and reuse §7.1's borrowed splitter.
**Scale:** high on character-filtered effects, which is most live/performance
card text. **Risk:** same as §7.1. **Outcomes:** whitespace normalisation is
load-bearing — corpus test as in §7.1.

Related, same family: `card_matches_name_fragments` (`util.rs:1790-1797`) calls
`normalize_name` on each fragment **per card**, when the fragment normalisation
is loop-invariant across cards. And `check_exclude_names` (`util.rs:1334-1349`)
rebuilds a normalized `String` per card then compares `char` iterators — the file
already has `eq_ignoring_whitespace` (`util.rs:625-635`), whose docstring literally
says it is *"the allocation-free form of the same rule, for a hot path"*.

### 7.4 The 4× trigger scan figure is measured against the wrong model

See §2.1 for the full call-site census and the recommended counter. The short
version: the model says 2–4 scans per play and measured 2.7%; the real rate is
**18–24 scans per turn from the live phase alone**, plus one per card-touching
ability in the main phase. The scan may well be a first-tier cost centre after
all — but that is a **hypothesis until §2.1's counter is run**, and this document
does not claim it as a finding.

### 7.5 `check_invalid_energy_cards` runs a hash probe per energy card, per call

`turn/actions/mod.rs:1791`:

```rust
let mut invalid_indices = Vec::new();
for (i, card_id) in player.energy_zone.cards.iter().enumerate() {
    if !card_db.get_card(*card_id).is_some_and(|c| c.is_energy()) { invalid_indices.push(i); }
}
```

`Vec::new()` does not allocate, so this costs nothing when there are no invalid
cards (the common case) — **but the loop runs a `get_card` hash probe per energy
card** (≤12 × 2 players) *every single time*. That is ~24 probes per
`check_timing`, ×5 `check_timing` per turn, ×2 per performance.

`check_invalid_live_cards` (`actions/mod.rs:1706-1722`) builds a
`Vec<(usize,i16,bool)>` eagerly then returns if empty — bounded by
`MAX_LIVE_CARDS = 3`, so small, but 2 allocations per `check_timing` for nothing
in the overwhelming majority of calls. `SmallVec<[(usize,i16,bool);3]>` costs
nothing to adopt.

**Fix:** combine with §6.12's single-probe-per-card and make both `SmallVec`.
**Scale:** small. **Risk:** trivial. **Outcomes:** cannot change.

### 7.6 `MoveCards` — `Vec::leak` per group-referenced discard move

`ability/effects/move/mod.rs:1293-1325`:

```rust
if !cond_has_grp && effect.group_reference_any() == Some("different_group_names")
    && c.source_str == "discard" {
    let mut stage_groups: SmallVec<[String; 8]> = SmallVec::new();
    for &cid in &player.stage.stage {
        …
        if !stage_groups.contains(&card.group.to_string()) { stage_groups.push(card.group.to_string()); }
        else {
            for known_group in util::KNOWN_GROUPS {                       // <-- inner loop
                if util::card_matches_group_str(card_db, cid, Some(known_group)) {
                    let s = known_group.to_string();
                    if !stage_groups.contains(&s) { stage_groups.push(s); }
                }
            }
        }
    }
    if !stage_groups.is_empty() {
        let groups_vec: Vec<String> = stage_groups.into_iter().collect();
        // This &'static leak persists for the game's lifetime.
        let leaked = Vec::leak(groups_vec);                               // <-- line 1322
        filter.exclude_group_names = Some(leaked);
    }
```

Three problems:

1. **`Vec::leak` per execution**, acknowledged in the comment. Every
   `different_group_names` move from the discard **leaks a `Vec<String>` for the
   life of the process**. The combined doc §7 records "Memory pressure: dead. No
   growth while games run" — which means this leak is rare enough in the tested
   decks not to show, but it is a real leak and it will appear on a deck that
   uses the group-reference pattern heavily.
2. **`stage_groups.contains(&card.group.to_string())` allocates a `String` for
   the comparison** on every stage card, then a *second* one for the push on
   success. `card.group` is an `ArcStr`; compare `&str`.
3. The inner `KNOWN_GROUPS` loop calls `card_matches_group_str` once per known
   group per stage card — with §7.1 unfixed that is
   `|KNOWN_GROUPS| × 4` allocations per stage card, **the worst amplification of
   §7.1 in the codebase**.

**Fix:** (1) take the filter by value, or use a `SmallVec<[String;8]>` owned by
the resolver for the duration of the effect — stop leaking. (2) one-line
comparison change. (3) falls out of §7.1.
**Scale:** low frequency, very high per occurrence.
**Risk:** the leak fix is medium — `exclude_group_names` is
`Option<&'static [String]>`-ish and that lifetime is load-bearing for the borrow
of `c`/`player`. Consider a `Vec<String>` field on the resolver, cleared per
effect. **Outcomes:** cannot change.

### 7.7 `resolve_source_revealed_cards` — O(R×Z) zone scans, then O(R²) `retain`

`ability/effects/move/mod.rs:1592-1611`:

```rust
let owned: Vec<i16> = gs.revealed_cards.iter()
    .filter(|&&cid| {
        player.hand.cards.contains(&cid) || player.waitroom.cards.contains(&cid)
            || player.stage.stage.contains(&cid) || player.stage.under_cards.iter().any(|v| v.contains(&cid))
            || player.energy_zone.cards.contains(&cid) || player.main_deck.cards.contains(&cid)   // up to 64
            || player.energy_deck.cards.contains(&cid) || player.live_card_zone.cards.contains(&cid)
            || player.success_live_card_zone.cards.contains(&cid) || gs.resolution_zone.cards.contains(&cid)
    }).copied().collect();
for &cid in &owned { gs.revealed_cards.retain(|id| *id != cid); }       // O(R) per cid => O(R²)
```

Per revealed card: up to ~120 linear comparisons across ten zones. Then removing
R cards from a pool of R is **O(R²)**.

Also largely redundant: `gs.revealed_cards` holds cards that came from live
cards, so `main_deck.cards.contains(&cid)` is nearly always false — but the code
cannot know that without the index.

**Fix:** single-pass partition. Iterate `revealed_cards` once, test each against
the zones once, push to `owned` or keep — one `O(R×Z)` pass instead of
`O(R×Z + R²)`. **⚠** The single-pass version must preserve the relative order of
the surviving `revealed_cards` deterministically, because
`Choice::SelectCard { filtered_indices }` **indexes into it**.
**Scale:** low-moderate; runs on every revealed-card move during the performance
phase. **Risk:** low. **Outcomes:** cannot change if the surviving order is
byte-identical — verify explicitly.

### 7.8 `resolve_from_revealed_cards` — O(n) `remove` in a loop, O(D) deck scans per card

`ability/effects/move/mod.rs:605-633`:

```rust
let taken: Vec<i16> = matching[..actual_take].iter().rev()
    .map(|&i| { let id = gs.revealed_cards[i]; gs.revealed_cards.remove(i); id })
    .collect();
for &id in &taken {
    if let Some(pos) = gs.player1.main_deck.cards.iter().position(|&c| c == id) { …remove(pos)… }
    else if let Some(pos) = gs.player2.main_deck.cards.iter().position(|&c| c == id) { … }
}
for &id in &taken { /* …the same again for the waitroom… */ }
```

`Vec::remove` is O(len) and shifts, so the first loop is O(take × len). The
second and third are a linear `position` over a `SmallVec<[i16; 64]>` **per taken
card**, against player 1 then player 2 — up to 128 comparisons each, then the
same again for the waitroom.

The deck and waitroom scans are correctness-motivated (*"the card is still in the
deck"*, *"yell cards went to waitroom"*) and cannot simply be deleted — they need
to be made cheap. A per-player `HashSet<i16>` built once, or a reverse index,
beats a `position` scan per card.

**Scale:** low. `taken` is typically 1–5 and the zones ≤64, so this is
microseconds per move on the reveal path, which runs on every performance.
**Risk:** the O(n²) → O(n) rewrite touches **card-removal order**, which is
load-bearing for index-based choices later in the same effect.
**Outcomes:** **can** change if indices shift. Gate carefully — full suite plus a
choice-trace diff.

### 7.9 `set_indices_waited` / `set_indices_active` — O(n·m) plus three allocations

`core/zones.rs:731-765`:

```rust
pub fn set_indices_waited(&mut self, indices: &[usize]) {
    let mut still_active = Vec::with_capacity(self.cards.len());   // alloc 1
    let mut rest = Vec::with_capacity(self.cards.len());           // alloc 2
    for (i, &cid) in self.cards.iter().enumerate() {
        if indices.contains(&i) || i >= active { rest.push(cid); } // O(|indices|) per element
        else { still_active.push(cid); }
    }
    self.cards = still_active.into_iter().collect();               // alloc 3 if it spills
```

`self.cards` is a `SmallVec`. Three allocations, plus `indices.contains(&i)`
inside the element loop. Energy zones hold ~10–20 cards; the doc comment at
`:725-727` already commits to positional semantics.

**Fix:** `SmallVec<[i16; 16]>` for both buffers (the callees are slice-based, so
no signature churn), and replace `indices.contains(&i)` with a
`SmallVec<[bool; 16]>` bitmap.
**Scale:** small, but it runs on every multi-energy payment.
**Risk:** trivial. **Outcomes:** cannot change — the positional semantics are
already committed in the doc comment.

### 7.10 `count_side` counts the whole zone where 69% of clauses ask only "is there one?"

`ability/condition/card.rs:916-951` (verified verbatim):

```rust
let count_side = |zone_cards: &[i16]| -> u8 {
    zone_cards.iter()
        .filter(|&&id| id != -1
            && filter.matches(card_db, id, true)
            && self.check_original_blade_filter(condition, id)
            && self.check_original_heart_filter(condition, id)
            && self.check_heart_type_all_per_card(condition, id))
        .count()
        .u8_count()
};
```

`.count()` forces a **full traversal** even when the answer is decided by the
first match.

**Census of `cards/abilities.json` (1,278 unique abilities, 526 reachable
condition clauses):**

| metric | value |
| --- | --- |
| abilities with an `effect.condition` | 624 / 1278 |
| effects with **both** `condition` and `activation_condition` | 64 |
| `location_condition` clauses | 100 |
| …of which have no `count` and no operator, or operator `>=` | **69 / 100** |
| `card_count_condition` | 117 |
| `comparison_condition` | 111 |
| `group_names` clauses | 167 (32%) |
| `cache: true` clauses | **0** (see §8.4) |
| nesting depth | depth0 = 414, depth1 = 109, depth2 = 3 |

For those 69 clauses, `effective_op` is `>=` (via `member_count_operator`,
`predicates.rs:168-175`) and `thresh` is 1 — the predicate is **monotone**:
`count >= 1` ⟺ `any(...)`.

**Zone sizes make the multiplier concrete:** `location=deck` = 64 cards;
`waitroom`/`discard` = 30; `hand` = 7 inline but unbounded; `energy_zone` = 12;
`stage` = 3. Corpus distribution: 47 clauses at `stage`, 24 at `revealed_cards`,
18 at `success_live_card_zone`, 10 at `discard`, 7 at `hand`, 4 at
`live_card_zone`, 2 at `energy_zone`, 1 at `deck`. So **23 of 100 clauses** gain,
by up to 30–64× on the scan.

**Fix:**

```rust
let monotone_exists = effective_op == Some(">=") && thresh <= 1;
let count_side = |zone_cards: &[i16]| -> u8 {
    if monotone_exists { zone_cards.iter().any(<same predicate>).u8_count() }
    else { zone_cards.iter().filter(<same predicate>).count().u8_count() }
};
```

The `any` closure body must be **byte-identical** to the `filter` body. Note
`check_heart_type_all_per_card` is already `any`-shaped, so there is no ordering
hazard.

**⚠ One observable difference:** under the fast path the `actual` value used by
`push_cond_verdict` (`card.rs:2574`) becomes 0/1 instead of the true count. That
changes the *log string only*. **Gate the fast path on
`!crate::ability::log::debug_enabled()`** so debug sessions keep the exact count.

**Scale:** up to 30–64× shorter scans for ~23% of `location_condition` clauses.
**Risk:** small. **Outcomes:** cannot change — `.any()` and `.count() >= 1` agree
for a monotone predicate.

**Same shape, opposite verdict:** `evaluate_card_count_condition`'s `count_filtered`
(`card.rs:2027-2086`) **genuinely needs the full count** and does not benefit.
Do not apply the fast path there.

### 7.11 `resolve_moved_cards_source` — O(n·m) with a per-card `SmallVec` alloc

`ability/condition/card.rs:1687-1710`:

```rust
let result: SmallVec<[i16; 8]> = event_cards.iter()
    .filter(|&&cid| {
        let tm: SmallVec<[&MovementEvent; 2]> = self.game_state.turn_movements.iter()
            .filter(|m| m.moved_card_id == cid)
            .collect();                       // <-- allocates per cid
        if tm.is_empty() { return true; }
        tm.iter().any(|m| { /* src/cause/dest check */ })
    })
    .copied().collect();
```

`|event_cards| × |turn_movements|` comparisons plus `|event_cards|`
allocations. `turn_movements` grows for the whole turn.

Also note `recently_moved_cards` is itself a **`.clone()` of a `Vec<i16>`** at
`card.rs:1663`, `:1738`, `:3845`, `:4333` — another allocation per call at four
sites.

**Fix:** invert to a single pass over `turn_movements` building a
`SmallVec<[bool; 8]>` parallel to `event_cards` — a card matches if *any* of its
events passes. One pass, O(movements + cards), no per-card allocation.

**⚠** Must preserve the `tm.is_empty() → result` precedence at `:1711-1715`
exactly: a card that moved at all passes when it has no matching event. An
inverted single pass has to reproduce that.
**Scale:** small in absolute terms (8 `movement_condition` clauses in the corpus).
**Risk:** small-medium, for that precedence. **Outcomes:** cannot change if the
precedence holds.

### 7.12 `max_distinct_names` allocates a fresh `Vec` per card in the DP

`ability/util.rs:1087-1137`:

```rust
let mut frontier: Vec<u128> = vec![0u128];
for mask in &option_masks {
    let mut next: Vec<u128> = Vec::with_capacity(frontier.len());   // per card
    for &f in &frontier { next.push(f | bit); }
    prune_dominated(&mut next);       // sort + dedup + Vec::with_capacity + retain + reassign
    frontier = next;
}
```

plus `prune_dominated` allocating a `kept` buffer and a `HashMap<&str, u32>` for
the universe (`util.rs:1060`). Reached from `max_distinct_group_names` →
`count_distinct_member_name_units` → **every `distinct` card-count condition**.

**Fix:** reuse two `Vec<u128>` buffers via `std::mem::swap`; reuse `prune_dominated`'s
`kept`. **The iteration order must be preserved** — `frontier` order feeds
`sort_unstable`, so buffer reuse is safe only if the order is maintained.
**Scale:** 2 allocations per card + a `HashMap`. **Risk:** trivial.
**Outcomes:** cannot change. **Gate:** `total_actions` + a unit test asserting
an identical `DistinctNamesResult` for the >128-universe greedy fallback path.

### 7.13 Small `Vec`s where a fixed array or `SmallVec` will do

- `ability/util.rs:2116-2138` (`count_in_zone`): the `UnderMember` arm
  `collect()`s ≤12 inline `i16`s into a heap `Vec` while the non-`UnderMember` arm
  already passes a borrowed slice. Make `count_matching` generic over
  `IntoIterator<Item = &i16>`.
- `ability/util.rs:2049-2099`: `zone_card_ids` is
  `zone_cards(…).to_vec()` — **always** allocates; `active_energy_indices` is
  `(0..n).collect()` — **always** allocates. Return
  `SmallVec<[i16; 4]>` and `SmallVec<[usize; 8]>`.
- `ability/effects/target.rs:339-342`: `p.stage.stage.to_vec()` for a **fixed
  `[i16; 3]`**; and `:234-242` collects the same 3 elements again. Index the
  array directly, or `SmallVec<[i16; 3]>`.
- `core/game_state/abilities.rs:2560-2584`: see §6.3 S3.
- `ability/effects/move/mod.rs:2042-2161` (`execute_move_cards`): see §10.1.

**Scale:** 1–2 allocations per zone query / per target prompt. **Risk:** trivial.
**Outcomes:** cannot change.

### 7.14 `Arc<str>` comparisons are content comparisons — do not "optimise" them

`PlayerId` is `Arc<str>`, and `p.id == gs.player1.id` is a **2-byte `memcmp`, not a
pointer compare**. It is ~1 ns and only on `condition.rs:366` (`owner_is_active`)
and 4 other sites. **There is nothing to win here** and a pointer-identity
rewrite would be a behaviour change. Recorded so nobody spends a session on it.

The `.id.to_string()` sites (`restriction.rs:121,126`, `change_state.rs:698,700`,
`blade.rs:61`, `restriction.rs:93`, `reyell.rs:195`) do allocate but already carry
a `PlayerId`, so converting them is a small, separate, low-risk cleanup — not a
tier-1 item.

---

## 8. Category F — the condition evaluator

### 8.1 `Condition::common()` is a 21-arm match, and it runs ~15× per node

`core/card.rs:3449-3473`:

```rust
fn common(&self) -> Option<&ConditionCommon> {
    match self {
        Condition::Compound { common, .. } => Some(common.as_ref()),
        // … 21 arms, each Some(common.as_ref())
    }
}
```

**Every single `condition.get_X()` is `self.common().and_then(|c| c.X)`.** So one
accessor call is a **21-way branch on the discriminant followed by a `Box` deref**.

`evaluate_condition` (`ability/condition.rs:493-628`) makes roughly **ten of them
before it even dispatches**:

| stage | line | cost |
| --- | --- | --- |
| `matches!(Unsupported)` | 495 | discriminant load |
| `matches!(Temporal)` + `get_aggregate()` + `get_heart_colors()` + `Zone::from_str(get_location())` | 499–503 | 3 × `common()` + a 30-arm string match |
| `AbDebug::new()` | 512 | free (no_std) / 1 word (std) |
| `buffer_len()` | 515 | thread-local access |
| `matches!(Compound)` | 517 | discriminant |
| `check_phase_gate` | 529 | 2–3 more `common()` |
| **the 21-arm dispatch match** | 533–570 | the actual dispatch |
| `is_plain_location` + 4 more `matches!` + `get_card_property()` ×2 + `get_heart_type()` + `get_location()` | 572–588 | 4 discriminants + 4 `common()` **in the negation guard** |
| `get_ability_filter()` + `matches!(Movement)` | 615–618 | 1 `common()` |
| recursion | 546, 463, 465, 489 | child-by-child |

**Accessor call counts:** `card.rs` **284**, `state.rs` 84, `condition.rs` 62,
`comparison.rs` 62. `Zone::from_str` (a 30-arm string match): `card.rs` 26,
`state.rs` 9, `comparison.rs` 5.

**Multiplication:** 526 clauses × ~15 `common()` calls × ~2 evaluations per
resolution × ~6 resolutions per turn ⇒ order 10⁵ `common()` calls per turn, each a
21-way branch.

**Fix — one borrow, ten reads:**

```rust
let Some(common) = condition.common() else { return false };
// read aggregate, heart_colors, location, negation, card_property, heart_type,
// ability_filter from that single borrow
```

Same for the negation guard at `:572-588`, which currently makes four separate
`common()` calls for fields **all in the same struct**.

**Scale:** ~9 of 10 discriminant matches removed per node. The cheapest, largest
**branch-count** win in the condition layer.
**Risk:** trivial. **Outcomes:** cannot change — `common()` is a pure projection.

### 8.2 `can_activate_effect` runs 4× per resolution, and two pairs are provably redundant

**This is the highest-value item in the condition layer, and it is the one the
combined doc's inference missed.** (The doc's inference was that the *clone path*
was done and the remaining money was in what `run_effect` **builds**. That is
true — §5 is large. It is not the whole story, because a redundant *evaluation*
is not a clone.)

`AbilityResolver::can_activate_effect` (`ability/resolver.rs:569`) has 14
references and **4 live call sites on the `resolve_ability` path**:

| # | call site | reached from |
| --- | --- | --- |
| 1 | `gates.rs:306` (`record_use_limit(Early)`) | `resolver.rs:1347` |
| 2 | `resolver.rs:1163` (`run_ability_effect`) | `resolver.rs:1372` |
| 3 | `ability/effects/mod.rs:60` (`gate_effect_activation`) | `execute_effect`, `resolver.rs:1180` |
| 4 | `gates.rs:323` (`record_use_limit(Final)`) | `resolver.rs:1381` |

(`gates.rs:141` `effect_condition_gate` is **dead** — `EFFECT_GATES` is
`&[incomplete_placement_gate]` at `gates.rs:261`, and the comment at
`gates.rs:1084-1088` says the duplicate pre-composite wrappers were removed. One
of them wasn't.)

Each call rebuilds a `ConditionContext` (see §4.12 — up to 12 zone scans) and
runs the full recursive `evaluate_condition`.

**Why (1) and (2) are provably redundant.** Between `resolver.rs:1347` and
`resolver.rs:1163` the sequence is:

- `resolver.rs:1349` `handle_pending_choice(…)` → returns `Ok(())` early if it
  creates a choice, so if we reach 1163 it took no branch and mutated nothing.
- `resolver.rs:1354` `check_ability_gates(POST_COST_GATES, …)` — `POST_COST_GATES`
  is `[post_cost_position_gate, optional_cost_skip_gate]` (`gates.rs:64-67`),
  neither of which mutates game state.
- `resolver.rs:1363-1367` sets `entry.cost_paid = true` — a queue-entry field, read
  by conditions only through `cost_already_paid` in `can_activate_effect:576-582`,
  which returns `true` (skip overlay) in **both** calls, so it cannot change the
  verdict.
- `resolver.rs:1369` a second `handle_pending_choice` — same early-return property.

**Why (2) and (3) are redundant** in the no-overlay case (the common case;
overlays require `in_flight.positional_overlay`, and `can_activate_effect:606-610`
returns `true` early whenever an overlay applies). Both run the *same function* on
the *same* effect.

**#3 and #4 CANNOT be merged** — `execute_effect` runs the effect, which mutates
state.

**Fix (safe half, recommended):** thread the verdict. Add to `InFlight`:

```rust
last_gate_verdict: Option<(*const AbilityEffect, bool)>,
```

set at the end of `can_activate_effect`, consumed and cleared by the
immediately-following call for the same effect pointer. Clear it in
`execute_effect` after `execute_effect_dispatch` returns, and in every
`resume_*` / `finalize_choice` path. **Pointer identity makes a stale hit
impossible**, and the lifetime is bounded to one `run_ability_effect` call.

**Do NOT merge #1 into #4** even though they are the same call. They are genuinely
separated by cost payment and effect execution, so they are semantically
distinct. `resolver.rs:630-631` states outright that the cache exists to avoid
*"re-evaluation against stale state (e.g. `revealed_cards` modified by a prior
`select_cards` filter)"* — i.e. **the 4 evaluations are load-bearing.** Collapsing
them is a semantics change, not a performance change.

**Scale:** 4 → 2 evaluations, unconditionally, on every conditional ability.
**Risk:** low for the pointer token. **High** for any structural merge.
**Outcomes:** the token version cannot change them. **Gate for the token
version:** `total_actions` fixed + a per-`Arc`-pointer instrumentation run printing
all 4 verdicts before and after, requiring identical. **If a structural version is
ever attempted, gate it on** a 500-game replay with identical final scores, win
rates and `total_actions`, and treat it as a **semantics change requiring sign-off**.

### 8.3 The condition dispatch is already a jump table — do not build an opcode list

This is worth stating explicitly because it is the most attractive-looking idea in
the condition layer and it is **not worth doing**.

`evaluate_condition` *is* a recursive `match` on the enum discriminant, but a
21-variant `enum` on a contiguous discriminant compiles to a bounds check plus an
**indexed jump**. There is no linear scan to eliminate and no arm-count problem.

Against a precompiled opcode form:

1. **The tree is tiny and shallow.** Depth histogram: 414 clauses at depth 0, 109
   at depth 1, **3 at depth 2**. A precompiled array would save at most 2 indirect
   calls on 3 clauses out of 526.
2. **The dispatch is already a jump table** (above).
3. The 526 clause *instances* dedupe to **201 unique shapes**, so a bytecode form
   would be 201 entries — but decoding is paid once and amortised over the game,
   while dispatch is paid thousands of times. The asymmetry is fine; the payoff
   is not.
4. **The real cost is the ten `common()` calls that precede and follow the
   dispatch**, each re-matching the same discriminant the dispatch just matched.
   That is §8.1, and it is where the win actually is.

### 8.4 The verdict memoisation mechanism exists, was written, and is 100% dead

`ability/resolver.rs:646-648`:

```rust
pub fn cached_condition_verdict(&self, gs: &GameState, condition: &Condition) -> Option<bool> {
    // Only cache conditions that opted in via `cache: true`.
    if !condition.get_cache().unwrap_or(false) { return None; }
    …
```

**Census: 0 of the 526 reachable condition clauses carry `"cache": true`.** The
same guard is at `:675-677` in `store_condition_verdict`.

Two consequences, and the first corrects a widely-repeated claim:

1. **The `format!("{:?}", condition)` key cost is currently zero in production.**
   Both functions return before reaching the `format!`. Do not spend a slot
   optimising it.
2. **The more interesting fact is that the memoisation design already exists, was
   written, and is switched off for the entire corpus.** The `cache: true`
   mechanism was clearly *intended* to fix exactly the §8.2 redundancy — evaluate
   once in the TAS pre-filter, reuse in `can_activate_effect` — and no ability
   uses it. **Deciding whether to enable it is a bigger question than
   optimising around it**, and it is a **gameplay** question: the doc at
   `resolver.rs:630-631` records that the cache's stated purpose is to avoid
   re-evaluation against stale state, which means a stale hit is a known hazard.

**If it is ever enabled**, the `format!` key is then a real cost, and the right
key is a provenance identity rather than a value identity:
`AbilityQueueEntry` has `ability_index: usize` (`queue.rs:84`), a stable
per-ability ordinal, so a per-resolution counter incremented in tree-walk order
makes the key a `u32` pair and shrinks `condition_cache: SmallVec<[(String,bool);2]>`
to `[(u32, bool); 2]`. **Note the direction of the change:** a value-identity key
collides for structurally-equal conditions; a provenance identity does not, so
switching makes the partition **strictly finer** — it can only ever produce *more*
cache misses, never a wrong hit. If a verdict changes, that is a **bug fix**, and
it must be reported separately from any perf claim.

### 8.5 The TAS pre-filter builds a `ConditionContext` when there is no condition

`core/game_state/abilities.rs:670-692`:

```rust
let event_driven = condition_is_event_based(&ability);
let cond_ok = match &ability.effect {
    Some(e) if event_driven => {
        let ctx = ConditionContext::new(
            self, Some(cid), Vec::new(), Vec::new(),
            entry.and_then(|e| e.trigger_moved_cards.clone()),
            entry.and_then(|e| e.trigger_appeared_cards.clone()),
        );
        e.condition.as_ref().is_none_or(|c| ctx.evaluate_condition(c))
    }
    _ => true,
};
```

This runs for **every card × every ability × every TAS trigger point**. Two
avoidable costs:

1. `ConditionContext::new` is built **per (card, ability)** even when the ability
   has no `effect.condition` — the `is_none_or` would short-circuit. **Move the
   `e.condition.is_none()` test *before* the construction.**
2. `entry.and_then(|e| e.trigger_moved_cards.clone())` and
   `…_appeared_cards.clone()` are two `Option<SmallVec<[i16;4]>>` clones per
   (card, ability), and they are only read by `movement_condition` and
   `appearance_condition` — **8 + 9 = 17 clauses of 526**. Hoist out of the
   ability loop, or gate the clone on whether the condition needs them.

**Scale:** per card placed, 2 stage cards × ~1.3 abilities × 4 scans ⇒ up to ~8
wasted `SmallVec` clones and ~8 wasted `ConditionContext` constructions.
**Risk:** trivial. **Outcomes:** cannot change — the pre-filter result is **not
cached anywhere**, so re-ordering the checks cannot change the enqueue decision.

### 8.6 The Rule 9.5.3 ordering prompt builds ~15 `String`s to ask for an index

`core/game_state/abilities.rs:1679-1711`:

```rust
let options = core::iter::once(idx).chain(core::iter::once(second)).chain(available_indices)
    .filter_map(|idx| {
        let entry = self.ability_queue.get_entry(idx)?;
        let card_name = self.card_database.get_card(cid).map(|c| c.name.to_string())
            .unwrap_or_else(|| entry.card_no.to_string());
        Some(AutoAbilityOption { card_name, ability_text: entry.ability.full_text.clone(), … })
    }).collect();
let choice = Choice::SelectAutoAbility {
    player_id: player_id.to_string(),
    description: "複数の自動能力が…".to_string(),
    description_en: Some("Multiple auto abilities…".to_string()),
    description_ja: Some("複数の自動能力が…".to_string()),
    options,
};
```

Only taken when 2+ same-player abilities are queued, so low frequency. The
`description` and `description_ja` pair is **byte-identical** — one should be a
shared `const`. The per-option `String`s are unavoidable given
`AutoAbilityOption` owns them (`ability/types.rs:310-320`) but are display-only.

**Scale:** ~15 allocations, low frequency. **Risk:** trivial.
**Outcomes:** cannot change.

### 8.7 `EffectFilter` accessor chains — 3 pointer chases per `*_any()`

The shape, from `core/card.rs:1886-1906`:

```rust
pub fn card_names_any(&self) -> Option<&Vec<String>> {
    self.kind.as_ref()?.filter()?.card_names.as_ref()
}
```

`self.kind: Option<Arc<EffectKind>>` → **deref 1**; `.filter()` is a **13-arm
match** on `EffectKind`; `.filter` is `Option<Box<EffectFilter>>` → **deref 2**. So
each accessor = 1 `Option` check + 13-arm match + 1 `Box` deref = **3 pointer
chases**.

`AbilityEffect::filter_subset` (`core/card.rs:2377-2434`) calls **20+ `*_any()`**
⇒ **~60 pointer chases** plus a 40-field struct write. See §6.11 for the fix.

`Condition::filter_subset` (`core/card.rs:3794`) is much cheaper — it goes via
`common()` (1 match + 1 `Box` deref) and reads 9 fields. **So the effect-side
`filter_subset` chain is the hot one, not the condition side**, which is the
opposite of where the question usually gets pointed.

`EffectKind::from_action` (`core/card.rs:1263-1268`) starts with
`let a = action.to_lowercase();` — one `String` per call, then a 60-arm match.
`ActionType::to_str()` already returns a canonical `&'static str`; match on
`action` directly with both the wire spelling and its lowercase form as arms, or
use `eq_ignore_ascii_case` in a small table — **preserving the lowercase fallback
for mixed-case inputs.** **Gate:** a corpus test that `from_action(s)` and
`from_action(&s.to_lowercase())` agree for all abilities.

---

## 9. Category G — the choice and resume path

`resume_with_choice` is 148 allocations per call. The answer side is cheap; the
expensive part is that **answering a choice re-offers one, and offering is
expensive.**

### 9.1 `ability_master_id()` heap-allocates on every call — 199 call sites

**This is the most-called function in the engine that is not free, and it is
probably the single largest count in the whole catalogue.**

`core/game_state/abilities.rs:2679-2695` (verified verbatim):

```rust
pub fn ability_master_id(&self) -> Option<String> {
    self.ability_queue.current_entry().map(|e| e.player_id.clone())     // <-- String heap alloc
}
fn resolve_master_id(&self) -> Option<String> {
    self.ability_master_id().or_else(|| {
        self.activating_card.and_then(|cid| self.owner_of_card(cid)).map(|p| p.id.to_string())  // 2nd alloc
    })
}
```

`AbilityQueueEntry.player_id` is a plain `String` (`ability/queue.rs:82`), so that
first clone is a **heap allocation, not an atomic** — see §6.7 for the field-type
fix that makes it free.

And the `String` exists only to be dropped. `resolve_target_player` / `_mut`
(`:2697-2760`, verified verbatim) test it against exactly two strings:

```rust
let master = self.resolve_master_id();
match (target, master.as_deref()) {
    ("self", Some("player2") | Some("p2")) => &self.player2,
    ("self", _) => &self.player1,
    ("opponent", Some("player2") | Some("p2")) => &self.player1,
    ("opponent", _) => &self.player2,
    …
```

**Call-site count: 199 `resolve_target_player` references** across
`ability/choice.rs` (~43), `ability/effects/*` (~60+), `ability/cost/handlers.rs`
(18), `ability/condition/card/comparison.rs` (8), `ability/effects/position.rs`
(18), `ability/effects/look.rs` (15), `ability/effects/energy.rs` (12),
`ability/effects/resource.rs` (11), `dynamic_count.rs` (9),
`ability/effects/change_state.rs` (8), `turn/phases.rs` — plus 22 direct
`ability_master_id()` callers.

The worst single file is `ability/effects/change_state.rs`: it eagerly does
`let target = effect.target_name().to_string();` at `:125` and then calls
`resolve_target_player(&target)` **8 times** (`:49, 106, 178, 208, 238, 311, 326,
421`) — **9 `String` allocations for one effect.**

Worse, the `owner_of_card` fallback (`:2723-2731`) does **six `Vec::contains` scans
per player = up to 12** over hand/waitroom (the large zones) and runs whenever
there is no current queue entry.

**Fix:** one helper, no allocation, no scans on the common path:

```rust
#[inline]
fn master_is_p2(&self) -> Option<bool> {
    self.ability_queue.current_entry()
        .map(|e| e.player_id == "p2" || e.player_id == "player2")
        .or_else(|| self.activating_card.and_then(|c| self.owner_of_card(c))
                  .map(|p| p.id.as_str() == "p2" || p.id.as_str() == "player2"))
}
```

then rewrite both `resolve_target_player` and `resolve_target_player_mut` on top
of it. The rewrite is **provably equivalent** — the discarded `String` was never
observed, and the match has exactly two arms per `target`, both keyed only on that
predicate. Also change `change_state.rs:124-125` to keep the `&str`.

**Scale:** 1 `String` allocation per call (+ up to 12 zone scans on the fallback
path) on a function with **199 call sites**. This is the largest single count in
the catalogue.
**Risk:** low effort, low risk. `ability_master_id()`'s signature change touches
its 22 direct callers; the 197 `resolve_target_player` sites are untouched.
**Outcomes:** cannot change. **Gate:** allocation count with `total_actions` fixed
as a *confirmation*, not as the risk control; plus a debug-assert build comparing
old vs new player resolution at every call site for one full run.

### 9.2 `provide_choice_result` deep-clones the whole `Choice`, twice

`ability/choice.rs:301-318`:

```rust
pub fn provide_choice_result(&mut self, gs: &mut GameState, result: ChoiceResult) -> Result<(), String> {
    let choice = self.awaiting.choice.clone();                      // up to ~25 allocs
    let context = self.in_flight.execution_context.clone();          // 2 String allocs
    let Some(choice) = choice else { return Err("No pending choice".to_string()); };
    dispatch_choice_result(self, gs, &choice, &result, context)
}
```

`Choice::SelectCard` (`ability/types.rs:97-149`) carries ~24 fields: 8
`String`/`Option<String>`, 4 `Option<Vec<_>>` (`characters: Option<Vec<String>>` is
a `Vec` **and** each `String`), and several scalars. The clone exists purely to
satisfy the borrow checker across `&mut self`.

**And the caller already owns the value** — `turn/actions/mod.rs:1545-1546`:

```rust
resolver.awaiting.choice = Some(choice);        // moved IN
let res = resolver.provide_choice_result(game_state, result);   // cloned straight back OUT
```

…then clones it a **second** time at `turn/actions/mod.rs:1571` for the
sub-choice path:

```rust
let sub_choice = resolver.awaiting.choice.clone().unwrap();
```

**Fix:** change `provide_choice_result` to take `choice: Choice` **by value** and
have `resume_queue_with_choice` **move** it in rather than store-then-clone.
`dispatch_choice_result` matches on `&choice` — pass the reference after the move.
For `ExecutionContext`, use `core::mem::take(&mut self.in_flight.execution_context)`
since every consumer clears it anyway (`finalize_choice` at `choice.rs:207-216`;
`process_current_ability` resets it at `abilities.rs:2041`).

**Scale:** up to ~50 allocations per choice resume.
**Risk:** medium — it touches the resume state machine, and
`resume_queue_with_choice` reads `resolver.awaiting.choice` **after** the call at
`:1569` and `:1571`, so the reordering needs care.
**Outcomes:** none, but **gate on a full 500-game replay with byte-identical
logs** + `total_actions` fixed.

### 9.3 `SelectionContext` re-clones 9–10 fields out of a live `Choice`

`ability/choice/result_handlers.rs:41-65`:

```rust
resolver.handle_select_card(gs, choice, zone, context,
    crate::ability::choice::SelectionContext {
        card_type: card_type.clone(),
        indices: indices.to_vec(),
        cost_limit_operator: cost_limit_operator.clone(),
        cost_total_operator: cost_total_operator.clone(),
        group: group.clone(),
        characters: characters.clone(),            // Vec<String> + N
        filtered_indices: filtered_indices.clone(),
        target_player_id: target_player_id.clone(),
        destination: destination.clone(),
        …
```

The destructuring at `:16-34` borrows straight out of `choice`, which is **alive
for the whole call**. `SelectionContext` (`choice.rs:33-51`) uses owned
`String`/`Vec` fields **purely to escape that borrow**.

**Fix:** give `SelectionContext` a lifetime — `pub card_type: Option<&'a str>`,
`pub characters: Option<&'a [String]>`, `pub filtered_indices: Option<&'a [usize]>`,
etc. The ~30 downstream readers (`handle_hand_selection`, `handle_stage_selection`,
`build_reprompt` at `choice.rs:1163`) already treat them as read-only;
`build_reprompt` is the one that will still need to clone, because it builds a
**new** `Choice` and genuinely needs owned data.

**Scale:** 9–10 allocations per `SelectCard` resume. **Risk:** medium — mechanical
but wide. **Outcomes:** cannot change. **Gate:** `total_actions` + full replay.

### 9.4 `resume_queue_with_choice` clones a ~300-byte `Choice` to hand it to the queue

`turn/actions/mod.rs:1571-1577`:

```rust
let sub_choice = resolver.awaiting.choice.clone().unwrap();
let targets_opponent = Self::sub_choice_targets_opponent(&resolver, &sub_choice);
Self::route_sub_choice_player(game_state, &sub_choice, targets_opponent);
resolver.store_pending_choice(game_state);
game_state.ability_queue.set_resolver(resolver);
game_state.ability_queue.pause_for_choice(sub_choice);
```

10–20 allocations for a `SelectCard` sub-choice.

**Restructuring, carefully.** The clone is only needed because `store_pending_choice`
reads `self.awaiting.choice` *after* `sub_choice` was cloned. But
`store_pending_choice` only **reads** `self.awaiting.choice` (`resolver.rs:821`) —
it does not write it. And `capture_choice_owner` reads `owner.executing`, which is
**independent** of the two predicates that use `sub_choice`.

So the safe order is: `capture_choice_owner()` → predicates on a **borrow** →
`store_pending_choice()` → `take()` → `pause_for_choice`. That requires splitting
`store_pending_choice` into `capture_and_mark` and `log_the_offer`.

**Scale:** 10–20 allocations per sub-choice resume. **Risk:** medium — a real
refactor of a load-bearing sequence, and the doc at `resolver.rs:102-119` records
that this exact area has produced bugs before.
**Outcomes:** none **if** the reordering is provably order-independent (the
predicates and `store_pending_choice` touch disjoint state).
**Gate:** full 500-game replay, byte-identical logs, `total_actions` fixed.

### 9.5 `route_sub_choice_player` stringifies a `ChoiceRoute` to run one `starts_with`

`turn/actions/mod.rs:1378-1387`:

```rust
let route = game_state.ability_queue.current_entry()
    .and_then(|entry| entry.choice_card_no.as_ref())
    .map(ToString::to_string);                                    // <-- String alloc
let self_position_route = route.as_deref()
    .is_some_and(|raw| raw.starts_with("position_change:self"));
```

**The codebase already has the right idiom two files away** —
`ability/queue.rs:331-333` matches it *structurally*:

```rust
matches!(route, ChoiceRoute::Raw(raw) if raw.starts_with("position_change:opponent:"))
```

**Fix:** match structurally, exactly as `queue.rs:331` does.
**Scale:** 1 allocation per sub-choice. **Risk:** trivial. **Outcomes:** cannot
change, **provided** `ChoiceRoute`'s `Display` for non-`Raw` variants does not
emit a `"position_change:self"`-prefixed string — it emits the variant name.

### 9.6 `resume_with_choice` clones a `ChoiceRoute` to take a `&` of it

`turn/actions/mod.rs:986-991`:

```rust
let choice_card_no = game_state.ability_queue.current_entry()
    .and_then(|e| e.choice_card_no.clone());        // <-- ChoiceRoute clone; Raw(String) = heap alloc
let result = Self::build_choice_result(&choice, card_id, card_indices, choice_card_no.as_ref())?;
```

The `.clone()` is immediately followed by `.as_ref()`. Worse, the comparison
*inside* `build_choice_result` allocates a `String` to build the literal it
compares against — **on every entry to that arm**, not once at compile time:

```rust
// turn/actions/mod.rs:1117-1120
if choice_card_no == Some(&ChoiceRoute::Raw("position_change:opponent:front".to_string()))
```

**Fix:** a `fn route_is(raw: &ChoiceRoute, s: &str) -> bool` helper doing
`matches!(r, ChoiceRoute::Raw(x) if x == s)`. That kills the per-call `to_string`
in the pattern.
**Scale:** 1 large `String` per card-selection resume, on the most common choice
shape. **Risk:** trivial. **Outcomes:** cannot change.

### 9.7 `ResumeSnapshot::capture` calls `current_entry()` ten times

`turn/actions/mod.rs:60-147`:

```rust
let entry = game_state.ability_queue.current_entry();
let cost_was_paid = entry.is_some_and(|e| e.cost_paid);
let effect_started = game_state.ability_queue.current_entry().is_some_and(|e| e.effect_started);
let just_completed_key: Option<u32> = game_state.ability_queue.current_entry().and_then(…);
let entry_player_id = game_state.ability_queue.current_entry().map(|e| e.player_id.clone());
let cost_entry_trigger = game_state.ability_queue.current_entry().map(|e| e.trigger_type.clone());
let cost_entry_card_id = game_state.ability_queue.current_entry().and_then(|e| e.card_id);
let cost_entry_opt_result = game_state.ability_queue.current_entry().and_then(|e| e.optional_cost_result);
let optional_skipped = game_state.ability_queue.current_entry().is_some_and(…);
let chose_to_pay = game_state.ability_queue.current_entry().is_some_and(…);
```

**Ten** separate lookups of the same entry. Each is cheap, but the borrow checker
is being defeated ten times because each line borrows a different field. Then
`e.player_id.clone()` and `e.trigger_type.clone()` are two owned clones of queue
state per choice answer.

**Fix:** destructure once into locals — the block at `abilities.rs:1908-1933`
already does exactly this correctly; **copy that pattern.** Then retype
`player_id`/`trigger_type` per §6.7.
**Scale:** 2 allocations + ~10 redundant lookups per choice answer.
**Risk:** trivial. **Outcomes:** cannot change.

### 9.8 `activation_ability_id` formats the full Japanese ability text

`turn/actions/mod.rs:458-474`:

```rust
fn activation_ability_id(..., ability: &crate::card::Ability) -> String {
    if let Some(gidx) = … { format!("{}_gained_{}", card_no, gidx) }
    else { format!("{}_{}", card_no, ability.full_text) }      // 30-150 bytes of Japanese text
}
```

Called at `turn/actions/mod.rs:581-582`, i.e. on **every `use_ability`**.

**The `AbilityId` doc (`ability/queue.rs:35-57`) already says the identity "is
already carried by `card_no` + `ability_index` + `trigger_type`"** and records that
eagerly formatting it "cost two heap allocations on every single enqueue" — that
is how `AbilityQueueEntry.id` was deleted in the first place. `handle_use_ability`
has `card_no` and `idx` in hand; thread them.

**Scale:** 1 large `String` per `use_ability`. **Risk:** low-medium (a signature
change through `trigger_auto_ability`). **Outcomes:** none as long as the rebuilt
string is byte-identical — the `AbilityId` is used only by the **test-only**
`has_entry_with_id` (`queue.rs:269-273`), so the risk is confined to tests.
**Gate:** `total_actions` + the `has_entry_with_id` tests.

### 9.9 The ability identity is a string that is parsed back to an index

Related to §9.8, and a latent correctness bug rather than only a perf item.

**Build side** — `turn/trigger_scan.rs:108-112` (and identically at `:284-288`):

```rust
abilities_to_trigger.push((
    format!("{}_{}", card_no, ability.full_text),   // String per matched ability
    card_no.to_string(),                            // String per matched ability
    card_id,
));
```

**Consume side** — `core/game_state/abilities.rs:1232-1253`:

```rust
let requested_text = ability_id.strip_prefix(card_no).and_then(|s| s.strip_prefix('_'));
for (ability_index, ability) in card.abilities.iter().enumerate() {
    … requested_text == Some(resolved_ability.full_text.as_str()) …
```

**The caller already knows the index** — it is `enumerate()`ing `card.abilities`
when it builds the string. The string concatenates two values the consumer then
uses to *re-derive the index it started with*, via a full-text string comparison
per ability.

**And the identity is not unique.** Two abilities on one card with identical
`full_text` (different `count`, different `target`) **collide**, and
`find_printed_ability` returns the first match. A `(card_id, ability_index)` key
makes that impossible. The combined doc §10.1 says exactly this about
`AbilityQueueEntry.id`.

Also: `find_printed_ability` resolves each ability **twice** — `ability.resolve()`
at the top of the loop, then `ability.to_arc()` on the match, and
`AbilityRef::to_arc` is documented as "returns `self.resolve()`"
(`ability/ability_store.rs:74-77`). **One-line fix:** return
`Some((ability_index, resolved_ability))`.

**Fix:** pass `(card_id, ability_index)` through `trigger_auto_ability` and drop
`ability_id` from the enqueue path. `abilities_to_trigger` becomes
`SmallVec<[(i16, usize); 4]>` instead of `SmallVec<[(String, String, i16); 4]>` —
a ~4× smaller inline size.

**Scale:** 2 `String`s + 1 `format!` per matched ability per scan, with 4+ scans
per play plus live-start/live-success.
**Risk: this one CAN change game outcomes.** It touches the enqueue API and the
identity used to route abilities, and **removing the string removes an accidental
collision-avoidance mechanism.** §5's advice applies: **do it as a deliberate
project, not in passing.**
**Gate:** `total_actions` fixed + full suite + a fixed-seed win-rate comparison.
The latent collision is a **bug to report separately** from the perf change.

### 9.10 `pay_cost_move_cards` and `store_pending_choice` — see §5.6 and §3.4

Both are filed above; listed here for completeness of the choice/effect boundary.

---

## 10. Category H — effect execution

### 10.1 `execute_move_cards` allocates ~8 owned `String`s before doing anything

`ability/effects/move/mod.rs:2042-2161`:

```rust
let (card_type_owned, or_resolved) = self.resolve_or_card_type(gs, effect)?;  // -> Option<String>
let tgt = effect.target.clone();                                              // 2072: Option<String> clone
let card_db = gs.card_database.clone();                                       // 2077: Arc bump
let character_filter: Option<Vec<String>> = effect.characters_any().cloned(); // 2082: Vec<String> clone
let name_fragments: Option<Vec<String>> = Self::resolve_name_fragments(gs, effect);
let source = effect.source_any().map(|s| s.to_string()).unwrap_or_default();   // 2089
let destination = effect.destination.map(|z| z.to_str().to_string()).unwrap_or_default();  // 2093
…
self.in_flight.spawn_context.destination = effect.destination.map(|z| z.to_str().to_string()); // 2154  ← AGAIN
self.in_flight.spawn_context.source = effect.source_any().map(|s| s.to_string());              // 2155  ← AGAIN
```

**Nine `String` allocations, four of which (`source`, `destination`) are computed
twice from the same effect field** — once into locals, once into `spawn_context`.
`MoveCards` is the single most common effect in the card pool (it backs
`DiscardCard` too, `dispatch.rs:66`), so this is the most-executed allocation
block in the effects tree.

`resolve_or_card_type` (`move/mod.rs:1769-1813`) returns `Option<String>` via
`effect.card_type_any().map(|s| s.to_string())` at `:1775` and `:1778` — and is
called on **every** `MoveCards`, including the ~95% with no `or_card_types` at all.

`resolve_name_fragments` (`:1816-1843`) returns `None` unless
`name_constraint == "contains_all"`, which is rare — its guard is two field
reads, so calling it unconditionally is fine.

**Fix:**
- `source` / `destination`: borrow. `Zone::as_str()` already returns a borrowable
  `&str` — `let source = effect.source_any().unwrap_or("")`.
- `tgt`: used only as `tgt.as_deref()` at `:2255` and `:2279`, so
  `effect.target.as_deref()`.
- `spawn_context.destination` / `.source` at `:2154-2155` need owned `String`s
  because `spawn_context` outlives the borrow of `effect` — but they are set to
  the *same values* as the locals, so build once and clone the two, or retype
  them `Option<ArcStr>` (`crate::types::ArcStr` is already in use at
  `turn/phases.rs:590`).

**Scale:** ~9 allocations per `MoveCards`, on the most common effect.
**Risk:** low. **Outcomes:** cannot change.

### 10.2 `execute_modify_required_hearts_standard` takes a `Vec<String>` for a slice

`ability/effects/dispatch.rs:191`:

```rust
ActionType::ModifyRequiredHeartsGlobal => resolver.execute_modify_required_hearts_standard(
    gs, effect,
    effect.operation_any().unwrap_or("increase"),
    effect.value_or_count(1),
    effect.heart_colors_any(),          // <-- Vec<String> clone per execution
    effect.target_name(),
    &effect.text,
),
```

There are **41 `state_condition` clauses** in the corpus, and `heart_colors` is
the field they all read. **Fix:** take `&[String]` (or `&[ArcStr]`) and clone
only if the callee mutates.
**Scale:** 1 `Vec` + N `String` allocations per heart modification.
**Risk:** low if the callee only reads. **Outcomes:** cannot change.

### 10.3 `position_any().cloned()` on every position-change effect

`ability/effects/dispatch.rs:82`:

```rust
ActionType::PositionChange => resolver.execute_position_change(
    gs, effect,
    effect.position_any().cloned(),                       // <-- Option<PositionInfo> CLONE
    effect.target_name(),
    effect.target_member_any().unwrap_or("this_member"),
),
```

`PositionInfo` is an enum carrying either a `String` or a `Box<Struct>`.
`execute_position_change_with_destination` is called for most position changes
anyway, so **this clone is frequently discarded.** **Fix:** take
`Option<&PositionInfo>`. **Scale:** 1–2 allocations per position change.
**Risk:** low. **Outcomes:** cannot change.

### 10.4 `effect` writes that are dead before the next read

Several `EffectOwner` writes (see §5.4) may be immediately overwritten.
`ability/effects/position.rs:287, 297, 301, 455` performs three clones for a
`both` target plus a saved copy; `ability/effects/target.rs:120, 130, 141, 347`
clones 2–3× in `handle_both_targets` and saves a fourth.

**The cheap audit, before any of §5.4's provenance work:** for each write, check
whether the value is read before the next write. Several executors write and then
re-enter dispatch, which overwrites it. **Dead writes are free to remove, carry
zero risk, and may account for a meaningful share of §5.4's count.**

### 10.5 `dispatch.rs:167` — `effect.action.to_str()` on the `Custom` arm

```rust
ActionType::Custom => resolver.execute_custom(gs, effect, effect.action.to_str()),
```

`execute_custom` uses `action_str` in only two places: the `duration_any()`
fallback at `:257` and a debug log at `:269`. **Worth a one-line check that
`to_str()` does not allocate** — `ActionType` is a fieldless enum, so it should
return a `&'static str` and be free. If it is already free, no action.

### 10.6 The effect dispatch itself is fine — do not touch it

`ability/effects/dispatch.rs:59-224` matches on `effect.action: ActionType` — a
**dense enum discriminant jump table**. LLVM compiles it to a bounds check plus
an indexed jump. **There is no fallthrough, no linear scan, and no arm-count
problem.** This is worth stating explicitly because "the dispatch is a 68-arm
match" sounds like a problem and is not.

The *arguments* to some arms are evaluated before entry, which is a real cost —
but those are §10.2, §10.3 and §3.2, not the dispatch.

---

## 11. Category I — the turn and phase loop

`advance_phase` is about a third of engine time, and `first_performance` /
`second_performance` are two of the top four cost centres.

### 11.1 `check_timing` is a whole-engine sweep, called on five of nine `advance_phase` arms

`turn/phases.rs:175, 198, 240, 349` call `Self::check_timing` directly;
`:258`/`:263` reach it transitively via `execute_performance_phase`.

`turn/actions/mod.rs:1595-1652`:

```rust
pub fn check_timing(game_state: &mut GameState) {
    Self::check_duplicate_members(&mut game_state.player1, &mut game_state.card_database);
    Self::check_duplicate_members(&mut game_state.player2, &mut game_state.card_database);
    …
    game_state.player1.refresh();
    game_state.player2.refresh();
    Self::check_victory_condition(game_state);
    Self::check_invalid_live_cards(game_state, true);
    Self::check_invalid_live_cards(game_state, false);
    Self::check_invalid_energy_cards(...);   // ×2
    Self::check_orphaned_under_cards(...);    // ×2
    game_state.recalculate_constants();
    Self::check_invalid_resolution_zone(game_state);
    game_state.process_pending_auto_abilities(&active_player_id);
}
```

Two concrete inefficiencies inside, both independent of the closed dirty-flag work:

- **`check_invalid_energy_cards`** runs a `get_card` hash probe per energy card
  (≤12 × 2 players) on every call, whether or not anything is invalid — see §7.5.
- **`check_invalid_live_cards`** allocates a `Vec<(usize,i16,bool)>` eagerly, then
  returns if empty — 2 allocations per `check_timing` for nothing in the
  overwhelming majority of calls. `SmallVec<[(usize,i16,bool);3]>` is free to adopt.

**Scale:** small in absolute terms — `check_timing` is itself a fraction of the
engine — but it multiplies by 5 per turn × 2 per performance.
**Risk:** trivial. **Outcomes:** cannot change.

### 11.2 `execute_performance_phase` builds two throwaway collections per performer

`turn/phases.rs:320-333`:

```rust
let nhm_flat: Vec<(i16, crate::card::HeartColor, ModifierEntry)> =
    game_state.mods.need_heart_modifiers.iter()
        .flat_map(|(&card_id, colors)| colors.iter().map(move |(&color, entry)| (card_id, color, *entry)))
        .collect();
```

A full flatten of the two-level `need_heart_modifiers` map into a heap `Vec` on
**both** performances, purely to hand to `build_snapshot` (`phases.rs:614-624`).
Typically 2–6 entries. **Fix:** `SmallVec<[(i16, HeartColor, ModifierEntry); 8]>`.

And `turn/phases.rs:547-561`, the `performer_owned_ids` `HashSet` — **§4.7, an
already-landed pattern not applied here.**

Then `phases.rs:562-575` partitions `ability_applications` into `apps` /
`other_apps` with two fresh `Vec`s and a push per element, then `other_apps.into()`.
**Fix:** reuse a scratch `Vec` — `GameState` already has the `scratch_*` idiom
(`modifiers.rs:592-613`).

**Scale:** low single digits of `execute_performance_phase`, which is 2 of the
top 4 cost centres. **Effort:** ~1 hour. **Risk:** none.

### 11.3 `push_revealed_card` allocates a `String` from a `&'static str`

`core/game_state/mod.rs:1012-1032`:

```rust
fn push_revealed_internal(
    card_database: &CardDatabase, cards: &mut SmallVec<[i16; 8]>, meta: &mut SmallVec<[RevealedCardMeta; 8]>,
    card_id: i16, source_card_id: Option<i16>, is_private: bool, owner: Option<u8>,
    reveal_type: &'static str,                                  // <-- &'static str …
) {
    let source_name = source_card_id.and_then(|sid| card_database.get_card(sid)).map(|c| c.name.to_string());
    cards.push(card_id);
    meta.push(RevealedCardMeta {
        source: source_card_id,
        source_name,                                             // fresh String per card in the batch
        is_private, owner,
        reveal_type: reveal_type.to_string(),                    // <-- … .to_string()'d on every call
    });
}
```

**Two findings:**

1. **`reveal_type` is a `&'static str` parameter that is `.to_string()`d on every
   call.** The value can only be one of a handful of literals (`"yell"`,
   `"ability"`, …). It is compared and serialised, never mutated. **Make
   `RevealedCardMeta.reveal_type` a `&'static str` or a small enum.** This is one
   allocation per revealed card on every path that reveals.
2. **`source_name` is a fresh `String` of the *same* source card's name,
   recomputed for every card in the batch.** At `look.rs:378-380` and
   `look.rs:1177-1179` the same `source_card_id` is passed for every card, so the
   same name string is allocated N times. **And `source_name` is redundant** — the
   metadata already carries `source: Option<i16>` (`:1027`), so the name is
   derivable. **Store the `i16` and resolve the name at render time.**

**Scale:** a direct, unconditional allocation on a per-revealed-card path.
`execute_reveal_per_group` (`look.rs:1178`) iterates `by_group.values()` and pushes
every card — for a Discard-source reveal that is the whole waitroom (~30–40
cards), each with a fresh `reveal_type.to_string()`. `execute_performance_phase`
calls it once per yell card, twice per turn.
**Risk:** medium **if** `RevealedCardMeta` crosses the wire — it very likely
does. Use an enum with `#[serde(rename_all = "snake_case")]` and confirm with
`test_blob_matches_json`-style wire tests. Low risk on the `source_name` half.
**Outcomes:** cannot change, as long as the serialisation shape is preserved.

### 11.4 `advance_phase`'s own loop — nothing to hoist

Recorded for completeness: after reading `turn/phases.rs` (1,721 lines) and
`turn/live.rs` (3,045 lines), the phase bodies themselves contain **no**
loop-invariant work worth hoisting. The cost of `advance_phase` is entirely in
what it *calls* — `check_timing` (§11.1), `recalculate_constants` (§3.7, §3.8,
§4.1, §5.10, §6.3) and the trigger scan (§2.1). Every `advance_phase` item in
this catalogue is therefore an item in one of those.

**That is the honest finding, and it matches the combined doc §3.** The reason a
flat profile has no hotspot is that each phase is a *sequence of whole-engine
sweeps*, and the sweeps are the work.

### 11.5 `Vec` growth and reallocation

- `truncate_rule_log` / `truncate_structured_log` (`core/game_state/mod.rs:1564-1582`)
  are **O(cap) memmove per push** (512 B / ~100 KB). Dead in the benchmark
  (both are `logging_enabled()`-gated) but **live in every shipping push** —
  a shipping-path cost, not a `profile_target` cost, and labelled as such.
- `ability/v7_ismcts.rs:363-364` drains `rule_log` to 96 entries by
  `drain(0..len-96)` — O(n) per call, and `drain` from the front of a `Vec` is a
  memmove. Minor.
- `GameState.turn_movements` / `turn_area_movements` / `batch_movements`
  (`mod.rs:147,151,154`) are `SmallVec<[MovementEvent; N]>` where `MovementEvent`
  is 33–40 bytes **with a `String` in it** — see §6.7.

### 11.6 What is already correct on the turn path

Recorded so nobody re-audits it:

- **Zone storage** — entirely inline, zero-allocation in steady state (§6.1).
- **`recalculate_constants` recursion** — `refresh_yell_sources` calls it
  exactly twice per card play, gated on `restored == restored_all` ("only when
  they can actually differ"). Correct and the right shape.
- **The `scratch_*` buffer idiom** — `scratch_exp_blade/score/heart` and
  `scratch_entry_positions` are correctly recycled via `mem::take`. §3.8 is about
  finishing the set, not about questioning the pattern.
- **`Timer` instrumentation** — `profiling` is off by default; `timer.rs:136-162`
  confirms zero-cost construction, and there are **0** `Instant::now()` calls
  outside `#[cfg(feature = "profiling")]`. The ~41 `Timer::start` sites cost
  nothing in benchmark builds.

---

## 12. Category J — `no_std` console targets

These do nothing for the desktop benchmark and everything for the console
builds. They are here because `no_std` removes a dependency the desktop relies
on, and the removal is *unmeasured* on any machine available here.

### 12.1 There is no ability-decode cache at all on `no_std`, and it is on the hot path

`ability/ability_store.rs:45-62`:

```rust
pub fn resolve(&self) -> Arc<Ability> {
    #[cfg(not(target_has_atomic = "ptr"))]
    { *self.decoded_slots()[self.ability_index as usize].get().expect("ability bytes not decoded") }
    #[cfg(target_has_atomic = "ptr")]
    { Arc::new(self.decode()) }          // <-- full decode, EVERY call
}
```

On the console targets there is **no `OnceLock`**. Every `resolve()` is:
`ability_bytes()` → `Ability::decode(bytes)` → the full VM decode from
`condition_decoder_gen.rs` → the whole `Ability` → `AbilityEffect` → `EffectKind`
tree (13 `Box<EffectFilter>` variants, every `Vec<String>`, every
`Option<Box<…>>`) → wrapped in a fresh `Arc`.

**Cost per call: on the order of tens to low-hundreds of allocations**, given a
165-field `EffectFilter` with 29 `Box`es (`core/card.rs:942-1159`).

**Where it runs on the condition path — and this is the bad part.** The
`SetCardIdentity` arm of `card_matches_group_str` (`ability/util.rs:709-717`) is
inside the **innermost zone filter**. A 64-card zone scan with a 2-name group
filter on `no_std` is **64 × 2 × (full ability decode) = 128 full ability-tree
constructions for one clause.** Same at `core/card.rs:336`, `:3358`, `:3404`,
`ability/condition/card/predicates.rs:57`, `util.rs:1044`, plus
`core/game_state/abilities.rs:611, 674, 787, 1058, 1123, 1409, 1493, 1535, 1555,
1577`, `resolver.rs`, and `effects/*`.

**The fix is not a cache — it is the trigger bitmask.** A per-ability `u32`
trigger mask stored next to the decode slot removes the `resolve()` from the
innermost filter entirely, and is **correct on `no_std`**. A `no_std` `OnceLock`
substitute would need `spin`-style or `Cell`-based caching, and `AbilityRef` is
behind a shared `&` with no interior mutability available on those targets — so a
genuine cache is **not available without changing the data structure.** The mask
is the right answer, and it is worth doing primarily for `no_std` and
secondarily for the atomics on std (§6.5).

**⚠ Safety requirement:** the mask must be **provably complete**, or a
`set_card_identity` card silently stops matching. Safest form: keep the existing
`resolve()` body as the fallback and add the mask as a **negative** pre-filter
only — `if ar.trigger_mask & SET_CARD_IDENTITY_BIT == 0 { return false }` before
resolving. If the mask is ever incomplete that is still a behaviour change, so
build it from the same `AbilityTrigger` enum the decoder uses and add a
`debug_assert!` that mask-set ⊆ resolved-triggers over the whole corpus.

**Scale:** `no_std` only, and it is the dominant condition-path cost there.
**Effort:** medium. **Risk:** medium (completeness). **Outcomes:** on `no_std`,
must be gated by a console build + a full-suite run under a `no_std` feature set.

### 12.2 `ArcStr` becomes a bigger problem without atomics

`Arc<str>` on a target without `target_has_atomic = "ptr"` is not an `Arc`; the
`no_std` targets use a different strategy. **The interning proposal (§6.4) is
strictly more valuable here** — 784 bytes saved per `EffectFilter` out of ~1,234
is a 63% reduction, and on a 1–4 MB RAM target (CD-i is 1 MB, Genesis 64 KB work
RAM) that is the difference between fitting and not.

**The serde caveat does not apply** on the `no_std` targets, because
`serde_support` is disabled there — which removes most of §6.4's cost as well as
all of its risk. **This is the cheapest place in the whole document to try
interning**, and it should be tried there first.

### 12.3 Console-specific allocations the desktop does not have

- `AbilityResolver` is a ~1.9 KB `Box` per resolution
  (`core/game_state/abilities.rs:2047-2050`, `resolver.rs:389-393`) with ~100
  `Option<Vec<_>>` fields. On a 1 MB target that is a real per-resolution
  allocation pressure that the desktop does not feel. **Reusing one resolver
  across a game's resolutions** (a `GameState`-owned scratch) would be a
  console-only win. **Risk: medium** — the resolver has a self-referential
  snapshot design (§6.9); prove the reuse is sound or scope it per-game.
- `compact_state` caps log/application `Vec`s to bounded sizes on the smallest
  targets, but the desktop builds do **not** have it, so `rule_log` and friends
  are uncapped there. Not a desktop cost (they are gated), and not to be changed.

### 12.4 Do not enable `headless` for the benchmark without a gameplay gate

`Cargo.toml:51` defines `headless`, absent from `default`, with 6 `cfg` sites
(`core/game_state/mod.rs:1435`, `modifiers.rs:693`, `turn/phases.rs:32`/`:51`,
`turn/actions/mod.rs:860`/`:868`). It compiles out `choice_resolved` capture and
phase-transition log entries.

**It would gain almost nothing on the benchmark** — `sim_bench` already sets
`logging_enabled = false`, so those paths already do not execute, and
`engine/.cargo/config.toml:9` sets `-C linker=rust-lld`, which dead-strips
unreferenced COMDATs from the `game/display.rs` / `menu.rs` / `platform_ui.rs`
that are compiled but unreferenced. The residual is build time, not icache.

**And it can change `total_actions`.** The feature doc claims gameplay-relevant
tracking is retained, but that is a claim, and the combined doc §6 documents a
code removal that silently changed which game was played. **Gate it on
`total_actions`, not on the test suite.** Not recommended.

---

## 13. Category K — build configuration and codegen

**Every item in this section is un-gateable by the allocation contract**, because
no build change alters the abstract work done: `alloc calls` must be
**bit-identical** and `total_actions` unchanged. That is a useful property — it
makes this the only class of change in the whole programme that can be validated
deterministically on this box while gps is unreadable. It also means **none of it
can be confirmed here.** Impacts below are codegen-level certainties, not
measurements.

### 13.1 `[profile.bench]` is missing, and it silently measures different code

`engine/Cargo.toml:130-136` declares a criterion bench
(`[[bench]] name = "performance"`, `required-features = ["profiling"]`) and the
comment says to run `cargo bench --features profiling`. **That command compiles
the engine with different settings than `sim_bench --release`.**

| | release | bench (cargo default) |
| --- | --- | --- |
| `opt-level` | 3 | 3 |
| `debug-assertions` | **false** | **true** |
| `overflow-checks` | **false** | **true** |
| `debug` | false | true |

Concrete consequences: **`core/zones.rs:253`** has a
`debug_assert!(self.invariant(), …)` inside `Stage::get_area` — a stage-slot
primitive that fires on every stage read. **`core/zones.rs:268` and `:275`** have
two more in `Stage::set_area`, so `invariant()` runs twice per write. Five more
hot-path `debug_assert!`s at `core/game_state/mod.rs:696`, `turn/phases.rs:78`,
`turn/actions/mod.rs:466`, `bot/strategy_v7.rs:2761`. And `overflow-checks = true`
inserts a panic branch on every `u8`/`u16`/`i16` add/sub/shift — `CountCast`
(`core/constants.rs`) and the score/resource arithmetic are dense with exactly
that. Repo-wide there are 11 `debug_assert!`s, **7 on engine hot paths**.

```toml
[profile.bench]
inherits = "release"
debug-assertions = false
overflow-checks = false
```

**Scope, stated honestly: this does nothing for the canonical metric.**
`sim_bench` is built with `cargo build --release` and is already unaffected. It
fixes the *criterion* harness, so criterion numbers and `sim_bench` numbers
describe one artefact — which matters because a profiling harness that measures a
different binary than the one you ship produces attributions that do not transfer.
**Do it first anyway: it is free and it is a measurement bug.**

**Outcomes:** cannot change.

### 13.2 `#[inline]` discipline: 123 in the crate, 74 of them in one file

Counted across `engine/src` (159 `.rs` files, 5.35 MB):

| File | `fn` declarations | `#[inline]` |
| --- | --- | --- |
| `src/core/card.rs` | **260** | **0** |
| `src/core/zones.rs` | **92** | **0** |
| `src/core/game_state/abilities.rs` | **92** | **0** |
| `src/ability/util.rs` | 104 | 5 |
| `src/core/game_state/mod.rs` | 71 | 1 |
| `src/ability/condition/card.rs` | 61 | **0** |
| `src/ability/enums.rs` | **38** | **0** |
| `src/game/game_setup.rs` | 38 | **0** |
| `src/ability/resolver.rs` | 37 | **0** |
| `src/core/player.rs` | 37 | 9 |
| `src/core/types.rs` | 35 | **0** |
| `src/ability/ability_store.rs` | 6 | **0** |

Repo-wide: **123 `#[inline]`, 0 `#[inline(always)]`, 0 `#[cold]`, 0
`#[likely]`/`#[unlikely]`, 0 `core::intrinsics::{likely, cold}`.** 74 of the 123
live in one file (`core/constants.rs`, the `CountCast` trait). **The discipline
is not systematic — it exists where someone already noticed a cost and stops
dead at the crate's data layer.**

**A1 — the highest-value `#[inline]` in this document is cross-crate.**
`sim_bench` is a separate `[[bin]]` target (`Cargo.toml:154-157`), i.e. a
separate crate from the `rabuka_engine` rlib. With `lto = false`, rustc performs
**no cross-crate codegen**. Only functions whose MIR was exported (`#[inline]`, or
generic) and generics cross the boundary — and the engine is overwhelmingly
non-generic.

The functions `sim_bench` calls **directly from its own inner loop**
(`sim_bench.rs:274-473`, bounded at 600 iterations per game) are therefore all
opaque calls, and several are tiny:

| called from | location | size |
| --- | --- | --- |
| `GameState::has_pending_choice` | `core/game_state/abilities.rs:2312` | 2 calls + `Option::is_some` |
| `GameState::can_player_act` | `core/game_state/mod.rs:462` | small |
| `GameState::active_player` | `core/game_state/mod.rs:703` | match over `Phase` |
| `game_setup::auto_advance_one` | `game/game_setup.rs:382` | moderate |
| `check_victory_condition` | `turn/actions/mod.rs:1654` | large — **do not** inline |
| `generate_possible_actions` | `game/game_setup.rs:634` | large — **do not** inline |
| `bin_common::execute_and_settle` | `bin_common.rs:87` | large — **do not** inline |

`core/game_state/abilities.rs` has **0** `#[inline]` across 92 fns;
`core/game_state/mod.rs` has **1** across 71.

**Fix: three `#[inline]` attributes.** This costs nothing, changes no semantics,
and is the only mechanism available short of LTO that lets the bench's inner loop
see into the engine. **Restrict to bodies under ~10 statements** — `#[inline]` on
a large function is a pessimisation.

**A2 — intra-crate: MIR inlining on one-instruction bodies.** With
`codegen-units = 16`, ThinLTO across a crate's own CGUs already permits
cross-module inlining of non-generic functions, so `#[inline]` here is a
threshold/layout effect rather than a hard-barrier removal. **But on a body that
is literally `matches!`, MIR inlining alone is decisive** — there is no threshold
question when the callee is one discriminant compare. The functions:

| Function | Location | Body | Call sites |
| --- | --- | --- | --- |
| `Card::is_member` | `core/card.rs:4218` | `matches!(self.card_type, CardType::Member)` | 21 |
| `Card::is_live` | `core/card.rs:4222` | `matches!(…, CardType::Live)` | 11 |
| `Card::is_energy` | `core/card.rs:4226` | `matches!(…, CardType::Energy)` | 16 |
| `Player::contains_card` | `core/player.rs:651` | 6 `Vec::contains` | 12 |
| `AbilityRef::resolve` | `ability/ability_store.rs:45` | `OnceLock::get` → `Arc::clone` | 50 |
| `Zone::from_str` | `ability/enums.rs:26` | **30-arm** `&str` match | **146** |
| `Zone::to_str` / `as_str` | `ability/enums.rs:125` / `:174` | 30-arm match | paired |
| `norm_group_name` | `ability/util.rs:639` | 2× `contains` + conditional `replace` | 9 |
| `eq_ignoring_whitespace` | `ability/util.rs:667` | dual `Chars` filter + loop | 3 |
| `Stage::get_area` / `set_area` | `core/zones.rs:252` / `:267` | index match + slot read/write | very hot |
| `u8_count` (`CountCast`) | `core/constants.rs:157,194,229,264,303,363,398,443` | clamp/saturate | already `#[inline]` |

**The 46 `is_member`/`is_live`/`is_energy` call sites are not cold code.** They
sit in `turn/actions/mod.rs:1715-1850` (action generation — the bench's 1:1
`generate_possible_actions` call), `game/game_setup.rs:1218-1220, 1765`,
`core/zones.rs:359, 462, 654, 662`, `ability/condition/card.rs:1212, 1254, 2382,
2428`, `ability/condition/state.rs:746, 1405, 1407`,
`core/game_state/modifiers.rs:312, 846` — several inside `.filter()` / `.find()` /
`.position()` closures over zone card lists, i.e. **per-card, per-scan**. A
one-instruction discriminant compare is currently a real `call`/`ret` pair through
a non-inlined symbol in 46 places.

**A3 — what is NOT an `#[inline]` problem, so nobody wastes a session.** A
`#[inline]` on `CardDatabase::get_card` (`core/card.rs:505`) would remove the call
but **not** the cost, which is the SipHash-1-3 on an `i16` plus a dependent L1
miss on the `Card`. That is §6.13's dense-index work. `#[inline]` is a legitimate
add-on to that work, not a substitute, and swapping the hasher is explicitly
closed (§1.2).

**Scale:** A1 low single digits (unmeasurable here); A2 low single digits.
**Risk:** code-size growth if applied to large functions. **Outcomes:** cannot
change. (`#[inline]` *can* alter floating-point results in principle via
reassociation — irrelevant here, there is no FP in the rules path.)

### 13.3 A `bench_no_log` feature — the only pure **code removal** in this document

`engine/Cargo.toml:12`: `log = { version = "=0.4.22", default-features = false }`.
`default-features = false` on `log` drops `std`, not the level features; **no
`max_level_*` feature is enabled**, so `STATIC_MAX_LEVEL` is `Trace` — every
level is statically live.

No logger is installed in `sim_bench` (the only `env_logger::init()` sites are
`src/main.rs:34-35`, `src/bin/harness.rs:13-14`, `src/bin/rabuka_3ds.rs:11-12`).
`log::max_level()` reads `MAX_LOG_LEVEL_FILTER`, a `static AtomicUsize` initialised
to `0` (`LevelFilter::Off`) that `sim_bench` never sets. So every `log::debug!`
evaluates a relaxed atomic load, a compare, and a never-taken branch.

**LLVM *could* fold that atomic to its initial value and delete all 638 sites. It
does not**, because with `lto = false` the atomic sits behind a cross-CGU boundary
within the crate and there is no inter-CGU atomic constant propagation without LTO.

| Metric | Count |
| --- | --- |
| `log::trace/debug/info/warn/error!` sites | **638** |
| of which `log::debug!` | **561** |
| `#[inline(always)]` anywhere in the crate | 0 |
| `#[cold]` anywhere in the crate | 0 |

By file (all on the measured path): `ability/choice.rs` 55,
`ability/effects/move/mod.rs` 48, `core/game_state/abilities.rs` 43,
`ability/condition/card.rs` 38, `turn/live.rs` 37, `ability/effects/resource.rs` 26,
`ability/condition/state.rs` 24, `ability/cost/handlers.rs` 24,
`core/game_state/modifiers.rs` 21, `turn/actions/mod.rs` 18, `ability/resolver.rs` 18.

**Two costs, and the second is underestimated.** The never-taken branch on the hot
ones. **And the code behind the branch still exists** — the `format_args!` /
`core::fmt::Arguments` machinery is compiled regardless, so all 561 `debug!` sites
contribute to `.text`, to link time, and to the instruction footprint that evicts
what you *do* want in L1i. **There is no `#[cold]` anywhere in this crate** to
mark any of them, so LLVM has no signal that these regions are unlikely.

**Fix:**

```toml
# `log`'s compile-time level cap. `release_max_level_off` applies only when
# debug_assertions are off, so debug builds and `cargo test` keep every
# log::debug! — which AGENTS.md depends on as a first-class diagnostic.
bench_no_log = ["log/release_max_level_off"]
```

```powershell
cargo build --release --features bench_no_log --bin sim_bench
```

**What it does:** `STATIC_MAX_LEVEL` becomes `Off`, `lvl <= STATIC_MAX_LEVEL`
folds to `false` at compile time, and **the entire call — including the
`format_args!` machinery — is deleted at codegen.** All 638 sites vanish. Zero
runtime branches, materially smaller `.text`.

**This is the only item in the whole document whose mechanism is *removal of
code* rather than *better code*,** which is why it ranks above `panic = "abort"`
and above the `#[inline]` work.

**Cost/blast radius:** you lose `log::debug!` output **in benchmark builds only** —
which is exactly the right place to lose it, since AGENTS.md says debug is for
failing tests only, and the full suite runs `cargo test --test run_all` under the
dev profile with default features, so all 561 traces remain available where they
are actually needed.

**⚠ Note that this changes the canonical measurement command.**
`docs/PERF_ENGINE_COMBINED.md:35` records a bare `cargo build --release --bin
sim_bench` as *the* contract. Deciding whether the canonical bench uses
`bench_no_log` is a real decision, and it should be made explicitly and written
down — otherwise a future comparison silently mixes a `--features` build against a
default one and the `alloc calls` numbers are incomparable (they would be, in
fact, identical, which is the point — and that is exactly the trap).

**Outcomes:** cannot change, **provided** `debug_conditions` is not touched at the
same time. The 49 `debug_conditions` sites in `condition_decoder_gen.rs` are a
*feature*, not a *level* — they are code, not logging, and `max_level_off` does
not reach them. That is what makes the feature approach safe where "just remove
the log lines" would not be.

### 13.4 `panic = "abort"` — the one profile flag with a real mechanism

Absent from `engine/Cargo.toml`. It removes landing pads and unwind cleanup
edges, and this engine has enormous drop glue (every `Vec`/`SmallVec`/`Arc` free
path is an unwind edge today), so the compiler must keep those cleanup paths
reachable.

**Scale:** low single digits, plausibly real, unverifiable here.
**Risk, and it is not zero:** `src/bin/bot_arena.rs:2841` uses
`std::panic::catch_unwind`. Under `panic = "abort"` that call site stops working
— `bot_arena` would abort the process instead of returning `Err`, destroying the
paired-statistics tooling that `Cargo.toml:214-218` describes. Cargo ignores
`panic` for `test`/`bench` targets, so **AGENTS.md's `cargo test --test run_all`
workflow is safe**; the blast radius is exactly one bin. **Gate it on
`bot_arena` still running.**

### 13.5 `lto = "thin"` alone is *not* the closed item

The combined doc closed `lto = "fat"` **bundled with** `codegen-units = 1`. The
two effects were never separated, so `lto = "thin"` alone — and `codegen-units = 1`
alone — are **untested**. `thin` targets the cross-crate gap in §13.2 A1 directly
and is the cheaper of the two to pay.

**And the combined doc's stated mechanism for the `--jobs` regression has no
mechanism as written.** `codegen-units` is a build-time parameter and `--jobs` is
a `sim_bench` runtime flag (`sim_bench.rs:55, 105, 115`) that spawns workers via
`std::thread::scope` (`sim_bench.rs:672`). They cannot interact. The plausible
real mechanism is **code layout**: fewer CGUs means LLVM has less freedom to
reorder and cluster hot functions, so at high `--jobs` the larger per-CGU text
footprint contends harder for L2/L3 between workers. That is a genuine effect, but
it is not "less codegen parallelism leaking into runtime", and the write-up should
say so — otherwise the next person to revisit this reasons from the wrong model.

**Cost/benefit runs the opposite way from how the doc frames it:**
`codegen-units = 1` **serialises rustc's codegen for this crate to a single
thread**. On a box where the doc says other work is already competing for CPU,
that is a build-time cost paid on every iteration — and §0's guidance is
explicitly *"if you are here to make the benchmark finish sooner, do that
first."* `codegen-units = 1` works directly against that.

### 13.6 Explicitly no-op — do not spend a measurement on these

| Proposal | Why it is a no-op |
| --- | --- |
| `[profile.release] opt-level = 3` | **Already the cargo default.** The most common cargo mistake. |
| `debug = false`, `overflow-checks = false`, `debug-assertions = false` | Already the release defaults. |
| `[profile.release.package."*"] opt-level = 3` | Cargo already applies `opt-level = 3` to every package, dependencies included. |
| `strip` | `debug = false` already, so there are no debug sections to strip. |
| `incremental` | Already `false` in release. |
| `[profile.dev.package."serde_derive"] opt-level` | Already at 3 via the existing `build-override`. |

An explicit `[profile.release]` is worth adding **only** as documentation and as a
place to hang `panic = "abort"` (§13.4) — the other flags are byte-identical to
the defaults.

### 13.7 Dead dependencies: build-time only, and that is all

- **`env_logger`** — optional dep, in `default` (`Cargo.toml:13, 33`), so every
  engine build compiles it **and its whole tree** (`humantime`, `jiff`/`time`,
  `termcolor`, `is-terminal`, `anstream`…). `sim_bench` never calls
  `env_logger::init()`. **Compiled but not linked** → zero runtime cost, zero
  icache cost, no `max_level` atomic installed. **The cost is build time** — a
  dead dependency tree recompiled on every clean build, on a box where the doc
  says build time is the thing people notice.
- **`rand`** — same shape, but cleaner: all `rand::` uses are in **bins only**
  (`ppo_collect.rs`, `web_server.rs`, `trace_game.rs`, `bot_data_gen.rs`,
  `calibrate.rs`, `bot_demo.rs`, `profile_target.rs`). **Zero uses in the
  library.** Compiled, not linked.
- **`serde_json`** — genuinely **required**: `sim_bench.rs:35, 38-39` imports
  `replay`, `serde`, and `serde_json` unconditionally, and `replay` is
  `#[cfg(all(not(no_std), feature = "serde_support"))]` (`lib.rs:171-172`).
  `serde_support` is 719 `cfg` sites across `engine/src` — the largest feature
  surface in the crate. Runtime cost is nil (the bench's uses are inside
  `if opts.trace.is_some()` branches); codegen cost is real but bounded, and
  **lld dead-strips unreferenced COMDATs** at link time. Not worth touching.
- **`miniz_oxide`** — already minimal (`default-features = false`, `with-alloc`
  only). **Every call site is inside `get_decompressed_bytecode`**
  (`ability/vm.rs:186-207`, the desktop path being a `OnceLock::get_or_init`).
  One inflate of ~92 KB, **once per process**, before `setup_secs` is even
  stamped. **Zero throughput impact.** This dependency is correctly configured.
- **`rmp-serde`** — `core/card_loader.rs:122`, `core/game_state/mod.rs`,
  `bot_arena.rs`. Startup-only; `sim_bench` calls `load_cards_from_file`, not the
  msgpack path.
- **`bytecode_abilities`** — only **1** `cfg` site in `engine/src`; the actual
  gating in `ability/abilities_gen.rs:14-17` is on `snes`/`gba`, not on this
  feature. On desktop the compressed-blob path is unconditional and the flag is
  **inert**. Nothing to tune.
- **`compact_card_data`** — 10 sites, all in `core/card_loader.rs` / `core/mod.rs`.
  `sim_bench` calls `load_cards_from_file` under it, so the bench reads baked
  per-deck blobs rather than `cards/cards.json` — **faster startup, outside the
  timed region** (`sim_bench.rs:781-787` measures `setup_secs` separately).

**Action:** drop `env_logger` and `rand` from `default` and add them as
explicit features on the three bins that call `init()`. **Build time only. Do it
if build time is an active complaint; do not expect a benchmark move.**

### 13.8 PGO — real mechanism, unmeasurable here, and the honest prior is "small"

`-Cprofile-generate` → run the bench → `llvm-profdata merge` → `-Cprofile-use`.

**Three Windows constraints:** `llvm-profdata.exe` must come from the **rustup
toolchain's** bundled LLVM
(`%USERPROFILE%\.rustup\toolchains\<tc>\lib\rustlib\x86_64-pc-windows-msvc\bin\llvm-profdata.exe`),
not a system LLVM, or `-Cprofile-use` rejects the profile. The instrumented build
is meaningfully slower. And **the profile must be collected on the same
seed/deck/policy you intend to measure**, or the hot-path weights are wrong —
which the bench's determinism makes easy.

**And note this conflicts with the closed LTO item in spirit:** PGO's main win is
better inlining and layout, the same mechanism as fat LTO. The prior should be
low single digits.

**Scale:** low single digits. **Effort:** medium. **Risk:** zero correctness risk;
the result is non-portable and cannot be validated here. **Outcomes:** cannot change.

### 13.9 Do NOT set `-C target-cpu=native`

`engine/.cargo/config.toml:8-9` has exactly one rustflags entry
(`-C linker=rust-lld`). Three hazards:

1. **It would break the platform builds.** `platforms/*` each have their own
   `.cargo/config.toml` and build for m68k, PowerPC, MIPS, wasm32 and ARM. A
   shared `native` rustflags entry is a host-cpu instruction set leaking into a
   cross build.
2. **`RUSTFLAGS` in the environment silently overrides `target.*.rustflags` from
   config.** Setting `RUSTFLAGS="-Ctarget-cpu=native"` in your shell to try this
   **drops `-C linker=rust-lld`**, changing the linker from lld to `link.exe` and
   changing dead-stripping behaviour. **Any A/B run done that way is comparing two
   different links and is invalid.**
3. **Portability.** A `native` binary is not comparable to any recorded baseline.

If you want to try it, the only safe form is a local, reverted
`--config 'target.x86_64-pc-windows-msvc.rustflags=[…]'` on the command line.
Given §1's noise floor, **do not.**

BOLT is not shipped with the rustup toolchain, needs a non-default linker
pipeline, overlaps PGO's mechanism, and is not verifiable here. **No.**

### 13.10 One workspace-robustness note, zero perf impact

There is **no root `Cargo.toml`** — the root `Cargo.lock` is a stale orphan
(337 packages, last written 2026-08-20) and `engine/Cargo.lock` (2026-09-27) is
the live one. Every `platforms/*` port has its **own** `Cargo.lock`, so there is
no lockfile contention. But only `ps1`, `genesis`, `jaguar`, `cdi` and `snes`
carry an explicit `[workspace]` table; `wasm`, `gba`, `ds`, `psp`, `dc`, `wii`
and `3ds` rely on the absence of a root manifest. **If anyone ever adds a root
`Cargo.toml`, seven ports silently become workspace members and start failing on
features they do not declare.** Adding `[workspace]` to those seven is cheap
insurance. Do not delete the orphan root lockfile as part of a perf change — it
is inert, and deleting it is out of scope.

---

## 14. Category L — threading and scaling

The combined doc §9 is the largest result in the programme and the least
engine-shaped: `--jobs 128` buys **+17% over 12 threads** and cuts run-to-run
spread from ~30% to ~8%. That is benchmark throughput, **not** engine speed.

**6.0× on 6 physical cores is ~100% of the machine. The CPU is done.** Every
further gain has to come from making a single action cheaper, i.e. §§3–11.

### 14.1 The one real multithreading cost: `Arc` refcount contention

`GameState.card_database` is an `Arc<CardDatabase>` shared by every worker,
cloned on every card play (`turn::phases::handle_play_member_to_stage`) plus once
per ability resolution. That is an atomic increment on a **single cache line**
from 12 cores, tens of thousands of times a game. Invisible at `--jobs 1`; it
appears only at the top of the thread range — **which is why three profiling
rounds, all of which mandated `--jobs 1`, never saw it.**

**Fix:** the card database is read-only for the whole run. `Box::leak` it once at
startup and store a `&'static`, or thread the reference through. **Zero refcount,
zero atomics.** This is the single highest-value *scaling* fix and it is not a
micro-optimisation — it is removing a contended write from the hot path.

**The same applies to `PlayerId(Arc<str>)`**, whose conversion from `String`
traded independent allocations for shared refcount bumps (a good trade — it removed
an allocation per comparison — but it moved traffic onto the contended line).
Note that §6.7's proposal to change `AbilityQueueEntry.player_id` to `PlayerId`
*adds* refcount traffic at the enqueue site while removing a heap allocation at
the 199 `resolve_target_player` sites. **At `--jobs 1` that is a clear win; at
`--jobs 128` it needs measuring.** File it under §13's rule: **measure
single-threaded and threaded separately**, because the two answers can differ.

### 14.2 `AbilityRef::resolve()` refcount traffic compounds the problem

§6.5 estimates **~200–300 atomic RMWs per action** on 2–3 shared cache lines.
**This is the item that gets worse with `--jobs`, not better**, and it was never
measured threaded because every profiling round mandated `--jobs 1`. The fix
(`peek() -> Option<&Ability>`, zero atomics) is therefore **more valuable at high
`--jobs` than the single-threaded estimate suggests** — plausibly 2–3% at
`--jobs 12` versus 0.5% at `--jobs 1`.

### 14.3 There is no intra-game parallelism to exploit

A single game is a strictly sequential turn loop. The doc records **2 jobs =
1.81×, not 2.0×** — a single game already keeps a core ~90% busy, so there is
only ~10% stall headroom, and the OS gives you that for free via process sharding.
This closes the "run several games interleaved per thread for instruction-level
parallelism" idea: **the ceiling is ~10%**.

### 14.4 GPU: no, and not close

The doc's reasoning stands and is worth restating so nobody re-litigates it: a
rules interpreter is close to the worst possible GPU shape — unpredictable
branching (which ability fires, in what order), pointer chasing through
`HashMap`/`SmallVec`/`Arc<str>`, and 7M host allocations per run. The only
theoretical path is batching N independent games into a struct-of-arrays lockstep
layout, which is a **rewrite of all ~2,280 cards' abilities, not an optimisation.**
If the training loop runs a neural policy, **batch that** instead — the engine
stays on CPU.

### 14.5 SMT is worth 1.27× (6→12 threads)

More than expected for a branch-heavy, allocation-bound workload, and still
nowhere near 2×. Recorded for completeness; no action.

---

## 15. Category M — the deliberately-large projects

The combined doc §10 concludes: *"If the two largest **available** dead
allocations were worth 0.7% combined, the remaining ~6.9M are the long tail …
each is individually safe and the set might reach 5–8%, but it is dozens of
mechanical edits across rules code for less than the `--jobs 128` flag. Do it as
one deliberate project or not at all."*

**This catalogue agrees with that verdict and is organised to enable it.** The
items that are *not* mechanical are collected here, each with what it depends on
and what gates it.

### 15.1 "Materialise the immutable card-derived indexes at load time"

**The keystone project.** Everything in §7.2, §4.10, §6.13, and the `no_std`
half of §12.1 depends on it, and `CardDatabase::load_or_create`
(`core/card.rs:472-503`) **already builds one such index** (`normalized_no_to_id`)
with a comment recording why.

Deliverables: `names`, `group_mask`, `identity_aliases`, `suppress_mask`,
`trigger_mask`, `cards_by_id` (§7.2's table).

**Properties that make it the right shape:**
- **Provably outcome-neutral** — `CardDatabase` is immutable during play; the only
  mutation is `create_copy` (`:462-469`), setup-time only.
- **A side table in `CardDatabase`, not a new `Card` field** — keeps the wire
  format and `test_blob_matches_json` untouched, which is what makes this
  tractable at all.
- **It is a precondition, not just an optimisation** — §7.1's fix is partial
  without it, because four separate group matchers reach the database by different
  routes (§7.2's table of implementations 1–5).

**Gate:** `total_actions` fixed + `create_copy`/empty-`CardDatabase` coverage with
a `#[cfg(test)]` invariant check, because a missed site is a **silent wrong
answer**, not a crash.

### 15.2 Interning the closed vocabulary — try it on `no_std` first

§6.4, in full. On a `no_std` target the serde caveat disappears entirely
(`serde_support` is disabled there), removing most of §6.4's cost *and all of its
risk*, on a platform where a 63% reduction in `EffectFilter` is the difference
between fitting in 1 MB and not. **Then** attempt the desktop version behind a
hand-written `Serialize`/`Deserialize` that resolves through the table.

### 15.3 `EffectOwner` provenance

§5.4. The only item in this document that needs a **new test harness** before it
can be attempted safely, and the only one where the doc at `resolver.rs:102-119`
warns that the change could reintroduce an entire recorded bug class. **Do the
dead-write audit first** (§10.4) — it may shrink the problem for free.

### 15.4 Replacing the derived-identity string with a `(card, index)` pair

§9.9. Medium-high effort, **can** change outcomes, and it carries a latent
collision bug that is worth reporting independently. Deliberate project only.

### 15.5 `Option<Box<CompoundBranch>>` → `Option<…>`

Still deferred per the combined doc §5, and this catalogue does not overturn it.
**But land the clone-*site* removals first** (§§5.2, 5.3, 5.6) — they are
wire-transparent, they capture a meaningful fraction of the benefit, and they
reduce the justification for a 60-site wire-format diff.

### 15.6 Caching `recalculate_constants` — the honest re-assessment

The combined doc §7/§11 closed this as "4.1% ceiling, not worth it", and §11
diagnosed the real obstacle precisely: `active_energy_count` is written from ~12
raw field-assignment sites plus a self-referential read of `mods`' own output that
any version-counter scheme must prove terminates.

**§4.1 and §4.3 show that the *stage* half of that problem is much easier than
the doc's general framing suggests.** `Stage.stage` is written through
`set_area` (`zones.rs:267-276`), `stage_member_for_check_timing` (`:278-292`),
`move_card_from_hand_to_stage`, and a handful of position-effect handlers — a
small, enumerable set, **not** the 12 divergent writers that made
`active_energy_count` hard. So:

- **Do the local, no-invalidation versions** (§4.3's plumbing option,
  §4.1's `entry_positions` widening) — they need no termination proof at all.
- **Only then** consider a version counter, and only for stage-derived values.
- **Leave `active_energy_count` alone.** Its 4.1% ceiling did not move, and that
  is the number that governs the whole idea.

---

## 16. Checked and found already correct

Recorded so nobody re-audits them. Each was read, not assumed.

| Item | Verdict |
| --- | --- |
| **Zone storage** | All inline, zero-allocation in steady state (~292 B/player). Well done — leave alone (§6.1). |
| **Effect dispatch** | A dense-enum jump table. No fallthrough, no linear scan, no arm-count problem (§10.6). |
| **Condition dispatch** | Already a jump table; an opcode list would buy ~nothing given the 414/109/3 depth histogram (§8.3). |
| **`recalculate_constants` recursion** | `refresh_yell_sources` calls it exactly 2×/play, gated on `restored == restored_all`. Correct shape. |
| **`scratch_*` buffer idiom** | Correctly applied to the three `exp_*` maps. §3.8 finishes the set. |
| **`Timer` instrumentation** | `profiling` off by default; 0 `Instant::now()` outside `#[cfg(feature = "profiling")]`; the ~41 sites cost nothing. |
| **`log::debug!` arguments** | The macro **is** lazy — `sequential.rs:226`'s `collect::<Vec<_>>()` inside the macro is correctly deferred. The bugs in §3 are the `let` bindings the args depend on, not the macro. |
| **`push_ability_result`** | Correctly gated at `resolver.rs:986-990` before any of its ~7 `String`s are built. **This is the counter-example that proves the §3 bugs are bugs, not the codebase's normal state.** |
| **`zone_for_card`** | Returns an owned `String` after 12 scans — but its only caller `push_ability_result` is gated *before* the call. Already correct; no action. |
| **`EffectOwner` accessors** | `owning_effect` / `answering_effect` (`resolver.rs:320, 334`) already borrow. §5.4 is about the *writes*. |
| **`describe_condition_actual`** | Correctly gated at `condition.rs:592-600`. `evaluate_condition_debug` (`:680`) is `serde_support`-gated to the debug endpoint. |
| **`Gate tables`** | Confirmed landed as `const` fn-pointer arrays (`gates.rs:52-89, 255-261`), correctly `#[inline]`. |
| **`Condition` `Box`→`Arc`** | Measured negative. The `Condition` boxes are fine; `EffectFilter` (§5.1, §6.4) is the real representation cost. |
| **`PlayerId` comparisons** | `Arc<str>` `PartialEq` is a 2-byte `memcmp`, not a pointer compare. Nothing to win (§7.14). |
| **`miniz_oxide`** | One `OnceLock` inflate of ~92 KB per process, before timing starts (§13.7). |
| **`describe.rs`** | The only semi-hot caller is `choice.rs:2690` (`describe_effect_en`, a 44-arm `String`-building match) on the re-prompt branch. Not a target. |
| **Memory pressure** | No growth while games run. All resident card data ~2.4 MB. The allocator share is CPU time, not memory. |
| **`AbilityResolver` field accessors** | `owning_effect`/`answering_effect` already fixed. |

---

## 17. Recommended sequencing

Two gates apply throughout, per §1.1: **allocation count with `total_actions`
fixed at the current trajectory**, and — for anything that can touch iteration
order, map keys, or a gameplay predicate — **a fixed-seed win-rate comparison
over a large `--games` count, not the test suite.**

### Phase 0 — measurement, before any optimisation

| # | Action | Why first |
| --- | --- | --- |
| 0.1 | Add the TAS call counter (§2.1) | Converts the biggest open argument into a fact. Decides whether §11.1/§11.2 matter. ~15 min, zero cost. |
| 0.2 | Add the per-site `can_activate_effect` counter (§2.2) | Distinguishes "slow evaluator" from "called 4× per decision". |
| 0.3 | Re-baseline: `total_actions` and `alloc calls` on the current tree | The doc's figures are stale after the 2026-09-29 reorganisation. |
| 0.4 | Add `[profile.bench]` (§13.1) | Free, and it is a measurement bug. |

### Phase 1 — mechanical, zero-risk, individually measurable

All provably outcome-neutral. Land as one batch behind the allocation gate, or
one commit each with its own A/B.

| # | Item | Est. allocs/action | Effort |
| --- | --- | --- | --- |
| 1.1 | `waited_stage_cards` → `SmallVec<[i16;3]>` (§4.6) | ~3 | trivial |
| 1.2 | `find_printed_ability` double-resolve (§9.9) | ~1 | 1 line |
| 1.3 | `ChoiceRoute` stringification ×2 (§9.5, §9.6) | ~2 | trivial |
| 1.4 | Gate `push_cond_verdict` + de-triplicate (§3.1) | several/verdict | trivial |
| 1.5 | Gate `push_effect_verdict` (§3.2) | 3–4/effect step | trivial |
| 1.6 | Gate `pay_ability_cost`'s `cost_desc` (§3.3) | 2–3/costed ability | trivial |
| 1.7 | The 21 direct `gs.rule_log` writes (§3.6) | up to 3/effect | 21 edits |
| 1.8 | Gate `rule_log_activated` (§3.5) | 2/logged action | trivial |
| 1.9 | The 6 condition-layer name lists (§3.10) | n+2/eval | trivial |
| 1.10 | Gate the `stats_pipeline` `HashMap` clone (§3.11) | 1 + 5 hashes/`stage_hearts` | trivial |
| 1.11 | Delete `hand_cost_matching_indices`' `match_names` (§3.9) | 1+N/hand cost | trivial |
| 1.12 | `requeue_remaining_after_choice` → `into_iter()` (§5.8) | ~32 × steps | trivial |
| 1.13 | Lazy `apply_modify_cost_to_ability_cost` clone (§5.5) | ~32/no-cost ability | trivial |
| 1.14 | `execute_modify_required_hearts_standard` → slice (§10.2) | 1+N/heart mod | low |
| 1.15 | `post_cost_position_gate` → no `Vec` (§4.15) | 1/resolution | trivial |
| 1.16 | `ResumeSnapshot::capture` — 10 lookups → 1 (§9.7) | 2/choice answer | 30 min |
| 1.17 | `ResolveSnapshot` / `Zone::from_str` hoists (§4.13) | 0 (branches) | trivial |
| 1.18 | `process_player_abilities_depth`'s `HashMap` → `SmallVec` (§4.14) | 1–2/drain | trivial |
| 1.19 | `set_indices_waited/active` buffers (§7.9) | 3/multi-energy payment | trivial |
| 1.20 | `performer_owned_ids` → three-zone scan (§4.7) | 1/performer | 30 min |
| 1.21 | `check_invalid_live_cards` → `SmallVec` (§7.5) | 2/`check_timing` | 20 min |
| 1.22 | `zone_card_ids`/`_occupied` → iterators (§6.2) | 2/zone query | low |
| 1.23 | `get_zone_cards` and friends → borrow (§6.3 S2, S4) | 1/zone eval | low |
| 1.24 | `counted_by_any().to_vec()` → borrow (§6.3 S1) | 1+N/card/count | trivial |
| 1.25 | `Active` phase: parse once, `SmallVec` not `HashSet` (§4.9) | 1/Active phase | 30 min |
| 1.26 | `execute_performance_phase`'s `nhm_flat` + partition (§11.2) | 3/performer | 1 h |
| 1.27 | `ArcStr::from(&str)` single-alloc (§6.6) | 1/decode | 1 line |
| 1.28 | `player_prefix()` → `player_prefix_id()` at 12 sites (§9.1 note) | 1/log line | low |
| 1.29 | `can_activate_effect` → `Option<&Ability>` pointer token (§8.2) | **~50% of condition evals** | low |
| 1.30 | Hoist `condition.common()` (§8.1) | 0 (branches) | small |
| 1.31 | `.count()` → `.any()` for monotone location clauses (§7.10) | 0 (scan length) | small |
| 1.32 | Hoist `filter_subset` out of per-card loops (§4.8) | 0 (alloc-free filters) | small |
| 1.33 | `count_side` guard for the weighted second pass (§4.8) | 1 zone filter | small |
| 1.34 | `ConditionContext` only when a condition exists (§8.5) | ~8/card play | small |
| 1.35 | `TriggerEvent` early-exit guard (§4.5) | ~4/card play | small |
| 1.36 | Finish the `recalculate_constants` scratch set (§3.8) | up to 14/call | low, **one field at a time** |
| 1.37 | Gate the 8 display-only accumulators (§3.7) | ~14/call | small |
| 1.38 | `entry_positions` carries the owner (§4.1, safe half) | 18 scans/entry | low |

### Phase 2 — the two largest single items

| # | Item | Est. impact | Effort | Gate |
| --- | --- | --- | --- | --- |
| 2.1 | **`ability_master_id()` → `Option<&str>`** (§9.1) | **199 call sites × 1 alloc + up to 12 scans** | low | `total_actions` + a debug-assert old-vs-new resolution diff |
| 2.2 | **`get_card_names` allocation-free + `normalize_name` borrowing** (§7.1) | **4 allocs × 67 call sites × per-candidate** | medium | exhaustive corpus equivalence test |
| 2.3 | `Store_pending_choice` gating (§3.4) | ~20/choice offer | low | `total_actions` + grep log assertions |
| 2.4 | `normalized_steps` + `prepare_sequential_action` borrow (§5.2, §5.3) | **~128/sequential ability** | medium | `total_actions` + corpus field-identity test |
| 2.5 | `filter_subset` / `CardFilter::from_effect` single-bind (§6.11, §6.12) | 200–300 dependent loads; 10 probes → 1 | medium | `total_actions` + full suite |
| 2.6 | `execute_move_cards` borrow its 8 `String`s (§10.1) | ~9/MoveCards | low | `total_actions` |
| 2.7 | `Option<&Ability>` in `AbilityRef` (§6.5) | ~200–300 atomics/action | small | measure at `--jobs 1` **and** `--jobs 128` |

### Phase 3 — the deliberate projects (§15)

Each needs its own gate and its own sign-off. Do not batch these.

15.1 card-derived indexes · 15.2 interning (try `no_std` first) ·
15.3 `EffectOwner` provenance (needs a new corpus harness) ·
15.4 `(card, index)` identity · 15.6 stage-scoped caching.
And the two already-deferred: `Option<Box<CompoundBranch>>` (after Phase 2.4),
and the 2.1-B kernel only if Phases 1–2 land first.

### Phase 4 — build config (§13)

`[profile.bench]` (0.4, free) → the three cross-crate `#[inline]`s →
`bench_no_log` (decide the canonical command explicitly) → `panic = "abort"`
(gate on `bot_arena`) → `lto = "thin"` alone → PGO if the machine ever becomes
measurable.

---

## 18. Appendix — how this catalogue was produced, and its confidence

**Method.** Read-only static analysis of `engine/src` (159 `.rs` files,
5.35 MB) across seven independent subsystem passes, plus a corpus census of
`cards/abilities.json` (1,278 unique abilities, 526 reachable condition clauses,
201 unique clause shapes, 985 abilities, ~2,280 cards) and a build-configuration
audit. **No benchmark was run and no file was modified.** Every `file:line` in
this document was re-verified against the source after the passes completed.

**Confidence tiers, stated honestly:**

| Tier | What it means | Which items |
| --- | --- | --- |
| **A — verified source facts** | Read directly and re-checked. Line numbers, code, and call-site counts confirmed. | Nearly all `Where` fields; the counts 199 / 67 / 50 / 638 / 857 / 34 / 26 / 146; the `recalculate_constants` scratch census; the 21 `rule_log` sites; the 11 `debug_assert!`s; the 526-clause corpus census |
| **B — derived** | Follows mechanically from a tier-A fact plus a documented cost model (e.g. "4 allocations per `get_card_names`" × "67 call sites" × "reached on the miss path"). | Every `Scale` field |
| **C — hypothesis** | Argues from code shape about runtime frequency or cost centre, and **should be measured before being acted on at scale**. | §2.1's 18–24 scans/turn; §6.4's 3–6%; §13.3's "low single digits"; §15.1's cumulative effect |
| **D — explicitly unverified** | The build-config items, because §1 makes wall-clock unmeasurable on this box. Cannot be confirmed here at all. | All of §13 |

**The most important caveat.** The combined doc §3 concludes there is no hotspot
and the profile is flat. **This catalogue does not overturn that**, and it should
not be read as claiming otherwise. What it does is decompose the flat profile into
its *mechanisms* — work behind disabled gates, loop-invariant re-derivation,
effect-tree clones, representation, and scans — and rank the items within each.
The ranking is the deliverable. **The measurement is still owed on every one of
them**, and §17 Phase 0 is deliberately sized to make that cheap.

**Two places where this catalogue corrects the existing record, both worth
flagging explicitly:**

1. **The combined doc §5's inference from the negative `Condition` `Box`→`Arc`
   result** — *"the clone path is done; the remaining ~380 allocations per
   `use_ability` call are in what `run_effect` builds, not in what the effect
   tree copies"* — is **half wrong**, and the wrong half is the expensive half.
   `Condition` boxes were the *top* of the tree; the ~164-field `EffectFilter`
   with 26 `Box`es underneath was untouched, and ~26 `effect.clone()` sites still
   pay ~4 allocations + ~1.7 KB each. §5 quantifies this.
2. **The "4× trigger scan = 2.7%" model is wrong by roughly an order of magnitude
   on the live phase**, because it omits the implicit scan in
   `process_current_ability` and the 18–24 scans the live phase issues per turn.
   The doc already flagged itself as undercounted; §2.1 gives the call-site
   census and the one-line counter that settles it. **Do not quote 2.7% × any
   factor** — that is a re-derivation, not a measurement.

**And one place where it reinforces the existing record with a correction to the
doc's reasoning rather than its conclusion:** the combined doc attributes fat-LTO's
`--jobs` regression to codegen parallelism interacting with thread count. Those
are a build-time parameter and a runtime flag and cannot interact. The plausible
mechanism is code layout (fewer CGUs ⇒ less freedom to cluster hot functions ⇒
more L2/L3 contention between workers). Same conclusion, wrong model — worth
fixing in the write-up so the next person does not reason from it.

