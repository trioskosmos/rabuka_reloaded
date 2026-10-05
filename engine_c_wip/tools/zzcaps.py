import re, glob, os
CR = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine_c_wip'
pat = re.compile(r'\b(RB_[A-Z0-9_]*(?:TRACE_TEXT|TEXT_CAP|MAX_TEXT|TEXT_MAX|BUF)[A-Z0-9_]*)\b\s+(\[[^\]]*\])?')
seen = {}
for p in glob.glob(os.path.join(CR, 'src', '**', '*.c'), recursive=True) + \
         glob.glob(os.path.join(CR, 'include', '*.h')) + \
         glob.glob(os.path.join(CR, 'tests', '*.c')):
    src = open(p, encoding='utf-8', errors='replace').read()
    for m in pat.finditer(src):
        ln = src[:m.start()].count('\n') + 1
        seen.setdefault(m.group(1), []).append((os.path.relpath(p, CR), ln))

print('=== text-cap constants and their declaration sites ===')
for k in sorted(seen):
    print(f'\n{k}')
    for f, ln in seen[k][:4]:
        print(f'   {f}:{ln}')

print('\n=== EVERY remaining raw byte-loop text copy in the tree ===')
loop = re.compile(r'while\s*\(\s*(\w+)\s*&&\s*\w+\s*<\s*([A-Za-z_0-9]+)\s*-\s*1\s*\)\s*\{?\s*\w+\[\w+\]\s*=')
for p in glob.glob(os.path.join(CR, 'src', '**', '*.c'), recursive=True):
    src = open(p, encoding='utf-8', errors='replace').read()
    lines = src.split('\n')
    for i, l in enumerate(lines):
        if loop.search(l):
            ctx = ' '.join(lines[max(0,i-4):i+3])
            has_fix = ('0xC0' in ctx) or ('0xc0' in ctx) or ('continuation' in ctx.lower())
            print(f'  {os.path.relpath(p, CR)}:{i+1}: {l.strip()[:88]}   '
                  f'[UTF8-WALK-BACK PRESENT: {has_fix}]')