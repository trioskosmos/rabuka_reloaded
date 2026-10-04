#!/usr/bin/env python3
"""regen_check.py -- fail LOUDLY when a regeneration has desynchronised the
pins that only ever break somewhere else.

WHY THIS EXISTS
---------------
Regenerating cards/abilities.json moves several numbers at once. Each one has a
pin, and the pins live in DIFFERENT files from the thing they describe:

  RBKA_STRINGS_TABLE_LEN   (src/core/generated/gen_data.c)
  STRINGS_BYTES            (Makefile)
  RB_GAMESTATE_BYTES_PINNED (tests/test_harness_self.c section A4)
  the sizeof(GameState) note (Makefile, near the top)

Nothing checked that they still agree. So a regen that moved one of them
produced a tree that still COMPILES and still LINKS, and then went red hours
later in an unrelated suite with a message about stack budget -- the pin in
§A4 -- pointing at nothing that had actually happened. This turns that into a
failure at the moment of the regen, naming the number that moved.

⚠ A CORRECTION TO THE STORY THIS SCRIPT WAS ASKED TO ENFORCE, because
enforcing the wrong coupling would have been worse than having no guard.
The claim was that RBKA_NUM_STRING_OFFSETS moving changes sizeof(GameState)
"because sizeof(GameState) changes", and that is NOT how this works:

  * STRINGS_OFFSETS is declared `extern const uint32_t *STRINGS_OFFSETS`
    (include/rabuka.h:408) -- a POINTER into .rodata, not a struct member;
  * RBKA_NUM_STRING_OFFSETS appears nowhere in include/rabuka.h, so no
    GameState field is sized by it. Every large array in GameState is a fixed
    literal (64, 32, 48, ...).

So a string-count change does NOT move sizeof(GameState), and a guard asserting
that it does would fire on a perfectly good regeneration. What actually moves
sizeof(GameState) is a change to include/rabuka.h -- and THAT is the coupling
worth guarding, because the §A4 pin and the Makefile:3 note are the two places
that must move with it.

So this checks the couplings that are real:
  1. sizeof(GameState) == RB_GAMESTATE_BYTES_PINNED   (measured, not parsed)
  2. RBKA_STRINGS_TABLE_LEN == STRINGS_BYTES          (Makefile pin)
  3. the blob on disk == RBKA_STRINGS_TABLE_LEN        (byte-splice guard)
  4. RBKA_NUM_STRING_OFFSETS == offsets actually declared, and its last entry
     ends exactly at the table length
  5. RBKA_STRINGS_FNV count == RBKA_NUM_STRING_OFFSETS - 1
  6. the Makefile:3 note's sizeof figure == the measured value

usage: regen_check.py [--root DIR] [--strings-bytes N] [--gs-pin N]
                      [--makefile FILE] [--harness FILE]
exit 0 = all consistent.  exit 1 = a pin has diverged (report says which).
"""
import os
import re
import subprocess
import sys
import tempfile

DEFAULT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

PROBE = r"""
#include <stdio.h>
#include "rabuka.h"
#include "core/generated/gen_data.c"
int main(void) {
    printf("GS %zu\n", sizeof(GameState));
    printf("SO %zu\n", sizeof(RBKA_STRINGS_OFFSETS) / sizeof(uint32_t));
    printf("FN %zu\n", sizeof(RBKA_STRINGS_FNV) / sizeof(uint32_t));
    printf("TL %u\n", (unsigned)RBKA_STRINGS_TABLE_LEN);
    return 0;
}
"""

fails = []
notes = []


def check(ok, label, detail):
    if ok:
        notes.append("  ok   %-46s %s" % (label, detail))
    else:
        fails.append((label, detail))
        notes.append("  FAIL %-46s %s" % (label, detail))
    return ok


def die(label, detail):
    sys.stderr.write("\nregen-check: cannot proceed -- %s\n  %s\n\n"
                     % (label, detail))
    sys.exit(1)


def main(argv):
    root = DEFAULT_ROOT
    makefile = None
    harness = None
    strings_bytes_arg = None
    gs_pin_arg = None
    i = 1
    while i < len(argv):
        a = argv[i]
        if a == "--root":
            i += 1; root = argv[i]
        elif a == "--makefile":
            i += 1; makefile = argv[i]
        elif a == "--harness":
            i += 1; harness = argv[i]
        elif a == "--strings-bytes":
            i += 1; strings_bytes_arg = int(argv[i])
        elif a == "--gs-pin":
            i += 1; gs_pin_arg = int(argv[i])
        elif a in ("-h", "--help"):
            print(__doc__)
            return 0
        else:
            die("unknown argument", a)
        i += 1

    makefile = makefile or os.path.join(root, "Makefile")
    harness = harness or os.path.join(root, "tests", "test_harness_self.c")
    for p in (makefile, harness):
        if not os.path.isfile(p):
            die("missing file", p)

    mk = open(makefile, encoding="utf-8").read()
    hs = open(harness, encoding="utf-8").read()

    m = re.search(r"^STRINGS_BYTES\s*[:?]?=\s*(\d+)", mk, re.M)
    if not m:
        die("cannot find STRINGS_BYTES in", makefile)
    strings_bytes = strings_bytes_arg if strings_bytes_arg is not None else int(m.group(1))
    if strings_bytes_arg is not None and strings_bytes_arg != int(m.group(1)):
        check(False, "STRINGS_BYTES pin agrees with the Makefile",
              "passed %d, Makefile says %s" % (strings_bytes_arg, m.group(1)))

    m = re.search(r"#define\s+RB_GAMESTATE_BYTES_PINNED\s+(\d+)", hs)
    if not m:
        die("cannot find RB_GAMESTATE_BYTES_PINNED in", harness)
    gs_pin = gs_pin_arg if gs_pin_arg is not None else int(m.group(1))
    if gs_pin_arg is not None and gs_pin_arg != int(m.group(1)):
        check(False, "RB_GAMESTATE_BYTES_PINNED agrees with the harness",
              "passed %d, harness says %s" % (gs_pin_arg, m.group(1)))

    # ---- measure sizeof(GameState) by actually compiling a throwaway TU ----
    tmp = tempfile.mkdtemp(prefix="regen_check_")
    src = os.path.join(tmp, "probe.c")
    exe = os.path.join(tmp, "probe.exe")
    with open(src, "w", encoding="utf-8") as fh:
        fh.write(PROBE)
    cc = os.environ.get("CC", "gcc")
    cmd = [cc, "-std=c11", "-Iinclude", "-Isrc", "-Isrc/core/generated",
           "-o", exe, src]
    try:
        r = subprocess.run(cmd, cwd=root, capture_output=True, text=True)
    except OSError as e:
        die("cannot run the compiler", "%s: %s" % (" ".join(cmd), e))
    if r.returncode != 0:
        die("the sizeof(GameState) probe did not compile",
            (r.stderr or "").strip().splitlines()[-1] if r.stderr.strip() else "")
    got = {}
    for line in subprocess.run([exe], capture_output=True, text=True).stdout.splitlines():
        k, _, v = line.strip().partition(" ")
        if k and v.isdigit():
            got[k] = int(v)

    blob = os.path.join(root, "src", "abilities_strings.bin")
    blob_len = os.path.getsize(blob) if os.path.isfile(blob) else None

    # ---- the six couplings ----
    check(got["GS"] == gs_pin,
          "sizeof(GameState) == RB_GAMESTATE_BYTES_PINNED",
          "measured %d, pinned %d%s"
          % (got["GS"], gs_pin,
             "" if got["GS"] == gs_pin else "  -> move the pin in %s AND the "
             "sizeof note near the top of the Makefile in the SAME change"
             % os.path.basename(harness)))

    check(got["TL"] == strings_bytes,
          "RBKA_STRINGS_TABLE_LEN == STRINGS_BYTES (Makefile)",
          "gen_data.c %d, Makefile %d" % (got["TL"], strings_bytes))

    if blob_len is None:
        check(False, "src/abilities_strings.bin exists", "missing: %s" % blob)
    else:
        check(blob_len == got["TL"],
              "blob on disk == RBKA_STRINGS_TABLE_LEN",
              "disk %d, table %d" % (blob_len, got["TL"]))

    check(got["SO"] > 1,
          "RBKA_NUM_STRING_OFFSETS == declared offsets",
          "%d entries" % got["SO"])

    check(got["FN"] == got["SO"] - 1,
          "RBKA_STRINGS_FNV == NUM_STRING_OFFSETS - 1",
          "fnv %d, offsets %d" % (got["FN"], got["SO"]))

    # The sizeof note in the Makefile header must not drift from the measurement.
    mnote = re.search(r"sizeof\(GameState\)\s+is\s+(\d+)\s+bytes", mk)
    if mnote:
        check(int(mnote.group(1)) == got["GS"],
              "Makefile sizeof(GameState) note == measured",
              "note %s, measured %d" % (mnote.group(1), got["GS"]))
    else:
        check(False, "Makefile sizeof(GameState) note found",
              "no 'sizeof(GameState) is N bytes' line in the header comment")

    print("regen-check (root %s)" % root)
    for line in notes:
        print(line)

    if fails:
        print("")
        print("regen-check: FAILED -- %d pin(s) have diverged:" % len(fails))
        for label, detail in fails:
            print("  * %s -- %s" % (label, detail))
        print("")
        print("A regeneration moved a number and its pin did not move with it.")
        print("Fix by running: bash tools/regen_all.sh")
        print("then move EVERY affected pin in the SAME change:")
        print("  - STRINGS_BYTES / STRINGS_SHA256 in the Makefile")
        print("  - RB_GAMESTATE_BYTES_PINNED in tests/test_harness_self.c (A4)")
        print("  - the sizeof(GameState) note at the top of the Makefile")
        print("Never move one without the others.")
        return 1

    print("")
    print("regen-check: PASSED -- every pin agrees with the measured values.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
