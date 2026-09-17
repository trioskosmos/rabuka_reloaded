# Flamegraph findings and optimization implementation plan

## Scope and status

This document consolidates the flamegraph investigation, real captures, visualization improvements, and proposed engine optimizations from 2026-09-17.

- Engine optimization changes described here are **proposals, not implemented changes**.
- The profiling work edited `engine/examples/gen_flamegraph.rs`, not engine gameplay source.
- The subsequent implementation investigation was read-only.
- Unrelated working-tree changes were present during the investigation and were left alone.
- Source references are to the inspected working tree around commit `d1993354`; line numbers may move.
- Successful game runs are workload evidence, not proof of behavioral equivalence or comprehensive engine correctness.

## 1. Original measurement problem

The timer records inclusive elapsed nanoseconds for every manually instrumented call path. The original flamegraph generator passed those values directly to Inferno, which interprets each folded record as an independent contribution.

For example:

```text
parent 100
parent;child 60
```

If the parent's 100 ns already includes the child's 60 ns, rendering those records unchanged gives the parent a combined width of 160 ns. Correct exclusive weights are:

```text
parent 40
parent;child 60
```

The generator now subtracts the inclusive totals of immediate children from each parent's inclusive total before rendering. It does not subtract all descendants, which would double-subtract deeper work.

Relevant source:

- `engine/src/timer.rs:62`: captures inclusive elapsed time.
- `engine/src/timer.rs:107`: the original textual timing percentages sum overlapping inclusive durations.
- `engine/src/timer.rs:135`: exports inclusive folded records unchanged.
- `engine/examples/gen_flamegraph.rs`: performs the corrected conversion outside engine code.

The engine timer and its original timing table were not changed. Use the corrected generator summaries rather than interpreting the timer table's original percentages as exclusive shares.

### Remaining measurement limitations

1. These are elapsed-time scopes, not sampled CPU stacks. Scheduling delays can contribute.
2. Self time means time outside explicitly instrumented children. It includes uninstrumented descendants and instrumentation overhead.
3. Timer operations include clocks, mutexes, stack cloning, and map updates; ancestor scopes include some descendant instrumentation overhead.
4. The timer stack is process-global. Mutex protection does not make nested scopes from concurrent threads independent. The inspected capture driver runs games sequentially.
5. Instrumented coverage is not whole-process elapsed time. Loading, setup, and other untimed work may sit outside the measured roots.
6. Enabling `profiling` suppresses some action metadata and cloning. It is not simply the normal build with timers added.
7. Invalid captures with missing inclusive parents or child totals exceeding a parent are rejected rather than silently clamped.

## 2. Flamegraph tooling improvements

Only `engine/examples/gen_flamegraph.rs` was changed for the visualization work.

Implemented capabilities:

- Inclusive-to-exclusive conversion by immediate-child subtraction.
- Explicit `--exclusive` mode for input already containing exclusive nanosecond weights; it is not a sample-count mode.
- Caller-oriented rendering by default.
- `--reverse` for leaf-first rendering.
- `--self` for a flat view aggregating identical leaf labels across callers.
- Rejection of the conflicting `--reverse --self` combination.
- Custom titles through `--title`.
- Accurate nanosecond labels and a subtitle distinguishing instrumented coverage from CPU samples and whole-run time.
- Stable name-based colors and larger frame height.
- Corrected self/inclusive path summaries and aggregated leaf summaries.
- Input checks for malformed weights, empty frames, missing parents, overlapping totals, overflow, and zero-duration captures.
- Handling of UTF-8 BOM, CRLF, comments, blank lines, and duplicate records.
- Prevention of using the same canonical file as input and output.

The folded input does not contain call counts. Per-call averages cannot be reconstructed from it; those remain available only in the separate raw timer table.

### Usage

From `engine`, the normal Cargo invocation is:

```powershell
cargo run --example gen_flamegraph -- INPUT.folded OUTPUT.svg
cargo run --example gen_flamegraph -- INPUT.folded OUTPUT.callee.svg --reverse
cargo run --example gen_flamegraph -- INPUT.folded OUTPUT.self.svg --self
```

These commands are usage examples, not a claim that normal Cargo builds are source-write-free. The engine build script can regenerate test module declarations. During this investigation, a temporary external Cargo manifest referenced the actual generator file and Inferno directly to avoid executing the engine build script.

### Tooling validation

Final generator validation passed:

- 11 regression tests.
- Strict Clippy with `-D warnings`.
- Cargo typecheck.
- Rustfmt check.
- Synthetic rendering and six real-capture SVG renders.

One new test initially used a fixture totaling 105 ns while supplying a 100 ns denominator. The fixture was corrected to total 100 ns. The final test verifies the rendered 40 ns / 40.00% child title and five aggregated leaf labels. Assertions were retained.

## 3. Real capture procedure and artifacts

### Initial executable probe

The existing release `profile_target.exe` ran games but produced no folded stacks. Its available build metadata did not enable `profiling`. The driver has a hardcoded 5,000-game workload and does not parse iteration or help flags.

Two initial non-instrumented probe runs were saved as:

- `test_output/flame_capture_20260917_a.folded`
- `test_output/flame_capture_20260917_a.stderr.txt`
- `test_output/flame_capture_20260917_b.folded`
- `test_output/flame_capture_20260917_b.stderr.txt`

Their folded files are empty and are **not usable flamegraph captures**. Their process times were approximately 48.55 s and 52.59 s. They are not a controlled performance baseline for the later profiling build.

### Instrumented build

An external temporary manifest referenced the unchanged workspace library and `profile_target.rs`, enabled default features plus `profiling`, and disabled the package build script. This avoided source-writing build-script side effects without modifying engine code.

Temporary manifest used:

```text
C:\Users\trios\AppData\Local\Temp\kilo\flame_capture_build\Cargo.toml
```

The resulting release executable was `C:\rust_targets\release\flame_capture_target.exe`.

Caveats:

- This was an isolated manifest with its own dependency resolution, not a locked reproduction of the repository package build.
- Build-script freshness checks were bypassed along with its source-writing behavior.
- Concurrent unrelated source work existed; the results describe the built executable, not a guaranteed clean-commit baseline.
- No finer-grained CPU sampling capture was taken.

### Usable instrumented captures

| Metric | Capture C | Capture D |
|---|---:|---:|
| Games | 5,000 | 5,000 |
| Actions | 1,289,149 | 1,298,357 |
| Process elapsed time | 52.94 s | 50.02 s |
| Corrected instrumented coverage | 49.258 s | 47.038 s |
| P1 wins | 2,482 | 2,499 |
| P2 wins | 2,460 | 2,459 |
| Reported permanent-loop draws | 58 | 42 |

Both processes exited successfully. The runs use random action selection and therefore differ in their exact workloads.

For each prefix below:

```text
test_output/flame_profile_20260917_c
test_output/flame_profile_20260917_d
```

The following artifacts were generated:

| Suffix | Contents |
|---|---|
| `.folded` | Raw inclusive timer capture |
| `.stderr.txt` | Outcomes, diagnostics, and raw timer table |
| `.caller.svg` | Corrected caller-oriented flamegraph |
| `.callee.svg` | Corrected leaf-first flamegraph |
| `.self.svg` | Flat aggregated self-time view |
| `.caller.summary.txt` | Corrected path summary |
| `.callee.summary.txt` | Corrected path summary accompanying reversed graph |
| `.self.summary.txt` | Aggregated self-time leaf summary |

PowerShell wrapped some native stderr output in diagnostic formatting. The actual executable exit status, outcomes, timing contents, and generated artifacts were inspected separately. Some output artifacts are ignored by Git and may not appear in ordinary tracked-file searches.

## 4. Measured priorities

These percentages use corrected exclusive weights. Aggregated leaves combine the same label across caller paths.

| Instrumented scope | C self time | C share | D self time | D share |
|---|---:|---:|---:|---:|
| `exec::play_member_to_stage` | 8.822 s | 17.91% | 8.427 s | 17.91% |
| `exec::resume_with_choice` | 8.029 s | 16.30% | 7.818 s | 16.62% |
| `generate_main_phase_actions` | 7.636 s | 15.50% | 7.148 s | 15.20% |
| `exec::use_ability` | 5.620 s | 11.41% | 5.370 s | 11.42% |
| `advance_phase::live_victory` | 3.400 s | 6.90% | 3.256 s | 6.92% |
| `execute_performance_phase` | 3.153 s | 6.40% | 3.018 s | 6.42% |
| `advance_phase::live_card_set` | 1.907 s | 3.87% | 1.828 s | 3.89% |
| `check_timing::recalculate_constants` | 1.780 s | 3.61% | 1.664 s | 3.54% |

The first four scopes account for approximately 61% of measured coverage in both runs. Their stable shares justify investigation, not a prediction that the proposed changes will remove that time.

Play, activation, choice resumption, performance, and victory all reach shared ability-resolution code. Uninstrumented shared work appears inside whichever timed wrapper called it. Do not attribute an entire wrapper's seconds to one allocation or lookup within that wrapper.

## 5. Recommended implementation sequence

| Order | Change | Scope | Complexity / behavior risk |
|---|---|---|---|
| 1 | Cache stage-group count within action generation | One function | Low |
| 2a | Remove `pending_entries()` temporary collection | One caller | Very low |
| 2b | Zero/one/many queue selection without intermediate index vector | One resolution function | Low–medium |
| 3 | Borrowed, stack-backed area descriptors | Action-generation internals | Low–medium |
| 4 | Construct performance snapshots with final enrichment fields | Builder and caller | Medium |
| 5 | Remove repeated candidate text-key formatting | Existing text lookup helper | Low, with exact matching |
| Defer | Direct numeric activation dispatch | Selection and enqueue semantics | Higher |
| Defer | Live-zone movement collection redesign | Victory and choice flows | Medium–high |

Start with local group caching and queue collection removal as separate, independently testable changes. Do not mix existing correctness issues into optimization patches.

## 6. Action generation: cache stage-group count

### Evidence

- `engine/src/game/game_setup.rs:1887`: stage abilities repeatedly calculate distinct stage groups.
- `engine/src/game/game_setup.rs:1997`: discard activation cost calculation repeats the count indirectly.
- `engine/src/core/game_state/abilities.rs:2412`: group counting checks canonical groups against stage cards.
- `engine/src/core/game_state/abilities.rs:2446`: `effective_activation_cost_for(cost, groups)` already exists.

### Proposed change

Use one lazy, call-local `Option<u8>` shared by the stage and discard activation loops. Initialize only when an eligible cost-bearing ability requires it.

Illustrative structure, not an applied patch:

```rust
let mut cached_groups: Option<u8> = None;

let groups = *cached_groups.get_or_insert_with(|| {
    game_state.distinct_stage_groups(&active_player.id)
});
let effective = game_state.effective_activation_cost_for(cost, groups);
```

Both loops read the same immutable `GameState`, so no invalidation machinery is needed inside this call.

### Preserve

- Existing multi-name group evaluation.
- Mandatory versus optional payment rules.
- Execution-time cost recalculation elsewhere.
- Per-call cache lifetime only: no persistent `GameState` cache.
- No new group scan on calls without eligible cost-bearing abilities.

### Tests to extend

`engine/tests/test_modules/effects/cost_mod/umi_q228_test.rs` already covers multi-name groups, affordability, displayed cost, and payment. Extend it for multiple eligible abilities, both stage and discard activation, P2 active, and fresh generation after stage mutation.

## 7. Queue allocation removal

### 7.1 Other-player discovery

Evidence:

- `engine/src/core/game_state/abilities.rs:1529`: `process_pending_auto_abilities`.
- `engine/src/core/game_state/abilities.rs:1546`: uses `pending_entries()` just to find another player's entry.
- `engine/src/ability_queue.rs:450`: `pending_entries()` collects a vector of references.
- `engine/src/ability_queue.rs:213`: a borrowed `iter()` API already exists.

Use the existing iterator and clone only the found player ID:

```rust
let non_active_id = self.ability_queue.iter()
    .find(|e| !e.completed && e.player_id != active_player_id)
    .map(|e| e.player_id.clone())
    .unwrap_or_default();
```

No queue API addition is required. The ID must be owned before calling code that mutably borrows the game state.

Preserve first-match order, the exact unfinished-entry predicate, active-player processing, pending-choice early returns, the empty-ID guard, and final completed-entry cleanup.

### 7.2 Zero/one/many available-entry selection

`engine/src/core/game_state/abilities.rs:1336` currently collects all available absolute indices into a vector.

Proposed structure:

1. Iterate matching indices within the existing `0..pre_len` boundary.
2. Read the first two matches.
3. Zero matches: stop.
4. One match: retain its absolute index and end the iterator borrow before promotion and execution.
5. Multiple matches: build the required owned choice options directly from first, second, and remaining matches.

The pending choice must still own its options. Remove only the intermediate index vector, not the options vector.

If diagnostic parity matters, collect the complete index list only under the existing `ABILITY_DEBUG` gate.

### Preserve exactly

- Absolute queue indices, not positions within a filtered list.
- Original option and player resolution order.
- Card-name lookup and fallback, full ability text, instance IDs, and all descriptions.
- `pause_for_auto_ability_choice`, including its `snapshot_requested` side effect.
- Consumption of `depth_first_cutoff` and the original `pre_len` boundary.
- Dynamic draining of newly appended entries without premature queue compaction.
- Reprocessing and drain guards, movement markers, and batch reruns.
- No queue reference, iterator, or stale cached index across resolver mutation or choice resumption.

Comments describing recursive behavior are stale in places: the inspected `process_player_abilities_depth` uses an outer batch-rerun loop. Do not redesign recursion based on those comments.

### Tests to extend

- `engine/src/ability_queue.rs`: `start_next_skips_completed`, `start_next_refuses_when_busy`, `auto_ability_choice_returns_to_idle`.
- `engine/tests/test_modules/integration/victory_road_test.rs`: `test_each_time_drains_between_live_starts_no_mix`, `test_one_live_start_each_time_drains_no_choice`, `test_three_live_starts_each_order_possible`.
- `engine/tests/test_modules/integration/pvp_room_test.rs`: `both_players_multiple_live_start_abilities_get_correct_choice_routing` is relevant source coverage, but `pvp_room_test` is excluded by the inspected build-script module generator. Do not assume this test runs in the standard suite; use an active test location for required regression coverage or address test registration separately.

Add explicit zero/one/many, completed-entry, mixed-player, exact-option-order, absolute-index, nested-trigger, and pause/resume assertions.

## 8. Action generation: borrowed area descriptors

### Evidence and current ordering

`engine/src/game/game_setup.rs:1441` contains `generate_main_phase_actions`. It emits pass, hand-member plays, stage abilities, then discard abilities. Single-area plays use left/center/right ordering, followed by double-baton actions.

At `:1527–1577`, every candidate member builds a three-element `Vec<AreaInfo>` with owned area strings and replacement-member names. This intermediate work happens even when no action is emitted or profiling omits output metadata.

### Proposed private representation

Keep public owned `Action`, `ActionParameters`, `AreaInfo`, and `DoubleBatonPair` unchanged. Introduce a private three-element array of borrowed descriptors:

```rust
#[derive(Clone, Copy)]
struct AreaCandidate<'a> {
    area: &'static str,
    available: bool,
    cost: u8,
    is_baton_touch: bool,
    existing_member_name: Option<&'a str>,
}
```

Populate `[AreaCandidate<'_>; 3]` using the current eligibility and cost logic. Materialize owned `Vec<AreaInfo>` only at each non-profiling output boundary, including both single and double actions.

This removes temporary ownership. Returned action fields still require their current owned representation. Use imports compatible with existing `no_std` configurations; no new library is required.

### Preserve exactly

- Unavailable-area defaults: printed card cost, no baton flag, no replacement name.
- Missing database entries for occupied stage IDs; do not treat these as empty slots.
- Incoming-card-dependent baton restrictions; do not hoist them across the hand loop.
- Deployed-this-turn identity rules, discounts, modifiers, saturation and conversions.
- All action ordering, descriptions, executable indices, and `None`/`Some` behavior.
- Double-baton-only availability even if no single play is affordable.
- Double-baton computation under profiling: its metadata may be omitted, but it still produces executable actions.
- Existing bounded action capacity reservation.

Defer the separate double-baton temporary pair/string redesign until its pair and placement ordering have independent coverage.

### Tests to extend

- `engine/tests/test_modules/rules/baton/baton_touch_test.rs`: `baton_touch_does_not_lock_all_full_lanes`; assert all three metadata records and unavailable defaults.
- `engine/tests/test_modules/effects/cost_mod/bp7_tang_keke_cost_test.rs`: discounted and unmodified costs, empty-area costs, action `final_cost`.
- `engine/tests/test_modules/rules/conditions/restriction_mechanics_test.rs`: `sumire_bpb4004_double_baton_removes_both_and_clamps_cost`; extend pair ordering and double-only availability.

Normal and profiling builds intentionally differ in UI metadata. Tests must check the appropriate representation in each configuration while preserving executable fields in both.

## 9. Performance snapshot ownership

### Verified discarded work

In `engine/src/turn/phases.rs`, the phase prepares final contributions, breakdown, and triggered abilities, then calls a builder that creates another set and immediately replaces them:

| Field | Phase preparation | Builder duplicate | Replacement |
|---|---|---|---|
| Member contributions | `phases.rs:548` | `live.rs:2788` | `phases.rs:646` |
| Breakdown vectors | `phases.rs:549–556` | `live.rs:2795–2802` | `phases.rs:647` |
| Triggered abilities | `phases.rs:557`, `:600–618` | `live.rs:2803–2839` | `phases.rs:648` |

Empty vector construction itself does not allocate. The removable work is populated cloning and duplicate triggered-ability construction/traversal.

### Proposed change

1. Add an internal snapshot constructor accepting final owned enrichment fields.
2. Preserve public `build_snapshot` at `engine/src/turn/live.rs:2755` as a compatibility wrapper with its existing default behavior.
3. Have the phase call the internal constructor with its enriched contribution, breakdown, and trigger data.
4. Remove immediate field replacement.
5. After the last required earlier use, use `core::mem::take` to transfer contribution and breakdown vectors from `perf_data`, rather than cloning them first.
6. Preserve application-derived score lines followed by constant score sources in their current order.

A possible internal payload:

```rust
pub(crate) struct SnapshotDetails {
    pub member_contributions: Vec<MemberContribution>,
    pub breakdown: Breakdown,
    pub triggered_abilities: Vec<TriggeredAbility>,
}
```

This is a design sketch, not a signature already added to the engine.

### Semantic traps

- Application enrichment deduplicates by source card plus ability text and ignores empty trigger text.
- The public default builder deduplicates contribution bonuses by ability text alone.
- Do not build the default trigger list and then enrich it as a shortcut: attribution, duplicates, and ordering can change.
- Preserve exactly one draw-effect trigger entry.
- Preserve performer-specific application partitioning.
- Preserve the separately captured `note_icons` value; substituting a later value from `perf_data` would be an additional behavioral change.

### Data that must remain

- Pre-trigger heart and need-heart modifier snapshots.
- Allocation payloads used by verdict and surplus logic.
- Yell cards and captured live-card IDs used after zone mutation.
- Performance-time modifier restoration.
- The snapshot itself, including under headless/profiling execution: winner determination reads it.

A broader consuming redesign of yell/success data is separate work. Do not remove required temporal snapshots as if they were UI-only copies.

### Tests to extend

- `engine/tests/test_modules/rules/phases/performance_pipeline_test.rs`: contributions, yell totals, allocations, base scores; strengthen the triggered-ability assertion that currently permits an empty list.
- `engine/tests/test_modules/characterization/performance_snapshot_audit_test.rs`: `audit_allocations_match_filled`.
- `engine/tests/test_modules/rules/conditions/condition_evaluation_test.rs`: `both_players_get_own_live_start_effects`; add exact snapshot attribution assertions.
- `engine/tests/test_modules/rules/phases/attacker_order_performance_test.rs`: P2-first performer and empty opposing lives.
- `engine/tests/test_modules/rules/phases/cheer_pipeline_test.rs`: score icons and choice-gated victory finalization.

Compare snapshots immediately after performance and after victory finalization. Cover duplicate text from different sources, empty text, heart/blade bonuses, transforms, score-line order, and draw-trigger multiplicity.

## 10. Activation identity and text lookup

### Current flow

`engine/src/turn/actions.rs:206–390` validates and selects an activation, potentially moves a hand card, formats an identity key, enqueues through `trigger_auto_ability`, processes the queue, and logs activation.

At `engine/src/core/game_state/abilities.rs:969`, the text helper scans printed abilities and formats candidate keys again. A numeric helper exists around `:1053`, but it is not semantically equivalent.

| Concern | Text helper | Numeric helper |
|---|---|---|
| Printed identity | First matching trigger and full text | Exact printed index |
| Trigger validation | Present | Not equivalent / absent in inspected helper |
| Gained abilities | Supported | Not supported |
| Missing explicit instance | Searches player zones | Does not enqueue |
| Printed enqueue debug note | Added | Not added |

### Safe first step

Strip the exact supplied card-number prefix and underscore once, then compare the borrowed suffix with each trigger-matching ability's full text. Preserve first printed-match precedence and gained fallback parsing.

This removes per-candidate formatting, not the caller's key allocation, scan, required queue identity strings, or logs.

### Why numeric dispatch is deferred

- Duplicate printed full text can cause the text helper to enqueue a different index from the one validated by selection.
- Indices participate in use limits, loop guards, and completed-ability tracking.
- Gained fallback currently ignores the requested index and differs from printed selection in checks.
- Gained identities use `10_000 + gained_index`; the recipient's card instance must remain the queue source.
- A printed matching-trigger text equal to `gained_N` takes precedence over gained-key parsing in the current control flow. Actual occurrence in card data was not established.
- Printed and gained branches have different logging behavior; `push_debug_note` is not merely a disabled debug-level log.

For strict compatibility, keep the legacy route for gained and ambiguous duplicate-text cases. Treat any correction to existing selection behavior as a separate correctness change with explicit tests, not a silent optimization.

### Tests to extend

- `engine/tests/test_modules/effects/conditional/bp7_kanata_choice_test.rs`: `kanata_cost_mills_top_three`, `kanata_use_limit_twice_per_turn`.
- `engine/tests/test_modules/effects/compound/pl_sp_pb2_005_test.rs`: `hazuki_activates_kidou_copied_from_under`.

Add characterization for duplicate printed text and distinct indices, gained requested indices, recipient identity, encoded queue indices, wrong-trigger entries, printed/gained key collisions, and log parity. Verify card identity against the master database before using card-specific fixtures.

## 11. Secondary candidate: live-zone movements

This was identified during the capture analysis but is not part of the recommended first implementation batch.

Evidence in `engine/src/turn/live.rs`:

- `:1275–1288`: copies live-zone cards before a choice helper performs basic eligibility checks at `:1213`.
- `:1319–1344`: clones live zones, moves cards, then scans waitrooms to rediscover which cards were discarded.
- `:1199`: repeatedly removes the first remaining card.

Possible future changes:

- Perform cheap eligibility checks before copying choice data.
- Build options from borrowed cards before mutating the queue.
- Record discarded IDs during movement rather than rediscovering them afterward.
- Use an order-preserving drain where equivalent.

Preserve first-card selection, discard order, P1-before-P2 events, choice early returns, and trigger dispatch after both players' normal movements. Verify unique card-instance assumptions before equating presence in waitroom afterward with a movement that just occurred.

## 12. Validation and benchmark requirements

### Correctness before timing

For each independently scoped change:

1. Establish the current baseline and record revision plus dirty-file state.
2. Extend the relevant active tests before relying on a full-suite result.
3. Compare complete ordered action outputs and executable fields where applicable.
4. Compare queue options, absolute indices, event order, pause/resume state, and use-limit identity.
5. Compare snapshots both before and after victory finalization.
6. Run targeted tests, then the documented engine suite from `engine`:

```powershell
cargo test --test run_all
```

No engine tests were run during the read-only implementation-planning phase. Test names listed in this document are proposed future validation targets, not claims that the candidate changes passed them.

### Deterministic workload controls

The existing `profile_target` is useful for hotspot discovery but insufficient by itself for exact before/after replay:

- It selects the first deck from unsorted directory enumeration (`engine/src/game/deck_parser.rs:158`).
- Its action policy uses `rand::thread_rng()` (`engine/src/bin/profile_target.rs:44`).
- It ignores action execution errors and has stuck/iteration cutoffs.
- Different runs therefore have different action counts and outcomes.

An external harness can use existing engine APIs without changing engine gameplay APIs:

1. Pin source, cards, an explicit deck, compiler, dependency versions, and feature set.
2. Load databases and build deck templates outside the timed region.
3. Run games serially with a fixed engine seed and a separate local seeded action policy.
4. Record semantic actions, not only indices into generated action lists.
5. Replay against the candidate build, checking legality, results, phase/action counts, ordered state, and a canonical state digest.
6. For repeated state snapshots, restore game state, engine RNG checkpoint, and policy RNG position.
7. Reject divergent/error workloads from speedup comparisons rather than treating early termination as faster execution.

Relevant existing controls:

- `engine/src/rng.rs:109`: engine seed.
- `engine/src/rng.rs:114`: checkpoint.
- `engine/src/rng.rs:118`: restore.
- `engine/src/game/deck_builder.rs:17`: deck shuffling uses the engine RNG.

RNG is process-global, not stored inside `GameState`. Mulligans, refreshes, and shuffle effects consume it after dealing. Parallel games can interleave consumption. Hash-map iteration is another ordering caveat; engine seeding alone does not make arbitrary serialized maps deterministic.

First-attacker selection is driven by generated RPS and first/second choices, not a separate setup coin flip. Mirroring the capture workload must preserve its exact execution entry point: substituting `execute_action` or `execute_and_settle` changes orchestration and parameter forwarding.

### Timing controls

- Measure one optimization at a time with identical workloads and builds.
- Alternate baseline and candidate repetitions rather than trusting one timing.
- Separate setup from the measured operation for microbenchmarks.
- The existing `check_timing` benchmark includes `build_game(0)` inside its measured iteration (`engine/benches/performance.rs:166`); it is not an isolated `check_timing` measurement.
- Keep verification/logging outside timed regions where possible.
- Compare both corrected flamegraph scope times and uninstrumented throughput under matched features.
- Do not compare profiling-on versus profiling-off as though timers were the only difference.
- Allocation tracking is available through `alloc_tracker`, but it adds overhead; use it for allocation counts separately from clean timing measurements.
- A real CPU sampling profile would help resolve broad wrappers into untimed descendants before deeper optimization.

## 13. Implemented external record/replay harness

The external harness is now implemented without engine source changes:

- `tools/flamegraph_replay.py`: external build launcher (Python 3.11+).
- `tools/flamegraph_replay.rs`: seeded record/replay executable.
- `tools/test_flamegraph_replay.py`: launcher regression tests.

### Build

From the repository root on Windows, use native Python, not the MSYS2 Python that may appear first on PATH:

```powershell
py -3 tools/flamegraph_replay.py build --build-dir "C:\Users\trios\AppData\Local\Temp\kilo\replay-build"
```

On other supported hosts use a native Python 3.11+ interpreter. The launcher rejects MSYS/Cygwin Python to avoid mixing POSIX path semantics with native Windows Cargo.

The build directory must be outside the repository and cannot be its ancestor. The launcher creates an owned external manifest referring to unchanged engine source, disables the engine package build script and automatic targets, preserves declared dependency/features/profile configuration, and copies the engine lockfile on first use. Cargo runs with `--offline --locked` and a separate external target directory. Missing cached dependencies or incompatible lockfiles fail rather than silently updating dependencies.

It writes `provenance.json` containing source and asset digests, compiler, features, lockfile, build command and executable hash. Source/assets/configuration are checked again after building; a detected concurrent input change invalidates the build provenance. The source-writing build script and its freshness checks are bypassed. Retain the provenance alongside any comparison; it is not embedded or automatically authenticated by the replay trace.

The command prints the executable path and provenance path. Add `--profiling` with a separate external build directory to enable existing engine timer scopes; this option's manifest mapping is unit-tested, but the new harness's end-to-end smoke test used profiling off. No engine optimization is applied by building this tool.

### Record and replay

Run the generated executable from the `engine` directory, using the actual executable path printed by the build. Example:

```powershell
& "C:\Users\trios\AppData\Local\Temp\kilo\replay-build\target\release\flamegraph_replay.exe" record "C:\Users\trios\AppData\Local\Temp\kilo\replay-trace.jsonl" "../web_ui/decks/muse_cup.txt" 2 42 7
& "C:\Users\trios\AppData\Local\Temp\kilo\replay-build\target\release\flamegraph_replay.exe" replay "C:\Users\trios\AppData\Local\Temp\kilo\replay-trace.jsonl" "../web_ui/decks/muse_cup.txt"
```

Record arguments after the paths are game count, engine seed, and policy seed. Games run serially; per-game seeds derive from the supplied seeds. The trace must not already exist, and its parent directory must exist. A failed recording can leave a partial trace, which must not be treated as a successful benchmark.

Replay checks exact deck text, trace version, projection and profiling mode. It validates ordered generated action payloads and finds the recorded action by its serialized payload rather than trusting its old array index. Actions include display metadata, so this is intentionally stricter than executable-parameter-only matching. Execution mirrors `profile_target`'s entry point, including its lack of explicit ability-index forwarding; it does not silently substitute the broader `execute_action` wrapper. Execution errors and stuck/iteration cutoffs fail the run.

A first mismatch reports a game/step/field path. Object field ordering is ignored, while array ordering is retained. Truncated and trailing records are rejected. A profiling-enabled executable prints folded stacks to stdout only after successful completion; diagnostics go to stderr.

### Precisely what is checked

The state projection covers serialized players and ordered zones, phase/turn/result, engine RNG checkpoint, serialized queue, performance snapshots, resolution zone, sorted turn-limit usage, selected movement histories, depth-first cutoff and RPS choices. It does **not** establish complete engine equivalence: modifiers, internal resolver state, serde-skipped fields, logs and other unlisted state are excluded. Build provenance records card assets, but replay itself only binds deck text and the listed header fields; compare provenance separately before drawing cross-build conclusions.

Reported engine-call time excludes setup, trace serialization/I/O and comparisons. It includes per-call clock overhead and is affected by tracing's cache/allocation perturbation. It is not clean game throughput, an allocation benchmark, or proof of an optimization speedup. Trace files can be large because they contain full per-step projections and action lists; begin with small game counts.

### Validation performed

- Native external launcher build succeeded using the repository lockfile, and wrote provenance after unchanged-input checks.
- Separate processes recorded and replayed two `muse_cup` games with seeds 42 and 7: 477 actions matched, with 446 and 254 steps respectively.
- A mismatched `aqours_cup` deck was rejected before gameplay.
- Rust harness unit tests: 3 passed.
- Python launcher tests: 20 passed, 1 skipped because native symlinks were unavailable.
- Rust typecheck and formatting passed.
- Strict Clippy was attempted but failed on pre-existing engine-library lint findings; engine files were not changed to resolve them. A clean strict-lint result for the combined build is not claimed.
- The full engine suite was not run for this tooling-only change.

### Post-audit hardening (later the same day)

An integrity audit and hardening pass fixed these issues in `tools/flamegraph_replay.rs`:

- Bounded trace reads (64 MiB per record), oversized/malformed/truncated record rejection, per-game `1..=2000` step validation, 1 MiB deck read limit with post-parse deck re-read verification.
- Recordings now write to a `.partial` staging file, publish only after all games succeed via no-clobber hard link, and remove staging on failure; existing destinations are never replaced.
- Trace header v2 with `deny_unknown_fields`, per-field error messages, per-game seed overflow rejection, and a completion record verifying games/actions after the final game.
- The header embeds the build identity emitted by the launcher (`replay-build-identity.json`, also passed via `RABUKA_REPLAY_BUILD_IDENTITY` at compile time). Replay rejects traces whose `assets_sha256`, `cargo_lock_sha256`, or feature set differ from the executing build; source/harness digests remain provenance-only so baseline-vs-candidate comparison works.
- Action matching remains strict whole-payload, ordered comparison (see limitations).

Launcher hardening in `tools/flamegraph_replay.py`:

- Emits the build identity file/environment variable binding the executable to asset/lock/feature digests.
- Rejects MinGW/MSYS Python via platform detection; preserves rendered compiler diagnostics; rejects redirected/hard-linked build outputs; re-validates the identity file after build.

Generator hardening in `engine/examples/gen_flamegraph.rs`:

- Enforces exact nonzero totals in both render paths, checked aggregation overflow, and rejects ambiguous numeric label suffixes that Inferno would misread as differential input.

Validation after hardening:

- Rust harness: 7 tests passed, typecheck and formatting clean; harness-only Clippy ran clean (the earlier `-D warnings` failure came from pre-existing engine-library lints, which remain unchanged).
- Python launcher suite: 26 tests passed, 1 skipped (symlink privileges).
- Native launcher build (non-profiling) and a second profiling build both succeeded with identity + provenance written.
- End-to-end `tools/smoke_flamegraph_replay.py`: two-game cross-process record/replay passed (477 actions), plus 7 negative cases (deck change, identity mismatch, state divergence, removed selected action, truncation, trailing records, trace already exists).
- The profiling smoke emitted real folded stacks, which the hardened generator rendered into caller and self-time SVGs (21.8 ms instrumented coverage for 2 games).
- Legacy v1 traces are rejected; re-record them with the v2 executable.

Remaining known limitations (documented, not fixed):

- Action payloads are compared with strict ordering and full display metadata; a display-only text change or action reordering will reject replay. This is intentional for optimization verification but is not order-insensitive semantic matching.
- Replay binds deck text and identity digests, not a full card-database snapshot hash, inside the trace itself; compare `replay-build-identity.json`/`provenance.json` across builds before interpreting results.
- The state projection excludes modifiers, resolver internals, serde-skipped fields, and logs; passing replay is strong evidence of identical behavior, not complete engine equivalence.
- Engine-call time includes per-call clock overhead and tracing perturbation; it is not clean throughput.


## 14. Engine code improvement list (for review — none applied)

Ranked by measured hotspot share (section 4), then by change risk. Percentages are shares of corrected instrumented coverage from captures C/D; they justify investigation, not guaranteed savings. All items preserve gameplay semantics unless marked otherwise.

| # | Improvement | Measured bucket (C share) | Complexity | Risk | Primary files |
|---|---|---:|---|---|---|
| 1 | Cache stage-group count per generation call | generate_main_phase_actions 15.5% | Low | Low | game_setup.rs |
| 2 | Queue: replace `pending_entries()` collect with iterator find | shared (use_ability 11.4%, resume 16.3%) | Very low | Low | abilities.rs |
| 3 | Queue: zero/one/many selection without index Vec | shared | Low–medium | Low | abilities.rs |
| 4 | Borrowed stack area descriptors in hand-play generation | generate_main_phase_actions 15.5% | Low–medium | Low–medium | game_setup.rs |
| 5 | Snapshot built once with final enrichment | performance phase 6.4% | Medium | Medium | phases.rs, live.rs |
| 6 | Strip text-key prefix once in activation lookup | use_ability 11.4% | Low | Low (exact-match preserved) | abilities.rs |
| 7 | Defer: numeric activation dispatch | use_ability 11.4% | Medium | Higher (duplicate text/gained identity) | abilities.rs, actions.rs |
| 8 | Defer: live-zone movement collection redesign | live_victory 6.9% | Medium–high | Medium | live.rs |

Details, preservation requirements, edge cases, and per-item test targets are in sections 6–11. Recommended first batch: items 1–3, measured independently with the section 13 harness (record a baseline trace, replay it against the candidate build, then compare engine-call time and fresh captures).

## Final recommendation

Begin with **call-local stage-group caching and the two queue-collection removals**, independently tested and measured. Then address borrowed area intermediates and snapshot ownership. Remove repeated text formatting before considering numeric dispatch. Keep gained-ability and duplicate-text correctness changes separate from performance work.

The verified outcome so far is improved flamegraph accounting, real captures, stable investigation priorities, and a concrete implementation plan—not an engine speedup.
