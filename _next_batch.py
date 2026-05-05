import json
with open("cards/cards.json","r",encoding="utf-8") as f:
    cards = json.load(f)
with open("cards/qa_data.json","r",encoding="utf-8") as f:
    qa = json.load(f)

# Next untested cards from the list after eli
targets = {
    "PL!-sd1-006-SD": "Q125",  # 西木野真姫
    "PL!-sd1-019-SD": "Q36",   # START:DASH!!
    "PL!S-bp2-004-R": "Q107",  # 黒澤ダイヤ (re-yell)
    "PL!SP-bp2-011-R": "Q118", # 鬼塚冬毬 
    "PL!-pb1-008-R": "Q183",   # 小泉花陽
    "PL!-pb1-009-R": "Q180",   # 矢澤にこ
    "PL!-pb1-013-R": "Q176",   # 園田海未
}

for cid, qid in targets.items():
    c = cards.get(cid, {})
    if c and c.get("ability"):
        print(f"\n--- {cid} ({c.get('name','')[:15]}) QA={qid} ---")
        print(f"  {c['ability'][:200]}")
    # Find the QA
    for e in qa:
        if e["id"] == qid:
            print(f"  Q: {e['question'][:200]}")
            print(f"  A: {e['answer'][:150]}")
            print(f"  Cards: {[r['card_no'] for r in e.get('related_cards',[])]}")
