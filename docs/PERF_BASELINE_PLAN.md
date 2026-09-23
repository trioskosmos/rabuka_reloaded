# Engine Performance Baseline Plan (Phase 0)

Status: executed as part of the engine-speedup program. This document is the
plan and the contract for what “baseline” means; recorded numbers live in
`test_output/baseline_<date>.md`.

## Goal

Establish a reproducible, provenance-stamped measurement of full-game
simulation throughput (games/sec) and its supporting metrics (allocs/game,
hotspot shares, Criterion anchors) **before** any engine optimization work.
Every later perf phase compares against this baseline.

## Why the existing tools were not enough

| Tool | Problem |
|---|---|
| `profile_target` | 5000 games hardcoded, **no timing output**, unstable `read_dir[0]` deck, nondeterministic `thread_rng` policy, raw `execute_main_phase_action` path (not the AI/production path) |
| `bot_arena` | Prints gps, but embeds `policy_call` checkpoints + `V3Plan` detection; different loop overhead — fine as cross-check, not as sole number |
| Criterion `game_throughput` | Only runs with `--features profiling`, which **changes gameplay behavior** (suppresses action metadata) — good for relative A/B, not absolute truth |
| `alloc_counter` | `start()` only wired into the test harness; no full-game alloc counts from any bin |
| Docs history | Only four uncontrolled wall-times (≈95–103 gps), explicitly caveated |

Silent traps: running without `--release` (≈10× off); comparing
profiling-on vs profiling-off as if timers were the only difference; bare
`cargo bench` runs nothing (needs `--features profiling`).

## Deliverables

1. **`engine/src/bin/sim_bench.rs`** — canonical throughput bench
2. **`tools/baseline.ps1`** — one-command baseline suite
3. **`test_output/baseline_<date>.md`** — recorded numbers + provenance

## `sim_bench` design

| Aspect | Decision |
|---|---|
| Execute path | `game_setup::execute_action` + `settle_single_player_state` (`bin_common::execute_and_settle`) — same path as bot_arena / search / RL |
| Decks | All decks in `../web_ui/decks` by default; `--deck NAME` to narrow |
| Games | `--games N` per deck (default 2000) |
| Determinism | Per-game `rng::seed(engine_seed)` + local `rng::Lcg` for policy (bot_arena `game_seeds` pattern) |
| Policies | `--policy random` (default, raw engine cost) \| any `BotKind` name (`v2`, `v6`, `v7`, …) |
| Loop routing | Mirrors bot_arena: pending-choice → Action; RPS → Lcg; ChooseFirstAttacker → S6 second-attacker preference; Mulligan/LiveSet/Main → registry dispatch; `policy_call` RNG checkpoint/restore around bot calls |
| Caps | 600 outer iterations / 200 same-turn stuck (bot_arena) |
| Timing region | Card DB + deck template build **outside**; per-game deal + full loop **inside** |
| Output | Per-deck: games, actions, wall time, **gps**, actions/sec, outcome histogram; aggregate mean/min/max gps; `--repeat N` for variance |
| Debug guard | Loud `SIM_BENCH=DEBUG-BUILD` warning when `cfg!(debug_assertions)` |
| Alloc mode | `--alloc` → `alloc_counter::start()` (needs `alloc_tracker` feature + `RABUKA_ALLOC_TRACK`); report printed after the timed loop |
| Logging | No `env_logger::init()`; default features; **no** `profiling` feature |

## Baseline suite (`tools/baseline.ps1`)

1. Provenance: git HEAD + `git status --short` + feature set.
2. **Canonical gps**: `sim_bench --policy random --repeat 3` over all decks.
3. **Policy-inclusive gps**: `sim_bench --policy v2` on a fixed deck
   (`5CP3Z idou`).
4. **bot_arena cross-check**: `bot_arena --games N --seed 1 random random`
   (labeled: different loop overhead).
5. **Criterion anchor**: `cargo bench --features profiling -- game_throughput
   --save-baseline base` (relative metric only; never compared absolutely to
   sim_bench).
6. **Flamegraph**: `profile_target --features profiling` folded output →
   `gen_flamegraph` caller + self SVGs (hotspot % baseline only).
7. **Alloc profile**: `sim_bench --alloc` with `--features alloc_tracker` →
   allocs/game, bytes/game, size buckets.
8. Summary written to `test_output/baseline_<date>.md`.

## Script consolidation

While establishing the baseline, scattered one-off analysis scripts are
consolidated:

- Current arena analyzers live in **`tools/analysis/`** and consume the
  versioned JSONL audit format. The obsolete CSV readers and one-off report
  scripts were removed rather than maintained alongside the current path.
- Root `tmp_joint_report.py` and `tmp_duration_gaps.py` were one-off reports; their
  useful conclusions are retained here or in the current audit, and the temporary
  analyzers were removed.
- CWD-relative paths (`../test_output/...`) rewritten to resolve from the
  repo root via `Path(__file__)`.
- Hardcoded `C:\Users\trios\AppData\Local\Temp\...` paths replaced with
  `sys.argv` inputs.
- Stale one-off `engine/*.log` files and root `tmp_*.txt` /
  `joint_group_report.txt` litter removed.
- Platform build scripts (`platforms/**`), `start.bat`, `tools/*` (already
  consolidated), and `cards/*` pipeline scripts are **left alone**.

## Verification bar

- gps variance across `--repeat 3` runs < ~5% (else investigate).
- Outcome histogram shows a plausible mix (not degenerate instant draws).
- Debug guard fires on a debug build.
- Suite re-run reproduces within ~5%.
- `cargo test --test run_all` green after adding the bin / moving scripts.

## Non-goals

- No engine optimizations in this phase (they start only after the baseline
  is recorded).
- No changes to `profile_target`’s loop (flamegraph/replay tooling depends on
  its shape).
- No PPO/RL pipeline work.

---

## Phase 1 — Training-mode gating (web full, sim fast)

Status: implemented and measured. Goal: keep the browser path fully playable
(display + logs) while training/sim strips UI-only work so the same binary is
fast offline.

### Runtime gates (`game_setup`)

| Flag | Default | Off skips |
|---|---|---|
| `ACTION_DISPLAY` (`set_action_display`) | ON | Action EN/JA `description`, `card_name`/`card_no`/`source_ability`, `existing_member_name`, mulligan/live-set card copies. Decision fields (`card_id`, `stage_area`, `available_areas`, costs, indices) always built. |
| `LOGGING` (`set_logging_enabled`) | ON | `push_rule_log` / `push_structured_log` / `push_debug_note` / choice offer+resolve / `commit_or_push_structured` / phase+RPS log construction; `format!` materialization via `*_fmt(format_args!)` helpers; UI provenance strings (`ui_text`/`ui_kind` in `recalculate_constants` BonusSource / score-source / jyouji status). |
| `set_training_mode(bool)` | false | Convenience: sets both flags off when `true`. |

Web/`main.rs` and the test suite never call these — defaults stay full
display + full logs. `sim_bench` sets them from env for same-binary A/B:

- `RABUKA_ACTION_DISPLAY` (default off in bench)
- `RABUKA_LOGS` (default off in bench)

### Hot-path call-site pattern

`push_rule_log_fmt(format_args!(...))` / `push_debug_note_fmt(format_args!(...))`
return before the format materializes when logs are off. Bare
`push_*(String)` still early-returns (saves the Vec push; callers that already
own a `String` are rare on the hot path). Structured `LogEntry { .. }`
construction is gated at the call site (phase, choice, live result, RPS).

### Not gated (correctness / gameplay)

- Performance snapshots (`build_snapshot`, `performance_snapshots`) — live
  victory and conditions read them.
- Decision fields on `Action`.
- Provenance **values** used for score math (`constant_score_bonuses` etc.);
  only display strings on BonusSource / score-source lines are emptied.
- Dirty-flag for `recalculate_constants` still deferred — conditions read
  live state (energy, zones) that a naive version counter misses (51 tests).

### Measured (same binary, `5CP3Z idou`, seed 1, 500×3, policy random)

| Mode | mean_gps | total_actions |
|---|---|---|
| display+logs ON (web-equivalent) | **163.2** | 390186 |
| display+logs OFF (training) | **216.3** | 390186 |

**+32.5% gps** with identical action count (decision path unchanged).
Earlier display-only A/B (logs still on): 151.5 → 167.0 (+10%); log gating is
the larger half.

Raw: `test_output/train_logs_{on,off}_ab.txt`.

### Verification

- `cargo build --release --bin sim_bench` OK
- `cargo test --test run_all` → **3418 passed**
- `python cards/test_inventory.py --check` EXIT=0
- Analysis `py_compile` OK; baseline script PARSE_OK

### Wired

- `bot_arena`: `set_action_display(false)`; `set_logging_enabled(options.logs)`
  (file dumps still available via `--logs`).
- `collect_data` / `bot_data_gen` / `ppo_collect` / `hunt_loop`: `set_training_mode(true)`.
- `sim_bench`: env-gated `RABUKA_ACTION_DISPLAY` / `RABUKA_LOGS` (default off).

### Still to wire

- Criterion `game_throughput` A/B vs saved `base` after broader multi-deck
  sim_bench confirmation.

---

## Phase 2 — Parallel multi-game execution (`--jobs`)

Status: implemented and measured. Goal: biggest honest throughput lever —
run independent full games on multiple threads (multi-core ≈ cores×, not
orders of magnitude).

### Thread-safety audit (desktop)

| Hazard | Finding |
|---|---|
| `GameState` | No `Rc`/`RefCell`/`Cell` in `core` — `Send` (asserted in `sim_bench` `main`) |
| Engine RNG | **Was** `static Mutex<u32>` with `seed`/`checkpoint`/`restore` per game — concurrent games stomped each other. **Now** thread-local `RefCell<u32>` on the std path (`rng.rs`); same API, per-thread streams |
| `bot/rollout` PLAN_CACHE | already `thread_local!` |
| `ability/log` VERDICT_BUFFER | already `thread_local!` |
| `ABILITY_DEBUG` buffers / `timer` | `Mutex` — off on the sim path (no `profiling`, debug flag false) |
| `ability_store` / bytecode / executors | `OnceLock` — fine |
| `Pool`/`EkBox` | `unsafe impl Send/Sync` + `Mutex` free list |
| `ACTION_DISPLAY` / `LOGGING` | `AtomicBool`, set once before workers |
| `card_binary` `static mut` blob | read-only after load on desktop path |

### `sim_bench --jobs N`

- Default **`--jobs 1`** (baseline-comparable, single-thread).
- `--jobs N` splits one deck’s game range across `std::thread::scope`
  workers; each game index still maps to one `game_seeds` pair (worker-
  independent). Engine RNG is thread-local; `run_game` seeds that worker.
- Setup + summary lines print `jobs=`.
- Keep `--jobs 1` when comparing to recorded baselines / Criterion.

### Measured (same binary, `5CP3Z idou`, seed 1, 500×3, random, training gates off)

| jobs | run | mean_gps | total_actions |
|---|---|---|---|
| 1 | A | 195.1 | 390186 |
| 1 | B (warmer) | **221.8** | 390186 |
| 8 | A | **840.4** | 390186 |
| 8 | B | **840.9** | 390186 |

- **Determinism:** `total_actions`, outcome histogram (741/723/36), and
  end reasons identical across all four runs → same games at any `--jobs`.
- **Speedup (honest):** 840.9 / 221.8 ≈ **3.8×** at `--jobs 8` (not 10⁴;
  limited by cores + SMT + short games / sync overhead).
- Raw: `test_output/jobs{1,8}_ab{,_rep}.txt`.

### Verification

- `cargo build --release --bin sim_bench` + `--bins` OK
- `cargo test --test run_all` → **3418 passed**
- `python cards/test_inventory.py --check` EXIT=0
- `py_compile` tools/analysis OK; `tools/baseline.ps1` PARSE_OK

### Caveats

- Engine RNG is now **per-thread**, not process-global: a second thread
  that never calls `seed` starts from the platform default (`1` on
  desktop), not from the main thread's mid-stream state. Single-threaded
  callers are unchanged. The `link.rs` concurrent test builds both
  states serially before spawn (already required with the old Mutex).
- Do not compare absolute gps of multi-job runs to single-job baselines
  without labeling `jobs=`.

## Deterministic replay harness

The external record/replay harness is the supported way to compare an
optimization candidate against a captured workload. It is implemented by
`tools/flamegraph_replay.py`, `tools/flamegraph_replay.rs`, and
`tools/test_flamegraph_replay.py`; it does not modify the engine build script.

Build it outside the repository, then record and replay from `engine/`:

```powershell
py -3 tools/flamegraph_replay.py build --build-dir "C:\\Users\\trios\\AppData\\Local\\Temp\\kilo\\replay-build"
& "C:\\Users\\trios\\AppData\\Local\\Temp\\kilo\\replay-build\\target\\release\\flamegraph_replay.exe" record "C:\\Users\\trios\\AppData\\Local\\Temp\\kilo\\replay-trace.jsonl" "../web_ui/decks/muse_cup.txt" 2 42 7
& "C:\\Users\\trios\\AppData\\Local\\Temp\\kilo\\replay-build\\target\\release\\flamegraph_replay.exe" replay "C:\\Users\\trios\\AppData\\Local\\Temp\\kilo\\replay-trace.jsonl" "../web_ui/decks/muse_cup.txt"
```

The launcher records source, asset, lockfile, feature, compiler, and executable
identity in `provenance.json`; replay rejects a mismatched build identity. It
checks ordered action payloads and a bounded state projection, including zones,
phase, RNG checkpoint, queue, snapshots, movement history, and RPS choices.

This is strong regression evidence, not complete engine equivalence: modifiers,
resolver internals, serde-skipped fields, and logs are outside the projection,
and display metadata changes intentionally invalidate a trace. Engine-call time
also includes tracing and clock overhead, so use clean `sim_bench` numbers for
throughput claims.

The old investigation and its detailed capture history were consolidated here;
the phase-by-phase record remains in Git history.
