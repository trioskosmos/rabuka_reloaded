import re,collections,os
# dead keys from the measured census
pop={}
for ln in open('zzs/extras_BASE.log',encoding='utf-8',errors='replace'):
    m=re.match(r'ZZKEY\s+(\S+)\s+(\d+)',ln)
    if m: pop[m.group(1)]=int(m.group(2))
pat=re.compile(r'([a-z0-9_]*extra[a-z0-9_]*)\s*\(\s*[^,()]+,\s*"([A-Za-z_0-9]+)"')
MINE={'src/core/data.c','src/core/card.c','src/bytecode_data.c','src/core/gen_data_rom.c','src/ability/util.c','src/ability/describe.c'}
hits=collections.defaultdict(list)
for f in sorted(MINE):
    for i,ln in enumerate(open(f,encoding='utf-8',errors='replace'),1):
        for m in pat.finditer(ln):
            k=m.group(2)
            if pop.get(k,0)==0: hits[k].append('%s:%d'%(f,i))
print('=== DEAD-KEY READ SITES INSIDE THE SIX FILES I OWN ===')
tot=0
for k,v in sorted(hits.items(),key=lambda x:-len(x[1])):
    print('%-28s %d site(s): %s'%(k,len(v),', '.join(v)))
    tot+=len(v)
print('dead keys read in my files: %d, read sites: %d, of %d dead keys overall'%(len(hits),tot,51))