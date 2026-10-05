import json
from pathlib import Path

missing = json.loads(Path("tools/decklog/missing_cards.json").read_text(encoding="utf-8"))
db = json.loads(Path("cards/cards.json").read_text(encoding="utf-8"))
dbn = {k.upper(): k for k in db}

print(f"cards.json now: {len(db)} cards\n")
found = nomatch = noability = 0
for m in missing:
    want = m["card_number"]
    key = dbn.get(want.upper()) or dbn.get(want.upper().replace("＋", "+"))
    if key is None:
        print(f"  STILL MISSING  {want}")
        nomatch += 1
        continue
    c = db[key]
    ab = (c.get("ability") or "").strip()
    if not ab and c.get("type") != "エネルギー":
        print(f"  no ability     {key}  ({c.get('type')})")
        noability += 1
    else:
        found += 1

print(f"\nresolved with data: {found}")
print(f"still missing     : {nomatch}")
print(f"present, no ability: {noability}")

for want in ("PL!S-PR-046-PR", "PL!HS-PR-040-PR", "PL!HS-PR-042-PR＋"):
    k = dbn.get(want.upper()) or dbn.get(want.upper().replace("＋", "+"))
    if k:
        c = db[k]
        print(f"\n{k}  cost={c.get('cost')} blade={c.get('blade')} "
              f"hearts={c.get('base_heart')} bh={c.get('blade_heart')}")
        print(f"  ability: {c.get('ability')}")