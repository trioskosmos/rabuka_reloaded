 Rabuka Console Port Analysis

## How Far Back Can We Go?

The engine is a card game. No physics, no 3D, no real-time constraints.
Portability is limited by three hard gates:

1. **CPU support** (Rust needs either LLVM or GCC codegen for the CPU)
2. **Enough RAM** (varies wildly by engine architecture — see below)
3. **Someone writes ~200 lines of platform glue** (display, input, allocator)

## The Real RAM Picture

The current engine uses a **JSON-based ability interpreter**: card abilities are
stored as serde-deserialized JSON structs (~4.5MB raw), then inflated further at
runtime by Vec (24B), String (24B), Arc<str> (16B + alloc), and Option<T>
overhead. This is what drives the ~4MB minimum.

A **bytecode-compiled** approach shrinks that to:
- Card stats: ~27KB (packed binary structs, ~12B per card × 2280)
- Ability bytecode: ~11KB (variable-length opcodes, ~14B avg per unique ability × 800)
- Ability lookup table: ~3KB (index per card × 2280)
- **Total: ~40KB** — a 100× reduction from the JSON-inflated ~4MB

The real minimum RAM for the engine depends entirely on how the card data
is stored. With JSON: ~4MB. With bytecode: **~1MB** (40KB data + 600KB code +
300KB state/stack). With bytecode + packed structs + no decoded effect
structs: **~150KB** (12B-per-card structs + bytecode interpreter + 2.5KB game state).

Everything below is sorted oldest-first within each tier.

---

## The Gates

### Gate 1: CPUs with LLVM Backends

| CPU | Used In | LLVM? |
|---|---|---|
| ARMv4t (ARM7TDMI) | GBA, DS (ARM7) | Yes |
| ARMv5te | DS (ARM9) | Yes |
| ARMv6k | 3DS | Yes |
| ARMv7-A | Vita, Switch (ergo) | Yes |
| MIPS R3000 (MIPS-I) | **PS1** | Yes (experimental) |
| MIPS R4000 (MIPS32) | **PSP**, N64 | Yes |
| PowerPC 750 | **GameCube**, **Wii** | Yes |
| m68k | Genesis, Neo Geo, Jaguar | Yes |
| SH-4 | **Dreamcast** | **No** |
| SH-2 | **Saturn** | **No** |
| 65816 | SNES | **Fork** (`llvm-mos`) |
| 6502 | NES, C64, Lynx | **Fork** (`llvm-mos`) |
| Z80 | MSX, Master System, GG, Spectrum | **Fork** (`llvm-z80` / `rust-gb`) |

\* The "No" entries are no longer strictly true — community LLVM **forks** now
exist for 65816/6502 (`llvm-mos`) and Z80 (`llvm-z80` / cranelift). They are
not mainline LLVM (no official rustc tier), and the compile-time cost is
absurd (see the 8-bit tier section below), so the RAM verdicts below are
unchanged. Detail in the [research section](#tier-7-the-research-below-8-bit-nes-snes-master-system).
| HuC6280 | TurboGrafx-16 | **No** |
| TLCS-900 | Neo Geo Pocket | **No** |
| V30MZ (x86-16) | WonderSwan | **No** |

Everything **without** an LLVM backend is immediately out — Rust can't
generate code for it. Dreamcast (SH-4), Saturn (SH-2), SNES (65816),
NES (6502), and all Z80-based machines are dead on arrival.

### Gate 2: RAM

With the **bytecode-compiled** approach, the engine needs roughly:
- **Card data**: ~40KB (packed binary — no serde, no heap allocations)
- **Code**: ~600KB-1MB (ability VM `vm.rs` is ~1,443 LOC instead of 179k LOC of serde structs)
- **Game state + heap + stack**: ~300KB
- **Realistic minimum** (bytecode): **~1MB**
- **Realistic minimum** (current JSON interpreter): **~4MB**

Consoles that fail either way:
- **GBA** (288KB) — not even close
- **Genesis / Neo Geo** (64KB) — laughable

Consoles that become viable with bytecode:
- **PS1** (2MB) — from "hard block" to **comfortable**
- **DS** (4MB) — from "borderline" to **comfortable**
- **N64** (4MB base / 8MB expansion) — **comfortable even without expansion**

Consoles that remain borderline:
- **Saturn** (2MB main + 1.5MB video) — tight but possible if video RAM is shared

---

## Tier 1: Proven Working

### Nintendo 3DS (2011)
- CPU: ARM11 MPCore @ 268MHz
- RAM: 128MB
- SDK: devkitARM (`armv6k-nintendo-3ds`), `ctru-rs`, `cargo-3ds`
- Status: **Already works.** See `ports/3ds/`.
- Engine changes: Zilch. Std works via newlib.

---

## Tier 2: Easy (std supported, official Rust target, enough RAM)

### PlayStation Vita (2011)
- CPU: 4x ARM Cortex-A9 @ 444MHz
- RAM: **512MB** (more than 3DS!)
- Target: `armv7-sony-vita-newlibeabihf` (official, Tier 3)
- Std: **Yes** (newlib via VITASDK, like 3DS has devkitARM)
- Tooling: `cargo-vita`, vitasdk, Docker images
- Engine changes: **none**
- Difficulty: Trivial — same pattern as 3DS crate

### PlayStation Portable (2004)
- CPU: MIPS R4000 @ 333MHz
- RAM: 32MB (adequate for card game)
- Target: `mipsel-sony-psp` (official, Tier 3)
- Std: **No** — no_std + alloc only (like PS1)
- Tooling: `cargo-psp`, `psp-rs` (814 stars)
- Engine changes: **need no_std migration** (~500 line changes)
- Note: Same MIPS-family CPU as PS1, but 16x the RAM

---

## Tier 3: Doable (need custom target.json, but std works via devkitPro)

### Nintendo Wii (2006)
- CPU: PowerPC 750CL @ 729MHz
- RAM: 64MB + 24MB = **88MB**
- Target: `powerpc-unknown-none-elf` + custom target.json
- Std: **Yes** (devkitPPC provides newlib, same pattern as 3DS)
- Tooling: `ogc-rs` bindings for libogc, devkitPPC
- Engine changes: **none**
- Difficulty: Need to create a target spec JSON (one-file job), port to
  the ~200 lines of Wii-specific display/input code
- **Most impressive older console with std support**

### Nintendo GameCube (2001)
- CPU: PowerPC 750CXe @ 485MHz
- RAM: 24MB + 16MB = **40MB**
- Same toolchain as Wii, slightly less RAM
- Engine changes: **none**
- Difficulty: Same as Wii — shares devkitPPC

---

## Tier 4: No_std Required

### PlayStation 1 (1994)
- CPU: MIPS R3000A @ 33MHz
- RAM: **2MB** (main) + 1MB (VRAM)
- Target: `mipsel-sony-psx` (official, Tier 3)
- Std: **No** — no_std + alloc
- SDK: `psx-sdk-rs` (experimental, single developer)
- Engine changes: **need bytecode VM migration + no_std**
- **Viable with bytecode** (40KB card data + 600KB code + 300KB state
  = ~940KB, well within 2MB). Current JSON interpreter overflows it.

### Nintendo 64 (1996)
- CPU: MIPS VR4300 @ 93MHz
- RAM: 4MB (8MB with Expansion Pak)
- Target: None (would need custom `mipsel-n64-none-eabi` JSON)
- Std: **No**
- Engine changes: **need bytecode VM migration + no_std**
- **Comfortable even at 4MB** with bytecode approach

### Nintendo DS (2004)
- CPU: ARM9 @ 67MHz + ARM7 @ 33MHz
- RAM: **4MB**
- Target: None (`armv5te-none-eabi` + custom JSON)
- Std: **No** — no_std + alloc
- SDK: `nds-rs` (very early, 10 stars)
- Engine changes: **need bytecode VM migration + no_std**
- **Comfortable with bytecode** — 4MB is plenty for 1MB engine

---

## Tier 5: CPU Supported but RAM Too Small

### Sega Genesis / Mega Drive (1988)
- CPU: 68000 @ 7.6MHz
- RAM: **64KB**
- Target: `m68k-unknown-none-elf` (LLVM m68k is experimental)
- Verdict: **Impossible** — 64KB is 1/60th of what's needed
  (REVISED Aug 2026 — the RAM math was wrong and the CPU gate is real but
  different than stated. See "Genesis native m68k attempt" below.)

### Neo Geo AES (1990)
- CPU: 68000 + Z80
- RAM: 64KB main + 64KB video
- Verdict: Same as Genesis

### Atari Jaguar (1993)
- CPU: 68000 + custom Tom/Jerry RISC
- RAM: **2MB**
- Verdict: 2MB is too small + custom RISC has no LLVM backend

### Game Boy Advance (2001)
- CPU: ARM7TDMI @ 16MHz
- RAM: **288KB** (32KB internal + 256KB external)
- Target: `armv4t-none-eabi` / `thumbv4t-none-eabi`
- SDK: `agb-rs` (most mature Rust console SDK — 474 stars)
- Verdict: **Possible with bytecode interpreter** — 150KB fits in 288KB.
  Requires packed card structs (12B each) and direct bytecode evaluation
  (no decoded Ability/AbilityEffect structs in RAM).

---

## Tier 6: CPU Not Supported by LLVM

| Console | CPU | Year | LLVM? | GCC? | RAM | Verdict |
|---|---|---|---|---|---|---|
| Sega Dreamcast | SH-4 | 1998 | No | **Yes** (SH-4 in GCC) | 16MB | **Unlocked by bytecode + rustc_codegen_gcc** |
| Sega Saturn | SH-2 | 1994 | No | **Yes** (SH-2 in GCC) | 2MB+1.5MB | RAM-tight, but CPU is reachable via same path |
| SNES | 65816 | 1990 | No | Yes (cc65) | 128KB | **CPU + RAM both kill it** |
| NES | 6502 | 1983 | No | Yes (cc65) | 2KB | Not a real computer |
| Master System / Game Gear | Z80 | 1985 | No | Yes (SDCC/Z88DK) | 8KB | Same |
| TurboGrafx-16 | HuC6280 | 1987 | No | Partial | 8KB | Same |
| WonderSwan | V30MZ | 1999 | No | Partial | 16KB | Same |
| Neo Geo Pocket | TLCS-900 | 1999 | No | Partial | 12KB | Same |
| Commodore 64 | 6510 | 1982 | No | Yes (cc65) | 64KB | Too small |
| MSX | Z80 | 1983 | No | Yes (SDCC) | 8-64KB | Too small |

### Actually reachable via bytecode + rustc_codegen_gcc

Two consoles in Tier 6 become feasible with the right approach:

**Dreamcast (16MB RAM, SH-4 CPU):**
- `rustc_codegen_gcc` (GCC backend for rustc) replaces LLVM — SH-4 is a first-class GCC target
- Proven: Falco Girgis (KallistiOS lead) demoed Rust 3D on Dreamcast via `rustc_codegen_gcc` in 2023
- 16MB RAM is comfortable for the bytecode-compiled engine (~40KB data + ~600KB code)
- Requires: the bytecode VM (replaces JSON interpreter) + ~200 lines of KallistiOS glue

**Saturn (2MB + 1.5MB VRAM, SH-2 CPU):**
- Same `rustc_codegen_gcc` approach (SH-2 is also a GCC target)
- 2MB main RAM is tight but feasible with bytecode (40KB data + 600KB code + 300KB state = ~940KB)
- Video RAM (1.5MB) can store card art — Saturn has no texture cache issue
- Harder than Dreamcast: SH-2 is dual-core (master + slave), Saturn's SDK is less mature

### Still impossible

Everything with <150KB RAM is dead regardless of language — even the
bytecode interpreter needs ~150KB minimum (40KB bytecode blob + 27KB card
data + 80KB code/stack). This rules out all 8/16-bit consoles except GBA.

The 8/16-bit consoles that remain (Genesis, Neo Geo, Jaguar) have m68k CPUs that
LLVM actually supports, but their 64KB-2MB RAM is too small even for the
bytecode-compiled engine. 64KB is a microcontroller, not a card game platform.

---

## Is It Really This Easy?

Yes. Here's why.

### Why porting a card game is easy

A card game engine is a pure state machine:
- Input → apply rules → update state → output
- No real-time loop, no physics ticks, no frame deadlines
- The "display" is just text — you can render it to anything

The platform-specific parts are a tiny surface area:

```
            ┌──────────────────────┐
            │   rabuka_engine      │  ← 100% portable, same code everywhere
            │  (game logic, AI,    │
            │   cards, rules)      │
            └────────┬─────────────┘
                     │
    ┌────────────────┼────────────────┐
    │                │                │
    ▼                ▼                ▼
 ports/3ds     engine_ps1       engine_pc_cli
 (display,      (display,        (display,
  input,         input,           input,
  audio,         allocator,       RNG)
  RNG)           panic handler)
```

Each platform binary is ~200 lines of glue plus the platform SDK.

### Why Rust specifically enables this

1. **`no_std` + `alloc`** — you get Vec, String, Box, Rc, HashMap
   without an OS. This covers 95% of what the engine uses.

2. **`cfg` gates** — `#[cfg(feature = "std")]` on file I/O,
   `#[cfg(target_os = "psx")]` on PS1-specific timing. The same
   source file can compile for PC and PS1.

3. **LLVM's architecture coverage** — ARM, MIPS, PowerPC, x86, RISC-V
   are all first-class. If the console has one of these, Rust can
   target it. For others, **`rustc_codegen_gcc`** swaps LLVM for GCC's
   libgccjit, unlocking SH-4 (Dreamcast), SH-2 (Saturn), and m68k
   (Genesis) without changing a line of Rust source.

4. **`include_bytes!`** — embed the compiled bytecode (~7KB for all
   800 abilities) directly into the binary at compile time. No filesystem,
   no loader, no runtime initialization. The data is in the .text section.

5. **The engine doesn't use `unsafe` much** — no inline assembly,
   no platform-specific intrinsics in core logic. The `rng.rs`
   abstraction proves this: swap `thread_rng()` for `xorshift64`
   with a single `#[cfg]`.

### But wasn't this always the case?

Yes. The game logic always was platform-independent — it ran on a PC
during development and was cross-compiled for the target. What changed:

| Era | Development | Target | Porting effort |
|---|---|---|---|
| 1990s | C/asm on SGI/PC | SNES, Genesis | Rewrite in asm per platform |
| 2000s | C on PC | PS2, GameCube, PSP | C SDK per platform, many bugs |
| 2010s | C++ on PC | Vita, 3DS, Switch | C++ SDK, some shared code |
| **Now** | **Rust on PC** | **Any LLVM target** | **Swap imports, add cfg gates** |

The difference isn't that ports are new — it's that the friction is
nearly zero. In the 90s, porting meant rewriting in assembly. In the
2000s, it meant writing C wrappers around proprietary SDKs that had
different ABIs, different threading models, different audio APIs.

With Rust, there are no "two languages" — the game logic *is* the
same language as the platform glue. The compiler handles all the
ABI details. The SDK is a `cargo add` away.

### Realistic port effort

Two paths depending on RAM budget:

**Path A: Target has ≥4MB RAM (PC, 3DS, Vita, PSP, Wii, GameCube)**
No bytecode VM needed — the current JSON interpreter works fine.
```
Engine no_std migration:       ~500 line changes (imports + feature gates)
Platform binary (display,      ~200 lines
  input, allocator, RNG)
Build script (.bat / Makefile) ~50 lines
Target JSON (if not official)  ~20 lines
──────────────────────────────────────
Total:                          ~770 lines
```

**Path B: Target has 1-4MB RAM (PS1, N64, DS, Dreamcast)**
Bytecode VM required (replaces JSON interpreter, eliminates serde).
The VM itself is ~1,443 LOC — a one-time cost that then unlocks every
low-RAM target.
```
Build-time compiler:            ~300 lines (compile_abilities.py)
Ability VM runtime:             ~1,443 lines (vm.rs)
Platform binary (display,       ~200 lines
  input, allocator, RNG)
Engine no_std migration:        ~200 lines (remove serde gates)
Build script + Target JSON:      ~70 lines
──────────────────────────────────────
Total (first low-RAM port):     ~1270 lines
Each subsequent low-RAM port:   ~270 lines (platform glue only)
```

And zero of those are logic changes — it's all mechanical
transformation. The game plays the same. The bugs are the same.
The cards are the same.

---

## Verdict

The 3DS (2011) is proven. The Vita (2011) would be the easiest
next port — same era, more RAM, official target, full std.

The **bytecode VM** is the key unlock: it's a one-time ~1,443 line
investment that takes PS1 from impossible to comfortable, N64 and DS
from borderline to comfortable, and Dreamcast from unreachable to
reachable (via `rustc_codegen_gcc`).

For older:

| Console | Year | Port effort | Path | Cool factor |
|---|---|---|---|---|
| **GameCube** | 2001 | ~250 lines | A (JSON) | PowerPC, tiny box |
| **Wii** | 2006 | ~250 lines | A (JSON) | Same chip, double RAM |
| **PSP** | 2004 | ~750 lines | A (JSON) | Most portable PlayStation |
| **PS1** | 1994 | ~1270 lines | B (bytecode) | First PlayStation — hardcore |
| **N64** | 1996 | ~1270 lines | B (bytecode) | Retro Mario machine |
| **DS** | 2004 | ~1270 lines | B (bytecode) | Dual screen card game |
| **Dreamcast** | 1998 | ~1270 lines | B (bytecode + GCC backend) | VMU memory card saves |

Everything below N64/DS (Genesis, SNES, etc.) is still dead —
their RAM is measured in kilobytes, not megabytes. No amount of
bytecode cleverness fits a card game engine in 64KB.

**GBA is the exception** — it was declared dead (288KB RAM) but got done
anyway: the engine's `compact_*` features + baked deck blobs fit, and the
sprite-based `ObjectTextRenderer` (from `agb`) renders text within the 32KB
sprite VRAM. See the GBA Port section below.

---

## Dreamcast Port — WAMR interpreter pipeline (Aug 2026) — **WORKING, replaces wasm2c**

Second working DC pipeline, and the new preferred one. The engine wasm is no
longer transpiled at all: a ~420KB WAMR classic interpreter runs the 2MB
`rabuka_wasm.wasm` in place on SH-4.

```
engine (Rust no_std) --cargo--> rabuka_wasm.wasm (2MB, embedded as ROM data)
WAMR classic interpreter (~420KB C incl. shell) --kos-cc/sh-elf-gcc--> rabuka_dc_wamr.elf
```

Measured (same wasm artifact as the wasm2c build):

| | wasm2c build | WAMR build |
|---|---|---|
| target-side code | 4.28MB (`-O2`) / 3.07MB (`-Os`) | **~420KB** (+ 2.08MB wasm blob data) |
| stripped ELF | 4.3MB | 2.5MB (mostly the blob itself) |
| linear memory | 93 pages malloc'd | same module, allocated via platform shim |
| engine changes | none | none |

Verified playable in Flycast: full flow boots fast, menus → mulligan → match,
Japanese names via BIOS Shift-JIS font, build tag `rust -> wasm -> WAMR interp (sh-4)`.

### Why this matters beyond Dreamcast

The gate for Tier 6 consoles was never "no LLVM backend" but "no *small* way to
get code there". An interpreter makes program size independent of engine size:
the same ~420KB runtime + any-GCC-target CPU runs the identical wasm. Research
pass (Aug 2026) that led here:

- **wasm3**: wrong shape — its tail-call "meta machine" pre-expands the module
  into RAM (~10-20MB for our build). Rejected.
- **WAMR classic interpreter**: plain-C switch loop, ~56KB core on cortex-m4f,
  no tail-call/TCO requirement, loads module from caller-owned buffer
  (ROM-resident, not copied). Chosen.
- **mrustc** (Rust→C): only x86_64/MSVC/macOS targets, bootstrap-oriented. Dead end.
- **rustc_codegen_gcc**: no SH-4/m68k story; same per-function size problem anyway.
- **cranelift**: no m68k/SH-4 backends. Dead end.
- No maintained Rust→C transpiler exists at all; monomorphization means every
  faithful translator reproduces the ~1,900 functions 1:1 (the wasm2c bloat).
  Only interpretation collapses it — and since speed doesn't matter for a card
  game, its cost is zero.

Console implications:

- **Jaguar** — easier than the wasm2c verdict: blob XIPs from cart, interp +
  tuned linear memory in 2MB DRAM.
- **CD-i** — reopened *conditionally*: interp+pages fit 1MB RAM; wall moves to
  "the 2MB wasm must load into the same 1MB". Needs wasm shrunk <~600KB — and
  under an interpreter every source-level de-genericization byte counts (unlike
  wasm2c where GCC re-inflated it).
- **Genesis / Neo Geo** — still dead, now mathematically: interpreter + one
  64KB wasm page cannot fit 64KB DRAM even with code XIP. Native C or native
  Rust (LLVM-m68k fix upstream) remain the only paths.
- **Saturn / PS1 / N64** — inherit a proven SH-4-class runtime path.

### Files

| File | Purpose |
|------|---------|
| `platforms/dc/wamr/dc_main_wamr.c` | DC shell over WAMR embed API (same text grid/maple input as wasm2c shell) |
| `platforms/dc/wamr_kos/platform_internal.h` | minimal WAMR platform shim for KOS (single-threaded, no-op sync) |
| `platforms/dc/wamr_kos/kos_platform.c` | KOS impl of WAMR platform API (malloc-backed mmap, stubs) |
| `platforms/dc/wamr_kos/wasm_blob.S` | `.incbin` embedding of rabuka_wasm.wasm |
| `platforms/dc/build_dc_wamr.sh` | WSL-side build (interpreter + shell + link), run via bash |

Build outputs: `platforms/dc/output/rabuka_dc_wamr{,_stripped}.elf`,
packaged with mkdcdisc like the wasm2c build.

### One-time WSL setup

Same toolchain base as the wasm2c port (`/root/kos`, `/root/sh-elf`,
`/root/mkdcdisc`), plus:

```
git clone --depth 1 https://github.com/bytecodealliance/wasm-micro-runtime.git /root/wamr
```

### Gotchas discovered while building (apply to future WAMR ports)

1. **Register natives BEFORE `wasm_runtime_load`.** Import linking happens in
   the loader; registering after load yields "failed to link import function"
   warnings at instantiate even though registration returned true.
2. **Compile the WAMR core against pure newlib (`-nostdinc`, sh-elf include
   dirs only).** KOS's `arch/types.h` typedefs `int8/uint16/...` which clash
   with WAMR's `platform_common.h` typedefs (char vs signed char). kos_platform.c
   alone includes kos.h — it renames the KOS types with macros around the
   include. The shell compiles normally with kos-cc.
3. **GCC 15 defaults to C23** where `f()` means `(void)` — breaks
   `invokeNative_general.c`. Build with `-std=gnu99`.
4. **This toolchain prepends `_` to C symbols but not asm labels.** C-side
   externs for the blob are declared without leading underscore
   (`binary_rabuka_wasm_wasm_start`) matching asm labels written with one.
5. **Don't embed the blob via objcopy binary objects** (`-O elf32-shl -B sh`)
   — ld rejects them on endian merge against the `-ml` link. Use `.incbin`.
6. `config.h` needs a BUILD_TARGET define (SH-4 isn't known): `-DBUILD_TARGET_ARM`
   is inert for interp-only builds (all uses are JIT/AOT/GC-guarded).
7. `wasm_trap_delete` referenced by `wasm_runtime_invoke_c_api_native` — one-line
   no-op stub in the platform file.
8. Use `WAMR_BUILD_INVOKE_NATIVE_GENERAL=1` semantics (compile
   `invokeNative_general.c`, no arch asm exists for SH-4). Our four imports are
   i32-only so the generic C path's ABI caveats don't apply.
9. Platform API surface needed (THREAD_MGR=0, JIT/AOT=0, hw-trap off):
   bh_platform_init/destroy, os_malloc/realloc/free, os_mmap/munmap/mprotect/
   mremap(+_slow_fixup), os_getpagesize, os_time_get_boot_us,
   os_time_thread_cputime_us, os_dcache_flush(void)/os_icache_flush(ptr,len),
   mutex/thread stubs, os_thread_get_stack_boundary (returns uint8_t*).

### Historical: the abandoned rustc_codegen_gcc attempt (Jul 2026)

Toolchain: working. Produces SH-4 ELF binaries.
Status: **Entry point not reaching Rust code** due to DCE in rustc_codegen_gcc.

### Root cause
rustc_codegen_gcc (GCC backend for rustc) dead-code eliminates all
user-defined symbols for `#![no_std]` targets on SH. Even `#[used]`,
`#[global_allocator]`, `#[export_name]`, C wrapper objects, and linker
`--no-gc-sections` fail to prevent this. The `hello` example works
because it uses `std`, which provides proper entry point routing.

The std path is blocked because the engine's dependency chain pulls in
`getrandom → libc`, and the KOS-patched libc version (0.2.175) doesn't
match what cargo fetches (0.2.186). Fixing this requires:
1. Updating the KOS libc patch to version 0.2.186 ✓ (done)
2. Adding missing types (sigset_t, etc.) to the KOS libc patch
3. Or removing the `rand` dep which pulls in getrandom (engine binary only, not lib)

### binary
- `platforms/dc/output/rabuka_dc.elf` — valid SH-4 ELF, statically linked, 4.6KB
- Machine: Renesas SH, Entry: 0x1038 (_start → abort via weak _arch_main)
- To make runnable: need entry point to reach Rust code

### Next
Two paths to finish:
- **A**: Patch KOS libc to include missing types → std `fn main()` works
- **B**: Fix rustc_codegen_gcc symbol emission for no_std (upstream fix)

---

## The wasm2c pipeline unlocks the rest of Tier 6 (Aug 2026)

> **Superseded by the WAMR interpreter pipeline** (see "Dreamcast Port — WAMR
> interpreter pipeline" above): same unlock, ~10× less target-side code, and
> under an interpreter wasm-shrinking actually counts (with wasm2c, GCC
> re-derived the bloat). Kept for history; the CD-i size analysis below is
> wasm2c-specific and pessimistic for WAMR.

The DC port proved the pattern: *any* CPU that GCC speaks can now run the engine,
because wasm2c output is plain C. The old "Tier 6" table (rustc_codegen_gcc era)
is obsolete — the gate is no longer "does Rust target this CPU" but two boring
questions: **(1) does m68k/sh/whatever-GCC compile 20MB of generated C, and
(2) does the board's RAM hold the wasm linear memory?**

### Atari Jaguar — revised verdict (was: dead, RAM too small)

Correcting the record from the Tier 5/6 notes above:

- **Cartridge size is a non-issue.** Official spec: ROM carts up to **6MB**
  (per the Jaguar Software Reference Manual; several commercial carts shipped at
  4MB, Skunkboard does 8MB flash). The recurring "2MB max cart" claim confuses
  cart ROM with the console's 2MB DRAM.
- **Cart ROM is memory-mapped and executable-in-place.** Unlike Dreamcast, where
  everything must fit in RAM, the ~4.3MB of generated code + baked card data can
  stay on the cart and execute from ROM. Only the wasm linear memory needs DRAM.
- **The real wall: 2MB DRAM vs 6.1MB linear memory** (93 pages, mostly the 4MB
  static `HEAP` bump allocator). `HEAP` is capacity, not usage — match state +
  scratch for a card game should be far under 1MB.
- **Plan:** instrument `BumpAlloc` with a high-water mark (headless
  `rabuka_wasm_match` prints it), shrink the static heap to measured usage +
  margin, drop linear memory to ~32–48 pages. Leaves room for C stack and a
  small framebuffer in 2MB.
- **Gotchas:** m68k has no native 64-bit ops → wasm2c's `i64` arithmetic is
  software-emulated at 13MHz (fine for per-turn compute). Big-endian host →
  wasm2c memory access must use the byte-wise path or every baked `u32` blob
  reads byteswapped (verify first with `rabuka_wasm_card_count()`). The 20MB C
  file will need `-Os` / `-fno-tree-*` patience on m68k-gcc.

Verdict: **feasible, blocked only on heap measurement.** Same generated C as DC.
Measured SH-4 `text` is 4.28MB at `-O2` / 3.07MB at `-Os` (see CD-i section
below) — both fit a 6MB cart executing in place, and m68k `-Os` density will be
similar or better. The heap-diet measurement is still wanted for the 2MB DRAM
budget, but nothing here threatens the port.

---

## Sega Genesis native m68k attempt — blocked on LLVM, not on RAM (Aug 2026)

A native (non-wasm2c) Genesis port was attempted in `platforms/genesis/`.
Result: **the pipeline works end-to-end and produced a linked ROM once; the
full-engine build now dies inside LLVM's m68k assembler.** The blocker is the
compiler backend, not RAM, not the engine code, and not the platform glue.

### What was proven working

- `m68k-unknown-none-elf` is a real rustc target (`rustc --print target-list`,
  Tier 3). No prebuilt rustup artifacts — requires nightly + `-Zbuild-std=core,alloc`.
- Engine lib cross-compiles for m68k with feature set identical to GBA
  (`genesis = no_std + bytecode_abilities + compact_cards + compact_card_data +
  compact_state`, added to `engine/Cargo.toml`).
- Link path: Ubuntu WSL's `m68k-linux-gnu-gcc` as rustc linker with
  `-nostdlib -lgcc -Tlink.ld`. (`rust-lld` fails with `unsupported e_machine
  value: 4`; bfd ld via gcc flavor works. libgcc supplies
  `__sync_val_compare_and_swap_1` — m68k has no 1-byte CAS.)
- A complete ROM image was produced and inspected: vectors at $0 (SSP=$FF0000,
  PC=$200), SEGA header at $100, `.text` at $200, work RAM at $FF0000,
  48KB bump heap at $FF4000. `m68k-linux-gnu-size`: text 23,844 / bss 28.
  Real joypad polling ($A10003/$A10009 TH-toggle) wired into the engine's
  `PlatformUi`.

### Corrected RAM math (the old "1/60th" verdict was wrong)

The Tier 5 entry conflated code with RAM. With cart XIP (code executes from
ROM, only data in DRAM — same argument as the Jaguar revision above):

- Card data: NOT the 586KB `CARD_BLOB` — per-deck baked blobs are ~20KB total
  (`platforms/gba/src/decks_baked.rs` pattern), decoded per-match into RAM.
- Heap: 48KB of the 64KB (engine `compact_state` match state measured well
  under this on GBA).
- So RAM fits comfortably. The old "64KB is a microcontroller" line applies to
  putting *code* in RAM, which nobody does on cartridge consoles.

Also corrected while here: `CARD_BLOB` is ~586KB (not "27KB") and the ability
bytecode blob is 28.5KB compressed (~692KB as source literals) — the "~40KB
total data" figure earlier in this doc undercounts the string table.

### The wall: LLVM m68k integrated assembler SIGILL

Compiling the full engine lib for m68k kills rustc:

```
signal: 4, SIGILL
llvm::MCAssembler::relaxOnce / MCAssembler::layout / MCObjectStreamer::finishImpl
```

Deterministic across every knob tried:

| knob | result |
|---|---|
| opt-level z / 2 / 1 | SIGILL all |
| codegen-units 1 / 16 | SIGILL all |
| code-model large | SIGILL |
| embed-bitcode=yes without LTO | SIGILL |
| `-C llvm-args=-no-integrated-as` | flag accepted, still SIGILL |
| nightly 1.99.0-nightly (LLVM 23.1, Aug 2026) | SIGILL |
| nightly-2026-07-15 | SIGILL |
| nightly-2025-08-14 | SIGSEGV in compiler_builtins instead |

Controls that isolate it:

- Minimal no_std crates (Vec/String/format/match) build fine → backend basically
  works; this is size/pattern-triggered.
- A stub-input build (input always "nothing pressed") compiled AND linked at
  23KB — because LLVM proved the menu loop never exits and dead-code-eliminated
  essentially the whole engine before assembly. Making input real (so the game
  code becomes reachable) is what exposed the assembler crash.
- Under fat/thin LTO the lib compiles clean — because cargo passes `-C lto`
  down and rustc emits bitcode-only rlibs, skipping native object emission.
  The crash then moves to the final LTO merge (one giant module → same MC
  layout pass). Thin LTO also dies there.
- Reverting the 5 recently-changed engine files to HEAD versions: still SIGILL
  (the source diff was not the trigger).
- `-C no-integrated-as` no longer exists as a rustc flag (removed upstream);
  routing through llvm-args doesn't reach the MC layer.

Reading: LLVM's m68k MC branch-relaxation cannot lay out our full-engine-sized
modules (~hundreds of KB across CGUs; multi-MB merged under LTO). It handles
small modules fine. This is exactly the "LLVM m68k is experimental" caveat
being load-bearing.

### Paths from here (none taken yet)

1. **Upstream**: file rust-lang/rust + llvm/llvm-project issue with a reduced
   reproducer (bisect engine modules to find the offending function/CGU size).
   The relaxOnce code has known-unreachable paths for unrelaxable branches;
   may be a genuine bug with a small fix.
2. **Bisect + shrink**: find the specific function that kills layout and
   restructure it (split giant matches in `vm.rs`/dispatch code). Unknown
   effort; could be one function or a systemic size threshold.
3. **wasm2c escape hatch** (proven on DC): same generated C compiles under
   m68k-linux-gnu-gcc. Code XIPs from a 4MB cart (3-4MB text OK). The hard
   part is wasm linear memory: default 6.1MB static array cannot live in 64KB
   DRAM — needs the same heap-diet measurement as Jaguar (shrink the 4MB
   `HEAP` bump allocator to measured usage, drop memory to ≤~48 pages) plus
   big-endian byte-wise memory access verification. Feasible on paper, heavy
   in practice.
4. **Wait**: LLVM m68k is actively maturing; a future nightly may just work.
   All scaffolding in `platforms/genesis/` is kept for that day.

Verdict: **RAM-solvable, CPU-toolchain-blocked.** Not "impossible" as previously
written — but not shippable today either.

---

## Philips CD-i — research: the most interesting wasm2c target (Aug 2026)

Why interesting: it's the platform everyone wrote off, with an active (if tiny)
homebrew scene, a real hardware download path, and — uniquely — no cartridge at
all: the game ships on a pressed CD-R alongside the actual consoles. A card game
is exactly the kind of title the CD-i library was full of anyway.

### Hardware (consumer players, e.g. CDI 205/210/220/450)

- CPU: Philips SCC68070 @ 15.5MHz — a plain 68000-compatible core, so
  `m68k-elf-gcc` targets it directly. No custom RISC needed (Tom/Jerry-style
  coprocessor problems don't exist here).
- RAM: **1MB total**. This is the whole ballgame — see below.
- Video: SCC66470/MCD 212, up to 768×560, 16-bit color; text UI trivially fine.
- OS: CD-RTOS (Microware OS-9 derivative) in the 512KB system ROM. Homebrew
  typically bypasses authoring tools entirely: build a bare module, load it via
  the serial stub, run.
- Media: CD-i disc (Green Book, CD-ROM XA sectors). Up to ~744MB per disc —
  size is irrelevant for us.

### Homebrew scene (researched Aug 2026)

Small but genuinely alive:

- **Hardware load path exists**: CD-i Fan's CD-i Link/Stub (cdiemu.org) loads
  binaries into player RAM over the built-in serial port; `Slamy/cdi-serial`
  (Rust, MIT, actively maintained — commits within days of this writing)
  reimplements the protocol: `download app.bin --address 8000 --end --reset`,
  plus debug terminal, NVRAM file access, FUSE mounting of `/cd` and `/nvr`.
- **Real homebrew titles ship**: Frog Feast (2005), Super Quartet (2018),
  Nobelia (2022). Community hub: cdinteractive.co.uk forums.
- **Emulators for dev loop**: CD-i Emulator (CD-i Fan) and MAME (MCD212-class
  drivers) both boot homebrew; MAME is enough for a text-mode card game.
- Toolchain: stock `m68k-elf-gcc` + custom linker script (app loaded at
  $8000-ish by the stub). No SDK license issues — Green Book was freed by
  Philips (1994 version public).

### Feasibility for the wasm2c pipeline

Same pipeline as DC/Jaguar: `cargo → wasm32 → wasm2c → m68k-elf-gcc → serial
download`. **Measured on the real toolchain (Aug 2026, SH-4 via kos-cc; m68k
will be in the same ballpark):**

| stage | size |
|---|---|
| wasm artifact (cargo release) | 2,078,813 B |
| after `wasm-opt -Oz` | 1,683,479 B (−19%) |
| generated C | 21.3MB → 17.9MB (−16%) |
| SH-4 `text` @ `-O2` (baseline) | **4,282,824 B** |
| SH-4 `text` @ `-O2`, from `-Oz` wasm | **4,284,840 B (unchanged!)** |
| SH-4 `text` @ `-Os` | **3,066,824 B (−28%)** |

Key findings:

1. **`wasm-opt` is worthless for console code size.** It shrinks the wasm
   artifact and the generated C, but GCC at `-O2` re-derives its own
   optimization and lands on byte-identical machine code. The bloat is not in
   the wasm — it's in what GCC does to the transcribed C (inlining/unrolling
   ~4,700 generated functions). Don't add wasm-opt to the pipeline for size.
2. **`-Os` on the target compiler is the only lever that works**, and it only
   buys −28%. There is no configuration path from 3MB to 700KB.
3. **Conclusion: CD-i is blocked on code size, not RAM diet.** Even with a
   perfect heap measurement, ~3MB of code cannot fit 1MB RAM. The remaining
   paths are structural, not config: split the wasm module and page content
   from disc between turns, or cut the engine surface drastically. Treat CD-i
   as **shelved** until someone wants to do that surgery; Jaguar (cart XIP,
   6MB) remains the easy m68k win.

### Why `wasm2c` is ~400K lines / 21MB C and why it can't be smaller

`wasm2c` doesn't create the bloat — `rustc` does. Verified on the real
`rabuka_wasm.wasm` (`2,078,813 B`): `wasm-objdump -x` shows `44` types,
`1924` functions, `Code 0x000f0ee1=987KB`, `Data 0x000ca6d9=829KB`,
`Custom "name" 0x000407fb=263KB`. Each `SmallVec<[T;N]>`/`Vec<T>`/
`HashMap<K,V,Hasher>`/`Option<T>` is monomorphized by Rust into a separate
machine-code copy: `drop_glue<Option<Vec<String>>>`, `RawVec<HeartColor>`,
`SmallVec<[i16;6]>` vs `[i16;8]`, `hashbrown::RawTable` variants, etc.
(`engine/src/ability/choice.rs` uses `SmallVec` in ~12 files with `N=2,4,6,8`.)
`platforms/wasm/Cargo.toml` already minimizes it (`opt-level="z"`,
`lto=true`, `codegen-units=1`, `panic="abort"`, `features=["wasm"]`
=`no_std+bytecode_abilities+compact_*`), but the wasm is still `~1K` funcs
of generics.

`wasm2c` is a faithful 1:1 transpiler — one wasm func → one C func
(`4045` `w2c_*` symbols, `1926` `FUNC_PROLOGUE`, `486,423` lines, `21.3MB`);
each wasm opcode expands to `3-7` C statements with `TRAP`/`MEM_ADDR`
bounds checks and `funcref_table[216]`/`elem_segment` for `call_indirect`,
so `987KB` wasm code becomes `~4×` C. `--no-debug-names` only strips the
`263KB` name section (`2.0M→1.8M` wasm, `21.3M→17.9M` C) with no `text` change.
It cannot dedup: wasm has lost generic info and every func is addressable.

Other way = don't use `wasm2c`: keep the `2.0MB` wasm and interpret it
(`wasm3`/`WAMR` `~30KB` interpreter + `9.3MB` linear memory `142` pages),
or compile natively via `rustc_codegen_gcc` (`~600KB-1MB`
`40KB+600KB+300KB`) which was abandoned for `SH-4`/`SH-2` due to `no_std`
DCE `engine/PORTS.md` Dreamcast section. Source-level collapses
(single `SmallVec` `N`, `Vec` instead of `SmallVec`, unified `Hasher`,
`dyn Trait`/`-Zshare-generics`) only shrink the *wasm* — which `wasm2c`
then copies.

Pipeline gotchas discovered while measuring (apply to any new port):

- `wasm-opt` (even plain `-Oz`) **strips the wasm name section**, which is
  where wasm2c derives module symbol names → link errors against the host
  shell (`undefined reference to wasm2c_0x24rabuka...`). If you ever use it,
  add `--debuginfo` to keep names (costs wasm-file bytes only, not machine
  code). wasm2c also falls back to the *filename* for naming, so the file must
  be named `rabuka_wasm.wasm` when transpiling.
- `sjis_table.c` (dc_main dependency) must be on the link line; the
  `build_dc_wasm.sh` in WSL's `/root/dcbuild` is the source of truth, not the
  copy in this repo.
- Repro scripts: `platforms/dc/wasm/build_opt_test.sh` (-O2) and
  `build_os_test.sh` (-Os) run against `/root/dcbuild/optbuild`.

---

## Wii Port — **Done, awaiting build** (Jul 2026)

Status: **Code complete in `platforms/wii/`**. 88MB RAM → Path A (JSON interpreter, std mode, no engine changes).

### Files
| File | Purpose |
|------|---------|
| `Cargo.toml` | Depends on `rabuka_engine` with `bytecode_abilities` (std mode) |
| `powerpc-unknown-eabi.json` | Target spec from `rust-wii/testing-project` |
| `.cargo/config.toml` | Empty rustflags for the target |
| `src/display.rs` | libogc `VIDEO_Init` + `CON_InitEx` → text console via `printf` |
| `src/input.rs` | Both GameCube (`PAD`) and Wii Remote (`WPAD`) input in one poll |
| `src/lib.rs` | Re-exports display + input modules |
| `src/bin/rabuka_wii.rs` | Game loop — identical logic to DC port, adapted for libogc |
| `platforms/wii/build_wii.bat` | Build script: `cargo +nightly build -Z build-std=std,panic_abort` |

### Build
```bash
# Requires: devkitPPC, nightly Rust, rust-src component
cargo +nightly build -Z build-std=std,panic_abort --target powerpc-unknown-eabi.json --release
powerpc-eabi-objcopy -O binary rabuka_wii rabuka_wii.dol
```

### Reference material
Saved to `research/wii/KEY_REFERENCE.md`:
- `rust-wii/ogc-rs` — safe Rust bindings for libogc (79 stars, active)
- `rust-wii/testing-project` — minimal no_std template (archived but target spec is correct)
- `ogc-engine` — game engine built on ogc-rs (6 stars)

### Design decisions
- **No `ogc-rs` dependency**: Using inline FFI extern blocks instead, following the same pattern as the DC port. Avoids the `no_std` requirement of `ogc-rs`.
- **Path A (std mode)**: The Wii has 88MB RAM — no bytecode VM or no_std migration needed.
- **PSP baked data**: Points to `psp/baked/` JSON decks (same as DC), embedded at compile time via `include_str!`.
- **Dual input**: `Input::poll()` scans both `PAD_ScanPads` (GameCube) and `WPAD_ScanPads` (Wii Remote) — whichever you press, it works.

---

## Obscure 6th/7th Gen Handhelds — the stupid interesting ones (Aug 2026)

Dreamcast worked because `wasm→C` dodges the `SH-4 No LLVM` gate. For `6th/7th`
handhelds the gate is `LLVM? Yes` (`ARM` `engine/PORTS.md:40`) — they're
boring `cargo --target armv4t` like `GBA`, not `wasm2c`. What's interesting is
*how dead* their scenes are and whether you can still build/load.

| device | gen | units | CPU / RAM | LLVM | homebrew scene | emulator / load path | verdict |
|---|---|---|---|---|---|---|---|
| **Tapwave Zodiac** `2003-2005` | 6th PDA+handheld | `50k` | `Motorola i.MX1 ARM9 @200MHz` `10MB` system + `32/128MB` storage + `8MB` `ATI Imageon W4200` | `armv4t` `Yes` | **Alive-ish**: Palm OS `5.2T` `prc-tools` `arm-palmos-gcc` free after register; `1200+` devs `2003`; `Tapwave Reborn` + `OpenHandhelds dl.openhandhelds.org/cgi-bin/zodiac.cgi` archives; leaked `Neverwinter Nights`, `Tomb Raider`, `ZDoomZ/ZHeretic/ZHexen`, `UAE`/`ScummVM`/`LJP` ports stock | `POSE` Palm OS Emulator + real HW via `SD` (2× `MMC/SD/SDIO` slots, `480×320` `16-bit`). `Palm OS` `5.2T` runs in `POSE`/`Garnet VM`; no cycle-accurate Zodiac emu but Palm tooling works | **Best boring-ARM stupid port** — `10MB` is `35× GBA` `engine/PORTS.md:195`, bytecode `~1MB` `engine/PORTS.md:69` fits easily; `480×320` `40×26` 12px grid like `Dreamcast` `platforms/dc/wasm/dc_main.c:1`; `Bluetooth`/`Wi-Fi SDIO` + analog nub like a proto-`Switch` |
| **GP32** `2001-2005` Game Park Korea | 6th | `~32k` | `Samsung S3C2400 ARM920T @133MHz` `8MB` `SmartMedia` `320×240` | `armv4t` `Yes` | **The** homebrew handheld: `Free Launcher` (register device → encrypted per-ID) then alt firmware dumps crypto; `gp32x.de` File Archive; `RPG Maker` ports, `DivX` player, `Ericsson Chatboard` mod → Linux ports | `GeP32` Windows emu + real `HW` `SMC`/`USB 1.1`/`JTAG`; successors `GP2X/Wiz/Caanoo` `GamePark Holdings` keep toolchain alive | `8MB` `400×` `SNES` `engine/PORTS.md:211` fits; `SmartMedia` `2-128MB` is the CD-i disc here; community still around |
| **Gizmondo** `2005-2006` Tiger Telematics | 7th | `<25k` worst seller `GamePro` | `Samsung S3C2440 ARM9 @400MHz` `GoForce 4500 1.2MB` `64MB` `WinCE` `GPS/GPRS` | `armv5te` `Yes` | **Dead**: `Windows CE 4.2` `eVC++ 4.0` `arm-wince-gcc` free; `30` cancelled titles (`Colors` GPS AR, `Chicane`), only `14` shipped (`Sticky Balls`); studios `Manchester/Sweden` liquidated `Feb 2006` `$400M` debt; `Wired` crack-up Ferrari `Enzo` `Stefan Eriksson` | No emu; `SD` + `ActiveSync` `USB` + `Regent Street` store; `Smart Adds` ad subs (`£129`) never launched — you get premium HW at discount | Amazing story (`Uppsala mafia`, `Le Mans`), `GPS` `AR` `Colors` was `Pokémon Go` `2005`, but `64MB` is `2× PSP` `engine/PORTS.md:112` — fits, scene just dead |
| **Zeebo** `2009-2011` Tectoy/Qualcomm | 7th | `<30k` Brazil/Mexico `3G` only | `MSM7201A ARM11/QDSP5 @528MHz` `BREW` `160MB` `VGA 640×480` `Adreno Imageon` | `armv6k` `Yes` (`3DS` `engine/PORTS.md:42`) | **BREW gated**: `BREW 3.1.5` `arm-elf-gcc` SDK via `Zeebo Inc` site; `Qualcomm` `Tectoy` `12` partners; `40` games `5-50MB` `OTA 3G` `Z-Credits` `Claro/Telcel`; no `SD` sideload (1 `SD` slot but `OTA` only), no public emu | Device only (`HSUPA/EDGE/GPRS`) + `3× USB 2.0` host; `OTA` firmware updates like `ZeeboNet`; no `MAME` | Most obscure `7th` (never hit `US`), `160MB` is a rocket vs `Zodiac 10MB`, but `BREW` toolchain is `Qualcomm` gated and `OTA`-only distribution kills quick dev |

**Takeaway for rabuka:** all four are `ARM` `Yes` — you never pay the `wasm2c 4.3MB→3.07MB -Os` `engine/PORTS.md:543` tax like `Dreamcast`/`Saturn` `engine/PORTS.md:48`. `compact_*` native `~150KB-1MB` already proven on `GBA 288KB` `platforms/gba/output/GBA_PORT_NOTES.md:1`. `Zodiac` is the sweet spot: obscure `50k` flop killed by `PSP/DS` pre-announce `May 2003`, `ARM9` `prc-tools` still downloadable, `POSE` emulates `Palm OS`, and `10MB` leaves `9MB` for `HEAP` after `~300KB` code. `GP32` runner-up if you want the `SmartMedia`/`Chatboard` meme and a still-seeded archive. `Gizmondo` for the crime story, `Zeebo` for `7th` `BREW` obscurity.

---

## GBA Port — **Done: boots & plays** (Aug 2026)

Status: **Boots and plays the full flow** (mode → deck → RPS → mulligan → match)
in mGBA. The "impossible 288KB" target was reached via `agb` + the engine's
`compact_*` features + baked deck blobs. See `platforms/gba/output/GBA_PORT_NOTES.md`.

| Piece | Detail |
|-------|--------|
| SDK | `agb` 0.25 (`thumbv4t-none-eabi`), `-Tgba.ld`, `agb-gbafix` |
| Build | `cargo +nightly build -Z build-std=core,alloc` + `platforms/gba/build_gba.bat` |
| Engine feature | `gba` = `no_std + bytecode_abilities + compact_cards + compact_card_data + compact_state` |
| Pin | `portable-atomic =1.13.1` (agb needs `unsafe-assume-single-core`; dropped on thumbv4t in 1.14+) |
| Atomics | none on ARMv4T → compat `Arc` is `alloc::rc::Rc` |
| Text | `ObjectTextRenderer` (sprite objects reuse shared glyph tiles) — the background renderer exhausts VRAM in a few screens |
| Crash fixed | re-render overflowed 32KB sprite VRAM (`AllocError` at agb `dynamic.rs:107`) → early-return on unchanged buffer + double `frame()` flush + 240-group cap |

### Files
- `platforms/gba/` — crate (display.rs, input.rs, decks_baked.rs, bin/rabuka_gba.rs)
- `platforms/gba/build_gba.bat`, `platforms/gba/output/rabuka_gba.gba`

### Notes
- No atomics on ARMv4T; RNG seeded (`rng::seed(0x5EED)`).
- GBA OAM = 128 objects max, sprite VRAM = 1024 4bpp tiles; screens capped at
  240 groups (16×16 sprites, 4 tiles each).
- Needs a longer soak test on real hardware.

---

## Tier 7: The research — below 8-bit (NES, SNES, Master System / Game Gear)

We went as low as PS1 (2MB) and GBA (288KB). What exists for *even weaker*
hardware? Two very different answers get conflated under "Rust on":

- **(a) Actually compiling Rust source** to the CPU — requires an LLVM/GCC
  backend for that ISA. Hard, resource-hungry, mostly proof-of-concept.
- **(b) Rust-based/DSL tooling** that hand-emits machine code — a Rust crate
  acting as an assembler/codegen, not real compiled Rust.

The three machines in this tier map to different situations.

### Real-Rust options now exist (LLVM forks) — but check the fine print

The blanket "no LLVM backend" verdict in Gate 1 is out of date. Three
community forks fill the gap:

| Tool | CPU | Consoles | Frontends | Caveats |
|---|---|---|---|---|
| **`llvm-mos`** | 6502 / 65c02 (+ `mosw65816` subtarget) | NES, C64, Lynx; SNES¹ | C, C++, **Zig**; Rust via separate **`rust-mos`²** | Latches C/C++/Zig. **No native 65816 codegen** |
| **`llvm-z80`** (backing `rust-gb`) | Z80 / SM83 | GB, **Master System, Game Gear**, MSX, Spectrum | Rust (via `rust-z80` fork) | WIP; real-Rust compile cost absurd |
| **`cranelift-z80`** | Z80 / SM83 | same | Rust (bespoke backend) | Early stage |

¹ **SNES caveat (`mosw65816`):** llvm-mos accepts a `mosw65816` subtarget and
has a full 65816 assembler/`lld`, so real `.sfc` (LoROM) ROMs *can* be built via
Zig (`zig-mos-examples` ships a Celeste demake). But it still **emits 8-bit
6502-style machine code** — 16-bit register codegen, 24-bit addressing, and
banking are all **open issues** (llvm-mos #32/#319/#320/#321). You get 6502 code
running on the 65816 in 8-bit mode, **not** a native 816 compile.

² **Rust frontend caveat (`rust-mos`):** the Rust path is a **separate rustc
fork** (mrk-its/rust-mos), not part of llvm-mos. Working, real code builds for
6502 exist. But it is **stale — ~2 years behind upstream** (Rust 1.77 era, no
substantive commits since early 2024). So "SNES is the most promising real-Rust
target" is **wrong today**: the Rust frontend is dormant and even C/Zig only get
8-bit codegen on the 65816.

### The honest cost of real Rust on 8-bit

A genuine Rust→Z80 compiler **works**, but it's not viable for real games
(tinycomputers.io, Dec 2025): compiling Rust's `core` for a Z80 peaked at
**~169 GB of RAM** on a 252GB/64-core server, taking 45min for LLVM + 11min for
stage-1 rustc + an unreasonably long `compiler_builtins` pass. The generated
code is decent, but `core`-based binaries exceed most 8-bit systems' capacity.
Java achievements: it's a research/demo path, not a production one.

### Per-console verdict

**SNES (Ricoh 5A22 / 65C816, 128KB + 64KB VRAM)**
- Real-Rust path exists **on paper** (`rust-mos` → `mosw65816`) but is stale
  (~2 years behind upstream rustc) and only produces **8-bit 6502-style code**,
  not native 65816. A working SNES homebrew path is documented via **Zig**
  (`zig-mos-examples`) instead of Rust.
- **R65** (r65.dev) — separate project, "hardware-transparent programming for
  the SNES/65816" with **Rust-inspired** syntax (type-safe registers, bank
  boundaries, first-class processor modes) — not actual Rust, more a 65816 DSL.
- Rest of the ecosystem is C/asm: PVSnesLib + `816-tcc`, `ca65`/`wla-65816`.
- **Verdict: ~128KB RAM + no native-816 codegen → dead for a card game.** Even
  the "real-Rust" route is a stale fork emitting 8-bit code.

**NES (Ricoh 2A03 / 6502, 2KB work RAM)**
- `llvm-mos` is the real-Rust route but nothing turnkey exists ("first Rust
  compiled to 8-bit 6502" is still just a PoC).
- **Millfork** — middle-level language for 6502/Z80, pragmatic middle ground.
- **`nessemble-rs`** — a 6502 assembler **written in Rust** (Rust-based tooling,
  not Rust code). Plus the C/asm stack: cc65, KickC, NESFab, asm6.
- **Verdict: 2KB RAM is not a real computer.** Dead for a card game, full stop.

**Master System / Game Gear (Zilog Z80, 8KB)**
- Key insight: the GB's SM83 **is Z80-family**, so this reuses the same backend
  as `rust-gb`/`llvm-z80`. SMS/GG use a true Z80; `WLA-DX` assembles for them.
- **`retroshield-z80-workbench`** (crate, 2025) — a Rust DSL that emits Z80
  machine code via a fluent `ld_a()`/`call()` API, auto-resolving labels.
  Powers real retro apps (a dBASE clone, a WordStar editor, a spreadsheet).
- **Verdict: same 8KB RAM problem → dead for a card game.** Best real-Rust
  tooling of the three, worst payoff.

### "Is anything in the engine too advanced?" — No. It's infrastructure, not capability

A close look at the real dependency surface shows nothing in the game logic is
untranslatable. The engine already routes around the genuinely hard hardware
constraints via its feature flags + compat layer, and those exact mechanisms
were proven on the weaker GBA:

**Dependency reality check (what a no_std SNES build actually compiles):**
- The critical runtime deps are **small and old and no_std-capable**: `log
  =0.4.22`, `smallvec =1.11`, `hashbrown =0.14`. `serde`/`serde_json`/`rand`/
  actix/tokio are all `optional` and off for console targets.
- Collections are standard `alloc`: `BTreeMap`, `VecDeque`, `Box`, `String`,
  `Vec`, `Rc`, `Arc`. Nothing exotic.
- **`arcstr` is a red herring** — it's not a dependency (only appears as
  variable names in `cards/generate_effect_decoder.py`). The engine's string
  type is its **own** `struct ArcStr(pub Arc<str>)` (`engine/src/core/types.rs`),
  and `engine/src/compat.rs` routes `Arc → Rc` when `target_has_atomic = "ptr"`
  is false. The 6502 has no atomics → same `Rc` fallback as GBA's ARMv4T. This
  specific worry is already solved infrastructure.

**So what's actually complicated? Three concrete frictions, none about Rust's
expressiveness:**

1. **The frozen `core`/`alloc` sysroot, not the crates.** 65816 isn't an
   official rustc target — `rust-mos` ships its *own patched* `core`/`alloc`,
   which is the part stuck at ~Rust 1.77. The risky surface is `alloc`
   **formatting**: `ToString`, `Display`, `format!`, `String::with_capacity`.
   `core::fmt` is notorious for huge code / compiler ICEs on LLVM's 8-bit
   backend (256-byte HW stack, no native 16-bit registers). That's the "slow
   but functional" caveat in practice.

2. **`hashbrown 0.14` as the `HashMap`** (`crate::HashMap = hashbrown::HashMap`)
   is the biggest allocation-heavy chunk and the most sensitive to both the
   sysroot and code size. A fit problem, not a logic problem.

3. **Version drift + tooling mismatch.** The engine pins exact versions
   (`=0.4.22`, `=1.0.228`, `hashbrown 0.14`), but rust-mos builds Xargo-style
   against a frozen 1.77-era libstd (not modern `cargo -Z build-std`), and the
   2026 registry resolves newer transitive deps than a 1.77 rustc accepts.
   Getting the lockfile to resolve on the old toolchain is fiddly.

**Bottom line of this analysis:** a SNES port is blocked by "the only toolchain
that can target 65816 is 2 years behind, ships its own standard library, and
that library's allocation/formatting is the exact part 8-bit LLVM codegen
handles worst" — **not** by the engine doing anything untranslatable.

### Bottom line for rabuka

The 8/16-bit tier stays **dead for a card game engine regardless of toolchain** —
the RAM floor (~150KB for the bytecode interpreter; NES=2KB, SMS/GG=8KB,
SNES=128KB) kills it. Nothing here changed the portability horizon. What the
research *did* update:

1. **Gate 1 is softer than documented** — LLVM forks now exist for 65816,
   6502, and Z80, so SNES/Master System are no longer *language*-blocked. For
   the SNES the RAM is *not* the wall either (128KB + ROM-backed read-only
   data would fit a card game) — the wall is **toolchain fragility**, not RAM.
2. Real Rust on 8-bit is possible but **frail**: the only Rust route on the
   6502 family is the stale `rust-mos` rustc fork (~Rust 1.77), and llvm-mos
   has **no native 65816 codegen** (SNES builds come out as 8-bit 6502 code).
   On Z80 it costs ~169GB RAM to build and oversizes binaries.
3. The productive niche is Rust **codegen/DSL** tooling (retroshield,
   nessemble, Millfork), not true Rust compilation.

**GBA remains the floor for a playable rabuka port.** Everything smaller is
a demo/hobbyist novelty, not a real target.

---

## SNES — the path forward (Aug 2026)

### The C-rewrite idea is dead. Real numbers.

A "just rewrite the rules in C" approach was floated. Measured source says no:

- **`vm.rs` (the ability interpreter): 1,443 LOC** — not 500 as earlier claimed
  (fixed above).
- **Hand-written rules/logic core** (`turn/`, `ability/`, `core/card.rs`,
  `core/game_state/`): roughly **25-40K LOC**. e.g. `live.rs` 2,562,
  `choice.rs` 3,122, `move_cards.rs` 3,012, `card.rs` 3,343, `modifiers.rs`
  1,689, `abilities.rs` 2,585.
- **~95,000 total** engine `.rs` lines — but most is generated *data*
  (`cards_gen.rs` 25K, `abilities_gen.rs` 3.8K) already compiled to binary
  blobs by `compact_card_data`/`bytecode_abilities`, so it is NOT part of a port.
- There's a whole `qa_test_suite.rs` (2,059 LOC) for regression coverage.

Re-expressing even the rules core in C = **tens of thousands of lines**, then
re-testing every ability and edge case. Months of work with a huge bug surface,
for the least-powerful target we'd ever ship. **Not worth it. C is out.**

### Consequence: rust-mos is the *only* path

The engine's architecture (bytecode abilities + baked data blobs + `Rc`-for-
strings via `compat.rs`) is already designed to run on weak no-atomics targets —
proven on GBA. So the ONLY way to get "working on SNES" without a rewrite is to
**compile the actual Rust engine with `rust-mos`**. Phase 0 is no longer an
optional cheap experiment — it's the whole bet.

```
Phase 0  Feasibility gate: does rust-mos build the engine's
         no_std + bytecode_abilities + compact_* core for mosw65816?
         Expected wall: alloc formatting (ToString/Display/format!)
         ICEs or blows up on the 8-bit backend. (Dependency resolution
         is NOT a wall — see audit below.)
         Pass  → everything below is plumbing (GBA-style port).
         Fail  → SNES is genuinely not worth doing (possible, but
                 disproportionate). 
Phase 1  If pass: strip formatting/Display (SNES display is tiles,
         not console text), pin deps to the rust-mos sysroot,
         get a "hello" .sfc via the mos-snes-none target + llvm-mos lld.
Phase 2  Runtime glue (~200-400 C/asm lines): Mode 0/1 tile engine,
         controller poll, Rc strings, ROM-backed card data.
Phase 3  If Phase 0 fails: STOP. No C rewrite.
```

**Toolchain — installable, and WSL works (this section corrects an earlier
overblown estimate):**

- rust-mos is a separate rustc fork, but it ships **prebuilt binaries** — no
  from-source compiler build required. It publishes GitHub release tarballs
  (`rust-mos-ubuntu-24.04`, `rust-mos-linux`, version **1.87.0-dev**,
  `x86_64-unknown-linux-gnu`) and Docker images `mrkits/rust-mos`
  (`latest`/`stable`) + `mrkits/llvm-mos`.
- Install a tarball by extracting and `rustup toolchain link mos <dir>`, then
  `cargo +mos`. A from-source build (`llvm-mos` + `llvm-mos-sdk` + `x.py
  stage-1`) is only ~2-4h / ~16GB RAM / ~35GB disk if you prefer — a normal
  machine, **not** the 169GB Z80 scenario (that figure was a hand-rolled Z80
  backend; llvm-mos is mature and packaged).
- **WSL/Ubuntu x86_64 is the right place to do this** — rust-mos targets
  `x86_64-unknown-linux-gnu`. Use the **WSL2 Linux filesystem**, not `/mnt/c`,
  for the build/output. (Windows-native rust-mos/llvm-mos is not really
  supported; the llvm-mos README flags Windows `core.autocrlf` breaking
  verification. WSL sidesteps this.)
- **SNES is a custom target, not built-in:** rust-mos ships only `mos-unknown-none`.
  The `mos-snes-none.json` spec is user-supplied (`arch:mos`, `cpu:mosw65816`,
  `vendor:snes`, `requires-lto:true`, `linker:mos-common-clang`) and needs
  llvm-mos-sdk's `snes` platform (crt0/`mos-snes-clang`) + custom linker scripts
  (`lorom.ld`/`fastrom.ld`) + a `build.rs`. `kassane/rust-mos-examples` is the
  reference and uses a newer fork (~1.98-dev).
- See `platforms/snes/` (drafted) for the scaffolding.

### Dependency audit vs rust-mos (Rust ~1.87 prebuilt) — good news

The feared "dependency wall" mostly evaporates. The SNES `no_std` core's dep
tree is tiny and old:

- Critical deps (all with `default-features = false`): `log =0.4.22`,
  `smallvec =1.11`, `hashbrown =0.14.5`.
- All three are **leaf crates in this config — zero transitive deps**
  (hashbrown/smallvec/log pull in nothing with default features off and the
  `serde`/`alloc` features disabled).
- **MSRVs all ≤ 1.60**, far below the rust-mos toolchain (~1.87 prebuilt):
  hashbrown 0.14 = 1.56, smallvec 1.11 = 1.36, log 0.4.22 = 1.60. The engine's
  `edition = "2021"` needs 1.56+. `serde`/`serde_json`/`rand`/actix/tokio are
  all `optional` and off for `snes`.

**Conclusion:** dependency *resolution* is NOT the blocker — it resolves fine on
the rust-mos toolchain. The two real remaining risks are (1) `alloc` formatting
(`ToString`/`Display`/`format!`) against llvm-mos's 8-bit codegen, and (2)
getting the SNES `mos-snes-none` custom target + SDK crt0/linker wired. If Phase
0 fails, it'll be on codegen, not on the lockfile.

### Real-world adoption — evidence it works, and where it's thin (Aug 2026)

Verified from the source (not theory): the toolchain **does** produce working
ROMs, but real-world usage is small and clustered, and **SNES is its least-trodden
corner**.

**Proof the pipeline works end-to-end:**
- `kassane/rust-mos-examples` (8 commits, CI) builds a real `demo-snes` →
  `target/mos-snes-none/release/snes-hello.sfc` (LoROM) via a documented one-liner
  (`cargo build --target targets/mos-snes-none.json -Zbuild-std ...`). It ships
  `chr/` graphics + `lorom.ld`/`fastrom.ld` + `build.rs`, so it renders actual
  tiles. Same toolchain builds NES/C64/MEGA65/sim targets.
- `mlund/mos-hardware` (53 stars, 285 commits, active) — Rust register/graphics
  crate for C64/MEGA65/Commander X16 with working demos (plasma, raster IRQ,
  sprites, SID sound). **The strongest evidence rust-mos can do real graphics** —
  but C64, not SNES.
- `mrk-its/llvm-mos-ferris-demo` (Atari 800 factorial PoC), `a800xl-utils`,
  `rust-mos-hello-world`, `retro-display` — the rest of the small ecosystem.

**Where it's thin — SNES specifically:**
- `mos-snes-none` returns **zero** GitHub code-search hits. **Rust→SNES exists only
  as kassane's single `demo-snes`.** It's an experiment, not a scene.
- The *shipping* SNES homebrew on the llvm-mos backend is **C** (Celeste SNES
  demake, SNESDEV-2025) and **Zig** (`kassane/zig-mos` SNES SDK with crt0 +
  LoROM/HiROM + real `.sfc` demos). **Rust is the least-developed of the four
  frontends for 65816.**
- rust-mos issue tracker confirms real codegen sharp edges: #29 **128-bit div ICE**
  (`LLVM ERROR: unable to legalize instruction: (s128) = G_UDIV`), #32 "excessively
  large loops", #27 wrong float→int casts, #35 `c_uint` 16→32-bit ABI drift.

**Bottom line for the bet:** "does it produce a working SNES ROM" = **proven**.
"does it compile a large, allocation-heavy card-game engine" = **genuinely
unproven** — no one has built anything near that scale on rust-mos, and the known
codegen bugs would plausibly bite a 95K-line engine. That is exactly what Phase 0
(a cheap feasibility build) exists to resolve. The evidence says the foundation is
solid; the "compiles the card game" claim is an open experiment, not a sure thing.

### Phase 0 result — RUN ON REAL TOOLCHAIN (Aug 2026)

Actually executed on WSL (Ubuntu 26.04, 12 cores, 3GB RAM) using the kassane
**rust-mos 1.98-dev** x86_64 prebuilt tarball (`rustup`-free: direct
`rustc`/`cargo` from the extracted dir) + the authoritative `mos-snes-none.json`
copied from kassane/rust-mos-examples. Build command used throughout:
`cargo build -Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem
-Zunstable-options -Zjson-target-spec --target mos-snes-none.json`.

**What passed:**
- rust-mos installs & runs on WSL as-is. No Docker, no from-source build.
- A minimal `no_std` crate (u16 loop + `copy_nonoverlapping`) codegen'd for
  `mos-snes-none` in **~21s** (core+alloc+compiler_builtins). Toolchain + target +
  `-Zbuild-std` all work; no OOM at 3GB RAM (`-j4`).
- Building the **engine's `no_std` core in `--release`** got past `core`/`alloc`/
  `compiler_builtins` and resolved the real deps (log 0.4.22, smallvec, hashbrown
  0.14.5) — the dependency-resolution worry is confirmed dead.

**What blocks (both were predicted, now confirmed empirically):**

1. **Debug builds cannot work — must build `--release`.** A non-`--release` build
   hits the **128-bit division ICE** in `core::fmt::num::exp_u128` and a float
   `G_FPTRUNC s32` error in `compiler_builtins` (rust-mos #29). Optimization
   dead-code-eliminates those float/format paths, so `--release` succeeds. This
   is exactly why every kassane demo builds `--release` only.

2. **`smallvec` is fundamentally incompatible with 16-bit `usize`** — its
   `impl_array!` generates `[T; 0x10_0000]`, which on a 16-bit pointer wraps to
   `[T; 0]`, colliding with the size-0 `Array` impl (`error[E0119]`). Fails
   identically on **both smallvec 1.11.0 and 1.15.2** (all versions). The engine
   uses `SmallVec` in ~12 files (dozens of sites), so a per-site `Vec` swap was
   rejected as too invasive.

   **Fix applied (verified):** vendored smallvec 1.15.2 to
   `platforms/snes/vendor/smallvec/` and split `impl_array!` so sizes `>= 0x10000`
   (all multiples of 65536 → wrap to 0) are gated behind
   `#[cfg(not(target_pointer_width = "16"))]`. On 32/64-bit targets the impls are
   byte-identical, so no other port changes. Wired via a scoped
   `[patch.crates-io] smallvec = { path = "vendor/smallvec" }` in
   `platforms/snes/Cargo.toml` (snes is standalone, not a workspace member →
   **zero effect on other engines**). After this, smallvec compiles and the build
   reaches the engine's own code.

3. **Engine const-data ICE (new, discovered by running) — rust-mos compiler bug.**
   Two distinct data problems, one fixed, one not:
   - **`DECK_CARD_FILES` (fixed):** `pub const DECK_CARD_FILES: &[&str]` at
     `deck_parser.rs:80` is 16 × `include_str!` of the baked **deck JSONs** (each
     >64KB, so a `&str` can't exist on 16-bit `usize`). It's only used by the
     serde/JSON path, which `snes` disables — **the SNES port doesn't need it.**
     Gated behind `#[cfg(feature = "serde_support")]`; the `E0080: slice is
     bigger than largest supported object` cleared.
   - **`CARD_BLOB` / `BYTECODE` (NOT fixed — genuine rust-mos ICE):** these ARE
     the needed compact card + ability data (`core/cards_gen.rs`,
     `ability/abilities_gen.rs`). rust-mos crashes with
     `thread 'rustc' panicked at .../consts.rs:221: called Option::unwrap() on a
     None value` while `typeck`-ing these large inline `const &[u8]` literals.
     A `const` → `static` experiment did **not** clear it (same ICE) — so it's a
     **compiler bug in the rust-mos fork on large byte-array typeck**, not a
     data-representation issue. No clean engine-side workaround is obvious.

**Workaround attempt — empirically resolved (Aug 2026).** Ran a focused
experiment on the real toolchain to characterize the failure precisely. Results:

- **A single `[u8; N]` over ~32KB is fundamentally impossible on 16-bit.** Both a
  literal and `include_bytes!` of a 40KB blob fail with
  `E0080: values of the type [u8; 40960] are too big for the target architecture`
  (16-bit max object size ≈ `isize::MAX`). Sizes ≤16KB compile fine. So the
  engine's `CARD_BLOB` (40KB, addressed by global byte offsets) **cannot be one
  contiguous array on 16-bit** — it needs a chunked/per-card data layout, which
  means restructuring `card_binary.rs`'s offset-based reads. Chunking into ≤16KB
  `const` arrays **is verified to compile**.
- **`BYTECODE` hits a second, independent rust-mos ICE** that no representation
  avoids: `consts.rs:221` during `check_match` of `BYTECODE`, triggered by
  `&BYTECODE[start..end]` in `vm.rs:102`. Tested and failed with **const**, with
  **`static`**, and with a **raw-pointer `from_raw_parts` slice** — same ICE
  every time. It's a robust compiler bug in type-checking the referenced bytecode
  const, not a code-shape issue.

**Conclusion (final):** the SNES port is **confirmed impractical for this
engine**, by actually fighting the toolchain rather than speculating. The walls
compound: (1) `smallvec` 16-bit — *fixed*; (2) `DECK_CARD_FILES` — *fixed*
(unneeded); (3) `CARD_BLOB` — hard 16-bit size limit, needs a data-layout
restructure; (4) `BYTECODE` — a `check_match` rust-mos ICE robust to const,
`static`, and raw-pointer forms. Four separate 16-bit issues, at least two
without clean workarounds, all on a niche ~2-yr-behind single-maintainer fork.
**SNES is a dead end for rabuka. Treat `platforms/snes/` (target spec, vendored
smallvec, scaffolding) as dormant research, not a shipping target.** GBA (288KB)
and PS1 (2MB) remain the proven floor. All temporary experiment edits to
`cards_gen.rs`/`abilities_gen.rs`/`vm.rs`/`deck_parser.rs` were reverted — the
only remaining SNES artifacts are the isolated `platforms/snes/` scaffolding
(which does not affect any other port).

4. **Windows host `cargo build`/`test` is broken for an unrelated reason.** A
   hello-world fails to link on this machine in any directory. Root cause: no
   Microsoft VS C++ Build Tools installed (no MSVC `link.exe`/`cl.exe`), and
   devkitPro's MSYS2 `link.exe` (`c:\devkitPro\msys2\usr\bin\link.exe`) — the
   Unix hard-link utility, not a linker — shadows the missing MSVC one on PATH.
   So the `x86_64-pc-windows-msvc` host target can't link at all. `cargo clean`
   is unrelated (confirmed). Fix: install MSVC C++ Build Tools, or switch to
   `x86_64-pc-windows-gnu` + MinGW. This does **not** affect the SNES work, which
   builds in WSL with rust-mos/lld.

**Phase 0 verdict (final):** the toolchain is real and works on WSL; core/alloc
codegen fine in release; deps resolve; **smallvec is fixed** (vendored +
cfg-gated); the **unneeded `DECK_CARD_FILES` is gated out**. But the engine still
does **not** compile for 65816, and the remaining blocker is a **genuine rust-mos
compiler ICE** on the large `CARD_BLOB`/`BYTECODE` byte tables (the data we
actually need). The `const`→`static` workaround failed, so this looks like a real
bug in a niche, ~2-years-behind, single-maintainer compiler fork — possibly with
a workaround (split the blobs smaller, or try a different rust-mos version), but
with no guarantee, and no one likely to fix the compiler. **Recommendation: treat
SNES as a research dead-end, not a shipping target.** The accumulated blockers
(smallvec 16-bit, const-data, then this ICE) are exactly the fragility predicted
at the start, and the rules logic still hasn't even been reached. GBA and PS1
remain the proven floor.

---

## SNES — REVERSED: it now COMPILES (Aug 2026)

**The engine's `no_std` core now builds for `mos-snes-none` (SNES/65816) on the
rust-mos toolchain.** The "dead end" verdict was wrong — it was based on hitting
compiler bugs without yet finding the right workaround. What actually fixed it
(all u8/u16, honoring "narrow offsets where possible"):

1. **Max object size on 16-bit is ~32KB (`isize::MAX`), NOT 64KB.** A single
   `[u8; 60000]` still fails ("extern static is too large"). Chunks must be
   **≤ ~30KB**. (Verified empirically: 16KB passes, 32KB fails.)

2. **`BYTECODE` (92KB) → chunked extern arrays + a `(u8, u16, u16)` loc table.**
   `compile_abilities.py` now splits the bytecode into ≤30KB chunks and emits,
   under `#[cfg(feature = "snes")]`:
   - `extern "C" { pub static BYTECODE_C0..C3: [u8; <30KB>]; }` — data placed in
     ROM by the linker (extern symbols avoid rust-mos's const-eval/match-check ICE).
   - `pub const ABILITY_LOCS: &[(u8, u16, u16)]` — per-ability
     (chunk_idx, start, len), so no offset exceeds 16-bit. A generated
     `bytecode_slice(ci, start, len)` helper returns the slice.
   - `vm.rs` is cfg-aware: host uses the old flat `BYTECODE`/`OFFSET_DELTAS`;
     snes uses `ABILITY_LOCS` + `bytecode_slice`. All `u8`/`u16`.

3. **`CARD_BLOB` (600KB full database) → gated OFF for snes.** The snes/GBA
   runtime loads **per-deck blobs** (`decks_cards_gen::DECK_CARD_BLOBS` via
   `load_two_decks` → `decode_all_cards_from_slice`), not the full database.
   So `CARD_BLOB` + its blob()-based consumers (`blob`, `parse_header`,
   `card_data_offset`, `decode_card_from_blob`, `blob_card_count`,
   `load_cards_from_blob`, `find_card_index_by_no`, `load_all_cards_from_blob`)
   are `#[cfg(not(feature = "snes"))]`; `decode_all_cards_from_slice` (slice-arg,
   no CARD_BLOB) remains available for snes.

**Result:** `cargo build --release -Zbuild-std=core,alloc --target
mos-snes-none.json` **succeeds** (EXIT 0). The host build is unaffected (still
uses the inline const path; both compile). The remaining work to a *running* SNES
game is the ROM build (link the extern blob data in via a linker script + crt0,
the llvm-mos-sdk `snes` platform) and the ~200 lines of tile/input glue — the
"compiles" milestone that was previously blocked is now done. GBA/PS1 remain
proven and shipped; SNES is no longer a compiler dead-end.

---

## SNES — the actual ROM link hits a backend codegen wall (Aug 2026, final)

Took the compile past the finish line to a **real `.sfc` link** and hit the
toolchain's true limit. Honest conclusion: **SNES does not run this engine on the
current rust-mos/llvm-mos, and it is not fixable from the engine side here.**

### What actually works
- **bsnes emulator** runs the demo `snes-hello.sfc` (`platforms/snes/emu/`).
- The full ROM pipeline is wired: crt0, the 92KB `BYTECODE` data object
  (`bytecode_data.c`, defines the extern `BYTECODE_C0..C3`), the LoROM linker
  script, and the SDK init libs are all passed to the linker.
- `platforms/snes/` is a complete port crate (display via a 8x16 font, joypad,
  bump allocator, SNES header, engine bootstrap).

### The wall: rust-mos/llvm-mos legalizer gaps
The `.sfc` link fails at **codegen** (LLVM GlobalISel "unable to legalize"):
- `G_TRUNC (s8)→(s8)` — a no-op truncate, in engine game functions
  (e.g. `execute_modify_required_hearts_success`).
- `G_SCMP (s16)→(s8)` — in **`core::slice::cmp::Ord::cmp`** (the standard library
  slice comparison used by any `Vec` compare/sort).
- `G_FCMP` — float compare in **`core::fmt`** (float formatting).

These are missing legalizer rules in the backend, **not** engine bugs. Even the
`no_std` essentials (`core`/`alloc`) can't be fully codegen'd; you cannot opt out
of `core`'s slice-cmp / `fmt` code. Optimization-level / LTO changes just move the
crash to a different missing pattern (high opt+LTO: engine ICEs; low opt/no-LTO:
`core` float ICEs).

### Why it's not fixable here (verified, not laziness)
- **Engine rewrite = whack-a-mole.** Trimming one ICEing function makes it
  compile, then the same bug fires in the next (`execute_modify_required_hearts_success`
  → `apply_success_zone_effect` → `core::slice::cmp` → ...). Dozens of game
  functions share the patterns. Fixing them guts the game logic, and it never
  reaches `core`, which is unavoidable.
- **Backend patch = needs a bigger machine + real compiler engineering.** llvm-mos
  is a fork of all of LLVM (huge C++); building it needs ~16GB RAM (this WSL box
  has 3GB) and an hour per iteration, so a legalizer fix can't be developed here.

### Bottom line
The engine **compiles** for the SNES target and the entire ROM link is wired —
real, preserved progress. But a **running** SNES game is blocked by rust-mos/llvm-mos
being too immature to codegen the standard library and the game logic. That is a
compiler-backend limitation, not a code problem, and not something the engine (or
this environment) can fix. **GBA remains the floor for a playable rabuka port.**
The SNES work is preserved as research: the engine is SNES-compilable and the port
scaffolding + emulator pipeline are in place for the day the backend matures.

