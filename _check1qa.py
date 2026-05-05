import json
with open("cards/cards.json","r",encoding="utf-8") as f:
    cards = json.load(f)
with open("cards/qa_data.json","r",encoding="utf-8") as f:
    qa = json.load(f)

# Pick a few 1-QA cards and show their abilities
targets = ["PL!-bp3-002-R","PL!-bp3-003-R","PL!-bp4-009-R",
           "PL!-pb1-008-R","PL!-pb1-009-R","PL!-pb1-013-R",
           "PL!-pb1-015-R","PL!-pb1-030-L","PL!-pb1-031-L",
           "PL!-sd1-002-SD","PL!-sd1-006-SD","PL!-sd1-019-SD",
           "PL!S-bp2-004-R","PL!S-bp2-005-R+","PL!SP-bp2-011-R"]

for cid in targets:
    c = cards.get(cid, {})
    if c and c.get("ability"):
        print(f"\n--- {cid} ---")
        print(f"  {c['ability'][:200]}")
