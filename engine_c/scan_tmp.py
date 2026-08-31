import re, glob, os
ROOT = os.path.dirname(os.path.abspath(__file__))
files = []
for ext in ('*.c','*.h'):
    files += glob.glob(os.path.join(ROOT,'src','**',ext), recursive=True)
    files += glob.glob(os.path.join(ROOT,'include','**',ext), recursive=True)
fn_re = re.compile(r'\b([A-Za-z_]\w*)\s*\(([^;{]*)\)\s*\{', re.S)
marker = re.compile(r'\b(stub|stubbed|not tracked|not yet|not implemented|unimplemented|placeholder|no-op|noop|todo|fixme|returns 0 \(not|unfilled)\b', re.I)
hits = []
for f in files:
    src = open(f, encoding='utf-8', errors='ignore').read()
    for m in fn_re.finditer(src):
        name = m.group(1)
        if name in ('if','for','while','switch','do','return','catch','sizeof'): continue
        start = m.end(); depth=1; i=start
        while i < len(src) and depth>0:
            if src[i]=='{': depth+=1
            elif src[i]=='}': depth-=1
            i+=1
        body = src[start:i-1]
        pre = src[:m.start()]
        cstart = pre.rfind('/*')
        cmt = pre[cstart:] if cstart>=0 else ''
        if marker.search(body) or marker.search(cmt) or body.strip()=='':
            rel = f.replace(ROOT+'/','')
            line = src[:m.start()].count('\n')+1
            hits.append((rel, line, name, (body.strip()=='')))
seen=set(); out=[]
for rel,line,name,empty in sorted(hits):
    key=(rel,name)
    if key in seen: continue
    seen.add(key)
    out.append(f"{rel}:{line}: {name}{'  [EMPTY]' if empty else ''}")
print('\n'.join(out))
print("TOTAL:", len(out))
