import io, json
from collections import Counter
from pathlib import Path

ROOT = Path(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded")
cards = json.load(io.open(ROOT / "cards" / "cards.json", encoding="utf-8"))

ser = Counter(c.get("series") for c in cards.values() if c.get("type") == "メンバー")
print("series among members:")
for s, n in ser.most_common():
    print(f"  {n:5d}  {s}")

print()
target = "\u30e9\u30d6\u30e9\u30a4\u30d6\uff01\u30b5\u30f3\u30b7\u30e3\u30a4\u30f3"
seen, out = set(), []
for no, c in cards.items():
    if c.get("type") == "\u30e1\u30f3\u30d0\u30fc" and c.get("series") == target:
        if c.get("name") in seen:
            continue
        seen.add(c.get("name"))
        out.append((c.get("cost"), no, c.get("name"), c.get("unit")))
out.sort()
print(f"distinct members of {target}: {len(out)}")
for cost, no, name, unit in out[:10]:
    print(f"  cost {cost}  {no:<22} {name}  unit={unit}")
