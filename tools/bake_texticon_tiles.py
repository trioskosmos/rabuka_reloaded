#!/usr/bin/env python3
"""Bake the ability-text texticons into GBA 4bpp tiles for inline rendering.

The engine keeps `{{name.png|label}}` markers in card ability text (see
`platform_ui::card_ability_text`). The 3DS renders them as inline images
(ctru_shim.c `_3ds_draw_label_icons`); this script bakes the same PNGs
(web_ui/img/texticon) so the GBA `Display` can blit them inline too.

Every icon is scaled to 16px tall (one text line = 2x2-tile glyph row) with
the width kept proportional, then sliced left-to-right into 16x16 cells
(= "part1 part2 ..." of the icon, 4 consecutive 8x8 4bpp tiles each).
Transparent pixels get the zone-fill palette index 2 exactly like
tools/bake_font_tiles.py, so icons sit on menu backgrounds seamlessly.
Opaque pixels are quantized to the fixed text-palette colours shared with
display.rs TEXT_PALETTE / DETAIL_TEXT_PALETTE entries 7..=15.

Run:  py -3 tools/bake_texticon_tiles.py
Writes platforms/gba/src/texticons_gen.rs + a preview PNG under
platforms/gba/output/.
"""
import os

from PIL import Image

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC_DIR = os.path.join(REPO, "web_ui", "img", "texticon")
OUT_PATH = os.path.join(REPO, "platforms", "gba", "src", "texticons_gen.rs")
PREVIEW_PATH = os.path.join(REPO, "platforms", "gba", "output", "texticon_preview.png")

CELL = 16  # px per baked cell (one fullwidth glyph slot)
# Same colours as display.rs palette entries; index -> RGB15 quantization targets.
PALETTE_TARGETS = {
    1: (255, 255, 255),
    4: (245, 158, 11),   # gold
    5: (46, 204, 113),   # green
    6: (160, 174, 192),  # dim
    7: (224, 32, 96),    # red
    8: (240, 128, 176),  # pink
    9: (112, 64, 144),   # purple
    10: (128, 192, 0),   # lime
    11: (232, 224, 0),   # yellow
    12: (0, 168, 168),   # teal
    13: (48, 160, 224),  # sky
    14: (184, 120, 48),  # brown/orange
    15: (128, 128, 128), # mid gray
}
ZONE_FILL = 2  # background nibble value, matching bake_font_tiles.py


def nearest_palette_idx(rgb):
    r, g, b = rgb
    best_i, best_d = 1, 1 << 30
    for i, (pr, pg, pb) in PALETTE_TARGETS.items():
        d = (r - pr) ** 2 * 2 + (g - pg) ** 2 * 4 + (b - pb) ** 2
        if d < best_d:
            best_i, best_d = i, d
    return best_i


def pack_4bpp_tile(pixels, cell_w, tx, ty):
    """32-byte 4bpp tile for the 8x8 region at tile (tx, ty) of `pixels`
    (a cell_h=16 bitmap of palette indices, -1 = transparent/zone fill)."""
    tile = bytearray(32)
    for i in range(32):
        tile[i] = ZONE_FILL | (ZONE_FILL << 4)
    for r in range(8):
        for c in range(8):
            x = tx * 8 + c
            if x >= cell_w:
                continue  # right-most partial cell: leave zone fill
            val = pixels[(ty * 8 + r) * cell_w + x]
            if val < 0:
                continue
            byte = r * 4 + c // 2
            shift = 0 if c % 2 == 0 else 4
            tile[byte] &= ~(0xF << shift)
            tile[byte] |= val << shift
    return bytes(tile)


def main():
    names = sorted(
        f[:-4]
        for f in os.listdir(SRC_DIR)
        if f.endswith(".png") and f != "lltcg-back.png"
    )
    print("baking", len(names), "texticons")

    tiles = bytearray()
    lookup = []  # (name, start_tile_index, cols_in_cells)

    for name in names:
        im = Image.open(os.path.join(SRC_DIR, name + ".png")).convert("RGBA")
        w16 = max(1, round(im.width * CELL / im.height))
        im = im.resize((w16, CELL), Image.LANCZOS)
        px = [
            -1 if a < 128 else nearest_palette_idx((r, g, b))
            for (r, g, b, a) in im.getdata()
        ]
        cells = (w16 + CELL - 1) // CELL
        start = len(tiles) // 32
        for cx in range(cells):
            # Right-most partial cell is zero-padded by clamping x reads.
            for ty in range(2):
                for tx in range(2):
                    tiles += pack_4bpp_tile(px, w16, cx * 2 + tx, ty)
        lookup.append((name, start, cells))
        print(f"  {name}: {w16}px -> {cells} cell(s)")

    print("total tiles:", len(tiles) // 32, "bytes:", len(tiles))

    # Preview reconstructing exactly what the GBA will blit.
    def unpack_4bpp(tb):
        img = Image.new("RGB", (8, 8), (26, 35, 50))
        rev = {ZONE_FILL: (26, 35, 50)}
        rev.update(PALETTE_TARGETS)  # index -> RGB
        for r in range(8):
            for c in range(8):
                v = (tb[r * 4 + c // 2] >> (0 if c % 2 == 0 else 4)) & 0xF
                if v:
                    img.putpixel((c, r), rev.get(v, (255, 0, 255)))
        return img

    row_cells = 16
    # Lay out with real per-row cell accounting so wide icons wrap, and
    # upscale x3 so shapes are inspectable.
    rows_layout = []
    row, x = [], 0
    for entry in lookup:
        if x + entry[2] > row_cells:
            rows_layout.append(row)
            row, x = [], 0
        row.append((entry, x))
        x += entry[2]
    rows_layout.append(row)
    preview = Image.new(
        "RGB", (row_cells * CELL * 3, len(rows_layout) * (CELL + 14) * 3), (40, 40, 40)
    )
    from PIL import ImageDraw

    pd = ImageDraw.Draw(preview)
    for r, entries in enumerate(rows_layout):
        oy = r * (CELL + 14)
        for (name, idx, cells), x in entries:
            ox = x * CELL
            for cix in range(cells):
                for q in range(4):
                    tb = tiles[(idx + cix * 4 + q) * 32:(idx + cix * 4 + q + 1) * 32]
                    sub = unpack_4bpp(tb)
                    preview.paste(
                        sub.resize((24, 24), Image.NEAREST),
                        (ox * 3 + cix * 24 + (q % 2) * 24, oy * 3 + (q // 2) * 24),
                    )
            pd.text((ox * 3, (oy + CELL + 1) * 3), name[:14], fill=(200, 200, 200))
    os.makedirs(os.path.dirname(PREVIEW_PATH), exist_ok=True)
    preview.save(PREVIEW_PATH)
    print("wrote", PREVIEW_PATH)

    with open(OUT_PATH, "w", encoding="utf-8") as f:
        f.write("// Auto-generated by tools/bake_texticon_tiles.py -- do not edit.\n")
        f.write("#[repr(align(4))]\n")
        f.write(f"pub struct AlignedTiles(pub [u8; {len(tiles)}]);\n")
        f.write("pub static TEXTICON_TILES: AlignedTiles = AlignedTiles([\n")
        for i in range(0, len(tiles), 16):
            f.write("    " + ", ".join(str(b) for b in tiles[i:i + 16]) + ",\n")
        f.write("]);\n")
        f.write("/// (icon name without .png, start tile index, width in cells)\n")
        f.write("pub const TEXTICON_GLYPHS: &[(&str, u16, u16)] = &[\n")
        for name, idx, cells in lookup:
            f.write(f'    ("{name}", {idx}, {cells}),\n')
        f.write("];\n")
    print("wrote", OUT_PATH)


if __name__ == "__main__":
    main()
