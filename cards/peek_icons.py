import io, json
from pathlib import Path

ROOT = Path(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded")
cards = json.load(io.open(ROOT / "cards" / "cards.json", encoding="utf-8"))


def icons(c):
    sh = c.get("special_heart") or {}
    bh = c.get("blade_heart") or {}
    out = set()
    if isinstance(sh, dict):
        if sh.get("score"):
            out.add("score")
        if sh.get("draw"):
            out.add("draw")
    if isinstance(bh, dict) and bh.get("b_all"):
        out.add("ball")
    return frozenset(out)


buckets = {}
for no, c in cards.items():
    if c.get("type") != "ライブ":
        continue
    k = icons(c)
    buckets.setdefault(k, []).append(no)

for k in sorted(buckets, key=lambda s: (len(s), sorted(s))):
    if len(k) <= 2:
        v = buckets[k]
        print(f"{sorted(k) or ['(none)']}: {len(v):4d}  e.g. {v[:3]}")
