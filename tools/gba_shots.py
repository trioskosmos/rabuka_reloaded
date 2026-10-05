"""Decode the SRAM filmstrip written by platforms/gba/src/shot.rs into PNGs.

The ROM packs each captured frame as 4bpp sample indices, two pixels per byte,
high nibble first:
  slots 0..15   -> BG palette bank 15 (the text/UI palette)
  slots 16..255 -> OBJ palette index 0..239 (the card master palette)
  slot 255      -> backdrop (drawn mid grey)

SRAM layout (see shot.rs):
  0x0000  header: 'S','H', width, height, frame_count, next_slot, frame#(u32)
  0x0020  frame ring: frame_count * (width*height/2) bytes
  then    BG palette bank 15 (32 bytes) followed by the OBJ palette (512 bytes)

Usage: python gba_shots.py <rom.sav> <out_dir> [--latest]
"""

import os
import struct
import sys

from PIL import Image

HDR = 0x0000
FRAME_BASE = 0x0040


def rgb15_to_rgb888(word):
    r = (word & 0x1F) << 3
    g = ((word >> 5) & 0x1F) << 3
    b = ((word >> 10) & 0x1F) << 3
    return (min(r + (r >> 2), 255), min(g + (g >> 2), 255), min(b + (b >> 2), 255))


def main():
    sav = sys.argv[1]
    out_dir = sys.argv[2]
    latest = "--latest" in sys.argv
    os.makedirs(out_dir, exist_ok=True)

    data = open(sav, "rb").read()
    print("sav bytes: %d" % len(data))
    if len(data) < 0x100:
        print("sav too small - the ROM probably never wrote SRAM")
        return 1
    if data[0:2] != b"SH":
        print("bad magic %r - no capture present" % data[0:2])
        return 1

    width, height, nframes, next_slot = data[2], data[3], data[4], data[5]
    frame_no = struct.unpack_from("<I", data, HDR + 8)[0]
    frame_bytes = width * height
    pal_base = FRAME_BASE + frame_bytes * nframes

    print("filmstrip %dx%d, %d slots, last write at frame %d (next slot %d)"
          % (width, height, nframes, frame_no, next_slot))

    bg = [rgb15_to_rgb888(struct.unpack_from("<H", data, pal_base + i * 2)[0])
          for i in range(16)]
    obj = [rgb15_to_rgb888(struct.unpack_from("<H", data, pal_base + 32 + i * 2)[0])
           for i in range(256)]

    # Walk slots oldest -> newest. The next slot is the one about to be
    # overwritten, so it is the oldest surviving frame.
    order = [(next_slot + i) % nframes for i in range(nframes)]
    if latest:
        order = [(next_slot - 1) % nframes]

    for out_i, slot in enumerate(order):
        base = FRAME_BASE + slot * frame_bytes
        img = Image.new("RGB", (width, height))
        px = []
        for i in range(width * height):
            b = data[base + (i >> 1)]
            s = (b >> 4) if i % 2 == 0 else (b & 0x0F)
            if s == 0xF:
                px.append((64, 64, 64))
            elif s < 16:
                px.append(bg[s])
            else:
                oi = s - 16
                px.append(obj[oi] if oi < len(obj) else (255, 0, 255))
        img.putdata(px)
        path = os.path.join(out_dir, "frame_%02d_slot%02d.png" % (out_i, slot))
        img.save(path)
        print("wrote %s" % path)
    return 0


if __name__ == "__main__":
    sys.exit(main())