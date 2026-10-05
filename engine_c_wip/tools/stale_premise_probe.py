#!/usr/bin/env python3
"""Run test suites via `make t-nogate T=<stem>`, capture stdout+stderr as
BYTES (never PowerShell `*>`: that writes UTF-16LE, and decoding it as UTF-8
zeroes every count), decode by BOM, and extract the summary.

THE SUMMARY SHAPES. The brief named six; the corpus in fact carries well over
that many, and a scanner keyed to one reads 0 on suites holding real failures.
Every pattern below is anchored on a real line observed in this tree, and the
harness's own verdict line
    FAILED: <stem>  [exit=N, R result line(s): assert=A ... renormalised=REN]
is kept as an independent cross-check rather than as the sole key.

Every match is (cases_total, cases_failed). `cases_total` is asserted non-zero
before a result is trusted, because a build that fails to link prints
`Error 1` and ZERO failures and would otherwise read as a clean pass.
"""

import json
import re
import subprocess
import sys

# (name, regex, (cases_idx, failures_idx), count_kind)
# `count_kind` matters: several suites summarise in ASSERTIONS or CHECKS, not
# cases. Reporting an assertion count as a case count is how a suite holding 29
# real failures gets printed as "23 cases".  Only `cases` kinds are compared
# against the non-zero floor.
PATTERNS = [
    ("boots",        r"(\d+)\s+ok,\s*(\d+)\s+FAILED,\s*(\d+)\s+CRASHED",          (0, 1), "cases"),
    ("eq_cases",     r"====[^\n=]*?(\d+)\s+cases passed,\s*(\d+)\s+cases failed",  (0, 1), "cases_sum"),
    # NOTE: `tests passed : N` is the PASSED count. The case total is N + the
    # failed count, which is what `cases_sum` computes. Keying this on the first
    # group alone understates the total by the entire failure count.
    ("block_parity", r"tests passed\s*:\s*(\d+)[\s\S]{0,160}?tests failed[^:]*:\s*(\d+)", (0, 1), "cases_sum"),
    ("cases_ok",     r"(\d+)\s+(?:test\s+)?cases:\s*(\d+)\s+ok,\s*(\d+)\s+failed",  (0, 2), "cases_sum"),
    ("minitest",     r"(\d+)\s+cases,\s*(\d+)\s+assertions?,\s*(\d+)\s+failures?", (0, 2), "cases"),
    ("checks_failed", r"(\d+)\s+checks failed",                                    (None, 0), "assertions"),
    ("checks_fail",  r"(\d+)\s+checks?,\s*(\d+)\s+failures?",                      (0, 1), "assertions"),
    ("assert_fail",  r"(\d+)\s+assertions?,\s*(\d+)\s+failures?",                  (0, 1), "assertions"),
    ("named_ok",     r"[=]{4}\s*[a-z_]+:\s*(\d+)\s+ok,\s*(\d+)\s+failed",         (0, 1), "cases_sum"),
    ("tests_ok",     r"(\d+)\s+tests ok,\s*(\d+)\s+failed",                       (0, 1), "cases_sum"),
    ("bare_ok",      r"[=:]{1,2}\s*(\d+)\s+ok,\s*(\d+)\s+failed",                  (0, 1), "cases_sum"),
    ("slash_ok",     r"ok\s+(\d+)\s*/\s*failed\s+(\d+)",                          (0, 1), "cases_sum"),
    ("passed_only",  r"(\d+)\s+cases passed,\s*(\d+)\s+cases failed",            (0, 1), "cases_sum"),
    # `mechanics` emits TAP-ish `ok:` lines and a prose all-clear, no numbers.
    ("ok_lines",     r"(?s)ALL MECHANIC CHECKS PASSED",                          ("OKLINES", None), "cases"),
]

VERDICT = re.compile(
    r"^(PASSED|FAILED):\s+(\S+)\s+\[exit=(-?\d+),"
    r"\s*(\d+) result line\(s\)(?::\s*(.*?))?\]", re.M)


def decode(raw: bytes) -> str:
    if raw.startswith(b"\xef\xbb\xbf"):
        return raw[3:].decode("utf-8", "replace")
    if raw.startswith((b"\xff\xfe", b"\xfe\xff")):
        return raw[2:].decode("utf-16", "replace")
    return raw.decode("utf-8", "replace")


def parse(text: str, stem: str) -> dict:
    out = {"stem": stem, "shape": None, "cases": None, "failures": None,
           "count_kind": None, "fail_lines": None, "verdict": None,
           "trustworthy": False}

    out["fail_lines"] = len(re.findall(r"^FAIL\b", text, re.M))

    v = VERDICT.search(text)
    if v:
        out["verdict"] = v.group(1)
        out["verdict_exit"] = int(v.group(3))
        det = v.group(5) or ""
        out["verdict_assert"] = det.split("assert=")[1].split()[0] if "assert=" in det else None

    for name, rx, idx, kind in PATTERNS:
        m = re.search(rx, text)
        if not m:
            continue
        if idx[0] == "OKLINES":
            out.update(shape=name, count_kind="cases",
                       cases=len(re.findall(r"^ok:", text, re.M)),
                       failures=len(re.findall(r"^(?:FAIL|not ok)\b", text, re.M)))
            break
        g = [int(x) for x in m.groups() if x is not None]
        fails = g[idx[1]] if idx[1] is not None else 0
        if kind == "cases_sum":
            cases = g[idx[0]] + fails
        else:
            cases = g[idx[0]] if idx[0] is not None else None
        out.update(shape=name, count_kind=kind, cases=cases, failures=fails,
                   raw_groups=g)
        break

    # A build/link failure prints `Error 1` (or 127) and produces NO summary.
    build_broke = out["shape"] is None and "Error" in text
    if build_broke:
        out["unattributed"] = "BUILD/LINK FAILURE -- no summary; counts are zero, not a pass"
    elif out["cases"] == 0:
        out["unattributed"] = "ZERO CASES -- result not trustworthy"
    elif out["count_kind"] == "assertions":
        # Honest, not untrustworthy: the suite summarised in assertions. The
        # floor check cannot apply, and the number must never be called cases.
        out["note"] = ("summarised in ASSERTIONS/CHECKS, not cases -- "
                       "the printed total is not a case count")
    else:
        out["trustworthy"] = True
    return out


def run(stem: str, cwd: str = ".") -> dict:
    p = subprocess.run(["make", "t-nogate", "T=" + stem], cwd=cwd,
                       capture_output=True, timeout=2400)
    r = parse(decode(p.stdout) + "\n" + decode(p.stderr), stem)
    r["rc"] = p.returncode
    with open(f"tests/_stale_{stem}.txt", "wb") as fh:
        fh.write(p.stdout + b"\n" + p.stderr)
    return r


if __name__ == "__main__":
    rows = [run(s) for s in sys.argv[1:]]
    for r in rows:
        print(json.dumps(r, ensure_ascii=False))
    ok = sum(1 for r in rows if r["trustworthy"])
    print(f"# trustworthy {ok}/{len(rows)} suites; "
          f"total cases-failed = {sum(r['failures'] or 0 for r in rows)}")