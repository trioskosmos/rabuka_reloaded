import json, sys, os, glob, struct

baked_dir = sys.argv[1]
out_dir = sys.argv[2]

with open(os.path.join(baked_dir, "decks.json"), "r", encoding="utf-8") as f:
    decks = json.load(f)

card_nos = []
for deck in decks:
    card_nos.extend(deck.get("cards", []))

seen = set()
unique = []
for c in card_nos:
    if c not in seen:
        seen.add(c)
        unique.append(c)

all_names = set()
for fpath in sorted(glob.glob(os.path.join(baked_dir, "deck_*_cards.json"))):
    with open(fpath, "r", encoding="utf-8") as f:
        cards = json.load(f)
    for card in cards:
        all_names.add(card.get("name", ""))
        all_names.add(card.get("ability_text", ""))
        for faq in card.get("faq", []):
            if isinstance(faq, str):
                all_names.add(faq)

# Collect all unique non-ASCII codepoints
codepoints = set()
for name in all_names:
    for ch in name:
        cp = ord(ch)
        if cp >= 0x80:
            codepoints.add(cp)

# Build UTF-32 -> SJIS map
entries = []
for cp in sorted(codepoints):
    ch = chr(cp)
    try:
        sjis = ch.encode("cp932", errors="replace")
        if len(sjis) == 1:
            entries.append((cp, 0, sjis[0]))
        elif len(sjis) == 2:
            entries.append((cp, sjis[0], sjis[1]))
        else:
            entries.append((cp, 0, ord("?")))
    except:
        entries.append((cp, 0, ord("?")))

# Write binary map: 6 bytes per entry = u32le(codepoint) + u8(sjis_hi) + u8(sjis_lo)
bin_path = os.path.join(out_dir, "sjis_map.bin")
with open(bin_path, "wb") as f:
    for cp, hi, lo in entries:
        f.write(struct.pack("<I", cp))
        f.write(struct.pack("BB", hi, lo))

# Also generate the Rust name lookup function (for exact card name SJIS bytes)
rs_path = os.path.join(out_dir, "sjis_table.rs")
lines = []
lines.append("pub fn card_name_sjis(card_no: &str) -> &'static [u8] {")
lines.append("    match card_no {")

for no in unique:
    name = None
    for fpath in sorted(glob.glob(os.path.join(baked_dir, "deck_*_cards.json"))):
        with open(fpath, "r", encoding="utf-8") as f:
            cards = json.load(f)
        for card in cards:
            if card.get("card_no", "") == no:
                name = card.get("name", "")
                break
        if name:
            break
    if name:
        try:
            sjis = name.encode("cp932", errors="replace")
            escaped = "".join("\\x{:02x}".format(b) for b in sjis)
            lines.append(f'        "{no}" => b"{escaped}",')
        except:
            lines.append(f'        "{no}" => b"",')
    else:
        lines.append(f'        "{no}" => b"",')

lines.append('        _ => b"",')
lines.append("    }")
lines.append("}")

with open(rs_path, "w", encoding="utf-8") as f:
    f.write("\n".join(lines))

print(f"Generated SJIS map: {len(entries)} codepoints, {len(unique)} card names")
