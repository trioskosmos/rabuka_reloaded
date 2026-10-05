import re, glob, os

ROOT = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded'
CR = os.path.join(ROOT, 'engine_c_wip')
RS = os.path.join(ROOT, 'engine')

card = open(os.path.join(RS, 'src/core/card.rs'), encoding='utf-8').read().split('\n')
methods = []
for i, l in enumerate(card, 1):
    m = re.match(r'\s*(pub(?:\([^)]*\))?\s+)?fn\s+([a-zA-Z0-9_]+)', l)
    if m:
        methods.append((i, m.group(2)))

cblob = '\n'.join(open(p, encoding='utf-8', errors='replace').read() for p in
                  glob.glob(os.path.join(CR, 'src', '**', '*.c'), recursive=True) +
                  glob.glob(os.path.join(CR, 'include', '*.h')))

# every OTHER rust file, with card.rs removed
rfiles = {}
for p in glob.glob(os.path.join(RS, 'src', '**', '*.rs'), recursive=True):
    if p.endswith(os.path.join('core', 'card.rs')):
        continue
    rfiles[os.path.relpath(p, RS)] = open(p, encoding='utf-8', errors='replace').read()

rows, seen = [], set()
for ln, name in methods:
    if name in seen:
        continue
    seen.add(name)
    has_c = any(c in cblob for c in {name, 'rb_' + name, 'rb_card_' + name,
                                      'rb_card_database_' + name})
    # accept BOTH `foo(` and `.foo(` and `::foo(` -- i.e. anything but `fn foo(`
    pat = re.compile(r'(?<![\w])' + re.escape(name) + r'\s*\(')
    callers = {}
    for f, src in rfiles.items():
        n = 0
        for m in pat.finditer(src):
            line = src[src.rfind('\n', 0, m.start()) + 1: m.end()]
            if re.match(r'\s*(pub(\([^)]*\))?\s+)?fn\s', line):
                continue          # a definition, not a call
            n += 1
        if n:
            callers[f] = n
    rows.append((ln, name, has_c, sum(callers.values()), callers))

noport_reach = [r for r in rows if not r[2] and r[3] > 0]
print(f'card.rs distinct methods              : {len(rows)}')
print(f'  a C twin exists (by name)          : {sum(1 for r in rows if r[2])}')
print(f'  NO C twin, CALLED from other .rs   : {len(noport_reach)}   <-- real candidates')
print(f'  NO C twin, not called anywhere     : {sum(1 for r in rows if not r[2] and r[3]==0)}')
print()
print('=== NO C twin, CALLED elsewhere in Rust ===')
for ln, name, hc, nc, callers in sorted(noport_reach, key=lambda r: -r[3]):
    top = sorted(callers.items(), key=lambda kv: -kv[1])[:4]
    print(f'  card.rs:{ln:<5} {name:<32} sites={nc:<3} ' +
          ' '.join(f'{f}:{c}' for f, c in top))