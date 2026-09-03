#!/usr/bin/env python3
"""Bake GBA card art from original WebP sources.

For each card referenced by any deck:
- Detail view: 8bpp per-card 240-colour palette (96x144)
- Fronts (hand/stage/live/waited): 4bpp per-card 16-colour palettes (banks 0-14)

Emits platforms/gba/src/card_art_gen.rs with:
    pub struct CardArt { pub card_no: &'static str, pub palette: &'static [u8; 480], pub tiles: &'static [u8; 13824] }
    pub struct CardFront { pub card_no: &'static str, pub palette: &'static [u8; 32], pub tiles: &'static [u8] }
    pub static CARD_ART: &[CardArt] = &[ ... ];
    pub static CARD_FRONTS: &[CardFront] = &[ ... ];
    etc.

Run:  py -3 tools/bake_card_art.py
"""

import json
import os
import re
import sys
from pathlib import Path

from PIL import Image, ImageFilter

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "cards"))
sys.path.insert(0, str(REPO / "tools"))
from bake_deck_cards import normalize, REPO as _REPO  # noqa: E402

ART_W = 96  # 12 tiles - detail portrait fills the height next to the text pane
ART_H = 144  # 18 tiles
N_COLORS = 240  # leave indices 240-255 (bank 15) for the 4bpp text palette
TILE = 8

# On-board card fronts: 4bpp per-card 16-colour palettes (banks 0-14).
# Detail: 8bpp per-card 240-colour palette.
FRONT_W = 24  # hand card pixels, 3 tiles
FRONT_H = 32
FRONT_GRID = (3, 4)
STAGE_W = 34  # stage card pixels (0.708, 1% off source) in 5 tiles
STAGE_H = 48
STAGE_GRID = (5, 6)
LIVE_W = 22  # live card pixels, landscape (1.375) in 3 tiles
LIVE_H = 16
LIVE_GRID = (3, 2)
WAIT_W = 32  # wait state: 90° rotated portrait = landscape 32x24 in 4x3 grid
WAIT_H = 24
WAIT_GRID = (4, 3)
FRONT_COLORS = 16

DUMMY_RGB = (255, 0, 255)  # bright magenta - won't appear in card art, forced to index 0 (transparent)
PAD_RGB = (26, 35, 50)  # dark blue - board backdrop, must NOT be index 0

BACK_PNG = REPO / "web_ui" / "img" / "texticon" / "lltcg-back.png"
BACK_GRID = (3, 2)  # live-slot geometry: backed-out slots reuse live layout

CACHE = REPO / "web_ui" / "img" / "cards_webp"
OUT = REPO / "platforms" / "gba" / "src" / "card_art_gen.rs"


def deck_card_nos() -> set:
    """Union of normalized cards.json card_nos used by all decks."""
    cards_dict = json.loads((REPO / "cards" / "cards.json").read_text(encoding="utf-8"))
    by_no = {}
    for k, v in cards_dict.items():
        by_no.setdefault(normalize(k), k)
    used = set()
    for f in sorted((REPO / "web_ui" / "decks").glob("*.txt")):
        for line in f.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line:
                continue
            # Formats: "count x card_no", "card_no x count", or bare card_no
            m = re.match(r"^(\d+)\s*x\s*(.+)$", line)
            if m:
                n = normalize(m.group(2).strip())
                if n in by_no:
                    used.add(by_no[n])
                continue
            m = re.match(r"^(.*?)\s*x\s*(\d+)$", line)
            if m:
                n = normalize(m.group(1).strip())
                if n in by_no:
                    used.add(by_no[n])
                continue
            # Bare card_no
            n = normalize(line)
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


def darkest_index(pal):
    """Palette index of the darkest colour (for the baked outline)."""
    best, best_l = 0, 1e9
    for i in range(FRONT_COLORS):
        r, g, b = pal[i * 3], pal[i * 3 + 1], pal[i * 3 + 2]
        l = 0.3 * r + 0.6 * g + 0.1 * b
        if l < best_l:
            best_l, best = l, i
    return best


def _quantize_method():
    """Prefer libimagequant if available, else MAXCOVERAGE."""
    for name in ("LIBIMAGEQUANT", "LIBIMAGEQUANT"):
        if hasattr(Image.Quantize, name):
            try:
                Image.new("RGB", (1, 1)).quantize(colors=2, method=getattr(Image.Quantize, name))
                return getattr(Image.Quantize, name)
            except Exception:
                pass
    if hasattr(Image.Quantize, "MAXCOVERAGE"):
        return Image.Quantize.MAXCOVERAGE
    return Image.Quantize.MEDIANCUT


_QUANT_METHOD = _quantize_method()


def build_master_palette(thumbnails):
    """Build a single 240-colour master palette from all thumbnails.
    Returns (master_palette_image P mode, rgb15 bytes). Index 0 forced to PAD_RGB."""
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
    q = composite.quantize(colors=240, method=_QUANT_METHOD)
    pal = q.getpalette()
    pal[0:3] = bytes(PAD_RGB)  # index 0 = board backdrop
    q.putpalette(pal)
    pal = q.getpalette()[: 240 * 3]
    pal_bytes = bytearray()
    for i in range(240):
        r, g, b = pal[i * 3], pal[i * 3 + 1], pal[i * 3 + 2]
        c = to_rgb15(r, g, b)
        pal_bytes += bytes([c & 0xFF, c >> 8])
    return q, bytes(pal_bytes)


def build_palette(thumbnails, colors=240):
    """Build a palette from thumbnails. Returns (palette_image P mode, rgb15 bytes).
    Index 0 = DUMMY_RGB (bright magenta) - never in card art, becomes transparent on GBA.
    Index 1 = PAD_RGB (dark blue) - padding color for board backdrop.
    Indices 2-239 = card art colors."""
    if not thumbnails:
        raise ValueError("no thumbnails for palette")
    w = thumbnails[0].width
    total_h = sum(im.height for im in thumbnails)
    # Composite: DUMMY_RGB background (most frequent → index 0 = transparent)
    # 64px dummy zone on left, thumbnails shifted right by 64px
    composite = Image.new("RGB", (w + 64, total_h), DUMMY_RGB)
    y = 0
    for im in thumbnails:
        # Shift thumbnails right by 64px, centered within their grid
        composite.paste(im, (64 + (w - im.width) // 2, y))
        y += im.height
    # Force index 0 = DUMMY_RGB by adding MASSIVE dummy pixels at (0,0) to (63,127)
    # This makes DUMMY_RGB the most frequent color = index 0 (transparent on GBA)
    for y in range(128):
        for x in range(64):
            composite.putpixel((x, y), DUMMY_RGB)
    # Ensure PAD_RGB is in palette at index 1 (padding color)
    composite.putpixel((64, 0), PAD_RGB)
    q = composite.quantize(colors=colors, method=_QUANT_METHOD)
    pal = q.getpalette()[: colors * 3]
    pal_bytes = bytearray()
    for i in range(colors):
        r, g, b = pal[i * 3], pal[i * 3 + 1], pal[i * 3 + 2]
        c = to_rgb15(r, g, b)
        pal_bytes += bytes([c & 0xFF, c >> 8])
    # Create palette image for quantizing card art
    q = Image.new("P", (1, 1))
    q.putpalette(pal[:768])
    return q, bytes(pal_bytes)

def bake_with_palette(img, w, h, palette_q, grid=None, dither=Image.Dither.FLOYDSTEINBERG, sharpen=False):
    """Generic contain-fit + quantize with given palette."""
    gw, gh = grid if grid else (w // TILE, h // TILE)
    iw, ih = img.size
    scale = min(w / iw, h / ih)
    nw, nh = max(1, int(iw * scale)), max(1, int(ih * scale))
    small = img.resize((nw, nh), Image.LANCZOS)
    if sharpen:
        try:
            small = small.filter(ImageFilter.UnsharpMask(radius=0.5, percent=30, threshold=1))
        except Exception:
            pass
    canvas = Image.new("RGB", (gw * TILE, gh * TILE), PAD_RGB)
    canvas.paste(small, ((gw * TILE - nw) // 2, (gh * TILE - nh) // 2))
    q = canvas.quantize(palette=palette_q, dither=dither)
    px = q.load()
    return pack_8bpp_tiles(px, gw * TILE, gh * TILE, gw, gh)


def bake_front_sized_with_master(img, w, h, palette_q, grid=None):
    """Hand/Stage front: no sharpen, Floyd-Steinberg dither."""
    return bake_with_palette(img, w, h, palette_q, grid, dither=Image.Dither.FLOYDSTEINBERG, sharpen=False)


def bake_live_sized_with_master(img, w, h, palette_q, grid=None):
    """Live mini: mild sharpen, ordered dither for small size."""
    return bake_with_palette(img, w, h, palette_q, grid, dither=Image.Dither.ORDERED, sharpen=True)


def bake_back_front(master_q):
    """Card back at live-slot geometry, quantized into the shared master
    palette (its colors join palette sampling in main). Used for face-down
    live-set slots, mirroring the 3DS/web card-back display."""
    img = Image.open(BACK_PNG).convert("RGB")
    return bake_live_sized_with_master(img, LIVE_W, LIVE_H, master_q, BACK_GRID)


def bake_waited_sized_with_master(img, w, h, palette_q, grid=None):
    """Wait state (tapped) - object-fit: contain, rotated 90° CW."""
    gw, gh = grid if grid else (w // TILE, h // TILE)
    iw, ih = img.size
    rotated = img.rotate(-90, expand=True)  # -90 = CW
    riw, rih = rotated.size
    scale = min((gw * TILE) / riw, (gh * TILE) / rih)
    nw, nh = max(1, int(riw * scale)), max(1, int(rih * scale))
    small = rotated.resize((nw, nh), Image.LANCZOS)
    canvas = Image.new("RGB", (gw * TILE, gh * TILE), PAD_RGB)
    canvas.paste(small, ((gw * TILE - nw) // 2, (gh * TILE - nh) // 2))
    q = canvas.quantize(palette=palette_q, dither=Image.Dither.ORDERED)
    px = q.load()
    return pack_8bpp_tiles(px, gw * TILE, gh * TILE, gw, gh)


def bake_detail(img, palette_q, palette_bytes):
    """Resize + quantize one card image to the 96x144 8bpp detail view.
    object-fit: contain — the whole card fits, padded with black (index 0),
    so nothing is cropped. Landscape cards get side bars instead of being
    squashed or trimmed."""
    iw, ih = img.size
    scale = min(ART_W / iw, ART_H / ih)
    nw, nh = max(1, int(iw * scale)), max(1, int(ih * scale))
    small = img.resize((nw, nh), Image.LANCZOS)
    canvas = Image.new("RGB", (ART_W, ART_H), (0, 0, 0))
    canvas.paste(small, ((ART_W - nw) // 2, (ART_H - nh) // 2))
    q = canvas.quantize(palette=palette_q, dither=Image.Dither.FLOYDSTEINBERG)
    px = q.load()
    tiles = bytearray()
    for ty in range(ART_H // TILE):
        for tx in range(ART_W // TILE):
            tile = bytearray(64)
            for rr in range(TILE):
                for cc in range(TILE):
                    tile[rr * TILE + cc] = px[tx * TILE + cc, ty * TILE + rr] & 0xFF
            tiles += tile
    return bytes(palette_bytes), bytes(tiles)


def fronts_from(entries):
    """(card_no, tiles) pairs for 8bpp shared-palette fronts."""
    return [(no, ftiles) for no, _pal, _tiles, ftiles, _stage, _live, _wait in entries]


def stage_fronts_from(entries):
    """(card_no, tiles) pairs for 8bpp shared-palette stage fronts."""
    return [(no, stiles) for no, _pal, _tiles, _front, stiles, _live, _wait in entries]


def live_fronts_from(entries):
    """(card_no, tiles) pairs for 8bpp shared-palette live fronts."""
    return [(no, ltiles) for no, _pal, _tiles, _front, _stage, ltiles, _wait in entries]


def waited_fronts_from(entries):
    """(card_no, tiles) pairs for 8bpp shared-palette waited fronts."""
    return [(no, wtiles) for no, _pal, _tiles, _front, _stage, _live, wtiles in entries]


def write_bytes_array(f, name, data, per_line):
    f.write(f"        {name}: &[\n")
    for i in range(0, len(data), per_line):
        f.write("            " + ", ".join(str(b) for b in data[i:i + per_line]) + ",\n")
    f.write("        ],\n")


def write_gen(entries, fronts, stage_fronts, live_fronts, waited_fronts, back_front, ui_tiles, master_pal_bytes):
    with open(OUT, "w", encoding="utf-8") as f:
        f.write("// Auto-generated by tools/bake_card_art.py -- do not edit.\n")
        f.write("// CardArt: 8bpp detail art (96x144 = 12x18 tiles)\n")
        f.write("// + 240-colour rgb15 palette (bank 15 reserved for text).\n")
        f.write("// Card fronts: 8bpp shared 240-colour MASTER_PAL.\n")
        f.write("// Detail: 8bpp per-card 240-colour palette.\n")
        f.write("// BOARD_UI: shared bank-15 board tiles (4bpp): 0 empty fill,\n")
        f.write("// 1 gold actionable badge, 2 white focus marker, 3 solid gold.\n\n")
        f.write("pub static MASTER_PAL: [u8; 480] = [\n")
        for i in range(0, len(master_pal_bytes), 24):
            f.write("    " + ", ".join(str(b) for b in master_pal_bytes[i:i + 24]) + ",\n")
        f.write("];\n\n")
        f.write("pub struct CardArt {\n")
        f.write("    pub card_no: &'static str,\n")
        f.write("    pub palette: &'static [u8; 480],\n")
        f.write("    pub tiles: &'static [u8; 13824],\n")
        f.write("}\n\n")
        f.write("pub struct CardFront {\n")
        f.write("    pub card_no: &'static str,\n")
        f.write("    pub tiles: &'static [u8],\n")
        f.write("}\n\n")
        f.write("pub static CARD_ART: &[CardArt] = &[\n")
        for card_no, pal, tiles, _fronts, _stage, _live, _wait in entries:
            f.write(f"    CardArt {{\n        card_no: {json.dumps(card_no, ensure_ascii=False)},\n")
            write_bytes_array(f, "palette", pal, 24)
            write_bytes_array(f, "tiles", tiles, 32)
            f.write("    },\n")
        f.write("];\n\n")
        f.write("pub static CARD_FRONTS: &[CardFront] = &[\n")
        for card_no, tiles in fronts:
            f.write(f"    CardFront {{ card_no: {json.dumps(card_no, ensure_ascii=False)},\n")
            write_bytes_array(f, "tiles", tiles, 32)
            f.write("    },\n")
        f.write("];\n\n")
        f.write("pub static STAGE_FRONTS: &[CardFront] = &[\n")
        for card_no, tiles in stage_fronts:
            f.write(f"    CardFront {{ card_no: {json.dumps(card_no, ensure_ascii=False)},\n")
            write_bytes_array(f, "tiles", tiles, 32)
            f.write("    },\n")
        f.write("];\n\n")
        f.write("pub static LIVE_FRONTS: &[CardFront] = &[\n")
        for card_no, tiles in live_fronts:
            f.write(f"    CardFront {{ card_no: {json.dumps(card_no, ensure_ascii=False)},\n")
            write_bytes_array(f, "tiles", tiles, 32)
            f.write("    },\n")
        f.write("];\n\n")
        f.write("pub static WAITED_FRONTS: &[CardFront] = &[\n")
        for card_no, tiles in waited_fronts:
            f.write(f"    CardFront {{ card_no: {json.dumps(card_no, ensure_ascii=False)},\n")
            write_bytes_array(f, "tiles", tiles, 32)
            f.write("    },\n")
        f.write("];\n\n")
        f.write("pub static BACK_FRONT: &[u8] = &[\n")
        for i in range(0, len(back_front), 32):
            f.write("    " + ", ".join(str(b) for b in back_front[i:i + 32]) + ",\n")
        f.write("];\n\n")
        f.write(f"pub static BOARD_UI: &[u8; {len(ui_tiles)}] = &[\n")
        for i in range(0, len(ui_tiles), 32):
            f.write("    " + ", ".join(str(b) for b in ui_tiles[i:i + 32]) + ",\n")
        f.write("];\n")


def make_thumb(img, w, h):
    """Cover-crop + resize to wxh for palette sampling."""
    target = w / h
    iw, ih = img.size
    if iw / ih > target:
        nw = int(ih * target)
        left = (iw - nw) // 2
        return img.crop((left, 0, left + nw, ih))
    else:
        nh = int(iw / target)
        top = (ih - nh) // 2
        return img.crop((0, top, iw, top + nh))


def main():
    used = deck_card_nos()
    print(f"{len(used)} unique deck cards to bake")

    # Load all images once from original WebP
    webp_cache = {}
    for card_no in sorted(used):
        webp = CACHE / f"{card_no}.webp"
        if webp.exists():
            webp_cache[card_no] = Image.open(webp).convert("RGB")

    # Build ONE shared master palette from ALL thumbs at their target sizes
    thumbs = []
    for card_no in sorted(used):
        img = webp_cache.get(card_no)
        if not img:
            continue
        for w, h, grid in [
            (FRONT_W, FRONT_H, FRONT_GRID),   # hand
            (STAGE_W, STAGE_H, STAGE_GRID),   # stage
            (LIVE_W, LIVE_H, LIVE_GRID),      # live
            (WAIT_W, WAIT_H, WAIT_GRID),      # waited
        ]:
            thumb = make_thumb(img, w, h).resize((grid[0]*TILE, grid[1]*TILE), Image.LANCZOS)
            thumbs.append(thumb)
    # Card-back colors join the shared palette so the facedown front is faithful.
    if BACK_PNG.exists():
        thumbs.append(
            Image.open(BACK_PNG).convert("RGB").resize((LIVE_W, LIVE_H), Image.LANCZOS)
        )
    master_q, master_pal = build_palette(thumbs, colors=240)
    print(f"master palette: 240 colours from {len(thumbs)} thumbs")

    entries = []
    missing = []
    for card_no in sorted(used):
        webp = CACHE / f"{card_no}.webp"
        if not webp.exists():
            missing.append(card_no)
            continue
        img = webp_cache[card_no]
        entries.append(
            (card_no,)
            + bake_detail(img, master_q, master_pal)
            + (
                # Hand minis use the small-size recipe (sharpen + ordered
                # dither) like the other small assets, not the stage recipe.
                bake_live_sized_with_master(img, FRONT_W, FRONT_H, master_q, FRONT_GRID),
                bake_front_sized_with_master(img, STAGE_W, STAGE_H, master_q, STAGE_GRID),
                bake_live_sized_with_master(img, LIVE_W, LIVE_H, master_q, LIVE_GRID),
                bake_waited_sized_with_master(img, WAIT_W, WAIT_H, master_q, WAIT_GRID),
            )
        )

    print(f"baked {len(entries)} cards, missing {len(missing)}")
    if missing:
        print("missing:", missing[:20])

    ui_tiles = bake_ui_tiles()
    back_front = bake_back_front(master_q)
    write_gen(entries, fronts_from(entries), stage_fronts_from(entries), live_fronts_from(entries), waited_fronts_from(entries), back_front, ui_tiles, master_pal)
    print(f"wrote {OUT} ({os.path.getsize(OUT)} bytes)")


if __name__ == "__main__":
    main()
