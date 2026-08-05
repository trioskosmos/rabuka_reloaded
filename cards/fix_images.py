"""Fix img URLs (+ -> 2), download remaining images, convert all to webp."""

import json
import urllib.parse
import urllib.request
from pathlib import Path
from PIL import Image

BASE = "https://llofficial-cardgame.com"
CARDS_JSON = Path(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\cards.json")
WEBP_DIR = Path(
    r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\web_ui\img\cards_webp"
)
RAW_DIR = Path(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\images")

NEW_EXPANSIONS = {"BP07", "NSD02"}


def img_filename(card_no):
    return card_no.replace("＋", "2")


def main():
    cards = json.loads(CARDS_JSON.read_text(encoding="utf-8"))

    # Fix img/_img URLs for new cards
    fixed = 0
    for cn, entry in cards.items():
        exp = None
        if entry.get("product") == "ブースターパック MELLOW MOMENT":
            exp = "BP07"
        elif (
            entry.get("product")
            == "スタートデッキ ラブライブ！虹ヶ咲学園スクールアイドル同好会 cheer"
        ):
            exp = "NSD02"
        if not exp:
            continue
        if "＋" in cn:
            fname = img_filename(cn)
            entry["img"] = (
                f"{BASE}/wordpress/wp-content/images/cardlist/{exp}/{fname}.png"
            )
            entry["_img"] = f"img/cards/{exp}/{fname}.png"
            fixed += 1
    print(f"Fixed {fixed} img URLs")

    # Download + convert
    ok = fail = 0
    for cn, entry in cards.items():
        exp = None
        if entry.get("product") == "ブースターパック MELLOW MOMENT":
            exp = "BP07"
        elif (
            entry.get("product")
            == "スタートデッキ ラブライブ！虹ヶ咲学園スクールアイドル同好会 cheer"
        ):
            exp = "NSD02"
        if not exp:
            continue
        webp = WEBP_DIR / f"{cn}.webp"
        if webp.exists():
            ok += 1
            continue
        fname = img_filename(cn)
        img_url = f"{BASE}/wordpress/wp-content/images/cardlist/{exp}/{urllib.parse.quote(fname)}.png"
        raw = RAW_DIR / exp / f"{fname}.png"
        try:
            raw.parent.mkdir(parents=True, exist_ok=True)
            if not raw.exists():
                req = urllib.request.Request(
                    img_url, headers={"User-Agent": "Mozilla/5.0"}
                )
                with urllib.request.urlopen(req, timeout=30) as resp:
                    raw.write_bytes(resp.read())
            Image.open(raw).convert("RGB").save(webp, "WEBP", quality=90)
            ok += 1
        except Exception as e:
            print(f"FAIL {cn}: {e}")
            fail += 1
    print(f"Images: {ok} ok, {fail} failed")

    CARDS_JSON.write_text(
        json.dumps(cards, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    print("cards.json updated")


if __name__ == "__main__":
    main()
