import io, json
from pathlib import Path

ROOT = Path(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded")
cards = json.load(io.open(ROOT / "cards" / "cards.json", encoding="utf-8"))
d = json.load(io.open(ROOT / "cards" / "abilities.json", encoding="utf-8"))

for no in ["PL!-pb2-008-PP", "PL!-pb2-018-P＋", "PL!-pb2-026-N", "PL!-pb2-032-N"]:
    c = cards.get(no, {})
    print("=" * 72)
    print(no, "|", c.get("name"), "|", c.get("unit"))
    for a in d["unique_abilities"]:
        if no in [x.split("|")[0].strip() for x in (a.get("cards") or [])]:
            cost = a.get("cost")
            print(f"  trigger={a.get('triggers')!r}")
            print(f"    cost: {json.dumps(cost, ensure_ascii=False)[:200] if cost else None}")
            print(f"    effect action: {(a.get('effect') or {}).get('action')}")
