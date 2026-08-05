"""Scrape new card data from llofficial-cardgame.com and add to cards.json.

This script:
1. Fetches card list pages for BP07, NSD02, and PR expansions
2. Parses card data from HTML
3. Downloads card images
4. Converts images to webp
5. Adds entries to cards.json
"""

import json
import os
import re
import struct
import sys
import urllib.request
import urllib.error
from pathlib import Path
from html.parser import HTMLParser

BASE_URL = "https://llofficial-cardgame.com"
IMG_BASE = "https://llofficial-cardgame.com/wordpress/wp-content/images/cardlist"
CARDS_DIR = Path(__file__).parent
CARDS_JSON = CARDS_DIR / "cards.json"
WEBP_DIR = CARDS_DIR.parent / "web_ui" / "img" / "cards_webp"

EXPANSIONS = {
    "BP07": "ブースターパック MELLOW MOMENT",
    "NSD02": "スタートデッキ ラブライブ！虹ヶ咲学園スクールアイドル同好会 cheer",
}

SERIES_MAP = {
    "BP07": {
        "PL!S": "ラブライブ！サンシャイン!!",
        "PL!N": "ラブライブ！虹ヶ咲学園スクールアイドル同好会",
        "PL!SP": "ラブライブ！スーパースター!!",
        "LL": "共通",
    },
    "NSD02": {
        "PL!N": "ラブライブ！虹ヶ咲学園スクールアイドル同好会",
    },
}

UNIT_MAP = {
    "Aqours": "Aqours",
    "虹ヶ咲": "虹ヶ咲",
    "Liella!": "Liella!",
    "QU4RTZ": "QU4RTZ",
    "DiverDiva": "DiverDiva",
    "R3BIRTH": "R3BIRTH",
    "A・ZU・NA": "A・ZU・NA",
    "CatChu!": "CatChu!",
    "KALEIDOSCORE": "KALEIDOSCORE",
    "5yncri5e!": "5yncri5e!",
    "Saint Snow": "Saint Snow",
}


class CardListParser(HTMLParser):
    """Parse card list HTML to extract card data."""

    def __init__(self):
        super().__init__()
        self.cards = []
        self.current_card = None
        self.in_card_name = False
        self.in_card_no = False
        self.in_card_type = False
        self.in_ability = False
        self.in_details_link = False
        self.capture_text = False
        self.current_text = ""
        self.tag_stack = []

    def handle_starttag(self, tag, attrs):
        attrs_dict = dict(attrs)

        # Card image - extract card_no from filename
        if tag == "img" and "src" in attrs_dict:
            src = attrs_dict["src"]
            if "/cardlist/" in src and src.endswith(".png"):
                # Extract card_no from filename like PL!S-bp7-001-R.png
                filename = src.split("/")[-1].replace(".png", "")
                if filename and not filename.startswith("thumb"):
                    self.current_card = {"card_no": filename, "img_src": src}

        # Card name - look for specific patterns
        if tag == "div" and "class" in attrs_dict:
            cls = attrs_dict.get("class", "")
            if "card-name" in cls or "card_name" in cls:
                self.in_card_name = True
                self.current_text = ""

        # Card details link
        if tag == "a" and "href" in attrs_dict:
            href = attrs_dict["href"]
            if "/cardlist/" in href and "details" in href:
                self.in_details_link = True

    def handle_endtag(self, tag):
        if self.in_card_name and tag == "div":
            if self.current_card and self.current_text.strip():
                self.current_card["name"] = self.current_text.strip()
            self.in_card_name = False

    def handle_data(self, data):
        if self.in_card_name:
            self.current_text += data


def fetch_page(url, retries=3):
    """Fetch a page with retries."""
    for attempt in range(retries):
        try:
            req = urllib.request.Request(
                url,
                headers={
                    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
                },
            )
            with urllib.request.urlopen(req, timeout=30) as resp:
                return resp.read().decode("utf-8", errors="replace")
        except Exception as e:
            print(f"  Attempt {attempt + 1} failed: {e}")
            if attempt < retries - 1:
                import time

                time.sleep(2)
    return None


def extract_cards_from_html(html, expansion):
    """Extract card data from HTML page."""
    cards = []

    # Find all card entries using regex
    # Pattern: card number in image src and ability text
    card_pattern = re.compile(
        r'<img[^>]*src="([^"]*cardlist/[^"]*/([^"]*?)(?:\.png|\.jpg))"[^>]*/?\s*>.*?'
        r"(?:カード番号\s*</[^>]+>\s*<[^>]+>\s*)?([A-Z!0-9\-+]+)",
        re.DOTALL,
    )

    # More robust: find card numbers in the HTML
    card_no_pattern = re.compile(
        r"(PL![SNP!]?-[a-zA-Z0-9]+-\d{3}[A-Z0-9＋\-]*|LL-[A-Z0-9\-]+)"
    )

    # Find image URLs
    img_pattern = re.compile(
        r'src="(/wordpress/wp-content/images/cardlist/[^"]+\.png)"'
    )

    # Find card names (Japanese text near card images)
    # The HTML structure has card name as text content near the card image

    # Extract card numbers and their associated images
    img_matches = list(img_pattern.finditer(html))

    for img_match in img_matches:
        img_src = img_match.group(1)
        filename = img_src.split("/")[-1].replace(".png", "")

        # Skip thumb images
        if "thumb" in filename.lower():
            continue

        # Extract the rarity from the filename
        # Format: PL!S-bp7-001-R or PL!N-sd2-001-SD2 etc.
        card_no = filename

        # Look for card name near this image
        # Search forward from the image for the card name
        search_start = img_match.end()
        search_text = html[search_start : search_start + 2000]

        # Find the card name - it's usually in a specific div after the image
        name_match = re.search(
            r'<(?:div|span)[^>]*class="[^"]*(?:card[_-]?name|cardname)[^"]*"[^>]*>([^<]+)',
            search_text,
        )
        if not name_match:
            # Try to find name in alt text or nearby text
            alt_match = re.search(
                r'alt="([^"]+)"', html[img_match.start() : img_match.end() + 200]
            )
            if alt_match:
                name = alt_match.group(1)
            else:
                # Look for text content after the image tag
                text_match = re.search(
                    r">\s*([ぁ-んァ-ヶ亜-熙a-zA-Z0-9☆！!?]+(?:\s*[ぁ-んァ-ヶ亜-熙a-zA-Z0-9☆！!?]+)*)\s*<",
                    search_text,
                )
                name = text_match.group(1).strip() if text_match else ""
        else:
            name = name_match.group(1).strip()

        # Determine card type
        type_match = re.search(
            r"カードタイプ\s*</[^>]+>\s*<[^>]+>\s*(メンバー|ライブ|エネルギー)",
            search_text,
        )
        card_type = type_match.group(1) if type_match else ""

        # Extract ability text
        ability_match = re.search(
            r"<img[^>]*texticon/[^>]*>\s*([^<]+(?:<img[^>]*texticon/[^>]*>[^<]*)*)",
            search_text,
        )
        ability_text = ""
        if ability_match:
            raw = ability_match.group(0)
            # Convert img tags to template syntax
            raw = re.sub(r"<img[^>]*texticon/(\w+)\.png[^>]*>", r"{{\1.png}}", raw)
            raw = re.sub(r"<[^>]+>", "", raw)
            ability_text = raw.strip()

        cards.append(
            {
                "card_no": card_no,
                "name": name,
                "type": card_type,
                "ability": ability_text,
                "img_src": img_src,
                "expansion": expansion,
            }
        )

    return cards


def download_image(img_src, dest_path):
    """Download an image from the website."""
    if dest_path.exists():
        return True

    url = BASE_URL + img_src if img_src.startswith("/") else img_src
    try:
        req = urllib.request.Request(
            url,
            headers={
                "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
            },
        )
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = resp.read()
            dest_path.parent.mkdir(parents=True, exist_ok=True)
            dest_path.write_bytes(data)
            return True
    except Exception as e:
        print(f"  Failed to download {url}: {e}")
        return False


def convert_to_webp(png_path, webp_path):
    """Convert PNG to WebP using PIL if available, otherwise copy."""
    if webp_path.exists():
        return True

    try:
        from PIL import Image

        img = Image.open(png_path)
        img.save(webp_path, "WEBP", quality=90)
        return True
    except ImportError:
        print("  PIL not available, trying cwebp...")
        import subprocess

        try:
            subprocess.run(
                ["cwebp", "-q", "90", str(png_path), "-o", str(webp_path)],
                check=True,
                capture_output=True,
            )
            return True
        except (subprocess.CalledProcessError, FileNotFoundError):
            print(f"  No WebP converter available, keeping PNG")
            return False


def get_series_from_card_no(card_no):
    """Determine series from card number prefix."""
    if card_no.startswith("PL!S-"):
        return "ラブライブ！サンシャイン!!"
    elif card_no.startswith("PL!N-"):
        return "ラブライブ！虹ヶ咲学園スクールアイドル同好会"
    elif card_no.startswith("PL!SP-"):
        return "ラブライブ！スーパースター!!"
    elif card_no.startswith("LL-"):
        return "共通"
    return ""


def get_rare_from_card_no(card_no):
    """Extract rarity from card number."""
    # Match the last part after the final dash
    match = re.search(r"-([A-Z0-9＋\-]+)$", card_no)
    if match:
        rare = match.group(1)
        # Normalize
        rare = rare.replace("＋", "+")
        return rare
    return ""


def build_card_entry(card, product_name, expansion):
    """Build a cards.json entry from scraped data."""
    series = get_series_from_card_no(card["card_no"])
    rare = get_rare_from_card_no(card["card_no"])

    # Determine the image subfolder based on expansion
    img_folder = expansion

    entry = {
        "card_no": card["card_no"],
        "img": f"https://llofficial-cardgame.com/wordpress/wp-content/images/cardlist/{img_folder}/{card['card_no']}.png",
        "name": card.get("name", ""),
        "product": product_name,
        "type": card.get("type", "メンバー"),
        "series": series,
        "rare": rare,
        "faq": [],
        "rare_list": [{"card_no": card["card_no"], "name": card.get("name", "")}],
        "_img": f"img/cards/{img_folder}/{card['card_no']}.png",
    }

    # Add ability if present
    if card.get("ability"):
        entry["ability"] = card["ability"]

    # Add type-specific fields
    if card.get("type") == "メンバー":
        entry["cost"] = 0
        entry["base_heart"] = {}
        entry["blade"] = 0
    elif card.get("type") == "ライブ":
        entry["score"] = 0
        entry["need_heart"] = {}

    return entry


def main():
    print("=" * 60)
    print("Scraping new cards from llofficial-cardgame.com")
    print("=" * 60)

    # Load existing cards
    if CARDS_JSON.exists():
        existing = json.loads(CARDS_JSON.read_text(encoding="utf-8"))
        print(f"Loaded {len(existing)} existing cards")
    else:
        existing = {}
        print("No existing cards.json found, starting fresh")

    new_cards = {}

    for expansion, product_name in EXPANSIONS.items():
        print(f"\n--- Scraping {expansion}: {product_name} ---")

        page = 1
        all_cards = []

        while True:
            url = f"{BASE_URL}/cardlist/searchresults/?expansion={expansion}&view=text&sort=new&page={page}"
            print(f"  Fetching page {page}...")

            html = fetch_page(url)
            if not html:
                print(f"  Failed to fetch page {page}")
                break

            cards = extract_cards_from_html(html, expansion)
            if not cards:
                print(f"  No more cards found on page {page}")
                break

            all_cards.extend(cards)
            print(f"  Found {len(cards)} cards on page {page}")

            # Check if there are more pages
            if f"page={page + 1}" not in html and len(cards) < 15:
                break

            page += 1
            import time

            time.sleep(1)  # Rate limiting

        print(f"  Total cards found for {expansion}: {len(all_cards)}")

        # Process each card
        for card in all_cards:
            card_no = card["card_no"]

            # Skip if already exists
            if card_no in existing:
                print(f"  Skipping {card_no} (already exists)")
                continue

            if card_no in new_cards:
                print(f"  Skipping {card_no} (already processed)")
                continue

            # Build entry
            entry = build_card_entry(card, product_name, expansion)
            new_cards[card_no] = entry

            # Download image
            img_dir = CARDS_DIR / "images" / expansion
            img_path = img_dir / f"{card_no}.png"

            if not img_path.exists():
                print(f"  Downloading {card_no}...")
                download_image(card["img_src"], img_path)

            # Convert to webp
            webp_path = WEBP_DIR / f"{card_no}.webp"
            if img_path.exists() and not webp_path.exists():
                print(f"  Converting {card_no} to webp...")
                convert_to_webp(img_path, webp_path)

    # Add new cards to existing
    if new_cards:
        existing.update(new_cards)

        # Sort by card_no
        sorted_cards = dict(sorted(existing.items()))

        # Write updated cards.json
        CARDS_JSON.write_text(
            json.dumps(sorted_cards, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        print(f"\n{'=' * 60}")
        print(f"Added {len(new_cards)} new cards to cards.json")
        print(f"Total cards now: {len(sorted_cards)}")
    else:
        print("\nNo new cards found to add")

    print("Done!")


if __name__ == "__main__":
    main()
