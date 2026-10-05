"""What did the BP07 run actually add, and can the API be searched by name?"""

import importlib.util
import json
import sys
import urllib.parse
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
CARDSDIR = HERE.parent.parent / "cards"
spec = importlib.util.spec_from_file_location("sx", CARDSDIR / "scrape_expansion.py")
sys.path.insert(0, str(CARDSDIR))
sx = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sx)

before = json.loads((CARDSDIR / "cards.json.bak-after-pr").read_text(encoding="utf-8"))
after = json.loads((CARDSDIR / "cards.json").read_text(encoding="utf-8"))
added = [k for k in after if k not in before]
print(f"BP07 run added {len(added)} cards:")
for k in added:
    print(f"  {k:24} {after[k].get('name','')}")

print("\n999-series already present BEFORE any of my runs:")
pre = json.loads((CARDSDIR / "cards.json.bak-before-pr").read_text(encoding="utf-8"))
for k in sorted(k for k in pre if "999" in k):
    print(f"  {k:24} {pre[k].get('name','')}")

BASE = "https://llofficial-cardgame.com"
API = f"{BASE}/manage/card-list-user"
HDRS = {"user-agent": "Mozilla/5.0", "referer": f"{BASE}/cardlist/searchresults/",
        "accept": "application/json"}
print("\ntrying name search params for '嵐 千砂都' / bp4-999:")
for params in (
    {"keyword": "999"},
    {"card_name": "999"},
    {"search": "999"},
    {"name": "999"},
    {"title": "BP04", "keyword": "999"},
):
    url = f"{API}/list?" + urllib.parse.urlencode(params)
    try:
        with urllib.request.urlopen(urllib.request.Request(url, headers=HDRS), timeout=30) as r:
            body = json.loads(r.read().decode("utf-8", "replace"))
        txt = json.dumps(body, ensure_ascii=False)
        print(f"  {str(params)[:44]:46} len={len(txt):6}  has999={'999' in txt}")
        if "bp4-999" in txt:
            print("      FOUND:", [c for c in sx.fetch_card_numbers_from(body)
                                    if "999" in c] if hasattr(sx, "fetch_card_numbers_from") else txt[:300])
    except Exception as exc:
        print(f"  {str(params)[:44]:46} ERR {str(exc)[:50]}")