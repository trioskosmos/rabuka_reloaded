#!/usr/bin/env python3
"""tools/corpus_measure.py -- whole-corpus measurement in ONE pass.

Links every tests/test_<stem>.c directly against the object library, runs it
through rb_verdict, and records the canonical `^FAIL` census per suite.

STRICTLY SEQUENTIAL, and each binary is deleted after it runs (disk).

Counting rules, fixed and stated so they cannot drift:
  * `^FAIL` with NO colon, case-SENSITIVE, over stdout AND stderr.  All five
    corpus prefixes are caught: `FAIL: `, `FAIL [`, `FAIL (VACUOUS):`,
    `FAILED  name`, `FAILED:`.
  * EXIT CODE IS NOT A SIGNAL and is recorded but never used to decide green.
  * `^FAIL` LINES ARE NOT A DEFECT COUNT: a failing test emits one `FAIL:`
    per failing assertion PLUS a `FAILED <name>` line.  Both counts are kept.

Usage:  python tools/corpus_measure.py <outdir>
"""
import os
import re
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TESTS = os.path.join(ROOT, "tests")
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "tests", "_measure")

OBJ = []
for dirpath, _dirs, files in os.walk(os.path.join(ROOT, "src")):
    for f in files:
        if f.endswith(".o"):
            OBJ.append(os.path.join(dirpath, f))
OBJ.sort()
# CORRECTED 2026-10-05: the comment here used to read "OBJ_LIB literally names
# src/web_server.o ... Dropped, per the Makefile's own comment about suite
# links", citing the Makefile `clean` block's claim that web_server.o is "NOT in
# SRC or OBJ_LIB". That claim was FALSE -- Makefile:61 (SRC) and :64 (OBJ_LIB)
# both name it -- so this file was dropping an object on the strength of a
# misreading. It is still dropped, for the real reason: it is the WEBSERVER TU
# and no suite calls into it, so keeping it would only add link time. NOTE the
# consequence, which is a live difference and not a cosmetic one: `make t-run`
# links $(OBJ_LIB) VERBATIM and therefore DOES include src/web_server.o, so a
# suite built by `make t T=<stem>` and the same suite built here are not linked
# from an identical object set. No duplicate symbol results either way (web_*
# symbols are defined in no other TU), so the two links agree on behaviour; only
# the object list differs. Not changed here, because changing it would move
# every recorded baseline at once.
#
# src/main.o is dropped for a second, harder reason: it defines its own `main`,
# so a suite linked against the full object list fails with `multiple
# definition of main' -- every one of the 166 suites, not a subset.
def _not_suite_link(p):
    base = os.path.basename(p)
    return base not in ("web_server.o", "main.o")

OBJ = [p for p in OBJ if _not_suite_link(p)]

CFLAGS = "-std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated -Wl,--stack,33554432".split()

stems = sorted(
    f[len("test_"):-2] for f in os.listdir(TESTS) if f.startswith("test_") and f.endswith(".c")
)
if os.path.isdir(OUT):
    for f in os.listdir(OUT):
        os.remove(os.path.join(OUT, f))
else:
    os.makedirs(OUT)

rows = []
for n, stem in enumerate(stems, 1):
    # This toolchain is MSYS2/PE: `cc -o name` produces `name.exe`.  Naming the
    # bare path makes rb_verdict's execv fail, the child prints setup-bug, and
    # every suite then measures as "FAILED" -- a whole corpus of false reds.
    exe = os.path.join(ROOT, "rb_engine_t_" + stem + ".exe")
    src = os.path.join(TESTS, "test_" + stem + ".c")
    link = subprocess.run(
        ["cc"] + CFLAGS + ["-o", exe] + OBJ + [src],
        cwd=ROOT, capture_output=True,
    )
    if link.returncode != 0:
        rows.append(dict(stem=stem, link_rc=link.returncode, link_err=link.stderr.decode("utf-8", "replace")[-400:]))
        print("[%3d/%d] %-34s LINK-FAIL" % (n, len(stems), stem), flush=True)
        continue
    t0 = time.time()
    r = subprocess.run(
        [os.path.join(ROOT, "rb_verdict.exe"), exe, stem],
        cwd=ROOT, capture_output=True, timeout=120,
    )
    dt = time.time() - t0
    # parity_required_hearts re-execs/deletes things around itself, so the
    # binary may already be gone; that is a property of the suite, not a
    # measurement failure.
    try:
        os.remove(exe)
    except OSError:
        pass
    blob = (r.stdout.decode("utf-8", "replace") + r.stderr.decode("utf-8", "replace"))
    with open(os.path.join(OUT, stem + ".log"), "w", encoding="utf-8", newline="") as fh:
        fh.write(blob)
    lines = blob.splitlines()
    # ^FAIL with NO colon, case-sensitive, over stdout AND stderr together.
    f_all = [l for l in lines if l.startswith("FAIL")]
    f_detail = [l for l in lines if l.startswith("FAIL:")]
    f_case = [l for l in lines if l.startswith("FAILED")]
    verdicts = [l for l in lines if "[exit=" in l]
    rows.append(dict(
        stem=stem, link_rc=0, sec=round(dt, 2), rc=r.returncode,
        fail_lines=len(f_all), fail_asserts=len(f_detail), fail_cases=len(f_case),
        n_verdict=len(verdicts), verdict=verdicts[-1] if verdicts else "",
        n_lines=len(lines),
    ))
    print("[%3d/%d] %-34s ^F=%-4d verdict=%s" % (
        n, len(stems), stem, len(f_all), "yes" if verdicts else "NONE"), flush=True)

import json
with open(os.path.join(OUT, "census.json"), "w", encoding="utf-8") as fh:
    json.dump(rows, fh, indent=1)
print("wrote", os.path.join(OUT, "census.json"), len(rows), "suites")