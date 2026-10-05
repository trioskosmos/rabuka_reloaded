"""Dump the GBA's IO/display registers while the game is running.

mGBA will not serve VRAM or OAM over GDB (E06), but I/O registers are
readable, and that is enough to tell whether the display is even configured:
DISPCNT mode/BG enables/OBJ enable, the four BGCNT words (charbase,
screenbase, tile format), VCOUNT, and the interrupt state (IE/IF/IME).
"""

import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gba_capture import MGBA, Rsp  # noqa: E402

# DISPCNT sub-registers in the 0x04000000 page.
NAMES = {
    0x00: "DISPCNT", 0x04: "DISPSTAT", 0x06: "VCOUNT",
    0x08: "BG0CNT", 0x0A: "BG1CNT", 0x0C: "BG2CNT", 0x0E: "BG3CNT",
    0x10: "BG0HOFS", 0x12: "BG0VOFS",
    0x48: "BLDCNT", 0x4A: "BLDALPHA", 0x50: "BLDY",
}
INTS = {0x00: "IE", 0x02: "IF", 0x04: "IME", 0x08: "IE2", 0x0A: "IF2"}


def dump(rsp, base, table, title):
    print("--- %s ---" % title)
    for off, name in sorted(table.items()):
        try:
            raw = rsp.read_mem(base + off, 2)
            v = int.from_bytes(raw, "little")
            extra = ""
            if name == "DISPCNT":
                extra = "  mode=%d obj1d=%d obj_on=%d bg_on=%x blank=%d" % (
                    v & 3, (v >> 6) & 1, (v >> 12) & 1,
                    (v >> 8) & 0xF, (v >> 7) & 1)
            if name.startswith("BG") and name.endswith("CNT"):
                extra = "  prio=%d charbase=%d tilefmt=%s screenbase=%d size=%d" % (
                    v & 3, ((v >> 2) & 3) * 0x4000,
                    "8bpp" if (v >> 7) & 1 else "4bpp",
                    ((v >> 8) & 0x1F) * 0x800, (v >> 14) & 3)
            if name == "DISPSTAT":
                extra = "  vblank_irq=%d hblank_irq=%d vcount=%d" % (
                    (v >> 3) & 1, (v >> 4) & 1, (v >> 8) & 0xFF)
            if name == "VCOUNT":
                extra = "  %s" % ("in vblank" if v >= 160 else "in visible")
            print("  %-9s %04x%s" % (name, v, extra))
        except Exception as exc:
            print("  %-9s <%s>" % (name, exc))


def main():
    rom = sys.argv[1]
    seconds = float(sys.argv[2]) if len(sys.argv) > 2 else 8.0
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
        print("pc=%08x sp=%08x lr=%08x" % (r["pc"], r["sp"], r["lr"]))
        dump(rsp, 0x04000000, NAMES, "display")
        dump(rsp, 0x04000000, INTS, "interrupts")
        # OBJ palette bank 15 first entry + BG bank 15 first entry: tells us
        # whether the palettes were ever written.
        for label, addr in (("objpal[0]", 0x05000200), ("objpal[240]", 0x050003E0),
                            ("bgpal[bank15][0]", 0x050001F0), ("bgpal[bank15][1]", 0x050001F2)):
            try:
                v = int.from_bytes(rsp.read_mem(addr, 2), "little")
                print("  %-18s %04x -> rgb(%d,%d,%d)" % (
                    label, v, (v & 0x1F) << 3, ((v >> 5) & 0x1F) << 3,
                    ((v >> 10) & 0x1F) << 3))
            except Exception as exc:
                print("  %-18s <%s>" % (label, exc))
        rsp.close()
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except Exception:
            proc.kill()


if __name__ == "__main__":
    main()