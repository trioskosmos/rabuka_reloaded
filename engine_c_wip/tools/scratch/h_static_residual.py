"""Classify what is LEFT after the static sweep: real frames, pointer
declarations, or §N-blocked sites. Prints the breakdown so the residual number
is attributed rather than reported as a lump.

usage: python tools/scratch/h_static_residual.py
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import h_static_fix as H  # noqa: E402

ROOT = 'tests'


def main():
    ptr = 0
    blocked = 0
    ptr_files = set()
    blocked_files = set()
    for fn in sorted(os.listdir(ROOT)):
        if not fn.endswith('.c'):
            continue
        path = os.path.join(ROOT, fn)
        before, lines, convert, reasons = H.classify(path)
        for i, line in enumerate(lines):
            j = 0
            while j < len(line) and line[j] in (0x20, 0x09):
                j += 1
            b = line[j:]
            tl = 0
            for t in H.TYPES:
                if b.startswith(t):
                    tl = len(t)
                    break
            if not tl or b[tl:tl + 1] not in (b' ', b'\t'):
                continue
            if b'static' in line or b'extern' in line:
                continue
            k = tl
            while k < len(b) and b[k] in (0x20, 0x09):
                k += 1
            if b[k:k + 1] == b'*':
                ptr += 1
                ptr_files.add(fn)
            elif convert[i]:
                pass
            else:
                blocked += 1
                blocked_files.add(fn)
    print('residual POINTER declarations (8 B, not a frame): %d in %d files'
          % (ptr, len(ptr_files)))
    print('residual BY-VALUE, blocked because the enclosing function has no')
    print('  initialiser and converting would ADD a §N site: %d in %d files'
          % (blocked, len(blocked_files)))


if __name__ == '__main__':
    main()
