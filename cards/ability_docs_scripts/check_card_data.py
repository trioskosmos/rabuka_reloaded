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
        "PL!HS-bp5-018-L",
        "PL!HS-bp2-020-L",
        "PL!HS-bp1-002-R",
        "PL!HS-bp2-022-L",
    ]:
        if cid in cards:
            c = cards[cid]
            out.write(f"{cid}:\n")
            for k in sorted(c.keys()):
                v = c[k]
                if isinstance(v, str) and len(v) > 80:
                    v = v[:80] + "..."
                out.write(f"  {k}: {v!r}\n")
        else:
            out.write(f"{cid}: MISSING\n")
        out.write("\n")
print("done")
