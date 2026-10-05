"""Round-trip every LZ77 compressor/decoder in the art pipeline.

The encoder is tools/bake_card_art.py:lz77_compress_bios; the decoder that
matters at runtime is the pure-Rust one in
platforms/gba/src/card_art_gen.rs (LSB-first flag bits). This mirrors that
decoder exactly and asserts encoder -> decoder -> original for representative
inputs, so a re-bake can never again ship unreadable blobs.
"""

import importlib.util
import os
import random
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BAKER = os.path.join(HERE, "bake_card_art.py")


def load_baker():
    spec = importlib.util.spec_from_file_location("bake_card_art", BAKER)
    mod = importlib.util.module_from_spec(spec)
    # The baker writes output at import time in some revisions; guard it.
    if hasattr(mod, "main"):
        pass
    spec.loader.exec_module(mod)
    return mod


def lz77_decode(src):
    """Mirror of platforms/gba/src/card_art_gen.rs::lz77_decompress_wram."""
    assert src[0] == 0x10, "bad type byte %02x" % src[0]
    declared = src[1] | (src[2] << 8) | (src[3] << 16)
    out = bytearray()
    si, di = 4, 0
    while di < declared:
        flags = src[si]
        si += 1
        for bit in range(8):          # LSB-first
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
                assert offset <= di, "offset %d > di %d" % (offset, di)
                for _ in range(length):
                    if di >= declared:
                        break
                    out.append(out[di - offset])
                    di += 1
    assert len(out) == declared, (len(out), declared)
    return bytes(out)


def main():
    mod = load_baker()
    enc = mod.lz77_compress_bios

    rnd = random.Random(1234)
    cases = {
        "zeros-192": bytes(192),
        "boardui-ish": bytes((0x22 if i % 7 else 0xF0) for i in range(192)),
        "tile-like": bytes((i * 13) % 256 for i in range(1920)),
        "runs": (b"\x21" * 40 + b"\x9b\xbc" + b"\x21" * 30) * 6,
        "random-384": bytes(rnd.randrange(256) for _ in range(384)),
        "compressible-1920": (b"abcabcabcabcabcd" * 128)[:1920],
    }
    # A realistic 8bpp tile grid: few distinct colours, long runs of index 0.
    cases["cardfront"] = bytes(rnd.choice([0, 0, 0, 5, 5, 200, 33]) for _ in range(1920))

    ok = True
    for name, data in cases.items():
        try:
            blob = enc(data)
        except Exception as exc:
            print("FAIL %-18s encoder raised %s" % (name, exc))
            ok = False
            continue
        try:
            back = lz77_decode(blob)
        except Exception as exc:
            print("FAIL %-18s decoder raised %s" % (name, exc))
            ok = False
            continue
        if back != data:
            print("FAIL %-18s mismatch (%d -> %d -> %d)" % (name, len(data), len(blob), len(back)))
            ok = False
        else:
            print("ok   %-18s %5d -> %5d (%.0f%%)" % (
                name, len(data), len(blob), 100.0 * len(blob) / len(data)))
    print()
    print("ROUND-TRIP", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())