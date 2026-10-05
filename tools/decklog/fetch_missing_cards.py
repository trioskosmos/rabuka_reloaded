"""Fetch card images and metadata for cards used in decks but absent from cards.json.

Everything available without new sources comes out of the harvested deck data: name,
rarity, type, blade count, cost, and the image path. Images are pulled from the same
official CDN the existing cards.json `img` URLs point at, and written into cards/images
in the layout `_img` expects.
"""

import json
import re
import sys
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
DB = REPO / "cards" / "cards.json"
IMAGES = REPO / "cards" / "images"
CDN = "https://llofficial-cardgame.com/wordpress/wp-content/images/cardlist/"

sys.path.insert(0, str(HERE))
from scrape_decklog import headers  # noqa: E402

BASE_RE = re.compile(r"^(?P<base>.+-(?:\d{3}|E\d{2}))(?P<rarity>.*)$")


def normalize(c):
    m = BASE_RE.match(c)
    return m.group("base") if m else c


def collect_missing():
    db = json.loads(DB.read_text(encoding="utf-8"))
    known = set(db)
    known_norm = {normalize(k) for k in db}
    missing = {}
    for f in sorted((REPO / "tools" / "decklog" / "raw").glob("*.json")):
        try:
            d = json.loads(f.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        if not isinstance(d, dict):
            continue
        for it in (d.get("list") or []) + (d.get("sub_list") or []):
            cn = it.get("card_number")
            if not cn or cn in known or normalize(cn) in known_norm:
                continue
            r = missing.setdefault(cn, {"card_number": cn, "decks": 0, "copies": 0})
            r["decks"] += 1
            r["copies"] += it.get("num") or 0
            for k in ("name", "rare", "img", "card_kind", "blade", "blade_heart",
                      "max", "p_param", "g_param"):
                if k in it and it[k] not in (None, "", {}, []):
                    r.setdefault(k, it[k])
    return missing


def fetch(url, dest):
    req = urllib.request.Request(url, headers=headers("https://llofficial-cardgame.com/"))
    with urllib.request.urlopen(req, timeout=40) as resp:
        data = resp.read()
    if data[:4] != b"\x89PNG":
        raise ValueError(f"not a png ({len(data)}b)")
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_bytes(data)
    return len(data)


def main():
    missing = collect_missing()
    print(f"missing cards: {len(missing)}\n")
    ok = fail = 0
    report = []
    for cn, info in sorted(missing.items(), key=lambda kv: -kv[1]["copies"]):
        img = info.get("img") or ""
        url = CDN + img if img else ""
        rel = "img/cards/" + img if img else ""
        dest = IMAGES / img if img else None
        status, size = "no img path", 0
        if url:
            try:
                size = fetch(url, dest)
                status = "downloaded"
                ok += 1
            except Exception as exc:
                status = f"FAILED {exc}"
                fail += 1
        print(f"{cn:24} {info.get('name',''):16} {info['copies']:5} copies  {status}")
        info["img_url"] = url
        info["_img"] = rel
        info["image_status"] = status
        info["image_bytes"] = size
        report.append(info)

    out = REPO / "tools" / "decklog" / "missing_cards.json"
    out.write_text(json.dumps(report, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"\ndownloaded {ok}, failed {fail}")
    print(f"metadata -> {out}")
    print(f"images   -> {IMAGES}")


if __name__ == "__main__":
    main()