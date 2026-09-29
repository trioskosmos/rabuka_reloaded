"""Audit the imported PBLL02 set against the official card site, card by card.

Run:  python cards/audit_vs_official.py

Checks, each of which fails differently:
  1. CARDS    \u2014 every official card number is held locally, and every parallel printing
               (the API's own `parallel_param`) is present.
  2. NODES    \u2014 every card whose official text is non-empty has at least one ability node.
  3. TEXT     \u2014 the ability string we hold equals the official one, after normalising the
               three markup differences that are NOT content differences:
                 official `\u3010\u30e9\u30a4\u30d6\u958b\u59cb\u6642\u3011`  vs  ours `{{live_start.png|\u30e9\u30a4\u30d6\u958b\u59cb\u6642}}`
                 official `[heart01]`        vs  ours `{{heart_01.png|heart01}}`
                 official `-`               means "no ability"
  4. CLAUSES  \u2014 every clause of the official text is present in some node or cost. Matched
               against the effect's own `text`, not `triggerless_text`: the latter keeps
               the trigger gate and so never shares a prefix with a bare clause. An
               earlier version of this check used `triggerless_text` and reported 30
               false gaps.

A finding here is either a real gap or a normalisation bug in this file. Three such
bugs were fixed while writing it, and each is noted at the point it bit.
"""
import io
import json
import re
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPANSION = "PBLL02"
LIST = "https://llofficial-cardgame.com/manage/card-list-user/list"
DETAIL = "https://llofficial-cardgame.com/manage/card-list-user/detail"
NO_ABILITY = {"-", "", "\u2015", "\u2014", "\u2013"}

cards = json.load(io.open(ROOT / "cards" / "cards.json", encoding="utf-8"))
abil = json.load(io.open(ROOT / "cards" / "abilities.json", encoding="utf-8"))


def get(url):
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.loads(r.read().decode("utf-8"))


def clean(s):
    """Reduce every spelling of an icon reference or trigger to its visible word."""
    s = re.sub(r"\{\{[^|]*\|([^}]*)\}\}", r"\1", s or "")
    s = re.sub(r"\[([^\[\]]*)\]", r"\1", s)
    s = re.sub(r"\u3010([^\u3011]*)\u3011", r"\1", s)
    return s


def norm(s):
    return re.sub(r"[\s\u3000\uff0f/]+", "", clean(s))


# A node's `triggerless_text` carries no trigger and an effect's `text` does not either,
# so a leading trigger has to come off the official clause before a prefix can match.
# Without this the check reports every clause as missing: 105 false gaps, all of the
# form 「ライブ開始時自分の…」 against a node that starts 「自分の…」.
TRIGGERS = (
    "\u5e38\u6642|\u81ea\u52d5|\u767b\u5834|\u8d77\u52d5|"
    "\u30e9\u30a4\u30d6\u958b\u59cb\u6642|\u30e9\u30a4\u30d6\u6210\u529f\u6642|\u30e9\u30a4\u30d6\u5931\u6557\u6642"
)


def drop_trigger(s):
    # ターン1回 is a COST marker, not a trigger: it is stored as a flag on the cost
    # (`turn_once`) and never appears in `cost.text`, so it has to come off the clause too.
    t = re.sub(rf"^({TRIGGERS})", "", clean(s or ""))
    return re.sub(r"^/?ターン1回", "", t)


def main():
    rows, page = [], 1
    while page <= 20:
        d = get(f"{LIST}?title={EXPANSION}&page={page}&per_page=100&sort=no")
        chunk = d.get("items") or d.get("data") or []
        if not chunk:
            break
        rows += chunk
        if len(chunk) < 100:
            break
        page += 1
    if not rows:
        print("could not read the card list")
        return 2

    details = {}
    for row in rows:
        det = get(f"{DETAIL}?id={row['id']}")
        c = det.get("card") or {}
        details[str(c.get("card_number") or row.get("card_number"))] = c

    nodes_by_card = {}
    for a in abil["unique_abilities"]:
        t = (a.get("triggerless_text") or "").strip()
        for c in a.get("cards") or []:
            nodes_by_card.setdefault(c.split("|")[0].strip(), []).append(a)

    missing, text_bad, node_bad, clause_bad = [], [], [], []
    parallel, with_text = [], 0

    for no, c in sorted(details.items()):
        if str(c.get("parallel_param") or "").strip() not in ("", "-"):
            parallel.append(no)
        if no not in cards:
            missing.append(no)
            continue
        off_raw = str(c.get("text") or "").strip()
        off = "" if off_raw in NO_ABILITY else off_raw
        loc_raw = str(cards[no].get("ability") or "").strip()
        loc = "" if loc_raw in NO_ABILITY else loc_raw
        if norm(off) != norm(loc):
            text_bad.append((no, norm(off), norm(loc)))
        if not off:
            continue
        with_text += 1
        nodes = nodes_by_card.get(no)
        if not nodes:
            node_bad.append((no, off[:70]))
            continue
        # The effect's own `text` carries the clause; `triggerless_text` carries the
        # gate and shares no prefix with it.
        have = [norm(t) for a in nodes for t in
                (a.get("triggerless_text"), (a.get("effect") or {}).get("text"),
                 (a.get("cost") or {}).get("text")) if t]
        # Split on 。/； AND on newlines: a card with two trigger sections
        # (【ライブ開始時】…。
        # 【ライブ成功時】…) separates them with a newline, so splitting on 。 alone
        # leaves the second trigger mid-clause and drop_trigger, which is anchored at
        # the start, cannot remove it.
        for clause in re.split("[。；\\n]", off):
            if len(clause.strip()) < 8:
                continue
            probe = norm(drop_trigger(clause))
            if len(probe) < 6:
                continue
            if not any(probe[:12] in h for h in have):
                clause_bad.append((no, probe[:50]))

    print(f"expansion            : {EXPANSION}")
    print(f"official cards       : {len(details)}")
    print(f"  held locally       : {len(details) - len(missing)}")
    print(f"  parallel printings : {len(parallel)} "
          f"(all held: {all(p in cards for p in parallel)})")
    print(f"cards with ability   : {with_text}")
    print()
    print(f"1. official cards missing locally : {len(missing)}")
    for m in missing[:20]:
        print("     ", m)
    print(f"2. ability-text differences       : {len(text_bad)}")
    for no, o, l in text_bad[:20]:
        print(f"     {no}\n        official: {o[:110]}\n        local   : {l[:110]}")
    print(f"3. ability text but no node       : {len(node_bad)}")
    for x in node_bad[:20]:
        print("     ", x)
    print(f"4. clauses not represented       : {len(clause_bad)}")
    for x in clause_bad[:25]:
        print("     ", x)

    bad = len(missing) + len(node_bad) + len(clause_bad)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
