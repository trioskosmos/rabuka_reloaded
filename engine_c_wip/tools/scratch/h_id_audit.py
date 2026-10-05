import os, re, collections, sys

res = []
root = 'tests'
for fn in sorted(os.listdir(root)):
    if not fn.endswith('.c') or fn == 'test_harness_self.c':
        continue
    src = open(os.path.join(root, fn), encoding='utf-8', errors='replace').read()
    funcs = []
    cur = None
    for line in src.split('\n'):
        if re.match(r'^(static\s+)?[A-Za-z_][\w \*]*\s+[A-Za-z_]\w*\s*\(', line) and line.rstrip().endswith('{'):
            cur = [line.strip(), []]
            funcs.append(cur)
        if cur is not None:
            cur[1].append(line)
    for hdr, body in funcs:
        text = '\n'.join(body)
        lits = re.findall(r'test_id\(\s*&?\w+\s*,\s*"([^"]+)"', text)
        c = collections.Counter(lits)
        for k, v in sorted(c.items()):
            if v > 1:
                res.append((fn, hdr[:70], k, v))

print('DUP FUNCTIONS:', len(res))
for r in res:
    print(r[0], '|', r[1], '|', r[2], 'x', r[3])
