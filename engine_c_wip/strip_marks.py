import io, re
p = 'tests/test_audit_conditions.c'
s = io.open(p, encoding='utf-8').read()
before = s
s = re.sub(r'[ \t]*fprintf\(stderr, "MARK[^\n]*\n', '', s)
io.open(p, 'w', encoding='utf-8', newline='\n').write(s)
print('markers left:', s.count('MARK'))
print('changed:', before != s)