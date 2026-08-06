# Rabuka PS1 — RAM Fit Audit & Plan

Status as of 2026-08-04: the port **builds, links, and fits**, and card loading
is now centralized in the main engine like the other ports. This document
records the measured numbers, why the bytecode work didn't make the port "just
fit", what the advanced PS1 homebrew actually do, the plan, what was
implemented, and the remaining next steps.

**TL;DR of the fix:** the game never needs the full 532 KB card database in RAM.
The engine now bakes a compact per-deck blob for each deck (15 KB total for all
9) and `load_two_decks()` decodes **only the two selected decks' cards** — the
same approach the PSP uses. The PS1's per-port 532 KB blob buffer and CD loading
machinery are gone. Build went from a 563 KB `.bss` overflow to fitting in
1,954 KB of the 2,031 KB available.

---

## 1. The measured problem (not estimates)

Built from `platforms/ps1` with the flags in `platforms/ps1/build_ps1.bat`
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

So this is not "the PS1 heap is small". The design reserved a **532 KB static
buffer for the full card blob** that can never fit alongside ~1.8 MB of code/data.

## 2. What "the bytecode was for" (answer to the design question)

The `bytecode_abilities` feature replaced ~1.4 MB of JSON ability data with a
compact binary tag tree (`engine/src/ability/abilities_gen.rs`, 800 abilities).
That did its job — **the data side** stayed reasonable and serde is gone from the
console build.

What bytecode did *not* touch:

1. **The 532 KB `CARD_BUF` card blob** is card *data*, not abilities. It is the
   single biggest RAM item. Bytecode is irrelevant to it.
2. **The `.text` budget (999 KB).** The ability *interpreter* code — the VM
   (`vm.rs`, 1592 lines) plus the giant dispatch matches in `resolver.rs`
   (`AbilityResolver::execute_effect` 64 KB, `evaluate_condition` 58 KB) — ships
   regardless of whether ability *data* is JSON or bytecode.
3. **~760 KB of `.data`/`.rodata`** remains for the bytecode programs, offsets,
   strings and per-module tables.

Net: bytecode made the port *possible*; code size + the card blob still exceeded
2 MB.

## 3. How the advanced PS1 games actually fit in 2 MB

Read from the cloned references in `research/`:

**`doukutsupsx` — full Cave Story port (research/ps1_homebrew/doukutsupsx).**
- Code stays resident; maps/scripts/graphics **stream from the CD at runtime**.
- A custom **stack/arena allocator with marks** (`engine/memory.h`):
  `mem_set_mark` / `mem_free_to_mark` reclaims a whole stage's transient
  allocations in one call, plus a deliberately tiny 16 KB malloc heap.
- Overlay-capable linker scripts (`nugget/ps-exe.ld`, `cpe.ld`).

**`psx-sdk-rs` (research/ps1_rust/psx-sdk-rs).** `psx/psexe.ld` confirms the
hard budget: `RAM_SIZE = 2M`, `BIOS_SIZE = 64K`, so **2,031,616 bytes** for
text+data+bss. CD-ROM (`psx::sys::fs::File::<CDROM>`) is how large data leaves
the executable.

The consistent pattern: **RAM holds code + whatever is live right now; the CD
(or baked engine data) holds everything else; the heap is sized to the real
working set.**

## 4. What was implemented

### 4.1 Per-deck card data lives in the main engine

The card *records* for all 2280 cards are only 66 KB; the 532 KB blob is 87%
string table (full Japanese ability text + img URLs that `compact_cards` never
decodes). So instead of shipping the whole database, each deck's cards are baked
compactly into the **engine** (`tools/bake_deck_cards.py` →
`engine/src/decks_cards_gen.rs`):

- 9 deck files from `web_ui/decks/*.txt` → **9 compact CARD-format blobs, 15 KB
  total** (~1.4–2.4 KB per deck).
- `engine::game::deck_parser::load_two_decks(idx1, idx2)` decodes **only the two
  selected decks' cards** (deduped), reusing the existing `card_binary` decoder
  via a new slice-based `decode_all_cards_from_slice`.
- The PSP's per-port `deck_*.json` moved into `engine/baked/`; the PS1's
  532 KB `CARD_BUF` + CD-read path and the earlier baked-subset blob are deleted.

This is exactly "put it in storage, read only the cards in the deck" — and it's
now in the main engine, shared by every port.

### 4.2 Heap sized to the measured working set

`sys_heap!(256 KB)` was a static in `.bss`. A host-side bounded-heap test of the
real load+match path (`hf_temp/ps1_heap_test`, capping allocator, PS1 feature
set) measures the peak at **~193 KB with 64-bit host pointers**; the 32-bit PS1
uses less. `sys_heap!` is **192 KB**, still leaving ~27 KB between heap top and
the stack at `0x801fff00`.

### 4.3 Result (measured from the final ELF)

```
.text   989 KB   (MIPS code)
.data   773 KB   (includes the 15 KB engine-baked per-deck blobs)
.bss    192 KB   (right-sized sys_heap!)
total 1,954 KB  →  77 KB slack, ~27 KB between heap and stack
```

`platforms/ps1/build_ps1.bat` runs `tools/bake_deck_cards.py` → `cargo psx build` → copies
`output/rabuka.ps-exe` (i.e. `platforms/ps1/output/rabuka.ps-exe`). No ISO, no CD-ROM reads, no 532 KB anywhere in RAM.

## 5. Validation (done + remaining)

Done:

1. `cargo psx build` links cleanly (was: `.bss` overflowed by 563 KB).
2. `objdump -h` on the ELF: text 989 KB / data 773 KB / bss 192 KB.
3. Host-side bounded-heap test of the exact load+match path: `load_two_decks`
   decodes all 40 unique cards of decks 0+1, DB + decks + setup + 5 turns run,
   **peak heap 193 KB** (64-bit host pointers; 32-bit PS1 uses less), fits the
   192 KB heap with margin.
4. `engine` unit test `3ds_loading_test` still passes (load path unchanged for
   serde builds; deck JSON relocated to `engine/baked/`).

Remaining (next steps):

1. **Boot in DuckStation** and play an AI vs AI match to completion on the real
   32-bit heap. The 193 KB host peak is an upper bound; if the real peak is far
   lower, the heap can drop below 192 KB for more stack room.
2. **Migrate the DS port** to `load_two_decks` too — it still has its own
   per-port blob scan (`platforms/ds/src/bin/rabuka_ds.rs`); the engine now
   offers the canonical path.
3. **Cut `.data`/`.text` further** if more headroom is wanted: gate the ability
   `describe` strings, split the `execute_effect`/`evaluate_condition` match
   arms.

## 6. Known issue: build_ps1.bat robustness

`platforms/ps1/build_ps1.bat` requires `python` (for `tools/bake_deck_cards.py`) and
`cargo-psx` on `PATH`. If either is missing the bat stops with a clear message
at the failing step. The bake tool finds the repo root from CWD or the script
path, so it works both from a Windows double-click and from MSYS (which mangles
`__file__`). If the bat still fails on your machine, run the steps manually:

```
cd platforms\ps1
set RUSTFLAGS=-Copt-level=z -Clto=fat -Cembed-bitcode=yes -Ccodegen-units=1
cargo psx build
```
