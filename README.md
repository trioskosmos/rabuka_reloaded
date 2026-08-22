# Rabuka Reloaded

A certain school idol collectible card game engine, AI, and web UI — built in Rust and ported to a dozen platforms.

**2,526 cards · 936 unique abilities · ~2,500 tests · 129K lines of Rust**

[Web UI](#web-ui) · [Console Ports](#target-platforms) · [AI Bot](#features) · [Quick Start](#quick-start) · [Docs](#documentation)

---

## Features

- **Full game rules engine** — all phases (Active, Energy, Draw, Main, Live), all card types, full zone system (hand, stage, deck, waitroom, energy zone, etc.)
- **2,526 real cards** with **936 unique abilities** compiled from Japanese card text via a Python extraction pipeline
- **Custom bytecode VM** — compiles abilities into binary opcodes, reducing on-disk size from 1.4MB to 136KB (90% reduction), enabling console targets with <1MB RAM
- **ISMCTS AI bot** — Information Set Monte Carlo Tree Search handling imperfect information (hidden cards), with a neural network evaluation function
- **PPO training pipeline** — Proximal Policy Optimization to train the neural evaluation function; collects trajectories via self-play
- **Web UI** — full game board, card browser, deck converter, interactive tutorial, multiplayer via SSE + chat, QR code deck sharing, i18n
- **Memory-optimized** — bytecode VM + struct compaction keeps runtime RAM small enough for retro consoles (as low as 2 MB). DS (4 MB) ability activation crash fixed.
- **15 binary targets** — CLI harness, bot arena/demo, training data generation, game tracing, profiling, and platform-specific builds

## Target Platforms

| Platform | RAM | Status |
|----------|-----|--------|
| **PC / Web** (x86-64, aarch64) | unlimited | ✅ Works |
| **Nintendo 3DS** (ARM11 @ 268MHz) | 128MB | ✅ Works |
| **Nintendo Wii** (PowerPC 750 @ 729MHz) | 88MB | 🟡 Displays Japanese text — GX FIFO bug |
| **PlayStation Portable** (MIPS R4000 @ 333MHz) | 32MB | 🟡 Displays Japanese text — font rendering |
| **Nintendo DS** (ARM9 @ 67MHz) | 4MB | 🟡 Boots, loads decks, plays — heap exhaustion on ability activation fixed; needs build test |
| **PlayStation 1** (MIPS R3000A @ 33MHz) | 2MB | ✅ Works — full game flow, BIOS vblank event; card data streams from CD |
| **Game Boy Advance** (ARM7TDMI @ 16MHz) | 288KB | 🟡 Boots & plays full flow via agb object-text rendering; sprite-VRAM crash fixed — needs longer play test |
| **Sega Dreamcast** (SH-4 @ 200MHz) | 16MB | ✅ **Works — playable** via new wasm→C pipeline (rust→wasm32→wasm2c→sh-elf-gcc); full engine, text UI in Flycast. See `platforms/dc/wasm/` |
| **WebAssembly** (wasm32 headless) | unlimited | ✅ Works — full no_std engine + bytecode VM; headless AI-vs-AI match harness with C ABI (`platforms/wasm/`) |
| **SNES** (5A22 @ ~21MHz) | 128KB | 🔧 In progress — crt0 + build scripts in place |
| **Mega Drive / Genesis** (68000 @ ~7.6MHz) | 64KB | 🔧 In progress — WSL assemble pipeline in place |
| **Atari Jaguar** (Tom & Jerry, m68k) | 2MB | 🟡 Display/input modules present; active development |
| **Philips CD-i** (SCC68070 @ 15.5MHz) | 1MB | 🟡 Native m68k port; proves the engine fits 1MB with compact bytecode |

For a full portability analysis covering 15+ consoles (PS1, N64, GameCube, Vita, GBA, Saturn, and more), see [engine/PORTS.md](engine/PORTS.md).

## Quick Start

```bash
# Build & run the engine (CLI)
cd engine
cargo run --release

# Build with web server
cargo run --release --features server -- web-server
# → Open http://127.0.0.1:8080

# Run all ~2,500 tests
cargo test --test run_all

# Run benchmarks
cargo bench
```

### Docker

```bash
docker build -f Dockerfile -t rabuka .
docker run -p 8080:8080 rabuka
```

## Project Structure

```
rabuka_reloaded/
├── engine/            # Core Rust crate (129K LOC, 100 source files)
│   ├── src/core/      #   Card data, game state, player, zones, types
│   ├── src/ability/   #   Ability system: effects, conditions, costs, VM
│   ├── src/game/      #   Game setup, display, deck builder, web server
│   ├── src/turn/      #   Turn phases, actions, live phase, triggers
│   ├── src/bot/       #   ISMCTS AI, neural network, determinization
│   ├── src/bin/       #   15 binary targets (harness, bot_arena, trace_game, etc.)
│   ├── tests/         #   416 test files, ~2,500 test functions
│   └── benches/       #   Criterion benchmarks
├── web_ui/            # Vanilla JS web frontend (63 JS files, 11 CSS)
│   ├── index.html     #   Game board
│   ├── card_browser.html
│   ├── deck_converter.html
│   └── tutorial.html
├── cards/             # Card data & compilation pipeline
│   ├── cards.json     #   Master database of 2,526 cards
│   ├── abilities.json #   936 unique ability definitions
│   ├── compile_cards.py / compile_abilities.py / gen_vm_decoder.py
│   └── ability_extraction/  # 15K-line Python parser
├── platforms/         # Console platform glue
│   ├── 3ds/           #   Nintendo 3DS (working)
│   ├── wii/           #   Nintendo Wii (code complete)
│   ├── psp/           #   PlayStation Portable
│   ├── dc/            #   Sega Dreamcast (playable, wasm→C pipeline)
│   ├── ds/            #   Nintendo DS
│   ├── ps1/           #   PlayStation 1
│   ├── gba/           #   Game Boy Advance (via agb)
│   ├── snes/          #   Super Nintendo (in progress)
│   ├── genesis/       #   Mega Drive / Genesis (in progress)
│   ├── jaguar/        #   Atari Jaguar (in progress)
│   ├── cdi/           #   Philips CD-i (native m68k)
│   └── wasm/          #   WebAssembly headless harness
├── training/          # PPO training artifacts & scripts
├── docs/              # GitHub Pages site (frontend synced from web_ui at deploy time)
├── tools/             # Image baking, font generation, deck analysis
└── deployment_scripts/# Hugging Face deploy helpers
```

## Build Configuration

The engine uses Cargo feature flags to toggle platform support and optimizations:

| Feature | Purpose |
|---------|---------|
| `server` | actix-web server + async runtime |
| `3ds` | 3DS target (xorshift64 RNG) |
| `wii` | Wii target (no_std + once_cell) |
| `psp` | PSP target (no_std + alloc) |
| `ds` | DS target (no_std + compact everything) |
| `ps1` | PS1 target (no_std + compact everything, card data streams from CD) |
| `gba` | GBA target (no_std + compact everything, via agb) |
| `dc` | Dreamcast target (no_std) |
| `bytecode_abilities` | Use bytecode VM instead of JSON serde |
| `compact_cards` | Strip display-only fields from Card struct |
| `compact_state` | Bounded log/application Vecs |
| `profiling` | Timer instrumentation for flamegraphs |

Console build scripts are provided (each lives in its platform folder): `platforms\3ds\build_3ds.bat`, `platforms\wii\build_wii.bat`, `platforms\psp\build_psp.bat`, `platforms\dc\build_dc.bat`, `platforms\ps1\build_ps1.bat`, `platforms\gba\build_gba.bat`. Builds output to `platforms\<platform>\output\`.

## Memory Optimization

The engine targets retro consoles with as little as 288 KB (GBA) and 2 MB (PS1). A bytecode VM and struct compaction have reduced runtime RAM from ~3 MB to ~130-170 KB, enabling full game flow on both platforms.

For the current memory/bytecode optimization state, see [docs/memory_optimization_combined.md](docs/memory_optimization_combined.md).

## Testing

- **~2,500 test functions** across **416 test files** in `engine/tests/test_modules/`
- Custom `TestGame` harness with helpers: `add_to_hand()`, `play_to_stage()`, `activate_ability()`, `fill_decks()`, etc.
- Tests use real card data from `cards/cards.json` and `cards/abilities.json`
- Card-specific tests per character (Chika, Yoshiko, Maki, etc.)
- Mechanic tests (baton touch, heart color, position change, live success, blade, score)
- Bytecode deep-compare tests (bytecode vs JSON execution paths)
- QA test suite for regression detection

```bash
cd engine
cargo test --test run_all          # all tests
cargo test --test run_all -- --nocapture  # with stdout
cargo bench                         # Criterion benchmarks
```

## Known Issues

See [engine/ISSUES_FOUND.md](engine/ISSUES_FOUND.md) for the full list.

- **DS ability activation** — Heap exhaustion on ability activation was fixed. Needs build test on actual DS hardware to confirm.
- **Dreamcast port** — SOLVED via the wasm→C pipeline (was: no LLVM backend for SH-4). `platforms/dc/build_dc.bat` builds engine→wasm→C→SH-4 ELF→bootable .cdi. Same pipeline unlocks Saturn/Jaguar later.
- **Exit code 1** — commands return exit code 1 even on success (breaks CI)
- **Wii GX FIFO** — `GX FIFO error 0x69` during system font rendering; currently falls back to printf console CLI
- **Clippy warnings** — unused imports/variables in ~10 test files, missing `Default` impls for `CardDatabase` and `GameModifiers`

## Documentation

| Document | Description |
|----------|-------------|
| [engine/PORTS.md](engine/PORTS.md) | 450-line console port feasibility analysis for 15+ consoles |
| [platforms/gba/output/GBA_PORT_NOTES.md](platforms/gba/output/GBA_PORT_NOTES.md) | GBA port build/toolchain + object-text rendering + VRAM crash fix |
| [engine/PORT_TO_3DS.md](engine/PORT_TO_3DS.md) | 3DS porting plan and progress |
| [docs/memory_optimization_combined.md](docs/memory_optimization_combined.md) | Unified memory & bytecode optimization guide (supersedes the older memory docs) |
| [engine/ISSUES_FOUND.md](engine/ISSUES_FOUND.md) | Known build issues, warnings, and clippy lints |
| [engine/tests/WRITING_TESTS.md](engine/tests/WRITING_TESTS.md) | 530-line guide for writing card tests |
| [cards/ABILITY_DOCUMENTATION.md](cards/ABILITY_DOCUMENTATION.md) | Ability system reference |
| [ai_design/nn_architecture.md](ai_design/nn_architecture.md) | Neural network design document |
| [ai_design/rabuka_bot_design.md](ai_design/rabuka_bot_design.md) | Bot architecture overview |
| [docs/QR_DECK_SHARING.md](docs/QR_DECK_SHARING.md) | QR code deck sharing guide |
| [docs/ABILITY_PIPELINE.md](docs/ABILITY_PIPELINE.md) | Card-text → bytecode pipeline documentation |
| [docs/REFACTOR_BACKLOG.md](docs/REFACTOR_BACKLOG.md) | Verified-remaining refactor items with necessity verdicts |

## License

TBD
