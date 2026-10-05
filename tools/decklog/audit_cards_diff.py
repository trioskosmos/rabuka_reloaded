"""Audit what the expansion scraper changed in cards.json.

The scraper rewrote 215 pre-existing cards, which was not reviewed at the time. This
classifies every change and calls out anything that looks like data loss rather than a
refresh from the official source.
"""

import json
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
OLD = json.loads((REPO / "cards/cards.json.bak-before-pr").read_text(encoding="utf-8"))
NEW = json.loads((REPO / "cards/cards.json").read_text(encoding="utf-8"))

added = [k for k in NEW if k not in OLD]
changed = [k for k in NEW if k in OLD and
           json.dumps(NEW[k], ensure_ascii=False, sort_keys=True) !=
           json.dumps(OLD[k], ensure_ascii=False, sort_keys=True)]

print(f"before {len(OLD)}  after {len(NEW)}  added {len(added)}  changed {len(changed)}\n")

# ---- data-loss detection -------------------------------------------------
lost = []
for k in changed:
    o, n = OLD[k], NEW[k]
    for f in set(o) | set(n):
        ov, nv = o.get(f), n.get(f)
        if f not in o:
            continue
        had = ov not in (None, "", [], {}, 0)
        now = nv not in (None, "", [], {}, 0)
        if had and not now:
            lost.append((k, f, ov, nv))

print(f"=== fields that LOST data: {len(lost)} ===")
by_field = Counter(f for _, f, _, _ in lost)
for f, c in by_field.most_common():
    print(f"  {f:16} {c}")
for k, f, ov, nv in lost[:12]:
    print(f"    {k:24} {f}: {json.dumps(ov, ensure_ascii=False)[:90]} -> {json.dumps(nv, ensure_ascii=False)[:40]}")

# ---- ability text changes ------------------------------------------------
print(f"\n=== ability text changed on pre-existing cards ===")
ab = [(k, OLD[k].get("ability") or "", NEW[k].get("ability") or "") for k in changed
      if (OLD[k].get("ability") or "") != (NEW[k].get("ability") or "")]
print(f"count: {len(ab)}\n")
for k, o, n in ab:
    print(f"--- {k}  ({OLD[k].get('name','')})")
    print(f"  OLD: {o[:300]!r}")
    print(f"  NEW: {n[:300]!r}")
    print()

# ---- benign refills ------------------------------------------------------
print("=== other changes: sample of what actually moved ===")
sample_fields = Counter()
for k in changed:
    o, n = OLD[k], NEW[k]
    for f in set(o) | set(n):
        if json.dumps(o.get(f), ensure_ascii=False, sort_keys=True) != \
           json.dumps(n.get(f), ensure_ascii=False, sort_keys=True):
            sample_fields[f] += 1
for f, c in sample_fields.most_common():
    print(f"  {f:16} {c}")

k = changed[0]
print(f"\n=== full before/after for one representative card: {k} ===")
print("OLD:", json.dumps(OLD[k], ensure_ascii=False, indent=1)[:900])
print("NEW:", json.dumps(NEW[k], ensure_ascii=False, indent=1)[:900])