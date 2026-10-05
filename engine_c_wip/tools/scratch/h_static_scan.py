"""Census of non-static `GameState` / `TestGame` locals in tests/*.c.

The detector mirrors test_harness_self.c §B scan_one EXACTLY (column-0
declaration, by value, no `static` on the line), and additionally reports
per-file CR/LF byte counts so a rewrite can be checked for line-ending
damage before and after.

usage: python tools/scratch/h_static_scan.py            (totals + per file)
       python tools/scratch/h_static_scan.py --json     (machine readable)
"""
import json
import os
import sys

ROOT = 'tests'
TYPES = ('GameState', 'TestGame')


def scan(path):
    data = open(path, 'rb').read()
    cr = data.count(b'\r')
    lf = data.count(b'\n')
    nonascii = sum(1 for b in data if b > 127)
    lines = data.decode('utf-8', errors='replace').split('\n')
    hits = []
    for i, line in enumerate(lines):
        p = line.lstrip(' \t')
        tlen = 0
        for t in TYPES:
            if p.startswith(t):
                tlen = len(t)
                break
        if not tlen:
            continue
        c = p[tlen]
        if c == '*':
            continue
        if c not in (' ', '\t'):
            continue
        if 'static' in line:
            continue
        hits.append((i + 1, line.rstrip('\r')))
    return hits, cr, lf, nonascii, len(data)


def main():
    rows = []
    total = 0
    files = 0
    for fn in sorted(os.listdir(ROOT)):
        if not fn.endswith('.c'):
            continue
        path = os.path.join(ROOT, fn)
        hits, cr, lf, na, size = scan(path)
        if hits:
            files += 1
            total += len(hits)
            rows.append({'file': fn, 'n': len(hits), 'cr': cr, 'lf': lf,
                         'na': na, 'size': size,
                         'lines': [h[0] for h in hits]})
    if '--json' in sys.argv:
        json.dump({'total': total, 'files': files, 'rows': rows},
                  open('zz_static_scan.json', 'w'), indent=1)
    else:
        for r in rows:
            print('%-46s %4d  (cr=%d lf=%d na=%d)' % (
                r['file'], r['n'], r['cr'], r['lf'], r['na']))
    print('TOTAL non-static state locals = %d in %d files' % (total, files))


if __name__ == '__main__':
    main()
