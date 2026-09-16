import json, os, re, collections
d = json.load(open('cards/abilities.json', encoding='utf-8'))
uas = d['unique_abilities']

card_tests = collections.defaultdict(set)
for root, dirs, files in os.walk('engine/tests/test_modules'):
    for f in files:
        if not f.endswith('.rs'):
            continue
        path = os.path.join(root, f)
        t = open(path, encoding='utf-8', errors='ignore').read()
        for m in re.findall(r'"((?:PL!|LL-)[^"]+)"', t):
            card_tests[m.strip().upper()].add(f)

pat_tests = collections.defaultdict(set)
pat_count = collections.Counter()
for a in uas:
    t = a.get('triggers') or 'none'
    if isinstance(t, list):
        t = t[0]
    eff = a.get('effect')
    acts = []

    def walk(node, depth):
        if isinstance(node, dict):
            if 'action' in node and depth == 0:
                acts.append(node['action'])
            for k, v in node.items():
                if k != 'action':
                    walk(v, depth + 1 if 'action' in node else depth)
        elif isinstance(node, list):
            for v in node:
                walk(v, depth)

    walk(eff, 0)
    key = (t, tuple(sorted(set(acts))))
    pat_count[key] += 1
    for c in (a.get('cards') or []):
        no = c.split('|')[0].strip()
        fs = card_tests.get(no.upper(), set())
        pat_tests[key] |= fs

out = ['pattern -> #abilities, #distinct test files touching it']
rows = sorted(pat_count.items(), key=lambda kv: -kv[1])
for (t, a), c in rows[:35]:
    nfiles = len(pat_tests[(t, a)])
    joined = ' + '.join(a) if a else '(empty)'
    out.append('%3d abilities  %3d files  [%s] %s' % (c, nfiles, t, joined))
open('test_pattern_report.txt', 'w', encoding='utf-8').write(chr(10).join(out))
print('written', len(rows))
