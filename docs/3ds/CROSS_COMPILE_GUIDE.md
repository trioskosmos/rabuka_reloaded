# Cross-compiling `rabuka_engine` for Nintendo 3DS — Guide & Checklist

This guide documents a practical path to cross-compile the `rabuka_engine` core for the Nintendo 3DS homebrew environment, the major portability blockers, and recommended mitigations. It assumes you validated the engine core locally (we added an interactive harness at `src/bin/harness.rs`).

SUMMARY
- Goal: produce a native 3DS binary that runs the engine core (single-player/hotseat), without web server or networking.
- Strategy: gate/remove non-portable crates (async, networking), target a 3DS-compatible std shim (eg. `ctr-std`), use devkitPro toolchain + `cargo-3ds` or a rust3ds template Docker image.

IMPORTANT: This is non-trivial and will require manual testing on real or emulated 3DS hardware (Citra). The web UI and multiplayer features are out-of-scope for the initial port.

1) High-level options
- Local toolchain (recommended for iteration): install devkitPro/devkitARM on Linux or Windows WSL, install `cargo-3ds` and follow `rust3ds` templates.
- Containerized build: use an existing Docker image (`nandolawson/rust-3ds` or `rust3ds` templates) to avoid host toolchain setup.

2) Preliminary repository changes (we already did some)
- Gate server deps behind `server` feature (done in `Cargo.toml`). Build core with `--no-default-features`.
- Provide a desktop harness binary (`src/bin/harness.rs`) to exercise the core separately (added).

3) Tooling & environment (recommended)
- Install devkitPro (https://devkitpro.org/) and its pacman packages for `devkitARM` and `libctru`.
- Install `cargo-3ds` (rust3ds toolchain) or use `rustup` + a `cargo` wrapper that integrates devkitARM. See `rust3ds/cargo-3ds` repository for details.
- Optional: use a prepared Docker image to avoid host installation.

4) Build steps (example using `cargo-3ds` / rust3ds templates)
- From the `engine/` folder, first ensure the code builds locally without server features:
```powershell
cargo build --bin harness --no-default-features
```

- To attempt a 3DS build (high-level example — adjust to your cargo-3ds installation):
```bash
# install cargo-3ds per rust3ds instructions
cargo 3ds build --bin harness --release
# or, with a Docker image: docker run --rm -v $(pwd):/work nandolawson/rust-3ds bash -lc "cd /work/engine && cargo build --bin harness --release"
```

5) Expected blockers and mitigation checklist
- actix-web / tokio: these are not usable on 3DS. We already gated them behind the `server` feature — build with `--no-default-features` and avoid web server code paths.
- Networking and async: remove or stub all uses of `tokio`, `async` IO, `std::net`, `actix` routes, SSE/WebSocket code.
- OS syscalls and threading: 3DS environment differs — prefer single-threaded synchronous operation for the initial port. Replace `tokio::spawn` usage with no-op or serial calls when `server` feature is disabled.
- getrandom / `uuid` v4: RNG sources may not be available; use `getrandom` with a 3DS-targeted backend or provide a deterministic fallback for non-security uses (e.g., use seeded `rand_chacha` for local builds).
- `local-ip-address` and other network discovery crates: remove or gate behind `server` feature.
- `actix-files` / file serving: not relevant for native port; use SD card file access via `libctru` or the `ctr-std` shim.
- `std` availability: choose a std shim (`ctr-std`) that provides enough of `std` (file I/O, panic, collections). Alternatively, adapt the code to compile with the `ctr-std` fork which exposes a subset of `std` for 3DS.
- Graphics / UI: web UI won't run; plan a minimal framebuffer text UI using `libctru` or use Citra console for debug I/O.
- Memory limits: reduce large in-memory caches or tune SmallVec sizes to avoid stack/heap pressure.

6) Recommended incremental port process
- A. Create a 3DS-specific Cargo profile/feature combination (e.g., `--features=3ds,portable-core`) that:
  - disables `server` and other desktop-only features
  - reduces default SmallVec inline sizes where necessary

- B. Make minimal shims and stubs in code guarded with `cfg(feature = "3ds")` or `cfg(not(feature = "server"))`:
  - Provide a simple `notify_room_clients()` stub
  - Replace `tokio::spawn` with a synchronous call or no-op under 3DS build

- C. Attempt to compile the harness via `cargo-3ds`. Fix compile errors iteratively (most will be missing crates or missing syscalls).

- D. Replace or adapt platform-specific functions (e.g., file paths) to use an SD-root path (`/3ds/rabuka/` or similar).

- E. Test on Citra (emulator) first, then on real hardware.

7) Debugging & testing tips
- Use verbose cargo output: `cargo 3ds build -v` and capture linker failures — many errors will be due to missing syscall bindings or libc incompatibilities.
- Keep iterative commits small and gate each platform-specific change behind a feature flag.

8) Minimal runtime expectations
- The first runnable 3DS binary will likely be a CLI-like harness that prints the game state to stdout (or framebuffer) and accepts button presses for actions. Multiplayer and the web UI are long-term efforts.

9) Resources
- devkitPro/libctru: https://github.com/devkitPro/libctru
- rust3ds organizations and templates (cargo-3ds, rust3ds-template): https://github.com/rust3ds
- Docker images and community notes: search `rust 3ds cargo-3ds` on GitHub for useful starting points.

If you'd like, I can now:
- Attempt a local cross-build using a known Docker image and report exact compile failures (I can create a Dockerfile and try a build), or
- Produce a smaller checklist of specific code locations to patch (I can grep for `tokio::`, `actix`, `std::net`, `spawn`, and generate a patch set that gates or stubs them), or
- Both.
