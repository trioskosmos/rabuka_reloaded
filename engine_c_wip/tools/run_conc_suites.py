#!/usr/bin/env python3
"""Run the concentration suites and report per-suite ^FAIL: counts AND case
counts. Counts ^FAIL: case-sensitively with a CRLF-tolerant anchor, on stdout
AND stderr together. EXIT CODE IS NOT TREATED AS A SIGNAL.

Usage: python tools/run_conc_suites.py <tag>
Writes engine_c_wip/conc_<tag>.txt (per-suite raw text) and
engine_c_wip/conc_<tag>.json (the parsed table).
"""
import io
import json
import os
import re
import subprocess
import sys

SUITES = [
    "parity_cost_and_effect", "parity_cost_compound", "parity_cost_discount",
    "parity_fw_cost_cardtype", "parity_card_score", "parity_card_filters",
    "parity_card_text", "card_data_decode", "decode_skew", "decode_fallback",
    "test_parity_dead_twins", "parity_group_names", "parity_zone_source",
    "parity_sequential", "parity_integration", "parity_small_clusters",
    "parity_cce_gaps", "parity_required_hearts", "parity_under_member",
    "parity_zones_position", "parity_live_total", "parity_recover_live",
    "parity_wave1a", "parity_wave1b", "parity_wave1c", "parity_full_round",
    "parity_full_round2", "parity_full_round3", "mechanics", "audit_fields",
]

# Every suite prints its own case total in its own words. All of these are
# counted; a suite whose total cannot be found is reported as UNKNOWN, never
# as zero.
TOTAL_RES = [
    re.compile(r"^(\d+) checks, (\d+) failures", re.M),
    re.compile(r"==== [^\n]*?: (\d+) tests ok, (\d+) crashed"),
    re.compile(r"==== [^\n]*?: (\d+) cases passed, (\d+) cases failed"),
    re.compile(r"decode-skew audit: (\d+) checks, (\d+) failed"),
]
TESTS_RUN_RE = re.compile(r"^tests run\s+: (\d+)", re.M)
TESTS_FAILED_RE = re.compile(r"^tests failed \(parity\)\s+: (\d+)", re.M)
FAIL_RE = re.compile(r"^FAIL:", re.MULTILINE)
RESULT_RE = re.compile(r"\[exit=(-?\d+), (\d+) result line\(s\)\]")


def total_of(nl):
    for rx in TOTAL_RES:
        m = rx.search(nl)
        if m:
            return int(m.group(1)), int(m.group(2))
    m = TESTS_RUN_RE.search(nl)
    if m:
        f = TESTS_FAILED_RE.search(nl)
        return int(m.group(1)), int(f.group(1)) if f else -1
    return -1, -1


def main():
    tag = sys.argv[1]
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    rows = []
    for s in SUITES:
        proc = subprocess.run(
            ["make", "t-nogate", "T=%s" % s],
            cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        )
        text = proc.stdout.decode("utf-8", "replace")
        nl = text.replace("\r\n", "\n").replace("\r", "\n")
        with io.open(os.path.join(root, "conc_%s_%s.log" % (tag, s)), "w",
                     encoding="utf-8", newline="") as fh:
            fh.write(text)
        fails = len(FAIL_RE.findall(nl))
        raw_fails = text.count("\nFAIL:")
        checks, reported = total_of(nl)
        r = RESULT_RE.search(nl)
        result_lines = int(r.group(2)) if r else -1
        missing = ("NO SUCH SUITE" in nl) or ("does not exist" in nl)
        rows.append(dict(suite=s, checks=checks, fails=fails,
                         raw_fails=raw_fails, reported=reported,
                         result_lines=result_lines, rc=proc.returncode,
                         missing=missing))
    with io.open(os.path.join(root, "conc_%s.json" % tag), "w",
                 encoding="utf-8") as fh:
        json.dump(rows, fh, indent=1)
    print("%-30s %8s %8s %8s %8s" % ("suite", "cases", "FAIL:", "reported", "rc"))
    for r in rows:
        tag_ = "MISSING" if r["missing"] else ""
        print("%-30s %8s %8s %8s %8s %s" % (r["suite"], r["checks"], r["fails"],
                                            r["reported"], r["rc"], tag_))
    print("\n-- integrity checks --")
    for r in rows:
        s = r["suite"]
        if r["missing"]:
            print("  %s: DOES NOT RESOLVE (not a pass)" % s)
        elif r["checks"] == -1:
            print("  %s: NO CASE TOTAL FOUND -- cannot be counted as a pass" % s)
        elif r["fails"] != r["reported"]:
            print("  %s: FAIL:=%d but the suite reported %d"
                  % (s, r["fails"], r["reported"]))
        elif r["raw_fails"] != r["fails"]:
            print("  %s: raw %d vs CRLF-normalised %d"
                  % (s, r["raw_fails"], r["fails"]))
    print("  (nothing else printed above = every suite agreed with itself)")


if __name__ == "__main__":
    main()