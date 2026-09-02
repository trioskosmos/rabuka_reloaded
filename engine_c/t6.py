import re
G = r'(?:game|tg|g|v|g2|game2|self)'
cases = [
 'test_add_to_energy(&tg, 0, g.id("LL-E-001-SD"));',
 'test_add_to_discard(&tg, g.new_id(DIVE));',
 'test_add_to_deck_pl(&tg, 0, v.id("PL!-sd1-010-SD"));',
]
for s in cases:
    nl = re.sub(G+r'\.new_id\s*\(\s*([^)]*)\s*\)', r'test_id(&tg, "\1")', s)
    nl = re.sub(G+r'\.id\s*\(\s*([^)]*)\s*\)', r'test_id(&tg, "\1")', nl)
    print(repr(s), '->', repr(nl))