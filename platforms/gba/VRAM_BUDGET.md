GBA VRAM budget for this port
============================

Derived from agb 0.25.0 sources (verified by reading them, not assumed):

  agb/src/display/tiled.rs:291      VRAM_START             = 0x0600_0000
  agb/src/display/tiled.rs:292      SCREENBLOCK_SIZE       = 0x800  (2 KiB)
  agb/src/display/tiled.rs:293      CHARBLOCK_SIZE         = 0x4000 (16 KiB)
  agb/src/display/tiled.rs:295      SCREENBLOCK_ALLOC_START= VRAM_START + CHARBLOCK_SIZE*2
                                    = 0x0600_8000
  agb/.../vram_manager/tile_allocator.rs:9   AFFINE_ALLOC_END
                                    = VRAM_START + 256 * 64 = 0x0600_4000
  agb/.../tile_allocator.rs:31      affine pool  = 0x0600_0040 .. 0x0600_4000
                                    256*2-2 = 510 tiles of 32 B
  agb/.../tile_allocator.rs:36      regular pool = 0x0600_4000 .. 0x0600_8000
                                    CHARBLOCK_SIZE*2/32 - 512 = 512 tiles of 32 B
  agb/.../tile_allocator.rs:42      alloc_for_regular() falls through to affine and
                                    then `.expect("Ran out of video RAM for tiles")`

So agb's 4bpp BG tile pool is 510 + 512 = 1,022 tiles (32,704 B) and NOTHING
else. Its BGCNT charbase is 0, i.e. the two charblocks 0x0600_0000-0x0600_8000.

OBJ is separate and also fixed:
  agb/.../sprites/sprite.rs:17       SPRITE_ALLOCATOR = 0x0601_0000 .. 0x0601_8000
                                    (1024 slots of 32 B, counts 4bpp; agb's
                                    BlockAllocator header is 8 B per block, so
                                    ~32,704 B usable, and 8bpp sprites consume
                                    64 B per 8x8 tile)

THE DEFECT THIS EXPOSED
-----------------------
The baked font is 952 glyphs: 840 fullwidth (2x2 = 4 tiles) + 112 halfwidth
(1x2 = 2 tiles) = 3,584 tiles. agb's entire BG pool is 1,022. A frame that
references the whole font needs 3.5x what exists, and `VRAM_MANAGER.gc()` only
runs inside `GraphicsFrame::commit()`, so the previous frame's tiles are still
resident while the next is built. Peak demand is two frames' worth.

At 4 tiles per fullwidth glyph, 1,022 tiles / 2 frames = 511 tiles = ~127
distinct fullwidth glyphs per frame. The Actions view alone shows 8 rows x
30 columns of Japanese ability text, so it can exceed that on its own.

The result is not a slowdown: it is `.expect("Ran out of video RAM for
tiles")`, an agb panic, on the main menu and on deck select. agb's panic
handler then renders in bitmap mode 3, clobbering VRAM, which is the
"glitching" and the crash.

THE FIX
-------
The font is immutable. Uploading it per-frame through a shared dynamic
allocator is the mistake. Bake it once into its own charbase and address it by
index:

  * Reserve one charblock for the font at 0x0600_8000 (charbase 2).
    0x4000 B = 16 KiB holds 512 tiles = 128 fullwidth glyphs, or the whole font
    if it is re-baked as 1x2/2x1 pairs. The font is currently 3,584 tiles =
    114 KiB, so it must be re-baked smaller OR split across charbases.
  * Set BGCNT charbase to 2 for the text BG, write the tile bytes once at boot,
    and have the text blitter write tile indices directly instead of calling
    `RegularBackground::set_tile` (which allocates).
  * The dynamic 1,022-tile pool is then only used for UI chrome (board_ui,
    texticons, zone fills), which is a few dozen tiles.

This makes text cost zero allocator budget and removes the panic entirely.

INTERIM GUARD (do this regardless)
----------------------------------
Until the font is resident, `blit_text` must not be able to overdraw the
pool. The cheapest correct mitigation is to bound how much text a single frame
blits (cap the Actions view at the rows that fit in the remaining budget) and
to report the shortfall rather than letting agb panic.

WHAT IS ALREADY CORRECT AND MUST NOT BE REVERTED
------------------------------------------------
  * tools/bake_card_art.py `lz77_compress_bios`: LZ77 flag bits are LSB-first
    (bit 0 describes the first item). They were MSB-first, which made all
    ~9,500 baked blobs invalid; that alone produced a black screen.
  * card_art_gen.rs `lz77_decompress_wram`: bounds-checked pure-Rust decoder
    replacing a hand-rolled `swi 0x11` whose inline asm did not clobber
    r4-r11, so the BIOS could return a clobbered pointer.
  * The art pipeline round-trips: tools/check_lz77_roundtrip.py PASSES and
    tools/bake_card_art.py now asserts every blob at bake time.
