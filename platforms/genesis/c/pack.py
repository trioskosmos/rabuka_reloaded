#!/usr/bin/env python3
"""Produce NUL-terminated ROM data blobs from engine_c's raw bins.

The Genesis has only 64 KB RAM, so the string tables cannot be copied into
the arena. Instead we keep them ROM-resident and NUL-terminate every string
(see data.c RB_ROM_STRINGS). Start offsets are preserved, so the engine's
existing offset tables still index valid C strings.
"""
import struct, re, sys

E = "/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/engine_c"

# ---- cards.bin: NUL-terminate each entry of the string table ----
cards = open(E + "/src/cards.bin", "rb").read()
assert cards[:4] == b"CARD", "bad cards.bin magic"
nc = struct.unpack("<H", cards[4:6])[0]
stl = struct.unpack("<I", cards[6:10])[0]
strtab = cards[10:10 + stl]
rest = cards[10 + stl:]          # lentab + card records (unchanged)

out = bytearray()
p = 0
while p < len(strtab):
    sl = struct.unpack("<H", strtab[p:p + 2])[0]
    p += 2
    out += strtab[p:p + sl]
    out += b"\x00"
    p += sl

new = bytearray()
new += cards[:4]
new += struct.pack("<H", nc)
new += struct.pack("<I", len(out))
new += out
new += rest
open("cards_gen.bin", "wb").write(new)

# ---- abilities_strings.bin: NUL-terminate each string by offset ----
abstr = open(E + "/src/abilities_strings.bin", "rb").read()
gd = open(E + "/src/core/generated/gen_data.c").read()
m = re.search(r"RBKA_STRINGS_OFFSETS\s*\[[^\]]*\]\s*=\s*\{(.*?)\};", gd, re.S)
assert m, "could not find RBKA_STRINGS_OFFSETS"
nums = [int(x, 0) for x in re.findall(r"0x[0-9A-Fa-f]+|\d+", m.group(1))]
assert len(nums) >= 2, "offset table too small"

out2 = bytearray()
for i in range(len(nums) - 1):
    a, b = nums[i], nums[i + 1]
    out2 += abstr[a:b]
    out2 += b"\x00"
open("abstr_gen.bin", "wb").write(out2)

print("cards_gen.bin : %d bytes (strtab %d -> %d)" % (len(new), stl, len(out)))
print("abstr_gen.bin  : %d bytes (%d strings)" % (len(out2), len(nums) - 1))
