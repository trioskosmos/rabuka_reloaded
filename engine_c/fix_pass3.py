import re
p = 'tools/gen_tests.py'
lines = open(p, encoding='utf-8').read().split('\n')
for i, l in enumerate(lines):
    if l.startswith('        body = "') and i+1 < len(lines) and lines[i+1].startswith('".join'):
        lines[i] = '        body = chr(10).join(final_lines[s:e+1])'
        lines[i+1] = ''
    if l.startswith('        declared = set(re.findall(r') and 'int' in l:
        lines[i] = "        declared = set(re.findall(r'\\bint\\s+(\\w+)\\b', body))"
    if l.startswith('        declared.update(re.findall(r') and 'Card' in l:
        lines[i] = "        declared.update(re.findall(r'\\bCard\\s+(\\w+)\\b', body))"
    if l.startswith('        declared.update(re.findall(r') and 'char' in l:
        lines[i] = "        declared.update(re.findall(r'\\bchar\\s+(\\w+)\\b', body))"
    if l.startswith('        real = re.sub(r') and '//.*' in l:
        lines[i] = "        real = re.sub(r'//.*', '', body)"
    if l.startswith('        if re.search(r') and "tok" in l and "'\\s*\\('" in l:
        lines[i] = "        if re.search(r'\\b' + re.escape(tok) + r'\\s*\\(', real):"
    if l.startswith('        if re.search(r') and "'.' + re.escape" in l:
        lines[i] = "        if re.search(r'\\.' + re.escape(tok) + r'\\b', real):"
    if l.startswith('    # --- pass 4b: inline g.id("X") / v.id("X") / g2.id("X") / g.new_id("X") ->    # --- pass 4b:'):
        lines[i] = '    # --- pass 4b: inline g.id("X") / v.id("X") / g2.id("X") / g.new_id("X") ->'
lines = [l for l in lines if l != '']
open(p, 'w', encoding='utf-8').write('\n'.join(lines))
import py_compile; py_compile.compile(p, doraise=True); print('ok')