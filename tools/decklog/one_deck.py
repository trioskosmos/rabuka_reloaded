"""Print the cards in one harvested deck, and derive its rarity-stripped combo key.

    python tools/decklog/one_deck.py --index 337
    python tools/decklog/one_deck.py --code XGD0
    python tools/decklog/one_deck.py --index 337 --no-energy

Combo key = the card list with rarity stripped. In Loveca the N / R / M printings of a
card number are the same card, so decks that differ only in foil collapse to one combo.
R+ (`R＋`) is left distinct because it is a genuinely different card, not a reprint.
Energy cards are excluded by default: they all do the same job.
"""

import argparse
import importlib.util
import json
import re
from pathlib import Path

RARITY_RE = re.compile(r"^(?P<base>.+-(?:\d{3}|E\d{2}))(?P<rarity>.*)$")


def load_module():
    here = Path(__file__).resolve().parent
    for d in (here, Path.cwd() / "tools" / "decklog", here.parent):
        f = d / "scrape_decklog.py"
        if f.exists():
            spec = importlib.util.spec_from_file_location("scrape_decklog", f)
            mod = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(mod)
            return mod
    raise SystemExit("cannot find scrape_decklog.py")


def strip_rarity(card_no):
    """Collapse every printing of a card onto its card number.

    The card number is everything up to and including the 3-digit number; whatever
    follows is the printing (N, R, M, P, PR, SEC, SD, SD2, RM, AR, L, R＋, ...). Rarity
    is cosmetic, so decks that differ only in foil collapse to one combo.
    """
    m = RARITY_RE.match(card_no)
    return m.group("base") if m else card_no


def combo_key(items, include_energy=False, strip=True):
    counts = {}
    for it in items:
        cid = it["card_number"]
        if not include_energy and it.get("card_kind") == "E":
            continue
        if strip:
            cid = strip_rarity(cid)
        counts[cid] = counts.get(cid, 0) + it["num"]
    return tuple(sorted(counts.items()))


def main():
    ap = argparse.ArgumentParser()
    src = ap.add_mutually_exclusive_group(required=True)
    src.add_argument("--code")
    src.add_argument("--index", type=int, help="1-based position in the codes list")
    ap.add_argument("--codes", default="C:/Users/trios/Downloads/search_meta_partial_p100.jsonl")
    ap.add_argument("--no-energy", action="store_true")
    ap.add_argument("--json-out")
    args = ap.parse_args()

    rows = [json.loads(l) for l in Path(args.codes).read_text(encoding="utf-8").splitlines() if l.strip()]
    if args.code:
        code = args.code
        meta = next((r for r in rows if r.get("code") == code), {})
    else:
        if not 1 <= args.index <= len(rows):
            raise SystemExit(f"index out of range: have {len(rows)} codes")
        meta = rows[args.index - 1]
        code = meta["code"]

    mod = load_module()
    data = mod.fetch_deck(code)

    print(f"deck code   : {code}")
    print(f"deck title  : {data.get('title') or '(none)'}")
    print(f"owner       : {meta.get('player_name') or '(hidden)'}   "
          f"published {meta.get('publish_date', '?')[:10]}   fav {meta.get('fav_count')}")
    print(f"regulation  : {data.get('deck_param1')}   game_title_id {data.get('game_title_id')}")

    main_list = data.get("list") or []
    energy = data.get("sub_list") or []

    print(f"\n--- MEMBERS + LIVES ({sum(i['num'] for i in main_list)}) ---")
    for kind, label in (("M", "member"), ("L", "live ")):
        rows_out = [i for i in main_list if i.get("card_kind") == kind]
        if not rows_out:
            continue
        print(f"  [{label}]")
        for i in sorted(rows_out, key=lambda x: x["card_number"]):
            flag = "" if strip_rarity(i["card_number"]) == i["card_number"] else "  (reprint)"
            print(f"    {i['num']}x  {i['card_number']:18} {i.get('rare', ''):3} {i.get('name', '')}{flag}")

    print(f"\n--- ENERGY ({sum(i['num'] for i in energy)}) ---")
    for i in energy:
        print(f"    {i['num']}x  {i['card_number']:18} {i.get('rare', ''):3} {i.get('name', '')}")

    key = combo_key(main_list, include_energy=False)
    print(f"\ncombo key (rarity-stripped, energy excluded): {len(key)} distinct cards")
    print("  " + " | ".join(f"{c}x{n}" for c, n in key))
    print("\ncombo key with rarity kept:")
    key2 = combo_key(main_list, include_energy=False, strip=False)
    print("  " + " | ".join(f"{c}x{n}" for c, n in key2))

    if args.json_out:
        Path(args.json_out).write_text(json.dumps(
            {"code": code, "title": data.get("title"),
             "combo_stripped": dict(key), "combo_with_rarity": dict(key2)},
            ensure_ascii=False, indent=1), encoding="utf-8")
        print(f"\nwrote {args.json_out}")


if __name__ == "__main__":
    main()