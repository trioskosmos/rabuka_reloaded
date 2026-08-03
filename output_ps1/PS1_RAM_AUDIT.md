# Rabuka PS1 — RAM Fit Audit & Plan

Status of the PS1 port as of 2026-08-04: **the current build does not link.** This
document records the measured numbers, why the bytecode work didn't make the port
"just fit", what the advanced PS1 homebrew actually do, and the concrete plan to
make `build_ps1.bat` produce a game that runs.

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
only **196 unique card numbers** (~7 KB in the engine's `card_binary` blob
format; each card is 25–41 bytes). Instead of:

```rust
static mut CARD_BUF: [u32; (532_378 + 3) / 4] = [0; ...];  // 532 KB in .bss
... load full CARDDATA.BIN from CD into it ...
```

bake the **deck-card subset** directly into the EXE as rodata (a generated
`decks_card_blob.rs`, ~7 KB) and point `card_binary::EXTERN_CARD_BLOB` at it.
Cards are already pre-resolved from the blob by card number at boot
(`load_deck_cards_from_blob`), so the logic is unchanged — only the source
shrink.

- Saves **~520 KB** of `.bss`.
- Removes the CD dependency entirely (no ISO needed; the EXE self-contained —
  DuckStation fastboot just works).
- This is the same trick the DS port uses (baked blob), just subset to the decks.

### 4.2 Right-size the heap (arena model)

`sys_heap!(256 KB)` is a static in `.bss`. doukutsupsx runs Cave Story on a 16 KB
malloc + arena. Measure the real peak allocation of a match and size the heap to
it (likely 32–64 KB for a card game state machine). Saves **~192–224 KB**.

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

### 4.4 Expected result

```
.text   999 KB   (code, maybe less after 4.3)
.data   759 KB   (maybe ~300 KB after 4.3)
.bss   ~260 KB   (heap 64 KB + statics; no card blob)
total  ~2018 KB  →  under 2,031 KB with real slack for the stack
```

`build_ps1.bat` stays: `cargo psx build` → copy EXE to `output_ps1/rabuka.ps-exe`
→ DuckStation fastboot. No ISO, no CD-ROM reads, no 532 KB anywhere in RAM.

## 5. Validation

1. `build_ps1.bat` links (currently fails).
2. Parse the PS-X EXE header: `load_size + bss < 0x200000 − 0x10000`.
3. Boot in DuckStation: mode select → deck select → a full AI vs AI match plays
   to completion without heap exhaustion or fault.
4. `objdump -h` the ELF (build with `cargo psx build --elf`) to confirm section
   sizes against the table above.
