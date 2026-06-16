#!/usr/bin/env python3
"""Compare working vs generated abilities.json and group differences by pattern."""

import json, subprocess, sys, re
from collections import defaultdict

sys.path.insert(
    0, str(__import__("pathlib").Path(__file__).parent.parent / "ability_extraction")
)

r = subprocess.run(["git", "show", "HEAD:cards/abilities.json"], capture_output=True)
working = json.loads(r.stdout)
with open("cards/abilities.json", encoding="utf-8") as f:
    generated = json.load(f)

wa = {u["full_text"]: u for u in working["unique_abilities"]}
ga = {u["full_text"]: u for u in generated["unique_abilities"]}

patterns = defaultdict(list)
FIELD_IGNORE = {"text", "generated_at", "schema"}

for ft, we in wa.items():
    ge = ga.get(ft)
    if ge is None:
        continue

    wf = we.get("effect") or {}
    gf = ge.get("effect") or {}
    wc = we.get("cost") or {}
    gc = ge.get("cost") or {}

    if wf == gf and wc == gc:
        continue

    # Determine the pattern of difference
    diffs = []
    for src_name, w_src, g_src in [("effect", wf, gf), ("cost", wc, gc)]:
        for k in set(list(w_src.keys()) + list(g_src.keys())):
            if k in FIELD_IGNORE:
                continue
            wv = w_src.get(k)
            gv = g_src.get(k)
            if wv != gv:
                diffs.append(f"{src_name}.{k}: {repr(wv)[:40]} vs {repr(gv)[:40]}")

    if not diffs:
        continue

    tt = we.get("triggerless_text", "")
    key_diffs = [d for d in diffs if "action" in d.split(".")[1][:10]]
    primary = diffs[0] if not key_diffs else key_diffs[0]
    patterns[primary].append((ft[:50], diffs))

print(f"Entries with differences: {len(patterns)}\n")

for pattern, entries in sorted(patterns.items(), key=lambda x: -len(x[1])):
    print(f"[{len(entries)}] {pattern}")
    for ft, diffs in entries[:3]:
        print(f"    {ft}")
        for d in diffs[:3]:
            print(f"      {d}")
    print()
