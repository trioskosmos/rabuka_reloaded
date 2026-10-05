import re,collections,json
rs=open('engine/src/core/card.rs',encoding='utf-8').read().splitlines()[963:973]
cs=open('engine_c_wip/src/core/card.c',encoding='utf-8').read().splitlines()[366:376]
# Rust: transcribe literally, no heuristics. Body of the match is lines 965-972.
rust=[]
for l in rs:
    if l.strip().startswith('"') and '=>' in l:
        lhs,rhs=l.split('=>',1)
        keys=re.findall(r'"([^"]*)"',lhs)
        grp=re.findall(r'"([^"]*)"',rhs)
        if not grp: grp=re.findall(r'"([^"]*)"',rs[rs.index(l)+1])
        for k in keys: rust.append((k,grp[0]))
    elif l.strip()=='}' and rust and rust[-1][1]=='__pending__':
        rust[-1]=(rust[-1][0],re.findall(r'"([^"]*)"',l)[0])
print('RUST arms (%d):'%len(rust))
for k,g in rust: print('   %-46r -> %r'%(k,g))
c=[];pend=[]
for l in cs:
    for m in re.finditer(r'!strcmp\(series,\s*"([^"]*)"\)',l): pend.append(m.group(1))
    g=re.search(r'return\s+"([^"]*)"',l)
    if g:
        for k in pend: c.append((k,g.group(1)))
        pend=[]
print('C arms (%d):'%len(c))
for k,g in c: print('   %-46r -> %r'%(k,g))
print()
print('IDENTICAL (key,group) sequence, in source order:',rust==c)
print('default arm: rust _ => "" ;  c return ""  :',
      any('Box::from("")' in l for l in rs), bool(cs))
cards=list(json.load(open('cards/cards.json',encoding='utf-8')).values())
seen=collections.Counter(x.get('series','') for x in cards)
rdict=dict(rust); cdict=dict(c)
bad=0
print()
print('every distinct SERIES form in cards/cards.json (%d forms, %d cards):'%(len(seen),len(cards)))
for s,n in sorted(seen.items(),key=lambda x:-x[1]):
    r=rdict.get(s,'""'); cm=cdict.get(s,'""')
    ok=r==cm
    if not ok: bad+=1
    print('  n=%-5d %-44r rust=%-12r c=%-12r %s'%(n,s,r,cm,'OK' if ok else '*** MISMATCH'))
print('SERIES MISMATCHES:',bad)
print()
print('GROUP literals either engine can emit:',sorted(set(rdict.values())))
print('corpus group field:',collections.Counter(x.get('group','') for x in cards))
print()
for k,g in c: print('  C arm %-46r claims %-5d corpus cards -> %r'%(k,seen.get(k,0),g))