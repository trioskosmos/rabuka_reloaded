# Rabuka DS Port — Rebuild Notes

This document records the complete history of the Nintendo DS port, the problems
with the original implementation, the debugging that led to its deletion, and the
working toolchain/approach that replaced it.

---

## 1. The original port (deleted)

The first DS port lived in `platforms/ds/` and used:

- **devkitPro devkitARM** (`C:/devkitPro/devkitARM`) + **calico** linker specs
  (`ds9-large.specs`).
- A handwritten C shim (`nds_shim.c`) wrapping **libnds** (video, console, keys).
- A **custom Rust global allocator** (`DsAllocator`) backed by newlib `malloc`
  via `ds_malloc/free/realloc`, with a custom `_sbrk` that grew a `_brk` pointer
  capped at `0x02400000`.
- Engine build with the `ds` feature (`no_std` + `compact_cards` +
  `compact_card_data` + `compact_state`).
- `serde_json` runtime parsing of `decks.json`.

It compiled and booted far enough to render menus, but **always crashed before
reaching the Main phase** of a game.

### Symptoms observed

1. **Hardware fault** (`FLAG=0x02` data abort) with `PC=0x020d61a8`,
   `R0=0x0000001c`, deep stack. Disassembly showed the fault inside **newlib
   malloc's free-list walk** dereferencing a corrupt node (`ldr r1,[r4]` where
   `r4≈0x1c`) — classic heap free-list corruption.
2. A redzone-canary allocator (added to catch the corrupting write) reported
   `HEAP OVRUN sz=0x12` (18 bytes) during boot.
3. `ds_heap_used()` always read **0** — the custom `_sbrk`/`_brk` was never
   actually driving the malloc in use. The heap setup was not what the code
   believed, i.e. the C heap was unreliable.

### Debugging detours

- A host-side repro (`hf_temp/repro_ds/`) proved the **engine load path
  completes cleanly** under a standard allocator — the game code itself was not
  the culprit.
- The host canary allocator used for that test was itself buggy (misaligned
  canary writes, `format!`-inside-allocator recursion), producing false alarms.
- Conclusion: the instability was in the **DS memory layer** (a broken
  C-heap/malloc interaction), not the game logic.

## 2. Decision: delete and rebuild

The DS port was deleted (`platforms/ds`, `output_ds`, `build_ds.bat`) and rebuilt
from scratch on a **modern, supported toolchain** rather than patching the old
one.

### Research material cloned under `research/ds_rust/`

- **`nds-rs`** (BlueTheDuck) — Rust safe wrapper over libnds for BlocksDS.
  Primary reference for the build recipe (target JSON, `build-std`, linker
  specs).
- **`libnds-rs`** (oxcabe), **`libnds-rust-bindings`** (STBoyden) — additional
  bindings for reference.
- Existing C homebrew under `research/ds_homebrew/` (devkitPro libnds).

## 3. Toolchain: Wonderful + BlocksDS

Installed manually (this machine's MSYS2 is devkitPro's Cygwin-based fork, so the
Windows bootstrap wasn't drivable headless):

- **Wonderful toolchain** at `/opt/wonderful`
  (`wf-pacman` bootstrap → `wf-tools`, `toolchain-gcc-arm-none-eabi-*`).
- **BlocksDS SDK** cloned to `/opt/wonderful/thirdparty/blocksds`, built and
  installed to `/opt/wonderful/thirdparty/blocksds/core`:
  - `libs/libnds` (libnds9.a, headers), `libs/maxmod`
  - `sys/crts` (ds_arm9/ds_arm7 specs + linker scripts)
  - `sys/arm7` ARM7 cores (`arm7_minimal.elf` etc.), installed as
    `sys/default_arm7/arm7.elf`
  - `tools/bin2c`, `tools/grit`
  - Notes: `tools/dlditool` had an `addr_t` clash with MSYS2 headers (renamed to
    `wd_addr_t`); `ndstool` needed `libiconv-devel`; the DS's own `ndstool` is
    packed with devkitPro's `ndstool.exe`.
- **Key difference vs old port**: BlocksDS uses **picolibc**, a modern newlib
  fork with a *correct* `_sbrk`/heap setup. The Rust global allocator simply
  wraps `malloc/free/realloc` (the same pattern `nds-rs` uses). No custom
  allocator, no canary scheme, no fragile C-heap workarounds.

## 4. The new `platforms/ds`

Minimal, self-contained crate (no bindgen, no libc crate, no serde at runtime):

```
platforms/ds/
  Cargo.toml                 # deps: rabuka_engine (ds feature), log
  build.rs                   # compiles nds_shim.c, links shim + libnds9
  .cargo/config.toml         # target json, nightly build-std, linker
  .cargo/armv5te-nintendo-ds-newlibeabi.json
  src/nds_shim.c             # tiny C shim: init, printf, clear, keys, vblank,
                             #   gettimeofday stub (no real DS clock)
  src/bin/rabuka_ds.rs       # no_std/no_main, allocator, panic handler, main
```

### Target JSON highlights

- `cpu: arm946e-s`, `+soft-float,+strict-align,+atomics-32`
- `max-atomic-width: 32`, `min-atomic-width: 8`, `atomic-cas: true` — required so
  the nightly rustc reports `target_has_atomic="ptr"` (enables `alloc::sync::Arc`,
  needed by the engine's `serde` dependency).
- `late-link-args`: `-lc -lgcc -specs=<ds_arm9.specs>` (Windows path form).
- Build with `cargo +nightly build --release -Zjson-target-spec` (build-std via
  `.cargo/config.toml`).

### Why no crashes now

- **Stable heap**: BlocksDS picolibc `malloc` is the global allocator.
- **No runtime JSON**: decks are baked into the ROM (the `bake` step), so the
  boot path does no parsing.
- **Proper crt0/linker scripts** from BlocksDS specs.

## 5. Status

- `cargo +nightly build --release -Zjson-target-spec` succeeds.
- `platforms/ds/output/rabuka.nds` packs the ARM9 ELF + ARM7 core; boots in
  **melonDS 1.1** (process stays alive, no crash) — the current build is a
  "hello world + frame counter" smoke test.
- Next: wire the real game — baked deck loading (`card_binary` blob),
  `platform_ui::PlatformUi` (display/input), and the main loop matching the
  3DS/PSP ports (with `reset_loop_detection` per action).

## 6. Build & run (this machine)

```bash
export PATH="/opt/wonderful/bin:/opt/wonderful/toolchain/gcc-arm-none-eabi/bin:$PATH"
export BLOCKSDS=/opt/wonderful/thirdparty/blocksds/core
export WONDERFUL_TOOLCHAIN=/opt/wonderful
cd platforms/ds
cargo +nightly build --release -Zjson-target-spec
# pack ROM:
"C:/devkitPro/tools/bin/ndstool.exe" -c ../ds/output/rabuka.nds \
  -9 C:/rust_targets/armv5te-nintendo-ds-newlibeabi/release/rabuka_ds.elf \
  -7 /opt/wonderful/thirdparty/blocksds/core/sys/default_arm7/arm7.elf
```
