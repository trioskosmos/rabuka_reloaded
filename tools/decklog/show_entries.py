import json
from pathlib import Path

e = json.loads(Path("tools/decklog/cards_additions.json").read_text(encoding="utf-8"))
mem = [x for x in e if x.get("type") == "メンバー" and x.get("ability")]
en = [x for x in e if x.get("type") == "エネルギー"]
null = [x for x in e if x.get("ability") is None]
print(f"total entries   : {len(e)}")
print(f"energy (no ability): {len(en)}")
print(f"members with ability: {len(mem)}")
print(f"null ability    : {len(null)}")
print()
for x in mem:
    print(f"  {x['card_no']:20} cost={x['cost']:2}  blade={x['blade']}  {x['name']}")
print()
for x in en:
    print(f"  {x['card_no']:20} energy  {x['name']}")