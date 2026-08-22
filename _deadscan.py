import re, os

root = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded'
bases = [
    os.path.join(root,'engine','src'),
    os.path.join(root,'engine','tests'),
]

defs = []
corpus = []
for base in bases:
    for dirpath, dirs, files in os.walk(base):
        for fn in files:
            if fn.endswith('.rs'):
                p = os.path.join(dirpath, fn)
                try:
                    s = open(p, encoding='utf-8').read()
                except Exception:
                    continue
                corpus.append(s)
                for m in re.finditer(r'^\s*pub(?:\([^)]*\))?\s+(?:async\s+)?fn\s+([a-z_0-9]+)', s, re.M):
                    defs.append((m.group(1), os.path.relpath(p, root)))

allsrc = '\n'.join(corpus)
dead = [(n,p) for n,p in defs if len(re.findall(r'\b'+re.escape(n)+r'\b', allsrc)) <= 1]
out = [f'{p}: {n}' for n,p in sorted(dead)]
open('engine_dead.txt','w').write('\n'.join(out))
print('total suspects:', len(dead))
