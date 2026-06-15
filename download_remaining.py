"""
Download remaining cards from official website URLs.
Skips corrupted card IDs (those with 白 characters).
"""

import json, os, sys, urllib.request, time
from pathlib import Path
from PIL import Image

BASE_DIR = Path(__file__).resolve().parent
CARDS_JSON = BASE_DIR / "cards" / "cards.json"
MAPPING_JSON = BASE_DIR / "web_ui" / "js" / "card_image_mapping.json"
WEBP_DIR = BASE_DIR / "web_ui" / "img" / "cards_webp"


def log(msg):
    print(msg, flush=True)


dry_run = "--download" not in sys.argv

with open(CARDS_JSON, encoding="utf-8") as f:
    cards_data = json.load(f)

existing_webps = (
    {f.stem for f in WEBP_DIR.glob("*.webp")} if WEBP_DIR.exists() else set()
)

# Filter out corrupted card IDs
all_missing = [c for c in cards_data if c not in existing_webps]
corrupted = [c for c in all_missing if "白" in c or "�" in c]
missing = [c for c in all_missing if c not in corrupted]

log(
    f"Cards missing webp: {len(all_missing)} total ({len(corrupted)} corrupted, {len(missing)} real)"
)

# Show examples of real missing cards
log("\nSample of real missing cards:")
for c in missing[:15]:
    card = cards_data[c]
    log(f"  {c}: type={card.get('type', '?')}, img={card.get('img', 'N/A')[:80]}")

if not dry_run and missing:
    WEBP_DIR.mkdir(parents=True, exist_ok=True)
    existing_mapping = {}
    if MAPPING_JSON.exists():
        with open(MAPPING_JSON, encoding="utf-8") as f:
            existing_mapping = json.load(f)

    downloaded = 0
    failed = 0
    for idx, card_no in enumerate(missing):
        card = cards_data[card_no]
        img_url = card.get("img", "")
        if not img_url or not img_url.startswith("http"):
            failed += 1
            continue

        webp_path = WEBP_DIR / f"{card_no}.webp"
        png_tmp = WEBP_DIR / f"{card_no}.png_tmp"

        try:
            req = urllib.request.Request(img_url, headers={"User-Agent": "Mozilla/5.0"})
            with urllib.request.urlopen(req, timeout=30) as resp:
                with open(png_tmp, "wb") as f:
                    f.write(resp.read())
            img = Image.open(png_tmp).convert("RGB")
            img.save(webp_path, "WEBP", quality=85)
            png_tmp.unlink(missing_ok=True)
            existing_mapping[card_no] = f"img/cards_webp/{card_no}.webp"
            downloaded += 1
            if downloaded % 20 == 0:
                log(f"  {downloaded}/{len(missing)} downloaded...")
                # Save progress
                with open(MAPPING_JSON, "w", encoding="utf-8") as f:
                    json.dump(existing_mapping, f, ensure_ascii=False, indent=2)
        except Exception as e:
            err = str(e)[:60]
            if "403" in err or "Forbidden" in err:
                log(f"  BLOCKED {card_no}: {err}")
            else:
                log(f"  FAILED {card_no}: {err}")
            if png_tmp.exists():
                png_tmp.unlink(missing_ok=True)
            failed += 1

    log(f"\nFrom official URLs: {downloaded} ok, {failed} failed")

    # Also add placeholder for corrupted cards (point to clean version's webp if exists)
    for c in corrupted:
        # Extract the clean card_no (before the corruption)
        clean = c
        for ch in ["白", "{", "�", "\ufffd"]:
            pos = clean.find(ch)
            if pos > 0:
                clean = clean[:pos]
                break
        if clean and clean != c:
            clean_webp = WEBP_DIR / f"{clean}.webp"
            if clean_webp.exists() and c not in existing_mapping:
                existing_mapping[c] = f"img/cards_webp/{clean}.webp"
                log(f"Mapped corrupted {c} -> {clean}")

    log("Saving final mapping...")
    with open(MAPPING_JSON, "w", encoding="utf-8") as f:
        json.dump(existing_mapping, f, ensure_ascii=False, indent=2)
    dist = BASE_DIR / "web_ui" / "dist" / "js" / "card_image_mapping.json"
    with open(dist, "w", encoding="utf-8") as f:
        json.dump(existing_mapping, f, ensure_ascii=False, indent=2)
