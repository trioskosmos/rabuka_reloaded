import re, sys

FILES = [
    r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine_c_wip\src\engine.c',
    r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine_c_wip\src\turn\phase.c',
    r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine_c_wip\src\web_server.c',
]

# every place a character string is copied into a FIXED-SIZE buffer
PATS = [
    ('strncpy',   re.compile(r'\bstrncpy\s*\(')),
    ('memcpy',    re.compile(r'\bmemcpy\s*\(')),
    ('snprintf',  re.compile(r'\bsnprintf\s*\(')),
    ('byte-loop', re.compile(r'while\s*\([^)]*\[[a-zA-Z_]+\][^)]*\)\s*\{?\s*[a-zA-Z_]+\[[a-zA-Z_]+\]\s*=')),
    ('sizeof(buf)', re.compile(r'sizeof\s*\(\s*(ja|buf|out|dst|tmp|scratch)\s*\)')),
]

for f in FILES:
    src = open(f, encoding='utf-8').read()
    lines = src.split('\n')
    name = f.rsplit('\\', 1)[-1]
    print('=' * 78)
    print(name)
    print('=' * 78)
    for ln, l in enumerate(lines, 1):
        for tag, p in PATS:
            if p.search(l) and ('%s' in l or '"' in l):
                # is this fixed-cap text copy? look for sizeof/NUM/MAX on same or prev line
                ctx = l
                if ln < len(lines) and ('sizeof' in l or re.search(r'RB_[A-Z_]+', l)):
                    ctx += ' || ' + lines[ln]
                if re.search(r'sizeof\s*\(|RB_[A-Z_]*(MAX|N_|TEXT|LEN|SZ)', ctx):
                    print(f'  {tag:11} {name}:{ln}: {l.strip()[:110]}')
    print()