"""Restore FAQ entries the scraper emptied, and report new-ability test coverage.

The official detail response does not carry FAQ for every card, so re-scraping blanked the
`faq` array on 53 pre-existing cards. That is real data loss of curated content, so it is
restored from the backup - the scraper's other changes are kept.

Then the question that was actually asked: which of the newly added cards bring
abilities the Rust engine has no test coverage for. Coverage uses the repo's own documented
method (cards/test_inventory.py): an ability counts as tested if any card id in its
`cards` array appears in a test file under engine/tests/.
"""

import json
import re
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
CARDS = REPO / "cards" / "cards.json"
BAK = REPO / "cards" / "cards.json.bak-before-pr"
ABILITIES = REPO / "cards" / "abilities.json"
TESTS = REPO / "engine" / "tests"


def restore_faq():
    old = json.loads(BAK.read_text(encoding="utf-8"))
    new = json.loads(CARDS.read_text(encoding="utf-8"))
    restored = []
    for k, v in new.items():
        if k in old and isinstance(v.get("faq"), list) and not v["faq"]:
            prior = old[k].get("faq")
            if isinstance(prior, list) and prior:
                v["faq"] = prior
                restored.append(k)
    if restored:
        CARDS.write_text(json.dumps(new, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"restored faq on {len(restored)} cards: {restored[:8]}{'...' if len(restored) > 8 else ''}")
    return restored


def test_card_ids():
    ids = set()
    for f in TESTS.rglob("*.rs"):
        try:
            ids.update(re.findall(r'"((?:PL!|LL-|IKZL)[^"]+)"', f.read_text(encoding="utf-8", errors="replace")))
        except OSError:
            pass
    return ids


def main():
    restore_faq()

    db = json.loads(CARDS.read_text(encoding="utf-8"))
    old = json.loads(BAK.read_text(encoding="utf-8"))
    new_cards = [k for k in db if k not in old]
    print(f"\nnew cards: {len(new_cards)}")

    ab = json.loads(ABILITIES.read_text(encoding="utf-8"))
    abilities = ab.get("unique_abilities", [])
    by_cards = defaultdict(list)
    for a in abilities:
        for c in a.get("cards", []):
            # entries look like 'PL!S-PR-046-PR | 渡辺 曜 (ab#0)' - key on the id only
            card_id = c.split("|")[0].strip()
            by_cards[card_id].append(a)

    tested = test_card_ids()
    print(f"card ids referenced in engine/tests: {len(tested)}")

    rows = []
    for card_no in sorted(new_cards):
        entries = by_cards.get(card_no, [])
        if db[card_no].get("type") == "エネルギー" or not entries:
            continue
        covered = card_no in tested
        rows.append({
            "card_no": card_no,
            "name": db[card_no].get("name", ""),
            "cost": db[card_no].get("cost"),
            "blade": db[card_no].get("blade"),
            "abilities": len(entries),
            "covered": covered,
        })

    untested = [r for r in rows if not r["covered"]]
    print(f"\nnew member cards with abilities : {len(rows)}")
    print(f"  covered by an existing test   : {len(rows) - len(untested)}")
    print(f"  NO test references them       : {len(untested)}\n")
    print(f"{'card':24} {'cost':>4} {'blade':>5} {'ab':>3}  name")
    for r in sorted(untested, key=lambda r: (-(r["blade"] or 0), r["cost"] or 0)):
        print(f"{r['card_no']:24} {str(r['cost']):>4} {str(r['blade']):>5} "
              f"{r['abilities']:>3}  {r['name']}")

    out = REPO / "tools/decklog/new_abilities_untested.json"
    out.write_text(json.dumps(untested, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"\nwrote {out}")


if __name__ == "__main__":
    main()