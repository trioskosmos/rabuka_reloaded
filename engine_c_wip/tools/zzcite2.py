import re, os
RS = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine'
ph = open(os.path.join(RS, 'src/turn/phases.rs'), encoding='utf-8').read().split('\n')
cd = open(r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine_c_wip\src\turn\phase.c',
          encoding='utf-8').read().split('\n')

# exact symbol -> definition line in phases.rs
real = {}
for i, l in enumerate(ph, 1):
    m = re.match(r'\s*(pub(?:\([^)]*\))?\s+)?fn\s+([a-z_0-9]+)', l)
    if m and m.group(2) not in real:
        real[m.group(2)] = i

# every `name (phases.rs:N)` or `name (phases.rs:N-M)` citation in phase.c
print('=== phase.c -> phases.rs citations, checked against the CURRENT Rust ===')
print(f'{"sym":<44}{"cited":>8}{"real":>8}{"drift":>8}')
seen = set()
drifts = []
for i, l in enumerate(cd, 1):
    for m in re.finditer(r'([a-z_0-9]{4,})\s*\(phases\.rs:(\d+)(?:-(\d+))?\)', l):
        sym, lo, hi = m.group(1), int(m.group(2)), m.group(3)
        if sym not in real:
            continue
        r = real[sym]
        d = lo - r
        if (sym, lo) in seen:
            continue
        seen.add((sym, lo))
        drifts.append(d)
        flag = '' if abs(d) <= 3 else '   <-- WRONG LINE'
        print(f'  {sym:<42}{lo:>8}{r:>8}{d:>+8}{flag}')
if drifts:
    drifts.sort()
    print(f'\n  citations checked : {len(drifts)}')
    print(f'  correct (<=3)     : {sum(1 for d in drifts if abs(d)<=3)}')
    print(f'  STALE (>3)        : {sum(1 for d in drifts if abs(d)>3)}')
    print(f'  drift range       : {drifts[0]:+d} .. {drifts[-1]:+d}')
    import collections
    print('  drift histogram   :', dict(sorted(collections.Counter(drifts).items())))