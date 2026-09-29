import io, json
from pathlib import Path

ROOT = Path(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded")
cards = json.load(io.open(ROOT / "cards" / "cards.json", encoding="utf-8"))

seen, out = set(), []
for no, c in cards.items():
    if c.get("type") == "メンバー" and c.get("unit") == "Liella!":
        if c.get("name") in seen:
            continue
        seen.add(c.get("name"))
        out.append((c.get("cost"), no, c.get("name")))
out.sort()
print(f"distinct Liella! members: {len(out)}")
for cost, no, name in out[:10]:
    print(f"  cost {cost}  {no:<22} {name}")

# Sanity: what group do the cards the DUO test currently uses report?
print()
for n in ["PL!-PR-001-PR", "PL!-PR-002-PR"]:
    c = cards.get(n, {})
    print(n, "->", c.get("unit"), "| series", c.get("series"))
