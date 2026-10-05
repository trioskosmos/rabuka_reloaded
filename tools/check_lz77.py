"""Decompress the baked BIOS LZ77 blobs in Python and sanity-check them.

This is the reference for the Rust decoder in
platforms/gba/src/card_art_gen.rs. If board_ui decompresses to all zeros the
whole UI (zone fills, borders, badges) is transparent and the screen is black.
"""

import os
import sys

BASE = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\platforms\gba\baked\card_art"


def lz77(src):
    assert src[0] == 0x10, "not an LZ77 stream: %02x" % src[0]
    declared = src[1] | (src[2] << 8) | (src[3] << 16)
    out = bytearray()
    si, di = 4, 0
    while di < declared:
        flags = src[si]
        si += 1
        for bit in range(8):
            if di >= declared:
                break
            if not (flags >> bit) & 1:
                out.append(src[si])
                si += 1
                di += 1
            else:
                b0, b1 = src[si], src[si + 1]
                si += 2
                length = (b0 >> 4) + 3
                offset = (((b0 & 0x0F) << 8) | b1) + 1
                for _ in range(length):
                    if di >= declared:
                        break
                    out.append(out[di - offset])
                    di += 1
    assert len(out) == declared, (len(out), declared)
    return bytes(out)


def describe(name, expect=None):
    p = os.path.join(BASE, name)
    raw = open(p, "rb").read()
    try:
        d = lz77(raw)
    except Exception as exc:
        print("%-16s DECODE FAILED: %s" % (name, exc))
        return None
    nonzero = sum(1 for b in d if b)
    print("%-16s declared=%-6d decoded=%-6d nonzero=%-6d (%.1f%%) %s" % (
        name, len(raw) and (raw[1] | raw[2] << 8 | raw[3] << 16), len(d),
        nonzero, 100.0 * nonzero / max(1, len(d)),
        "EXPECTED %s" % expect if expect else ""))
    return d


if __name__ == "__main__":
    d = describe("board_ui.bin")
    if d:
        print("  first 32 bytes:", " ".join("%02x" % b for b in d[:32]))
        # 4bpp: byte 0 holds pixels 1,0 -> the high nibble is the LEFT pixel.
        # If every nibble is 0 the tile is entirely colour 0 = transparent.
        left_nonzero = 0
        for b in d:
            if (b >> 4) != 0:
                left_nonzero += 1
        print("  bytes with non-zero HIGH nibble (left pixel): %d" % left_nonzero)
    describe("stage_LL_bp1_001_Rp.bin", "1920")
    describe("live_LL_bp1_001_Rp.bin", "384")
    describe("front_LL_bp1_001_Rp.bin", "768")
    describe("back_front.bin", "384")