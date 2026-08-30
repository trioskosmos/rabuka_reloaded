#!/usr/bin/env python3
"""Emit gen_data.bin from the generated gen_data.c.

Layout (little-endian, matches the host representation the engine expects):
    [936 x uint16]  RBKA_OFFSET_DELTAS
    [5717 x uint32] RBKA_STRINGS_OFFSETS
The CD-i loader (gen_data_cdi.c) byte-swaps these to big-endian on load.
"""
import re, struct, sys

SRC = "src/core/generated/gen_data.c"
OUT = "src/gen_data.bin"

def grab(name, text):
    m = re.search(name + r"\s*\[\s*\d*\s*\]\s*=\s*\{([^}]*)\}", text, re.S)
    if not m:
        sys.exit("could not find array " + name)
    nums = re.findall(r"0x[0-9a-fA-F]+|\d+", m.group(1))
    return [int(x, 16) if x.startswith("0x") else int(x) for x in nums]

def main():
    text = open(SRC, encoding="utf-8", errors="replace").read()
    deltas = grab("RBKA_OFFSET_DELTAS", text)
    offsets = grab("RBKA_STRINGS_OFFSETS", text)
    pairs = grab("RBKA_CARD_ABILITY_PAIRS", text)
    print(f"RBKA_OFFSET_DELTAS = {len(deltas)} entries")
    print(f"RBKA_STRINGS_OFFSETS = {len(offsets)} entries")
    print(f"RBKA_CARD_ABILITY_PAIRS = {len(pairs)} entries")
    with open(OUT, "wb") as f:
        for v in deltas:
            f.write(struct.pack("<H", v & 0xFFFF))
        for v in offsets:
            f.write(struct.pack("<I", v & 0xFFFFFFFF))
        for v in pairs:
            f.write(struct.pack("<H", v & 0xFFFF))
    print(f"wrote {OUT}")

if __name__ == "__main__":
    main()
