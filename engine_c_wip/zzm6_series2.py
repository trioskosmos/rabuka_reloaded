import re,collections,json
rs=open('engine/src/core/card.rs',encoding='utf-8').read().splitlines()[963:973]
cs=open('engine_c_wip/src/core/card.c',encoding='utf-8').read().splitlines()[366:376]
rust=[]
pending=[]
grp=None
for l in rs:
    for m in re.finditer(r'"([^"]*)"\s*(=>|\|)',l): pending.append(m.group(1))
    g=re.findall(r'"([^"]*)"',l)
    if '=>' in l and g:
        grp=g[-1]
        if l.rstrip().endswith('}') and '=>' not in pending[-1] if pending else False: pass
    # decide: does this line CLOSE an arm (has a group literal and no trailing comma-open)?
    tail=l.split('=>')[1] if '=>' in l else l
    if g and ('=>' in l or l.strip()=='}'):
        gg=g[-1]
        if '蓮ノ空' == gg or re.match(r'^[A-Za-zぁ-んァ-ヶ一-龯!\'\-]+$',gg) and gg not in pending:
            for k in pending: rust.append((k,gg))
            pending=[]
    if l.strip().startswith('_ =>'): break
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
cards=list(json.load(open('cards/cards.json',encoding='utf-8')).values())
seen=collections.Counter(x.get('series','') for x in cards)
rdict=dict(rust); cdict=dict(c)
print()
print('every distinct SERIES form in cards/cards.json (%d forms, %d cards):'%(len(seen),len(cards)))
bad=0
for s,n in sorted(seen.items(),key=lambda x:-x[1]):
    r=rdict.get(s,'"" (default arm)'); cm=cdict.get(s,'"" (default arm)')
    ok=r==cm
    if not ok: bad+=1
    print('  n=%-5d %-44r rust=%-14r c=%-14r %s'%(n,s,r,cm,'OK' if ok else '*** MISMATCH'))
print('SERIES MISMATCHES:',bad)
print()
print('every GROUP literal either engine can emit:',sorted(set(rdict.values())|set(cdict.values())))
print('corpus `group` field values:',collections.Counter(x.get('group','') for x in cards))
print()
print('every C arm, and how many corpus cards it claims:')
for k,g in c: print('  %-46r n=%-5d -> %r'%(k,seen.get(k,0),g))