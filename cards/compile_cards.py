"""Compile cards.json into a compact binary blob + cards_gen.rs.

Each card is encoded as a fixed-header + variable-length heart data,
stored in a ROM-friendly binary blob with an offset table for O(1) random
access.  Deck cards can be decoded on-demand from the blob into RAM,
while the blob itself lives in ROM/readonly data.

Format (little-endian):
  [Header: 12B]
    magic: b"CARD"
    num_cards: u32
    strtab_len: u32

  [String Table: strtab_len bytes]
    Entry: u16 len + u8[len] data  (index 0 = empty string)

  [Offset Table: (num_cards + 1) × u32]
    byte offset of each card from start of CARD_DATA section

  [Card Data: variable per card]
    Fixed 20-byte header:
      card_no_idx: u16
      name_idx: u16
      series_idx: u16
      group_idx: u16
      unit_idx: u16        (0xFFFF = None)
      img_idx: u16         (0xFFFF = None)
      product_idx: u16
      rare_idx: u16
      ability_idx: u16     (0xFFFF = None / default)
      type_flags: u8       (bits 0-1: type, bit 2: has_special_heart)
      cost: u8             (0 = None/unknown)
      blade: u8
      score: u8            (0 = None/unknown)
      num_base_hearts: u8
      num_blade_hearts: u8
      num_need_hearts: u8
    Heart pairs: (color: u8, count: u8) × (num_base + num_blade + num_need)
    Special heart [if has_special]: color: u8, count: u8
"""

import json, struct, sys
from pathlib import Path

HEART_COLORS = {
    "heart00": 0,
    "heart01": 1,
    "heart02": 2,
    "heart03": 3,
    "heart04": 4,
    "heart05": 5,
    "heart06": 6,
    "b_all": 7,
    "draw": 8,
    "score": 9,
    "all": 10,
}
CARDTYPE = {"メンバー": 0, "ライブ": 1, "エネルギー": 2}


def build_string_table(cards: list[dict]) -> tuple[list[str], dict[str, int]]:
    strings = [""]  # index 0 = empty
    idx_map = {"": 0}

    def intern(s: str) -> int:
        if s not in idx_map:
            idx_map[s] = len(strings)
            strings.append(s)
        return idx_map[s]

    for card in cards:
        intern(card.get("card_no", ""))
        intern(card.get("name", ""))
        intern(card.get("series", ""))
        intern(card.get("unit", ""))
        intern(card.get("img", ""))
        intern(card.get("product", ""))
        intern(card.get("rare", ""))
        intern(card.get("ability", ""))
        # group is derived from series at runtime
    return strings, idx_map


def encode_strtab(strings: list[str]) -> bytes:
    out = bytearray()
    for s in strings:
        encoded = s.encode("utf-8")
        out.extend(struct.pack("<H", len(encoded)))
        out.extend(encoded)
    return bytes(out)


def encode_card(card: dict, idx: dict[str, int]) -> bytes:
    ctype = CARDTYPE.get(card.get("type", ""), 0)
    cost = card.get("cost", 0)
    blade = card.get("blade", 0)
    score = card.get("score", 0)

    # Hearts
    base_hearts = parse_hearts(card.get("base_heart", {}))
    blade_hearts = parse_hearts(card.get("blade_heart", {}))
    need_hearts = parse_hearts(card.get("need_heart", {}))
    special = card.get("special_heart", None)
    if special and any(v > 0 for v in special.values()):
        has_special = 1
    else:
        has_special = 0
        special = None
    # Presence bits: 0x08 = has_cost (cost may legitimately be 0),
    # 0x10 = has_score (score may legitimately be 0).
    has_cost = 1 if "cost" in card else 0
    has_score = 1 if "score" in card else 0
    type_flags = ctype | (has_special << 2) | (has_cost << 3) | (has_score << 4)

    nb = len(base_hearts)
    nbl = len(blade_hearts)
    nn = len(need_hearts)

    out = bytearray()
    # Fixed header: 20 bytes
    out.extend(struct.pack("<H", idx.get(card.get("card_no", ""), 0)))
    out.extend(struct.pack("<H", idx.get(card.get("name", ""), 0)))
    out.extend(struct.pack("<H", idx.get(card.get("series", ""), 0)))
    out.extend(struct.pack("<H", idx.get(card.get("group", ""), 0)))
    out.extend(
        struct.pack(
            "<H", idx.get(card.get("unit", ""), 0) if card.get("unit") else 0xFFFF
        )
    )
    out.extend(struct.pack("<H", idx.get(card.get("img", ""), 0)))
    out.extend(struct.pack("<H", idx.get(card.get("product", ""), 0)))
    out.extend(struct.pack("<H", idx.get(card.get("rare", ""), 0)))
    out.extend(
        struct.pack(
            "<H", idx.get(card.get("ability", ""), 0) if card.get("ability") else 0xFFFF
        )
    )
    out.extend(struct.pack("<B", type_flags))
    out.extend(struct.pack("<B", min(cost, 255)))
    out.extend(struct.pack("<B", min(blade, 255)))
    out.extend(struct.pack("<B", min(score, 255)))
    out.extend(struct.pack("<B", nb))
    out.extend(struct.pack("<B", nbl))
    out.extend(struct.pack("<B", nn))

    # Hearts: (color, count) pairs
    for h in base_hearts + blade_hearts + need_hearts:
        out.extend(struct.pack("<BB", h[0], min(h[1], 255)))

    # Special heart
    if special:
        for color_str, count in special.items():
            col_idx = HEART_COLORS.get(color_str, 0)
            out.extend(struct.pack("<BB", col_idx, min(count, 255)))
            break

    return bytes(out)


def parse_hearts(h: dict | None) -> list[tuple[int, int]]:
    """Parse heart dict into list of (color_idx, count) pairs.
    Handles both heartXX and b_heartXX keys (b_ prefix stripped for heartXX,
    but NOT for b_all which maps to BAll (7), not All (10))."""
    if not h:
        return []
    result = []
    for color_str, count in h.items():
        # b_all is a direct match to BAll, not All with b_ stripped
        if color_str == "b_all":
            col_idx = 7
        elif color_str.startswith("b_"):
            # b_heartXX -> heartXX
            col_idx = HEART_COLORS.get(color_str[2:], 0)
        else:
            col_idx = HEART_COLORS.get(color_str, 0)
        result.append((col_idx, min(count, 255)))
    return result


def compile_all(cards_dict: dict) -> tuple[bytes, list[int], list[str]]:
    """Compile cards into (blob, offsets, strings)."""
    # Sort cards by card_no for deterministic order
    items = sorted(cards_dict.items(), key=lambda x: x[0])
    cards = [c for _, c in items]

    strings, idx = build_string_table(cards)
    strtab = encode_strtab(strings)

    # Per-card byte lengths (max card is well under 256 bytes → u8).
    # Start offsets are derived by prefix-sum on decode, replacing a u32 table.
    lengths = []
    card_data = bytearray()
    for card in cards:
        before = len(card_data)
        card_data.extend(encode_card(card, idx))
        lengths.append(len(card_data) - before)

    header = struct.pack("<4sHI", b"CARD", len(cards), len(strtab))
    length_table = struct.pack(f"<{len(lengths)}B", *lengths)

    blob = header + strtab + length_table + bytes(card_data)
    return blob, lengths, strings


def generate_cards_gen(
    blob: bytes, offsets: list[int], strings: list[str], build_dir: Path
):
    """Generate cards_gen.rs with embedded blob data."""
    hex_lines = []
    for i in range(0, len(blob), 24):
        chunk = blob[i : i + 24]
        hex_lines.append("    " + ", ".join(f"0x{b:02x}" for b in chunk) + ",")

    str_lits = ", ".join(json.dumps(s, ensure_ascii=False) for s in strings)

    src = f"""// Auto-generated by compile_cards.py
//
// Each card from cards.json is stored in a compact binary format with
// an interned string table.  Cards can be decoded on demand from the
// blob, enabling ROM-based storage and per-deck-card RAM loading.

pub const CARD_BLOB: &[u8] = &[
{chr(10).join(hex_lines)}
];

/// String table for card data. Indexed by u16 references in the blob.
pub const CARD_STRINGS: &[&str] = &[{str_lits}];
"""
    (build_dir / "cards_gen.rs").write_text(src, encoding="utf-8")


def main():
    cards_path = Path(__file__).parent / "cards.json"
    if not cards_path.exists():
        print(f"cards.json not found at {cards_path}", file=sys.stderr)
        sys.exit(1)

    cards_dict = json.loads(cards_path.read_text(encoding="utf-8"))
    print(f"Loaded {len(cards_dict)} cards from {cards_path}")

    blob, offsets, strings = compile_all(cards_dict)
    print(f"Blob size: {len(blob)} bytes")
    print(f"Cards: {len(offsets) - 1}")
    print(f"Strings: {len(strings)}")
    avg = (offsets[-1] / max(len(offsets) - 1, 1)) if len(offsets) > 1 else 0
    print(f"Avg bytes per card: {avg:.1f}")
    print(f"Offset table size: {len(offsets) * 4} bytes")
    print(f"String table size: {struct.unpack_from('<I', blob, 8)[0]} bytes")

    # Write blob for testing
    build_dir = Path(__file__).parent / "build"
    build_dir.mkdir(exist_ok=True)
    (build_dir / "cards.bin").write_bytes(blob)
    print(f"Wrote cards.bin ({len(blob)} bytes)")

    # Write generated Rust
    generate_cards_gen(blob, offsets, strings, build_dir)
    print("Wrote cards_gen.rs")

    # Also write to engine src directory
    engine_dir = Path(__file__).parent.parent / "engine" / "src" / "core"
    generate_cards_gen(blob, offsets, strings, engine_dir)
    print(f"Wrote cards_gen.rs to {engine_dir}")
    return blob, offsets, strings


if __name__ == "__main__":
    main()
