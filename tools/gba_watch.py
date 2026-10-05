"""Poll the GBA target's registers until the CPU derails, then report the
last known-good PC plus the stack, so the faulting site can be resolved to a
symbol in the ELF.

mGBA refuses VRAM/OAM reads over GDB (E06) but happily serves IWRAM, EWRAM and
ROM, which is enough to see where execution goes wrong and what the stack held.

Usage: python gba_watch.py <rom> [--seconds N]
"""

import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gba_capture import MGBA, Rsp  # noqa: E402

ROM_LO, ROM_HI = 0x08000000, 0x09FFFFFF


def classify(pc):
    if ROM_LO <= pc < ROM_HI:
        return "rom"
    # The BIOS is mirrored at 0x00000000..0x0000FFFF and agb reaches it via
    # SWI (VBlankIntrWait, IntrWait, LZ77, division). Jumping there is normal,
    # so it must not be reported as a derail -- doing so produced a false
    # positive on every VBlank wait.
    if pc < 0x00010000:
        return "bios"
    if 0x02000000 <= pc < 0x02040000:
        return "ewram"
    if 0x03000000 <= pc < 0x03008000:
        return "iwram"
    if pc == 0:
        return "null"
    return "INVALID"


def main():
    rom = sys.argv[1]
    seconds = 180.0
    args = sys.argv[2:]
    for i, a in enumerate(args):
        if a == "--seconds":
            seconds = float(args[i + 1])

    proc = subprocess.Popen(
        [MGBA, "--gdb", rom],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        creationflags=getprocessattr(),
    )
    try:
        time.sleep(2.5)
        rsp = Rsp()
        rsp.cmd("?")
        rsp.continue_()

        # Confirm we can read the memory regions we care about.
        for name, addr in (("iwram", 0x03000000), ("ewram", 0x02000000), ("rom", 0x08000000)):
            try:
                rsp.read_mem(addr, 16)
                print("readable: %s" % name)
            except Exception as exc:
                print("NOT readable: %s (%s)" % (name, exc))

        hist = []
        t0 = time.time()
        bad = False
        while time.time() - t0 < seconds:
            try:
                rsp.halt()
                r = rsp.regs()
            except Exception:
                break
            pc, sp = r["pc"], r["sp"]
            where = classify(pc)
            hist.append((round(time.time() - t0, 2), pc, sp, where))
            if where in ("INVALID", "null", "ewram"):
                print("DERAIL at t=%.2fs pc=%08x (%s) sp=%08x" % (hist[-1][0], pc, where, sp))
                for t, p, s, w in hist[-12:]:
                    print("   t=%6.2f pc=%08x sp=%08x %s" % (t, p, s, w))
                # Grab the top of stack: this is where the return chain is.
                try:
                    stack = rsp.read_mem(sp, 96)
                    words = [int.from_bytes(stack[i:i + 4], "little")
                             for i in range(0, len(stack) - 3, 4)]
                    print("stack words: " + " ".join("%08x" % w for w in words))
                except Exception as exc:
                    print("stack read failed:", exc)
                bad = True
                break
            rsp.continue_()
            time.sleep(0.05)

        if not bad:
            print("ran %.1fs with no derail" % (time.time() - t0))
            for t, p, s, w in hist[-10:]:
                print("   t=%6.2f pc=%08x sp=%08x %s" % (t, p, s, w))
        rsp.close()
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except Exception:
            proc.kill()


def getprocessattr():
    return getattr(subprocess, "CREATE_NO_WINDOW", 0)


if __name__ == "__main__":
    main()