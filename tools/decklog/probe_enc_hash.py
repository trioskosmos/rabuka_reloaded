"""Is enc a reversible function of the deck code (or the numeric id)?

If it is a plain keyed hash with a static salt, every enc from the search endpoint can be
inverted offline and no login is needed at all. Uses only known pairs already on disk.
"""

import hashlib
import itertools
import json
from pathlib import Path

CHECKPOINT = Path("C:/Users/trios/Downloads/search_meta_partial_p100.jsonl")
RAW = Path("tools/decklog/raw")


def load_pairs():
    pairs = []
    if CHECKPOINT.exists():
        for ln in CHECKPOINT.read_text(encoding="utf-8").splitlines():
            if ln.strip():
                r = json.loads(ln)
                if r.get("code") and r.get("enc"):
                    pairs.append((r["code"], r["enc"]))
    ids = {}
    for f in RAW.glob("*.json"):
        try:
            d = json.loads(f.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        if isinstance(d, dict) and d.get("id") and d.get("deck_id"):
            ids[d["deck_id"]] = d["id"]
    return pairs, ids


SALTS = [
    "", "11", "LLC", "llc", "decklog", "DECKLOG", "bushiroad", "BushiRoad",
    "decklog.bushiroad.com", "bushiroad.com", "card", "loveca", "ラブカ",
    "ll", "secret", "salt", "deck", "decklog11", "11decklog", "sysdeck",
    "system", "app", "api", "detail", "enc", "decklog_secret", "BushiroadDeck",
]
HASHES = {
    "md5": hashlib.md5,
    "sha1": hashlib.sha1,
    "sha256": hashlib.sha256,
    "sha512": hashlib.sha512,
}


def candidates(value):
    for s in SALTS:
        for sep in ("", ":", "_", "-", "|"):
            yield s + sep + value
            yield value + sep + s


def main():
    pairs, ids = load_pairs()
    print(f"known (code, enc) pairs: {len(pairs)}")
    print(f"known (code, id) pairs:  {len(ids)}")
    if not pairs:
        return
    probe = pairs[:6]
    print(f"probing {len(probe)} pairs\n")

    hits = []
    for hname, h in HASHES.items():
        for order in ("salt_first", "value_first", "salt_mid"):
            for sep in ("", ":", "|", "-", "_"):
                for s in SALTS:
                    ok = True
                    for code, enc in probe:
                        if order == "salt_first":
                            src = f"{s}{sep}{code}"
                        elif order == "value_first":
                            src = f"{code}{sep}{s}"
                        else:
                            src = f"{s}{sep}{code}{sep}{s}"
                        d = h(src.encode()).hexdigest()
                        if not enc.startswith(d[:32]):
                            ok = False
                            break
                    if ok:
                        hits.append((hname, order, repr(sep), repr(s)))
                        print(f"MATCH  {hname} order={order} sep={sep!r} salt={s!r}")

    # id-based
    idpairs = [(str(i), e) for c, e in pairs if c in ids for i in [ids[c]]]
    if idpairs:
        print(f"\nprobing {len(idpairs)} (id, enc) pairs")
        for hname, h in HASHES.items():
            for order in ("salt_first", "value_first"):
                for sep in ("", ":", "|", "-"):
                    for s in SALTS:
                        ok = True
                        for i, enc in idpairs[:6]:
                            src = f"{s}{sep}{i}" if order == "salt_first" else f"{i}{sep}{s}"
                            if not enc.startswith(h(src.encode()).hexdigest()[:32]):
                                ok = False
                                break
                        if ok:
                            print(f"MATCH  {hname} order={order} sep={sep!r} salt={s!r}")
                            hits.append((hname, order, sep, s))

    if not hits:
        print("\nno keyed-hash construction matched.")
        print("-> enc is not a static-salt hash of the code or the id (a random or")
        print("   server-side-keyed token, most likely a random DB column).")
    else:
        print(f"\n{len(hits)} constructions match; enc is invertible offline.")


if __name__ == "__main__":
    main()