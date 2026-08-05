"""Scrape card stats (cost, hearts, blade, score) from the detail API."""

import json
import re
import sys
import time
import urllib.request
from pathlib import Path

DETAIL_URL = "https://llofficial-cardgame.com/cardlist/detail/"
CARDS_JSON = Path(__file__).parent / "cards.json"
OUTPUT = Path(__file__).parent / "card_stats.json"


def fetch_detail(card_no):
    """POST to the detail endpoint and return HTML."""
    data = f"cardno={card_no}".encode("utf-8")
    req = urllib.request.Request(
        DETAIL_URL,
        data=data,
        headers={
            "Content-Type": "application/x-www-form-urlencoded",
            "User-Agent": "Mozilla/5.0",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            return resp.read().decode("utf-8", errors="replace")
    except Exception as e:
        print(f"  ERROR {card_no}: {e}")
        return None


def parse_stats(html, card_no):
    """Extract cost, hearts, blade, score from detail HTML."""
    stats = {"card_no": card_no}

    # Cost: look for コスト value
    m = re.search(r"<dt>コスト</dt>\s*<dd[^>]*>(\d+)</dd>", html)
    if m:
        stats["cost"] = int(m.group(1))

    # Blade: look for ブレード value
    m = re.search(r"<dt>ブレード</dt>\s*<dd[^>]*>(\d+)</dd>", html)
    if m:
        stats["blade"] = int(m.group(1))

    # Score: look for スコア value
    m = re.search(r"<dt>スコア</dt>\s*<dd[^>]*>(\d+)</dd>", html)
    if m:
        stats["score"] = int(m.group(1))

    # Hearts: look for heart icons with counts
    # Pattern: icon_heart01.png etc with surrounding text showing count
    hearts = {}
    # Try to find heart info section
    heart_section = re.search(r"<dt>ハート</dt>\s*<dd[^>]*>(.*?)</dd>", html, re.DOTALL)
    if heart_section:
        heart_html = heart_section.group(1)
        # Find all heart icons: icon_heartXX.png and nearby count
        # Format seems to be like: <img ... icon_heart01.png ... /> <span class="count">2</span>
        # Or maybe: heart01 × 2
        # Let's try multiple patterns
        for m in re.finditer(
            r"icon_heart(\d+)\.png[^>]*>(?:\s*</img>)?\s*(?:×\s*)?(\d+)", heart_html
        ):
            hearts[f"heart{m.group(1)}"] = int(m.group(2))
        if not hearts:
            # Try another pattern - maybe the count is in a nearby span
            for m in re.finditer(r"icon_heart(\d+)\.png", heart_html):
                hearts[f"heart{m.group(1)}"] = 1
    if hearts:
        stats["base_heart"] = hearts

    return stats


def main():
    # Load existing cards to get all card numbers
    if not CARDS_JSON.exists():
        print("cards.json not found!")
        sys.exit(1)

    cards = json.loads(CARDS_JSON.read_text(encoding="utf-8"))

    # Get BP07 and NSD02 card numbers
    bp07 = [
        k
        for k in cards
        if "-bp7-" in k
        or (
            k.startswith("PL!S-bp2-")
            and cards[k].get("product") == "ブースターパック MELLOW MOMENT"
        )
    ]
    nsd02 = [k for k in cards if "-sd2-" in k]

    target_cards = bp07 + nsd02
    print(f"Found {len(target_cards)} cards to scrape stats for")
    print(f"  BP07: {len(bp07)}")
    print(f"  NSD02: {len(nsd02)}")

    # Check which already have stats
    need_stats = []
    for cn in target_cards:
        c = cards[cn]
        if c.get("type") == "メンバー" and c.get("cost", 0) == 0:
            need_stats.append(cn)
        elif c.get("type") == "ライブ" and c.get("score", 0) == 0:
            need_stats.append(cn)
        elif c.get("type") not in ("メンバー", "ライブ"):
            # Energy cards etc - skip
            continue
        elif c.get("cost", 0) == 0 and c.get("score", 0) == 0:
            need_stats.append(cn)

    print(f"  Need stats: {len(need_stats)}")

    results = {}
    for i, cn in enumerate(need_stats):
        print(f"[{i + 1}/{len(need_stats)}] {cn}...", end=" ", flush=True)
        html = fetch_detail(cn)
        if html:
            stats = parse_stats(html, cn)
            results[cn] = stats
            print(
                f"cost={stats.get('cost', '-')} blade={stats.get('blade', '-')} score={stats.get('score', '-')} hearts={stats.get('base_heart', {})}"
            )
        else:
            print("FAILED")
        time.sleep(0.3)  # Be nice to the server

    OUTPUT.write_text(
        json.dumps(results, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    print(f"\nSaved {len(results)} stats to {OUTPUT}")

    # Show first few
    for k in list(results.keys())[:5]:
        print(f"  {k}: {results[k]}")


if __name__ == "__main__":
    main()
