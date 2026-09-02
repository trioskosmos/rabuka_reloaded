import re
G = r'(?:game|tg|g|v|g2|game2|self)'
stripped = 'test_add_to_energy(&tg, 0, g.id("LL-E-001-SD"));'
print('cond:', bool(re.search(r'\b(?:test_|rb_|CHECK|tg\.state|if\s*\()', stripped)))
nl = re.sub(G+r'\.new_id\s*\(\s*"([^"]+)"\s*\)', r'test_id(&tg, "\1")', stripped)
nl = re.sub(G+r'\.id\s*\(\s*"([^"]+)"\s*\)', r'test_id(&tg, "\1")', nl)
print(repr(nl))