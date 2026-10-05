"""Is there a public endpoint that returns full card data (ability text included)?

The 21 cards missing from cards.json need their ability text, and the repo's extractor
only parses what cards.json already contains. Probes a handful of plausible endpoints
for one known card number.
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from scrape_decklog import get, post, headers, BASE  # noqa: E402

CARD = "PL!HS-PR-038-PR"
NAME = "セラス 柳田 リリエンフェルト"


def show(label, body):
    if body is None:
        print(f"  {label:44} empty")
        return
    txt = body if isinstance(body, str) else json.dumps(body, ensure_ascii=False)
    has_ability = any(k in txt for k in ("ability", "起動", "常時", "自動", "ライブ成功"))
    print(f"  {label:44} len={len(txt):6}  ability_text={has_ability}")
    if has_ability:
        print(f"      {txt[:300]}")


print(f"probing for full card data: {CARD} ({NAME})\n")

print("-- decklog card endpoints --")
for label, url in [
    ("cardparam/11", f"{BASE}/system/app/api/cardparam/11"),
    (f"cards/{CARD}", f"{BASE}/system/app/api/cards/{CARD}"),
    (f"card/{CARD}", f"{BASE}/system/app/api/card/{CARD}"),
    (f"cardlist/{CARD}", f"{BASE}/system/app/api/cardlist/{CARD}"),
    (f"detail/{CARD}", f"{BASE}/system/app/api/detail/{CARD}"),
    ("search/cards", f"{BASE}/system/app/api/search/cards"),
]:
    try:
        show(label, get(url, f"{BASE}/search?c=11"))
    except Exception as exc:
        print(f"  {label:44} ERR {str(exc)[:50]}")

print("\n-- official card list site --")
for label, url in [
    ("cardlist search", "https://llofficial-cardgame.com/cardlist/"),
    ("cardlist detail", f"https://llofficial-cardgame.com/cardlist/detail/?cardno={CARD}"),
]:
    try:
        body = get(url, "https://llofficial-cardgame.com/")
        txt = body if isinstance(body, str) else json.dumps(body, ensure_ascii=False)
        has = any(k in txt for k in ("起動", "常時", "自動", "ライブ成功"))
        print(f"  {label:44} len={len(txt):6}  ability_text={has}")
        if CARD in txt or NAME in txt:
            print(f"      FOUND the card. snippet: {txt[:200]}")
    except Exception as exc:
        print(f"  {label:44} ERR {str(exc)[:50]}")