# Porting Rabuka Engine to Nintendo 3DS — Plan & Progress

This document explains what I am doing, why, and the progress so far toward making the Rust `rabuka_engine` portable to constrained platforms such as the Nintendo 3DS.

## Goals
- Identify portability failpoints for 3DS and similar constrained targets.
- Provide a minimal, testable harness to exercise game logic on desktop without `actix`/`tokio`.
- Add build-time feature gating so server-only dependencies can be excluded for cross-compilation.
- Produce a proof-of-concept 3DS binary that auto-plays through the game.

## Changes made (progress)

### 1. Cargo feature gating (done)
- `actix-web`, `actix-cors`, `actix-files`, `tokio`, `tokio-stream`, `local-ip-address`, `uuid`, `bytes` are all optional, gated behind the `server` feature.
- Building with `--no-default-features` produces a minimal binary with no networking/async deps.

### 2. main.rs gating (done)
- `run_web_server` is gated with `#[cfg(feature = "server")]`.
- The `web-server` subcommand shows a debug message when the feature is disabled.
- The `rabuka_engine` binary compiles cleanly without `--features server`.

### 3. Interactive harness (done)
- `src/bin/harness.rs` — REPL-style binary for hot-seat play.
- Loads cards, builds decks, runs setup, lists legal actions, executes chosen actions.
- Includes `settle_single_player_state` to auto-advance through automatic phases.

### 4. 3DS proof-of-concept auto-play (done)
- `src/bin/rabuka_3ds.rs` — fully automated game loop.
- Picks actions automatically based on the current phase:
  - RPS: Rock for P1, Paper for P2 (guarantees a winner)
  - ChooseFirstAttacker: picks first option
  - Mulligan: skips (keeps all cards)
  - LiveCardSet: selects first card, then confirms
  - Automatic phases (Active, Energy, Draw, etc.): auto-advanced
- Resets loop detection after each action to prevent false-positive Draw results.
- Demonstrates end-to-end game progression through all phases.

### 5. start.bat fix (done)
- Updated to `cargo run --release --features server --bin rabuka_engine web-server` so the web server builds correctly.

## Dependency audit for 3DS cross-compilation

### Always-on dependencies (still included without `server` feature):
| Crate | 3DS risk | Notes |
|-------|----------|-------|
| `log` | Low | Works with ctr-std |
| `env_logger` | Medium | Needs stderr; replace with 3DS log output |
| `serde` + `serde_json` | Low | Heavy but portable; consider `serde_json` optional |
| `rand` | Medium | `getrandom` syscall may require 3DS backend |
| `smallvec` | Low | No OS dependencies |
| `uuid` | None | Now gated behind `server` |
| `bytes` | None | Now gated behind `server` |

### Always-excluded (behind `server` feature):
- `actix-web`, `actix-cors`, `actix-files` — not portable
- `tokio`, `tokio-stream` — not portable (async runtime)
- `local-ip-address` — not portable

## Key failpoints for 3DS

1. **`getrandom` for `rand` + UUID**: `getrandom` needs a 3DS-compatible backend. Use `getrandom` with `custom` feature or seed RNG manually.
2. **File I/O**: `cards.json` loaded from relative path `../cards/cards.json`. On 3DS, load from SD card (`/3ds/rabuka/cards.json`) via `ctru-rs` FS APIs.
3. **`env_logger`**: writes to stderr. Replace with `ctru-rs` console output or stub it.
4. **Threading / `std::sync::Mutex`**: The global `GAME_STATE` uses `Mutex`. On 3DS, a simple `RefCell` or no concurrency wrapper is sufficient for single-threaded use.
5. **Memory**: 3DS has ~128MB available. The card database and serde_json parsing may be heavy. Consider memory profiling.
6. **UI**: Web UI won't run. Must render via `libctru` (top screen) and accept button input (bottom screen).
7. **Multiplayer**: Network code depends on `actix` + `tokio`. Stub entirely for 3DS.
8. **Cross-compilation toolchain**: Requires `devkitARM` + `cargo-3ds` (https://github.com/rust3ds/cargo-3ds).

## How to build and run (desktop)

### Interactive harness (recommended for testing)
```powershell
cd engine
cargo run --bin harness
```

### Auto-play proof of concept
```powershell
cd engine
cargo run --bin rabuka_3ds --no-default-features
```

### Full web-server (original)
```powershell
start.bat
# or manually:
cd engine
cargo run --release --features server --bin rabuka_engine web-server
```

## Next steps for 3DS port

1. Set up `devkitARM` + `cargo-3ds` toolchain (Linux/WSL or Docker).
2. Add `ctru-rs` as a dependency behind a `3ds` feature flag.
3. Create a 3DS-specific entry point with:
   - FS: load `cards.json` from `sdmc:/3ds/rabuka/`
   - Render: display game state text on top screen
   - Input: D-pad/buttons for action selection
4. Cross-compile and test on Citra emulator.

Progress status:
- Research Rust on 3DS: done
- Repo scan: done
- Engine portability analysis: done
- Harness + feature gating: done
- 3DS proof-of-concept binary: done
- Interactive harness: done
- Documentation: done
- **Next: cross-compile attempt with cargo-3ds**
