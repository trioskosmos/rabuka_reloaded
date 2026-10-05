#!/usr/bin/env python3
"""Mutation test for the REMOVED `rest`/`rested` arm in pay_cost_change_state.

The guard alone is not observable (with it or without it, a non-"wait" cost
changes nothing once the rest branch is gone), so the mutation that matters is
re-inserting the Rust-absent branch. That is what this does.

Deletes cost.o (make will not rebuild a .o whose mtime looks newer than its
.c), runs the new suite and asserts it goes RED. Restores and hash-verifies.
"""
import hashlib
import io
import os
import subprocess
import sys
import time

SRC = "src/ability/cost.c"
OBJ = "src/ability/cost.o"

GUARD = '    if (strcmp(state_change, "wait") != 0) return 1;'

LOOP = """        for (int i = 0; i < n; i++) {
            int cid = pos[i];
            rb_mods_set_orientation(&g->mods, cid, "wait");
            g->last_cost_waited_members[g->n_last_cost_waited_members++] = (int16_t)cid;
        }
"""

RESTORED = """        for (int i = 0; i < n; i++) {
            int cid = pos[i];
            if (!strcmp(state_change, "wait")) {
                rb_mods_set_orientation(&g->mods, cid, "wait");
                g->last_cost_waited_members[g->n_last_cost_waited_members++] = (int16_t)cid;
            } else if (!strcmp(state_change, "rest") || !strcmp(state_change, "rested")) {
                rb_mods_set_orientation(&g->mods, cid, "rest");
                g->last_cost_waited_members[g->n_last_cost_waited_members++] = (int16_t)cid;
            }
        }
"""


def sha(path):
    return hashlib.sha256(io.open(path, "rb").read()).hexdigest()


def write(path, text):
    for _ in range(60):
        try:
            io.open(path, "w", encoding="utf-8", newline="").write(text)
            return True
        except PermissionError:
            time.sleep(1)
    return False


def main():
    orig = io.open(SRC, encoding="utf-8", newline="").read()
    h0 = sha(SRC)
    if orig.count(LOOP) != 1 or orig.count(GUARD) != 1:
        print("REFUSING: loop=%d guard=%d"
              % (orig.count(LOOP), orig.count(GUARD)))
        return 2
    try:
        # Revert BOTH halves: the pre-fix state had the unconditional scan AND
        # the rest/rested branch. Re-inserting only the branch is inert, because
        # the guard short-circuits before the loop.
        mutated = orig.replace(GUARD, "    /* guard removed by mutation test */")
        mutated = mutated.replace(LOOP, RESTORED)
        if not write(SRC, mutated):
            print("could not write the mutated source")
            return 2
        print("mutated %s -> %s" % (SRC, sha(SRC)))
        if os.path.exists(OBJ):
            os.remove(OBJ)
        out = subprocess.run(["make", "t-nogate", "T=parity_conc_cost_card"],
                             stdout=subprocess.PIPE,
                             stderr=subprocess.STDOUT).stdout.decode("utf-8", "replace")
        fails = [l for l in out.replace("\r\n", "\n").split("\n") if l.startswith("FAIL:")]
        print("MUTATION %s: %d FAIL lines"
              % ("DETECTED" if fails else "NOT DETECTED", len(fails)))
        for l in fails:
            print("   " + l)
        return 0 if fails else 1
    finally:
        ok = write(SRC, orig)
        time.sleep(1)
        print("restored %s -> %s (identical: %s)" % (SRC, sha(SRC), sha(SRC) == h0))
        if os.path.exists(OBJ):
            os.remove(OBJ)
        if not ok:
            return 3


if __name__ == "__main__":
    sys.exit(main())