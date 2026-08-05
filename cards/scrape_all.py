"""Complete scraper for BP07 and NSD02 cards.

1. Enumerate all card numbers via text-view list + cardsearch_ex pagination
2. Fetch full stats (cost, hearts, blade, score, unit, ability, name) via detail API
3. Build cards.json entries matching existing format
4. Download card images and convert to WebP
"""

import json
import os
import re
import time
import urllib.parse
import urllib.request
from pathlib import Path

BASE = "https://llofficial-cardgame.com"
HDRS = {
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
    "X-Requested-With": "XMLHttpRequest",
}

CARDS_JSON = Path(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\cards.json")
WEBP_DIR = Path(
    r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\web_ui\img\cards_webp"
)
RAW_DIR = Path(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\images")

PRODUCT_BP07 = "ブースターパック MELLOW MOMENT"
PRODUCT_NSD02 = "スタートデッキ ラブライブ！虹ヶ咲学園スクールアイドル同好会 cheer"


def fetch(url, data=None):
    req = urllib.request.Request(url, data=data if data else None, headers=HDRS)
    with urllib.request.urlopen(req, timeout=20) as resp:
        return resp.read().decode("utf-8", errors="replace")


def enumerate_cards(expansion, pages):
    """Return set of card_no from text list pagination."""
    seen = {}
    # Page 1 comes from the main searchresults page
    url = f"{BASE}/cardlist/searchresults/?expansion={expansion}&view=text&sort=new"
    html = fetch(url)
    parse_list(html, seen)
    # Additional pages via cardsearch_ex
    for pg in range(2, pages + 1):
        url = f"{BASE}/cardlist/cardsearch_ex?expansion={expansion}&view=text&page={pg}&t={int(time.time() * 1000)}"
        try:
            html = fetch(url)
        except Exception as e:
            print(f"  page {pg} failed: {e}")
            continue
        parse_list(html, seen)
        time.sleep(0.2)
    return seen


def parse_list(html, seen):
    for m in re.finditer(r'card="([^"]+)"', html):
        seen[m.group(1)] = True


def fetch_detail(card_no):
    ts = int(time.time() * 1000)
    url = f"{BASE}/cardlist/detail/?t={ts}"
    data = f"cardno={card_no}".encode("utf-8")
    req = urllib.request.Request(
        url,
        data=data,
        headers={
            "User-Agent": HDRS["User-Agent"],
            "Content-Type": "application/x-www-form-urlencoded",
            "Referer": f"{BASE}/cardlist/searchresults/",
            "X-Requested-With": "XMLHttpRequest",
        },
    )
    with urllib.request.urlopen(req, timeout=20) as resp:
        return resp.read().decode("utf-8", errors="replace")


def parse_detail(html, card_no):
    """Extract all fields from detail HTML."""
    info = {"card_no": card_no}
    fields = {}
    for m in re.finditer(
        r'<div class="dl-Item">\s*<dt><span>(.*?)</span></dt>\s*<dd>(.*?)</dd>',
        html,
        re.DOTALL,
    ):
        raw_dd = m.group(2)
        label = re.sub(r"<[^>]+>", "", m.group(1)).strip()
        text = re.sub(r"<[^>]+>", "", raw_dd).strip()
        # hearts special
        if "ハート" in label:
            hearts = {}
            for hm in re.finditer(r'icon\s+heart(\d+)">(\d+)', raw_dd):
                hearts[f"heart{hm.group(1)}"] = int(hm.group(2))
            fields["hearts"] = hearts
        else:
            fields[label] = text
    info.update(fields)

    # name + ability
    m = re.search(r'<p class="info-Heading">(.*?)</p>', html)
    if m:
        info["name"] = unescape(m.group(1).strip())
    m = re.search(
        r'<p class="info-Text">(.*?)</p>\s*<div class="info-Switch"', html, re.DOTALL
    )
    if m:
        ability_html = m.group(1)
        ability = unescape(re.sub(r"<[^>]+>", "", ability_html))
        ability = ability.replace("\n", " ").strip()
        info["ability"] = ability
    else:
        info["ability"] = ""
    return info


def unescape(s):
    return (
        s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#039;", "'")
        .replace("&quot;", '"')
        .replace("<br />", "\n")
    )


def map_series(card_no):
    if "PL!N" in card_no:
        return "ラブライブ！虹ヶ咲学園スクールアイドル同好会"
    if "PL!S" in card_no:
        return "ラブライブ！サンシャイン!!"
    if "PL!SP" in card_no:
        return "ラブライブ！スーパースター!!"
    if "LL-" in card_no or "LL!" in card_no:
        return "ラブライブ！"
    return ""


def get_unit(info):
    return info.get("参加ユニット", "")


def build_entry(info, product, expansion):
    card_no = info["card_no"]
    card_type = info.get("カードタイプ", "")
    series_raw = info.get("作品名", "")
    if not series_raw:
        series_raw = map_series(card_no)
    rare = info.get("レアリティ", "")

    entry = {
        "card_no": card_no,
        "img": f"{BASE}/wordpress/wp-content/images/cardlist/{expansion}/{img_filename(card_no)}.png",
        "name": info.get("name", ""),
        "product": product,
        "type": card_type,
        "series": series_raw,
        "rare": rare,
        "faq": [],
        "rare_list": [{"card_no": card_no, "name": info.get("name", "")}],
        "_img": f"img/cards/{expansion}/{img_filename(card_no)}.png",
    }
    unit = get_unit(info)
    if unit:
        entry["unit"] = unit

    if "コスト" in info:
        entry["cost"] = int(info["コスト"])
    if "ブレード" in info:
        entry["blade"] = int(info["ブレード"])
    if "スコア" in info:
        entry["score"] = int(info["スコア"])
    hearts = info.get("hearts")
    if hearts:
        entry["base_heart"] = hearts
    ability = info.get("ability", "")
    if ability:
        entry["ability"] = ability
    return entry


def img_filename(card_no):
    return card_no.replace("＋", "2")


def download_and_convert(card_no, expansion):
    fname = img_filename(card_no)
    img_url = f"{BASE}/wordpress/wp-content/images/cardlist/{expansion}/{urllib.parse.quote(fname)}.png"
    raw = RAW_DIR / expansion / f"{fname}.png"
    webp = WEBP_DIR / f"{card_no}.webp"
    if not webp.exists():
        try:
            raw.parent.mkdir(parents=True, exist_ok=True)
            req = urllib.request.Request(
                img_url, headers={"User-Agent": HDRS["User-Agent"]}
            )
            with urllib.request.urlopen(req, timeout=30) as resp:
                raw.write_bytes(resp.read())
            from PIL import Image

            Image.open(raw).save(webp, "WEBP", quality=90)
        except Exception as e:
            print(f"    img fail {card_no}: {e}")


def process_expansion(expansion_pages, product, expansion, extra_cardnos=None):
    print(f"\n=== {expansion} : {product} ===")
    seen = enumerate_cards(expansion, expansion_pages)
    if extra_cardnos:
        for cn in extra_cardnos:
            seen[cn] = True
    cardnos = sorted(seen)
    print(f"  Found {len(cardnos)} card numbers")

    entries = {}
    for i, cn in enumerate(cardnos):
        ok = False
        for attempt in range(3):
            try:
                html = fetch_detail(cn)
                if html.startswith("NG") or not html.strip():
                    time.sleep(1)
                    continue
                info = parse_detail(html, cn)
                entries[cn] = build_entry(info, product, expansion)
                print(
                    f"  [{i + 1}/{len(cardnos)}] {cn} {info.get('name', '')} cost={info.get('cost', '-')} blade={info.get('blade', '-')} score={info.get('score', '-')}"
                )
                ok = True
                break
            except Exception as e:
                print(f"    retry {cn}: {e}")
                time.sleep(1)
        if not ok:
            print(f"  FAILED (3 tries): {cn}")
        download_and_convert(cn, expansion)
        time.sleep(0.15)
    return entries


def main():
    if CARDS_JSON.exists():
        cards = json.loads(CARDS_JSON.read_text(encoding="utf-8"))
    else:
        cards = {}
    print(f"Loaded {len(cards)} existing cards")

    # BP07: 13 pages. Extra: reprint card PL!S-bp2-023-SECL not in bp7 listing
    bp07 = process_expansion(
        13, PRODUCT_BP07, "BP07", extra_cardnos={"PL!S-bp2-023-SECL"}
    )
    # NSD02: 3 pages
    nsd02 = process_expansion(3, PRODUCT_NSD02, "NSD02")

    cards.update(bp07)
    cards.update(nsd02)

    sorted_cards = dict(sorted(cards.items()))
    CARDS_JSON.write_text(
        json.dumps(sorted_cards, ensure_ascii=False, indent=2), encoding="utf-8"
    )

    print(f"\nAdded {len(bp07)} BP07 + {len(nsd02)} NSD02")
    print(f"Total cards now: {len(sorted_cards)}")


if __name__ == "__main__":
    main()
