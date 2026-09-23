import json
import sys
from collections import defaultdict

from _paths import ABILITIES_JSON, CARDS_JSON

sys.stdout.reconfigure(encoding="utf-8")

cards = json.load(open(CARDS_JSON, encoding="utf-8"))
cards = cards if isinstance(cards, list) else list(cards.values())
multi = [c for c in cards if "\n" in (c.get("series") or "")]

KNOWN = ["μ's", "Aqours", "虹ヶ咲", "Liella!", "蓮ノ空"]


def series_groups(series):
    out = []
    for line in series.split("\n"):
        if "サンシャイン" in line:
            out.append("Aqours")
        elif "虹ヶ咲" in line:
            out.append("虹ヶ咲")
        elif "スーパースター" in line:
            out.append("Liella!")
        elif "蓮ノ空" in line:
            out.append("蓮ノ空")
        elif "ラブライブ！" in line:
            out.append("μ's")
    return list(dict.fromkeys(out))


ab = json.load(open(ABILITIES_JSON, encoding="utf-8"))


def walk(o, acc):
    if isinstance(o, dict):
        acc.append(o)
        for v in o.values():
            walk(v, acc)
    elif isinstance(o, list):
        for v in o:
            walk(v, acc)


nodes = []
walk(ab, nodes)

hits = defaultdict(list)
for n in nodes:
    gns = n.get("group_names")
    if not isinstance(gns, list):
        continue
    known = [g for g in gns if g in KNOWN]
    if not known:
        continue
    text = n.get("text") or n.get("full_text") or n.get("source_text") or ""
    cards_ref = n.get("example_cards") or n.get("cards") or n.get("card_examples") or []
    if isinstance(cards_ref, str):
        cards_ref = [cards_ref]
    kind = n.get("action") or n.get("kind") or n.get("action_type") or ""
    for g in known:
        hits[g].append((text[:200], cards_ref[:4], kind))

out = []
out.append("=== MULTI-LINE JOINT CARDS (Card.group empty; needs JP series line-match) ===")
for c in multi:
    cno = c["card_no"]
    name = c.get("name", "")
    sgs = series_groups(c.get("series") or "")
    has_ab = bool((c.get("ability") or "").strip())
    out.append(f"{cno} | {name}")
    out.append(f"  series lines -> groups: {sgs}")
    out.append(f"  has_printed_ability: {has_ab}")

out.append("")
out.append("=== group_names NODES USING KNOWN_GROUPS (joint cards must match these) ===")
for g in KNOWN:
    out.append(f"--- {g!r}: {len(hits[g])} nodes ---")
    seen = set()
    uniq = []
    for t, cr, kind in hits[g]:
        k = t[:80]
        if k in seen:
            continue
        seen.add(k)
        uniq.append((t, cr, kind))
    for t, cr, kind in uniq[:15]:
        out.append(f"  [{kind}] {t}")
        if cr:
            out.append(f"     cards: {cr}")
    if len(uniq) > 15:
        out.append(f"  ... +{len(uniq) - 15} more unique texts")

out.append("")
out.append("=== ABILITIES PRINTED ON MULTI-LINE JOINT CARDS (with group_names) ===")
joint_nos = sorted({c["card_no"] for c in multi})


def find_gns(o, acc):
    if isinstance(o, dict):
        if o.get("group_names"):
            acc.append(o["group_names"])
        for v in o.values():
            find_gns(v, acc)
    elif isinstance(o, list):
        for v in o:
            find_gns(v, acc)


def find_texts(o, acc):
    if isinstance(o, dict):
        for k in ("full_text", "text", "source_text", "ability"):
            if isinstance(o.get(k), str) and o[k].strip():
                acc.append(o[k])
        for v in o.values():
            find_texts(v, acc)
    elif isinstance(o, list):
        for v in o:
            find_texts(v, acc)


seen_joint = set()
for n in nodes:
    blob = json.dumps(n, ensure_ascii=False)
    matched = [j for j in joint_nos if j in blob]
    if not matched:
        continue
    accg = []
    find_gns(n, accg)
    texts = []
    find_texts(n, texts)
    # prefer unique ability texts mentioning group quotes
    for t in texts:
        if any(x in t for x in ("『", "「")) or accg:
            key = (matched[0], t[:100], str(accg))
            if key in seen_joint:
                continue
            seen_joint.add(key)
            out.append(f"  on {matched}: group_names={accg}")
            out.append(f"    {t[:220]}")

out.append("")
out.append("=== UNIT-only group_names (matched via unit field, NOT series) sample ===")
unit_nodes = 0
for n in nodes:
    gns = n.get("group_names")
    if isinstance(gns, list) and gns and not any(g in KNOWN for g in gns):
        unit_nodes += 1
out.append(f"nodes with group_names but NONE in KNOWN_GROUPS: {unit_nodes}")

path = "joint_group_report.txt"
with open(path, "w", encoding="utf-8") as f:
    f.write("\n".join(out))
print(f"wrote {path} lines={len(out)}")
