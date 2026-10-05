"""Fetch card images for the cards just added to cards.json.

The scraper writes cards.json before its image step, so a failure there (missing Pillow
or httpx) leaves the database updated but the artwork absent. This pulls the PNGs for
every card that is new relative to the backup and converts them to WebP like the repo
does, into the same paths `_img` points at.
"""

import json
import sys
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
DB = REPO / "cards" / "cards.json"
BAK = REPO / "cards" / "cards.json.bak-before-pr"
RAW_DIR = REPO / "cards" / "images"
WEBP_DIR = REPO / "source_images"
BASE = "https://llofficial-cardgame.com"
HDRS = {"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"}


def one(card_no, url, rel):
    png = RAW_DIR / rel
    webp = WEBP_DIR / f"{card_no}.webp"
    if png.exists() and png.stat().st_size > 0 and webp.exists():
        return card_no, "cached", 0
    dest = png
    dest.parent.mkdir(parents=True, exist_ok=True)
    req = urllib.request.Request(url, headers=HDRS)
    with urllib.request.urlopen(req, timeout=60) as r:
        data = r.read()
    dest.write_bytes(data)
    size = len(data)
    try:
        from PIL import Image
        WEBP_DIR.mkdir(parents=True, exist_ok=True)
        with Image.open(dest) as im:
            im.save(webp, "WEBP", quality=90)
    except Exception as exc:
        return card_no, f"png ok, webp failed ({exc})", size
    return card_no, "ok", size


def main():
    db = json.loads(DB.read_text(encoding="utf-8"))
    old = json.loads(BAK.read_text(encoding="utf-8")) if BAK.exists() else {}
    new_cards = [k for k in db if k not in old]
    print(f"cards.json : {len(db)}   new since backup: {len(new_cards)}\n")

    jobs = []
    for k in new_cards:
        rel = (db[k].get("_img") or "").replace("img/cards/", "")
        if not rel:
            continue
        url = BASE + "/wordpress/wp-content/images/cardlist/" + urllib.parse.quote(rel)
        jobs.append((k, url, rel))

    ok = cached = bad = 0
    with ThreadPoolExecutor(max_workers=8) as pool:
        futs = {pool.submit(one, *j): j[0] for j in jobs}
        for fut in as_completed(futs):
            code, status, size = fut.result()
            if status == "ok":
                ok += 1
            elif status == "cached":
                cached += 1
            else:
                bad += 1
                print(f"  {code}: {status}")
    print(f"\nimages: {ok} downloaded, {cached} already there, {bad} failed")
    print(f"png  -> {RAW_DIR}")
    print(f"webp -> {WEBP_DIR}")

    missing = [k for k in new_cards if not (db[k].get("_img") or "").replace("img/cards/", "")
               or not (RAW_DIR / (db[k].get("_img") or "").replace("img/cards/", "")).exists()]
    if missing:
        print(f"\nno image for {len(missing)}: {missing[:10]}")


if __name__ == "__main__":
    main()