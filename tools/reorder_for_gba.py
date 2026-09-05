#!/usr/bin/env python3
"""Reorder deck for minimal GBA D-pad presses.

Usage:
  python tools/reorder_for_gba.py <deck_file.txt> [output.txt]
  
Deck file format: one card_no per line, or "card_no x qty", or "qty x card_no"
Output: card numbers in optimal navigation order for GBA deck builder.
"""

import re
import sys
from pathlib import Path
from collections import Counter, defaultdict

REPO = Path(__file__).resolve().parent.parent

def normalize(card_no: str) -> str:
    out = []
    for ch in card_no:
        if "a" <= ch <= "z":
            out.append(chr(ord(ch) - 32))
        elif "\uff41" <= ch <= "\uff5a":
            out.append(chr(ord(ch) - 0xFEE0))
        elif "\uff10" <= ch <= "\uff19":
            out.append(chr(ord(ch) - 0xFEE0))
        elif ch == "\uff0b":
            out.append("+")
        elif ch == "\uff01":
            out.append("!")
        elif ch == "\uff0d":
            out.append("-")
        else:
            out.append(ch)
    return "".join(out)

def extract_series(card_no: str) -> str:
    parts = card_no.split("-")
    return parts[0] if parts else ""

def extract_rarity(card_no: str) -> str:
    parts = card_no.split("-")
    return parts[-1] if len(parts) >= 2 else ""

def load_global_freq():
    """Precompute global frequency from all baked decks."""
    deck_dir = REPO / "web_ui" / "decks"
    global_counter = Counter()
    for f in sorted(deck_dir.glob("*.txt")):
        for line in f.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line:
                continue
            m = re.match(r"^(\d+)\s*x\s*(.+)$", line)
            if m:
                qty = int(m.group(1))
                for _ in range(qty):
                    global_counter[normalize(m.group(2).strip())] += 1
                continue
            m = re.match(r"^(.*?)\s*x\s*(\d+)$", line)
            if m:
                qty = int(m.group(2))
                for _ in range(qty):
                    global_counter[normalize(m.group(1).strip())] += 1
                continue
            global_counter[normalize(line)] += 1
    return global_counter

GLOBAL_FREQ = load_global_freq()

def parse_deck(deck_file: Path) -> list[tuple[str, int]]:
    """Parse deck file into list of (card_no, qty)."""
    cards = []
    for line in deck_file.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        m = re.match(r"^(\d+)\s*x\s*(.+)$", line)
        if m:
            cards.append((normalize(m.group(2).strip()), int(m.group(1))))
            continue
        m = re.match(r"^(.*?)\s*x\s*(\d+)$", line)
        if m:
            cards.append((normalize(m.group(1).strip()), int(m.group(2))))
            continue
        cards.append((normalize(line), 1))
    return cards

def sort_key(card_no: str) -> tuple:
    """Optimal GBA navigation sort key: (series, rarity, -global_freq, card_no)."""
    series = extract_series(card_no)
    rarity = extract_rarity(card_no)
    freq = GLOBAL_FREQ.get(card_no, 0)
    return (series, rarity, -freq, card_no)

def reorder_deck(deck_file: Path, output_file: Path = None):
    cards = parse_deck(deck_file)
    
    # Expand to individual entries for sorting
    expanded = []
    for card_no, qty in cards:
        expanded.extend([card_no] * qty)
    
    # Sort by optimal GBA order
    expanded.sort(key=sort_key)
    
    # Count back to quantities
    result = []
    current = None
    count = 0
    for c in expanded:
        if c != current:
            if current:
                result.append((current, count))
            current = c
            count = 1
        else:
            count += 1
    if current:
        result.append((current, count))
    
    # Output
    out_lines = [f"{no} x {qty}" for no, qty in result]
    output = "\n".join(out_lines)
    
    if output_file:
        output_file.write_text(output + "\n", encoding="utf-8")
        print(f"Wrote {len(result)} unique cards ({sum(q for _, q in result)} total) to {output_file}")
    else:
        print(output)
    
    # Show stats
    print(f"\nTotal: {sum(q for _, q in result)} cards, {len(result)} unique")
    print("\nNavigation order breakdown:")
    by_group = defaultdict(list)
    for no, qty in result:
        s = extract_series(no)
        r = extract_rarity(no)
        by_group[(s, r)].append((no, qty))
    for (s, r), items in sorted(by_group.items()):
        print(f"  {s}/{r}: {len(items)} unique, {sum(q for _, q in items)} cards")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python tools/reorder_for_gba.py <deck_file.txt> [output.txt]")
        sys.exit(1)
    
    deck_file = Path(sys.argv[1])
    output_file = Path(sys.argv[2]) if len(sys.argv) > 2 else None
    reorder_deck(deck_file, output_file)