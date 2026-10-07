#!/usr/bin/env python3
"""Produce NUL-terminated ROM data blobs + ROM pointer tables from
engine_c_wip, for a native m68k console port (Jaguar).

Why the blobs are rewritten at all: cards.bin's string table is
length-prefixed (uint16 len + bytes, no NUL) and abilities_strings.bin
is a raw interned pool (offset-indexed, no NUL). The engine's
RB_ROM_STRINGS path points char* tables straight into the blobs, so
every string must be NUL-terminated in ROM. pack.py rewrites both
blobs with a NUL after each string and emits the matching pointer
tables as ROM .long arrays (romdata.s .include's them).

RAM budget: the pointer tables themselves stay in ROM too (the
Jaguar has 2 MB DRAM and GameState alone is ~1.04 MB), so the
shell defines the engine's g_card_strings_rom / g_strings_rom
pointer VARIABLES aimed at these ROM tables.

Also emits gen_data_jag.bin: the little-endian stream
rb_load_gen_data() validates against the compiled-in tables
([985 x u16 deltas][6033 x u32 string offsets][4232 x u16
card-ability pairs]), which rb_load_streaming() feeds it.

Same verify-then-write discipline as the Genesis pack.py: the
interned string pool is UNSEPARATED, so a short read is a silent
byte splice rather than a parse error -- every output is staged and
renamed only after the blob and gen_data.c are proven to be from
the same run.
"""
import struct, re, os, sys

E = "/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/engine_c_wip"


def write_atomic(path, data):
    tmp = path + ".tmp"
    with open(tmp, "wb") as f:
        f.write(data)
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, path)


def write_text_atomic(path, text):
    write_atomic(path, text.encode("utf-8"))


def extract_array(text, name):
    m = re.search(re.escape(name) + r"\s*\[[^\]]*\]\s*=\s*\{(.*?)\};", text, re.S)
    assert m, "could not find " + name + " in gen_data.c"
    return [int(x, 0) for x in re.findall(r"0x[0-9A-Fa-f]+|\d+", m.group(1))]


# ---- gen_data.c: the compiled-in offset tables (the single source
# of truth every other artifact is verified against) ----
gd = open(E + "/src/core/generated/gen_data.c").read()
gh = open(E + "/src/core/generated/gen_data.h").read()


def macro(name):
    m = re.search(r"#define\s+" + name + r"\s+(\d+)", gh)
    assert m, "missing " + name + " in gen_data.h"
    return int(m.group(1))


N_ABIL = macro("RBKA_NUM_ABILITIES_C")          # 985
N_OFF = macro("RBKA_NUM_STRING_OFFSETS")        # 6033
N_PAIRS = macro("RBKA_NUM_CARD_ABILITY_PAIRS")  # 2116

deltas = extract_array(gd, "RBKA_OFFSET_DELTAS")
offsets = extract_array(gd, "RBKA_STRINGS_OFFSETS")
pairs = extract_array(gd, "RBKA_CARD_ABILITY_PAIRS")
assert len(deltas) == N_ABIL, "RBKA_OFFSET_DELTAS count != RBKA_NUM_ABILITIES_C"
assert len(offsets) == N_OFF, "RBKA_STRINGS_OFFSETS count != RBKA_NUM_STRING_OFFSETS"
assert len(pairs) == N_PAIRS * 2, "RBKA_CARD_ABILITY_PAIRS count != 2*RBKA_NUM_CARD_ABILITY_PAIRS"

# gen_data_jag.bin: the LE stream rb_load_gen_data() byte-checks
# against the arrays above (tools/gen_gen_data_bin.py layout).
gen_data_blob = bytearray()
for v in deltas:
    gen_data_blob += struct.pack("<H", v)
for v in offsets:
    gen_data_blob += struct.pack("<I", v)
for v in pairs:
    gen_data_blob += struct.pack("<H", v)

# ---- cards.bin: NUL-terminate each entry of the string table ----
cards = open(E + "/src/cards.bin", "rb").read()
assert cards[:4] == b"CARD", "bad cards.bin magic"
nc = struct.unpack("<H", cards[4:6])[0]
stl = struct.unpack("<I", cards[6:10])[0]
strtab = cards[10:10 + stl]
rest = cards[10 + stl:]          # lentab + card records (unchanged)

out = bytearray()
card_ptrs = []                    # offset of each string in the NEW blob
p = 0
while p < len(strtab):
    sl = struct.unpack("<H", strtab[p:p + 2])[0]
    p += 2
    card_ptrs.append(10 + len(out))   # header(10) + running offset in out
    out += strtab[p:p + sl]
    out += b"\x00"
    p += sl

new = bytearray()
new += cards[:4]
new += struct.pack("<H", nc)
new += struct.pack("<I", len(out))
new += out
new += rest

# ---- abilities_strings.bin: NUL-terminate each string by offset ----
# ⚠ VERIFY-THEN-SPLIT GATE (same as the Genesis pack.py).
#
# `offsets` comes from gen_data.c; `abstr` comes from
# abilities_strings.bin. If those two are from DIFFERENT generator
# runs, slicing them together would byte-splice the interned pool —
# every string stops at its neighbour's offset — and no compiler or
# linker ever complains. Refuse outright instead.
abstr = open(E + "/src/abilities_strings.bin", "rb").read()


def _refuse(why, detail):
    sys.stderr.write(
        "\n================================================================\n"
        "PACK.PY REFUSING TO SPLIT THE INTERNED STRING TABLE\n"
        "================================================================\n"
        "%s\n%s\n\n"
        "  abilities_strings.bin   %d bytes\n"
        "  RBKA_STRINGS_TABLE_LEN   %s\n"
        "  RBKA_STRINGS_OFFSETS     %d entries\n\n"
        "The offsets and the blob are from DIFFERENT generator runs.\n"
        "FIX: bash tools/regen_all.sh\n"
        "DO NOT hand-patch the blob and DO NOT pass --force.\n"
        "================================================================\n\n"
        % (why, detail, len(abstr), table_len, len(offsets))
    )
    sys.exit(1)


m = re.search(r"RBKA_STRINGS_TABLE_LEN\s*=\s*(\d+)\s*;", gd)
assert m, "could not find RBKA_STRINGS_TABLE_LEN in gen_data.c"
table_len = int(m.group(1))

if len(abstr) != table_len:
    _refuse("the blob and gen_data.c disagree on the table length.",
            "  blob is %d bytes, gen_data.c records RBKA_STRINGS_TABLE_LEN = %d."
            % (len(abstr), table_len))

if offsets[0] != 0:
    _refuse("the offset table does not start at 0.",
            "  first offset is %d." % offsets[0])
for i in range(1, len(offsets)):
    if offsets[i] < offsets[i - 1]:
        _refuse("the offset table is out of order.",
                "  offsets[%d] = %d is BELOW offsets[%d] = %d."
                % (i, offsets[i], i - 1, offsets[i - 1]))
if offsets[-1] != table_len:
    _refuse("the offset table does not end at the end of the blob.",
            "  last offset is %d, the table is %d bytes."
            % (offsets[-1], table_len))

mfnv = re.search(r"RBKA_STRINGS_FNV\s*\[[^\]]*\]\s*=\s*\{", gd)
n_fnv_declared = int(re.search(r"RBKA_STRINGS_FNV\s*\[\s*(\d+)\s*\]", gd).group(1))
if n_fnv_declared != len(offsets) - 1:
    _refuse("RBKA_STRINGS_FNV and RBKA_STRINGS_OFFSETS disagree.",
            "  FNV declares %d digests but there are %d offset intervals."
            % (n_fnv_declared, len(offsets) - 1))

out2 = bytearray()
abstr_ptrs = []
for i in range(len(offsets) - 1):
    a, b = offsets[i], offsets[i + 1]
    abstr_ptrs.append(len(out2))   # offset of this string in the NEW blob
    out2 += abstr[a:b]
    out2 += b"\x00"

# ---- everything verified; now write, every output staged + renamed ----
write_atomic("cards_jag.bin", bytes(new))
write_atomic("abstr_jag.bin", bytes(out2))
write_atomic("gen_data_jag.bin", bytes(gen_data_blob))

write_text_atomic("card_ptrs.inc", "".join(
    ["/* auto-generated by pack.py - ROM pointer table into cards_jag.bin */\n",
     ".globl card_ptrs_table\n",
     "card_ptrs_table:\n"]
    + [".long cards_bin + %d\n" % off for off in card_ptrs]))

write_text_atomic("abstr_ptrs.inc", "".join(
    ["/* auto-generated by pack.py - ROM pointer table into abstr_jag.bin */\n",
     ".globl abstr_ptrs_table\n",
     "abstr_ptrs_table:\n"]
    + [".long abstr_bin + %d\n" % off for off in abstr_ptrs]))

print("cards_jag.bin   : %d bytes (%d strings)" % (len(new), len(card_ptrs)))
print("abstr_jag.bin   : %d bytes (%d strings)" % (len(out2), len(abstr_ptrs)))
print("gen_data_jag.bin: %d bytes (%d+%d+%d)" % (len(gen_data_blob), N_ABIL, N_OFF, N_PAIRS))
