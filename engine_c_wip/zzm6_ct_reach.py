import json,collections
d=json.load(open('../cards/abilities.json',encoding='utf-8'))
# every object that has BOTH an `action` and a `card_type`
pairs=collections.Counter()
ct_only=collections.Counter()
def walk(o):
    if isinstance(o,dict):
        if 'card_type' in o:
            ct=o.get('card_type')
            ct_only[str(ct)]+=1
            a=o.get('action')
            if a: pairs[(str(a),str(ct))]+=1
        for v in o.values(): walk(v)
    elif isinstance(o,list):
        for v in o: walk(v)
walk(d)
print('=== all card_type values in the corpus ===')
for k,v in ct_only.most_common(): print('  %-16r %d'%(k,v))
print()
print('=== (action, card_type) pairs, top 30 ===')
for (a,c),v in pairs.most_common(30): print('  %-28s %-14r %d'%(a,c,v))
print()
print('=== non-member_card card_type on any cost-modifying action ===')
for (a,c),v in sorted(pairs.items()):
    if c!='member_card' and ('cost' in a or 'reduce' in a or 'discount' in a):
        print('  %-28s %-14r %d'%(a,c,v))
print()
print('=== max==true + count pairs (util.c:3009-3016) ===')
mc=collections.Counter()
def walk2(o):
    if isinstance(o,dict):
        if o.get('max') is True: mc[(str(o.get('action')), 'count' in o)]+=1
        for v in o.values(): walk2(v)
    elif isinstance(o,list):
        for v in o: walk2(v)
walk2(d)
for k,v in mc.most_common(): print('  action=%-24s has_count_key=%s  n=%d'%(k[0],k[1],v))