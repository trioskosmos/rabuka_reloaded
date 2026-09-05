#!/usr/bin/env python3
"""Compare D-pad presses between original and optimized deck ordering."""

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
    series = extract_series(card_no)
    rarity = extract_rarity(card_no)
    freq = GLOBAL_FREQ.get(card_no, 0)
    return (series, rarity, -freq, card_no)

def count_dpad_presses(deck_file: Path) -> tuple:
    """Count Up/Down presses needed to navigate through the deck in order."""
    cards = parse_deck(deck_file)
    
    # Expand to individual cards in order
    expanded = []
    for card_no, qty in cards:
        expanded.extend([card_no] * qty)
    
    # Simulate navigation: start at first card, count presses to each next card
    # GBA navigation: Series -> Rarity -> Card (Up/Down within filtered list)
    # We count presses to switch between cards in the sequence
    presses = 0
    prev_series = None
    prev_rarity = None
    prev_index_in_group = None
    
    for i, card_no in enumerate(expanded):
        series = extract_series(card_no)
        rarity = extract_rarity(card_no)
        
        if prev_series is None:
            # First card: no navigation needed
            pass
        elif series != prev_series:
            # Switch series: Left to Series field, Up/Down to new series, Right to Rarity, Up/Down to rarity, Right to Card
            presses += 1  # Left to Series
            # We don't know exact position, but switching series is ~1-7 presses
            presses += 3  # Estimate: Up/Down to find series
            presses += 1  # Right to Rarity
            presses += 1  # Right to Card
        elif rarity != prev_rarity:
            # Switch rarity within same series
            presses += 1  # Left to Rarity
            presses += 2  # Estimate: Up/Down to find rarity
            presses += 1  # Right to Card
        
        # Within same (series, rarity) group: Up/Down to find card
        # Need to know position in filtered list
        # For simplicity, count as 1 press per card within group (worst case)
        if series == prev_series and rarity == prev_rarity:
            presses += 1
        
        prev_series = series
        prev_rarity = rarity
    
    # Also count presses to add quantities (Right to Qty, Up/Down to qty, A to confirm)
    qty_presses = sum(qty for _, qty in cards) * 3  # ~3 presses per card add
    
    return presses + qty_presses, len(expanded)

def main():
    deck_dir = REPO / "web_ui" / "decks"
    deck_files = sorted(deck_dir.glob("*.txt"))
    
    print(f"{'Deck':<30} {'Original':>10} {'Optimized':>10} {'Savings':>10} {'Cards':>6}")
    print("-" * 70)
    
    total_orig = 0
    total_opt = 0
    total_cards = 0
    
    for f in deck_files:
        orig_presses, n_cards = count_dpad_presses(f)
        
        # Parse and reorder
        cards = parse_deck(f)
        expanded = []
        for card_no, qty in cards:
            expanded.extend([card_no] * qty)
        expanded.sort(key=sort_key)
        
        # Count optimized presses
        opt_presses = 0
        prev_series = None
        prev_rarity = None
        for card_no in expanded:
            series = extract_series(card_no)
            rarity = extract_rarity(card_no)
            
            if prev_series is None:
                pass
            elif series != prev_series:
                opt_presses += 5  # Left+Up/Down+Right+Right
            elif rarity != prev_rarity:
                opt_presses += 3  # Left+Up/Down+Right
            else:
                opt_presses += 1  # Up/Down within group
            prev_series = series
            prev_rarity = rarity
        
        qty_presses = sum(qty for _, qty in cards) * 3
        opt_presses += qty_presses
        
        savings = orig_presses - opt_presses
        pct = savings / orig_presses * 100 if orig_presses > 0 else 0
        
        print(f"{f.stem:<30} {orig_presses:>10} {opt_presses:>10} {savings:>10} ({pct:.0f}%)  {n_cards:>6}")
        total_orig += orig_presses
        total_opt += opt_presses
        total_cards += n_cards
    
    print("-" * 70)
    print(f"{'TOTAL':<30} {total_orig:>10} {total_opt:>10} {total_orig-total_opt:>10} ({(total_orig-total_opt)/total_orig*100:.0f}%)  {total_cards:>6}")

if __name__ == "__main__":
    main()