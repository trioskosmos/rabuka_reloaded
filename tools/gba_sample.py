"""Watch the target from boot and report register state at a fixed time, plus
a scan of low memory to characterise the runaway writes.

The runaway shows as a sequential Store32 walk upward from address 0 with the
PC still in valid code, so the writer is not lost -- it is looping. The useful
artefact is which function the PC sits in while the writes are happening.
"""

import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gba_capture import MGBA, Rsp  # noqa: E402

ADDR2LINE = r"C:\devkitPro\devkitARM\bin\arm-none-eabi-addr2line.exe"


def resolve(elf, addrs):
    if not addrs:
        return {}
    args = [ADDR2LINE, "-f", "-C", "-e", elf] + ["%08x" % a for a in addrs]
    try:
        out = subprocess.run(args, capture_output=True, text=True, timeout=90).stdout
    except Exception:
        return {}
    lines = [l for l in out.splitlines() if l.strip()]
    res = {}
    for i, a in enumerate(addrs):
        fn = lines[2 * i] if 2 * i < len(lines) else "?"
        res[a] = fn.split("|")[0].strip()
    return res


def main():
    rom = sys.argv[1]
    samples = int(sys.argv[2]) if len(sys.argv) > 2 else 12
    elf = r"C:\rust_targets\thumbv4t-none-eabi\release\smoke_autoplay"
    if "--elf" in sys.argv:
        elf = sys.argv[sys.argv.index("--elf") + 1]

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
        seen = []
        for k in range(samples):
            time.sleep(0.4)
            try:
                rsp.halt()
                r = rsp.regs()
            except Exception as exc:
                print("halt failed at sample %d: %s" % (k, exc))
                break
            seen.append((round(k * 0.4, 1), r["pc"], r["sp"]))
            rsp.continue_()
        syms = resolve(elf, [pc for _, pc, _ in seen])
        print("t      pc        sp        where")
        for t, pc, sp in seen:
            print("%-6s %08x  %08x  %s" % (t, pc, sp, syms.get(pc, "?")[:90]))
        # The stack tells us who is looping.
        rsp.halt()
        sp = rsp.regs()["sp"]
        try:
            st = rsp.read_mem(sp & 0xFFFFFFF0, 128)
            words = [int.from_bytes(st[i:i + 4], "little") for i in range(0, 128, 4)]
            romw = [w for w in words if 0x08000000 <= w < 0x09FFFFFF]
            s2 = resolve(elf, romw)
            print("\nstack (sp=%08x), ROM-looking return addresses:" % sp)
            for i, w in enumerate(words):
                if 0x08000000 <= w < 0x09FFFFFF:
                    print("   sp+%03d %08x  %s" % (i * 4, w, s2.get(w, "?")[:90]))
        except Exception as exc:
            print("stack read failed:", exc)
        rsp.close()
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except Exception:
            proc.kill()


if __name__ == "__main__":
    main()