"""Capture the SRAM frames from src/shot.rs by reading SRAM over mGBA's GDB stub.

This is far more reliable than relying on the emulator to flush a .sav on exit:
the game halts on a halt request, we read the 32 KiB SRAM window directly, and
nothing has to be flushed at all.

Usage: python gba_grab.py <rom> <out.sav> [--seconds N]
"""

import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gba_capture import MGBA, Rsp  # noqa: E402

SRAM = 0x0E000000
SRAM_LEN = 0x8000


def main():
    rom, out = sys.argv[1], sys.argv[2]
    seconds = 12.0
    args = sys.argv[3:]
    for i, a in enumerate(args):
        if a == "--seconds":
            seconds = float(args[i + 1])

    proc = subprocess.Popen(
        [MGBA, "--gdb", rom],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
    )
    try:
        time.sleep(2.5)
        rsp = Rsp()
        rsp.cmd("?")
        rsp.continue_()
        time.sleep(seconds)
        rsp.halt()
        r = rsp.regs()
        print("pc=%08x sp=%08x" % (r["pc"], r["sp"]))
        raw = rsp.read_mem(SRAM, SRAM_LEN)
        open(out, "wb").write(raw)
        print("read %d bytes -> %s  magic=%r" % (len(raw), out, raw[:2]))
        rsp.close()
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except Exception:
            proc.kill()


if __name__ == "__main__":
    main()