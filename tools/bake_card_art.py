#!/usr/bin/env python3
"""Bake GBA card art from original WebP sources — using include_bytes! binary files.

For each card:
- Detail view: 8bpp tiles (96x144 = 13824 bytes) against the shared MASTER_PAL
- Fronts (hand/stage/live/waited): 8bpp shared 240-colour MASTER_PAL

Every blob is written as a ZLIB (RFC1950/deflate) stream and inflated at
runtime by miniz_oxide, which the engine already links in for the ability
bytecode. This replaced the BIOS LZ77 (SWI 0x11) format the bake used to emit.

=== WHY DEFLATE, NOT BIOS LZ77 ===
Measured over all 1809 baked cards (baked/card_art, before this change):

    kind   raw            BIOS LZ77    deflate-9    zstd-19
    tiles  25,007,616     18,240,121   13,834,806   13,285,852
    stage   3,473,280      3,222,938    2,430,153    2,421,124
    front   1,389,312      1,413,043    1,066,393    1,062,393
    wait    1,389,312      1,375,317    1,056,608    1,052,477
    live      694,656        642,496      515,717      503,306
    pal       868,320        985,905        (dropped)  (dropped)
    TOTAL   32,822,496     25,879,820   18,904,157   18,325,632

Two independent reasons LZ77 lost:

1. The art is dithered, so its order-0 entropy is 6.41 bits/symbol — nearly
   flat. LZ77 is a dictionary matcher with no entropy stage, so it has nothing
   to work with. (BIOS Huffman would be worse still: at 6.41 bits/symbol its
   floor is ~512KB for 553KB of input, i.e. an expansion.) Deflate pairs LZ77
   with Huffman, which is exactly the stage the data needs.
2. The BIOS window is 4KB and applied per blob, so no card can ever match
   another even though all 1809 share one 240-colour palette.

zstd-19 is ~3% better than deflate but needs a new ~40KB no_std decoder
(ruzstd) and is ~3.5x slower to decode on a 16.78MHz ARM7. deflate reuses
miniz_oxide, which is already in this ROM. That trade is worth 570KB.

=== PALETTES ===
`pal_*.bin` used to be written once per card — 1809 byte-identical copies of
MASTER_PAL, 985,905 bytes of ROM, and the runtime never read `CardArt.palette`
at all (it builds all 16 OBJ palettes from MASTER_PAL in display.rs). The field
and the blobs are gone; the struct keeps only what is read.

Emits platforms/gba/src/card_art_gen.rs with:
    pub static MASTER_PAL: [u8; 480] = *include_bytes!("../baked/card_art/master_pal.bin");
    pub static CARD_ART: &[CardArt] = &[ CardArt { card_no: "PL!-BP1-001-R",
        tiles: include_bytes!("../baked/card_art/tiles_PL!-BP1-001-R.bin") }, ... ];
    pub static CARD_FRONTS: &[CardFront] = &[ CardFront { card_no: "...", tiles: include_bytes!(...) }, ... ];

Run:  py -3 tools/bake_card_art.py
"""

import json
import os
import random
import re
import sys
import time
import zlib
from concurrent.futures import ProcessPoolExecutor, as_completed
from pathlib import Path
from functools import lru_cache

from PIL import Image, ImageFilter

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "cards"))
from bake_deck_cards import normalize  # noqa: E402

# Pre-compile regex patterns
RE_COUNT_X = re.compile(r"^(\d+)\s*x\s*(.+)$")
RE_X_COUNT = re.compile(r"^(.*?)\s*x\s*(\d+)$")


def compress_blob(data: bytes) -> bytes:
    """Compress one blob to a zlib stream (level 9).

    Level 9 rather than zlib's default 6: this runs once per blob at bake
    time and never on the console, so the extra CPU is free. Measured on 300
    real detail tiles, 9 is 3.41% smaller than 6 (79,503 B on that sample),
    which extrapolates to roughly 500KB across all 1905 cards — worth having
    on a 32MB cart. Level 1 for reference is 37% larger than 9.
    """
    if not data:
        return zlib.compress(b"", 9)
    return zlib.compress(data, 9)

ART_W = 96
ART_H = 144
N_COLORS = 256  # 8bpp detail art (256 colors), shared MASTER_PAL
DETAIL_BPP = 8
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
MAX_WORKERS = max(1, (os.cpu_count() or 4) - 1)  # Leave one core free

# Global image cache for palette building (loaded once, reused)
_image_cache: dict[str, Image.Image] = {}

def load_image_cached(card_no: str) -> Image.Image | None:
    """Load a single image on demand with caching."""
    if card_no in _image_cache:
        return _image_cache[card_no]
    webp = CACHE / f"{card_no}.webp"
    if webp.exists():
        img = Image.open(webp).convert("RGB")
        _image_cache[card_no] = img
        return img
    return None


def clear_image_cache():
    """Clear the image cache to free memory."""
    global _image_cache
    _image_cache.clear()


def is_energy_card(card: dict) -> bool:
    """Check if card is energy type (Japanese 'エネルギー')."""
    return card.get('type') == 'エネルギー'


def all_non_energy_card_nos() -> set:
    """ALL non-energy card_nos from cards.json (for full card set)."""
    cards_dict = json.loads((REPO / "cards" / "cards.json").read_text(encoding="utf-8"))
    used = set()
    for k, v in cards_dict.items():
        if not is_energy_card(v):
            used.add(v["card_no"])
    return used


def deck_card_nos() -> set:
    """Union of normalized cards.json card_nos used by all decks (excludes energy)."""
    cards_dict = json.loads((REPO / "cards" / "cards.json").read_text(encoding="utf-8"))
    by_no = {}
    for k, v in cards_dict.items():
        by_no.setdefault(normalize(k), v)
    used = set()
    for f in sorted((REPO / "web_ui" / "decks").glob("*.txt")):
        for line in f.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line:
                continue
            m = RE_COUNT_X.match(line)
            if m:
                n = normalize(m.group(2).strip())
                if n in by_no and not is_energy_card(by_no[n]):
                    used.add(by_no[n]["card_no"])
                continue
            m = RE_X_COUNT.match(line)
            if m:
                n = normalize(m.group(1).strip())
                if n in by_no and not is_energy_card(by_no[n]):
                    used.add(by_no[n]["card_no"])
                continue
            # Bare card_no
            n = normalize(line)
            if n in by_no and not is_energy_card(by_no[n]):
                used.add(by_no[n]["card_no"])
    return used


def to_rgb15(r, g, b):
    return ((r >> 3) & 31) | (((g >> 3) & 31) << 5) | (((b >> 3) & 31) << 10)


def pack_8bpp_tiles(px, w, h, tiles_w, tiles_h):
    """Pack 8bpp palette indices into tiles, ty-major then tx. 64B per tile."""
    out = bytearray(tiles_w * tiles_h * 64)
    for ty in range(tiles_h):
        row_base = ty * tiles_w * 64
        for tx in range(tiles_w):
            base = row_base + tx * 64
            for rr in range(TILE):
                row_offset = rr * TILE
                for cc in range(TILE):
                    out[base + row_offset + cc] = px[tx * TILE + cc, ty * TILE + rr] & 0xFF
    return bytes(out)


def pack_4bpp_tiles(px, w, h, tiles_w, tiles_h):
    """Pack 4bpp palette indices into tiles, ty-major then tx. 32B per tile (2 pixels per byte)."""
    out = bytearray(tiles_w * tiles_h * 32)
    for ty in range(tiles_h):
        row_base = ty * tiles_w * 32
        for tx in range(tiles_w):
            base = row_base + tx * 32
            for rr in range(TILE):
                half_row = rr * (TILE // 2)
                for cc in range(0, TILE, 2):
                    v0 = px[tx * TILE + cc, ty * TILE + rr] & 0x0F
                    v1 = px[tx * TILE + cc + 1, ty * TILE + rr] & 0x0F
                    out[base + half_row + cc // 2] = v0 | (v1 << 4)
    return bytes(out)


def palette_bytes_16(pal, n=16):
    """First n entries of a PIL palette as rgb15 little-endian bytes."""
    out = bytearray(n * 2)
    idx = 0
    for i in range(n):
        r, g, b = pal[i * 3], pal[i * 3 + 1], pal[i * 3 + 2]
        c = to_rgb15(r, g, b)
        out[idx] = c & 0xFF
        out[idx + 1] = c >> 8
        idx += 2
    return bytes(out)


def palette_bytes_240(pal):
    """240 entries of a PIL palette as rgb15 little-endian bytes."""
    return palette_bytes_16(pal, 240)


def build_palette(thumbs, colors=240):
    """Build shared 240-colour palette from thumbnails."""
    if not thumbs:
        # Fallback palette
        q = Image.new("RGB", (16, 16))
        q.putpalette([i for i in range(256)] * 3)
        return q, q.getpalette()
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
    """96x144 8bpp detail view (240 colors)."""
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
    """Shared board UI tiles (bank-15 palette): 6 tiles x 32 bytes = 192 bytes.

    4bpp-packed (2 pixels/byte, low nibble first) to match the FourBpp
    TileSets in display.rs. A previous revision emitted 64 raw bytes per
    solid tile (288 total), which decodes as stripes, not solids.
    """
    tiles = bytearray()

    # 0: solid gray empty zone fill (color 2)
    tiles.extend([0x22] * 32)

    # 1: gold diamond actionable badge (color 4 on transparent 0)
    badge = [[0] * 8 for _ in range(8)]
    for y in range(8):
        d = abs(y - 3.5)
        for x in range(8):
            if abs(x - 3.5) + d <= 3:
                badge[y][x] = 4
    for y in range(8):
        for x in range(0, 8, 2):
            tiles.append(badge[y][x] | (badge[y][x + 1] << 4))

    # 2: white right-pointing triangle focus marker (color 1)
    marker = [[0] * 8 for _ in range(8)]
    for y in range(8):
        for x in range(8):
            if x <= 3 + abs(y - 3.5) * 1.4:
                marker[y][x] = 1
    for y in range(8):
        for x in range(0, 8, 2):
            tiles.append(marker[y][x] | (marker[y][x + 1] << 4))

    # 3: solid gold (color 4) for hand cursor border
    tiles.extend([0x44] * 32)

    # 4: fully transparent (color 0) for clearing front text BG
    tiles.extend([0x00] * 32)

    # 5: edge badge - gold diamond nudged right
    edge = [[0] * 8 for _ in range(8)]
    for y in range(8):
        d = abs(y - 3.5)
        for x in range(8):
            if abs(x - 6.5) + d <= 3:
                edge[y][x] = 4
    for y in range(8):
        for x in range(0, 8, 2):
            tiles.append(edge[y][x] | (edge[y][x + 1] << 4))

    return bytes(tiles)  # 6 tiles x 32 bytes = 192 bytes


def sanitize_filename(card_no: str) -> str:
    """Convert card_no to safe filename."""
    return card_no.replace("!", "").replace("+", "p").replace("＋", "p").replace("-", "_")


def sanitize_ident(card_no: str) -> str:
    """Convert card_no to a safe Rust identifier fragment (for AL_* statics)."""
    return "".join(
        c if (c.isascii() and (c.isalnum() or c == "_")) else "_"
        for c in sanitize_filename(card_no)
    )


def build_master_palette(card_nos: list[str], sample_size: int = 500) -> tuple:
    """Build master palette from a representative sample of cards."""
    if len(card_nos) <= sample_size:
        sample = card_nos
    else:
        # Use random.sample without importing random again
        import random
        sample = random.sample(card_nos, sample_size)
    
    thumbs = []
    for card_no in sample:
        img = load_image_cached(card_no)
        if img is None:
            continue
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
    # Reserve palette index 0 as magenta (255,0,255): index 0 is the
    # hardware-transparent index on OBJ sprites, and the runtime treats
    # every 0 byte as gutter (see upload_part). Before this reservation,
    # index 0 held near-white and up to ~24% of some cards' pixels
    # quantized to it — rendering as black holes. Magenta never occurs
    # in real art, so forcing it here keeps every natural pixel opaque.
    # (A genuinely magenta pixel would still land on 0; no card in the
    # current set does — the bake audit below would print it.)
    master_pal[0:3] = [255, 0, 255]
    master_q.putpalette(master_pal)
    master_pal_bytes = palette_bytes_240(master_pal)
    r, g, b = master_pal[0], master_pal[1], master_pal[2]
    assert (r, g, b) == (255, 0, 255)
    print(f"master palette: 240 colours from {len(thumbs)} thumbs (sample of {len(sample)} cards), index 0 reserved magenta")
    return master_q, master_pal_bytes


def process_card(card_no: str, master_q, master_pal_bytes):
    """Process a single card - returns all baked data.

    No per-card palette is returned or written: every card quantizes against
    the one shared MASTER_PAL, so the old `pal_<card>.bin` files were 1809
    byte-identical copies of it (985,905 bytes of ROM) and nothing ever read
    `CardArt.palette` — display.rs builds all 16 OBJ palettes from MASTER_PAL.
    """
    img = load_image_cached(card_no)
    if img is None:
        return card_no, None
    up, rotated = maybe_upright(img)
    _pal, tiles = bake_detail(img, master_q, master_pal_bytes)
    ftiles = bake_with_palette(up, FRONT_W, FRONT_H, master_q, FRONT_GRID, dither=Image.Dither.FLOYDSTEINBERG, sharpen=True)
    stiles = bake_with_palette(up, STAGE_W, STAGE_H, master_q, STAGE_GRID, dither=Image.Dither.FLOYDSTEINBERG)
    ltiles = bake_with_palette(img, LIVE_W, LIVE_H, master_q, LIVE_GRID, dither=Image.Dither.ORDERED, sharpen=True)
    # Waited (tapped) fronts mirror the 3DS tapped path: the portrait card
    # rotated 90° CCW so its dimensions swap and it fills the landscape
    # 32x24 slot. (Contain-fitting the unrotated portrait here used to bake
    # a shrunken upright card floating in padding — no visible wait state.)
    wtiles = bake_with_palette(up.rotate(90, expand=True), WAIT_W, WAIT_H, master_q, WAIT_GRID, dither=Image.Dither.ORDERED, sharpen=True)
    return card_no, (tiles, ftiles, stiles, ltiles, wtiles, rotated)


def write_gen(entries, fronts, stage_fronts, live_fronts, waited_fronts, back_front, ui_tiles, master_pal_bytes):
    """Generate card_art_gen.rs with include_bytes! references (zlib deflate)."""
    BIN_DIR.mkdir(parents=True, exist_ok=True)

    # Helper to write compressed data (records sizes for the 4-aligned
    # static declarations emitted below).
    written = set()
    blob_sizes = {}

    def write_compressed(path: Path, data: bytes):
        compressed = compress_blob(data)
        with open(path, "wb") as f:
            f.write(compressed)
        written.add(path.name)
        blob_sizes[path.name] = len(compressed)

    # Write binary files (compressed)
    # NOTE: master_pal stays RAW — display.rs indexes it directly as
    # rgb15 pairs (`MASTER_PAL[i*2]`), and the generated static demands
    # exactly 480 bytes. Compressing it breaks the type AND the palette
    # parse. Everything else goes through write_compressed.
    with open(BIN_DIR / "master_pal.bin", "wb") as f:
        f.write(master_pal_bytes)
    written.add("master_pal.bin")

    for card_no, tiles, _f, _s, _l, _w in entries:
        safe = sanitize_filename(card_no)
        write_compressed(BIN_DIR / f"tiles_{safe}.bin", tiles)

    for card_no, tiles in fronts:
        safe = sanitize_filename(card_no)
        write_compressed(BIN_DIR / f"front_{safe}.bin", tiles)

    for card_no, tiles in stage_fronts:
        safe = sanitize_filename(card_no)
        write_compressed(BIN_DIR / f"stage_{safe}.bin", tiles)

    for card_no, tiles in live_fronts:
        safe = sanitize_filename(card_no)
        write_compressed(BIN_DIR / f"live_{safe}.bin", tiles)

    for card_no, tiles in waited_fronts:
        safe = sanitize_filename(card_no)
        write_compressed(BIN_DIR / f"wait_{safe}.bin", tiles)

    write_compressed(BIN_DIR / "back_front.bin", back_front)
    write_compressed(BIN_DIR / "board_ui.bin", ui_tiles)

    # Generate Rust source with include_bytes! + runtime decompression
    with open(OUT_RS, "w", encoding="utf-8") as f:
        f.write("// Auto-generated by tools/bake_card_art.py -- do not edit.\n")
        f.write("// CardArt: 8bpp detail art (96x144 = 13824 bytes), shared MASTER_PAL\n")
        f.write("// Card fronts: 8bpp shared MASTER_PAL (4bpp on GBA via palette bank)\n")
        f.write("// Every .bin is a zlib (deflate) stream, inflated by miniz_oxide in\n")
        f.write("// display.rs::decompress_art. This replaced the BIOS LZ77 (SWI 0x11)\n")
        f.write("// format; see the module docstring in tools/bake_card_art.py for the\n")
        f.write("// measured 25.9MB -> 18.9MB across all baked cards.\n")
        f.write("#![allow(non_upper_case_globals)]\n")
        f.write("// (AL_* statics mirror card numbers in mixed case so art\n")
        f.write("// blobs stay grep-able from panic messages and debug logs.)\n\n")

        f.write("pub static MASTER_PAL: [u8; 480] = *include_bytes!(\"../baked/card_art/master_pal.bin\");\n\n")

        f.write("pub struct CardArt {\n")
        f.write("    pub card_no: &'static str,\n")
        f.write("    pub tiles: &'static [u8],\n")
        f.write("}\n\n")

        f.write("pub struct CardFront {\n")
        f.write("    pub card_no: &'static str,\n")
        f.write("    pub tiles: &'static [u8],\n")
        f.write("}\n\n")

        f.write("/// Alignment wrapper kept for the blob statics. inflate reads the\n")
        f.write("/// stream byte-wise so it has no alignment requirement, but the\n")
        f.write("/// BIOS no longer word-loads these and 4-byte alignment still\n")
        f.write("/// keeps each blob's address a round number in panic/debug output.\n")
        f.write("#[repr(align(4))]\n")
        f.write("pub struct AlignedBlob<const N: usize>(pub [u8; N]);\n\n")

        # 4-aligned blob statics (one per .bin; tables below borrow `&AL_x.0`).
        def emit_blob(var: str, name: str):
            n = blob_sizes[name]
            f.write(f"pub static {var}: AlignedBlob<{n}> = AlignedBlob(*include_bytes!(\"../baked/card_art/{name}\"));\n")

        for card_no, _tiles, _f, _s, _l, _w in entries:
            safe = sanitize_filename(card_no)
            ident = sanitize_ident(card_no)
            emit_blob(f"AL_tiles_{ident}", f"tiles_{safe}.bin")
        for card_no, _tiles in fronts:
            safe = sanitize_filename(card_no)
            ident = sanitize_ident(card_no)
            emit_blob(f"AL_front_{ident}", f"front_{safe}.bin")
        for card_no, _tiles in stage_fronts:
            safe = sanitize_filename(card_no)
            ident = sanitize_ident(card_no)
            emit_blob(f"AL_stage_{ident}", f"stage_{safe}.bin")
        for card_no, _tiles in live_fronts:
            safe = sanitize_filename(card_no)
            ident = sanitize_ident(card_no)
            emit_blob(f"AL_live_{ident}", f"live_{safe}.bin")
        for card_no, _tiles in waited_fronts:
            safe = sanitize_filename(card_no)
            ident = sanitize_ident(card_no)
            emit_blob(f"AL_wait_{ident}", f"wait_{safe}.bin")
        emit_blob("AL_back_front", "back_front.bin")
        emit_blob("AL_board_ui", "board_ui.bin")
        f.write("\n")

        # CARD_ART
        f.write("pub static CARD_ART: &[CardArt] = &[\n")
        for card_no, _tiles, _f, _s, _l, _w in entries:
            ident = sanitize_ident(card_no)
            f.write(f"    CardArt {{\n")
            f.write(f'        card_no: "{card_no}",\n')
            f.write(f'        tiles: &AL_tiles_{ident}.0,\n')
            f.write("    },\n")
        f.write("];\n\n")

        # CARD_FRONTS (hand)
        f.write("pub static CARD_FRONTS: &[CardFront] = &[\n")
        for card_no, _tiles in fronts:
            ident = sanitize_ident(card_no)
            f.write(f"    CardFront {{ card_no: \"{card_no}\",\n")
            f.write(f'        tiles: &AL_front_{ident}.0,\n')
            f.write("    },\n")
        f.write("];\n\n")

        # STAGE_FRONTS
        f.write("pub static STAGE_FRONTS: &[CardFront] = &[\n")
        for card_no, _tiles in stage_fronts:
            ident = sanitize_ident(card_no)
            f.write(f"    CardFront {{ card_no: \"{card_no}\",\n")
            f.write(f'        tiles: &AL_stage_{ident}.0,\n')
            f.write("    },\n")
        f.write("];\n\n")

        # LIVE_FRONTS
        f.write("pub static LIVE_FRONTS: &[CardFront] = &[\n")
        for card_no, _tiles in live_fronts:
            ident = sanitize_ident(card_no)
            f.write(f"    CardFront {{ card_no: \"{card_no}\",\n")
            f.write(f'        tiles: &AL_live_{ident}.0,\n')
            f.write("    },\n")
        f.write("];\n\n")

        # WAITED_FRONTS
        f.write("pub static WAITED_FRONTS: &[CardFront] = &[\n")
        for card_no, _tiles in waited_fronts:
            ident = sanitize_ident(card_no)
            f.write(f"    CardFront {{ card_no: \"{card_no}\",\n")
            f.write(f'        tiles: &AL_wait_{ident}.0,\n')
            f.write("    },\n")
        f.write("];\n\n")

        f.write("pub static BACK_FRONT: &[u8] = &AL_back_front.0;\n\n")
        f.write(f"pub static BOARD_UI: &[u8] = &AL_board_ui.0;\n")
    written.add("back_front.bin")
    written.add("board_ui.bin")

    # Prune stale files (cards removed/renamed since the last bake).
    # Stale .bins are invisible to the build (only referenced files embed)
    # but they pile up and confuse every future audit — 4,302 of them
    # accumulated before this step existed.
    for stale in BIN_DIR.glob("*.bin"):
        if stale.name not in written:
            stale.unlink()

    # Print the number this bake exists to control. A 32MB cart is the hard
    # ceiling and the art is ~95% of the ROM, so a silent regression here is
    # the difference between a bootable ROM and one that will not fit. Broken
    # out per kind because detail art is 73% of it and is the thing that
    # moves when the card count changes.
    per_kind: dict[str, int] = {}
    for name, size in blob_sizes.items():
        kind = name.split("_", 1)[0]
        per_kind[kind] = per_kind.get(kind, 0) + size
    total = sum(per_kind.values()) + len(master_pal_bytes)
    budget = 32 * 1024 * 1024
    print("baked art size (this is what embeds into the ROM):")
    for kind, size in sorted(per_kind.items(), key=lambda kv: -kv[1]):
        print(f"    {kind:<10} {size:>12,}B")
    print(f"    {'masterpal':<10} {len(master_pal_bytes):>12,}B")
    print(f"    {'TOTAL':<10} {total:>12,}B  ({total/1024/1024:.2f} MB, "
          f"{total/budget:.1%} of a 32MB cart)")


def _process_card_worker(args):
    """Worker function for multiprocessing - takes tuple of (card_no, master_q, master_pal_bytes)"""
    return process_card(*args)


def main():
    start_total = time.time()
    # Bake ALL non-energy cards (no second-class citizens)
    used = all_non_energy_card_nos()
    card_list = sorted(used)
    print(f"{len(card_list)} unique non-energy cards to bake")

    # Build master palette from a representative sample (much faster)
    print("Building master palette...")
    start = time.time()
    master_q, master_pal_bytes = build_master_palette(card_list)
    print(f"Palette build: {time.time() - start:.2f}s")

    # Process all cards in parallel using ProcessPoolExecutor for true parallelism
    entries = []
    fronts = []
    stage_fronts = []
    live_fronts = []
    waited_fronts = []
    missing = []

    print("Processing cards...")
    start = time.time()
    processed = 0
    
    # Prepare arguments for workers
    worker_args = [(card_no, master_q, master_pal_bytes) for card_no in card_list]
    
    with ProcessPoolExecutor(max_workers=MAX_WORKERS) as executor:
        # Submit all tasks
        futures = {executor.submit(_process_card_worker, args): args[0] for args in worker_args}
        
        for fut in as_completed(futures):
            processed += 1
            if processed % 100 == 0:
                elapsed = time.time() - start
                print(f"  Processed {processed}/{len(card_list)} cards ({elapsed:.1f}s, {processed/elapsed:.1f} cards/s)")
            card_no, result = fut.result()
            if result is None:
                missing.append(card_no)
                continue
            tiles, ftiles, stiles, ltiles, wtiles, rotated = result
            if rotated:
                print(f"  upright: {card_no}")
            entries.append((card_no, tiles, ftiles, stiles, ltiles, wtiles))
            fronts.append((card_no, ftiles))
            stage_fronts.append((card_no, stiles))
            live_fronts.append((card_no, ltiles))
            waited_fronts.append((card_no, wtiles))

    elapsed = time.time() - start
    print(f"Processed {len(entries)} cards in {elapsed:.1f}s ({len(entries)/elapsed:.1f} cards/s)")
    if missing:
        print(f"missing: {missing[:20]}")

    # Audit: index 0 is hardware-transparent, so any art pixel quantized
    # to 0 renders as a hole. With magenta reserved at 0 this must be ~0
    # everywhere; print the worst offenders either way.
    worst = []
    for card_no, tiles, f, s, li, w in entries:
        total = len(tiles) + len(f) + len(s) + len(li) + len(w)
        zeros = tiles.count(0) + f.count(0) + s.count(0) + li.count(0) + w.count(0)
        if zeros:
            worst.append((zeros / total, card_no, zeros, total))
    worst.sort(reverse=True)
    if worst:
        print(f"index-0 pixels: {len(worst)} cards affected, worst:")
        for pct, no, z, total in worst[:10]:
            print(f"  {pct:.1%} {no} ({z}/{total})")
    else:
        print("index-0 pixels: none (all art opaque)")

    # Clear image cache to free memory before final write
    clear_image_cache()

    # These are fast, do sequentially
    print("Baking UI tiles...")
    ui_tiles = bake_ui_tiles()
    print("Baking back front...")
    back_front = bake_back_front(master_q)
    
    print("Writing output...")
    start = time.time()
    write_gen(entries, fronts, stage_fronts, live_fronts, waited_fronts, back_front, ui_tiles, master_pal_bytes)
    print(f"Write gen: {time.time() - start:.2f}s")
    print(f"Total time: {time.time() - start_total:.1f}s")
    print(f"wrote {OUT_RS}")


if __name__ == "__main__":
    main()