import io, re, sys
a, b = sys.argv[1], sys.argv[2]
pat = re.compile(r'^\s+([A-Za-z0-9_]+)\s+\d+ assertion')
def cases(p):
    s = io.open(p, 'r', encoding='utf-8', errors='replace').read()
    return [m.group(1) for m in (pat.match(l) for l in s.split('\n')) if m]
ca, cb = cases(a), cases(b)
print("control: %d case lines, fixed: %d case lines" % (len(ca), len(cb)))
print("--- present in control only (dropped by the fix) ---")
for c in sorted(set(ca) - set(cb)): print("   ", c)
print("--- present in fixed only (added by the fix) ---")
for c in sorted(set(cb) - set(ca)): print("   ", c)
