#!/usr/bin/env python3
"""Bake the game's used glyphs into a static GBA 4bpp tile set.

Regenerates tools/font/used_chars.txt from the single source of truth
(tools/font/used_chars.py, which scans card data, engine/platform Rust,
locales and decks), rasterizes each from assets/yoster_ja.ttf, packs them into
8x8 4bpp tiles (halfwidth=1 tile, fullwidth=2x2 tiles), and emits
engine/src/font_tiles_gen.rs with the raw tile bytes and a
char->(tile_index, tile_cols) lookup.

Run:  py -3 tools/bake_font_tiles.py
"""
import sys, os, math
from PIL import Image, ImageDraw, ImageFont
from fontTools.ttLib import TTFont

sys.path.insert(0, os.path.dirname(__file__))
from font.used_chars import compute_used_chars, repo_root

FONT_PATH = "platforms/gba/assets/yoster_ja.ttf"
NOTO_PATH = "tools/font/NotoSansCJKjp-Regular.otf"
USED_PATH = "tools/font/used_chars.txt"
OUT_PATH = "platforms/gba/src/font_tiles_gen.rs"
PX = 12  # rasterization size
TILE = 8


def is_fullwidth(c):
    cp = ord(c)
    return (
        (0x1100 <= cp <= 0x11FF) or (0x2E80 <= cp <= 0x2EFF) or
        (0x3000 <= cp <= 0x30FF) or (0x3100 <= cp <= 0x31FF) or
        (0x3200 <= cp <= 0x32FF) or (0x3300 <= cp <= 0x4DBF) or
        (0x4E00 <= cp <= 0x9FFF) or (0xAC00 <= cp <= 0xD7AF) or
        (0xF900 <= cp <= 0xFAFF) or (0xFE30 <= cp <= 0xFE4F) or
        (0xFF00 <= cp <= 0xFFEF)
    )


def char_bitmap(font, c, cell_w, cell_h, left_align=False):
    """Rasterize char into a cell_w x cell_h monochrome bitmap (0/1).
    Draws the glyph directly at full size (no shrink). Halfwidth glyphs are
    left-aligned so they pack tightly at 8px; fullwidth are centered.
    Placement is clamped so the glyph's bounding box stays inside the cell
    (some fonts report ascender/descender boxes that would otherwise push
    short glyphs like `{`, `|`, `_` off the bottom of the cell)."""
    img = Image.new("1", (cell_w, cell_h), 0)
    d = ImageDraw.Draw(img)
    bbox = d.textbbox((0, 0), c, font=font)
    w = bbox[2] - bbox[0]
    h = bbox[3] - bbox[1]
    x = -bbox[0] if left_align else (cell_w - w) // 2 - bbox[0]
    y = (cell_h - h) // 2 - bbox[1]
    if w <= cell_w and h <= cell_h:
        x = max(0, min(x, cell_w - w))
        y = max(0, min(y, cell_h - h))
    d.text((x, y), c, font=font, fill=1)
    data = img.tobytes()
    # PIL '1' mode -> 1 bit/pixel, row-major
    bmp = [0] * (cell_w * cell_h)
    for i, byte in enumerate(data):
        for bit in range(8):
            if byte & (0x80 >> bit):
                p = i * 8 + bit
                if p < cell_w * cell_h:
                    bmp[p] = 1
    return bmp


def pack_4bpp_tile(bmp, cell_w, tx, ty):
    """Return the 32-byte 4bpp tile for the 8x8 region at tile (tx,ty) of a
    cell_w-wide bitmap (cell_h assumed 16 => 2 tiles tall)."""
    tile = bytearray(32)
    for r in range(8):
        for c in range(8):
            src = (ty * 8 + r) * cell_w + (tx * 8 + c)
            val = bmp[src] & 0xF
            byte = r * 4 + c // 2
            # GBA 4bpp: each byte = 2 pixels, left pixel in the low nibble,
            # right pixel in the high nibble (even col -> shift 0, odd -> 4).
            shift = 0 if c % 2 == 0 else 4
            tile[byte] |= val << shift
    return bytes(tile)


def main():
    # Regenerate the used-character file from the single source of truth, so the
    # GBA font always covers every string (incl. engine-hardcoded Japanese).
    used_text = compute_used_chars(repo_root())
    with open(USED_PATH, "w", encoding="utf-8") as f:
        f.write(used_text)
    chars = sorted(set(used_text))
    print("baking", len(chars), "glyphs")

    font12 = ImageFont.truetype(FONT_PATH, 12)
    noto12 = ImageFont.truetype(NOTO_PATH, 12)
    # Fall back to Noto Sans CJK for any glyph the primary pixel font lacks
    # (ASCII punctuation, Greek, symbols like ?/??/?, ...) so nothing bakes
    # as a .notdef tofu box.
    y_cmap = set(TTFont(FONT_PATH).getBestCmap())

    def pick_font(ch):
        return noto12 if ord(ch) not in y_cmap else font12

    tiles = bytearray()
    lookup = []  # (char, tile_index, tile_cols)

    for c in chars:
        # All glyphs at 12px in 16x16 cells (2x2 tiles). Advance is proportional:
        # halfwidth glyphs advance 1 tile if they fit 8px, else 2; fullwidth always 2.
        full = is_fullwidth(c)
        def render(fnt):
            return char_bitmap(fnt, c, 16, 16, left_align=(not full))
        fnt = pick_font(c)
        bmp = render(fnt)
        if not any(bmp):
            # primary font drew nothing — retry the other one
            bmp = render(noto12 if fnt is font12 else font12)
        idx = len(tiles) // 32
        for ty in range(2):
            for tx in range(2):
                tiles += pack_4bpp_tile(bmp, 16, tx, ty)
        if full:
            cols = 2
        else:
            # pixel width of the glyph -> tiles (proportional advance)
            w = 0
            for xx in range(16):
                for yy in range(16):
                    if bmp[yy * 16 + xx]:
                        w = xx + 1
            cols = max(1, -(-w // 8))
        lookup.append((c, idx, cols))

    print("total tiles:", len(tiles) // 32, "bytes:", len(tiles))

    # Preview PNG of the baked glyph cells so the rendered shapes can be inspected.
    preview = Image.new("RGB", (32 * 16, ((len(lookup) // 32) + 1) * 16), (40, 40, 40))
    pd = ImageDraw.Draw(preview)
    for n, (c, _idx, _cols) in enumerate(lookup):
        px, py = (n % 32) * 16, (n // 32) * 16
        full = is_fullwidth(c)
        bmp = char_bitmap(pick_font(c), c, 16, 16)
        for yy in range(16):
            for xx in range(16):
                if (yy * 16 + xx) < len(bmp) and bmp[yy * 16 + xx]:
                    preview.putpixel((px + xx, py + yy), (255, 255, 255))
    preview.save("platforms/gba/output/font_preview.png")
    print("wrote platforms/gba/output/font_preview.png")

    # "Packed" preview: reconstruct glyphs from the actual 4bpp tile bytes the
    # GBA uses (reverse of pack_4bpp_tile). If this looks garbled, the packing is
    # wrong; if it looks good, the bug is on the Rust load/placement side.
    def unpack_4bpp(tile_bytes):
        img = Image.new("1", (8, 8), 0)
        for r in range(8):
            for c in range(8):
                byte = tile_bytes[r * 4 + c // 2]
                val = (byte >> (0 if c % 2 == 0 else 4)) & 0xF
                if val:
                    img.putpixel((c, r), 1)
        return img

    packed_preview = Image.new("RGB", (32 * 16, ((len(lookup) // 32) + 1) * 16), (40, 40, 40))
    for n, (c, idx, cols) in enumerate(lookup):
        px, py = (n % 32) * 16, (n // 32) * 16
        t = tiles[idx * 32:idx * 32 + 4 * 32]
        cell = Image.new("1", (16, 16), 0)
        for ty in range(2):
            for tx in range(2):
                sub = unpack_4bpp(t[(ty * 2 + tx) * 32:(ty * 2 + tx + 1) * 32])
                cell.paste(sub, (tx * 8, ty * 8))
        for yy in range(16):
            for xx in range(16):
                if cell.getpixel((xx, yy)):
                    packed_preview.putpixel((px + xx, py + yy), (255, 255, 255))
    packed_preview.save("platforms/gba/output/font_preview_packed.png")
    print("wrote platforms/gba/output/font_preview_packed.png")

    with open(OUT_PATH, "w", encoding="utf-8") as f:
        f.write("// Auto-generated by tools/bake_font_tiles.py -- do not edit.\n")
        f.write("#[repr(align(4))]\n")
        f.write(f"pub struct AlignedTiles(pub [u8; {len(tiles)}]);\n")
        f.write(f"pub static FONT_TILES: AlignedTiles = AlignedTiles([\n")
        for i in range(0, len(tiles), 16):
            f.write("    " + ", ".join(str(b) for b in tiles[i:i+16]) + ",\n")
        f.write("]);\n")
        f.write("pub const FONT_GLYPHS: &[(char, u32, u32)] = &[\n")
        for c, idx, cols in lookup:
            f.write(f"    ('\\u{{{ord(c):04x}}}', {idx}, {cols}),\n")
        f.write("];\n")
    print("wrote", OUT_PATH)


if __name__ == "__main__":
    main()
