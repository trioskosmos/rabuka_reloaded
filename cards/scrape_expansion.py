"""Scrape one expansion from the official Love Live card site into cards/cards.json.

The site's card browser was rebuilt on a JSON API. The old HTML endpoints used by
llocg_db/database.py are gone:

    GET  /cardlist/cardsearch_ex?expansion=..&limit=100   -> 200 but no card="..." markup
    POST /cardlist/detail/            (cardno=..)          -> 404 for every card

The replacements are declared by PWOCG_CFG in any cardlist page:

    GET {apiBase}/card-list-user/list?title=<CODE>&page=N&per_page=100&sort=no
    GET {apiBase}/card-list-user/detail?id=<numeric card id>

with apiBase = https://llofficial-cardgame.com/manage

Field mapping below was derived by diffing every card we already own (2372 shared
cards) against this API; see docs/EXPANSION_SCRAPING.md.

Usage:
    python cards/scrape_expansion.py PBLL02 [--no-images] [--dry-run]
"""

import argparse
import asyncio
import json
import re
import sys
import urllib.parse
import urllib.request
from collections import defaultdict
from pathlib import Path

BASE = "https://llofficial-cardgame.com"
API = f"{BASE}/manage/card-list-user"
HDRS = {
    "user-agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
    "referer": f"{BASE}/cardlist/searchresults/",
    "accept": "application/json",
}

CARDS_JSON = Path(__file__).parent / "cards.json"
WEBP_DIR = Path(__file__).parent.parent / "web_ui" / "img" / "cards_webp"
RAW_DIR = Path(__file__).parent / "images"

# Base-heart palette: index 00 is the colourless heart.
BASE_HEART_IDX = {"桃": "01", "緑": "02", "青": "03", "赤": "04", "黄": "05", "紫": "06", "無": "0"}

# The blade-heart palette is a DIFFERENT ordering from the base palette. Derived over
# ~800 sampled cards; do not "simplify" these two tables into one.
BLADE_HEART_IDX = {"桃": "01", "赤": "02", "黄": "03", "緑": "04", "青": "05", "紫": "06", "無": "07"}

# Series is derived from the card-number prefix. Order matters: PL!SP starts with PL!S.
SERIES_PREFIXES = [
    ("PL!N", "ラブライブ！虹ヶ咲学園スクールアイドル同好会"),
    ("PL!HS", "ラブライブ！蓮ノ空女学院スクールアイドルクラブ"),
    ("PL!SP", "ラブライブ！スーパースター!!"),
    ("PL!S", "ラブライブ！サンシャイン!!"),
]
SERIES_DEFAULT = "ラブライブ！"

# Ability icon tokens -> the {{file|label}} form stored in cards.json.
ICONS = {
    "【ライブ開始時】": "{{live_start.png|ライブ開始時}}",
    "【ライブ成功時】": "{{live_success.png|ライブ成功時}}",
    "【登場】": "{{toujyou.png|登場}}",
    "【常時】": "{{jyouji.png|常時}}",
    "【起動】": "{{kidou.png|起動}}",
    "【自動】": "{{jidou.png|自動}}",
    "[ターン1回]": "{{turn1.png|ターン1回}}",
    "【ターン1回】": "{{turn1.png|ターン1回}}",
    "[ターン2回]": "{{turn2.png|ターン2回}}",
    "【ターン2回】": "{{turn2.png|ターン2回}}",
    "[E]": "{{icon_energy.png|E}}",
    "[ブレード]": "{{icon_blade.png|ブレード}}",
    "[スコア]": "{{icon_score.png|スコア}}",
    "[ドロー]": "{{icon_draw.png|ドロー}}",
    "[ALLブレード]": "{{icon_b_all.png|ALLブレード}}",
    "[全ブレード]": "{{icon_b_all.png|ALLブレード}}",
    "ALL1": "{{icon_b_all.png|ALLブレード}}",
    "[ALLハート]": "{{icon_all.png|ハート}}",
    "[全ハート]": "{{icon_all.png|ハート}}",
    "【センター】": "{{center.png|センター}}",
    "【左サイド】": "{{leftside.png|左サイド}}",
    "【右サイド】": "{{rightside.png|右サイド}}",
    "[heart0]": "{{heart_00.png|heart0}}",
}
for _jp, _idx in BASE_HEART_IDX.items():
    if _idx != "0":
        ICONS[f"[{_jp}ハート]"] = f"{{{{heart_{_idx}.png|heart{_idx}}}}}"
        ICONS[f"[{_jp}ブレードハート]"] = f"{{{{blade_heart{_idx}.png|b_heart{_idx}}}}}"
for _i in range(1, 7):
    ICONS[f"[heart{_i:02d}]"] = f"{{{{heart_{_i:02d}.png|heart{_i:02d}}}}}"

# Longest tokens first so "[heart01]" is not eaten by a shorter prefix.
ICON_ORDER = sorted(ICONS, key=len, reverse=True)


# --------------------------------------------------------------------------- fetch


def _get_json(path, **params):
    url = f"{API}/{path}"
    if params:
        url += "?" + urllib.parse.urlencode(params)
    req = urllib.request.Request(url, headers=HDRS)
    with urllib.request.urlopen(req, timeout=45) as r:
        return json.loads(r.read().decode("utf-8", "replace"))


def fetch_card_numbers(expansion):
    out, page = [], 1
    while True:
        data = _get_json("list", title=expansion, page=page, per_page=100, sort="no")
        items = data.get("items", [])
        if not items:
            break
        out += items
        if len(out) >= data.get("total", 0):
            break
        page += 1
    return out


def fetch_detail(card_id, tries=4):
    for attempt in range(tries):
        try:
            return _get_json("detail", id=card_id)["card"]
        except Exception as exc:  # noqa: BLE001 - transient upstream failures
            if attempt == tries - 1:
                raise RuntimeError(f"detail {card_id} failed: {exc}") from exc
            import time

            time.sleep(1.5 * (attempt + 1))


# ------------------------------------------------------------------- field mapping


def derive_series(card_no):
    for prefix, name in SERIES_PREFIXES:
        if card_no.startswith(prefix):
            return name
    return SERIES_DEFAULT


def product_name(expansion_name):
    """Local products drop the hyphen and the spaces: 'ブースターパック MELLOW MOMENT'
    -> 'ブースターパックMELLOWMOMENT'."""
    return re.sub(r"[-\s]+", "", expansion_name or "")


def to_ability(text):
    if not text or text == "-":
        return ""
    for token in ICON_ORDER:
        text = text.replace(token, ICONS[token])
    return text


def parse_blade_heart(raw):
    """'桃1' -> {'b_heart01': 1}; 'ALL1' -> {'b_all': 1}; '無2' -> {'b_heart07': 1}."""
    if not raw or raw in ("-", ""):
        return {}
    raw = raw.strip()
    if raw == "ALL1":
        return {"b_all": 1}
    m = re.fullmatch(r"(無)?([桃緑青赤黄紫])?(\d+)", raw)
    if not m:
        raise ValueError(f"unparsed blade_heart: {raw!r}")
    colorless, colour, count = m.groups()
    if colorless:
        # b_heart07 encodes as colour index 0, matching the existing 无-blade-heart cards.
        return {"b_heart07": 1}
    if colour is None:
        return {}
    return {"b_heart" + BLADE_HEART_IDX[colour]: int(count)}


def parse_hearts(card, include_colourless):
    out = {}
    for jp, idx in BASE_HEART_IDX.items():
        key = "heart0" if idx == "0" else f"heart{idx}"
        if idx == "0" and not include_colourless:
            continue
        value = card.get(key, "0")
        if value not in ("0", "-", "", None):
            out[key] = int(value)
    return out


def build_entry(card, product):
    card_no = card["card_number"]
    img_rel = card["picture"]  # e.g. PBLL02/PL!-pb2-E02-PE2.png
    entry = {
        "card_no": card_no,
        "img": f"{BASE}/wordpress/wp-content/images/cardlist/{img_rel}",
        "name": card["card_name"],
        "product": product,
        "type": card["card_kind"],
        "series": derive_series(card_no),
        "rare": card["rare"],
    }

    unit = (card.get("unit_name") or "").strip()
    if unit and unit != "-":
        entry["unit"] = unit

    kind = card["card_kind"]
    if kind == "メンバー":
        entry["cost"] = int(card["cost"])
        entry["base_heart"] = parse_hearts(card, include_colourless=False)
        if card["attack"] != "-":
            entry["blade"] = int(card["attack"])
        blade_hearts = parse_blade_heart(card.get("blade_heart"))
        if blade_hearts:
            entry["blade_heart"] = blade_hearts
    elif kind == "ライブ":
        # For live cards the API's `blade_heart` field carries the score, and the
        # blade hearts live in `attack`. This inversion is the API's, not ours.
        entry["score"] = int(card.get("blade_heart") or 0)
        entry["need_heart"] = parse_hearts(card, include_colourless=True)
        blade_hearts = parse_blade_heart(card.get("attack"))
        if blade_hearts:
            entry["blade_heart"] = blade_hearts
        if card.get("cost") in ("スコア", "ドロー"):
            entry["special_heart"] = {}

    ability = to_ability(card.get("text"))
    if ability:
        entry["ability"] = ability
    entry["faq"] = []
    entry["_img"] = f"img/cards/{img_rel}"
    return entry


def attach_rare_lists(entries):
    """rare_list groups the prints that share a card number (e.g. ...-001-R and
    ...-001-P＋), not cards that merely share a name."""
    by_base = defaultdict(list)
    for card_no in entries:
        by_base[card_no.rsplit("-", 1)[0]].append(card_no)
    for card_no, entry in entries.items():
        siblings = sorted(by_base[card_no.rsplit("-", 1)[0]])
        entry["rare_list"] = [{"card_no": c, "name": entries[c]["name"]} for c in siblings]


# ------------------------------------------------------------------------- images


async def download_images(entries, concurrency=10):
    try:
        from PIL import Image
    except ImportError:
        print("Pillow not installed - skipping WebP conversion (PNGs still saved)", file=sys.stderr)
        Image = None

    sem = asyncio.Semaphore(concurrency)

    async def one(session, card_no, entry):
        rel = entry["_img"][len("img/cards/"):]  # PBLL02/....png
        url = f"{BASE}/wordpress/wp-content/images/cardlist/{urllib.parse.quote(rel)}"
        raw = RAW_DIR / rel
        webp = WEBP_DIR / f"{card_no}.webp"
        if webp.exists() and raw.exists():
            return "skip"
        if not raw.exists():
            async with sem:
                raw.parent.mkdir(parents=True, exist_ok=True)
                r = await session.get(url, headers=HDRS)
                r.raise_for_status()
                raw.write_bytes(r.content)
        if Image is not None and not webp.exists():
            webp.parent.mkdir(parents=True, exist_ok=True)
            img = Image.open(raw)
            img.save(webp, "WEBP", quality=90)
        return "ok"

    import httpx

    todo = list(entries)
    async with httpx.AsyncClient(timeout=60) as session:
        results = await asyncio.gather(*(one(session, k, entries[k]) for k in todo))
    counts = defaultdict(int)
    for r in results:
        counts[r] += 1
    return dict(counts)


# -------------------------------------------------------------------------- main


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("expansion", help="expansion code, e.g. PBLL02")
    ap.add_argument("--no-images", action="store_true")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    cards = json.loads(CARDS_JSON.read_text(encoding="utf-8"))
    print(f"loaded {len(cards)} existing cards")

    listing = fetch_card_numbers(args.expansion)
    print(f"{args.expansion}: {len(listing)} cards listed")

    raw, expansion_name = {}, ""
    for item in listing:
        card = fetch_detail(item["id"])
        raw[card["card_number"]] = card
    if raw:
        first = next(iter(raw.values()))
        expansion_name = first.get("expansion_name") or ""
    if not expansion_name:
        # detail cards carry an empty expansion_name; fall back to the list payload
        expansion_name = listing[0].get("expansion_name") or ""
    product = product_name(expansion_name)
    print(f"product name: {product!r}")

    entries, skipped, replaced = {}, [], []
    for card_no, card in raw.items():
        entry = build_entry(card, product)
        if card_no in cards:
            (replaced if cards[card_no] != entry else skipped).append(card_no)
        entries[card_no] = entry
    attach_rare_lists(entries)

    new = [k for k in entries if k not in cards]
    print(f"new: {len(new)} | already present & identical: {len(skipped)} | changed: {len(replaced)}")
    if replaced:
        print("  CHANGED:", replaced[:20])
    if args.dry_run:
        sample = sorted(entries)[0]
        print(json.dumps(entries[sample], ensure_ascii=False, indent=1))
        return

    cards.update(entries)
    CARDS_JSON.write_text(json.dumps(dict(sorted(cards.items())), ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"wrote {CARDS_JSON} ({len(cards)} cards)")

    if not args.no_images:
        print("downloading images:", download_images_sync(entries))


def download_images_sync(entries):
    return asyncio.run(download_images(entries))


if __name__ == "__main__":
    main()
