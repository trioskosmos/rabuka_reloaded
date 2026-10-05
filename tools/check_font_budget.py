"""Arithmetic on the baked font vs agb's BG tile pool.

agb's 4bpp tile allocator (agb/src/display/tiled/vram_manager/tile_allocator.rs)
splits 0x06000040-0x06008000 into an affine region and a 512-tile regular
region, and `alloc_for_regular` ends in `.expect("Ran out of video RAM for
tiles")`. Every distinct tile a frame references must be resident, and
`VRAM_MANAGER.gc()` only runs at commit, so the previous frame's tiles are
still allocated while the next is built.

A fullwidth glyph is 2x2 = 4 tiles. So the pool holds at most
(1022) / 4 ~= 255 distinct fullwidth glyphs across two frames, i.e. about
127 per frame. A 16-row Japanese action list plus headers blows straight
through that, and the result is a panic, not a slow-down.

This prints the numbers so the budget is a checked constant rather than a
surprise.
"""

import io
import re

FONT = r"platforms/gba\src\font_tiles_gen.rs"
ICON = r"platforms\gba\src\texticons_gen.rs"

# agb tile_allocator: regular region = CHARBLOCK_SIZE*2/32 - 512 tiles,
# affine region = 256*2-2 tiles. Both 4bpp here.
REGULAR = 16384 * 2 // 32 - 512
AFFINE = 256 * 2 - 2
POOL = REGULAR + AFFINE


def glyphs(path, name):
    s = io.open(path, encoding="utf-8").read()
    m = re.search(name + r": &\[\(char, u32, u32\)\] = &\[(.*?)\n\];", s, re.S)
    if not m:
        return None
    ents = re.findall(r"\('\\u\{([0-9a-fA-F]+)\}',\s*(\d+),\s*(\d+)\)", m.group(1))
    return [(int(c, 16), int(i), int(w)) for c, i, w in ents]


def main():
    g = glyphs(FONT, "FONT_GLYPHS")
    if not g:
        print("could not parse FONT_GLYPHS")
        return 1
    full = [x for x in g if x[2] == 2]
    half = [x for x in g if x[2] == 1]

    print("=== agb 4bpp BG tile pool ===")
    print("  regular region : %d tiles" % REGULAR)
    print("  affine region  : %d tiles" % AFFINE)
    print("  total          : %d tiles (%d bytes)" % (POOL, POOL * 32))

    print()
    print("=== baked font ===")
    print("  glyphs         : %d" % len(g))
    print("  fullwidth 2x2  : %d  (4 tiles each)" % len(full))
    print("  halfwidth 1x2  : %d  (2 tiles each)" % len(half))
    need = len(full) * 4 + len(half) * 2
    print("  if every glyph is on screen at once: %d tiles" % need)
    print("  -> exceeds the pool by %d tiles (%.1fx)" % (need - POOL, need / POOL))

    print()
    print("=== how much text actually fits ===")
    per_frame = POOL // 2          # gc only runs at commit: 2 frames resident
    print("  tiles usable in one frame (2-frame residency): %d" % per_frame)
    print("  fullwidth glyphs per frame                    : %d" % (per_frame // 4))
    print("  halfwidth glyphs per frame                    : %d" % (per_frame // 2))
    print("  a 16-row x 30-col Japanese action list needs up to 480 glyphs")

    print()
    print("=== the fix ===")
    print("  The font must not be uploaded per-frame through agb's shared")
    print("  allocator. It is immutable: bake it into a FIXED charbase region")
    print("  at boot and address it by index, so it costs no allocator budget")
    print("  and cannot panic. The dynamic pool is then only for UI chrome.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())