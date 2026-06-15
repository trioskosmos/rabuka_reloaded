import json

with open(
    r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\cards.json",
    "r",
    encoding="utf-8",
) as f:
    cards = json.load(f)

with open(
    r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\ability_docs_scripts\_card_data.txt",
    "w",
    encoding="utf-8",
) as out:
    for cid in [
        "PL!-sd1-019-SD",
        "PL!-sd1-010-SD",
        "PL!-sd1-001-SD",
        "PL!-sd1-013-SD",
        "PL!-sd1-002-SD",
        "PL!-sd1-003-SD",
    ]:
        if cid in cards:
            c = cards[cid]
            out.write(
                f"{cid}: type={c.get('type')!r}, unit={c.get('unit')!r}, series={c.get('series')!r}, name={c.get('name')!r}\n"
            )
        else:
            out.write(f"{cid}: MISSING\n")
print("done")
