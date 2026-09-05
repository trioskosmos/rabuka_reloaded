#!/usr/bin/env python3
"""Analyze baked decks to determine optimal card ordering for minimal input."""

import re
from pathlib import Path
from collections import Counter, defaultdict

REPO = Path(__file__).resolve().parent.parent

def load_decks():
    """Load all deck files from web_ui/decks/*.txt"""
    deck_dir = REPO / "web_ui" / "decks"
    decks = {}
    for f in sorted(deck_dir.glob("*.txt")):
        card_nos = []
        for line in f.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line:
                continue
            m = re.match(r"^(\d+)\s*x\s*(.+)$", line)
            if m:
                qty = int(m.group(1))
                card_nos.extend([m.group(2).strip()] * qty)
                continue
            m = re.match(r"^(.*?)\s*x\s*(\d+)$", line)
            if m:
                qty = int(m.group(2))
                card_nos.extend([m.group(1).strip()] * qty)
                continue
            card_nos.append(line)
        decks[f.stem] = card_nos
    return decks

def normalize(card_no: str) -> str:
    """Normalize card number (uppercase, fullwidth->halfwidth)."""
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
    """Extract series prefix from card_no."""
    # e.g., PL!-BP1-001-R -> PL!
    #       PL!SP-BP1-001-R -> PL!SP
    #       LL-BP1-001-R -> LL
    #       PL!N-BP1-001-R -> PL!N
    parts = card_no.split("-")
    if len(parts) >= 1:
        return parts[0]
    return ""

def extract_rarity(card_no: str) -> str:
    """Extract rarity suffix from card_no."""
    # e.g., PL!-BP1-001-R -> R
    #       PL!-BP1-001-R+ -> R+
    #       PL!-BP1-001-SEC -> SEC
    parts = card_no.split("-")
    if len(parts) >= 2:
        return parts[-1]
    return ""

def extract_base(card_no: str) -> str:
    """Extract base card number (without rarity)."""
    parts = card_no.split("-")
    if len(parts) >= 2:
        return "-".join(parts[:-1])
    return card_no

def main():
    decks = load_decks()
    
    # Count global usage across all decks
    global_counter = Counter()
    for deck_name, cards in decks.items():
        global_counter.update(normalize(c) for c in cards)
    
    # Group by (series, rarity)
    by_group = defaultdict(list)
    for card_no, count in global_counter.items():
        series = extract_series(card_no)
        rarity = extract_rarity(card_no)
        base = extract_base(card_no)
        by_group[(series, rarity)].append((card_no, count, base))
    
    print("=" * 80)
    print("OPTIMAL ORDERING: By global usage frequency (most used = least Up/Down presses)")
    print("=" * 80)
    
    for (series, rarity), cards in sorted(by_group.items()):
        print(f"\n### Series: {series} | Rarity: {rarity} ({len(cards)} unique bases)")
        # Sort by global count desc, then card_no
        cards.sort(key=lambda x: (-x[1], x[0]))
        
        # Show top 10 per group
        for i, (card_no, count, base) in enumerate(cards[:15]):
            print(f"  {i+1:2d}. {card_no:30s}  used {count:3d}x  (base: {base})")
        if len(cards) > 15:
            print(f"  ... and {len(cards) - 15} more")

    print("\n" + "=" * 80)
    print("TOP 20 MOST USED CARDS OVERALL")
    print("=" * 80)
    for i, (card_no, count) in enumerate(global_counter.most_common(20)):
        series = extract_series(card_no)
        rarity = extract_rarity(card_no)
        base = extract_base(card_no)
        print(f"  {i+1:2d}. {card_no:30s}  {count:3d}x  [{series}/{rarity}] base={base}")

    print("\n" + "=" * 80)
    print("DECK COMPOSITION ANALYSIS (first 3 decks)")
    print("=" * 80)
    
    deck_names = list(decks.keys())[:3]
    for deck_name in deck_names:
        cards = decks[deck_name]
        print(f"\n--- {deck_name} ({len(cards)} cards) ---")
        counter = Counter(normalize(c) for c in cards)
        # Group by type (would need card DB, but we can infer from prefixes)
        by_series = defaultdict(Counter)
        for card_no, count in counter.items():
            series = extract_series(card_no)
            rarity = extract_rarity(card_no)
            by_series[series][rarity] += count
        
        for series, rarities in sorted(by_series.items()):
            for rarity, count in sorted(rarities.items()):
                print(f"  {series}/{rarity}: {count} cards")

if __name__ == "__main__":
    main()