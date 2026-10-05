"""Report the cards used in decks but absent from cards/cards.json.

Pulls everything the harvested deck data already knows about them (name, rarity, image
path, cost/blade params) straight out of raw/<code>.json, and checks cards.json directly
for near-miss spellings so a renamed key is not mistaken for a missing card.
"""

import json
import re
from collections import Counter
from pathlib import Path

DB = Path("cards/cards.json")
RAW = Path("tools/decklog/raw")
BASE_RE = re.compile(r"^(?P<base>.+-(?:\d{3}|E\d{2}))(?P<rarity>.*)$")


def normalize(c):
    m = BASE_RE.match(c)
    return m.group("base") if m else c


def main():
    db = json.loads(DB.read_text(encoding="utf-8"))
    db_keys = set(db)
    db_norm = {normalize(k) for k in db}

    missing = {}          # full card number -> info
    used_counts = Counter()
    for f in RAW.glob("*.json"):
        try:
            d = json.loads(f.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        if not isinstance(d, dict):
            continue
        for it in (d.get("list") or []) + (d.get("sub_list") or []):
            cn = it.get("card_number")
            if not cn:
                continue
            used_counts[cn] += it.get("num") or 0
            if cn in db_keys or normalize(cn) in db_norm:
                continue
            rec = missing.setdefault(cn, {
                "card_number": cn, "base": normalize(cn), "name": it.get("name"),
                "rare": it.get("rare"), "img": it.get("img"),
                "card_kind": it.get("card_kind"), "blade": it.get("blade"),
                "blade_heart": it.get("blade_heart"), "max": it.get("max"),
                "p_param": it.get("p_param"), "g_param": it.get("g_param"),
                "copies": 0, "decks": 0,
            })
            rec["copies"] += it.get("num") or 0
            rec["decks"] += 1

    print(f"cards.json entries      : {len(db)}")
    print(f"distinct card numbers   : {len(db_norm)}")
    print(f"cards used in decks     : {len(used_counts)}")
    print(f"MISSING from cards.json : {len(missing)}\n")

    if not missing:
        print("nothing missing")
        return

    for cn, info in sorted(missing.items(), key=lambda kv: -kv[1]["copies"]):
        base = info["base"]
        # near-miss: does the db hold this card number under another printing?
        cousins = sorted(k for k in db_norm if k == base)
        print(f"{cn}")
        print(f"   name  : {info['name']}   rare={info['rare']}  "
              f"kind={info['card_kind']}  blade={info['blade']}  max={info['max']}")
        print(f"   img   : {info['img']}")
        print(f"   used  : {info['copies']} copies across {info['decks']} decks")
        print(f"   p_param: {json.dumps(info['p_param'], ensure_ascii=False)}")
        print(f"   db printings of {base}: {cousins if cousins else 'NONE'}")
        print()

    out = Path("tools/decklog/missing_cards.json")
    out.write_text(json.dumps(sorted(missing.values(), key=lambda r: -r["copies"]),
                                ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"wrote {out}")


if __name__ == "__main__":
    main()