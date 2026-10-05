import io, re, sys
pat = re.compile(r'^\s+([A-Za-z0-9_]+)\s+\d+ assertion\(s\), (\d+) failure')
def cases(p):
    s = io.open(p, 'r', encoding='utf-8', errors='replace').read()
    d = {}
    for l in s.split('\n'):
        m = pat.match(l)
        if m: d[m.group(1)] = int(m.group(2))
    return d
a, b = cases(sys.argv[1]), cases(sys.argv[2])
print("cases with failures in CONTROL but not in FIXED (fixed by the change):")
for k in sorted(a):
    if a[k] and not b.get(k): print("   %-56s control_fail=%d" % (k, a[k]))
print("cases with failures in FIXED but not in CONTROL (REGRESSIONS):")
for k in sorted(b):
    if b[k] and not a.get(k): print("   %-56s fixed_fail=%d" % (k, b[k]))
print("cases failing in BOTH: %d" % len([k for k in a if a[k] and b.get(k)]))
