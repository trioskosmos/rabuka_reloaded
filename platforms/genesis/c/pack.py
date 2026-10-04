#!/usr/bin/env python3
"""Produce NUL-terminated ROM data blobs + ROM pointer tables from engine_c.

The Genesis has only 64 KB RAM, so the string *pointer* arrays (~32 KB for the
full database) cannot live in the arena either. Instead we keep the strings
ROM-resident (NUL-terminated) AND emit their pointer tables into ROM (see
romdata.s, which `.include`s the *.inc files). The engine's RB_ROM_STRINGS path
then points g_card_strings / g_strings straight at these ROM tables.
"""
import struct, re, os, sys

E = "/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/engine_c"


def write_atomic(path, data):
    """Write `data` to `path` through a temp file + rename.

    The blob is assembled entirely in memory and only becomes visible under its
    final name, so a consumer can never read a partially written one. Same
    discipline as tools/guard_data_blob.sh, and for the same reason: the
    interned string pool is UNSEPARATED, so a short read is a silent byte
    splice rather than a parse error.
    """
    tmp = path + ".tmp"
    with open(tmp, "wb") as f:
        f.write(data)
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, path)


def write_text_atomic(path, text):
    write_atomic(path, text.encode("utf-8"))

# ---- cards.bin: NUL-terminate each entry of the string table ----
cards = open(E + "/src/cards.bin", "rb").read()
assert cards[:4] == b"CARD", "bad cards.bin magic"
nc = struct.unpack("<H", cards[4:6])[0]
stl = struct.unpack("<I", cards[6:10])[0]
strtab = cards[10:10 + stl]
rest = cards[10 + stl:]          # lentab + card records (unchanged)

out = bytearray()
card_ptrs = []                    # ROM addresses of each string (relative to file)
p = 0
while p < len(strtab):
    sl = struct.unpack("<H", strtab[p:p + 2])[0]
    p += 2
    card_ptrs.append(10 + p)      # file offset of the string content
    out += strtab[p:p + sl]
    out += b"\x00"
    p += sl

new = bytearray()
new += cards[:4]
new += struct.pack("<H", nc)
new += struct.pack("<I", len(out))
new += out
new += rest
# NOTE: deliberately NOT written yet. The abilities gate below can refuse, and
# when it does, this run must leave no new output on disk at all -- otherwise a
# refusal leaves cards_gen.bin from this run beside a stale abstr_gen.bin from
# an earlier one, which is the same class of hazard one layer up.

# ---- abilities_strings.bin: NUL-terminate each string by offset ----
# ⚠ VERIFY-THEN-SPLIT GATE (added 2026-10-04).
#
# This is the SECOND place a data blob is cut up without verification. The
# first is guarded by tools/guard_data_blob.sh; this one had nothing, and that
# matters more than it looks, because of what happens downstream:
#
#   `nums`  comes from gen_data.c          (the C generation)
#   `abstr` comes from abilities_strings.bin (whatever bytes are on disk)
#
# If those two are from DIFFERENT generator runs, the loop below does not
# raise, does not warn, and does not fail the build. It silently produces a
# blob where every entry stops at its neighbour's offset -- a byte splice of
# the interned pool, which is exactly what corrupted 494 abilities (2026-09-30)
# and 1408 string refs (2026-10-01). gen_data.c's table was current; the .bin
# was a generation behind. Nothing was wrong with either file on its own.
#
# So: refuse outright, with the same discipline and the same remedy
# (tools/regen_all.sh, one ordered run -- never one without the other) that
# tools/guard_data_blob.sh uses. It verifies; it never repairs, and there is
# deliberately no --force: a mismatched pair is not something to patch by hand.
abstr = open(E + "/src/abilities_strings.bin", "rb").read()
gd = open(E + "/src/core/generated/gen_data.c").read()


def _refuse(why, detail):
    sys.stderr.write(
        "\n================================================================\n"
        "PACK.GY REFUSING TO SPLIT THE INTERNED STRING TABLE\n"
        "================================================================\n"
        "%s\n"
        "%s\n\n"
        "  abilities_strings.bin   %d bytes\n"
        "  RBKA_STRINGS_TABLE_LEN   %s\n"
        "  RBKA_STRINGS_OFFSETS     %d entries\n\n"
        "The offsets and the blob are from DIFFERENT generator runs. Slicing\n"
        "them together would byte-splice the interned pool: every string stops\n"
        "at its neighbour's offset, and no compiler or linker ever complains.\n"
        "That is how 494 abilities and 1408 string refs were silently wrong.\n\n"
        "FIX: bash tools/regen_all.sh\n"
        "     (regenerates cards/build AND the committed C tables in ONE ordered\n"
        "      run -- never one without the other)\n"
        "DO NOT hand-patch the blob and DO NOT pass --force.\n"
        "================================================================\n\n"
        % (why, detail, len(abstr), table_len, len(nums))
    )
    sys.exit(1)


m = re.search(r"RBKA_STRINGS_TABLE_LEN\s*=\s*(\d+)\s*;", gd)
assert m, "could not find RBKA_STRINGS_TABLE_LEN in gen_data.c"
table_len = int(m.group(1))

m = re.search(r"RBKA_STRINGS_OFFSETS\s*\[[^\]]*\]\s*=\s*\{(.*?)\};", gd, re.S)
assert m, "could not find RBKA_STRINGS_OFFSETS"
nums = [int(x, 0) for x in re.findall(r"0x[0-9A-Fa-f]+|\d+", m.group(1))]
assert len(nums) >= 2, "offset table too small"

# The length the offsets were generated against must equal the bytes on disk.
if len(abstr) != table_len:
    _refuse("the blob and gen_data.c disagree on the table length.",
            "  blob is %d bytes, gen_data.c records RBKA_STRINGS_TABLE_LEN = %d."
            % (len(abstr), table_len))

# Offsets must start at 0, never run backwards, and end exactly at the end of
# the table. A decrease means the table is out of order; an end that is not
# table_len means it was generated against a different blob.
# NOTE: equal ADJACENT offsets are legal (one empty interned string -- there is
# exactly 1 at the current generation), so this is a non-decreasing check, not
# a strictly-increasing one.
if nums[0] != 0:
    _refuse("the offset table does not start at 0.", "  first offset is %d." % nums[0])
for i in range(1, len(nums)):
    if nums[i] < nums[i - 1]:
        _refuse("the offset table is out of order.",
                "  offsets[%d] = %d is BELOW offsets[%d] = %d."
                % (i, nums[i], i - 1, nums[i - 1]))
if nums[-1] != table_len:
    _refuse("the offset table does not end at the end of the blob.",
            "  last offset is %d, the table is %d bytes."
            % (nums[-1], table_len))

# One FNV digest per interned string, and one string per interval between
# consecutive offsets, so the two counts differ by exactly one.
mfnv = re.search(r"RBKA_STRINGS_FNV\s*\[[^\]]*\]\s*=\s*\{", gd)
n_fnv_declared = int(re.search(r"RBKA_STRINGS_FNV\s*\[\s*(\d+)\s*\]", gd).group(1))
if n_fnv_declared != len(nums) - 1:
    _refuse("RBKA_STRINGS_FNV and RBKA_STRINGS_OFFSETS disagree.",
            "  FNV declares %d digests but there are %d offset intervals."
            % (n_fnv_declared, len(nums) - 1))

out2 = bytearray()
abstr_ptrs = []
for i in range(len(nums) - 1):
    a, b = nums[i], nums[i + 1]
    abstr_ptrs.append(a)          # ROM offset of string content
    out2 += abstr[a:b]
    out2 += b"\x00"

# ---- everything verified; now write, all four outputs, each staged + renamed ----
# Stage beside the destination and rename, so a consumer can never observe a
# half-written blob -- the same discipline guard_data_blob.sh uses for src/.
write_atomic("abstr_gen.bin", bytes(out2))

write_text_atomic("abstr_ptrs.inc", "".join(
    ["/* auto-generated by pack.py - ROM pointer table into abilities_strings */\n",
     ".globl g_strings_rom\n",
     "g_strings_rom:\n"]
    + [".long abstr_bin + %d\n" % off for off in abstr_ptrs]))

write_atomic("cards_gen.bin", bytes(new))

write_text_atomic("card_ptrs.inc", "".join(
    ["/* auto-generated by pack.py - ROM pointer table into cards strtab */\n",
     ".globl g_card_strings_rom\n",
     "g_card_strings_rom:\n"]
    + [".long cards_bin + %d\n" % off for off in card_ptrs]))

print("cards_gen.bin : %d bytes (%d strings)" % (len(new), len(card_ptrs)))
print("abstr_gen.bin  : %d bytes (%d strings)" % (len(out2), len(abstr_ptrs)))
