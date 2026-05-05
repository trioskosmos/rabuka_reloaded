import json
with open("cards/cards.json", "r", encoding="utf-8") as f:
    cards = json.load(f)
with open("cards/qa_data.json", "r", encoding="utf-8") as f:
    qa = json.load(f)

targets = [
    "PL!-pb1-001-R",  # Q166, Q167 - 高坂穂乃果
    "PL!HS-bp1-022-L", # Q107, Q36 - AWOKE
    "PL!S-bp3-019-L", # Q182, Q36 - MIRACLE WAVE
    "PL!S-pb1-002-R", # Q130, Q171 - 桜内梨子
    "PL!SP-bp2-001-R+", # Q106, Q171 - 澁谷かのん
    "PL!SP-bp4-023-L", # Q187, Q192 - Dazzling Game
]

for cid in targets:
    c = cards.get(cid, {})
    if c:
        print(f"\n=== {cid} ===")
        print(f"  name: {c.get('name','')}")
        print(f"  ability: {c.get('ability','')[:200]}")
        print(f"  type: {c.get('type','')}")

for e in qa:
    if e["id"] in ("Q166","Q167","Q107","Q182","Q130","Q106"):
        cards_found = [r["card_no"] for r in e.get("related_cards",[])]
        print(f"\n=== {e['id']} ===")
        print(f"  Q: {e['question'][:200]}")
        print(f"  A: {e['answer'][:150]}")
        print(f"  Cards: {cards_found[:3]}")
