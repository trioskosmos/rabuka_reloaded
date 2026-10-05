"""Enumerate the DISTINCT summary shapes in a corpus capture and count both
numbers the rules require: the suite's own case total and its `^FAIL` lines.

Sixteen shapes are known to exist and keying on the wrong one is how
parity_sequential got reported as 23 cases when it is 52 -- so this file does
NOT assume a shape. It matches every line it can, prints the shape name with
the raw line that produced it, and lists anything UNRECOGNISED rather than
dropping it. `block_parity` reports PASSED, not a total, and is labelled as
such: it can never be added into a case total.

usage: python tools/scratch/h_summaries.py [dir]
"""
import os
import re
import sys
import collections

# (shape name, regex, which number it is)
SHAPES = [
    ('A1 total/fail  "%d/%d assertions passed"',
     re.compile(r'(\d+)\s*/\s*(\d+)\s+assertions passed'), 'cases'),
    ('A2 "N ok, M FAILED, K CRASHED, P/T assertions passed"',
     re.compile(r'(\d+)\s+ok,\s*(\d+)\s+FAILED,\s*(\d+)\s+CRASHED,\s*(\d+)\s*/\s*(\d+)\s+assertions passed'), 'cases'),
    ('A3 "ALL PASSED (N assertions)"', re.compile(r'ALL PASSED \((\d+) assertions'), 'cases'),
    ('A4 "N checks failed" / FAILED: N checks failed', re.compile(r'(\d+) checks failed'), 'cases_is_failures'),
    ('A5 "N passed, M failed"', re.compile(r'(\d+) passed,\s*(\d+) failed'), 'cases'),
    ('A6 "N/M suites" or "passed N/M"', re.compile(r'(\d+)\s*/\s*(\d+)\s+(?:suites?|cases?|tests?)'), 'cases'),
    ('A7 block_parity "passed: N" (PASSED, NOT A TOTAL)',
     re.compile(r'block_parity.*passed[:=]\s*(\d+)'), 'passed_only'),
    ('A8 "cases: N"', re.compile(r'cases[:=]\s*(\d+)'), 'cases'),
    ('A9 "N cases, M failures"', re.compile(r'(\d+)\s+cases?,\s*(\d+)\s+failures?'), 'cases'),
    ('A10 "PASSED N" / "PASSED: N"', re.compile(r'PASSED[: ]\s*(\d+)'), 'passed_only'),
    ('A11 "N/M tests"', re.compile(r'(\d+)\s*/\s*(\d+)\s+tests?'), 'cases'),
    ('A12 "TOTAL N"', re.compile(r'TOTAL[:= ]\s*(\d+)'), 'cases'),
    ('A13 "N assertions"', re.compile(r'(\d+)\s+assertions?\b'), 'cases'),
    ('A14 "GREEN: 0 checks failed"', re.compile(r'GREEN:\s*0\s+checks failed'), 'zero_green'),
    ('A15 "ALL SUITES GREEN"', re.compile(r'ALL SUITES GREEN'), 'suite_level'),
    ('A16 "ok: N" verdict lines', re.compile(r'^ok:\s'), 'ok_lines'),
]


def classify(line):
    for name, rx, kind in SHAPES:
        m = rx.search(line)
        if m:
            return name, kind, m
    return None, None, None


def main():
    d = sys.argv[1] if len(sys.argv) > 1 else 'zzcorpus'
    shapes = collections.Counter()
    cases = {}
    passed_only = {}
    zero_green = []
    unknown = {}
    build_red = []
    suite_level = []
    for fn in sorted(os.listdir(d)):
        if not fn.endswith('.out'):
            continue
        stem = fn[:-4]
        raw = open(os.path.join(d, fn), 'rb').read()
        err = open(os.path.join(d, stem + '.err'), 'rb').read()
        text = raw.decode('utf-8', errors='replace')
        etext = err.decode('utf-8', errors='replace')
        both = text + '\n' + etext
        nf = len([l for l in etext.split('\n') if l.startswith('FAIL')])
        nf += len([l for l in text.split('\n') if l.startswith('FAIL')])
        found = None
        for line in reversed([l for l in both.split('\n') if l.strip()]):
            name, kind, m = classify(line)
            if name:
                found = (name, kind, line.strip(), m)
                break
        if not found:
            unknown[stem] = both.strip().split('\n')[-3:] if both.strip() else ['(empty)']
            cases[stem] = None
            continue
        name, kind, line, m = found
        shapes[name] += 1
        if kind == 'cases_is_failures':
            cases[stem] = (int(m.group(1)), nf)
        elif kind == 'cases':
            g = [int(x) for x in m.groups()]
            cases[stem] = (g[0] if len(g) == 1 else g[1], nf)
        elif kind == 'passed_only':
            passed_only[stem] = int(m.group(1))
            cases[stem] = (None, nf)
        elif kind == 'zero_green':
            zero_green.append(stem)
            cases[stem] = (None, nf)
        elif kind == 'suite_level':
            suite_level.append(stem)
            cases[stem] = (None, nf)
        else:
            cases[stem] = (None, nf)

    print('=== %d suite logs ===' % len(cases))
    print('--- summary shapes, %d distinct ---' % len(shapes))
    for name, n in shapes.most_common():
        print('  %4d  %s' % (n, name))
    tot = sum(v[0] for v in cases.values() if v and v[0] is not None)
    totf = sum(v[1] for v in cases.values() if v)
    print('--- totals over the %d suites that PRINTED A CASE COUNT ---'
          % sum(1 for v in cases.values() if v and v[0] is not None))
    print('  case total  = %d' % tot)
    print('  ^FAIL lines = %d   (all %d suites)' % (totf, len(cases)))
    print('--- suites printing NO case total (a count cannot be taken from these) ---')
    n1 = [s for s, v in cases.items() if not v or v[0] is None]
    print('  %d: %s' % (len(n1), ' '.join(sorted(n1))))
    if zero_green:
        print('--- "GREEN: 0 checks failed" with NO total (BROKEN shape, not green) ---')
        print('  %d: %s' % (len(zero_green), ' '.join(sorted(zero_green))))
    if passed_only:
        print('--- PASSED-only shapes, NOT summable as a total ---')
        for s in sorted(passed_only):
            print('  %-44s passed=%d ^FAIL=%d' % (s, passed_only[s], cases[s][1]))
    if unknown:
        print('--- UNRECOGNISED summary shape (%d) ---' % len(unknown))
        for s in sorted(unknown):
            print('  %-44s %s' % (s, unknown[s]))


if __name__ == '__main__':
    main()
