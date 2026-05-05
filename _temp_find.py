import json
with open("cards/cards.json", "r", encoding="utf-8") as f:
    data = json.load(f)
# Find cards with "Printemps" in any field
for cid, c in data.items():
    if isinstance(c, dict):
        for k, v in c.items():
            if isinstance(v, str) and "Printemps" in v:
                print(f"{cid}.{k} = {v}")
            elif isinstance(v, list):
                for item in v:
                    if isinstance(item, str) and "Printemps" in item:
                        print(f"{cid}.{k}[] = {item}")
