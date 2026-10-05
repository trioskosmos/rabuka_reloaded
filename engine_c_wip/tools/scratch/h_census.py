"""Corpus census: case totals AND `^FAIL` counts, over every suite log.

The classifier is written against the tails that were DUMPED and read
(tools/scratch/h_tails.py -> zz_tails.txt), not against guessed regexes: the
first attempt at this file matched 12 phantom shapes because keys like
"N assertions" and "ok:" hit prose and per-case verdicts mid-report. Each shape
below is anchored, carries the count it yields, and says which number it is.
Anything unrecognised is LISTED, never dropped -- a shape this file cannot read
is exactly the shape that silently understates a total.

`block_parity`-style shapes report PASSED, not a total, and are labelled
`passed_only` so they can never be summed into a case count.

usage: python tools/scratch/h_census.py [dir]
"""
import os
import re
import sys
import collections

# name -> (regex, kind, 1-BASED group index holding the TOTAL, or None)
# Group numbering is 1-based because group(0) is the whole match, which is how
# the first version of this file asked for "35 cases, 223 assertions, 3
# failures" and crashed on int().
SHAPES = [
    ('checks/failures            "N checks, N failures"',
     re.compile(r'^(\d+) checks,\s*(\d+) failures\s*$'), 'cases', 1),
    ('ok/FAILED/CRASHED/SETUPBUG "N ok, N FAILED, N CRASHED, N SETUPBUG"',
     re.compile(r'^(\d+) ok,\s*(\d+) FAILED,\s*(\d+) CRASHED,\s*(\d+) SETUPBUG'), 'cases', 1),
    ('assertions/failures        "N assertions, N failures"',
     re.compile(r'^(\d+) assertions,\s*(\d+) failures\s*$'), 'cases', 1),
    ('checks=N failures=N',
     re.compile(r'^checks=(\d+) failures=(\d+)\s*$'), 'cases', 1),
    ('cases/assertions/failures  "N cases, N assertions, N failures"',
     re.compile(r'^(\d+) cases,\s*(\d+) assertions,\s*(\d+) failures'), 'cases', 1),
    ('"cases: N ok, N failed, N crashed"',
     re.compile(r'^cases:\s*(\d+) ok,\s*(\d+) failed,\s*(\d+) crashed'), 'cases', 1),
    ('assertions:/failures:/setup bugs:/prompts seen:',
     re.compile(r'^assertions:\s*(\d+),\s*failures:\s*(\d+),\s*setup bugs:\s*(\d+),\s*prompts seen:\s*(\d+)'),
     'cases', 1),
    ('ok/failed/crashed/expected gaps',
     re.compile(r'^(\d+) ok,\s*(\d+) failed,\s*(\d+) crashed,\s*(\d+) expected gaps'), 'cases', 1),
    ('ok/failed/crashed; checks= failures= gaps= setup_bugs=',
     re.compile(r'^(\d+) ok,\s*(\d+) failed,\s*(\d+) crashed;\s*checks=(\d+) failures=(\d+) gaps=(\d+) setup_bugs=(\d+)'),
     'cases', 5),
    ('assertion(s)/failure(s)/setup bug(s)/prompt(s)',
     re.compile(r'^(\d+) assertion\(s\),\s*(\d+) failure\(s\),\s*(\d+) setup bug\(s\),\s*(\d+) prompt\(s\)'),
     'cases', 1),
    ('cases= assertions= failures= setup_bugs= gaps= prompts=',
     re.compile(r'^cases=(\d+) assertions=(\d+) failures=(\d+) setup_bugs=(\d+) gaps=(\d+) prompts=(\d+)'),
     'cases', 2),
    ('"N test cases: N ok, N failed, N setup/vacuous, N crashed"',
     re.compile(r'^(\d+) test cases:\s*(\d+) ok,\s*(\d+) failed,\s*(\d+) setup/vacuous,\s*(\d+) crashed'),
     'cases', 2),
    ('"==== label: N cases passed, N cases failed, N crashed, N harness gap(s) ===="',
     re.compile(r'^.*==== .*: (\d+) cases passed, (\d+) cases failed'), 'cases', 1),
    ('"label: N cases, N failures"  (per-suite self-named)',
     re.compile(r'^\S+:\s*(\d+) cases,\s*(\d+) failures\s*$'), 'cases', 1),
    ('GREEN: N checks failed',
     re.compile(r'^GREEN:\s*(\d+) checks failed\s*$'), 'is_failures', 1),
    ('FAILED: N checks failed / "N checks failed"',
     re.compile(r'(\d+) checks failed'), 'is_failures', 1),
    ('"N test case(s) FAILED."  (no total printed)',
     re.compile(r'^\d+ test case\(s\) FAILED\.'), 'no_total', None),
    ('"N runtime failures"  (no total printed)',
     re.compile(r'^\d+ runtime failures\s*$'), 'no_total', None),
    ('"N failure(s) across N crashed case(s)"',
     re.compile(r'^\d+ failure\(s\) across \d+ crashed case\(s\)'), 'no_total', None),
    ('"tests failed (parity): N"  (failures only, no total)',
     re.compile(r'^tests failed \(parity\)\s*:\s*(\d+)'), 'no_total', None),
    ('"N check(s) failed"  (no total printed)',
     re.compile(r'^\d+ check\(s\) failed\s*$'), 'no_total', None),
    ('"ALL PASSED" / "CHECKS PASSED" with NO TOTAL -- BROKEN SHAPE',
     re.compile(r'(ALL .*PASSED|CHECKS PASSED|PASSED: \S+$)'), 'broken_no_total', None),
    ('aligned label block "assertions             : N" / "failures  (shared)    : N"',
     re.compile(r'^assertions\s*:\s*(\d+)\s*$'), 'cases', 1),
    ('"tests crashed (engine) : N"  (failures only, no total)',
     re.compile(r'^tests crashed \((?:engine|parity)\)\s*:?\s*(\d+)'), 'no_total', None),
    ('"expected gaps          : N"  (not a total)',
     re.compile(r'^expected gaps\s*:\s*(\d+)\s*$'), 'no_total', None),
    ('"fixture setup bugs    : N"  (not a total)',
     re.compile(r'^fixture setup bugs\s*:\s*(\d+)\s*$'), 'no_total', None),
    ('block_parity "passed: N"  (PASSED, NOT A TOTAL)',
     re.compile(r'block_parity.*\bpassed\b[:=]?\s*(\d+)'), 'passed_only', None),
    ('"FAIL <label> <N...>" or "PARITY ...: N failure(s)"',
     re.compile(r'^FAIL \S+.*|\b(\d+) real failure\(s\)'), 'no_total', None),
]

# The summary is not always in the last few lines: many suites print a debug
# trace AFTER their verdict. 8 was too narrow and produced 59 "unrecognised"
# tails that were all just the last debug line, so the window is 80.
TAIL = 80


def main():
    d = sys.argv[1] if len(sys.argv) > 1 else 'zzcorpus'
    shapes = collections.Counter()
    cases = {}
    fcounts = {}
    unrec = {}
    for fn in sorted(os.listdir(d)):
        if not fn.endswith('.out'):
            continue
        stem = fn[:-4]
        out = open(os.path.join(d, fn), 'rb').read().decode('utf-8', errors='replace')
        err = open(os.path.join(d, stem + '.err'), 'rb').read().decode('utf-8', errors='replace')
        nfail = sum(1 for l in err.split('\n') if l.startswith('FAIL'))
        nfail += sum(1 for l in out.split('\n') if l.startswith('FAIL'))
        fcounts[stem] = nfail
        # The FLAT-suite verdict layer (test_game.h's FLAT_VERDICT) prints one
        # `ok: <case>` or `FAILED: <case>` line PER TEST, so for those suites the
        # case count is countable even when the summary line omits it. Counted
        # here rather than in a shape rule because it is orthogonal: a suite can
        # print both a verdict per case AND a summary.
        nverdict = sum(1 for l in out.split('\n')
                       if l.startswith('ok:') or l.startswith('FAILED:'))
        cands = []
        for stream in (out, err):
            ls = [l.strip() for l in stream.split('\n') if l.strip()]
            cands.extend(reversed(ls[-TAIL:]))
        hit = None
        for line in cands:
            for name, rx, kind, idx in SHAPES:
                m = rx.search(line)
                if m:
                    hit = (name, kind, idx, line, m)
                    break
            if hit:
                break
        if not hit:
            shapes['UNRECOGNISED'] += 1
            cases[stem] = (nverdict or None, nfail)
            unrec[stem] = cands[-1] if cands else '(empty)'
            continue
        name, kind, idx, line, m = hit
        shapes[name] += 1
        if kind == 'cases' and idx is not None:
            cases[stem] = (int(m.group(idx)), nfail)
        elif kind == 'passed_only':
            cases[stem] = (nverdict or None, nfail)
        else:
            cases[stem] = (nverdict or None, nfail)

    print('=== %d suite logs in %s ===' % (len(cases), d))
    print('--- %d distinct summary shapes (tail of last %d non-empty lines each) ---'
          % (len(shapes), TAIL))
    for name, n in shapes.most_common():
        print('  %4d  %s' % (n, name))
    withtot = [s for s, v in cases.items() if v[0] is not None]
    tot = sum(cases[s][0] for s in withtot)
    print('--- CASE TOTALS over the %d suites that print one ---' % len(withtot))
    print('  case total   = %d' % tot)
    print('  ^FAIL lines  = %d   (counted over ALL %d suites)' % (sum(fcounts.values()), len(cases)))
    print('--- suites with NO summable case total ---')
    for s in sorted(cases):
        if cases[s][0] is None:
            print('  %-44s ^FAIL=%d' % (s, cases[s][1]))
    if unrec:
        print('--- UNRECOGNISED summary tail (%d) ---' % len(unrec))
        for s in sorted(unrec):
            print('  %-44s %s' % (s, unrec[s][:120]))
    print('--- per-suite case total AND ^FAIL, for the %d that have one ---' % len(withtot))
    for s in sorted(withtot):
        print('  %-44s cases=%-6d ^FAIL=%d' % (s, cases[s][0], cases[s][1]))


if __name__ == '__main__':
    main()
