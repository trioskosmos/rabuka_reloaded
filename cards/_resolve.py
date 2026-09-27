"""Resolve an inventory `base` to a real card_no, and show the printed text.

The coverage report stores a 12-character PREFIX of the card number, so resolving
means "the card whose card_no starts with this prefix", taking the LONGEST such
match. Taking `base[:11]` and taking the first hit instead -- which is what an
earlier version of the tooling did -- picks an arbitrary card sharing 11
characters, and is how a text once got attributed to the wrong card entirely.

Run from the `cards` directory:

    python _resolve.py PL!S-bp7-010-N PL!SP-bp1-015-N
"""
import json
from pathlib import Path

CARDS = json.loads(Path("cards.json").read_text(encoding="utf-8"))


def resolve(base):
    """The canonical card for a report `base`: the longest card_no with that prefix."""
    hits = [k for k in CARDS if k.startswith(base)]
    if not hits:
        return None
    return max(hits, key=len)


def printed(base, limit=300):
    k = resolve(base)
    if not k:
        return None, None, "NO SUCH CARD"
    c = CARDS[k]
    return k, c.get("name"), (c.get("ability") or "NO ABILITY")[:limit]


if __name__ == "__main__":
    import sys

    for b in sys.argv[1:]:
        k, name, text = printed(b)
        print(f"=== {b} -> {k}")
        if k:
            print(f"  {name}")
            print(f"  {text}")
        else:
            print("  NO SUCH CARD")
        print()
