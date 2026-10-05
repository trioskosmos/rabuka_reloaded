"""Decode the SRAM frames written by platforms/gba/src/shot.rs into PNGs.

Two bitmaps per frame at full 240x160:
  * BG coverage bitmap + BG palette bank 15 -> exact text/chrome colours
  * OBJ coverage bitmap -> sprite extent (sprites are marked, not colour-coded)

Rendered as: BG pixels in their real palette colour, sprite-only pixels in
green, and uncovered pixels in dark red, so "is the text clean" and "where are
the sprites" are both answerable from one image.

Usage: python gba_shots.py <rom.sav> <out_dir>
"""

import os
import sys

from PIL import Image

HDR = 0x0000
FRAME_BASE = 0x0020
W, H = 240, 160
FRAME_BYTES = W * H // 8
PAL_BASE = FRAME_BASE + FRAME_BYTES
OBJ_BASE = PAL_BASE + 0x220


def rgb15(word):
    r = (word & 0x1F) << 3
    g = ((word >> 5) & 0x1F) << 3
    b = ((word >> 10) & 0x1F) << 3
    return (min(r + (r >> 2), 255), min(g + (g >> 2), 255), min(b + (b >> 2), 255))


def getbit(buf, i):
    return (buf[i >> 3] >> (i & 7)) & 1


def main():
    sav, out_dir = sys.argv[1], sys.argv[2]
    os.makedirs(out_dir, exist_ok=True)
    data = open(sav, "rb").read()
    if len(data) < 0x200:
        print("sav too small (%d B) - the ROM never wrote SRAM" % len(data))
        return 1
    if data[0:2] != b"SF":
        print("bad magic %r - no capture" % data[0:2])
        return 1
    w, h, nframes, slot = data[2], data[3], data[4], data[5]
    print("frame %dx%d, %d slots, header slot=%d" % (w, h, nframes, slot))

    bg_pal = [rgb15(int.from_bytes(data[PAL_BASE + i * 2:PAL_BASE + i * 2 + 2], "little"))
              for i in range(256)]

    for s in range(nframes):
        bbuf = data[FRAME_BASE + s * FRAME_BYTES:FRAME_BASE + (s + 1) * FRAME_BYTES]
        sbuf = data[OBJ_BASE + s * FRAME_BYTES:OBJ_BASE + (s + 1) * FRAME_BYTES]
        if len(bbuf) < FRAME_BYTES:
            print("slot %d not written" % s)
            continue
        img = Image.new("RGB", (w, h))
        px = []
        n_sprite = n_bg = 0
        for i in range(w * h):
            if getbit(sbuf, i):
                px.append((0, 220, 0))          # sprite coverage
                n_sprite += 1
            elif getbit(bbuf, i):
                px.append(bg_pal[i & 0xF])      # exact BG colour
                n_bg += 1
            else:
                px.append((60, 0, 0))           # nothing drawn here
        img.putdata(px)
        path = os.path.join(out_dir, "frame%d.png" % s)
        img.save(path)
        print("wrote %s  bg=%d px sprite=%d px" % (path, n_bg, n_sprite))
    return 0


if __name__ == "__main__":
    sys.exit(main())