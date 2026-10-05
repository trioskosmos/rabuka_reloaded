#!/usr/bin/env python3
"""Compare two corpus_measure.py censuses suite by suite.

Used to answer one question only: DID MY CHANGE REGRESS ANYTHING.  A suite that
changed state, or whose counted result lines moved, is listed; everything else
is byte-comparable on the fields that matter.

`^FAIL` lines are NOT a defect count (a failing test emits one line per failing
assertion PLUS a `FAILED <name>` line), so the headline is the VERDICT per suite
and the assertion/detail split, never a raw line delta presented as defects.
"""
import json
import sys

A, B = sys.argv[1], sys.argv[2]
a = {r["stem"]: r for r in json.load(open(A + "/census.json", encoding="utf-8"))}
b = {r["stem"]: r for r in json.load(open(B + "/census.json", encoding="utf-8"))}


def verdict(r):
    if r.get("link_rc"):
        return "LINK-FAIL"
    v = r.get("verdict", "")
    return "PASSED" if v.startswith("PASSED") else ("FAILED" if v else "NO-VERDICT")


assert set(a) == set(b), (set(a) ^ set(b))
changed = []
for s in sorted(a):
    x, y = a[s], b[s]
    if verdict(x) != verdict(y) or x["fail_asserts"] != y["fail_asserts"] \
            or x["fail_cases"] != y["fail_cases"]:
        changed.append((s, verdict(x), verdict(y), x["fail_asserts"], y["fail_asserts"]))

print("suites: %d   changed: %d   unchanged: %d" % (len(a), len(changed), len(a) - len(changed)))
print()
print("%-36s %-10s %-10s %-9s %-9s" % ("suite", "before", "after", "asserts", "asserts"))
for s, va, vb, fa, fb in changed:
    print("%-36s %-10s %-10s %-9s %-9s" % (s, va, vb, fa, fb))

ta = sum(r["fail_asserts"] for r in a.values())
tb = sum(r["fail_asserts"] for r in b.values())
ca = sum(r["fail_cases"] for r in a.values())
cb = sum(r["fail_cases"] for r in b.values())
print()
print("corpus failing assertions: %d -> %d  (%+d)" % (ta, tb, tb - ta))
print("corpus FAILED case lines: %d -> %d  (%+d)" % (ca, cb, cb - ca))
for lbl, d in (("before", a), ("after", b)):
    from collections import Counter
    c = Counter(verdict(r) for r in d.values())
    print("%-7s verdicts: %s" % (lbl, dict(c)))