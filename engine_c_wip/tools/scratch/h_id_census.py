"""Measure `test_id(` occurrences in tests/*.c two ways.

RAW     : every occurrence on a line, exactly what test_harness_self.c's
          §D detector counts today (comments and string literals included).
CODE    : the same, after block comments and // comments are blanked, so
          prose can no longer move the number.

usage: python tools/scratch/h_id_census.py            (whole-corpus totals)
       python tools/scratch/h_id_census.py --per      (per-file)
"""
import os
import re
import sys
import json

ROOT = 'tests'
SELF = 'test_harness_self.c'


def blank_comments(text):
    out = []
    i = 0
    n = len(text)
    state = 0  # 0 code, 1 block, 2 line
    while i < n:
        c = text[i]
        nxt = text[i + 1] if i + 1 < n else ''
        if state == 0:
            if c == '/' and nxt == '*':
                state = 1
                out.append('  ')
                i += 2
                continue
            if c == '/' and nxt == '/':
                state = 2
                out.append('  ')
                i += 2
                continue
            out.append(c)
            i += 1
            continue
        if state == 1:
            if c == '*' and nxt == '/':
                state = 0
                out.append('  ')
                i += 2
                continue
            out.append('\n' if c == '\n' else ' ')
            i += 1
            continue
        # line comment
        if c == '\n':
            state = 0
            out.append(c)
        else:
            out.append(' ')
        i += 1
    return ''.join(out)


def count(text):
    """One hit per LINE, matching the detector: it breaks out of the line
    after the first `test_id(` so a line with two hits counts once."""
    n = 0
    for line in text.split('\n'):
        p = line
        while True:
            i = p.find('test_id')
            if i < 0:
                break
            p = p[i + 7:]
            if p[:1] == '(':
                n += 1
                break
    return n


def main():
    per = bool('--per' in sys.argv)
    raw_total = code_total = 0
    raw_files = code_files = 0
    rows = []
    for fn in sorted(os.listdir(ROOT)):
        if not fn.endswith('.c') or fn == SELF:
            continue
        text = open(os.path.join(ROOT, fn), encoding='utf-8', errors='replace').read()
        r = count(text)
        c = count(blank_comments(text))
        raw_total += r
        code_total += c
        if r:
            raw_files += 1
        if c:
            code_files += 1
        rows.append((fn, r, c))
    if per:
        for fn, r, c in rows:
            if r or c:
                print('%-46s raw=%-5d code=%d' % (fn, r, c))
    print('RAW  calls=%d files=%d' % (raw_total, raw_files))
    print('CODE calls=%d files=%d' % (code_total, code_files))
    json.dump(rows, open('zz_id_census_measure.json', 'w'), indent=0)


if __name__ == '__main__':
    main()
