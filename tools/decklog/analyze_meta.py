"""Analyse a search_meta*.jsonl checkpoint from export_codes_browser.js.

    python tools/decklog/analyze_meta.py C:/Users/trios/Downloads/search_meta_partial_p100.jsonl
    python tools/decklog/analyze_meta.py <file> --spot-check 10
"""

import argparse
import json
import random
import re
import sys
from collections import Counter
from pathlib import Path

def _load_sibling():
    import importlib.util
    here = Path(__file__).resolve().parent
    candidates = [here, Path.cwd() / "tools" / "decklog", here.parent]
    for d in candidates:
        f = d / "scrape_decklog.py"
        if f.exists():
            spec = importlib.util.spec_from_file_location("scrape_decklog", f)
            mod = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(mod)
            return mod.fetch_deck, mod.record_from_raw, mod.flat_cards
    raise SystemExit("could not locate scrape_decklog.py next to this script")


fetch_deck, record_from_raw, flat_cards = _load_sibling()

JAPANESE = re.compile(r"[\u3040-\u30ff\u4e00-\u9fff]")


def load(path):
    rows, bad = [], 0
    for i, ln in enumerate(Path(path).read_text(encoding="utf-8").splitlines(), 1):
        ln = ln.strip()
        if not ln:
            continue
        try:
            rows.append(json.loads(ln))
        except json.JSONDecodeError as e:
            bad += 1
            if bad <= 3:
                print(f"  line {i}: unparseable ({e})")
    return rows, bad


def pct(n, d):
    return f"{100.0 * n / d:5.1f}%" if d else "  n/a"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("path")
    ap.add_argument("--spot-check", type=int, default=0)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--top", type=int, default=12)
    args = ap.parse_args()

    rows, bad = load(args.path)
    n = len(rows)
    print(f"file            : {args.path}")
    print(f"records         : {n}   unparseable lines: {bad}")

    codes = [r.get("code") for r in rows]
    encs = [r.get("enc") for r in rows]
    uc, ue = len(set(codes)), len(set(encs))
    print(f"unique codes    : {uc}  duplicates: {n - uc}")
    print(f"unique encs     : {ue}  duplicates: {n - ue}")
    print(f"code/enc 1:1    : {'yes' if uc == ue else 'NO - mismatch'}")

    print("\n-- field completeness --")
    for f in ("code", "enc", "title", "player_name", "publish_date", "comment",
              "regulation", "fav_count"):
        missing = sum(1 for r in rows if r.get(f) in (None, "", []))
        print(f"  {f:14} present {pct(n - missing, n)}  missing {missing}")

    print("\n-- regulation --")
    for k, v in Counter(r.get("regulation") or "(empty)" for r in rows).most_common():
        print(f"  {k:12} {v:5}  {pct(v, n)}")

    print("\n-- publish_date range --")
    dates = sorted(r["publish_date"] for r in rows if r.get("publish_date"))
    if dates:
        print(f"  earliest {dates[0][:10]}    latest {dates[-1][:10]}")
        print("  by day:")
        for day, c in sorted(Counter(d[:10] for d in dates).items(), reverse=True):
            print(f"    {day}  {c:5}  {pct(c, n)}")

    print("\n-- fav_count --")
    favs = [r.get("fav_count") or 0 for r in rows]
    zero = sum(1 for f in favs if f == 0)
    print(f"  zero-fav decks : {zero}  {pct(zero, n)}")
    print(f"  max            : {max(favs)}   mean {sum(favs) / n:.2f}")
    if max(favs) > 0:
        print("  top by favs:")
        for r in sorted(rows, key=lambda r: -(r.get("fav_count") or 0))[:args.top]:
            print(f"    {r.get('fav_count'):5}  {r['code']:8} {(r.get('title') or '')[:34]}")

    print("\n-- most common titles --")
    for t, c in Counter((r.get("title") or "").strip() for r in rows).most_common(args.top):
        if t:
            print(f"  {c:4}  {t[:50]}")

    print("\n-- most prolific owners --")
    for name, c in Counter((r.get("player_name") or "").strip() for r in rows).most_common(args.top):
        if name:
            print(f"  {c:4}  {name}")

    print("\n-- text sanity --")
    mojibake = sum(1 for r in rows if "\ufffd" in (r.get("title") or "") + (r.get("player_name") or ""))
    jp = sum(1 for r in rows if JAPANESE.search(r.get("title") or ""))
    print(f"  titles with U+FFFD replacement chars : {mojibake}")
    print(f"  titles containing Japanese           : {jp}  {pct(jp, n)}")

    if args.spot_check:
        sample = random.Random(args.seed).sample(rows, min(args.spot_check, n))
        print(f"\n-- live spot check: resolving {len(sample)} codes --")
        ok = irregular = 0
        for r in sample:
            code = r["code"]
            try:
                rec = record_from_raw(fetch_deck(code), code)
            except Exception as exc:
                print(f"  {code:8} FETCH FAILED {exc}")
                continue
            good = rec["total"] in (30, 40, 60)
            ok += 1
            irregular += 0 if good else 1
            print(f"  {code:8} {rec['total']:3} cards  {len(rec['energy']):2} energy  "
                  f"{len(flat_cards(rec)):3} distinct  {'ok' if good else 'IRREGULAR'}  "
                  f"{(rec.get('title') or '')[:26]}")
        print(f"  resolved {ok}/{len(sample)}   irregular {irregular}")
        mism = [r for r in sample
                if r.get("title") and r["title"] != (record_from_raw(fetch_deck(r['code']), r['code']).get('title') or '')]
        if mism:
            print(f"  title mismatches search-vs-data: {len(mism)}")
            for r in mism[:5]:
                print(f"    {r['code']}: search={r['title']!r} data-fetched title differs")


if __name__ == "__main__":
    main()