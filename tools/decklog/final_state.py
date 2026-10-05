import json
from pathlib import Path

d = json.loads(Path("cards/cards.json").read_text(encoding="utf-8"))
print(f"cards.json: {len(d)} cards")
for k in ("PL!SP-bp4-999-SEC＋", "PL!HS-PR-040-PR", "PL!-PR-022-PR", "PL!S-PR-046-PR"):
    c = d.get(k) or d.get(k.replace("＋", "+"))
    if not c:
        print(f"  {k}: NOT FOUND")
        continue
    ab = (c.get("ability") or "").replace("\n", " / ")
    print(f"  {k:22} {c.get('name','')}")
    print(f"      {ab[:110]}")

print()
print("missing-cards audit:")
import subprocess
r = subprocess.run(["python", "tools/decklog/find_missing_cards.py"],
                   capture_output=True, text=True)
for line in r.stdout.splitlines()[:4]:
    print("  " + line)