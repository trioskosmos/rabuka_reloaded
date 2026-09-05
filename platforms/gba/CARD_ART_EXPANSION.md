# GBA Card Art — Expansion Considerations & Technical Reference

## ROM Size Constraints

| Cart Type | Max ROM | Practical Limit | Our Current |
|-----------|---------|-----------------|-------------|
| Standard | 32 MB | 32 MB | **10.8 MB** |
| Bank-switched | 64 MB | Rare/expensive | — |

**GBA addressable ROM:** 0x08000000 (WS0), 0x0A000000 (WS1), 0x0C000000 (WS2) — all mirror the same physical ROM (gbadoc).

---

## Card Art Format

### Detail Art (Zoom View)
- **Resolution:** 96×144 pixels (12×18 tiles)
- **Format:** 8bpp (256 colors) → 96×144 = 13,824 bytes tiles
- **Palette:** 240 colors (indices 0-239), rgb15 little-endian → 480 bytes
- **Total per card:** 14,304 bytes

### Front Variants (Hand/Stage/Live/Waited)
All use **shared MASTER_PAL** (240 colors, 480 bytes):

| Variant | Resolution | Tiles | Size (8bpp) |
|---------|------------|-------|-------------|
| Hand (front) | 24×32 | 3×4 = 12 | 768 B |
| Stage | 34×48 | 5×6 = 30 | 1,920 B |
| Live | 22×16 | 3×2 = 6 | 384 B |
| Waited | 32×24 | 4×3 = 12 | 768 B |

**MASTER_PAL:** 240 colors, shared across all fronts + detail art (Tier 2).

---

## Current Build (Deck-Only)

| Metric | Value |
|--------|-------|
| Cards baked | 302 (deck-used, non-energy) |
| Detail art | 302 × 14,304 B = 4.3 MB |
| Fronts (4 variants) | ~3.5 MB |
| MASTER_PAL + UI | ~0.5 MB |
| **Total ROM** | **~10.8 MB** |

---

## Card Database (cards.json)

| Type (Japanese) | Count | Baked? |
|-----------------|-------|--------|
| メンバー (Member) | 1,518 | Deck-used only |
| ライブ (Live) | 291 | Deck-used only |
| エネルギー (Energy) | 717 | **Never** — generated at runtime |

**Energy cards:** Not in WebP sources. Engine generates 12 energy cards/player at match start (Rule 6.1.1.3).

---

## Expansion Scenarios

### Tier 1: All Non-Energy Cards (1,809 cards)

| Approach | ROM | Feasibility |
|----------|-----|-------------|
| Current quality, uncompressed | ~36 MB | ❌ Exceeds 32 MB |
| LZ77 compressed (BIOS SWI 0x11) | ~22 MB | ✅ Fits |
| LZ77 + shared MASTER_PAL (detail) | ~21 MB | ✅ Comfortable |
| LZ77 + shared palette + 64×96 detail | ~15 MB | ✅ Headroom |

### Tier 2: All 2,526 Cards

| Approach | ROM | Feasibility |
|----------|-----|-------------|
| LZ77 + shared palette + 64×96 | ~21 MB | ✅ Fits 32 MB |

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

**In `agb` crate:**
```rust
gba::bios::LZ77UnCompWRAM(src: *const u8, dest: *mut u8);
gba::bios::LZ77UnCompVRAM(src: *const u8, dest: *mut u16);
```

### Build-Time Compression

```python
# tools/bake_card_art.py additions
import lz4_flex  # or custom LZ77 matching BIOS format

def compress_lz77_bios(data: bytes) -> bytes:
    """Produce BIOS-compatible LZ77 (LZSS variant)."""
    # Use gbacomp / gba-lz77 for byte-identical output
    # Header: 32-bit (type=1, size=decompressed_len)
    pass
```

### Runtime Usage

```rust
// Decompress Tier 2 card on-demand to EWRAM
let mut ewram_buf = [0u8; 14304]; // detail art size
unsafe {
    gba::bios::LZ77UnCompWRAM(compressed_ptr, ewram_buf.as_mut_ptr());
}
// Upload to VRAM for display
```

**EWRAM:** 256 KB — fits ~18 Tier 2 detail arts simultaneously. LRU eviction.

---

## Hybrid Tiered Strategy (Recommended)

| Tier | Cards | Quality | Storage |
|------|-------|---------|---------|
| **Tier 1** | 302 (deck-used) | Full 96×148 8bpp + per-card palette | Uncompressed, fast |
| **Tier 2** | 1,507 (other non-energy) | 64×96 4bpp + shared MASTER_PAL | LZ77 compressed |

**ROM estimate:**
- Tier 1: ~10.8 MB (current)
- Tier 2 compressed: ~9 MB
- **Total: ~20 MB** — fits 32 MB with headroom

---

## Detail Screen Resolution Fix

**Current:** `ART_W = 96, ART_H = 144` — this IS the baked resolution.

**Display path:** `Display::render_card_detail()` renders the portrait at 96×144 pixels (12×18 tiles) in the left portion of the screen.

**No change needed** — the baked resolution matches what the detail screen uses.

---

## Build Process

1. **Deck list** → `tools/bake_deck_cards.py` → `web_ui/decks/*.txt`
2. **Card art** → `tools/bake_card_art.py` → `platforms/gba/baked/card_art/`
   - Only bakes cards referenced in decks (non-energy)
   - Outputs: `pal_*.bin`, `tiles_*.bin`, `front_*.bin`, `stage_*.bin`, `live_*.bin`, `wait_*.bin`
3. **Rust source** → `include_bytes!()` embeds binary blobs
4. **Cargo build** → `output/rabuka_gba.gba` (~11 MB)

---

## Adding New Cards

1. Add card to `cards/cards.json` with WebP image in `web_ui/img/cards_webp/`
2. Add card to deck file in `web_ui/decks/*.txt`
3. Run `./build_gba.bat` (re-bakes only changed cards ideally)
4. Card appears in Deck Builder and baked decks

---

## Future: Full Card Set (All 1,809 Non-Energy)

### Required Changes

1. **Build script:** Switch from `deck_card_nos()` to `all_non_energy_card_nos()`
2. **Compression:** Add LZ77 compression for Tier 2 cards
3. **Runtime:** On-demand BIOS decompression with EWRAM cache
4. **Palette:** Use MASTER_PAL for Tier 2 detail art (no per-card palette)

### Estimated Timeline

| Task | Effort |
|------|--------|
| LZ77 build pipeline | 2-3 hours |
| Runtime decompress + cache | 3-4 hours |
| Tier 1/Tier 2 logic | 2 hours |
| Testing on hardware/emulator | 2 hours |
| **Total** | **~10 hours** |

---

## References

- [GBATEK BIOS Decompression](https://problemkaputt.de/gbatek-bios-decompression-functions.htm)
- [gbadoc Memory Layout](https://gbadev.net/gbadoc/memory.html)
- [agb::bios docs](https://docs.rs/gba/latest/gba/bios/)
- [GBA-compress (LZ77/LZ4)](https://github.com/HorstBaerbel/GBA-compress)
- [gba-lz77 (byte-identical)](https://github.com/lunasorcery/gba-lz77)
- [GBA Cartridge Specs](https://expertbeacon.com/what-are-the-game-sizes-for-game-boy-advance/)