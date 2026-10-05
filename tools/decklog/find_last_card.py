"""Locate the one remaining card (PL!SP-bp4-999-SEC＋) in the official API.

999-series cards are secret promos. Two of them came from BP07, so check what we now
hold and search the expansions that could plausibly own a bp4 secret.
"""

import json
import re
import sys
import urllib.parse
import urllib.request
from pathlib import Path

BASE = "https://llofficial-cardgame.com"
API = f"{BASE}/manage/card-list-user"
HDRS = {
    "user-agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
    "referer": f"{BASE}/cardlist/searchresults/",
    "accept": "application/json",
}
WANT = "PL!SP-bp4-999-SEC＋"


def get_json(path, **params):
    url = f"{API}/{path}?" + urllib.parse.urlencode(params)
    req = urllib.request.Request(url, headers=HDRS)
    with urllib.request.urlopen(req, timeout=40) as r:
        return json.loads(r.read().decode("utf-8", "replace"))


def main():
    db = json.loads(Path("cards/cards.json").read_text(encoding="utf-8"))
    have = sorted(k for k in db if "999" in k)
    print(f"999-series cards now in cards.json ({len(have)}):")
    for k in have:
        print(f"  {k}  {db[k].get('name','')}")
    print(f"\nlooking for: {WANT}\n")

    expansions = ["BP07", "PBLL02", "PBLL", "PR", "BP01", "BP02", "BP03",
                  "BP04", "BP05", "BP06", "PLSD01", "PB01", "PB02", "SPSD01"]
    expansions += [f"PBSP{i:02d}" for i in range(1, 6)]
    expansions += [f"PBLL{i:02d}" for i in range(1, 4)]
    expansions += [f"BP{i:02d}" for i in range(1, 12)]

    for exp in dict.fromkeys(expansions):
        try:
            res = get_json("list", title=exp, page=1, per_page=100, sort="no")
        except Exception as exc:
            print(f"  {exp:8} ERR {str(exc)[:50]}")
            continue
        data = res.get("data") if isinstance(res, dict) else None
        cards = (data or {}).get("list") or res.get("list") or []
        if not cards:
            continue
        hit = [c for c in cards if "bp4-999" in (c.get("card_no") or "")]
        if hit:
            print(f"  FOUND in {exp}:")
            for c in hit:
                print(f"    {json.dumps(c, ensure_ascii=False)[:400]}")
            return
    print("  not present in any listed expansion (secret cards are often unlisted)")


if __name__ == "__main__":
    main()