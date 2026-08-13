"""Build a compact subsetted Japanese BCFNT font for the 3DS.

Pipeline:
  1. Scan cards.json / abilities.json / locales / decks -> set of used chars
  2. Write whitelist of codepoints (mkbcfnt needs "-w <codepoints>")
  3. Run devkitPro mkbcfnt on the source TTF at the target size
  4. Copy the result to romfs/font.bcfnt

Rebuilds automatically whenever the used-character set changes. The size is
intentionally small so mkbcfnt stays under its per-run glyph limit; text is
scaled up by the game renderer.
"""

import glob
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.normpath(os.path.join(os.path.dirname(__file__), "..", "..", "..", "tools", "font")))
from used_chars import compute_used_chars  # single source of truth for glyphs

SRC_TTF = os.path.normpath(os.path.join(os.path.dirname(__file__), "..", "..", "..", "tools", "font", "MPLUS1-Regular.ttf"))
WORK_DIR = os.path.normpath(os.path.join(os.path.dirname(__file__), "..", ".font_tmp"))
DEST = os.path.normpath(os.path.join(os.path.dirname(__file__), "..", "romfs", "font.bcfnt"))
SIZE = int(os.environ.get("RABUKA_FONT_SIZE", "24"))
MKBCFNT = os.path.normpath(os.path.join(os.environ.get("DEVKITPRO", "C:/devkitPro"), "tools", "bin", "mkbcfnt.exe"))
ROOT = os.path.normpath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))


def main():
    os.makedirs(WORK_DIR, exist_ok=True)
    if not os.path.exists(SRC_TTF):
        print(f"[font] source font missing: {SRC_TTF}")
        return 1

    chars = compute_used_chars(ROOT)
    wl = os.path.join(WORK_DIR, "wl.txt")
    new_wl = " ".join(str(ord(c)) for c in chars)
    try:
        old_wl = open(wl, encoding="utf-8").read()
    except OSError:
        old_wl = ""
    if old_wl == new_wl and os.path.exists(DEST):
        print(f"[font] used chars unchanged ({len(chars)}) - font up to date")
        return 0

    open(wl, "w", encoding="utf-8").write(new_wl)
    out = os.path.join(WORK_DIR, "font.bcfnt")
    if os.path.exists(out):
        os.remove(out)
    cmd = [MKBCFNT, "-o", out, "-s", str(SIZE), "-w", wl, SRC_TTF]
    print(f"[font] building {len(chars)} glyphs at {SIZE}px...")
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode != 0 or not os.path.exists(out):
        print(f"[font] mkbcfnt FAILED ({r.returncode}): {r.stderr}", file=sys.stderr)
        return 1
    import shutil
    shutil.copyfile(out, DEST)
    print(f"[font] wrote {DEST} ({os.path.getsize(DEST)//1024} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
