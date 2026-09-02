#!/usr/bin/env python3
"""Render every GBA card art size to PNG folders so the baked pixels can be
inspected visually.

Reuses the exact bake functions from bake_card_art.py (same cover-crop, same
quantization, same palette) but instead of emitting Rust tile arrays it
reconstructs the RGB image each bake would produce on VRAM and writes it to:

    platforms/gba/preview/<size>/<card_no>.png

Folders:
    detail/   96x128  8bpp  per-card 240-col palette   (CARD_ART)
    hand/     24x32   8bpp  shared 240-col master      (CARD_FRONTS)
    stage/    40x48   8bpp  shared 240-col master      (STAGE_FRONTS)
    live/     16x24   8bpp  shared 240-col master      (LIVE_FRONTS)

Also writes a contact sheet per folder (card_no labels) for quick scanning.

Run:  py -3 tools/preview_card_art.py [card_no ...]
     no args  -> all deck cards; args filter by card_no substring
"""

import os
import sys
from pathlib import Path

from PIL import Image, ImageDraw

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "tools"))
sys.path.insert(0, str(REPO / "cards"))

import bake_card_art as B  # noqa: E402

OUT = REPO / "platforms" / "gba" / "preview"

SIZES = {
    "detail": (B.ART_W, B.ART_H, None, "8bpp per-card 240"),
    "hand": (B.FRONT_W, B.FRONT_H, B.FRONT_GRID, "8bpp shared 240"),
    "stage": (B.STAGE_W, B.STAGE_H, B.STAGE_GRID, "8bpp shared 240"),
    "live": (B.LIVE_W, B.LIVE_H, B.LIVE_GRID, "8bpp shared 240"),
}


def rgb15_to_rgb(b):
    r = (b[0] & 31) << 3
    g = ((b[1] >> 5) & 31) << 3
    bl = (b[1] & 31) << 3
    return (r, g, bl)


def reconstruct_8bpp(tiles: bytes, w: int, h: int, pal_q, grid=None):
    """Unpack 8bpp tile bytes -> RGB image using PIL palette image `pal_q`.
    `w`/`h` are the card pixel size; `grid` (tiles_w, tiles_h) is the tile
    grid the bake used (may be larger than the card, with padding)."""
    gw, gh = grid if grid else (w // 8, h // 8)
    im = Image.new("P", (gw * 8, gh * 8))
    px = im.load()
    for ty in range(gh):
        for tx in range(gw):
            base = (ty * gw + tx) * 64
            for rr in range(8):
                row = tiles[base + rr * 8: base + rr * 8 + 8]
                for cc in range(8):
                    px[tx * 8 + cc, ty * 8 + rr] = row[cc]
    im.putpalette(pal_q.getpalette())
    return im.convert("RGB")


def reconstruct_detail(tiles: bytes, pal_bytes: bytes, w: int, h: int):
    """Detail art: 8bpp tiles + rgb15 palette bytes -> RGB image."""
    # Build a PIL palette image from the rgb15 palette
    pal_img = Image.new("P", (1, 1))
    pal = bytearray(256 * 3)
    for i in range(240):
        r, g, b = rgb15_to_rgb(pal_bytes[i * 2: i * 2 + 2])
        pal[i * 3] = r
        pal[i * 3 + 1] = g
        pal[i * 3 + 2] = b
    pal_img.putpalette(bytes(pal))
    return reconstruct_8bpp(tiles, w, h, pal_img)


def main():
    filters = sys.argv[1:]
    used = B.deck_card_nos()
    if filters:
        used = {n for n in used if any(f.lower() in n.lower() for f in filters)}
    cache = B.CACHE

    # Build shared master palette once (same as bake_card_art.main)
    thumbs = []
    png_cache = {}
    for card_no in sorted(used):
        png = cache / f"{card_no}.png"
        if not png.exists():
            continue
        img = Image.open(png).convert("RGB")
        png_cache[card_no] = img
        target = B.STAGE_W / B.STAGE_H
        iw, ih = img.size
        if iw / ih > target:
            nw = int(ih * target)
            left = (iw - nw) // 2
            thumb = img.crop((left, 0, left + nw, ih))
        else:
            nh = int(iw / target)
            top = (ih - nh) // 2
            thumb = img.crop((0, top, iw, top + nh))
        thumbs.append(B.preprocess(thumb).resize((B.STAGE_W, B.STAGE_H), Image.LANCZOS))
    master_q, _ = B.build_master_palette(thumbs)

    for name, (w, h, grid, desc) in SIZES.items():
        folder = OUT / name
        folder.mkdir(parents=True, exist_ok=True)
        n = 0
        for card_no in sorted(used):
            png = cache / f"{card_no}.png"
            if not png.exists():
                continue
            img = png_cache[card_no]
            if name == "detail":
                _pal_bytes, tiles = B.bake_detail(img)
                rgb = reconstruct_detail(tiles, _pal_bytes, w, h)
            elif name == "live":
                tiles = B.bake_live_sized_with_master(img, w, h, master_q, grid)
                rgb = reconstruct_8bpp(tiles, w, h, master_q, grid)
            else:
                tiles = B.bake_front_sized_with_master(img, w, h, master_q, grid)
                rgb = reconstruct_8bpp(tiles, w, h, master_q, grid)
            rgb.save(folder / f"{card_no}.png")
            n += 1
        gw, gh = grid if grid else (w // 8, h // 8)
        print(f"{name:7s} card={w}x{h} grid={gw}x{gh} {desc:18s} -> {n} pngs in {folder}")

    make_sheets(OUT, sorted(used))


def make_sheets(out_root, card_nos):
    """One labeled contact sheet per size folder."""
    for name, (w, h, grid, desc) in SIZES.items():
        folder = out_root / name
        pngs = [folder / f"{n}.png" for n in card_nos if (folder / f"{n}.png").exists()]
        if not pngs:
            continue
        # Cell size = the actual rendered PNG (grid dims), not card pixels
        sample = Image.open(pngs[0])
        cell_w, cell_h = sample.size
        cols = 8
        cell = max(cell_w, 16)
        label_h = 14
        rows = (len(pngs) + cols - 1) // cols
        sheet = Image.new("RGB", (cols * cell, rows * (cell + label_h)), (24, 24, 24))
        d = ImageDraw.Draw(sheet)
        for i, p in enumerate(pngs):
            im = Image.open(p).convert("RGB")
            r, c = divmod(i, cols)
            x, y = c * cell, r * (cell + label_h)
            sheet.paste(im, (x, y + label_h))
            d.text((x + 2, y + 2), p.stem[:22], fill=(255, 255, 120))
        sheet.save(out_root / f"sheet_{name}.png")
        print(f"sheet_{name}.png  {len(pngs)} cards  ({sheet.size[0]}x{sheet.size[1]})")


if __name__ == "__main__":
    main()