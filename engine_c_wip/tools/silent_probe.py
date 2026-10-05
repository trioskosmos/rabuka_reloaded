#!/usr/bin/env python3
"""tools/silent_probe.py -- did a 'PASSED' suite actually RUN anything?

The verdict runner's PASSED requires exit 0 AND zero result lines.  That is
stronger than "no FAIL lines", but it is NOT the same as "the suite executed
assertions": a suite whose main() returns without calling a single check
satisfies it exactly.  This counts the per-case result lines each suite
actually emits, which is the difference between a measurement and a shrug.
"""
import json
import os
import re
import sys

OUT = sys.argv[1]
rows = json.load(open(os.path.join(OUT, "census.json"), encoding="utf-8"))

OKCASE = re.compile(r"^ok\b", re.M)
STRUCT = re.compile(r"assertion\(s\), *(\d+) failure")
CHECKS = re.compile(r"^(\d+) checks?, *(\d+) failed", re.M)
ALLP = re.compile(r"^ALL\b.*PASSED", re.M)
PERCASE_F = re.compile(r"^FAILED\b", re.M)

print("%-34s %-9s %-6s %-6s %-6s %-6s %-6s" %
      ("suite", "verdict", "ok:", "strct", "chk", "allp", "fcase"))
silent = []
for r in sorted(rows, key=lambda r: r["stem"]):
    if r["link_rc"]:
        continue
    log = open(os.path.join(OUT, r["stem"] + ".log"), encoding="utf-8",
               errors="replace").read()
    if not r["verdict"].startswith("PASSED"):
        continue
    n_ok = len(OKCASE.findall(log))
    m = STRUCT.search(log)
    n_st = m.group(1) if m else "-"
    m2 = CHECKS.search(log)
    n_ck = m2.group(1) if m2 else "-"
    n_ap = len(ALLP.findall(log))
    n_fc = len(PERCASE_F.findall(log))
    print("%-34s %-9s %-6d %-6s %-6s %-6d %-6d" %
          (r["stem"], "PASSED", n_ok, n_st, n_ck, n_ap, n_fc))
    if n_ok == 0 and n_st == "-" and n_ck == "-" and n_ap == 0:
        silent.append(r["stem"])

print()
print("=== PASSED but reported NOTHING per-case (%d) ===" % len(silent))
for s in silent:
    print("  " + s)