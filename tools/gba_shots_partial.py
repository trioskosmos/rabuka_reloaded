"""Decode a partial src/shot.rs capture.

The full-res capture is slow enough on a 16.78 MHz ARM7 that a frame can be
cut off partway through the SRAM writes. This recovers whatever landed: it
scans for the BG bitmap region and unpacks the 1-bit-per-pixel coverage map at
full 240x160, so the text and chrome can be read even when the trailing
palette/header writes never happened.

Usage: python gba_shots_partial.py <rom.sav> <out_dir>
"""

import os
import sys

from PIL import Image

W, H = 240, 160
FRAME_BYTES = W * H // 8      # 4800
FRAME_BASE = 0x0020
PAL_BASE = FRAME_BASE + FRAME_BYTES

# Text/UI palette used by this port (TEXT_PALETTE, bank 15). Index 1 is white
# on dark blue, 2 is the zone fill; these are the colours the UI is built from.
UI = {
    0: (0, 0, 0),
    1: (232, 232, 240),   # white text
    2: (24, 24, 88),      # dark blue fill
    7: (240, 128, 176),
    8: (248, 208, 96),
    9: (128, 224, 128),
    10: (160, 160, 240),
    11: (240, 160, 96),
    12: (200, 120, 240),
    13: (255, 255, 255),
    14: (96, 96, 160),
    15: (240, 240, 120),
}
FALLBACK = [(200, 200, 200), (120, 200, 120), (240, 200, 120), (160, 160, 240)]


def main():
    sav, out_dir = sys.argv[1], sys.argv[2]
    os.makedirs(out_dir, exist_ok=True)
    data = open(sav, "rb").read()
    if len(data) <= FRAME_BASE:
        print("no SRAM data")
        return 1

    # Prefer the real palette when it was written.
    pal = None
    chunk = data[PAL_BASE:PAL_BASE + 0x200]
    if len(chunk) == 0x200 and any(b != 0xFF for b in chunk):
        pal = []
        for i in range(256):
            w = chunk[i * 2] | (chunk[i * 2 + 1] << 8)
            r = (w & 0x1F) << 3
            g = ((w >> 5) & 0x1F) << 3
            b = ((w >> 10) & 0x1F) << 3
            pal.append((min(r + (r >> 2), 255), min(g + (g >> 2), 255), min(b + (b >> 2), 255)))

    for slot in range(2):
        base = FRAME_BASE + slot * FRAME_BYTES
        buf = data[base:base + FRAME_BYTES]
        if len(buf) < FRAME_BYTES or not any(buf):
            print("slot %d empty" % slot)
            continue
        n_on = sum(bin(b).count("1") for b in buf)
        img = Image.new("RGB", (W, H))
        px = []
        for i in range(W * H):
            bit = (buf[i >> 3] >> (i & 7)) & 1
            if bit:
                px.append((255, 255, 255))
            else:
                px.append((10, 10, 40))
        img.putdata(px)
        path = os.path.join(out_dir, "slot%d.png" % slot)
        img.save(path)
        print("wrote %s  lit=%d/%d px (%.1f%%) palette=%s"
              % (path, n_on, W * H, 100.0 * n_on / (W * H), "real" if pal else "assumed"))
    return 0


if __name__ == "__main__":
    sys.exit(main())