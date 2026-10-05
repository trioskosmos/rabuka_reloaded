#!/usr/bin/env python3
"""tools/corpus_classify.py -- classify every suite by what it can REPORT.

The 20 "unverified" suites of BASELINE3 ran to completion, exited 0, and
emitted no verdict line of any form.  That is the state that matters: a suite
that cannot say "I passed" is not a measurement, and calling it green is the
defect.  This classifies all 166 into the four states from the raw log bytes,
plus the "silent" sub-classes that explain WHY.
"""
import json
import os
import re
import sys

OUT = sys.argv[1]
rows = json.load(open(os.path.join(OUT, "census.json"), encoding="utf-8"))

# Per-suite reporting vocabularies observed in the corpus, most-specific first.
VOCAB = [
    ("structured", re.compile(r"assertion\(s\),.*failure\(s\)")),
    ("PASSED-line", re.compile(r"^PASSED\b", re.M)),
    ("ALL-PASSED", re.compile(r"^ALL\b.*PASSED", re.M)),
    ("N-checks", re.compile(r"^\d+ checks?, \d+ failed", re.M)),
    ("FAILED-per-case", re.compile(r"^FAILED\b", re.M)),
    ("ok-per-case", re.compile(r"^ok\b", re.M)),
    ("ok-colon", re.compile(r"^ok:", re.M)),
]

rows.sort(key=lambda r: r["stem"])
out = []
for r in rows:
    if r["link_rc"]:
        out.append((r["stem"], "LINK-FAIL", "c", "c", "does not link", ""))
        continue
    log = open(os.path.join(OUT, r["stem"] + ".log"), encoding="utf-8",
               errors="replace").read()
    vocab = [name for name, rx in VOCAB if rx.search(log)]
    verdict = r["verdict"]
    passed = verdict.startswith("PASSED:")
    nf = r["fail_lines"]
    if not verdict:
        state, why = "NO-VERDICT", "died on a signal; INCONCLUSIVE"
    elif passed:
        # exit 0 AND zero result lines.  The question is whether it RAN anything.
        if vocab == []:
            state = "SILENT"
            why = "PASSED with no per-test line of any kind: ran, reported nothing"
        else:
            state = "GREEN"
            why = "PASSED via %s" % ",".join(vocab)
    else:
        state = "RED"
        why = "FAILED (%d ^FAIL line(s): %d assert, %d case)" % (
            nf, r["fail_asserts"], r["fail_cases"])
    if state == "SILENT" and r["rc"] != 0:
        state, why = "BROKEN", "PASSED-shape but exit %d" % r["rc"]
    out.append((r["stem"], state, str(nf), str(r["rc"]),
                why, ",".join(vocab) or "-"))

from collections import Counter
cnt = Counter(o[1] for o in out)
print("=== states ===")
for k, v in sorted(cnt.items(), key=lambda kv: -kv[1]):
    print("  %-12s %d" % (k, v))
print()
for want in ("SILENT", "BROKEN", "NO-VERDICT", "LINK-FAIL", "GREEN"):
    sel = [o for o in out if o[1] == want]
    if not sel:
        continue
    print("=== %s (%d) ===" % (want, len(sel)))
    for o in sel:
        print("  %s" % ("%-34s ^F=%-5s rc=%-4s %-58s vocab=%s" % (o[0], o[2], o[3], o[4], o[5])))
    print()
json.dump([dict(zip(("stem", "state", "nf", "rc", "why", "vocab"), o)) for o in out],
          open(os.path.join(OUT, "states.json"), "w", encoding="utf-8"), indent=1)