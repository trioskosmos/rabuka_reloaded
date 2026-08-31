#!/usr/bin/env python3
"""Bake GBA card art from the 3DS pipeline's PNG cache.

The 3DS already converts every card's WebP to a resized PNG cache and a
manifest (card_no -> image). We reuse that directly — no re-deriving the
card_no -> image mapping. Each card's PNG basename IS its cards.json card_no.

For each card referenced by any deck (same resolution as bake_deck_cards.py),
resize the cached PNG to a fixed portrait grid, quantize to 240 colours
(reserving palette bank 15 / indices 240-255 for the 4bpp text palette), and
emit platforms/gba/src/card_art_gen.rs with:

    pub struct CardArt { pub card_no: &'static str, pub palette: &'static [u8; 480], pub tiles: &'static [u8; 192*64] }
    pub static CARD_ART: &[CardArt] = &[ ... ];

Run:  py -3 tools/bake_card_art.py
"""

import json
import os
import sys
from pathlib import Path

from PIL import Image, ImageEnhance, ImageFilter

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "cards"))
sys.path.insert(0, str(REPO / "tools"))
from bake_deck_cards import normalize, REPO as _REPO  # noqa: E402

ART_W = 96  # 12 tiles
ART_H = 128  # 16 tiles
N_COLORS = 240  # leave indices 240-255 (bank 15) for the 4bpp text palette
TILE = 8

# On-board card fronts: 8bpp shared 240-colour master palette (bank 15
# reserved for text). All targets are portrait; bake preserves aspect via
# fit + edge-clamp so every zone respects the card's 0.708 aspect.
FRONT_W = 24  # hand cards, 3 tiles
FRONT_H = 32  # 4 tiles
STAGE_W = 40  # stage cards, 5 tiles
STAGE_H = 48  # 6 tiles  (letterboxed fit preserves 0.708)
LIVE_W = 16  # live/success zone mini, 2 tiles
LIVE_H = 24  # 3 tiles  (portrait)
FRONT_COLORS = 16

CACHE = REPO / "platforms" / "3ds" / ".card_png_cache"
OUT = REPO / "platforms" / "gba" / "src" / "card_art_gen.rs"


def deck_card_nos() -> set:
    """Union of normalized cards.json card_nos used by all decks."""
    cards_dict = json.loads((REPO / "cards" / "cards.json").read_text(encoding="utf-8"))
    by_no = {}
    for k, v in cards_dict.items():
        by_no.setdefault(normalize(k), k)  # normalized -> original cards.json key
    used = set()
    for f in sorted((REPO / "web_ui" / "decks").glob("*.txt")):
        for line in f.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line:
                continue
            card_no = None
            parts = line.split()
            if parts:
                if parts[0].isdigit() and len(parts) >= 2:
                    card_no = " ".join(parts[1:]).split("x")[0].strip()
                elif parts[-1].isdigit() and len(parts) >= 2:
                    card_no = " ".join(parts[:-2]) if len(parts) >= 2 else parts[0]
                else:
                    card_no = line
            if card_no:
                n = normalize(card_no)
                if n in by_no:
                    used.add(by_no[n])
    return used


def to_rgb15(r, g, b):
    return ((r >> 3) & 31) | (((g >> 3) & 31) << 5) | (((b >> 3) & 31) << 10)


def pack_4bpp_tiles(px, w, h, tiles_w, tiles_h):
    """Pack pixels (palette indices) into 4bpp tiles, ty-major then tx.

    Tile (tx, ty) starts at byte ((ty * tiles_w) + tx) * 16; within a tile the
    8x8 pixels are row-major with the LOW nibble holding the left pixel.
    """
    out = bytearray(tiles_w * tiles_h * 32)
    for ty in range(tiles_h):
        for tx in range(tiles_w):
            base = (ty * tiles_w + tx) * 32
            for rr in range(TILE):
                for cc in range(TILE):
                    v = px[tx * TILE + cc, ty * TILE + rr] & 0x0F
                    out[base + rr * (TILE // 2) + cc // 2] |= v if cc % 2 == 0 else v << 4
    return bytes(out)


def pack_8bpp_tiles(px, w, h, tiles_w, tiles_h):
    """Pack 8bpp palette indices into tiles, ty-major then tx. 64B per tile."""
    out = bytearray(tiles_w * tiles_h * 64)
    for ty in range(tiles_h):
        for tx in range(tiles_w):
            base = (ty * tiles_w + tx) * 64
            for rr in range(TILE):
                for cc in range(TILE):
                    out[base + rr * TILE + cc] = px[tx * TILE + cc, ty * TILE + rr] & 0xFF
    return bytes(out)


def palette_bytes_16(pal, n=16):
    """First n entries of a PIL palette as rgb15 little-endian bytes."""
    out = bytearray()
    for i in range(n):
        r, g, b = pal[i * 3], pal[i * 3 + 1], pal[i * 3 + 2]
        c = to_rgb15(r, g, b)
        out += bytes([c & 0xFF, c >> 8])
    return bytes(out)


def bake_ui_tiles():
    """Shared board UI tiles (bank-15 palette): single solid gray empty slot,
    gold badge, focus marker. Single tile repeated for all empty zones is
    VRAM-cheap (tonc `char block` advice: deduplicate)."""

    tiles = []

    # Single solid gray tile for empty zones (zone fill, color 2)
    tiles.append([2] * 64)

    # Actionable badge: gold diamond on transparent.
    badge = [[0] * 8 for _ in range(8)]
    for y in range(8):
        d = abs(y - 3.5)
        for x in range(8):
            if abs(x - 3.5) + d <= 3:
                badge[y][x] = 4
    tiles.append([badge[y][x] for y in range(8) for x in range(8)])

    # Focus marker: white right-pointing triangle on transparent.
    marker = [[0] * 8 for _ in range(8)]
    for y in range(8):
        for x in range(8):
            if x <= 3 + abs(y - 3.5) * 1.4:
                marker[y][x] = 1
    tiles.append([marker[y][x] for y in range(8) for x in range(8)])

    # Solid gold tile for hand cursor border (opaque)
    tiles.append([4] * 64)

    flat = bytearray()
    for t in tiles:
        for rr in range(TILE):
            for cc in range(0, TILE, 2):
                flat.append(t[rr * TILE + cc] | (t[rr * TILE + cc + 1] << 4))
    return bytes(flat)  # 4 tiles x 32 bytes


def preprocess(img):
    """Mild boost so detail survives 240-colour quantization without crushing darks."""
    img = ImageEnhance.Color(img).enhance(1.15)
    img = ImageEnhance.Contrast(img).enhance(1.08)
    return img


def darkest_index(pal):
    """Palette index of the darkest colour (for the baked outline)."""
    best, best_l = 0, 1e9
    for i in range(FRONT_COLORS):
        r, g, b = pal[i * 3], pal[i * 3 + 1], pal[i * 3 + 2]
        l = 0.3 * r + 0.6 * g + 0.1 * b
        if l < best_l:
            best_l, best = l, i
    return best


def build_master_palette(thumbnails):
    """Build a single 240-colour master palette from all thumbnails (as Tonc
    recommends: one shared 8bpp palette for a tiled BG, vs per-tile 16-colour
    banks). `thumbnails` is a list of RGB PIL Images. Returns
    (master_palette_image P mode, rgb15 bytes). Indices 240-255 are reserved
    for the text/UI palette (bank 15), matching `display.rs` detail view."""
    if not thumbnails:
        raise ValueError("no thumbnails for master palette")
    w = thumbnails[0].width
    total_h = sum(im.height for im in thumbnails)
    composite = Image.new("RGB", (w, total_h), (0, 0, 0))
    y = 0
    for im in thumbnails:
        x = (w - im.width) // 2
        composite.paste(im, (x, y))
        y += im.height
    # 240 colours for art (0-239), 240-255 reserved for text/UI bank 15.
    # Index 0 is forced to black so the 8bpp BG backdrop/transparent stays black.
    q = composite.quantize(colors=240, method=_QUANT_METHOD)
    pal = q.getpalette()
    pal[0:3] = bytes([0, 0, 0])
    q.putpalette(pal)
    pal = q.getpalette()[: 240 * 3]
    pal_bytes = bytearray()
    for i in range(240):
        r, g, b = pal[i * 3], pal[i * 3 + 1], pal[i * 3 + 2]
        c = to_rgb15(r, g, b)
        pal_bytes += bytes([c & 0xFF, c >> 8])
    return q, bytes(pal_bytes)


def _quantize_method():
    """Prefer libimagequant if available (far better than MEDIANCUT), else MAXCOVERAGE."""
    # PIL exposes these as ints on Image.Quantize; check availability
    for name in ("LIBIMAGEQUANT", "LIBIMAGEQUANT"):
        if hasattr(Image.Quantize, name):
            try:
                # probe: quantize a 1x1 image with it
                Image.new("RGB", (1, 1)).quantize(colors=2, method=getattr(Image.Quantize, name))
                return getattr(Image.Quantize, name)
            except Exception:
                pass
    if hasattr(Image.Quantize, "MAXCOVERAGE"):
        return Image.Quantize.MAXCOVERAGE
    return Image.Quantize.MEDIANCUT


_QUANT_METHOD = _quantize_method()

def bake_front_sized_with_master(img, w, h, master_q):
    """Fit one card image into wxh preserving aspect via black letterbox (not
    edge-stretch) so every zone respects the card's 0.708 aspect like 3DS.
    Bars are solid black (palette index 0) so the card never looks stretched."""
    iw, ih = img.size
    scale = min(w / iw, h / ih)
    nw, nh = int(iw * scale), int(ih * scale)
    small = preprocess(img).resize((nw, nh), Image.LANCZOS)
    canvas = Image.new("RGB", (w, h), (0, 0, 0))
    canvas.paste(small, ((w - nw) // 2, (h - nh) // 2))
    q = canvas.quantize(
        palette=master_q,
        dither=Image.Dither.FLOYDSTEINBERG,
    )
    px = q.load()
    tiles_w, tiles_h = w // TILE, h // TILE
    return pack_8bpp_tiles(px, w, h, tiles_w, tiles_h)


def bake_live_sized_with_master(img, w, h, master_q):
    """Live mini (16x24) - preserve aspect with black letterbox, not edge-stretch.
    Like 3DS, live cards are centered with correct 0.708 aspect; bars are solid
    black so the card never looks stretched. Uses same master palette."""
    iw, ih = img.size
    scale = min(w / iw, h / ih)
    nw, nh = int(iw * scale), int(ih * scale)
    small = preprocess(img).resize((nw, nh), Image.LANCZOS)
    # Optional light sharpen for tiny live thumbs (helps at 16x22)
    try:
        small = small.filter(ImageFilter.UnsharpMask(radius=0.8, percent=80, threshold=1))
    except Exception:
        pass
    canvas = Image.new("RGB", (w, h), (0, 0, 0))
    canvas.paste(small, ((w - nw) // 2, (h - nh) // 2))
    q = canvas.quantize(
        palette=master_q,
        dither=Image.Dither.FLOYDSTEINBERG,
    )
    px = q.load()
    tiles_w, tiles_h = w // TILE, h // TILE
    return pack_8bpp_tiles(px, w, h, tiles_w, tiles_h)


def bake_front_sized(img, w, h):
    """Cover-crop + resize one card image to a wxh portrait 4bpp front."""
    target = w / h
    iw, ih = img.size
    if iw / ih > target:  # too wide -> crop width
        nw = int(ih * target)
        left = (iw - nw) // 2
        img = img.crop((left, 0, left + nw, ih))
    else:  # too tall -> crop height
        nh = int(iw / target)
        top = (ih - nh) // 2
        img = img.crop((0, top, iw, top + nh))
    img = preprocess(img).resize((w, h), Image.LANCZOS)
    q = img.quantize(
        colors=FRONT_COLORS,
        method=Image.Quantize.MEDIANCUT,
        dither=Image.Dither.FLOYDSTEINBERG,
    )
    pal = q.getpalette()[: FRONT_COLORS * 3]
    px = q.load()
    # Baked 1px outline in the darkest palette colour so thumbs pop off the
    # board instead of bleeding into it.
    dark = darkest_index(pal)
    for xx in range(w):
        px[xx, 0] = dark
        px[xx, h - 1] = dark
    for yy in range(h):
        px[0, yy] = dark
        px[w - 1, yy] = dark
    tiles_w, tiles_h = w // TILE, h // TILE
    return palette_bytes_16(pal), pack_4bpp_tiles(px, w, h, tiles_w, tiles_h)


def bake_front(img):
    return bake_front_sized(img, FRONT_W, FRONT_H)


def bake_stage_front(img):
    return bake_front_sized(img, STAGE_W, STAGE_H)


def bake_detail(img):
    """Resize + quantize one card image to the 96x128 8bpp detail view.
    Index 0 forced to black so backdrop/transparent stays black, not white-on-white."""
    img = img.resize((ART_W, ART_H), Image.LANCZOS)
    q = img.quantize(colors=N_COLORS, method=_QUANT_METHOD)  # P mode
    pal = q.getpalette()
    pal[0:3] = bytes([0, 0, 0])
    q.putpalette(pal)
    pal = q.getpalette()[: N_COLORS * 3]
    pal_bytes = bytearray()
    for i in range(N_COLORS):
        r, g, b = pal[i * 3], pal[i * 3 + 1], pal[i * 3 + 2]
        c = to_rgb15(r, g, b)
        pal_bytes += bytes([c & 0xFF, c >> 8])
    # 8bpp tile indices (P-mode pixel values are palette indices)
    px = q.load()
    tiles = bytearray()
    for ty in range(ART_H // TILE):
        for tx in range(ART_W // TILE):
            tile = bytearray(64)
            for rr in range(TILE):
                for cc in range(TILE):
                    tile[rr * TILE + cc] = px[tx * TILE + cc, ty * TILE + rr] & 0xFF
            tiles += tile
    return bytes(pal_bytes), bytes(tiles)


def fronts_from(entries):
    """(card_no, front tiles) pairs from baked entries (8bpp shared palette)."""
    return [(no, ftiles) for no, _pal, _tiles, ftiles, _stage, _live in entries]


def stage_fronts_from(entries):
    """(card_no, stage-front tiles) pairs (8bpp shared palette)."""
    return [(no, stiles) for no, _pal, _tiles, _ftiles, stiles, _live in entries]


def live_fronts_from(entries):
    """(card_no, live-front tiles) pairs (8bpp shared palette)."""
    return [(no, ltiles) for no, _pal, _tiles, _ftiles, _stiles, ltiles in entries]


def write_bytes_array(f, name, data, per_line):
    f.write(f"        {name}: &[\n")
    for i in range(0, len(data), per_line):
        f.write("            " + ", ".join(str(b) for b in data[i:i + per_line]) + ",\n")
    f.write("        ],\n")


def write_gen(entries, fronts, stage_fronts, live_fronts, ui_tiles, master_pal_bytes):
    with open(OUT, "w", encoding="utf-8") as f:
        f.write("// Auto-generated by tools/bake_card_art.py -- do not edit.\n")
        f.write("// CardArt: 8bpp detail art (96x128 = 12x16 tiles) + 240-colour\n")
        f.write("// rgb15 palette (bank 15 reserved for text).\n")
        f.write("// Card fronts: 8bpp shared 240-colour master palette\n")
        f.write("// (tonc 8bpp BG + butano `bg_palette_items` sharing). Bank 15\n")
        f.write("// 240-255 reserved for text/UI.\n")
        f.write("// BOARD_UI: shared bank-15 board tiles (4bpp): 0-19 empty slot,\n")
        f.write("// 20 gold actionable badge, 21 white focus marker.\n\n")
        f.write("pub static MASTER_PAL: [u8; 480] = [\n")
        for i in range(0, len(master_pal_bytes), 24):
            f.write("    " + ", ".join(str(b) for b in master_pal_bytes[i:i + 24]) + ",\n")
        f.write("];\n\n")
        f.write("pub struct CardArt {\n")
        f.write("    pub card_no: &'static str,\n")
        f.write("    pub palette: &'static [u8; 480],\n")
        f.write("    pub tiles: &'static [u8; 12288],\n")
        f.write("}\n\n")
        f.write("pub struct CardFront {\n")
        f.write("    pub card_no: &'static str,\n")
        f.write("    pub tiles: &'static [u8],\n")
        f.write("}\n\n")
        f.write("pub static CARD_ART: &[CardArt] = &[\n")
        for card_no, pal, tiles, _fronts, _stage, _live in entries:
            f.write(f"    CardArt {{\n        card_no: {json.dumps(card_no, ensure_ascii=False)},\n")
            write_bytes_array(f, "palette", pal, 24)
            write_bytes_array(f, "tiles", tiles, 32)
            f.write("    },\n")
        f.write("];\n\n")
        f.write("pub static CARD_FRONTS: &[CardFront] = &[\n")
        for card_no, tiles in fronts:
            f.write(f"    CardFront {{ card_no: {json.dumps(card_no, ensure_ascii=False)}, tiles: &[\n")
            for i in range(0, len(tiles), 32):
                f.write("        " + ", ".join(str(b) for b in tiles[i:i + 32]) + ",\n")
            f.write("    ]},\n")
        f.write("];\n\n")
        f.write("pub static STAGE_FRONTS: &[CardFront] = &[\n")
        for card_no, tiles in stage_fronts:
            f.write(f"    CardFront {{ card_no: {json.dumps(card_no, ensure_ascii=False)}, tiles: &[\n")
            for i in range(0, len(tiles), 32):
                f.write("        " + ", ".join(str(b) for b in tiles[i:i + 32]) + ",\n")
            f.write("    ]},\n")
        f.write("];\n\n")
        f.write("pub static LIVE_FRONTS: &[CardFront] = &[\n")
        for card_no, tiles in live_fronts:
            f.write(f"    CardFront {{ card_no: {json.dumps(card_no, ensure_ascii=False)}, tiles: &[\n")
            for i in range(0, len(tiles), 32):
                f.write("        " + ", ".join(str(b) for b in tiles[i:i + 32]) + ",\n")
            f.write("    ]},\n")
        f.write("];\n\n")
        f.write(f"pub static BOARD_UI: &[u8; {len(ui_tiles)}] = &[\n")
        for i in range(0, len(ui_tiles), 32):
            f.write("    " + ", ".join(str(b) for b in ui_tiles[i:i + 32]) + ",\n")
        f.write("];\n")


def main():
    used = deck_card_nos()
    print(f"{len(used)} unique deck cards to bake")

    # First pass: build a shared 256-colour master palette from all
    # thumbnails (tonc/butano style: one 8bpp palette for a tiled BG).
    # Use stage-sized thumbs for palette generation (more pixels, richer colours).
    thumbs = []
    png_cache = {}
    for card_no in sorted(used):
        png = CACHE / f"{card_no}.png"
        if not png.exists():
            continue
        img = Image.open(png).convert("RGB")
        png_cache[card_no] = img
        # cover-crop + preprocess + resize to stage size, as bake will do
        target = STAGE_W / STAGE_H
        iw, ih = img.size
        if iw / ih > target:
            nw = int(ih * target)
            left = (iw - nw) // 2
            thumb = img.crop((left, 0, left + nw, ih))
        else:
            nh = int(iw / target)
            top = (ih - nh) // 2
            thumb = img.crop((0, top, iw, top + nh))
        thumb = preprocess(thumb).resize((STAGE_W, STAGE_H), Image.LANCZOS)
        thumbs.append(thumb)
    master_q, master_pal_bytes = build_master_palette(thumbs)
    print(f"master palette: 240 colours from {len(thumbs)} thumbs")

    entries = []
    missing = []
    for card_no in sorted(used):
        png = CACHE / f"{card_no}.png"
        if not png.exists():
            missing.append(card_no)
            continue
        img = png_cache[card_no]
        entries.append(
            (card_no,)
            + bake_detail(img)
            + (
                bake_front_sized_with_master(img, FRONT_W, FRONT_H, master_q),
                bake_front_sized_with_master(img, STAGE_W, STAGE_H, master_q),
                bake_live_sized_with_master(img, LIVE_W, LIVE_H, master_q),
            )
        )

    print(f"baked {len(entries)} cards, missing {len(missing)}")
    if missing:
        print("missing:", missing[:20])

    ui_tiles = bake_ui_tiles()
    write_gen(entries, fronts_from(entries), stage_fronts_from(entries), live_fronts_from(entries), ui_tiles, master_pal_bytes)
    print(f"wrote {OUT} ({os.path.getsize(OUT)} bytes)")


if __name__ == "__main__":
    main()
