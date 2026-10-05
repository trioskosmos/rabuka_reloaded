#!/usr/bin/env python3
"""Mutation test for the pay_cost_all_discard movement-event fix.

Temporarily removes the per-card rb_record_card_movement loop from
rb_effect_pay_cost_all_discard, rebuilds ONLY that object (make will not
rebuild a .o whose mtime looks newer than its .c, so the .o is deleted), runs
the new suite, and asserts it goes RED on the movement-event assertions.
Then restores the file byte-for-byte and hash-verifies.

Usage: python tools/mutate_movement_events.py
"""
import hashlib
import io
import os
import subprocess
import sys

SRC = "src/ability/effects/misc.c"
OBJ = "src/ability/effects/misc.o"

MUTATION = """    for (int i = 0; i < n_moved; i++)
        rb_record_card_movement(g, ids[i], src_zone, RB_ZONEID_DISCARD, who, 0);
"""


def sha(path):
    return hashlib.sha256(io.open(path, "rb").read()).hexdigest()


def run(args):
    return subprocess.run(args, cwd=".", stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT).stdout.decode("utf-8", "replace")


def main():
    orig = io.open(SRC, encoding="utf-8", newline="").read()
    h0 = sha(SRC)
    if orig.count(MUTATION) != 1:
        print("REFUSING: mutation target occurs %d times" % orig.count(MUTATION))
        return 2
    mutated = orig.replace(MUTATION, "    (void)n_moved; (void)src_zone; (void)ids;\n")
    try:
        io.open(SRC, "w", encoding="utf-8", newline="").write(mutated)
        print("mutated %s -> %s" % (SRC, sha(SRC)))
        if os.path.exists(OBJ):
            os.remove(OBJ)
        out = run(["make", "t-nogate", "T=parity_conc_cost_card"])
        red = "FAIL: " in out and "0 failures" not in out.split("assertions,")[-1][:40]
        fails = [l for l in out.replace("\r\n", "\n").split("\n") if l.startswith("FAIL:")]
        print("MUTATION %s: %d FAIL lines" %
              ("DETECTED" if fails else "NOT DETECTED", len(fails)))
        for l in fails:
            print("   " + l)
        return 0 if fails else 1
    finally:
        io.open(SRC, "w", encoding="utf-8", newline="").write(orig)
        h1 = sha(SRC)
        print("restored %s -> %s (identical to pre-mutation: %s)"
              % (SRC, h1, h1 == h0))
        if os.path.exists(OBJ):
            os.remove(OBJ)


if __name__ == "__main__":
    sys.exit(main())
