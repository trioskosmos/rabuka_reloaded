import re
p = 'tools/gen_tests.py'
lines = open(p, encoding='utf-8').read().split('\n')
for i, l in enumerate(lines):
    if l.strip().startswith('if re.search(r') and "tok" in l:
        lines[i] = "            if re.search(r'\\b' + re.escape(tok) + r'\\s*\\(', real):"
    if l.strip().startswith('if re.search(r') and "re.escape" in l and "real" in l:
        lines[i] = "            if re.search(r'\\.' + re.escape(tok) + r'\\b', real):"
open(p, 'w', encoding='utf-8').write('\n'.join(lines))
import py_compile; py_compile.compile(p, doraise=True); print('ok')