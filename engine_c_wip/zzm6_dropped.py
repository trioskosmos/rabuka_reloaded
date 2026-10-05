import json,collections,re
d=json.load(open('../cards/abilities.json',encoding='utf-8'))
keys=collections.Counter()
def walk(o):
    if isinstance(o,dict):
        for k,v in o.items(): keys[k]+=1; walk(v)
    elif isinstance(o,list):
        for v in o: walk(v)
walk(d)
pop={}
for ln in open('zzs/extras_BASE.log',encoding='utf-8',errors='replace'):
    m=re.match(r'ZZKEY\s+(\S+)\s+(\d+)',ln)
    if m: pop[m.group(1)]=int(m.group(2))
pat=re.compile(r'([a-z0-9_]*extra[a-z0-9_]*)\s*\(\s*[^,()]+,\s*"([A-Za-z_0-9]+)"')
reads=collections.Counter()
for f in ['../engine_c_wip/src/core/data.c']:
    pass
import glob
for f in glob.glob('src/**/*.c',recursive=True)+['include/rabuka.h']:
    for ln in open(f,encoding='utf-8',errors='replace'):
        for m in pat.finditer(ln): reads[m.group(2)]+=1
dead=[k for k in reads if pop.get(k,0)==0]
print('%-34s %8s %10s %6s'%('key','in corpus','populated','readers'))
print('--- dead keys that the CORPUS ACTUALLY CARRIES (decoder drops them) ---')
n=0
for k in sorted(dead,key=lambda x:-keys.get(x[0],0)):
    if keys.get(k,0)>0:
        n+=1; print('%-34s %8d %10s %6d'%(k,keys[k],0,reads[k]))
print('count:',n)
print()
print('--- dead keys the corpus does NOT carry (dead read, no data to lose) ---')
m=0
for k in sorted(dead):
    if keys.get(k,0)==0: m+=1; print('%-34s %8d %10s %6d'%(k,0,0,reads[k]))
print('count:',m)