import json
d = json.load(open("../cards/cards.json", encoding="utf-8"))
a = json.load(open("../cards/abilities.json", encoding="utf-8"))
ca = set()
for e in a["unique_abilities"]:
    for c in e.get("cards", []):
        ca.add(c.split(" |")[0])
for cid, card in d.items():
    if card.get("type") != "メンバー" or cid in ca:
        continue
    s = card.get("series", "")
    if "スーパースター" in s:
        print(f"{cid}: {card.get('name','?')}")
