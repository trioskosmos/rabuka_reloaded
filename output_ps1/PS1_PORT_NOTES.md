# Rabuka PS1 Port — Notes

The PlayStation 1 port of Rabuka, rebuilt on the modern Rust PS1 ecosystem.

## Toolchain (installed)

- **Rust `nightly-2025-05-23`** + `rust-src` (`rustup toolchain install nightly-2025-05-23 --component rust-src`).
- **`cargo-psx`** — installed from `research/ps1_rust/psx-sdk-rs/cargo-psx`.
- **`psx-sdk-rs`** (ayrtonm) — the Rust PS1 SDK: built-in `mipsel-sony-psx` rustc
  target, `psx` crate (GPU/framebuffer, gamepad, CD-ROM fs, heap), `psexe.ld`
  linker script producing a raw PS-X EXE via `rust-lld` + `--oformat=binary`.
  No external C toolchain needed.
- **Emulator**: DuckStation
  (`C:\Users\trios\AppData\Local\Programs\DuckStation\duckstation-qt-x64-ReleaseLTCG.exe`)
  with a PS1 BIOS (`SCPH5501.BIN` from the OpenEmu BIOS Pack) in
  `%LOCALAPPDATA%\DuckStation\bios\`.

## Reference material (research folder)

- `research/ps1_rust/psx-sdk-rs` — the Rust PS1 SDK (cloned).
- `research/ps1_rust/ps1-rs-game` — a real PS1 homebrew game in Rust (cloned).
- `research/ps1_homebrew/doukutsupsx` — a full Cave Story port for PS1 (cloned);
  shows the standard PS1 model: code in the EXE (RAM), data as files on the CD
  streamed at runtime (`main.gfx`, `stage00.stg`, ...).
- `research/ps1_homebrew/Tetrade` — another PS1 game (cloned).

## How the port is structured

`platforms/ps1/` is a self-contained no_std crate:

- `Cargo.toml` — deps: `rabuka_engine` (`ps1` feature) + `psx` (git dep).
- `src/lib.rs` — `psx::sys_heap!(256 KB)` global allocator (linked_list_allocator
  over the data-cache scratchpad). Note: the `sys_heap!` `MB` arm is broken
  upstream (missing braces); use `KB`.
- `src/display.rs` — double-buffered 320x240 `Framebuffer` + default-font
  `TextBox` (white text on black), the same rendering path as the SDK examples.
- `src/input.rs` — `Gamepad` wrapper (Up/Down/Left/Right/Cross/Circle/Square/
  Triangle/Start/Select) with just-pressed/held tracking.
- `src/bin/rabuka_ps1.rs` — entry: boots, loads all cards from the engine's
  compact blob, builds the `CardDatabase`, prints status, then loops.

The intended full-game model (like doukutsupsx): **code in the 2MB RAM EXE,
card database streamed from the CD at runtime** (`psx::sys::fs::File::<CDROM>`).

## The two big blockers solved

### 1. serde / no-atomics
The PS1's MIPS R3000A (MIPS-I) has **no atomic instructions**. `alloc::sync::Arc`
(needed by serde's `alloc` feature) doesn't exist on such targets, and **LLVM
crashes** (`Cannot select: MipsISD::Sync`) if you try to claim atomics in a
custom target spec. Fix: made `serde`/`serde_json` **optional** in `rabuka_engine`
behind a new `serde_support` feature (off for `ds`/`ps1`/`psp`/`wii`/`dc`, on for
default/3DS), and gated all serde derives/attrs/uses. The engine's own `Arc` now
falls back to `alloc::rc::Rc` on no-atomic targets via `compat.rs`; the
`ABILITY_DEBUG` AtomicBool falls back to a compile-time-false flag. This also
removed ~2MB of serde machinery from the DS build.

### 2. RAM (2MB)
PS1 has 2MB main RAM; the EXE loads at `0x80010000` with ~1.9MB available.
MIPS code is less dense than ARM Thumb, so the current full-game DS build (~2.4MB
with baked data) would not fit. The measured code is only ~810KB (ARM); the rest
was baked card data. On PS1, card data moves to the CD. Current build: 256KB heap
+ ~1.5MB MIPS code fits.

## Building

```bat
build_ps1.bat
```
or manually:
```
cd platforms/ps1
cargo psx build
```
Output: `output_ps1\rabuka.ps-exe` (a valid PS-X EXE; `file` reports
`Sony Playstation executable`).

## Running

```
duckstation-qt-x64-ReleaseLTCG.exe output_ps1\rabuka.ps-exe
```
DuckStation boots the EXE through the BIOS; the game renders white text on the
PS1 framebuffer.
