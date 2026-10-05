import glob,re,collections
pop={}
for ln in open('zzs/extras_BASE.log',encoding='utf-8',errors='replace'):
    m=re.match(r'ZZKEY\s+(\S+)\s+(\d+)',ln)
    if m: pop[m.group(1)]=int(m.group(2))
pat=re.compile(r'([a-z0-9_]*extra[a-z0-9_]*)\s*\(\s*[^,()]+,\s*"([A-Za-z_0-9]+)"')
reads=collections.Counter(); loc=collections.defaultdict(list)
for f in glob.glob('src/**/*.c',recursive=True)+['include/rabuka.h']:
    for i,ln in enumerate(open(f,encoding='utf-8',errors='replace'),1):
        for m in pat.finditer(ln):
            reads[m.group(2)]+=1; loc[m.group(2)].append('%s:%d'%(f,i))
dead=[]; live=[]
for k,v in sorted(reads.items()):
    if pop.get(k,0)==0: dead.append((k,v))
    else: live.append((k,v,pop[k]))
print('=== READ BUT NEVER POPULATED (permanent NULL) ===')
for k,v in dead: print('%-32s reads=%-4d e.g. %s'%(k,v,', '.join(loc[k][:4])))
print('total dead keys=%d read sites=%d'%(len(dead),sum(v for _,v in dead)))
print()
print('=== READ AND POPULATED ===')
for k,v,p in live: print('%-32s reads=%-4d populated_on=%d'%(k,v,p))
print('total live keys=%d'%len(live))
print()
print('=== POPULATED BUT NEVER READ (dead store) ===')
for k,v in sorted(pop.items()):
    if k not in reads: print('%-40s populated_on=%d'%(k,v))