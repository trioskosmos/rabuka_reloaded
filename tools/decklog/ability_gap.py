"""For each untested new ability, find the closest already-tested ability.

Mirrors the method in docs/PBLL02_ABILITY_REVIEW.md: text similarity to the closest
pre-existing ability, and whether that ability has a test to model on. A new ability whose
closest neighbour is untested - or whose text is far from anything tested - is the risky
one, because nothing in the suite would catch a wrong implementation.
"""

import difflib
import json
import re
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
AB = json.loads((REPO / "cards/abilities.json").read_text(encoding="utf-8"))
DB = json.loads((REPO / "cards/cards.json").read_text(encoding="utf-8"))
OLD = json.loads((REPO / "cards/cards.json.bak-before-pr").read_text(encoding="utf-8"))
TESTS = REPO / "engine" / "tests"

tested_ids = set()
for f in TESTS.rglob("*.rs"):
    tested_ids.update(re.findall(r'"((?:PL!|LL-|IKZL)[^"]+)"',
                                 f.read_text(encoding="utf-8", errors="replace")))


def covered(ability):
    for c in ability.get("cards", []):
        if c.split("|")[0].strip() in tested_ids:
            return True
    return False


def card_ids(ability):
    return [c.split("|")[0].strip() for c in ability.get("cards", [])]


abilities = AB["unique_abilities"]
by_card = {}
for a in abilities:
    for cid in card_ids(a):
        by_card.setdefault(cid, []).append(a)

new_cards = [k for k in DB if k not in OLD]
new_ids = set(new_cards)

new_abs = []
seen = set()
for cid in new_cards:
    for a in by_card.get(cid, []):
        if a["full_text"] in seen:
            continue
        seen.add(a["full_text"])
        new_abs.append((cid, a))

existing = [a for a in abilities
            if a["full_text"] not in seen and any(
                c.split("|")[0].strip() in tested_ids for c in a.get("cards", []))]
existing_untested = [a for a in abilities
                     if a["full_text"] not in seen and not covered(a)]

print(f"new abilities needing work : {len(new_abs)}")
print(f"pre-existing tested        : {len(existing)}")
print(f"pre-existing untested      : {len(existing_untested)}\n")

rows = []
for cid, a in new_abs:
    t = a.get("triggerless_text") or a.get("full_text") or ""
    best, score = None, 0.0
    for e in existing:
        et = e.get("triggerless_text") or e.get("full_text") or ""
        r = difflib.SequenceMatcher(None, t, et).ratio()
        if r > score:
            best, score = e, r
    rows.append((cid, a, best, score))

print(f"{'card':22} {'new':<44} {'closest tested ability':<44} {'sim':>5} {'risk':>5}")
print("-" * 126)
for cid, a, best, score in sorted(rows, key=lambda r: r[3]):
    t = (a.get("triggerless_text") or a.get("full_text") or "").replace("\n", " ")
    t = re.sub(r"\{\{[^}]*\}\}", lambda m: m.group(0).split("|")[-1].rstrip("}"), t)
    t = t[:42]
    bt = ""
    if best:
        bt = re.sub(r"\{\{[^}]*\}\}", lambda m: m.group(0).split("|")[-1].rstrip("}"),
                    (best.get("triggerless_text") or ""))[:42]
    risk = "HIGH" if score < 0.5 else ("med" if score < 0.8 else "low")
    print(f"{cid:22} {t:<44} {bt:<44} {score:5.2f} {risk:>5}")

out = REPO / "tools/decklog/ability_gap_analysis.json"
out.write_text(json.dumps(
    [{"card_no": cid,
      "ability": a.get("full_text"),
      "triggers": a.get("triggers"),
      "use_limit": a.get("use_limit"),
      "closest_tested": (best or {}).get("full_text"),
      "closest_cards": card_ids(best)[:4] if best else [],
      "similarity": round(score, 3),
      "risk": "HIGH" if score < 0.5 else ("med" if score < 0.8 else "low")}
     for cid, a, best, score in rows],
    ensure_ascii=False, indent=1), encoding="utf-8")
print(f"\nwrote {out}")