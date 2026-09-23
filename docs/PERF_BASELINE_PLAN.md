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

- All `engine/analyze_*.py`, `engine/*_probe.py`, `engine/show_game.py`,
  `engine/search_card.py`, `engine/count_batons.py`,
  `engine/extract_examples.py`, `engine/hunt_bad_plays.py` →
  **`tools/analysis/`**
- Root `tmp_joint_report.py` → `tools/analysis/joint_series_report.py`
- Root `tmp_duration_gaps.py` → `tools/analysis/ability_duration_gaps.py`
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
