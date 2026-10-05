import json
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
db = json.loads((REPO / "cards/cards.json").read_text(encoding="utf-8"))
old = json.loads((REPO / "cards/cards.json.bak-before-pr").read_text(encoding="utf-8"))
new = [k for k in db if k not in old]
rows = json.loads((REPO / "tools/decklog/new_abilities_untested.json").read_text(encoding="utf-8"))

for r in rows:
    c = db[r["card_no"]]
    print(f"=== {r['card_no']}  {c.get('name','')}  cost={c.get('cost')} "
          f"blade={c.get('blade')} hearts={c.get('base_heart')} bh={c.get('blade_heart')}")
    print(f"    {c.get('ability')}")
    print()