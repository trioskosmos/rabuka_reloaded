"""Fast-poll the GBA target to catch the instruction that jumps to garbage.

mGBA's GDB stub refuses VRAM/OAM, but registers are readable, so the way to
localise a wild jump is to sample the PC far faster than the poll interval
used before. This walks the PC as fast as the link allows and, on leaving the
valid ranges, reports the last stretch of PCs resolved to symbols through
arm-none-eabi-addr2line.

Usage: python gba_fastwatch.py <rom> [--seconds N] [--elf PATH]
"""

import os
import re
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gba_capture import MGBA, Rsp  # noqa: E402

ADDR2LINE = r"C:\devkitPro\devkitARM\bin\arm-none-eabi-addr2line.exe"
ROM_LO, ROM_HI = 0x08000000, 0x09FFFFFF


def ok_region(pc):
    if pc == 0:
        return False
    if pc < 0x00010000:            # BIOS (SWI handlers: VBlankIntrWait etc.)
        return True
    if ROM_LO <= pc < ROM_HI:
        return True
    if 0x02000000 <= pc < 0x02040000:   # EWRAM
        return True
    if 0x03000000 <= pc < 0x03008000:   # IWRAM
        return True
    if 0x07000000 <= pc < 0x07000400:   # OAM
        return True
    if 0x04000000 <= pc < 0x04000400:   # IO
        return True
    return False


class Syms:
    """Lazy addr2line, batched."""

    def __init__(self, elf):
        self.elf = elf
        self.cache = {}

    def lookup(self, addrs):
        todo = [a for a in addrs if a not in self.cache]
        if not todo:
            return
        args = [ADDR2LINE, "-f", "-C", "-e", self.elf] + ["%08x" % a for a in todo]
        try:
            out = subprocess.run(args, capture_output=True, text=True, timeout=60).stdout
        except Exception:
            for a in todo:
                self.cache[a] = "?"
            return
        lines = [l for l in out.splitlines() if l.strip()]
        for i, a in enumerate(todo):
            fn = lines[2 * i] if 2 * i < len(lines) else "?"
            fn = re.sub(r"\s*\|.*$", "", fn).strip()
            fn = re.sub(r"<[^<>]*>", "", fn).replace("rabuka_gba.", "").replace("rabuka_engine.", "")
            fn = re.sub(r"::h[0-9a-f]{16}.*$", "", fn)
            self.cache[a] = fn or "?"

    def get(self, pc):
        return self.cache.get(pc, "?")


def main():
    rom = sys.argv[1]
    seconds, elf = 300.0, None
    args = sys.argv[2:]
    for i, a in enumerate(args):
        if a == "--seconds":
            seconds = float(args[i + 1])
        if a == "--elf":
            elf = args[i + 1]
    syms = Syms(elf) if elf and os.path.exists(elf) else None

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

        trail = []
        t0 = time.time()
        derailed = False
        while time.time() - t0 < seconds:
            try:
                rsp.halt()
                r = rsp.regs()
            except Exception:
                break
            pc = r["pc"]
            trail.append((pc, r["sp"]))
            if len(trail) > 400:
                trail.pop(0)
            if not ok_region(pc):
                print("DERAIL t=%.2f pc=%08x sp=%08x" % (time.time() - t0, pc, r["sp"]))
                derailed = True
                break
            rsp.continue_()

        if derailed and trail:
            # Distinct PCs in the trail, newest last.
            uniq = []
            for pc, _ in trail:
                if not uniq or uniq[-1][0] != pc:
                    uniq.append((pc, None))
            pcs = [pc for pc, _ in uniq]
            if syms:
                syms.lookup(pcs)
            print("trail (%d samples, oldest first):" % len(uniq))
            for pc in pcs[-30:]:
                tag = syms.get(pc) if syms else ""
                print("   %08x  %s" % (pc, tag[:96]))
            try:
                sp = trail[-1][1]
                st = rsp.read_mem(sp, 64)
                words = [int.from_bytes(st[i:i + 4], "little") for i in range(0, 64, 4)]
                romw = [w for w in words if ROM_LO <= w < ROM_HI]
                if syms and romw:
                    syms.lookup(romw)
                print("stack:")
                for i, w in enumerate(words):
                    tag = syms.get(w) if syms and ROM_LO <= w < ROM_HI else ""
                    print("   sp+%02d %08x %s" % (i * 4, w, tag[:88]))
            except Exception as exc:
                print("stack read failed:", exc)
        else:
            print("ran %.1fs with no derail" % (time.time() - t0))
        rsp.close()
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except Exception:
            proc.kill()


if __name__ == "__main__":
    main()