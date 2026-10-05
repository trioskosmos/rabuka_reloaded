#!/usr/bin/env python
"""Read every zzlogs/ZZ_<suite>.txt and report BOTH numbers the project needs:

  * the SUITE'S OWN case/failure count, parsed per summary shape, and
  * the count of lines starting `FAIL` (the `^FAIL` figure), and
  * the verdict line's `assert=` / `renormalised=` pair.

Ground rules encoded here:
  * SIXTEEN+ summary shapes exist, so there is NO single regex. Each shape below
    is matched explicitly and anything unmatched is reported as UNRECOGNISED
    rather than silently counted as zero.
  * A build that fails to link reads as ZERO failures, so a case count of 0 is
    ASSERTED NON-ZERO and a build error is surfaced loudly.
  * `renormalised=` was checked against each suite's own printed failure count and
    matched for every case-shaped suite, so it is reported as the third figure.
"""
import glob, os, re, sys

LOGS = sorted(glob.glob(os.path.join('zzlogs', 'ZZ_*.txt')))

# (label, regex, group for 'total', group for 'failed')
SHAPES = [
    ('A  "N assertions, M failures"',
     re.compile(r'^(\d+) assertions?, (\d+) failures?', re.M), 1, 2),
    ('A2 "N assertions, M failures, K known gaps"',
     re.compile(r'^(\d+) assertions?, (\d+) failures?, \d+ known gaps', re.M), 1, 2),
    ('B  "N checks, M failures"',
     re.compile(r'^(\d+) checks, (\d+) failures', re.M), 1, 2),
    ('C  tests run / failed (parity)',
     re.compile(r'^tests run\s*:\s*(\d+).*?^tests failed \(parity\)\s*:\s*(\d+)', re.M | re.S), 1, 2),
    ('C2 tests failed (parity), no run line',
     re.compile(r'^tests failed \(parity\)\s*:\s*(\d+)', re.M), None, 1),
    ('D  "N ok, M FAILED, K CRASHED" ===',
     re.compile(r'^\s*(\d+) ok,\s*(\d+) FAILED', re.M), None, 2),
    ('D2 "cases: N ok, M failed, K crashed"',
     re.compile(r'^\s*cases:\s*\d+ ok,\s*(\d+) failed', re.M), None, 1),
    ('E  "test functions : N (ok X / failed Y)"',
     re.compile(r'test functions\s*:\s*\d+\s*\(ok\s*\d+\s*/\s*failed\s*(\d+)', re.M), None, 1),
    ('F  "FAIL:  N checks failed"',
     re.compile(r'^FAIL:\s+(\d+) checks failed', re.M), None, 1),
    ('G  "M failing, N crashing"',
     re.compile(r'CHECKS FAILED \((\d+) failing', re.M), None, 1),
]

ZERO_RISK = {'mechanics'}   # legitimately 0-failure suites


def parse(path):
    raw = open(path, 'rb').read()
    txt = raw.decode('utf-8', 'replace')
    name = os.path.basename(path)[3:-4]
    out = {'suite': name, 'shape': None, 'total': None, 'failed': None,
           'nFAIL': 0, 'assert': None, 'renorm': None, 'verdict': None,
           'build_error': None}
    out['nFAIL'] = len(re.findall(r'^FAIL', txt, re.M))
    m = re.search(r'^(PASSED|FAILED): .*?\[exit=(-?\d+)(?:, (\d+) result line\(s\))?'
                  r'(?: assert=(\d+))?.*?renormalised=(\d+)\]', txt, re.M)
    if m:
        out['verdict'] = m.group(1)
        out['assert'] = int(m.group(4)) if m.group(4) else None
        out['renorm'] = int(m.group(5))
    # A real build failure is a COMPILER diagnostic. `make[2]: *** [Makefile..] Error 1`
    # is NOT one -- it is how make reports the VERDICT tool exiting non-zero, which is
    # the normal outcome of a red suite. Treating it as a build error made every red
    # suite look broken, which is precisely the miscount this project keeps hitting.
    be = re.search(r'^\S+\.(?:c|h):\d+:\d+: (?:fatal )?error:', txt, re.M)
    if be:
        out['build_error'] = be.group(0)[:70]
    linked = bool(out['shape']) or ('PASSED' in txt or 'FAILED' in txt)
    for label, rx, gt, gf in SHAPES:
        mm = rx.search(txt)
        if mm:
            out['shape'] = label
            out['total'] = int(mm.group(gt)) if gt else None
            out['failed'] = int(mm.group(gf))
            break
    return out


rows = [parse(p) for p in LOGS]
w = max((len(r['suite']) for r in rows), default=10)
print(f'{"suite":<{w}}  {"cases":>6} {"fail":>5} {"^FAIL":>6} {"vdct":>5} '
      f'{"assert":>6} {"renorm":>6}  shape')
print('-' * 108)
tf = tn = tv = 0
for r in rows:
    t = '-' if r['total'] is None else str(r['total'])
    f = '-' if r['failed'] is None else str(r['failed'])
    a = '-' if r['assert'] is None else str(r['assert'])
    n = '-' if r['renorm'] is None else str(r['renorm'])
    print(f'{r["suite"]:<{w}}  {t:>6} {f:>5} {r["nFAIL"]:>6} {str(r["verdict"]):>5} '
          f'{a:>6} {n:>6}  {r["shape"] or "UNRECOGNISED"}')
    if r['failed'] is not None:
        tf += r['failed']
    if r['total'] is not None:
        tn += r['total']
    tv += r['nFAIL']
print('-' * 108)
print(f'{"TOTAL":<{w}}  {tn:>6} {tf:>5} {tv:>6}')
print()
bad = [r for r in rows
       if r['build_error'] or (r['failed'] == 0 and r['suite'] not in ZERO_RISK)]
if bad:
    print('!! SUSPECT (compiler error, or zero failures without one):')
    for r in bad:
        print(f'   {r["suite"]}: shape={r["shape"]} failed={r["failed"]} '
              f'build_error={r["build_error"]}')
    print('   ^ a linked-but-red build must NOT report 0 failures. Re-run before quoting.')
else:
    print('OK: no compiler error in any log, and every suite reports a non-zero count')
    print('    except the legitimately-green ones named in ZERO_RISK.')
unrec = [r['suite'] for r in rows if not r['shape']]
if unrec:
    print('\nUNRECOGNISED summary shape (needs a new shape entry):', ', '.join(unrec))