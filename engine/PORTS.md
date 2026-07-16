 Rabuka Console Port Analysis

## How Far Back Can We Go?

The engine is a card game. No physics, no 3D, no real-time constraints.
Portability is limited by three hard gates:

1. **LLVM must support the CPU** (Rust can't target unsupported architectures)
2. **Enough RAM** (~4MB minimum: 2MB card data + 1MB code + 1MB state/stack)
3. **Someone writes ~200 lines of platform glue** (display, input, allocator)

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
| 65816 | SNES | **No** |
| 6502 | NES, C64, Lynx | **No** |
| Z80 | MSX, Master System, GG, Spectrum | **No** |
| HuC6280 | TurboGrafx-16 | **No** |
| TLCS-900 | Neo Geo Pocket | **No** |
| V30MZ (x86-16) | WonderSwan | **No** |

Everything **without** an LLVM backend is immediately out — Rust can't
generate code for it. Dreamcast (SH-4), Saturn (SH-2), SNES (65816),
NES (6502), and all Z80-based machines are dead on arrival.

### Gate 2: RAM

The engine needs roughly:
- **Card database**: ~2MB (baked binary format, not JSON)
- **Code**: ~500KB-1MB
- **Game state + heap + stack**: ~1MB
- **Realistic minimum**: ~4MB

Consoles that fail:
- **PS1** (2MB) — not viable despite having an official Rust target
- **GBA** (288KB) — not even close
- **DS** (4MB) — borderline, would need extreme trimming
- **Genesis / Neo Geo** (64KB) — laughable
- **N64** (4-8MB) — borderline, could barely work with expansion pak

---

## Tier 1: Proven Working

### Nintendo 3DS (2011)
- CPU: ARM11 MPCore @ 268MHz
- RAM: 128MB
- SDK: devkitARM (`armv6k-nintendo-3ds`), `ctru-rs`, `cargo-3ds`
- Status: **Already works.** See `engine_3ds/`.
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
- Engine changes: **need no_std migration**
- **Hard block: 2MB RAM is insufficient.** Card database alone
  overflows this. Requires either streaming from CD-ROM (slow) or
  gutting the card pool to a tiny subset.

### Nintendo 64 (1996)
- CPU: MIPS VR4300 @ 93MHz
- RAM: 4MB (8MB with Expansion Pak)
- Target: None (would need custom `mipsel-n64-none-eabi` JSON)
- Std: **No**
- Engine changes: **need no_std migration**
- Borderline even at 8MB — RAM constraint is the limiter

### Nintendo DS (2004)
- CPU: ARM9 @ 67MHz + ARM7 @ 33MHz
- RAM: **4MB**
- Target: None (`armv5te-none-eabi` + custom JSON)
- Std: **No** — no_std + alloc
- SDK: `nds-rs` (very early, 10 stars)
- Engine changes: **need no_std migration + extreme memory trimming**
- 4MB is borderline for Rust + alloy + card data

---

## Tier 5: CPU Supported but RAM Too Small

### Sega Genesis / Mega Drive (1988)
- CPU: 68000 @ 7.6MHz
- RAM: **64KB**
- Target: `m68k-unknown-none-elf` (LLVM m68k is experimental)
- Verdict: **Impossible** — 64KB is 1/60th of what's needed

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
- Verdict: **Impossible** — 288KB is ~1/15th of what's needed

---

## Tier 6: CPU Not Supported by LLVM

| Console | CPU | Year | Why |
|---|---|---|---|
| Sega Dreamcast | SH-4 | 1998 | No LLVM backend for SuperH |
| Sega Saturn | SH-2 | 1994 | No LLVM backend for SuperH |
| SNES | 65816 | 1990 | Not in LLVM |
| NES | 6502 | 1983 | Not in LLVM |
| Master System / Game Gear | Z80 | 1985 | Not in LLVM |
| TurboGrafx-16 | HuC6280 | 1987 | Not in LLVM |
| WonderSwan | V30MZ (x86-16) | 1999 | LLVM has no 16-bit x86 mode |
| Neo Geo Pocket | TLCS-900 | 1999 | Not in LLVM |
| Commodore 64 | 6510 | 1982 | Not in LLVM |
| MSX | Z80 | 1983 | Not in LLVM |

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
 engine_3ds     engine_ps1       engine_pc_cli
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
   target it.

4. **The engine doesn't use `unsafe` much** — no inline assembly,
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

```
Engine no_std migration:       ~500 line changes (imports + feature gates)
Platform binary (display,      ~200 lines
  input, allocator, RNG)
Build script (.bat / Makefile) ~50 lines
Target JSON (if not official)  ~20 lines
──────────────────────────────────────
Total:                          ~770 lines
```

And zero of those are logic changes — it's all mechanical
transformation. The game plays the same. The bugs are the same.
The cards are the same.

---

## Verdict

The 3DS (2011) is proven. The Vita (2011) would be the easiest
next port — same era, more RAM, official target, full std.

For older:

| Console | Year | Port effort | Std? | Cool factor |
|---|---|---|---|---|
| **GameCube** | 2001 | ~250 lines | Yes | Very cool — PowerPC, tiny box |
| **Wii** | 2006 | ~250 lines | Yes | Same chip as GC, double RAM |
| **PSP** | 2004 | ~750 lines | No | Most portable PlayStation |
| **PS1** | 1994 | ~750 lines | No | Hardcore, but 2MB kills it |

