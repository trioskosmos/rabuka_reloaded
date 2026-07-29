#!/usr/bin/env python3
"""
Deck compression analysis for QR code sharing.

Reads cards.json dynamically and calculates:
- Card counts by type (member, live, energy)
- Information-theoretic minimum bits to encode a deck
- Binary encoding size (fixed-width index per card)
- QR code version required for each encoding
- Whether 8-letter passwords are feasible

Regenerate anytime: just re-run after cards.json changes.

Rules derived from deck_builder.rs and card_browser.html DECK_CHECKS:
  - Main deck: 48 members + 12 live = 60 cards
  - Energy deck: 12 energy cards
  - Max 4 copies per card number (across rarities)
  - Energy cards excluded from copy limit
"""

import json
import math
import sys
from pathlib import Path

PROJECT = Path(__file__).parent
CARDS_PATH = PROJECT / "cards.json"

# QR code capacity (version, bytes) for error correction level M
# Source: ISO/IEC 18004 QR code specification
QR_CAPACITY = {
    1: 26, 2: 44, 3: 70, 4: 100, 5: 134, 6: 172, 7: 196,
    8: 242, 9: 292, 10: 346, 11: 404, 12: 466, 13: 532, 14: 581,
    15: 655, 16: 733, 17: 815, 18: 901, 19: 991, 20: 1085, 21: 1156,
    22: 1258, 23: 1364, 24: 1474, 25: 1588, 26: 1706, 27: 1828, 28: 1921,
    29: 2051, 30: 2185, 31: 2323, 32: 2465, 33: 2611, 34: 2761, 35: 2876,
    36: 3034, 37: 3196, 38: 3362, 39: 3532, 40: 3706,
}

QR_VERSION_SIZE = {v: 21 + 4 * v for v in QR_CAPACITY}

# Deck composition rules (from deck_builder.rs and card_browser.html DECK_CHECKS)
MAIN_MEMBER_COUNT = 48
MAIN_LIVE_COUNT = 12
ENERGY_DECK_COUNT = 12
TOTAL_DECK_SIZE = 72
MAX_COPIES = 4


def load_cards(path: Path) -> dict:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def count_cards_by_type(cards: dict) -> dict[str, int]:
    type_counts: dict[str, int] = {}
    for card_no, card_data in cards.items():
        card_type = card_data.get("type", "unknown")
        type_counts[card_type] = type_counts.get(card_type, 0) + 1
    return type_counts


def ways_to_choose(k: int, n: int, max_copies: int) -> int:
    """
    Number of ways to choose k items from n types with max max_copies each.
    Coefficient of x^k in (1 + x + ... + x^max_copies)^n.
    """
    if k == 0:
        return 1
    if n == 0 or k > n * max_copies:
        return 0

    dp = [0] * (k + 1)
    dp[0] = 1

    for _ in range(n):
        new_dp = [0] * (k + 1)
        for j in range(k + 1):
            if dp[j] == 0:
                continue
            for c in range(min(max_copies, k - j) + 1):
                new_dp[j + c] += dp[j]
        dp = new_dp

    return dp[k]


def log2_ways_to_choose(k: int, n: int, max_copies: int) -> float:
    """Log2 of ways_to_choose, using big integers for exactness."""
    if k == 0:
        return 0.0
    if n == 0 or k > n * max_copies:
        return float("-inf")
    ways = ways_to_choose(k, n, max_copies)
    if ways <= 0:
        return float("-inf")
    return math.log2(ways)


def min_bits_fixed_width(member_count: int, live_count: int, energy_count: int) -> int:
    """Bits needed for fixed-width index encoding (one index per card)."""
    member_bits = math.ceil(math.log2(member_count)) if member_count > 0 else 0
    live_bits = math.ceil(math.log2(live_count)) if live_count > 0 else 0
    energy_bits = math.ceil(math.log2(energy_count)) if energy_count > 0 else 0
    return (
        MAIN_MEMBER_COUNT * member_bits
        + MAIN_LIVE_COUNT * live_bits
        + ENERGY_DECK_COUNT * energy_bits
    )


def min_bits_information_theoretic(
    member_count: int, live_count: int, energy_count: int
) -> float:
    """Minimum bits using information-theoretic entropy (multiset selection)."""
    member_bits = log2_ways_to_choose(MAIN_MEMBER_COUNT, member_count, MAX_COPIES)
    live_bits = log2_ways_to_choose(MAIN_LIVE_COUNT, live_count, MAX_COPIES)
    energy_bits = log2_ways_to_choose(ENERGY_DECK_COUNT, energy_count, MAX_COPIES)
    return member_bits + live_bits + energy_bits


def strip_rarity_suffix(card_no: str) -> str:
    parts = card_no.split("-")
    return "-".join(parts[:-1])


def count_unique_base_card_nos(cards: dict, card_type: str) -> int:
    base_nos = set()
    for card_no, card_data in cards.items():
        if card_data.get("type") == card_type:
            base_nos.add(strip_rarity_suffix(card_no))
    return len(base_nos)


def rarity_stripped_bits(member_count: int, live_count: int, energy_count: int) -> int:
    """Bits needed if encoding using base card_no indices only (no rarity encoding).
    Energy cards cost 0 bits (forced to default). Member/live cards use ceil(log2(unique_base_count)).
    """
    member_bits = math.ceil(math.log2(member_count)) if member_count > 0 else 0
    live_bits = math.ceil(math.log2(live_count)) if live_count > 0 else 0
    energy_bits = 0
    return (
        MAIN_MEMBER_COUNT * member_bits
        + MAIN_LIVE_COUNT * live_bits
        + ENERGY_DECK_COUNT * energy_bits
    )


def rarity_stripped_text_size(cards: dict) -> int:
    """Approximate text size for the rarity-stripped compressed deck representation.
    Each member/live card line is base_card_no x QTY (no rarity suffix).
    Energy cards are omitted (forced to default).
    QTY is omitted when it is 1.
    """
    total_bytes = 0
    for card_type, deck_slots in [("メンバー", MAIN_MEMBER_COUNT), ("ライブ", MAIN_LIVE_COUNT)]:
        base_nos = set()
        for card_no, card_data in cards.items():
            if card_data.get("type") == card_type:
                base_nos.add(strip_rarity_suffix(card_no))
        unique_count = len(base_nos)
        if unique_count == 0:
            continue
        avg_base_len = sum(len(bn) for bn in base_nos) / unique_count
        estimated_unique_in_deck = max(1, deck_slots // 2)
        avg_qty = max(1, deck_slots // estimated_unique_in_deck)
        qty_suffix = f" x {avg_qty}" if avg_qty > 1 else ""
        line_len = int(avg_base_len) + len(qty_suffix) + 1
        total_bytes += estimated_unique_in_deck * line_len
    return total_bytes


def bits_to_qr_version(bits: int) -> int:
    """Find the minimum QR version that can hold the given number of bits."""
    bytes_needed = math.ceil(bits / 8)
    for version in sorted(QR_CAPACITY):
        if QR_CAPACITY[version] >= bytes_needed:
            return version
    return -1


def main():
    cards = load_cards(CARDS_PATH)
    type_counts = count_cards_by_type(cards)

    member_count = type_counts.get("\u30e1\u30f3\u30d0\u30fc", 0)
    live_count = type_counts.get("\u30e9\u30a4\u30d6", 0)
    energy_count = type_counts.get("\u30a8\u30cd\u30eb\u30ae\u30fc", 0)

    print("=" * 60)
    print("DECK COMPRESSION ANALYSIS")
    print("=" * 60)
    print(f"\nCard counts from cards.json:")
    print(f"  Member cards:  {member_count}")
    print(f"  Live cards:    {live_count}")
    print(f"  Energy cards:  {energy_count}")
    print(f"  Total unique:  {member_count + live_count + energy_count}")

    print(f"\nDeck composition (from deck_builder.rs / card_browser.html):")
    print(f"  Main deck:     {MAIN_MEMBER_COUNT} members + {MAIN_LIVE_COUNT} live = {MAIN_MEMBER_COUNT + MAIN_LIVE_COUNT} cards")
    print(f"  Energy deck:   {ENERGY_DECK_COUNT} energy cards")
    print(f"  Total:         {TOTAL_DECK_SIZE} cards")
    print(f"  Max copies:    {MAX_COPIES} per card number (across rarities)")

    # Fixed-width index encoding
    fw_bits = min_bits_fixed_width(member_count, live_count, energy_count)
    fw_bytes = math.ceil(fw_bits / 8)
    fw_qr_ver = bits_to_qr_version(fw_bits)
    fw_qr_size = QR_VERSION_SIZE.get(fw_qr_ver, -1)

    print(f"\n--- Fixed-width index encoding ---")
    print(f"  Member index:  {math.ceil(math.log2(member_count))} bits/card")
    print(f"  Live index:    {math.ceil(math.log2(live_count))} bits/card")
    print(f"  Energy index:  {math.ceil(math.log2(energy_count))} bits/card")
    print(f"  Total bits:    {fw_bits}")
    print(f"  Total bytes:   {fw_bytes}")
    if fw_qr_ver > 0:
        print(f"  QR version:    {fw_qr_ver} ({fw_qr_size}x{fw_qr_size} modules)")
    else:
        print(f"  QR version:    TOO LARGE")

    # Information-theoretic minimum
    it_bits = min_bits_information_theoretic(member_count, live_count, energy_count)
    it_bytes = math.ceil(it_bits / 8)
    it_qr_ver = bits_to_qr_version(int(it_bits))
    it_qr_size = QR_VERSION_SIZE.get(it_qr_ver, -1)

    print(f"\n--- Information-theoretic minimum ---")
    print(f"  Member entropy: {log2_ways_to_choose(MAIN_MEMBER_COUNT, member_count, MAX_COPIES):.2f} bits")
    print(f"  Live entropy:   {log2_ways_to_choose(MAIN_LIVE_COUNT, live_count, MAX_COPIES):.2f} bits")
    print(f"  Energy entropy: {log2_ways_to_choose(ENERGY_DECK_COUNT, energy_count, MAX_COPIES):.2f} bits")
    print(f"  Total bits:     {it_bits:.2f}")
    print(f"  Total bytes:    {it_bytes}")
    if it_qr_ver > 0:
        print(f"  QR version:     {it_qr_ver} ({it_qr_size}x{it_qr_size} modules)")
    else:
        print(f"  QR version:     TOO LARGE")

    # Rarity-stripped compression
    rs_member_count = count_unique_base_card_nos(cards, "\u30e1\u30f3\u30d0\u30fc")
    rs_live_count = count_unique_base_card_nos(cards, "\u30e9\u30a4\u30d6")
    rs_energy_count = count_unique_base_card_nos(cards, "\u30a8\u30cd\u30eb\u30ae\u30fc")

    print(f"\n--- Rarity-stripped compression ---")
    print(f"  Unique base card_nos per type:")
    print(f"    Member:  {rs_member_count}")
    print(f"    Live:    {rs_live_count}")
    print(f"    Energy:  {rs_energy_count}")
    print(f"  Bits per card:")
    print(f"    Member:  {math.ceil(math.log2(rs_member_count)) if rs_member_count > 0 else 0}")
    print(f"    Live:    {math.ceil(math.log2(rs_live_count)) if rs_live_count > 0 else 0}")
    print(f"    Energy:  0 (forced default)")

    rs_bits = rarity_stripped_bits(rs_member_count, rs_live_count, rs_energy_count)
    rs_bytes = math.ceil(rs_bits / 8)
    rs_qr_ver = bits_to_qr_version(rs_bits)
    rs_qr_size = QR_VERSION_SIZE.get(rs_qr_ver, -1)

    print(f"  Total bits:    {rs_bits}")
    print(f"  Total bytes:   {rs_bytes}")
    if rs_qr_ver > 0:
        print(f"  QR version:    {rs_qr_ver} ({rs_qr_size}x{rs_qr_size} modules)")
    else:
        print(f"  QR version:    TOO LARGE")

    rs_text_size = rarity_stripped_text_size(cards)
    rs_text_qr_ver = bits_to_qr_version(rs_text_size * 8)
    rs_text_qr_size = QR_VERSION_SIZE.get(rs_text_qr_ver, -1)

    print(f"  Text format size: ~{rs_text_size} bytes")
    if rs_text_qr_ver > 0:
        print(f"  QR version:      {rs_text_qr_ver} ({rs_text_qr_size}x{rs_text_qr_size} modules)")
    else:
        print(f"  QR version:      TOO LARGE")

    # 8-letter password analysis
    password_space = 26 ** 8
    password_bits = math.log2(password_space)
    print(f"\n--- 8-letter password analysis ---")
    print(f"  Password space: 26^8 = {password_space:,}")
    print(f"  Password bits:  {password_bits:.2f}")
    print(f"  Deck bits needed: {it_bits:.2f}")
    print(f"  Can encode? {'NO' if it_bits > password_bits else 'YES (but only if server stores the mapping)'}")
    print(f"  Note: 8-letter passwords cannot encode arbitrary decks client-side.")
    print(f"        Server-side lookup key is required.")

    # Current text format comparison
    text_format_size = 500  # approximate bytes for "card_no x QTY" format
    text_qr_ver = bits_to_qr_version(text_format_size * 8)
    text_qr_size = QR_VERSION_SIZE.get(text_qr_ver, -1)

    print(f"\n--- Current text format (for reference) ---")
    print(f"  Approx size:   ~{text_format_size} bytes")
    if text_qr_ver > 0:
        print(f"  QR version:    {text_qr_ver} ({text_qr_size}x{text_qr_size} modules)")
    else:
        print(f"  QR version:    TOO LARGE")

    # Encoding comparison
    print(f"\n--- Encoding comparison ---")
    print(f"  Text format:        ~{text_format_size} bytes  (QR v{text_qr_ver})")
    print(f"  Fixed-width:        {fw_bytes} bytes       (QR v{fw_qr_ver})")
    print(f"  Theoretical:        {it_bytes} bytes       (QR v{it_qr_ver})")
    print(f"  Rarity-stripped:    {rs_bytes} bytes       (QR v{rs_qr_ver})")
    print(f"  Improvement vs text: {text_format_size - rs_bytes} bytes saved (rarity-stripped)")

    # Check if binary encoding fits in QR version 4 (33x33)
    qr4_capacity = QR_CAPACITY.get(4, 0)
    print(f"\n--- QR version 4 (33x33) check ---")
    print(f"  QR v4 capacity: {qr4_capacity} bytes")
    print(f"  Fixed-width fits: {'YES' if fw_bytes <= qr4_capacity else 'NO'}")
    print(f"  Theoretical fits: {'YES' if it_bytes <= qr4_capacity else 'NO'}")
    print(f"  Rarity-stripped fits: {'YES' if rs_bytes <= qr4_capacity else 'NO'}")

    return 0


if __name__ == "__main__":
    sys.exit(main())