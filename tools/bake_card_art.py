#!/usr/bin/env python3
"""Bake GBA card art from original WebP sources — using include_bytes! binary files.

For each card:
- Detail view: 8bpp per-card 240-colour palette (96x144 = 13824 bytes tiles + 480 palette)
- Fronts (hand/stage/live/waited): 8bpp shared 240-colour MASTER_PAL

Emits platforms/gba/src/card_art_gen.rs with:
    pub static MASTER_PAL: [u8; 480] = *include_bytes!("../baked/card_art/master_pal.bin");
    pub static CARD_ART: &[CardArt] = &[ CardArt { card_no: "PL!-BP1-001-R", 
        palette: include_bytes!("../baked/card_art/pal_PL!-BP1-001-R.bin"),
        tiles: include_bytes!("../baked/card_art/tiles_PL!-BP1-001-R.bin") }, ... ];
    pub static CARD_FRONTS: &[CardFront] = &[ CardFront { card_no: "...", tiles: include_bytes!(...) }, ... ];

Run:  py -3 tools/bake_card_art.py
"""

import json
import os
import random
import re
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

from PIL import Image, ImageFilter

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "cards"))
from bake_deck_cards import normalize  # noqa: E402

ART_W = 96
ART_H = 144
N_COLORS = 240  # leave indices 240-255 (bank 15) for the 4bpp text palette
TILE = 8

# Front geometries (all use shared 8bpp MASTER_PAL now)
FRONT_W = 24
FRONT_H = 32
FRONT_GRID = (3, 4)
STAGE_W = 34
STAGE_H = 48
STAGE_GRID = (5, 6)
LIVE_W = 22
LIVE_H = 16
LIVE_GRID = (3, 2)
WAIT_W = 32
WAIT_H = 24
WAIT_GRID = (4, 3)
FRONT_COLORS = 16

DUMMY_RGB = (255, 0, 255)
PAD_RGB = (26, 35, 50)

BACK_PNG = REPO / "web_ui" / "img" / "texticon" / "lltcg-back.png"
BACK_GRID = (3, 2)

CACHE = REPO / "web_ui" / "img" / "cards_webp"
OUT_RS = REPO / "platforms" / "gba" / "src" / "card_art_gen.rs"
BIN_DIR = REPO / "platforms" / "gba" / "baked" / "card_art"
MAX_WORKERS = 8  # CPU cores


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


def load_image(card_no: str):
    """Load a single image on demand."""
    webp = CACHE / f"{card_no}.webp"
    if webp.exists():
        return card_no, Image.open(webp).convert("RGB")
    return card_no, None


def to_rgb15(r, g, b):
    return ((r >> 3) & 31) | (((g >> 3) & 31) << 5) | (((b >> 3) & 31) << 10)


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


def palette_bytes_240(pal):
    """240 entries of a PIL palette as rgb15 little-endian bytes."""
    return palette_bytes_16(pal, 240)


def build_palette(thumbs, colors=240):
    """Build shared 240-colour palette from thumbnails."""
    contact = Image.new("RGB", (sum(t.width for t in thumbs), max(t.height for t in thumbs)))
    x = 0
    for t in thumbs:
        contact.paste(t, (x, 0))
        x += t.width
    q = contact.quantize(colors=colors, method=Image.Quantize.MAXCOVERAGE)
    return q, q.getpalette()


def bake_with_palette(img, w, h, palette_q, grid=None, dither=Image.Dither.FLOYDSTEINBERG, sharpen=False):
    """Resize + quantize to w,h using given palette."""
    if sharpen:
        try:
            img = img.filter(ImageFilter.UnsharpMask(radius=0.5, percent=30, threshold=1))
        except Exception:
            pass
    gw, gh = grid if grid else (w // TILE, h // TILE)
    iw, ih = img.size
    scale = min((gw * TILE) / iw, (gh * TILE) / ih)
    nw, nh = max(1, int(iw * scale)), max(1, int(ih * scale))
    small = img.resize((nw, nh), Image.LANCZOS)
    canvas = Image.new("RGB", (gw * TILE, gh * TILE), PAD_RGB)
    canvas.paste(small, ((gw * TILE - nw) // 2, (gh * TILE - nh) // 2))
    q = canvas.quantize(palette=palette_q, dither=dither)
    px = q.load()
    return pack_8bpp_tiles(px, gw * TILE, gh * TILE, gw, gh)


def maybe_upright(img):
    """Live-card art is landscape; rotate 90° CCW for portrait boxes."""
    if img.width > img.height:
        return img.rotate(90, expand=True), True
    return img, False


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


def bake_detail(img, palette_q, palette_bytes):
    """96x144 8bpp detail view."""
    img, _ = maybe_upright(img)
    iw, ih = img.size
    scale = min(ART_W / iw, ART_H / ih)
    nw, nh = max(1, int(iw * scale)), max(1, int(ih * scale))
    small = img.resize((nw, nh), Image.LANCZOS)
    canvas = Image.new("RGB", (ART_W, ART_H), (0, 0, 0))
    canvas.paste(small, ((ART_W - nw) // 2, (ART_H - nh) // 2))
    q = canvas.quantize(palette=palette_q, dither=Image.Dither.FLOYDSTEINBERG)
    px = q.load()
    tiles = pack_8bpp_tiles(px, ART_W, ART_H, ART_W // TILE, ART_H // TILE)
    return bytes(palette_bytes), tiles


def bake_back_front(master_q):
    """Card back at live-slot geometry."""
    img = Image.open(BACK_PNG).convert("RGB")
    return bake_with_palette(img, LIVE_W, LIVE_H, master_q, BACK_GRID, dither=Image.Dither.ORDERED)


def bake_ui_tiles():
    """Shared board UI tiles (bank-15 palette): 6 tiles x 32 bytes = 192 bytes."""
    tiles = []

    # 0: solid gray empty zone fill (color 2)
    tiles.append(bytes([2] * 64))

    # 1: gold diamond actionable badge (color 4 on transparent 0)
    badge = [[0] * 8 for _ in range(8)]
    for y in range(8):
        d = abs(y - 3.5)
        for x in range(8):
            if abs(x - 3.5) + d <= 3:
                badge[y][x] = 4
    flat = bytearray()
    for y in range(8):
        for x in range(0, 8, 2):
            flat.append(badge[y][x] | (badge[y][x + 1] << 4))
    tiles.append(bytes(flat))

    # 2: white right-pointing triangle focus marker (color 1)
    marker = [[0] * 8 for _ in range(8)]
    for y in range(8):
        for x in range(8):
            if x <= 3 + abs(y - 3.5) * 1.4:
                marker[y][x] = 1
    flat = bytearray()
    for y in range(8):
        for x in range(0, 8, 2):
            flat.append(marker[y][x] | (marker[y][x + 1] << 4))
    tiles.append(bytes(flat))

    # 3: solid gold (color 4) for hand cursor border
    tiles.append(bytes([4] * 64))

    # 4: fully transparent (color 0) for clearing front text BG
    tiles.append(bytes([0] * 64))

    # 5: edge badge - gold diamond nudged right
    edge = [[0] * 8 for _ in range(8)]
    for y in range(8):
        d = abs(y - 3.5)
        for x in range(8):
            if abs(x - 6.5) + d <= 3:
                edge[y][x] = 4
    flat = bytearray()
    for y in range(8):
        for x in range(0, 8, 2):
            flat.append(edge[y][x] | (edge[y][x + 1] << 4))
    tiles.append(bytes(flat))

    return b"".join(tiles)  # 6 tiles x 32 bytes = 192 bytes


def sanitize_filename(card_no: str) -> str:
    """Convert card_no to safe filename."""
    return card_no.replace("!", "").replace("+", "p").replace("＋", "p").replace("-", "_")


def build_master_palette(card_nos: list[str], sample_size: int = 500) -> tuple:
    """Build master palette from a representative sample of cards."""
    sample = random.sample(card_nos, min(sample_size, len(card_nos)))
    thumbs = []
    for card_no in sample:
        result = load_image(card_no)
        if result[1] is None:
            continue
        img = result[1]
        for w, h, grid in [
            (FRONT_W, FRONT_H, FRONT_GRID),
            (STAGE_W, STAGE_H, STAGE_GRID),
            (LIVE_W, LIVE_H, LIVE_GRID),
            (WAIT_W, WAIT_H, WAIT_GRID),
        ]:
            thumb = make_thumb(img, w, h).resize((grid[0]*TILE, grid[1]*TILE), Image.LANCZOS)
            thumbs.append(thumb)
    if BACK_PNG.exists():
        thumbs.append(
            Image.open(BACK_PNG).convert("RGB").resize((LIVE_W, LIVE_H), Image.LANCZOS)
        )
    master_q, master_pal = build_palette(thumbs, colors=240)
    master_pal_bytes = palette_bytes_240(master_pal)
    print(f"master palette: 240 colours from {len(thumbs)} thumbs (sample of {len(sample)} cards)")
    return master_q, master_pal_bytes


def process_card(card_no: str, master_q, master_pal_bytes):
    """Process a single card - returns all baked data."""
    webp = CACHE / f"{card_no}.webp"
    if not webp.exists():
        return card_no, None
    img = Image.open(webp).convert("RGB")
    up, rotated = maybe_upright(img)
    pal, tiles = bake_detail(img, master_q, master_pal_bytes)
    ftiles = bake_with_palette(up, FRONT_W, FRONT_H, master_q, FRONT_GRID, dither=Image.Dither.FLOYDSTEINBERG, sharpen=True)
    stiles = bake_with_palette(up, STAGE_W, STAGE_H, master_q, STAGE_GRID, dither=Image.Dither.FLOYDSTEINBERG)
    ltiles = bake_with_palette(img, LIVE_W, LIVE_H, master_q, LIVE_GRID, dither=Image.Dither.ORDERED, sharpen=True)
    wtiles = bake_with_palette(img, WAIT_W, WAIT_H, master_q, WAIT_GRID, dither=Image.Dither.ORDERED)
    return card_no, (pal, tiles, ftiles, stiles, ltiles, wtiles, rotated)


def write_gen(entries, fronts, stage_fronts, live_fronts, waited_fronts, back_front, ui_tiles, master_pal_bytes):
    """Generate card_art_gen.rs with include_bytes! references."""
    BIN_DIR.mkdir(parents=True, exist_ok=True)

    # Write binary files
    with open(BIN_DIR / "master_pal.bin", "wb") as f:
        f.write(master_pal_bytes)

    for card_no, pal, tiles, _f, _s, _l, _w in entries:
        safe = sanitize_filename(card_no)
        with open(BIN_DIR / f"pal_{safe}.bin", "wb") as f:
            f.write(pal)
        with open(BIN_DIR / f"tiles_{safe}.bin", "wb") as f:
            f.write(tiles)

    for card_no, tiles in fronts:
        safe = sanitize_filename(card_no)
        with open(BIN_DIR / f"front_{safe}.bin", "wb") as f:
            f.write(tiles)

    for card_no, tiles in stage_fronts:
        safe = sanitize_filename(card_no)
        with open(BIN_DIR / f"stage_{safe}.bin", "wb") as f:
            f.write(tiles)

    for card_no, tiles in live_fronts:
        safe = sanitize_filename(card_no)
        with open(BIN_DIR / f"live_{safe}.bin", "wb") as f:
            f.write(tiles)

    for card_no, tiles in waited_fronts:
        safe = sanitize_filename(card_no)
        with open(BIN_DIR / f"wait_{safe}.bin", "wb") as f:
            f.write(tiles)

    with open(BIN_DIR / "back_front.bin", "wb") as f:
        f.write(back_front)

    with open(BIN_DIR / "board_ui.bin", "wb") as f:
        f.write(ui_tiles)

    # Generate Rust source with include_bytes!
    with open(OUT_RS, "w", encoding="utf-8") as f:
        f.write("// Auto-generated by tools/bake_card_art.py -- do not edit.\n")
        f.write("// CardArt: 8bpp detail art (96x144 = 13824 bytes) + 240-colour rgb15 palette\n")
        f.write("// Card fronts: 8bpp shared MASTER_PAL (4bpp on GBA via palette bank)\n")
        f.write("// All binary data loaded via include_bytes! from ../baked/card_art/\n\n")

        f.write("pub static MASTER_PAL: [u8; 480] = *include_bytes!(\"../baked/card_art/master_pal.bin\");\n\n")

        f.write("pub struct CardArt {\n")
        f.write("    pub card_no: &'static str,\n")
        f.write("    pub palette: &'static [u8; 480],\n")
        f.write("    pub tiles: &'static [u8; 13824],\n")
        f.write("}\n\n")

        f.write("pub struct CardFront {\n")
        f.write("    pub card_no: &'static str,\n")
        f.write("    pub tiles: &'static [u8],\n")
        f.write("}\n\n")

        # CARD_ART
        f.write("pub static CARD_ART: &[CardArt] = &[\n")
        for card_no, _pal, _tiles, _f, _s, _l, _w in entries:
            safe = sanitize_filename(card_no)
            f.write(f"    CardArt {{\n")
            f.write(f'        card_no: "{card_no}",\n')
            f.write(f'        palette: include_bytes!("../baked/card_art/pal_{safe}.bin"),\n')
            f.write(f'        tiles: include_bytes!("../baked/card_art/tiles_{safe}.bin"),\n')
            f.write("    },\n")
        f.write("];\n\n")

        # CARD_FRONTS (hand)
        f.write("pub static CARD_FRONTS: &[CardFront] = &[\n")
        for card_no, _tiles in fronts:
            safe = sanitize_filename(card_no)
            f.write(f"    CardFront {{ card_no: \"{card_no}\",\n")
            f.write(f'        tiles: include_bytes!("../baked/card_art/front_{safe}.bin"),\n')
            f.write("    },\n")
        f.write("];\n\n")

        # STAGE_FRONTS
        f.write("pub static STAGE_FRONTS: &[CardFront] = &[\n")
        for card_no, _tiles in stage_fronts:
            safe = sanitize_filename(card_no)
            f.write(f"    CardFront {{ card_no: \"{card_no}\",\n")
            f.write(f'        tiles: include_bytes!("../baked/card_art/stage_{safe}.bin"),\n')
            f.write("    },\n")
        f.write("];\n\n")

        # LIVE_FRONTS
        f.write("pub static LIVE_FRONTS: &[CardFront] = &[\n")
        for card_no, _tiles in live_fronts:
            safe = sanitize_filename(card_no)
            f.write(f"    CardFront {{ card_no: \"{card_no}\",\n")
            f.write(f'        tiles: include_bytes!("../baked/card_art/live_{safe}.bin"),\n')
            f.write("    },\n")
        f.write("];\n\n")

        # WAITED_FRONTS
        f.write("pub static WAITED_FRONTS: &[CardFront] = &[\n")
        for card_no, _tiles in waited_fronts:
            safe = sanitize_filename(card_no)
            f.write(f"    CardFront {{ card_no: \"{card_no}\",\n")
            f.write(f'        tiles: include_bytes!("../baked/card_art/wait_{safe}.bin"),\n')
            f.write("    },\n")
        f.write("];\n\n")

        f.write("pub static BACK_FRONT: &[u8] = &*include_bytes!(\"../baked/card_art/back_front.bin\");\n\n")
        f.write(f"pub static BOARD_UI: &[u8; {len(ui_tiles)}] = &*include_bytes!(\"../baked/card_art/board_ui.bin\");\n")


def main():
    used = deck_card_nos()
    card_list = sorted(used)
    print(f"{len(card_list)} unique deck cards to bake")

    # Build master palette from a representative sample (much faster)
    master_q, master_pal_bytes = build_master_palette(card_list)

    # Process all cards in parallel
    entries = []
    fronts = []
    stage_fronts = []
    live_fronts = []
    waited_fronts = []
    missing = []

    with ThreadPoolExecutor(max_workers=MAX_WORKERS) as executor:
        futures = {executor.submit(process_card, card_no, master_q, master_pal_bytes): card_no 
                   for card_no in card_list}
        
        for fut in as_completed(futures):
            card_no, result = fut.result()
            if result is None:
                missing.append(card_no)
                continue
            pal, tiles, ftiles, stiles, ltiles, wtiles, rotated = result
            if rotated:
                print(f"  upright: {card_no}")
            entries.append((card_no, pal, tiles, ftiles, stiles, ltiles, wtiles))
            fronts.append((card_no, ftiles))
            stage_fronts.append((card_no, stiles))
            live_fronts.append((card_no, ltiles))
            waited_fronts.append((card_no, wtiles))

    print(f"baked {len(entries)} cards, missing {len(missing)}")
    if missing:
        print("missing:", missing[:20])

    # These are fast, do sequentially
    ui_tiles = bake_ui_tiles()
    back_front = bake_back_front(master_q)
    write_gen(entries, fronts, stage_fronts, live_fronts, waited_fronts, back_front, ui_tiles, master_pal_bytes)
    print(f"wrote {OUT_RS}")


if __name__ == "__main__":
    main()