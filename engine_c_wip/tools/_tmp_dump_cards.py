import json
db = json.load(open(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json", encoding="utf-8"))
ua = db["unique_abilities"]
items = list(ua.items()) if isinstance(ua, dict) else list(enumerate(ua))
for cid, c in items:
    txt = json.dumps(c, ensure_ascii=False)
    if "PL!N-bp4-026-L" in txt or "PL!N-bp7-022-N" in txt or "PL!N-bp5-019-N" in txt:
        print("=== idx", cid)
        print(json.dumps(c, ensure_ascii=False, indent=1))
        print()
