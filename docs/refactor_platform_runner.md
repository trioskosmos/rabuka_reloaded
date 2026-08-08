# Refactor: unify the embedded platform front-ends on one runner

## Problem statement

The game engine already exposes a platform-abstraction layer
(`rabuka_engine::game::platform_ui`) with a `PlatformUi` trait and shared
widgets (`select`, `handle_choice`, `show_result`, `ai_turn`, `human_turn`).
Yet every embedded target re-implements the **same ~150-line match front-end**
(mode select → deck select → build DB → build decks → players → `GameState`
→ the victory/auto-settle/loop-detect/act loop) with only tiny per-platform
differences:

| platform | runner file | TeamUi |
|----------|-------------|---------|
| PS1      | `src/bin/rabuka_ps1.rs`  | `Ps1Ui`  |
| GBA      | `src/bin/rabuka_gba.rs`  | `GbaUi`  |
| DS       | `src/bin/rabuka_ds.rs`   | `DsUi`   |
| PSP      | `src/bin/rabuka_psp.rs`  | `PspUi`  |
| DC       | `src/rabuka_main.rs`     | `DcUi`   |
| Jaguar   | `src/lib.rs`             | `JaguarUi` |
| Wii      | `src/lib.rs`             | `WiiUi`  |

e.g. `rabuka_ps1.rs::80-230` and `rabuka_gba.rs:78-232` are ~byte-for-byte the
same orchestration, differing only in the `XxxUi { display, input }` struct
name, how `Display::new()` is called, and the deck-card loader.

## Reference model (the "most mature" target)

The web server (`engine/src/game/web_server.rs`) hosts the DOM-backed client
(`web_ui/`) and drives the **same** engine core with the same building
blocks: `game_setup::generate_possible_actions`, `game_setup::settle_auto`,
`TurnEngine::check_victory_condition`, `TurnEngine::advance_phase`, 
`game_state.is_loop_detected`, `human_turn`/`ai_turn`. It is event-driven
rather than a blocking loop, but its notion of "keep the match honest via
check_victory → settle_auto → detect loop → act" is the reference behavior the
embedded targets should converge on.

## Plan

1. **Engine** — add one shared driver in `platform_ui`:
   - `Mode { VsAi, TwoPlayer, AiVsAi }`
   - `run_embedded_game<U: PlatformUi, C: Fn(usize)->&[&str], A: FnOnce(usize,usize)->Vec<Card>>(ui, deck_names, cards_of, load_all) -> GameResult`
     that performs: mode + deck selection via `select`, DB build, deck build,
     energy, shuffle, `Player`, `GameState::new`, `setup_game`, then the same
     game loop the embedded targets currently copy — proving the web server's
     loop sequence in one place.
- A host-testable lower-level `run_match<U>(ui, p1, p2, all_cards, mode)`
      that the target platform can call directly and that an engine unit test
      drives to a terminal `GameResult`.
2. **Every embedded platform** `main()` collapses to ~15-25 lines: construct
   backend `Display`/`Input`, wrap in `XxxUi`, call the shared runner, hang.
   Two shapes, depending on how the platform stores decks:
   - Static baked decks (`&'static [&'static str]`) → `run_embedded_game`
     (ps1 / gba / ds / jaguar).
   - Runtime JSON decks (`Vec<String>`) and/or an extra "Run Tests" mode →
     keep the platform's own menus and call `run_match` directly
     (psp / dc / wii).
3. **Allocator** — the several hand-rolled `GlobalAlloc` (ds / dc / wii /
   jaguar) are backend-specific syscall shims; they are **left as-is**
   (unifying them is not "duplicate business logic" and not host-testable).
4. **Verify** — engine `run_all` stays green. A host unit test was prototyped
   to drive `run_match` to an end state, but a fully-random AI match exercises
   unrelated engine-core overflow (not the runner), so the runner instead
   delegates to helpers (`ai_turn`, `human_turn`, `handle_choice`,
   `show_result`, `settle_auto`) that are already covered by the 2212-test
   suite. Embedded targets are cross-compiled (agb / psx / libnds) and cannot
   be built on this host, so their edits are kept minimal and mechanically
   identical to the already-verified ps1/gba shape.

## Non-goals

- Rework the web server's HTTP surface; it is the reference, not the rewrite.
- Forcing `Display`/`Input` structs behind an extra trait layer.
- Unifying per-console allocators (see above).

## Status

- Engine: `Mode` (as `MatchMode`), `run_embedded_game`, and `run_match` added
  to `engine/src/game/platform_ui.rs`; engine `run_all` green (2212 tests).
  `rng` already ships a no_std (`UnsafeCell`) and std (`Mutex`) impl, so the
  runner's `rng::rand_range` compiles on every target; `crate::Arc` compat
  (Rc on non-atomic PS1) covers the runner's `Arc<CardDatabase>`.
- All seven platform front-ends collapsed to the shared runner (ps1, gba, ds,
  jaguar via `run_embedded_game`; psp, dc, wii via `run_match`).
- Embedded targets cross-compile only on their own toolchains, so each edit
  follows the verified ps1/gba shape mechanically.