import re, sys
p = 'tests/test_ported_generated.c'
src = open(p, encoding='utf-8', errors='replace').read().split('\n')

# 1. Inline g.id("X") / v.id("X") / g2.id("X") / g.new_id("X") -> test_id(&tg, "X")
#    (only string-literal args; bare const names stay as TODO fallback)
def sub_id(m):
    arg = m.group(1).strip()
    if not ((arg.startswith('"') and arg.endswith('"')) or (arg.startswith("'") and arg.endswith("'"))):
        return m.group(0)
    return 'test_id(&tg, "%s")' % arg[1:-1]

G = r'(?:game|tg|g|v|g2|game2|self)'
new = []
for l in src:
    l = re.sub(G+r'\.new_id\s*\(\s*([^)]*)\s*\)', sub_id, l)
    l = re.sub(G+r'\.id\s*\(\s*([^)]*)\s*\)', sub_id, l)
    new.append(l)
src = new

# 2. Terminate bare test_get_*_modifier(...) / test_id(...) expression statements
out = []
for l in src:
    s = l.split('//')[0].rstrip()
    if (s.startswith('test_get_') or s.startswith('test_id(&tg')) and s.endswith(')') and not s.endswith(';'):
        l = l.rstrip() + ';'
    out.append(l)
src = out

# 3. Declare `idx`/`i`/`v`/`g`/`g2` if used but undeclared (auto-fix missing decl already
#    handles most; here we only add the specific ones the compiler complains about).
#    (handled by postprocess pass 3 already)

open(p, 'w', encoding='utf-8').write('\n'.join(src))
print('post-processed', p)