"""Print the LAST 3 non-empty lines of every suite's stdout AND stderr.

Enumerating summary shapes by guessing regexes produced 12 false shapes in the
first attempt (the key `N assertions` matched prose mid-report, and `ok:` matched
a per-case verdict line). So: dump the tails, look at them, THEN write the
classifier against what is actually there.

usage: python tools/scratch/h_tails.py [dir] > zz_tails.txt
"""
import os
import sys

d = sys.argv[1] if len(sys.argv) > 1 else 'zzcorpus'
for fn in sorted(os.listdir(d)):
    if not fn.endswith('.out'):
        continue
    stem = fn[:-4]
    out = open(os.path.join(d, fn), 'rb').read().decode('utf-8', errors='replace')
    err = open(os.path.join(d, stem + '.err'), 'rb').read().decode('utf-8', errors='replace')
    ol = [l for l in out.split('\n') if l.strip()]
    el = [l for l in err.split('\n') if l.strip()]
    print('=== %s' % stem)
    for l in ol[-3:]:
        print('  O| %s' % l.strip()[:150])
    for l in el[-3:]:
        print('  E| %s' % l.strip()[:150])
