import io, re, sys
s = io.open("tests/parity_zones_position.log", 'r', encoding='utf-8', errors='replace').read() if False else None
p = sys.argv[1]
data = io.open(p, 'r', encoding='utf-8', errors='replace').read()
for l in data.split('\n'):
    if 'diagnostic: route' in l or 'FAIL' in l and ('route' in l or 'entry carries' in l or 'there IS' in l):
        print(l)
