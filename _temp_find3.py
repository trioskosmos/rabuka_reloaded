import json
with open("cards/cards.json", "r", encoding="utf-8") as f:
    data = json.load(f)
count = 0
for cid, c in data.items():
    if isinstance(c, dict) and c.get("unit") == "Printemps" and c.get("card_type") == "member_card":
        print(f"  {cid}: {c.get('name','')} cost={c.get('cost','N/A')}")
        count += 1
        if count >= 5:
            break
