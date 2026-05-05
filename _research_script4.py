import json, os
os.chdir(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded")

card_ids = ["PL!N-bp3-030-L", "PL!N-bp1-011-R", "PL!-bp3-025-L"]

# 1. cards.json
cards = json.load(open("cards/cards.json","r",encoding="utf-8"))
print("="*70)
print("SECTION 1: RAW ABILITY TEXT FROM cards.json")
print("="*70)
for cid in card_ids:
    card = cards.get(cid, {})
    print()
    print("--- {} ({}) ---".format(cid, card.get("name","?")))
    print("ability: {}".format(card.get("ability","(none)")))

# 2. abilities.json
abilities = json.load(open("cards/abilities.json","r",encoding="utf-8"))
print()
print("="*70)
print("SECTION 2: CURRENT PARSER OUTPUT FROM abilities.json")
print("="*70)
ua_list = abilities.get("unique_abilities", [])
print("Total unique abilities: {}".format(len(ua_list)))
for cid in card_ids:
    print()
    print("--- Searching for: {} ---".format(cid))
    found = False
    for i, ua in enumerate(ua_list):
        cards_list = ua.get("cards", [])
        if cid in cards_list:
            print("  Found in unique_abilities[{}]".format(i))
            print("  full_text: {}".format(ua.get("full_text","")))
            print("  triggerless_text: {}".format(ua.get("triggerless_text","")))
            print("  triggers: {}".format(json.dumps(ua.get("triggers",[]), ensure_ascii=False)))
            print("  use_limit: {}".format(ua.get("use_limit")))
            print("  is_null: {}".format(ua.get("is_null")))
            print("  cost: {}".format(ua.get("cost")))
            print("  effect: {}".format(json.dumps(ua.get("effect"), ensure_ascii=False, indent=2)))
            found = True
            break
    if not found:
        print("  NOT FOUND in unique_abilities")

# 3. qa_data.json
qa = json.load(open("cards/qa_data.json","r",encoding="utf-8"))
print()
print("="*70)
print("SECTION 3: QA ENTRIES FROM qa_data.json")
print("="*70)
print("Total QA entries: {}".format(len(qa)))
if len(qa) > 0:
    print("First entry keys: {}".format(list(qa[0].keys())))
    print("First entry sample: {}".format(json.dumps(qa[0], ensure_ascii=False)[:300]))

for cid in card_ids:
    print()
    print("--- QA entries for: {} ---".format(cid))
    matches = []
    for entry in qa:
        related = entry.get("related_cards", [])
        for rc in related:
            if isinstance(rc, dict) and rc.get("card_no") == cid:
                matches.append(entry)
                break
            elif isinstance(rc, str) and cid in rc:
                matches.append(entry)
                break
    if matches:
        for m in matches:
            print("  QA id: {}".format(m.get("id")))
            print("  date: {}".format(m.get("date")))
            print("  Q: {}".format(m.get("question","")))
            print("  A: {}".format(m.get("answer","")))
            print("  related_cards: {}".format(json.dumps(m.get("related_cards",[]), ensure_ascii=False)))
            print()
    else:
        print("  (none found)")

print()
print("Done.")
