"""Full-corpus deck analysis: dedupe, combo clustering, card frequency, coverage.

Consumes the raw API JSON in raw/ (source of truth) and produces:

  analysis_report.txt      human-readable findings
  combos.jsonl             one record per unique card combo, with member deck codes
  deduped_decks.txt        one deck per unique combo, web-UI import format
  unique_combos.txt        bare combo keys, one per line

A "combo" is the card list with rarity stripped (rarity is cosmetic) and energy cards
excluded (every energy card does the same job). Two decks differing only in foil or in
which energy they run are therefore the same combo.
"""

import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

BASE_RE = re.compile(r"^(?P<base>.+-(?:\d{3}|E\d{2}))")

CARD_DB = "cards/cards.json"


def base_card(card_no):
    return BASE_RE.match(card_no).group("base") if BASE_RE.match(card_no) else card_no


def load_raw(raw_dir):
    decks = []
    for f in sorted(Path(raw_dir).glob("*.json")):
        try:
            d = json.loads(f.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        if not isinstance(d, dict):
            continue
        items = d.get("list") or []
        sub = d.get("sub_list") or []
        if not items:
            continue
        counts, lives, members, energy = {}, {}, {}, {}
        for it in items:
            cid, n = base_card(it["card_number"]), int(it.get("num") or 0)
            if n <= 0:
                continue
            counts[cid] = counts.get(cid, 0) + n
            tgt = lives if it.get("card_kind") == "L" else members
            tgt[cid] = tgt.get(cid, 0) + n
        ecounts = {}
        for it in sub:
            cid, n = base_card(it["card_number"]), int(it.get("num") or 0)
            if n > 0:
                ecounts[cid] = ecounts.get(cid, 0) + n
        decks.append({
            "code": f.stem,
            "title": d.get("title") or "",
            "regulation": d.get("deck_param1") or "",
            "has_error": d.get("has_recipe_error") or 0,
            "total": sum(counts.values()),
            "energy_total": sum(ecounts.values()),
            "combo": tuple(sorted(counts.items())),
            "members": members,
            "lives": lives,
            "energy": ecounts,
        })
    return decks


def load_owner_map(path):
    out = {}
    p = Path(path)
    if not p.exists():
        return out
    for ln in p.read_text(encoding="utf-8").splitlines():
        ln = ln.strip()
        if not ln:
            continue
        try:
            r = json.loads(ln)
        except json.JSONDecodeError:
            continue
        if r.get("code"):
            import html
            out[r["code"]] = {
                "owner": html.unescape(r.get("player_name") or ""),
                "date": (r.get("publish_date") or "")[:10],
                "fav": r.get("fav_count") or 0,
            }
    return out


def jaccard(a, b):
    sa, sb = set(a), set(b)
    if not sa or not sb:
        return 0.0
    return len(sa & sb) / len(sa | sb)


def owner_stats(decks, owners):
    """Split a combo's decks by attribution.

    Bushiroad leaves `player_name` empty on some decks, and it is heavily concentrated on
    the widely-copied archetype lists. Conflating "hidden" with "one person" would call a
    12x shared official list a single player's collection, so keep them apart.
    """
    names = {owners.get(d["code"], {}).get("owner", "") for d in decks}
    names.discard("")
    hidden = sum(1 for d in decks if not owners.get(d["code"], {}).get("owner", ""))
    return sorted(names), hidden


def classify(decks, owners):
    names, hidden = owner_stats(decks, owners)
    if not names:
        return "all owners hidden"
    if len(names) == 1:
        return f"1 visible owner{f' (+{hidden} hidden)' if hidden else ''}"
    return f"{len(names)} visible owners"
    sa, sb = set(a), set(b)
    if not sa or not sb:
        return 0.0
    return len(sa & sb) / len(sa | sb)


def cluster(decks, threshold):
    """Bucket by live-card set, then greedily cluster by member-card Jaccard."""
    buckets = defaultdict(list)
    for d in decks:
        buckets[tuple(sorted(d["lives"].items()))].append(d)
    clusters = []
    for live_key, group in buckets.items():
        reps = []
        for d in group:
            placed = False
            for c in reps:
                if jaccard(c["members"], d["members"]) >= threshold:
                    c["members_list"].append(d)
                    placed = True
                    break
            if not placed:
                reps.append({"live_key": live_key, "members": d["members"],
                             "members_list": [d]})
        clusters.extend(reps)
    return clusters


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw-dir", default="tools/decklog/raw")
    ap.add_argument("--owners", default="tools/decklog/search_meta.jsonl")
    ap.add_argument("--threshold", type=float, default=0.8)
    ap.add_argument("--out-dir", default="tools/decklog/analysis")
    ap.add_argument("--min-cluster", type=int, default=2)
    args = ap.parse_args()

    out = Path(args.out_dir)
    out.mkdir(parents=True, exist_ok=True)
    R = []

    def say(s=""):
        R.append(s)
        print(s)

    decks = load_raw(args.raw_dir)
    owners = load_owner_map(args.owners)
    say("=" * 74)
    say("DECKLOG CORPUS ANALYSIS")
    say("=" * 74)
    say(f"raw decks read      : {len(decks)}")
    if not decks:
        raise SystemExit("no raw decks yet")

    regs = Counter(d["regulation"] for d in decks)
    say(f"regulation          : {dict(regs)}")
    err = sum(1 for d in decks if d["has_error"])
    say(f"flagged has_error   : {err}")

    # Energy is deliberately ignored here: every energy card does the same job, so it
    # never decides whether a list is interesting. Legality is the main deck's size.
    legal = [d for d in decks if d["regulation"] == "S" and d["total"] == 60]
    half = [d for d in decks if d["regulation"] == "H" and d["total"] == 30]
    say(f"legal S (60 main) : {len(legal)}  ({100.0*len(legal)/len(decks):.1f}%)")
    say(f"half decks (30)   : {len(half)}")
    main_ok = [d for d in decks if d["total"] in (30, 60)]
    say(f"main total 30 or 60: {len(main_ok)}")
    say(f"dropped (other)   : {len(decks) - len(main_ok)}")

    pool = legal or main_ok or decks
    say(f"\nanalysing {len(pool)} decks")

    combos = defaultdict(list)
    for d in pool:
        combos[d["combo"]].append(d)
    say(f"unique combos       : {len(combos)}")
    uniq_ratio = len(combos) / len(pool) if pool else 0
    say(f"distinctness        : {uniq_ratio:.3f} unique decks per deck")
    dupes = sorted(((len(v), k) for k, v in combos.items()), reverse=True)
    say(f"decks discarded as duplicates: {len(pool) - len(combos)}")

    say("\nduplicate combos    : %d" % sum(1 for n, _ in dupes if n > 1))
    buckets = Counter()
    for n, key in dupes:
        if n >= 2:
            buckets[classify(combos[key], owners)] += 1
    say("  attribution split:")
    for k, v in buckets.most_common():
        say(f"    {v:5} combos  {k}")

    named_total = sum(1 for d in pool
                      if owners.get(d["code"], {}).get("owner", ""))
    say(f"\nowner names visible: {named_total}/{len(pool)} "
        f"({100.0*named_total/len(pool):.1f}%)  hidden: {len(pool)-named_total}")

    say("\n--- most repeated exact combos ---")
    for n, key in dupes[:15]:
        say(f"  {n:4}x  {len(key):2} cards  {classify(combos[key], owners)}")
        say(f"        deck(s): {', '.join(d['code'] for d in combos[key][:6])}")

    say("\n--- card lists for the 10 most repeated combos (rarity stripped, energy excluded) ---")
    for n, key in dupes[:10]:
        ds = combos[key]
        say(f"  combo x{n}  ({len(key)} distinct cards, main total "
            f"{sum(c for _, c in key)})  rep={ds[0]['code']}  {ds[0]['title']}")
        names = Counter(owners.get(d["code"], {}).get("owner", "") or "(hidden)" for d in ds)
        say(f"    owners: {dict(names)}")
        say(f"    dates : {sorted({owners.get(d['code'], {}).get('date','?') for d in ds})[:6]}")
        cards = sorted(key, key=lambda kv: (-kv[1], kv[0]))
        for i in range(0, len(cards), 6):
            say("    " + "  ".join(f"{c} x{q}" for c, q in cards[i:i + 6]))
        say("")

    say("\n--- near-duplicate clusters (live-set bucket + member Jaccard) ---")
    clusters = cluster(pool, args.threshold)
    clusters.sort(key=lambda c: len(c["members_list"]), reverse=True)
    multi_c = [c for c in clusters if len(c["members_list"]) >= args.min_cluster]
    say(f"clusters (>= {args.min_cluster} decks): {len(multi_c)} of {len(clusters)}")
    say(f"decks inside clusters          : {sum(len(c['members_list']) for c in multi_c)}")
    for c in multi_c[:12]:
        ds = c["members_list"]
        names = Counter(owners.get(d["code"], {}).get("owner", "") or "(hidden)" for d in ds)
        lives = " ".join(f"{cid}x{n}" for cid, n in c["live_key"])
        say(f"  {len(ds):4}x  owners={len(names)}  lives: {lives[:88]}")

    freq = Counter()
    for d in pool:
        for cid, n in d["combo"]:
            freq[cid] += n
    deck_freq = Counter()
    for d in pool:
        for cid, _n in d["combo"]:
            deck_freq[cid] += 1
    say("\n--- most played cards (by copies) ---")
    for cid, n in freq.most_common(25):
        say(f"  {n:7} copies  in {deck_freq[cid]:5} decks ({100.0*deck_freq[cid]/len(pool):5.1f}%)  {cid}")

    if Path(CARD_DB).exists():
        db = json.loads(Path(CARD_DB).read_text(encoding="utf-8"))
        known = {base_card(k) for k in db}
        used = set(freq)
        say(f"\n--- coverage vs {CARD_DB} ---")
        say(f"  printings in db        : {len(db)}")
        say(f"  distinct card numbers  : {len(known)}  (rarity collapsed)")
        say(f"  distinct cards used    : {len(used)}")
        unknown = sorted(used - known)
        say(f"  used but NOT in db     : {len(unknown)}")
        for cid in unknown[:15]:
            say(f"      {cid}  ({freq[cid]} copies in {deck_freq[cid]} decks)")
        never = sorted(known - used)
        say(f"  in db, never played    : {len(never)}")
        by_series = Counter(c.split("-")[1] if "-" in c else c for c in never)
        say(f"      by product         : {dict(by_series.most_common(10))}")

    energy_used = Counter()
    for d in pool:
        for cid, n in d["energy"].items():
            energy_used[cid] += n
    say(f"\n--- energy cards (excluded from combo keys) ---")
    say(f"  distinct energy cards used: {len(energy_used)}")
    for cid, n in energy_used.most_common(8):
        say(f"  {n:7} copies  {cid}")

    fav = [owners[d["code"]]["fav"] for d in pool if d["code"] in owners]
    if fav:
        say(f"\n--- favourites (from search meta, {len(fav)} decks matched) ---")
        say(f"  zero-fav: {sum(1 for f in fav if f == 0)}  max: {max(fav)}  mean {sum(fav)/len(fav):.2f}")

    with open(out / "combos.jsonl", "w", encoding="utf-8") as fh:
        for key, ds in sorted(combos.items(), key=lambda kv: -len(kv[1])):
            names = Counter(owners.get(d["code"], {}).get("owner", "") or "(hidden)" for d in ds)
            fh.write(json.dumps({
                "cards": dict(key),
                "copies": len(ds),
                "deck_codes": [d["code"] for d in ds],
                "titles": sorted({d["title"] for d in ds if d["title"]}),
                "owners": names.most_common(),
                "energy_options": len({tuple(sorted(d["energy"].items())) for d in ds}),
            }, ensure_ascii=False) + "\n")
    say(f"\nwrote {out/'combos.jsonl'}")

    with open(out / "deduped_decks.txt", "w", encoding="utf-8") as fh:
        for key, ds in sorted(combos.items(), key=lambda kv: -len(kv[1])):
            d = ds[0]
            fh.write(f"# combo x{len(ds)}  rep={d['code']}  {d['title']}\n")
            fh.write(f"# members {sum(d['members'].values())} / lives {sum(d['lives'].values())}\n")
            for cid, n in sorted(d["combo"]):
                fh.write(f"{cid} x {n}\n")
            fh.write("\n")
    say(f"wrote {out/'deduped_decks.txt'} ({len(combos)} decks)")

    with open(out / "unique_combos.txt", "w", encoding="utf-8") as fh:
        for key, _ in sorted(combos.items(), key=lambda kv: -len(kv[1])):
            fh.write("|".join(f"{c}x{n}" for c, n in key) + "\n")
    say(f"wrote {out/'unique_combos.txt'}")

    (out / "analysis_report.txt").write_text("\n".join(R) + "\n", encoding="utf-8")
    print(f"\nwrote {out/'analysis_report.txt'}")


if __name__ == "__main__":
    main()