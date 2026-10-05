import re,unicodedata,collections,sys,json,os
rs=open('engine/src/core/card.rs',encoding='utf-8').read().splitlines()
cs=open('engine_c_wip/src/core/card.c',encoding='utf-8').read().splitlines()
# Rust map_series_to_group (card.rs:959-970)
rstart=959
rbody=[l for l in rs[rstart-1:rstart+12] if '=>' in l or '|' in l]
print('--- Rust arms ---')
rust_arms=[]
for l in rbody:
    m=re.findall(r'"([^"]*)"\s*(?:\||=>)',l)
    for s in m:
        rust_arms.append((s,'=>' in l))
print('--- C arms ---')
cbody=[l for l in cs if 'strcmp(series' in l]
c_arms=[]
for l in cbody:
    for m in re.finditer(r'strcmp\(series,\s*"([^"]*)"\)',l):
        c_arms.append(m.group(1))
for m in re.finditer(r'return "([^"]*)"',l if False else ''): pass
groups=[re.search(r'return "([^"]*)"',l).group(1) for l in cbody if 'return' in l]
print('C series literals:',[ (a,[hex(ord(ch)) for ch in a[:6]]) for a in c_arms])
print('C group literals:',groups)
print()
rust_keys=[s for s,_ in rust_arms]
print('RUST keys:',[(a,[hex(ord(ch)) for ch in a[:8]]) for a in rust_keys])
print('MATCH series literals:', rust_keys==c_arms)
# group mapping
rmap={}
for l in rs[rstart-1:rstart+13]:
    ms=re.findall(r'"([^"]*)"',l)
    if '=>' in l and len(ms)>=2: rmap[ms[0]]=ms[1]
    elif '|' in l and ms:
        for k in ms[:-1]: rmap[k]=ms[-1]
print('RUST map:',rmap)
print()
# Corpus: every series + group string in cards.json
cards=json.load(open('cards/cards.json',encoding='utf-8'))
if isinstance(cards,dict): cards=list(cards.values())
series=collections.Counter(); groups_seen=collections.Counter()
for c in cards:
    series[c.get('series','')]+=1
    groups_seen[c.get('group','')]+=1
print('corpus cards=%d distinct series=%d'%(len(cards),len(series)))
for s,ct in sorted(series.items(),key=lambda x:-x[1]):
    mapped=None
    for k,v in rmap.items():
        if k==s: mapped=v
    cgot = next((g for k,g in zip(c_arms,groups) if k==s),'<none>')
    ok = (mapped==cgot)
    print('  %-40r n=%-5d rust=%-10r c=%-10r %s'%(s,ct,mapped,cgot,'OK' if ok else '*** MISMATCH'))
print('distinct group strings in corpus:',dict(groups_seen))
# group strings that are NOT produced by map_series_to_group
prod=set(rmap.values())
print('corpus group values not in map_series_to_group outputs:',set(groups_seen)-prod)