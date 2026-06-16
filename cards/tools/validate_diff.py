#!/usr/bin/env python3
"""Show entries sorted by fewest diffs (ascending). Fix #1, re-run."""

import json, sys, codecs
from pathlib import Path

sys.stdout = codecs.getwriter("utf-8")(sys.stdout.buffer)
sys.path.insert(0, str(Path(__file__).parent.parent / "ability_extraction"))

working = json.loads(
    Path(
        r"C:\Users\trios\Downloads\rabuka_reloaded-master (2)\rabuka_reloaded-master\cards\abilities.json"
    ).read_text(encoding="utf-8")
)
generated = json.loads(Path("cards/abilities.json").read_text(encoding="utf-8"))

wa = {u["full_text"]: u for u in working["unique_abilities"]}
ga = {u["full_text"]: u for u in generated["unique_abilities"]}
M = "MISSING_FIELD"
IGNORE_FIELDS = {"text", "full_text", "triggerless_text", "generated_at", "schema"}


def find_all_diffs(w, g, path="", report=None):
    if report is None:
        report = []
    if w == g:
        return report
    if type(w) != type(g):
        report.append((path, str(type(w).__name__), str(type(g).__name__)))
        return report
    if isinstance(w, dict):
        for k in sorted(set(list(w.keys()) + list(g.keys()))):
            nk = f"{path}.{k}" if path else k
            if k in IGNORE_FIELDS or nk.rstrip("]0123456789").endswith(".text"):
                continue
            if k not in w:
                report.append((nk, M, "MISSING"))
            elif k not in g:
                report.append((nk, repr(w[k])[:60], M))
            elif w[k] != g[k]:
                if isinstance(w[k], (dict, list)):
                    find_all_diffs(w[k], g[k], nk, report)
                else:
                    report.append((nk, repr(w[k])[:60], repr(g[k])[:60]))
    elif isinstance(w, list):
        for i in range(max(len(w), len(g))):
            nk = f"{path}[{i}]"
            if i >= len(w):
                report.append((nk, M, repr(g[i])[:60]))
            elif i >= len(g):
                report.append((nk, repr(w[i])[:60], M))
            else:
                find_all_diffs(w[i], g[i], nk, report)
    return report


# Collect all entries with diffs, sorted by diff count DESC
entries = []
for ft, we in wa.items():
    ge = ga.get(ft)
    if not ge:
        continue
    diffs = find_all_diffs(we.get("effect", {}), ge.get("effect", {}), "effect")
    diffs += find_all_diffs(we.get("cost", {}), ge.get("cost", {}), "cost")
    if not diffs:
        continue
    entries.append((len(diffs), ft, diffs))

entries.sort(key=lambda x: x[0])

total = len(entries)
print(f"\033[1mEntries with diffs: {total}\033[0m\n")

# Show bottom 5 (easiest) entries
for rank in range(min(5, total)):
    cnt, ft, diffs = entries[rank]
    print(
        f"\033[1m#{(rank + 1):2d}: {cnt} diff{'s' if cnt > 1 else ' '}  {ft[:80]}...\033[0m"
    )
    for path, t_val, g_val in diffs[:20]:
        print(f"     {path}")
        print(f"       T: {t_val[:70]}")
        print(f"       G: {g_val[:70]}")
    if len(diffs) > 20:
        print(f"     ... ({len(diffs) - 20} more diffs)")
    print()

print(f"\n\033[1mTop entry has {entries[0][0]} diffs\033[0m")

# Show FULL effects for the top entry
ft0 = entries[0][1]
we0 = wa[ft0]
ge0 = ga[ft0]
print(f"\n\033[1m=== TARGET effect:\033[0m")
print(json.dumps(we0.get("effect"), indent=2, ensure_ascii=False)[:800])
print(f"\033[1m=== GENERATED effect:\033[0m")
print(json.dumps(ge0.get("effect"), indent=2, ensure_ascii=False)[:800])
print(f"\033[1m=== COST target:\033[0m")
print(json.dumps(we0.get("cost"), indent=2, ensure_ascii=False)[:400])
print(f"\033[1m=== COST generated:\033[0m")
print(json.dumps(ge0.get("cost"), indent=2, ensure_ascii=False)[:400])

print(f"\nFix the #1 entry, re-run, and the next will become #1.")
