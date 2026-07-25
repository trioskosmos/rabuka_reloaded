# Rabuka Reloaded

A certain school idol collectible card game engine, AI, and web UI — built in Rust and ported to 6 platforms.

**2,280 cards · 800 unique abilities · ~1,800 tests · 90K lines of Rust**

[Web UI](#web-ui) · [Console Ports](#target-platforms) · [AI Bot](#features) · [Quick Start](#quick-start) · [Docs](#documentation)

---

## Features

- **Full game rules engine** — all phases (Active, Energy, Draw, Main, Live), all card types, full zone system (hand, stage, deck, waitroom, energy zone, etc.)
- **2,280 real cards** with **800 unique abilities** compiled from Japanese card text via a Python extraction pipeline
- **Custom bytecode VM** — compiles abilities into binary opcodes, reducing on-disk size from 1.4MB to 136KB (90% reduction), enabling console targets with <1MB RAM
- **ISMCTS AI bot** — Information Set Monte Carlo Tree Search handling imperfect information (hidden cards), with a neural network evaluation function
- **PPO training pipeline** — Proximal Policy Optimization to train the neural evaluation function; collects trajectories via self-play
- **Web UI** — full game board, card browser, deck converter, interactive tutorial, multiplayer via SSE + chat, QR code deck sharing, i18n
- **Memory-optimized** — 56 completed refactor tasks; AbilityEffect struct from 1,536B → 152B, Condition from 1,864B → 520B, EffectKind from 1,248B → 544B, `text` fields from String → ArcStr (~48 KB saved), 7 HashMaps/HashSets → SmallVec, recalculate_constants scratch buffers (eliminates 7 HashMap allocs per call), AbilityEffect clone elimination in recalculate_constants (saves 1.5–4.6 KB per recalc), MovementEvent zones from String → ZoneId enum, TriggerEvent/AbilityQueueEntry fields → SmallVec, GameState Vec fields → SmallVec, AbilityResolver fields → SmallVec, deployed_this_turn HashSet → SmallVec, BTreeMap/BTreeSet → HashMap/SmallVec. DS (4 MB) ability activation crash fixed.
- **11 binary targets** — CLI harness, bot demo, training data generation, game tracing, profiling, REPL play, and platform-specific builds

## Target Platforms

| Platform | RAM | Status |
|----------|-----|--------|
| **PC / Web** (x86-64, aarch64) | unlimited | ✅ Works |
| **Nintendo 3DS** (ARM11 @ 268MHz) | 128MB | ✅ Works |
| **Nintendo Wii** (PowerPC 750 @ 729MHz) | 88MB | 🟡 Displays Japanese text — GX FIFO bug |
| **PlayStation Portable** (MIPS R4000 @ 333MHz) | 32MB | 🟡 Displays Japanese text — font rendering |
| **Nintendo DS** (ARM9 @ 67MHz) | 4MB | 🟡 Boots, loads decks, plays — ability activation heap exhaustion fixed (scratch buffers + ZoneId + SmallVec triggers); needs DS build test |
| **Sega Dreamcast** (SH-4 @ 200MHz) | 16MB | 🔴 Blocked — no LLVM backend for SH-4; rustc_codegen_gcc dead-code eliminates all user symbols on SH-4 no_std |

For a full portability analysis covering 15+ consoles (PS1, N64, GameCube, Vita, GBA, Saturn, and more), see [engine/PORTS.md](engine/PORTS.md).

## Quick Start

```bash
# Build & run the engine (CLI)
cd engine
cargo run --release

# Build with web server
cargo run --release --features server -- web-server
# → Open http://127.0.0.1:8080

# Run all ~1,800 tests
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
├── engine/            # Core Rust crate (90K LOC, 83 source files)
│   ├── src/core/      #   Card data, game state, player, zones, types
│   ├── src/ability/   #   Ability system: effects, conditions, costs, VM
│   ├── src/game/      #   Game setup, display, deck builder, web server
│   ├── src/turn/      #   Turn phases, actions, live phase, triggers
│   ├── src/bot/       #   ISMCTS AI, neural network, determinization
│   ├── src/bin/       #   11 binary targets (harness, bot_demo, etc.)
│   ├── tests/         #   318 test files, ~1,800 test functions
│   └── benches/       #   Criterion benchmarks
├── web_ui/            # Vanilla JS web frontend (63 JS files, 12 CSS)
│   ├── index.html     #   Game board
│   ├── card_browser.html
│   ├── deck_converter.html
│   └── tutorial.html
├── cards/             # Card data & compilation pipeline
│   ├── cards.json     #   Master database of 2,280 cards
│   ├── abilities.json #   800 unique ability definitions
│   ├── compile_cards.py / compile_abilities.py / gen_vm_decoder.py
│   └── ability_extraction/  # 11K-line Python parser
├── platforms/         # Console platform glue
│   ├── 3ds/           #   Nintendo 3DS (working)
│   ├── wii/           #   Nintendo Wii (code complete)
│   ├── psp/           #   PlayStation Portable
│   ├── dc/            #   Sega Dreamcast
│   └── ds/            #   Nintendo DS (working)
├── training/          # PPO training artifacts & scripts
├── docs/              # GitHub Pages site
├── tools/             # Image baking, font generation, etc.
└── research/          # Platform research materials
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
| `dc` | Dreamcast target (no_std) |
| `arena_allocator` | Bump arena for temp allocs (~15K → ~150 per trigger) |
| `bytecode_abilities` | Use bytecode VM instead of JSON serde |
| `compact_cards` | Strip display-only fields from Card struct |
| `compact_state` | Bounded log/application Vecs |
| `profiling` | Timer instrumentation for flamegraphs |

Console build scripts are provided: `build_3ds.bat`, `build_wii.bat`, `build_ds.bat`, `build_psp.bat`, `build_dc.bat`.

## Memory Optimization

The engine targets retro consoles with as little as 4 MB RAM (DS) and 16 MB (Dreamcast). A bytecode VM and 55 completed refactor tasks have reduced runtime RAM from ~3 MB to ~130-170 KB. The Dreamcast port is blocked on toolchain issues (no LLVM for SH-4), so the primary optimization target is **Nintendo DS (4 MB)**.

### DS Crash Analysis

The DS boots, loads decks, and plays the game correctly **until an ability is activated**. The crash was caused by allocation storms in the ability resolution hot path. The following fixes have been applied:

1. **`recalculate_constants()` scratch buffers** (`modifiers.rs:23`) — Pre-allocated HashMap/Vec scratch buffers on GameState, reused across calls via `core::mem::take`/swap pattern. Eliminates 7+ HashMap allocations per call (previously ~10-20 KB of allocs per activation).

2. **MovementEvent ZoneId enum** (`types.rs:500`) — Replaced `source_zone: String` and `dest_zone: String` with compact `ZoneId` enum (1 byte each). Eliminates 2 heap allocations per movement event (~46 bytes saved per event, ~10-20 events per turn).

3. **TriggerEvent SmallVec conversion** — `moved_cards` and `appeared_cards` changed from `Vec` to `SmallVec<[i16; 4]>` / `SmallVec<[(i16, String); 4]>`. Inline stack storage eliminates heap allocation for typical 0–4 card batches.

4. **condition_cache HashMap → SmallVec** — `AbilityQueueEntry.condition_cache` changed from `HashMap<String, bool>` to `SmallVec<[(String, bool); 2]>`. Eliminates 64+ B per queue entry overhead.

5. **Remaining hot-path allocations** — `format!()` calls in the hot path are bounded by SmallVec capacity (8 items) and are not significant allocators. The `.collect::<Vec<_>>()` and `.to_vec()` calls in condition evaluation create temporary Vecs but are short-lived.

To test if the DS crash is resolved, build with `build_ds.bat` and activate an ability on the DS.

### Current RAM Budget (console features enabled)

| Component | Current Size | Target | Status |
|-----------|-------------|--------|--------|
| Card data (120 deck cards) | ~14 KB | 1.4 KB | `compact_cards` gates display fields; packed binary deferred |
| Ability structs in RAM | ~120 KB | 0 KB | Decode-on-demand via bytecode VM, no cache |
| EffectKind + AbilityEffect | ~20 KB | 0 KB | Bytecode interpreter path planned |
| GameState | ~10-45 KB | 2.5 KB | `compact_state` caps logs at 500; 7 HashMaps/HashSets → SmallVec |
| String/ArcStr data | ~30 KB | 0 KB | u16 indices into compile-time table deferred |
| Code (engine + VM) | ~600-900 KB | ~600 KB | Compiler-dependent |
| Per-trigger heap (peak) | ~3 KB | ~3 KB | Arena v0 done, v1 (cursor reset) deferred |
| **Total runtime** | **~130-180 KB** | **~150 KB** | Close |

### Completed Optimizations

| Optimization | Size Before | Size After | Saving |
|-------------|------------|-----------|--------|
| AbilityEffect flat struct | 1,536 B | 152 B | 90% |
| EffectKind tagged union | 1,248 B | 544 B | 56% |
| Condition tagged union | 1,864 B | 520 B | 72% |
| `text` fields (AbilityEffect, Choice, etc.) | String (24 B + heap) | ArcStr (16 B, shared) | ~48 KB |
| JSON-based ability loading | ~1.4 MB | ~136 KB (bytecode) | 90% |
| GameState HashMap/HashSet → SmallVec (7 fields) | ~40-80 B overhead per empty map | 0-24 B inline | ~20-40 KB |
| Bump arena allocator | ~15,000 per-trigger allocs | ~150 | 99% |
| recalculate_constants scratch buffers | 7 HashMap allocs per call | 0 (reused across calls) | ~10-20 KB per activation |
| MovementEvent source/dest zones | String (24 B + heap) × 2 | ZoneId enum (1 B) | ~46 B per event × ~10-20 events/turn |
| TriggerEvent moved_cards/appeared_cards | Vec (24 B + heap) × 2 | SmallVec (inline, 0 allocs) | ~48 B per event × ~10-20 events/turn |
| AbilityQueueEntry.condition_cache | HashMap (64+ B per entry) | SmallVec<[(String,bool); 2]> (inline) | ~64 B per queue entry |
| recently_moved_cards | Option&lt;Vec&gt; (24 B + heap) | Option&lt;SmallVec&lt;[i16; 4]&gt;&gt; (inline) | ~500 B + 17 allocs/activation |
| batch_movements/turn_area_movements/turn_movements | Vec × 3 (72 B + 3 heaps) | SmallVec (inline) | ~700 B per activation |
| position_change_events | Vec (24 B + heap) | SmallVec&lt;[PositionChangeEvent; 2]&gt; | ~80 B per batch |
| AbilityResolver small Vec fields × 5 | Vec × 5 (120 B + 5 heaps) | SmallVec (inline) | ~180 B per resolver |
| deployed_this_turn | HashSet (24 B + heap) | SmallVec&lt;[i16; 4]&gt; | ~400 B per player |
| BTreeMap/BTreeSet → HashMap/SmallVec | BTree nodes (~40 B each) | HashMap/SmallVec | ~150 B per call |

### Remaining Work

| Optimization | Est. Saving | Effort | File | Priority |
|-------------|------------|--------|------|----------|
| collect_constant_stage_effects() AbilityEffect clone avoidance | 1.5–4.6 KB | High | `abilities.rs:138` | High |
| effect.clone() in execute_effect (23 sites) | 2–5 KB | High | `resolver.rs` | High |
| Missing scratch buffers (exp_prohibition, exp_global_need_heart, jyouji_statuses) | ~600 B per call | Low | `modifiers.rs` | Low |
| Vec collecting in conditions (.to_vec/.collect) | ~50-100 B per condition | Low | `condition/card.rs` | Medium |
| Remove 4 redundant flat fields from AbilityEffect | ~360 KB | Medium | `card.rs:2118-2124` | Medium |
| Store gained abilities as AbilityRef (2 B) instead of Ability (~600 B) | ~50-100 KB | Medium | `game_state/mod.rs:77` | Medium |
| DynamicCount/QuotedText String fields → enums | ~12 KB | Low | `card.rs:3645-3659` | Low |
| PositionInfo/DistinctInfo String variants → enums | ~12 KB | Low | `card.rs:3625-3666` | Low |
| Arena v1 (cursor reset) — eliminates remaining per-trigger allocs | ~3 KB per trigger | Medium | `arena.rs` | Low |
| Unblock CondBox (Condition pool, 64 slots) | ~33 KB | Medium | `pool.rs:174` | Low |

## Testing

- **~1,800 test functions** across **318 test files** in `engine/tests/test_modules/`
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

- **DS ability activation** — Heap exhaustion on ability activation was fixed (scratch buffers + ZoneId). Needs build test on actual DS hardware to confirm.
- **Dreamcast toolchain** — No LLVM backend for SH-4. `rustc_codegen_gcc` dead-code eliminates all user symbols on SH-4 no_std targets. Port blocked until upstream Rust SH-4 support or GCC codegen matures.
- **Exit code 1** — commands return exit code 1 even on success (breaks CI)
- **Wii GX FIFO** — `GX FIFO error 0x69` during system font rendering; currently falls back to printf console CLI
- **Clippy warnings** — unused imports/variables in ~10 test files, missing `Default` impls for `CardDatabase` and `GameModifiers`

## Documentation

| Document | Description |
|----------|-------------|
| [engine/PORTS.md](engine/PORTS.md) | 450-line console port feasibility analysis for 15+ consoles |
| [engine/PORT_TO_3DS.md](engine/PORT_TO_3DS.md) | 3DS porting plan and progress |
| [engine/MEMORY_REFACTOR.md](engine/MEMORY_REFACTOR.md) | 1,000-line history of 40+ memory optimization tasks |
| [engine/ISSUES_FOUND.md](engine/ISSUES_FOUND.md) | Known build issues, warnings, and clippy lints |
| [engine/tests/WRITING_TESTS.md](engine/tests/WRITING_TESTS.md) | 530-line guide for writing card tests |
| [cards/ABILITY_DOCUMENTATION.md](cards/ABILITY_DOCUMENTATION.md) | Ability system reference |
| [ai_design/nn_architecture.md](ai_design/nn_architecture.md) | Neural network design document |
| [ai_design/rabuka_bot_design.md](ai_design/rabuka_bot_design.md) | Bot architecture overview |
| [docs/QR_DECK_DECK_SHARING.md](docs/QR_DECK_DECK_SHARING.md) | QR code deck sharing guide |

## License

TBD
