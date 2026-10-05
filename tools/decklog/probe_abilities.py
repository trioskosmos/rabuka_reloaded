import json
import re
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
ab = json.loads((REPO / "cards/abilities.json").read_text(encoding="utf-8"))
print("abilities.json top-level keys:", list(ab)[:10])
ua = ab.get("unique_abilities", [])
print(f"unique_abilities: {len(ua)}")
if ua:
    print("entry keys:", list(ua[0]))
    print("sample cards field:", ua[0].get("cards", [])[:6])

txt = (REPO / "cards/abilities.json").read_text(encoding="utf-8")
for probe in ("PL!S-PR-046-PR", "PL!S-PR-046", "PL!-PR-022-PR", "PR-046",
              "PL!SP-bp4-999-SEC＋", "bp4-999"):
    print(f"  {probe:22} in abilities.json: {probe in txt}")

db = json.loads((REPO / "cards/cards.json").read_text(encoding="utf-8"))
old = json.loads((REPO / "cards/cards.json.bak-before-pr").read_text(encoding="utf-8"))
new = [k for k in db if k not in old]
members = [k for k in new if db[k].get("type") == "メンバー" and (db[k].get("ability") or "")]
print(f"\nnew members with non-empty ability: {len(members)}")
for k in members[:12]:
    print(f"  {k:24} {(db[k].get('ability') or '')[:70]}")