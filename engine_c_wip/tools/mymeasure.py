#!/usr/bin/env python3
"""Measurement helper for the C-behind-Rust port work.

Runs `make t-nogate T=<stem>` with BYTE pipes (never PowerShell redirection,
which writes UTF-16LE and silently zeroes every count), then extracts:
  * the case count  (total, not "passed" -- block_parity reports passed)
  * the failed count, whatever summary shape the suite happens to use
  * the ^FAIL count in stderr
Every shape found is printed with a label so a shape is never guessed.
"""
import os
import re
import subprocess
import sys
import hashlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUTDIR = os.path.join(ROOT, "mm")


def decode(b):
    if b[:2] in (b"\xff\xfe", b"\xfe\xff"):
        return b.decode("utf-16", errors="replace")
    if b[:3] == b"\xef\xbb\xbf":
        return b[3:].decode("utf-8", errors="replace")
    return b.decode("utf-8", errors="replace")


def src_manifest():
    """sha256 of every C source + header, so a mid-run peer move is visible."""
    h = {}
    for base in ("src", "include", "tests"):
        d = os.path.join(ROOT, base)
        for dp, _dn, fn in os.walk(d):
            for f in fn:
                if f.endswith((".c", ".h")):
                    p = os.path.join(dp, f)
                    try:
                        h[os.path.relpath(p, ROOT)] = hashlib.sha256(
                            open(p, "rb").read()).hexdigest()[:12]
                    except OSError:
                        pass
    return h


# SIXTEEN distinct summary shapes exist. Enumerated from the suite sources
# themselves (each printf that emits a total), not guessed. Each entry is
# (label, cases-regex, failures-regex); the verdict line's `renormalised=` is
# the ONLY cross-shape failure key, so it is always reported too.
SHAPES = [
    ("cases_passed_cases_failed", r"cases passed:\s*(\d+) cases passed", None),
    ("A small-cluster", r"^.*: (\d+) cases passed, (\d+) cases failed", None),
    ("F under_member", r"^.*: (\d+) tests ok, (\d+) failed", None),
    ("B keyed_tests_run", r"tests run\s*:\s*(\d+)", r"tests failed(?: \(parity\))?\s*:\s*(\d+)"),
    ("E assertions_total", r"== (\d+) assertions total",
     r"== \d+ assertions total, (\d+) assertion failures"),
    ("C case_run_only", r"^.*: (\d+) test case\(s\) run", None),
    ("D verdict_only", None, None),
]


def shape_of(text):
    hits = []
    for lbl, creg, freg in SHAPES:
        if creg is None:
            continue
        m = re.search(creg, text, re.M)
        if not m:
            continue
        cases = m.groups()
        fail = None
        if len(cases) > 1 and cases[1].isdigit():
            fail = int(cases[1])
        if freg:
            m2 = re.search(freg, text)
            if m2:
                fail = int(m2.group(1))
        hits.append((lbl, int(cases[0]), fail))
    return hits


def run(stem, note=""):
    os.makedirs(OUTDIR, exist_ok=True)
    pre = src_manifest()
    cp = subprocess.run(
        ["make", "-C", ROOT, "t-nogate", "T=%s" % stem],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    out = decode(cp.stdout)
    err = decode(cp.stderr)
    post = src_manifest()
    moved = sorted(k for k in set(pre) | set(post) if pre.get(k) != post.get(k))
    bo = os.path.join(OUTDIR, "%s.out" % stem)
    be = os.path.join(OUTDIR, "%s.err" % stem)
    open(bo, "wb").write(cp.stdout)
    open(be, "wb").write(cp.stderr)
    nfail = len([l for l in err.splitlines() if l.startswith("FAIL:")])
    both = out + "\n" + err
    linkfail = re.search(r"error LNK\d+|undefined (reference|symbol)|"
                         r"cannot open (include )?file|No rule to make", err)
    print("### %s %s" % (stem, note))
    print("  rc=%d  BYTES out=%d err=%d" % (cp.returncode, len(cp.stdout),
                                            len(cp.stderr)))
    if linkfail:
        print("  !! LINK/BUILD FAILURE -- counts meaningless: %s"
              % linkfail.group(0))
    hits = shape_of(both)
    renorm = re.search(r"renormalised=(\d+)", both)
    renorm = int(renorm.group(1)) if renorm else None
    print("  SHAPES SEEN:")
    for h in hits:
        print("     %-26s cases=%-5s fail=%s" % h)
    if not hits and not renorm:
        print("     !! NONE -- BUILD BROKEN, treat as UNKNOWN, never as 0")
    print("  renormalised(=uniform fail key)=%s" % renorm)
    print("  ^FAIL(stderr)=%d   moved_sources=%s" % (nfail, moved or "none"))
    return hits, renorm, nfail, moved


if __name__ == "__main__":
    stems = sys.argv[1:]
    if not stems:
        stems = ["parity_small_clusters"]
    for s in stems:
        run(s)
