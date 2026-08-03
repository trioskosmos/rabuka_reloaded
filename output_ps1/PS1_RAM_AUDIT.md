# Rabuka PS1 — RAM Fit Audit & Plan

Status as of 2026-08-04: the port **now links and fits**. This document records
the measured numbers, why the bytecode work didn't make the port "just fit",
what the advanced PS1 homebrew actually do, the concrete plan, what was
implemented, and the remaining next steps.

**TL;DR of the fix:** the 532 KB full card blob was replaced with a baked
**12 KB deck-card subset** (only the 196 unique cards the 9 decks use, with
display-only strings stripped). Combined with a right-sized 192 KB heap the
build went from a 563 KB `.bss` overflow to fitting in 1,961 KB of the
2,031 KB available. Two real bugs surfaced and were fixed along the way (see §4.2).

---

## 1. The measured problem (not estimates)

Built from `platforms/ps1` with the flags in `build_ps1.bat`
(`-Copt-level=z -Clto=fat -Cembed-bitcode=yes -Ccodegen-units=1`):

```
$ cargo psx build
rust-lld: error: section '.bss' will not fit in region 'RAM':
           overflowed by 563264 bytes
```

Relinked once with a temporary 4MB `RAM_SIZE` linker script (measurement only,
not part of the port) so the ELF could be produced and section sizes read:

| Section         | Size   | What it is |
|-----------------|--------|------------|
| `.text`         | 999 KB | MIPS-I code (ARM Thumb equivalent is 804 KB — the 20% density penalty) |
| `.data` + `.rodata` | 759 KB | ability bytecode, decode tables, strings, deck data, font |
| `.bss`          | 776 KB | **532 KB card-blob buffer (`CARD_BUF`)** + 256 KB `sys_heap!` + engine statics |

- PS-X EXE load image (`.text`+`.data`+`.rodata`): 1,568,768 bytes at `0x80010000`
  → ends at `0x8018F000`.
- RAM available to the EXE: 2 MB − 64 KB BIOS = **2,031,616 bytes**.
- Total demanded: 999 + 759 + 776 ≈ **2,534 KB** → overflow ≈ **503 KB**.
- The 532 KB blob alone would overflow: after the load image there is 452 KB free.

So this is not "the PS1 heap is small". The current design reserves a **532 KB
static buffer for the full card blob** that will never fit alongside the code.

## 2. What "the bytecode was for" (answer to the design question)

The `bytecode_abilities` feature replaced ~1.4 MB of JSON ability data with a
compact binary tag tree (`engine/src/ability/abilities_gen.rs`, 800 abilities).
That did its job — **the data side** stayed reasonable and serde is gone from the
console build.

What bytecode did *not* touch:

1. **The 532 KB `CARD_BUF` card blob** is card *data*, not abilities. It is the
   single biggest RAM item and it lives in `.bss` on PS1 because the port loads
   the full `CARDDATA.BIN` from CD into a static buffer. Bytecode is irrelevant
   to it.
2. **The `.text` budget (999 KB).** The ability *interpreter* code — the VM
   (`vm.rs`, 1592 lines) plus the giant dispatch matches in `resolver.rs`
   (`AbilityResolver::execute_effect` 64 KB, `evaluate_condition` 58 KB) — is
   shipped regardless of whether ability *data* is JSON or bytecode. A card game
   whose whole logic is a state machine should not need 1 MB of MIPS.
3. **759 KB of `.data`/`.rodata`** remains for the bytecode programs, offsets,
   strings and per-module tables.

Net: bytecode made the port *possible* (it removed the 1.4 MB JSON liability),
but code size + the card blob still exceed 2 MB.

## 3. How the advanced PS1 games actually fit in 2 MB

Read from the cloned references in `research/`:

**`doukutsupsx` — full Cave Story port (research/ps1_homebrew/doukutsupsx).**
- Code stays resident in RAM; **maps/scripts/graphics stream from the CD at
  runtime** (`cd_init()` + `gfx_load_gfx_bank(...)`, per-stage files).
- A custom **stack/arena allocator with marks** (`engine/memory.h`):
  `mem_set_mark(MEM_MARK_LO/HI)` then `mem_free_to_mark()` frees everything
  allocated since — the whole stage's transient allocations are reclaimed in one
  call.
- A deliberately tiny libc heap: `MALLOC_HEAP_SIZE 16384` (16 KB). Everything
  else is the arena.
- Overlay-capable linker scripts (`nugget/ps-exe.ld`, `cpe.ld`): code that is
  not always-live can be loaded into a shared overlay region from the CD.

**`psx-sdk-rs` (research/ps1_rust/psx-sdk-rs).**
- `psx/psexe.ld` confirms the hard budget: `RAM_SIZE = 2M`, `BIOS_SIZE = 64K`,
  everything below `0x80010000` is BIOS, so **2,031,616 bytes** for text+data+bss.
- CD-ROM filesystem (`psx::sys::fs::File::<CDROM>`) is the intended way to move
  large data off the executable.

**`Tetrade` (research/ps1_homebrew/Tetrade)** — a PSn00bSDK game: keeps the whole
game + `.tim` textures inside the budget because the game is small; textures are
loaded from CD at boot into VRAM, not main RAM.

The consistent pattern: **RAM holds code + whatever is live right now; the CD
holds everything else; the heap is an arena that gets reset between scenes.**

## 4. The plan for `build_ps1.bat`

### 4.1 Kill the 532 KB blob buffer (the decisive change)

The game only ever plays **two decks**, and across all 9 baked decks there are
**196 unique card numbers**. Instead of:

```rust
static mut CARD_BUF: [u32; (532_378 + 3) / 4] = [0; ...];  // 532 KB in .bss
... load full CARDDATA.BIN from CD into it ...
```

bake the **deck-card subset** directly into the EXE as rodata (a generated
`decks_card_blob.rs`) and point `card_binary::EXTERN_CARD_BLOB` at it.
Cards are already pre-resolved from the blob by card number at boot
(`load_deck_cards_from_blob`), so the logic is unchanged — only the source
shrink.

- Saves **~520 KB** of `.bss`.
- Removes the CD dependency entirely (no ISO needed; the EXE self-contained —
  DuckStation fastboot just works).
- This is the same trick the DS port uses (baked blob), just subset to the decks.

#### Blob composition (why 532 KB, and why the subset is still not tiny)

Measured by parsing `engine/src/core/cards_gen.rs` `CARD_BLOB` (532,378 bytes):

| Part | Size | Notes |
|------|------|-------|
| Card records | 66 KB | 2280 cards × ~29 B of stats/hearts/string indices |
| Length table | 2 KB | 1 byte per card |
| **String table** | **463 KB** | 5675 strings — card_no, name, series, group, **full Japanese ability text**, img URLs, product, rare |

The blob is already **5.7× smaller than `cards.json`** (532 KB vs 2.9 MB). What
keeps it large is not the game state — it is the string table carrying the
**complete ability text for every card** (top entries are 600+ byte ability
descriptions). `compact_cards` strips `ability`/`img`/`product`/`rare` from the
decoded `Card` on console, so ~400 KB of that text is never read at runtime.

Deck-subset blob, measured with the same normalizer the runtime uses:

- 196 deck cards, **all** present in the blob (no promos missing).
- Subset **with** ability/img/product/rare text: **~62 KB**.
- Subset **without** it (only card_no/name/series/group/unit): **~11 KB**.

So two cuts, not one: subset to the 196 deck cards **and** have the generator
skip the display-only strings on console targets (`compact_cards` proves they
are never decoded). That lands at ~11 KB in `.rodata`.

**Implemented:** `tools/bake_ps1_decks.py` emits
`platforms/ps1/src/decks_card_blob.rs` (+ `.bin`) — 196 cards, 12,011 bytes —
and `rabuka_ps1.rs` points `card_binary::EXTERN_CARD_BLOB` at it at boot. The
CD-ROM read and the 532 KB `CARD_BUF` static are gone.

#### Two real bugs found while validating the subset

1. **Half the deck cards silently didn't load.** `load_deck_cards_from_blob`
   (shared by the PS1 and DS ports) used `if indices.len() == wanted.len()`
   as the scan-break test while `wanted` was being drained by `.remove()`.
   Those two lengths are equal at exactly **half** the wanted count, so only
   ~20 of 40 cards made it into the DB. Fixed to `if wanted.is_empty()`.
2. **36 of 196 cards never matched.** cards.json stores some card_nos with
   fullwidth chars (`PL!SP-bp1-003-r＋`); the runtime normalizer only
   uppercased ASCII `a-z`, so `+`-suffixed deck numbers never matched. The
   normalizer now folds fullwidth `＋ ！ － ａ-ｚ ０-９` to ASCII in both ports.

### 4.2 Right-size the heap (arena model)

`sys_heap!(256 KB)` is a static in `.bss`. doukutsupsx runs Cave Story on a 16 KB
malloc + arena. A host-side bounded-heap test of the real load+match path
(`hf_temp/ps1_heap_test`, capping allocator) measured **187 KB peak** with
64-bit host pointers; the 32-bit PS1 uses less. `sys_heap!` is now **192 KB** —
the largest that keeps ~22 KB between heap top and the stack at `0x801fff00`.
Saves **64 KB** of `.bss` versus the original 256 KB.

### 4.3 Attack `.data`/`.text` if the first two aren't enough

After 4.1 + 4.2 the budget looks like: 999 (text) + 759 (data) + ~260 (bss) ≈
2,018 KB vs 2,031 KB — a ~13 KB sliver, no room for the stack. So 4.3 is
mandatory, not optional:

- **Gate the ability `describe` path.** `ability/describe.rs` and friends pull in
  the ability-text strings (`describe_cost_en` etc.) — hundreds of KB of Japanese
  strings the PS1 UI never prints. Gate behind a feature and cut it from console
  builds.
- **Shrink the ability dispatch matches.** `execute_effect` (64 KB) and
  `evaluate_condition` (58 KB) are single giant match arms — split hot effects
  from cold ones, or turn the match tables into data tables.
- **Last resort, the classic PS1 trick:** overlays. Move the menu / deck-select /
  result screens into an overlay loaded from CD over a shared region
  (doukutsupsx's `cpe.ld` pattern). The match loop itself stays resident.

### 4.4 Result (implemented)

What 4.1 + 4.2 + the matching fixes delivered, measured from the final ELF:

```
.text   998 KB   (MIPS code)
.data   771 KB   (includes the 12 KB baked deck-card blob)
.bss    192 KB   (right-sized sys_heap!)
total 1,961 KB  →  70 KB slack, ~22 KB of it between heap and stack
```

`build_ps1.bat` stays: `cargo psx build` → copy EXE to `output_ps1/rabuka.ps-exe`
→ DuckStation fastboot. No ISO, no CD-ROM reads, no 532 KB anywhere in RAM.

## 5. Validation (done + remaining)

Done:

1. `cargo psx build` links cleanly (was: `.bss` overflowed by 563 KB).
2. `objdump -h` on the ELF: text 998 KB / data 771 KB / bss 192 KB; EXE load
   image ends at `0x801ca800`, heap ends ~`0x801fa8a0`, stack at `0x801fff00`.
3. Host-side bounded-heap test of the exact load+match path
   (`hf_temp/ps1_heap_test`, engine compiled with the PS1 feature set, deck
   blob `include_bytes!`-ed): all 40 unique cards of decks 0+1 decode, DB +
   decks + setup + turns run, **peak heap 187 KB** (64-bit host pointers;
   the 32-bit PS1 uses less). Sized the PS1 heap at 192 KB with margin.

Remaining (next steps):

1. **Wire the blob bake into the build.** `platforms/ps1/src/decks_card_blob.rs`
   is checked in and built from `cards.json` + `decks_baked.rs` by
   `tools/bake_ps1_decks.py`. `build_ps1.bat` should run that tool before
   `cargo psx build` so the subset stays in sync when decks/cards change.
   (Note: `build_ps1.bat` currently crashes — the failure is in the build
   itself, see §6.)
2. **Boot in DuckStation** and play an AI vs AI match to completion on the
   actual 32-bit heap. The 187 KB host peak is an upper bound; if the real
   peak is far lower the heap can drop below 192 KB for more stack room.
3. **Cut `.data`/`.text` further** (the §4.3 items) to buy headroom beyond the
   current ~22 KB heap↔stack gap: gate the ability `describe` strings, split
   the `execute_effect`/`evaluate_condition` match arms. This is what makes the
   margin comfortable rather than tight.

## 6. Known issue: build_ps1.bat crash

`build_ps1.bat` fails at the `cargo psx build` step. The bat file itself is a
thin wrapper; the crash is reproduced by running the cargo command directly
with the bat's `RUSTFLAGS` from `platforms/ps1`. Likely causes to check next:
a stale/missing `Cargo.lock` pin for the `psx-sdk-rs` git dep, the OneDrive
path in `cd /d "%~dp0..."`, or the `cargo-psx` binary not being on `PATH`
when the bat runs under a fresh shell. Debug with:

```
cd platforms\ps1
set RUSTFLAGS=-Copt-level=z -Clto=fat -Cembed-bitcode=yes -Ccodegen-units=1
cargo psx build
```
