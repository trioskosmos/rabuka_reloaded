# Rabuka 3DS — Proof of Concept

A standalone Nintendo 3DS port of the Rabuka card game engine.

## Structure

```
engine_3ds/
├── Cargo.toml           # Minimal deps (depends on ../engine without server)
├── .cargo/config.toml   # 3DS cross-compilation target config
├── src/
│   └── bin/
│       └── rabuka_3ds.rs  # 3DS entry point (auto-play proof of concept)
```

## How it works

`engine_3ds` depends on `rabuka_engine` (from `../engine`) with `default-features = false`.
This excludes all web server dependencies (actix, tokio, etc.) and only pulls in the core
game logic: card database, deck builder, game state, turn engine, ability resolver.

## Building & Running

### Desktop (development/testing)
```powershell
cd engine_3ds
cargo run --bin rabuka_3ds
```

This runs the auto-play binary that exercises all game phases end-to-end.

### 3DS (cross-compilation)
Requires devkitPro + cargo-3ds toolchain:
```bash
# Install cargo-3ds (see https://github.com/rust3ds/cargo-3ds)
cargo 3ds build --bin rabuka_3ds --release
```

The resulting `.3dsx` or `.cia` file can be run on Citra emulator or real hardware.

## 3DS Feature Flag

When built with `--features 3ds`, the binary uses `ctru-rs` for:
- Top screen: framebuffer rendering of game state
- Bottom screen: button input handling
- SD card: loading `cards.json` from `/3ds/rabuka/`

On desktop (without `--features 3ds`), the binary uses stdout for rendering and
auto-plays through the game with a hardcoded action strategy.

## Notes

- Only single-player/hot-seat mode is supported on 3DS (no networking).
- The web server, multiplayer, and SSE event streaming are excluded.
- The auto-play proof of concept demonstrates that the engine core functions
  correctly without any web/async dependencies.
