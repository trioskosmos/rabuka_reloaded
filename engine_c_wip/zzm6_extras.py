import glob,re,collections,sys
pat=re.compile(r'([a-z0-9_]*extra[a-z0-9_]*)\s*\(\s*[^,()]+,\s*"([A-Za-z_0-9]+)"')
c=collections.Counter(); loc=collections.defaultdict(list); fn=collections.Counter()
files=glob.glob('src/**/*.c',recursive=True)+['include/rabuka.h']
for f in files:
    for i,ln in enumerate(open(f,encoding='utf-8',errors='replace'),1):
        for m in pat.finditer(ln):
            c[m.group(2)]+=1; fn[m.group(1)]+=1
            loc[m.group(2)].append('%s:%d'%(f,i))
print('--- extras keys read, by count ---')
for k,v in sorted(c.items(),key=lambda x:(-x[1],x[0])):
    print('%-30s %3d  %s'%(k,v,', '.join(loc[k][:5])))
print('--- reader functions ---')
for k,v in sorted(fn.items(),key=lambda x:-x[1]):
    print('%-30s %3d'%(k,v))