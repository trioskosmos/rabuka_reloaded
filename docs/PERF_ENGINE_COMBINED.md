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

**The canonical build is the PGO build** (landed 2026-10-09,
+47.7% jobs=1 / +47.0% jobs=256 — the largest single lever
measured; the profile is committed at `engine/pgo/merged.profdata`):

    RUSTFLAGS="-Cprofile-use=pgo/merged.profdata" cargo build --release --bin sim_bench

LLVM matches profiles by function hash, so an engine edit
degrades gracefully (edited functions lose their profile, the
rest keep it), but re-run the pipeline after significant
engine changes — the retraining recipe is in `engine/Cargo.toml`
under `[profile.release]`. Training corpus: all 13 decks,
26,000 random-policy games, 4.29M actions, with
`SIM_BENCH_WARMUP_SECS=0` (the multi-deck warmup seeds hit a
pathological (deck, seed) pair — see §4.6).

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
work: `--jobs 256` buys 6.8× (the throughput peak; 128 is 6.4×).

**2026-10-09 (this session — five landed levers, cumulative
+80% jobs=1 / +74% jobs=256, every one gated on
`total_actions` byte-identical and 4364/4364 tests):**

1. **`fast_alloc` promoted to a default feature (+11.5%
   jobs=1, +10.6% jobs=256).** Re-measured on the current
   tree before believing the old +13%: interleaved in-process
   A/B, pool 147,780 vs base 132,556 mean aps (jobs=1),
   817,449 vs 739,156 (jobs=256). The full suite passes with
   the pool active (4364/4364, 4.29s run), which was the
   gate for default-on. `no_std` ports are unaffected — they
   declare their own allocators and never enable the feature.
2. **`lto = "thin"` + `codegen-units = 1` landed (+5.6%
   jobs=1, +4.2% jobs=256).** The catalogue's blocker was
   that the full suite "could not be run to completion under
   LTO" — it did not reproduce: 4364/4364 in 4.30s under the
   LTO build (the 8m37s is the ~27 bins plus the harness
   target compiling with LTO, not a failure). No high-`--jobs`
   regression (unlike `lto = "fat"`, which was measured and
   rejected).
3. **`get_card_names` → `Vec<&str>` (−71,301 allocs per
   200-game bench, −2.9%; +1.7% jobs=1).** The 12 call sites
   in the condition layer were cloning every name part of every
   candidate card. `max_distinct_names`/`_greedy` became
   generic over `AsRef<str>`; the one site that must own the
   strings (`resolve_name_fragments`, feeding `CardFilter`,
   whose field type is shared with the effect's printed
   `card_names`) maps `String::from` at its boundary — a cold
   path.
4. **Mechanical batch (+1.1% jobs=1, −14,940 allocs):**
   `ResumeSnapshot::capture`'s nine `current_entry()` probes
   collapsed to one (every field it reads is `Copy` except
   `player_id`); `entry_player_or_p1` returns `&str` borrowed
   from the snapshot instead of cloning the `String` (all four
   consumers take `&str`); `take_cards_from_standard_zone`
   borrows the zone slice via `zone_cards` instead of cloning
   it through `zone_card_ids` (NLL releases the borrow before
   the first `&mut player` use); `execute_modify_required_hearts_standard`'s
   two per-call `Vec`s became `SmallVec`s.
5. **PGO, corpus-trained (+47.7% jobs=1, +47.0% jobs=256).**
   Trained on the full 13-deck corpus (26,000 games, 4.29M
   actions) with `SIM_BENCH_WARMUP_SECS=0`; the profile is
   committed at `engine/pgo/merged.profdata` and the build
   command is in §1. A single-deck profile measured +43.1% —
   the corpus profile is both faster and more general.
6. **Activation-overlay condition cache + typed hasher
   writers (+7.4% jobs=1, +129% jobs=256, interleaved
   A/B; −81% alloc calls per 200-game bench).** Two
   outcome-neutral changes landed together and gated
   together (total_actions byte-identical in all 20 A/B
   runs, 4364/4364 tests, allocator self-time 17.1% →
   5.1%):
   - `ability_activation_gate_holds` deep-cloned the
     activation condition into a fresh `Arc` on every
     action-generation pass whenever the effect carried a
     position the printed condition did not — a `Condition`
     is a recursive tree with a `Box<ConditionCommon>` per
     node, so that clone was 10-30 allocations per
     activatable ability per pass, and the pass runs ~52×
     per game. The merge is a pure function of the
     ability's own data, so it is now cached on `Ability`
     in a `OnceLock<Option<Arc<Condition>>>`
     (`activation_overlay`, serde-skipped, the same
     pattern as `trigger_mask`). This is the
     alloc-removal lever: with the pool's single Mutex as
     the jobs=256 bottleneck, cutting alloc calls from
     12.42M to 2.33M per 200 games is what compounds into
     the +129% at high job counts (the jobs=1 gain is the
     CPU-bound +7.4%).
    - `generate_state_hash`'s `SimpleHasher` hashed every
      `i16`/`usize`/`bool` element through the default
      `write(&bytes)` call plus a per-byte loop; the typed
      `Hasher` writers are now overridden so each element is
      a single multiply-add. `record_action_boundary` self
      time dropped 0.41 → 0.29 µs/call (−29%). Only hash
      equality is ever consumed (`game_state_history`
      containment and the repetition count), so the changed
      values are behaviour-preserving.
    - Correction: the pool (`pool_alloc.rs`) is
      **thread-local** (`thread_local! { static FREE:
      Cell<*const FreeLists> }`) and has **no Mutex** —
      the "pool's single Mutex" phrasing was wrong. The
      contended resource at high job counts is the
      *system* allocator's heap lock, reached on a
      free-list miss or a `PER_CLASS` overflow; that is
      what an alloc-call reduction relieves.

7. **Choice-offer signature gating + value-keyed
   condition cache (−25% alloc calls on the bench deck,
   +8.9% jobs=1 aps).** Two outcome-neutral changes,
   gated on `total_actions` byte-identical (30231 on
   `5CP3Z idou`, seed 1, 200 games) and 4364/4364
   tests:
   - `store_pending_choice` materialized the full
     `choice_offered_labels` Vec — a `String` per
     candidate plus a resolved card name per index
     (`resolve_index_any_player`) — on EVERY choice
     store, to build a dedup signature whose only
     consumer (`push_choice_offered`) is itself gated
     on `logging_enabled()`, and whose only other
     reader (the debug-choice log) is gated on
     `log_enabled!(Debug)`. sim_bench runs headless
     (both flags false), so the whole materialization
     was waste: ~30 of the ~84 allocations of a
     choice-storing ability. The signature block now
     runs only when `logging_enabled() ||
     log_enabled!(Debug)`. Measured on the bench deck:
     1,053,120 → 788,151 alloc calls per 200 games
     (−25.2%), 150,107 → 163,556 mean aps at jobs=1
     (+8.9%), `total_actions=30231` byte-identical.
   - The per-queue-entry condition cache was keyed by
     `format!("{:?}", condition)` — two full
     condition-tree Debug formats per cached-condition
     evaluation, each a ~650 B allocation that bypasses
     the small-allocation pool entirely (the pool only
     serves ≤255 B). `Condition` derives `PartialEq`
     and its derived `Debug` prints every field, so
     string-key equality and value equality are the
     same relation; the cache is now keyed on the
     `Condition` value itself
     (`SmallVec<[(Condition, bool); 2]>`), which
     removes both formats (lookup is a `PartialEq`
     compare, store is one deep clone into the inline
     SmallVec).
   - Audit method note: the single-frame allocation
     tracer misattributes under LTO/inlining — it
     reported `Phase::fmt` and `clear_movement_tracking`
     as top sites, but arming `check_timing` directly
     measured **0 allocations**, and `clear_movement_tracking`
     only clears collections. Those rows are merged-symbol
     artifacts. The tracer now keeps the innermost three
     engine frames (`alloc_counter.rs`), and
     `check_timing` gained a trace site. The remaining
     allocations in the hottest function
     (`process_current_ability`, ~20/call after this
     change) are genuine: the `EffectOwner` deep clones
     (`executing = Some(effect.clone())`, 8 write sites,
     plus `capture_choice_owner`'s snapshot — an
     `Arc<AbilityEffect>` EffectOwner would remove only
     the snapshot clone, not the write-site clones), the
     `Choice` builder's display strings (game state, not
     display-only), and `normalized_steps`' owned
     compound-step lists.

Also measured and **rejected** this session: `target-cpu=native`
(+0.8% over pool+LTO — noise; the workload is branch-heavy
pointer-chasing, not vectorizable).

**2026-10-09 (the pool allocator, +13% — the single largest result of
the session):** the allocator was 11.5% of elapsed time (7.3M
allocations, 850 MB per 500-game bench) — the largest removable share
after the enumeration itself. `pool_alloc.rs` was already in the tree
but inactive, and measured 2.8% *slower* than System. The bug: the
previous AI's `FreeLists` was `#[derive(Clone, Copy)]` = 310 bytes
(`heads: [*mut u8; 31]` + `counts: [u16; 31]`), and `Cell::get()`/
`Cell::set()` copied the **entire struct on every alloc and every
dealloc** — ~1.5 KB per alloc+dealloc pair × 7.3M pairs ≈ 11 GB of
copying, which dwarfed whatever the pool saved. The fix is an 8-byte
pointer cell (`Cell<*const FreeLists>`) mutating through a raw
pointer, with the struct allocated through `System.alloc` at first
use so `Box::new` cannot re-enter and stack-overflow on the first
allocation. Verified: jobs=1 143,231 / jobs=256 879,066 aps vs 124,386 /
844,883 pool-only (+13%), `total_actions=328173` byte-identical,
4354/4354 tests pass with the pool active, allocator self-time 11.5%
→ 3.0%. Enable with `--features fast_alloc`.

**2026-10-09 (pool floor 32B → 16B, +4%):** the allocation report's
single largest class is 8–15B (2.2M of 7.3M allocations, 30%),
which fell through to System because the pool floor was 32. Lowering
`MIN_CLASS` to 16 captures it: a 16-byte block is 8B header + 8B
payload, enough for any align-≤8 request up to 15B (`is_pooled`
already requires `layout.align() <= POOL_ALIGN` = 8), and the free-list
link still fits in the first word. Verified: `total_actions` identical,
4354/4354 tests.

**2026-10-09 (loop-invariant hoist, noise-neutral):** in
`generate_main_phase_actions` the per-hand-card loop recomputed
`active_player.hand.cards.len()` and
`active_player.energy_zone.active_count()` for every hand card
(144,781 calls per benchmark) although both depend only on
`active_player`. Now computed once before the loop. Verified
deterministic; the previous AI's "per-hand-card by design" claim was
wrong about these two.

**2026-10-09 (action-buffer pre-sizing, noise-neutral):**
`generate_main_phase_actions` (39,239 calls/bench) and
`generate_live_card_set_actions` (407,533 calls/bench, the most-called
function in the profile) started their `Vec` with `vec![<first>]`
(capacity 1) and pushed ~30 / ~5–10 actions per call, reallocating
~5 times per call and memcpy-ing the growing buffer. Both now use
`Vec::with_capacity(48)`. Verified deterministic; noise-neutral in aps.

**2026-10-09 (`--jobs` plateau corrected to 256, not 128):** the
high-end sweep (128/256/512/1024/2048) shows the throughput peak is at
256 (844,883 aps), not 128 (795,987); it degrades beyond that (512
−4%, 1024 −16%, 2048 −11%). The old "flat from 64 to 256" line was
measured before the pool landed and is stale. `total_actions=328173`
is identical at every setting, so determinism holds across the whole
range.

**2026-10-10 (display-only source attribution gated + two
`SmallVec` conversions; `total_actions` byte-identical,
outcomes exact):** four outcome-neutral allocation
reductions, each gated on the exact outcome match
(P1 189 / P2 252 / draw 1059 over 500×3, seed 1;
`total_actions=219948`):
- The success-zone and constant-cost `*_sources`
  `Vec<BonusSource>` pushes
  (`evaluate_success_zone_constant_modifiers`,
  `recalculate_constant_cost_modifiers_with_ids`) ran
  unconditionally — growing a `Vec` and, on the heart
  arm, allocating a `color_str.to_string()` per color —
  on every `recalculate_constants` pass, even though
  those fields are read **only** by `game/display.rs`
  (the same contract the stage-constant path's
  `DisplaySources` already encodes). They are now gated
  on `action_display_enabled()`.
- `advance_phase::active`'s `to_activate` was a
  `Vec<i16>` allocated every Active phase (~75,599 per
  bench); the stage is exactly 3 slots, so it is now a
  `SmallVec<[i16;3]>` that never spills.
- The success-zone constant pass collected
  `Vec<(i16, usize, AbilityEffect)>`, deep-cloning every
  `AbilityEffect` (whose `compound`/`effect_steps`
  `Box`/`Vec` fields allocate when present) and
  allocating a `Vec` buffer every pass. It now stores the
  resolved `Arc<Ability>` in a
  `SmallVec<[(i16, usize, Arc<Ability>);4]>` — a
  refcount bump replaces the clone, and the common ≤4-entry
  board stays on the stack. The effect is borrowed from the
  `Arc` in the application loop, which is disjoint from the
  `&mut self` that applies the bonus, so the borrow that
  forced the collect-up-front still holds.

The jobs=1 gain (+1.2%, 1598 vs 1579 mean over 3 runs)
is below this box's ±12% gps noise floor and is not a
reliable speed claim on its own; the gate is the exact
allocation reduction and the byte-identical `total_actions`.
The PGO profile was retrained on the full 13-deck corpus
after the edit. Two Windows PGO gotchas hit during
retraining and are now encoded in the recipe: with
`--jobs>1` every worker writes the *same* default
`.profraw` and clobbers the others, so set
`LLVM_PROFILE_FILE=<dir>\pgo-%p.profraw` (per-PID) when
training; and `cargo clean` before the `-Cprofile-use`
build, or cargo reuses dependency objects cached under the
identical RUSTFLAGS string and the thin-LTO link fails on
conflicting `ProfileSummary` module flags.

**2026-10-10 (allocation audit round 2 — two borrow
conversions; `total_actions` byte-identical):** ranked the
remaining allocation sites with the timer report's
**inclusive per-call-path allocation table** and its
**size-class breakdown** (the doc's own ranking signal),
then confirmed each candidate with a single-call
`RABUKA_TRACE_AT` trace (the innermost-3-frame tracer).
Baseline on the bench deck (warmup off): 1,247,252 alloc
calls / 200 games, `total_actions=29875`.

- **`execute_gain_resource`** (`ability/effects/resource.rs`)
  built four owned `String`s — `resource`, `target`,
  `card_type_filter`, `group_filter` — via `.to_string()`
  on values the accessors already return borrowed
  (`resource_any()`/`target_name()`/`group_name()` →
  `&str`; `card_type_any()` → `&CardType`, whose
  `as_card_str()` is `&'static str`), then re-borrowed
  them for the `GainResourceRequest` (whose fields are
  already `&'a str`). All four are now borrowed directly.
  `−2,589` alloc calls (−0.2%); `total_actions=29875`
  byte-identical. Small because the per-call-site estimate
  overpredicts (the catalogue's §19.3 correction again):
  `card_type_filter`/`group_filter` are `None` on the
  common path, so only `resource`+`target` (2/call) were
  actually removed.
- **`SelectionFilter.card_property`**
  (`ability/effects/move/handlers.rs`) was the one
  `Option<String>` among `Option<&'a str>` siblings; the
  construction did `e.card_property_any().map(|s|
  s.to_string())` and the only reader did
  `.as_deref()`. Now `Option<&'a str>`, borrowed straight
  from `card_property_any()`. Correct and outcome-neutral
  (`total_actions` byte-identical), but **not measurable on
  this deck** — `card_property` is `None` on the common
  path, so the alloc it removes only fires for effects that
  carry a card-property filter. Kept because it is the
  right shape (matches the sibling fields) and removes a
   real alloc on the decks that use it.

**2026-10-10 (allocation audit round 3 — pending-choice
`card_no`/`card_name` display-gating; `total_actions`
byte-identical):** the `generate_pending_choice_actions`
arms built `ActionParameters.card_no` (23 sites) and
`card_name` (1 site) unconditionally —
`Some("select".to_string())`, `Some(n.to_string())`,
`Some(real_card_no.clone())`, `Some(opt.card_name.clone())`,
etc. — even though `execute_action` and the bots route
every action by `card_id` alone and only the web UI and
debug output (which already falls back to `"-"`) read
these fields. The main-phase arms already gated them on
`cfg!(not(feature = "profiling")) && display`; the
pending-choice arms were the outliers. All now go through
a `display_str` helper that returns `None` (no allocation)
when `action_display_enabled()` is off — the same gate
`make_action`/`with_ja` apply to the description — and the
`real_card_no` scratch in the `SelectCard` arm is likewise
gated. `−79,461` alloc calls (−6.4%: 1,247,252 →
1,167,791); `total_actions=29875` byte-identical.

**2026-10-10 (allocation audit round 4 — profiler
call-path interning; `total_actions` byte-identical):**
the `Timer::drop` call-path key was a fresh
`Arc<[&'static str]>` on *every* drop — the single
largest allocator in a `--features profiling,
alloc_tracker` build (~42% of every counted
allocation, and the reason the inclusive per-call-path
table's own rows were dominated by the profiler). Call
paths are highly repetitive (the engine replays the
same ability sequences thousands of times), so the key
is now interned in a per-thread `PATH_CACHE:
HashMap<Box<[&'static str]>, TimerKey>`: a repeated
path reuses the cached `Arc` (a refcount bump) and a
fresh `Arc` is allocated only the first time a given
path is seen on a thread. The `Arc` keys still hash and
compare by value, so the four global maps aggregate
across threads exactly as before and `timer_report.txt`
is unchanged (same paths, counts, and times — verified
against the pre-change report). `−474,715` alloc calls
(−40.6%: 1,167,791 → 693,076); `total_actions=29875`
byte-identical; 4346 passed / 21 failed (the
pre-existing profiling-feature `batch_movements`
failures, unchanged). This is instrumentation overhead,
not engine work — it vanishes in the production
(no-`profiling`) build — but it was drowning the
allocation signal the audit measures by, so collapsing
  it makes every remaining engine allocation visible.

**2026-10-10 (allocation audit round 5 — `SmallVec<[usize; 2]>`
for `card_indices`/`indices`; `total_actions` byte-identical):**
the last `Vec` on the per-action path. `ActionParameters.
card_indices` and `ChoiceResult::CardSelected.indices` were
`Vec<usize>`, and every `SelectLiveCard` action and every
pending-choice card answer built a one-element `Vec` for them
(`Some(vec![hand_index])` in `generate_live_card_set_actions`,
15,040 calls; the `SelectCard`/`SelectTarget` arms of
`generate_pending_choice_actions`, 4,649 calls). Both are now
`SmallVec<[usize; 2]>` — the indices are a hand index plus, at
most, a second index for the double-baton area pair, so two
elements sit inline on the stack and the common one-element case
never touches the heap. The boundary structs that serialize the
indices (`link.rs` `LinkAction`, `web_server.rs` `FrameAction`)
keep `Vec<usize>` and convert at the boundary, so the wire
format is unchanged. `−116,196` alloc calls (−16.8%: 693,076 →
576,880), entirely from the 8–15 B size class (214,462 →
98,266, −54% — a one-element `Vec` header is exactly that
class); `total_actions=29875` byte-identical. The production
build (default features, pool active) is timing-neutral-to-faster:
aqours_cup 2000 games ~165–170k aps, above the 143,231 record.
Test call sites updated with `.into()` conversions (28 sites
across 11 files); the `resume_traced` helper signature is now
`Option<SmallVec<[usize; 2]>>`.

**What the audit found is left, and why each is not a
quick win** (so it is not rediscovered as if new):
- ~~The **`Timer` instrumentation is ~42% of every
  allocation counted in a `--features profiling,
  alloc_tracker` build**: each `Timer::drop` allocates
  one `Arc<[&'static str]>` call-path key (already
  reduced from four map-insert clones to one).~~
  **Addressed in round 4** (above): the key is now
  interned per unique path, so `Timer::drop` no longer
  allocates on repeated paths. `check_timing`'s engine
  work remains allocation-free.
- The **`execute_move_cards` `destination`/`target`
  `String`s are blocked by a borrow conflict**, not by
  effort: `dest` (derived from `destination`) is live
  across the `gs.resolve_target_player_mut(&target)` call
  (handlers.rs:407), so a `Cow`/borrowed `destination`
  would borrow `gs` across a mutable `gs` borrow. The
  `card_property` half of the same item *was* landable
  (above) because its `filter` dies at the `retain_valid`
  call before any mutable borrow.
- ~~The **`card_indices: Some(vec![hand_index])`** on every
  `SelectLiveCard` action (~75k allocs, the dominant 8–15 B
  class in `generate_live_card_set_actions`) is
  **load-bearing**: `resolve_hand_answer_index` prefers
  `card_indices.first()` over the `card_id` fallback, and
  `get_card_index_by_id` returns the *first* match, so a
  hand holding two copies of the same card id is
  disambiguated only by the exact `hand_index`. Removing it
  needs a `SmallVec` type change across every `card_indices`
  consumer, not a local fix.~~ **Landed in round 5** (above):
  `card_indices`/`indices` are `SmallVec<[usize; 2]>`, so the
  one-element `Vec` is gone while the exact `hand_index`
  disambiguation is preserved (the `SmallVec` still carries it).
- The **choice-action label `String`s** (`generate_pending_choice_actions`'
  PAY_SKIP arm uses raw `.to_string()`, not the display-gated
  `action_desc!`) are display-only — `make_action` and
  `with_ja` both gate on `action_display_enabled()` and
  discard them in headless mode — but gating the *caller's*
  construction is low-frequency (the cost prompt) and the
  `card_no` field may be read by bots, so it is left for a
  deliberate pass.
- The rest are **genuine**: the `EffectOwner` deep clones
  (the 128–255 B class in the ability-resolution path), the
  `TemporaryEffect` owned `String`s (stored in a `Vec`, they
  outlive the call), and `push_temporary_effect`'s three
  `to_string()`s. Already-correct and not re-proposed:
  `push_movement_event` (`ArcStr` cause, gated
  `batch_movements`, conditional `recently_moved_from_zone`),
  `make_params`/`make_action_params` (all-`None`, empty
  description), `action_desc!` (gated, `String::new()` in
  headless).

**Pre-existing test failures under `--features
"profiling alloc_tracker"` (test-infrastructure, not a
perf regression and not from these changes):** `cargo
test --release --features "profiling alloc_tracker"
--test run_all` fails **21 tests** (4346 passed) on the
*clean* tree — identical before and after this round's
edits, so these edits add zero failures. The failures
cluster in the auto-ability trigger path:
`jidou/energy_watch/*` (energy-placed-by-own-effect →
gain blade), `rules/trigger_paths/*` (paired abilities,
trigger scope), `effects/recover/per_card/
movement_tracking_characterization_test`
(`push_movement_event_syncs_all_views`), and
`bp7_parser_gap_cards`. These are **`profiling`-feature-
specific, not correctness regressions**: the tests assert
on behaviour that depends on `batch_movements`, which
`push_movement_event` gates behind
`cfg!(not(feature = "profiling"))`. The
energy-placement condition
(`ability/condition/movement.rs:810-861`) reads
`batch_movements` for its **pre-enqueue** scan (the
durable `snapshot_movements` governs only post-enqueue),
so in a `profiling` build the pre-enqueue scan finds no
movements and the watcher does not fire — hence the
blade-count assertions read 0. `push_movement_event_
syncs_all_views` asserts on `batch_movements` directly.
The non-profiling confirmation run (`cargo test --release
--test run_all`, no features) was attempted but its
from-scratch LTO recompile exceeded the session budget;
the profiling-specificity is established from the code
path, not from that run. The doc's "4364/4364" gate
should be read as the **no-feature** run; the
`profiling,alloc_tracker` run is the allocation-
measurement build, where these 21 are expected to fail.
Separate from the allocation work.

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
| `find_printed_ability` double-resolve | ~1 | ✓ landed (returns the `Arc` already in hand) |
| `ChoiceRoute` stringification ×2 | ~2 | open (trivial) |
| `requeue_remaining_after_choice` → `into_iter()` | ~32 × steps | ✓ landed (slices `queued[appended_from..]`, no `tail` Vec) |
| lazy `apply_modify_cost_to_ability_cost` clone | ~32/no-cost ability | open |
| `execute_modify_required_hearts_standard` → slice | 1+N/heart mod | ✓ landed (both Vecs → `SmallVec`, 2026-10-09) |
| `post_cost_position_gate` → no `Vec` | 1/resolution | ✓ landed (`SmallVec<[Keyword;3]>`) |
| `ResumeSnapshot::capture` — 10 lookups → 1 | 2/choice answer | ✓ landed (one probe; `entry_player_or_p1` borrows, 2026-10-09) |
| `ResolveSnapshot` / `Zone::from_str` hoists | 0 (branches) | open |
| `process_player_abilities_depth`'s `HashMap` → `SmallVec` | 1–2/drain | ✓ landed (`SmallVec<[((i16,usize),u8);8]>`) |
| `set_indices_waited/active` buffers | 3/multi-energy payment | open (callers' Vecs are empty-on-common-path) |
| `performer_owned_ids` → three-zone scan | 1/performer | ✓ landed (`performer_owns` closure over three zones) |
| `check_invalid_live_cards` → `SmallVec` | 2/`check_timing` | ✗ closed — measured no-alloc: `collect()` on a `filter_map` (lower bound 0) does not pre-allocate, so the all-live/empty common path allocates nothing |
| `zone_card_ids`/`_occupied` → iterators | 2/zone query | ✓ landed (the one caller borrows via `zone_cards`; `zone_card_ids` remains for API users) |
| `get_zone_cards` and friends → borrow | 1/zone eval | ✓ landed (`zone_cards` returns `&[i16]`) |
| `counted_by_any().to_vec()` → borrow | 1+N/card/count | ✓ landed (`recalculate_constant_cost_modifiers_with_ids` borrows the iterator, 2026-10-10) |
| `Active` phase: parse once, `SmallVec` not `HashSet` | 1/Active phase | ✓ landed (`constant_blocked` parsed once into `SmallVec<[i16;8]>`; `all_card_ids` already a `SmallVec`) |
| `execute_performance_phase`'s `nhm_flat` + partition | 3/performer | ✗ closed — `collect()` on an empty `need_heart_modifiers` map (the common case) allocates nothing |
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
| **`ability_master_id()` → `Option<&str>`** — 199 call sites × 1 alloc + up to 12 zone scans on the fallback; the largest single allocation count in the catalogue | high | low | `total_actions` + a debug-assert old-vs-new player-resolution diff over one full run | ✓ landed (borrowed, 2026-10-09 or earlier — verify against source) |
| **`get_card_names` allocation-free + `normalize_name` borrowing** — 4 allocs × 67 call sites × per-candidate | high | medium | exhaustive corpus equivalence test | ✓ landed 2026-10-09 (`Vec<&str>`, −71,301 allocs/200 games, 4364/4364) |
| `store_pending_choice` gating | ~20/choice offer | low | `total_actions` + grep log assertions | ✓ landed 2026-10-10 — the dedup-signature block is gated on `logging_enabled() \|\| log_enabled!(Debug)`; −25% alloc calls on the bench deck (§7) |
| `filter_subset` / `CardFilter::from_effect` single-bind | 200–300 dependent loads; 10 probes → 1 | medium | `total_actions` + full suite |
| `execute_move_cards` borrow its 8 `String`s | ~9/MoveCards | low | `total_actions` | partial 2026-10-10 — the `SelectionFilter.card_property` `String` → `Option<&'a str>` landed (borrowed from `card_property_any()`); the `destination`/`target` `String`s are **blocked by a borrow conflict** (`dest` is live across the `resolve_target_player_mut` call at handlers.rs:407), not by effort |
| `Option<&Ability>` in `AbilityRef` (the remaining ~50 tight-loop `resolve()` sites) | ~200–300 atomics/action | small | measure at `--jobs 1` **and** `--jobs 256` | partial — `AbilityRef::peek()` (`Option<&'static Ability>` off the `OnceLock` cache) exists; the ~50 `resolve()` call sites are not yet converted to it |
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

### 4.5 Build configuration

**Landed 2026-10-09:** `lto = "thin"` + `codegen-units = 1`
(+5.6% jobs=1, +4.2% jobs=256 — the suite-completion blocker
did not reproduce; 4364/4364 under the LTO build) and **PGO**
(+47.7% jobs=1, +47.0% jobs=256, profile committed at
`engine/pgo/merged.profdata`, build command in §1). Both are
in `Cargo.toml` / the canonical command respectively.
`target-cpu=native` was measured and rejected (+0.8% — noise;
the workload is branch-heavy pointer-chasing, not
vectorizable). Still open: `[profile.bench]` (free, and it is
a measurement bug) → the three cross-crate `#[inline]`s →
`bench_no_log` (decide the canonical command explicitly) →
`panic = "abort"` (gate on `bot_arena`).

### 4.6 Multi-deck warmup pathology (pre-existing, open — engine bug, not perf)

`sim_bench`'s warmup loop (the 10s pre-sweep that runs 4
games per deck per pass, seeds
`game_seeds(seed ^ 0xA5A5A5A5, pass*10000 + deck_idx*10 + w + 1)`)
**hangs on the full 13-deck corpus**: a run with the default
warmup never completes (killed at 15 min, 105 s+ CPU and
counting), while the same binary with
`SIM_BENCH_WARMUP_SECS=0` finishes the whole corpus in
0.57 s. Reproduces on the **unmodified pre-session binary**
(`sim_bench_base`, no pool, no LTO, no code changes), so it
is not a regression. Passes 1–16 × all decks × all 4 warmup
seeds probed clean (per-game `--seed` replication of the
warmup seed math, 3 s watchdog each); the pathological
(pass, deck, w) is in passes 17–63 and was not isolated.
The game loop's 600-iteration cap bounds *iterations*, not
*per-iteration cost*, so this is either a game whose
iterations are pathologically expensive or an unbounded
internal loop (e.g. in `auto_advance_one` /
`execute_and_settle`'s settle loop) that the cap does not
bound. Worth a dedicated investigation: a random-policy game
that never terminates can hang a real match, not just the
bench. Workaround for corpus-wide runs:
`SIM_BENCH_WARMUP_SECS=0`.

## 5. Closed — do not retry without re-measuring

| idea | recorded result |
| --- | --- |
| `lto = "fat"` + `codegen-units = 1` | +2–4% single-threaded, inside noise; appeared to cost throughput at high `--jobs`. Deliberately uncommitted. |
| Pooling allocator (`fast_alloc`) | **Landed 2026-10-09: +13%, extended to +17% with the floor fix.** jobs=1: 143,231 aps vs 124,386 baseline; jobs=256: 879,066 vs 844,883 pool-only; `total_actions=328173` byte-identical; 4354/4354 tests pass *with the pool active*; allocator self-time 11.5% → 3.0%. The previous "2.8% slower" measurement was on a pool whose `FreeLists` was `#[derive(Clone, Copy)]` = 310 bytes, and `Cell::get()`/`Cell::set()` copied the whole struct on every alloc and every dealloc — ~1.5 KB per alloc+dealloc pair × 7.3M pairs ≈ 11 GB of copying, which dwarfed whatever the pool saved. The fix is an 8-byte pointer cell (`Cell<*const FreeLists>`) mutating through a raw pointer, with the struct allocated through `System.alloc` at first use so `Box::new` cannot re-enter and stack-overflow on the first allocation. The floor extension 32B → 16B (`3f7d41f3`) captures the 8–15B class (2.2M of 7.3M allocations, 30%) that fell through to System. Enable with `--features fast_alloc`. |
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
| `--jobs` plateau is 256, not 128 | **Corrected 2026-10-09.** The high-end sweep (128/256/512/1024/2048) shows the peak is at 256 (844,883 aps), not 128 (795,987); it degrades past that (512 −4%, 1024 −16%, 2048 −11%). The old "flat from 64 to 256" line was measured before the pool landed and is stale. `total_actions=328173` is identical at every setting. |
| `card_database` refcount contention is the top multithreading cost | **Disproven 2026-10-09 by measurement.** jobs=64 → 779,913 aps, jobs=128 → 879,066 aps (+12%). If contention were the bottleneck, jobs=128 would be *worse* than jobs=64; it is better, and throughput degrades only past the 256 plateau. The "invisible at `--jobs 1`, appears only at the top of the thread range" explanation was wrong — three profiling rounds that mandated `--jobs 1` never saw it because it was never there. The `Box::leak` fix is real but noise-level. |
| `lto = "thin"` + `codegen-units = 1` | **Measured 2026-10-09: +5.6%** (909,674 / 898,071 / 868,727 aps at jobs=256 vs 844,883 pool-only, `total_actions=328173` byte-identical), but doubles build time (~30s → ~59s) and the full test suite could not be run to completion under LTO here, so it is NOT set in `Cargo.toml` — the measurement is recorded in the `Cargo.toml` comment for whoever wants to re-measure with the 4354/4354-tests gate. |

## 6. Parallelism — the largest single result

Not engine work; worth more than everything in §3 combined. Machine:
Ryzen 5 5600H, 6 physical cores / 12 logical. In games mode
`total_actions` is identical at every setting (984,519 on the
2000-game × 3 aqours_cup sweep); in secs mode the played count varies
with machine speed, which is inherent to a wall-clock budget.

Current numbers (`--deck aqours_cup --games 2000 --seed 1 --policy
random`, pool-only build, floor=32 — the sweep I actually measured):

| `--jobs` | gps | aps | speedup |
| --- | --- | --- | --- |
| 1 | 759 | 124,386 | 1.00x |
| 2 | 1,486 | 244,296 | 1.96x |
| 4 | 2,651 | 434,139 | 3.49x |
| 6 | 3,351 | 550,006 | 4.42x |
| 12 | 4,407 | 723,730 | 5.82x |
| 64 | 4,754 | 779,913 | 6.27x |
| 128 | 4,851 | 795,987 | 6.40x |
| **256** | 5,149 | **844,883** | **6.80x** |
| 512 | 4,930 | 808,901 | 6.50x |
| 1024 | 4,307 | 706,755 | 5.68x |
| 2048 | 4,576 | 750,920 | 6.04x |

After the pool floor extension to 16B (`3f7d41f3`), only two points were
re-measured cleanly: jobs=1 → 143,231 and jobs=256 → 879,066 (+4%).
The full sweep was NOT re-run after the floor extension, so the table
above is the pool-only sweep and the +4% should be applied to it, not
to a fabricated row. The pre-sizing change (`81105460`) was measured
under transient machine load (jobs=256 came back 474k / 748k / 766k
aps across three consecutive runs) and is noise-neutral — those runs
are excluded from every number in this document.

**Run the benchmark at `--jobs 256`, not `--jobs 12`** — +6% over 128
threads, and it is the throughput peak. The plateau is **256, not 128**:
the old "flat from 64 to 256" line was measured before the pool landed and
is stale. Beyond 256, oversubscription degrades (512 −4%, 1024 −16%,
2048 −11%) — thread-spawn overhead and cache pressure outweigh the
parallelism. `total_actions=328173` is identical at every setting, so
determinism holds across the whole range. Each game is ~0.4 ms and the
live state is L1-resident, so 10× oversubscription of a 12-thread box
is free until ~512 threads.

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
worker, cloned from 35 sites on every card play plus once per ability
resolution — an atomic increment on a single cache line from many
cores. **The claim that this is the top multithreading cost is
disproven by measurement:** jobs=64 → 779,913 aps, jobs=128 → 879,066
aps (+12%). If refcount contention were the bottleneck, jobs=128 would
be *worse* than jobs=64 (more cores hammering one cache line); it is
better, and throughput degrades only past the plateau at 256. So the
`Box::leak` fix is real but noise-level, and the "invisible at
`--jobs 1`, appears only at the top of the thread range" explanation
was wrong — three profiling rounds that mandated `--jobs 1` never saw
it because it was never there. The card database is read-only for the
whole run: `Box::leak` it once at startup and store a `&'static`, or
thread the reference through. Zero refcount, zero atomics. The same
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
