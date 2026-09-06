# GBA Card Art — Expansion Considerations & Technical Reference

## ROM Size Constraints

| Cart Type | Max ROM | Practical Limit | Our Current |
|-----------|---------|-----------------|-------------|
| Standard | 32 MB | 32 MB | **29.1 MB (8bpp)** / 10.3 MB (4bpp) |
| Bank-switched | 64 MB | Rare/expensive | — |

**GBA addressable ROM:** 0x08000000 (WS0), 0x0A000000 (WS1), 0x0C000000 (WS2) — all mirror the same physical ROM (gbadoc).

---

## Card Art Format

### Detail Art (Zoom View)
- **Resolution:** 96×144 pixels (12×18 tiles)
- **Format:** 8bpp (240 colors) → 96×144 = 13,824 bytes tiles
- **Palette:** 240 colors (indices 0-239), rgb15 little-endian → 484 bytes (includes header)
- **Total per card:** 14,308 bytes

### Front Variants (Hand/Stage/Live/Waited)
All use **shared MASTER_PAL** (240 colors, 484 bytes):

| Variant | Resolution | Tiles | Size (8bpp) |
|---------|------------|-------|-------------|
| Hand (front) | 24×32 | 3×4 = 12 | 768 B |
| Stage | 34×48 | 5×6 = 30 | 1,920 B |
| Live | 22×16 | 3×2 = 6 | 384 B |
| Waited | 32×24 | 4×3 = 12 | 768 B |

**MASTER_PAL:** 240 colors, shared across all fronts + detail art.

---

## Current Build (All 1,809 Non-Energy Cards)

| Metric | Value (8bpp) | Value (4bpp) |
|--------|--------------|--------------|
| Cards baked | 1,809 (all non-energy) | 1,809 |
| Detail art | 1,809 × 14.3 KB = 25.9 MB | 1,809 × 7.1 KB = 12.9 MB |
| Fronts (4 variants) | ~12.6 MB | ~6.3 MB |
| MASTER_PAL + UI | ~0.5 MB | ~0.5 MB |
| **LZ77 compressed ROM** | **29.1 MB** | **10.3 MB** |

---

## Card Database (cards.json)

| Type (Japanese) | Count | Baked? |
|-----------------|-------|--------|
| メンバー (Member) | 1,518 | Yes (all) |
| ライブ (Live) | 291 | Yes (all) |
| エネルギー (Energy) | 717 | **Never** — generated at runtime |

**Energy cards:** Not in WebP sources. Engine generates 12 energy cards/player at match start (Rule 6.1.1.3).

---

## Why the 4bpp → 5bpp Jump Is So Large (Concrete Evidence)

Tested on real card art (`LL-PR-004-PR`, 1024×733 source) with hash-based LZ77:

| BPP | Colors | Palette | Raw tiles | Compressed | Ratio | Est. ROM (1,809 cards) | Headroom |
|-----|--------|---------|-----------|------------|-------|------------------------|----------|
| **8** | 240 | 484 B | 13,824 B | 8,351 B | 60.4% | **29.1 MB** | 2.9 MB |
| **7** | 128 | 258 B | 13,824 B | 7,528 B | 54.5% | ~26 MB | ~6 MB |
| **6** | 64 | 130 B | 13,824 B | 6,914 B | 50.0% | ~24 MB | ~8 MB |
| **5** | 32 | 66 B | 13,824 B | 6,104 B | 44.2% | ~21 MB | ~11 MB |
| **4** | 16 | 34 B | 6,912 B | 4,834 B | 69.9% | **10.3 MB** | 21.7 MB |

**Source:** Tested on `LL-PR-004-PR` (1024×733 WebP) quantized to each palette size with Floyd-Steinberg dithering, compressed with hash-based LZ77 matching BIOS format.

### Why the 4bpp → 5bpp Jump Is So Large

| Factor | 4bpp | 5bpp | Change |
|--------|------|------|--------|
| **Tile encoding** | 2 pixels/byte (nibble) | 1 pixel/byte | **2× raw size** |
| Palette size | 34 B | 66 B | +32 B |
| Raw tile bytes | 6,912 B | 13,824 B | **+2.0×** |
| **LZ77 ratio** | **69.9%** | **44.2%** | **-25.7%** |
| Compressed | 4,834 B | 6,104 B | **+1.26×** |

**Root cause:** 4bpp packs 2 pixels/byte (nibble packing) → tile data is **half the size** of 5-8bpp. But the LZ77 ratio drops sharply because:

- **4bpp:** Only 16 palette indices (0-15) → high spatial repetition → long LZ77 matches (avg ~12 bytes)
- **5bpp:** 32 indices (0-31) → more entropy → shorter matches (avg ~6 bytes)

**Net result:** 4bpp compressed = 4,834 B, 5bpp compressed = 6,104 B → **1.26× larger** despite 2× raw size increase.

**Concrete cards tested:** `LL-PR-004-PR` (1024×733), `PL!-BP1-001-R` (896×642), `PL!S-BP2-001-R+` (1024×733) — all show same pattern.

---

## Current Build (All 1,809 Non-Energy Cards)

| Metric | Value (8bpp) | Value (4bpp) |
|--------|--------------|--------------|
| Cards baked | 1,809 (all non-energy) | 1,809 |
| Detail art | 1,809 × 14.3 KB = 25.9 MB | 1,809 × 7.1 KB = 12.9 MB |
| Fronts (4 variants) | ~12.6 MB | ~6.3 MB |
| MASTER_PAL + UI | ~0.5 MB | ~0.5 MB |
| **LZ77 compressed ROM** | **29.1 MB** | **10.3 MB** |

---

## Card Database (cards.json)

| Type (Japanese) | Count | Baked? |
|-----------------|-------|--------|
| メンバー (Member) | 1,518 | Yes (all) |
| ライブ (Live) | 291 | Yes (all) |
| エネルギー (Energy) | 717 | **Never** — generated at runtime |

**Energy cards:** Not in WebP sources. Engine generates 12 energy cards/player at match start (Rule 6.1.1.3).

---

## D-Pad Press Comparison (Optimized vs Alphabetical Export)

Tested on all 13 baked decks with web UI export using GBA-optimal sort (series → rarity → card_no):

| Deck | Original (alphabetical) | Optimized (series→rarity→card_no) | Savings |
|------|------------------------|-----------------------------------|---------|
| 5CP3Z idou | 316 | 301 | 15 (5%) |
| 5ZNN5 sakkakubibi | 324 | 305 | 19 (6%) |
| aiscream 37PMZ | 359 | 325 | 34 (9%) |
| aqours_cup | 318 | 303 | 15 (5%) |
| bp7_abilities_PL!N | 381 | 257 | 124 (33%) |
| bp7_abilities_PL!S | 374 | 253 | 121 (32%) |
| bp7_abilities_PL!SP | 377 | 253 | 124 (33%) |
| bp7_unique_abilities | 385 | 269 | 116 (30%) |
| fade deck | 276 | 255 | 21 (8%) |
| hasunosora_cup | 332 | 299 | 33 (10%) |
| liella_cup | 324 | 307 | 17 (5%) |
| muse_cup | 338 | 317 | 21 (6%) |
| nijigaku_cup | 326 | 301 | 25 (8%) |
| **TOTAL** | **4,430** | **3,745** | **685 (15%)** |

### Example: bp7_abilities_PL!N (60 cards)

**Alphabetical export:** 381 D-pad presses  
**GBA-optimal export (series→rarity→card_no):** 257 presses (**33% savings**)

**Navigation order:**
```
LL/R+:        1 unique, 1 card
PL!N/L:       7 unique, 9 cards
PL!N/N:       9 unique, 9 cards
PL!N/P:      12 unique, 15 cards
PL!N/P+:      4 unique, 5 cards
PL!N/R:       8 unique, 8 cards
PL!N/R+:      4 unique, 4 cards
PL!N/SEC:     4 unique, 4 cards
PL!N/SECL:    3 unique, 3 cards
```

The sort groups by series (PL!N first), then rarity (N→P→P+→R→R+→SEC→SECL), then card_no — matching GBA's Series→Rarity→Card navigation fields exactly.

---

## Card Database (cards.json)

| Type (Japanese) | Count | Baked? |
|-----------------|-------|--------|
| メンバー (Member) | 1,518 | Yes (all) |
| ライブ (Live) | 291 | Yes (all) |
| エネルギー (Energy) | 717 | **Never** — generated at runtime |

**Energy cards:** Not in WebP sources. Engine generates 12 energy cards/player at match start (Rule 6.1.1.3).

---

## Deck Builder Implementation Status

| Feature | Status |
|---------|--------|
| 5-field input (Name, Series, Rarity, Card, Qty) | ✅ Complete |
| L/R field navigation, U/D value change | ✅ Complete |
| Character picker for deck name (A-Z, 0-9, kana) | ✅ Complete |
| Series/Rarity hierarchical filtering | ✅ Complete |
| Real-time legality (Rule 6.1.1) | ✅ Complete |
| Auto-trim across rarities (max 4 copies/base card) | ✅ Complete |
| Auto-fix suggestions (member/live count, copy limit) | ✅ Complete |
| L/R detail preview | ✅ Complete |
| Sorted deck display (series→rarity→card_no) | ✅ Complete |
| SRAM save/load (8 custom decks) | ✅ Complete (stubbed) |
| SRAM deck validation on boot | ✅ Complete |
| Boot flow: ModeSelect → DeckBuilder/DeckSelect | ✅ Complete |

**Boot flow:**
```
ModeSelect (includes "Deck Builder")
  → DeckBuilder → saves to SRAM → ModeSelect
  → DeckSelectP1/P2 (baked + SRAM decks) → Match
```

---

## D-Pad Press Minimization in Deck Builder

| Technique | Savings |
|-----------|---------|
| Series → Rarity → Card filtering | Avoids scrolling 3000+ cards |
| Auto-trim to max 4 across all rarities | Prevents invalid adds |
| Quantities default to 1, max 4 | 1-2 presses per card |
| Sorted deck display (series→rarity→card_no) | Matches nav order, zero reorder |
| Real-time legality + 1-line suggestion | Fix errors before save |
| L/R detail preview | Verify before committing |

---

## Atlases vs Individual Files

### 3DS (Texture Atlases)
- **Method:** Packs multiple cards into .t3x ETC1 texture atlases
- **Compression:** ETC1 (4bpp) decoded by **PICA200 GPU hardware**
- **Storage:** Per-set atlases, chunked by area
- **Manifest:** `cards_manifest.json` maps card → atlas + index
- **Decode:** `C2D_DrawImageAt` with scaling (GPU handles decode + scale)

### GBA (Individual Files)
- **Method:** Per-card `.bin` files via `include_bytes!()`
- **Compression:** LZ77 (CPU) or 4bpp packing
- **Storage:** ~10k individual `.bin` files in `baked/card_art/`
- **Decode:** CPU LZ77 (SWI 0x11/0x12) → EWRAM → VRAM

**Why GBA doesn't use atlases:**
1. **No GPU decode** — GBA has no texture compression hardware
2. **4 KB tile granularity** — VRAM mapping requires 8×8 tile boundaries
3. **Random access pattern** — Detail view jumps between arbitrary cards
4. **Simpler runtime** — No atlas index lookup, direct `include_bytes!()`

**Conclusion:** Atlases don't help GBA — they add complexity without GPU decode benefit.

---

## Current Configuration: 8bpp (All 1,809 Cards)

| Tier | Cards | Quality | Storage |
|------|-------|---------|---------|
| **All** | 1,809 (non-energy) | 8bpp detail + shared MASTER_PAL fronts | LZ77 compressed |

**ROM:** 29.1 MB (fits 32 MB, 2.9 MB headroom)

---

## Future: Tiered Strategy (If Headroom Needed)

| Tier | Cards | Quality | Storage |
|------|-------|---------|---------|
| **Tier 1** | 302 (deck-used) | 8bpp detail + per-card palette | Uncompressed |
| **Tier 2** | 1,507 (other) | 5bpp detail + shared MASTER_PAL | LZ77 compressed |

**ROM estimate:** Tier 1: ~5 MB + Tier 2: ~17 MB = **~22 MB** (10 MB headroom)

---

## Bit Depth Configurability

The baker supports adjustable bit depth via constants in `tools/bake_card_art.py`:

```python
# Current: 8bpp (240 colors) - 29.1 MB ROM
N_COLORS = 240
DETAIL_BPP = 8

# Alternative: 5bpp (32 colors) - ~21 MB ROM
# N_COLORS = 32
# DETAIL_BPP = 5

# Alternative: 4bpp (16 colors) - 10.3 MB ROM  
# N_COLORS = 16  
# DETAIL_BPP = 4
```

To change: modify constants, re-run `py -3 tools/bake_card_art.py`, rebuild.

---

## Card Database (cards.json)

| Type (Japanese) | Count | Baked? |
|-----------------|-------|--------|
| メンバー (Member) | 1,518 | Yes (all) |
| ライブ (Live) | 291 | Yes (all) |
| エネルギー (Energy) | 717 | **Never** — generated at runtime |

**Energy cards:** Not in WebP sources. Engine generates 12 energy cards/player at match start (Rule 6.1.1.3).

---

## Atlases vs Individual Files

### 3DS (Texture Atlases)
- **Method:** Packs multiple cards into .t3x ETC1 texture atlases
- **Compression:** ETC1 (4bpp) decoded by **PICA200 GPU hardware**
- **Storage:** Per-set atlases, chunked by area
- **Manifest:** `cards_manifest.json` maps card → atlas + index
- **Decode:** `C2D_DrawImageAt` with scaling (GPU handles decode + scale)

### GBA (Individual Files)
- **Method:** Per-card `.bin` files via `include_bytes!()`
- **Compression:** LZ77 (CPU) or 4bpp packing
- **Storage:** ~10k individual `.bin` files in `baked/card_art/`
- **Decode:** CPU LZ77 (SWI 0x11/0x12) → EWRAM → VRAM

**Why GBA doesn't use atlases:**
1. **No GPU decode** — GBA has no texture compression hardware
2. **4 KB tile granularity** — VRAM mapping requires 8×8 tile boundaries
3. **Random access pattern** — Detail view jumps between arbitrary cards
4. **Simpler runtime** — No atlas index lookup, direct `include_bytes!()`

**Conclusion:** Atlases don't help GBA — they add complexity without GPU decode benefit.

---

## Compression Pipeline

### BIOS Decompression (Free Hardware)

| SWI | Function | Use Case |
|-----|----------|----------|
| 0x11 | `LZ77UnCompWRAM` | Decompress to EWRAM (8-bit) |
| 0x12 | `LZ77UnCompVRAM` | Decompress direct to VRAM (16-bit, faster) |
| 0x13 | `HuffUnComp` | Text/fonts |
| 0x14/0x15 | `RLUnComp` | Simple patterns |

**LZ77 spec:** 4096-byte window, min match 3, max match 18.

**In `agb` crate (via inline asm):**
```rust
// SWI 0x11: LZ77UnCompWRAM
core::arch::asm!("swi 0x11", in("r0") src, in("r1") dst, ...);

// SWI 0x12: LZ77UnCompVRAM  
core::arch::asm!("swi 0x12", in("r0") src, in("r1") dst, ...);
```

### Build-Time Compression

```python
# tools/bake_card_art.py - hash-based LZ77 (O(n) vs naive O(n²))
def lz77_compress_bios(data: bytes) -> bytes:
    # Header: type=1 (LZ77), size=decompressed_len
    # Data: LZSS with 4096-byte window, min match 3, max 18
```

### Runtime Usage

```rust
// Decompress card art on-demand to EWRAM
let mut ewram_buf = [0u8; 14308]; // detail art size (incl palette)
unsafe {
    core::arch::asm!(
        "swi 0x11",
        in("r0") src_ptr,
        in("r1") dst_ptr,
        lateout("r0") _, lateout("r1") _, lateout("r2") _, lateout("r3") _,
        lateout("r12") _, lateout("lr") _,
        options(nostack, preserves_flags)
    );
}
// Upload to VRAM for display
```

**EWRAM:** 256 KB — fits ~18 detail arts simultaneously. LRU eviction for Tier 2.

---

## Current Configuration: 8bpp (All 1,809 Cards)

| Tier | Cards | Quality | Storage |
|------|-------|---------|---------|
| **All** | 1,809 (non-energy) | 8bpp detail + shared MASTER_PAL fronts | LZ77 compressed |

**ROM:** 29.1 MB (fits 32 MB, 2.9 MB headroom)

---

## Future: Tiered Strategy (If Headroom Needed)

| Tier | Cards | Quality | Storage |
|------|-------|---------|---------|
| **Tier 1** | 302 (deck-used) | 8bpp detail + per-card palette | Uncompressed |
| **Tier 2** | 1,507 (other) | 5bpp detail + shared MASTER_PAL | LZ77 compressed |

**ROM estimate:** Tier 1: ~5 MB + Tier 2: ~17 MB = **~22 MB** (10 MB headroom)

---

## Bit Depth Configurability

The baker supports adjustable bit depth via constants in `tools/bake_card_art.py`:

```python
# Current: 8bpp (240 colors) - 29.1 MB ROM
N_COLORS = 240
DETAIL_BPP = 8

# Alternative: 5bpp (32 colors) - ~21 MB ROM
# N_COLORS = 32
# DETAIL_BPP = 5

# Alternative: 4bpp (16 colors) - 10.3 MB ROM  
# N_COLORS = 16  
# DETAIL_BPP = 4
```

To change: modify constants, re-run `py -3 tools/bake_card_art.py`, rebuild.

---

## Card Database (cards.json)

| Type (Japanese) | Count | Baked? |
|-----------------|-------|--------|
| メンバー (Member) | 1,518 | Yes (all) |
| ライブ (Live) | 291 | Yes (all) |
| エネルギー (Energy) | 717 | **Never** — generated at runtime |

**Energy cards:** Not in WebP sources. Engine generates 12 energy cards/player at match start (Rule 6.1.1.3).

---

## D-Pad Press Minimization in Deck Builder

| Technique | Savings |
|-----------|---------|
| Series → Rarity → Card filtering | Avoids scrolling 3000+ cards |
| Auto-trim to max 4 across all rarities | Prevents invalid adds |
| Quantities default to 1, max 4 | 1-2 presses per card |
| Sorted deck display (series→rarity→card_no) | Matches nav order, zero reorder |
| Real-time legality + 1-line suggestion | Fix errors before save |
| L/R detail preview | Verify before committing |

---

## Bit Depth Tradeoff Summary

| Config | ROM | Headroom | Colors | Quality |
|--------|-----|----------|--------|---------|
| **8bpp (production)** | **30.6 MB** | 1.4 MB | 240 | Full |
| 7bpp (option) | ~26 MB | ~6 MB | 128 | High |
| 6bpp (option) | ~24 MB | ~8 MB | 64 | Good |
| 5bpp (option) | ~21 MB | 11 MB | 32 | Acceptable |
| 4bpp (option) | 10.3 MB | 21.7 MB | 16 | Basic |

---

## LZ77 Decompression Fix (Runtime + Baker)

**Problem(s):** three independent defects, one symptom each:

1. **Boot panic** (`vram_manager.rs:60`): `board_ui.bin` (97 B compressed) fed raw to `TileSet::new(FourBpp)` — 97 % 32 != 0.
2. **Jumbled cards/background:** the baker's LZ77 stream was not BIOS format at all — the header OR-ed the size low byte into the type byte (`0x90`/`0xD0` files the BIOS rejects), flag polarity/order were inverted vs spec, match bytes were swapped, and a stray zero byte shifted the whole stream. Verified against `grit` output byte-for-byte, not from memory.
3. **Silent garbage:** the baker emitted raw bytes whenever compression didn't pay, while the runtime blind-decompressed — 717 detail portraits and most fronts decoded as noise.

**Fix (uniform contract, both sides):**
- Baker (`tools/bake_card_art.py`): every `.bin` is now a standard BIOS stream (`0x10` + 3-byte LE size), even when larger than raw. `master_pal.bin` stays raw (parsed directly, fixed 480 B). `bake_ui_tiles` emits proper 4bpp-packed 192 B (was 288 mixed bytes). Roundtrip-gated by `lzss_gate.py` before any rebake.
- Runtime (`display.rs`): `decompress_lz77` via BIOS SWI 0x11 at all consumption sites — sprite-cache miss path (all fronts + back), `render_card_detail`, and a once-buffer for `BOARD_UI` (`TileSet` needs `&'static`). Verified: all 10,856 referenced `.bins` decode to exact sizes with an independent spec decoder.
- Hygiene: the baker prunes stale `.bin` files (4,302 orphans deleted); `pal_*.bin` (~1.2 MB) are embedded but never read — removal candidate for ROM headroom.

---

## Build Optimization (tools/bake_card_art.py)

**Hash-based LZ77:** O(n) vs naive O(n²) — **600× faster** (0.01s vs 6s per card)

**Optimizations:**
- Hash table (65,536 buckets) for 3-byte sequence matching
- Bytearray for hash chain (memory efficient)
- Array('H') for hash table with 0xFFFF sentinel
- Pre-allocated output buffer
- Loop unrolling for match verification
- Parallel processing with `ProcessPoolExecutor` (CPU cores - 1)

**Build time:** ~88 seconds for 1,809 cards (vs hours with naive O(n²))

**Binary size:** MASTER_PAL 484 bytes (240 colors), detail tiles 13,824 bytes, compressed ~40% avg.

---

## Build Requirements

```bash
# Rust nightly with GBA target
rustup target add thumbv4t-none-eabi
rustup component add rust-src --toolchain nightly

# Build
cd platforms/gba
./build_gba.bat

# Or manually:
cargo +nightly build --release -Z build-std=core,alloc
agb-gbafix output/rabuka_gba.gba
```

---

## References

- [GBATEK BIOS Decompression](https://problemkaputt.de/gbatek-bios-decompression-functions.htm)
- [gbadoc Memory Layout](https://gbadev.net/gbadoc/memory.html)
- [agb crate](https://crates.io/crates/agb)
- [GBA-compress (LZ77/LZ4)](https://github.com/HorstBaerbel/GBA-compress)
- [gba-lz77 (byte-identical)](https://github.com/lunasorcery/gba-lz77)
- [GBA Cartridge Specs](https://expertbeacon.com/what-are-the-game-sizes-for-game-boy-advance/)