import json, glob, os, collections, sys
ROOT = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded'
p = glob.glob(os.path.join(ROOT, '**', 'cards', 'cards.json'), recursive=True)
print('corpus path:', p)
cards = json.load(open(p[0], encoding='utf-8'))
if isinstance(cards, dict):
    cards = list(cards.values())
names = [c['name'] for c in cards if isinstance(c, dict) and isinstance(c.get('name'), str)]
print('cards:', len(cards), 'names:', len(names), 'distinct:', len(set(names)))
nl = sorted({n for n in names if '\n' in n or '\r' in n})
dq = sorted({n for n in names if '"' in n or '\\' in n})
print()
print('DISTINCT names with literal newline :', len(nl))
for n in nl: print('   ', repr(n))
print('DISTINCT names with dquote/backslash:', len(dq))
for n in dq: print('   ', repr(n))
# multi-name cards
multi = [c for c in cards if isinstance(c, dict) and isinstance(c.get('name'), str)
         and ('\n' in c['name'] or '"' in c['name'])]
print('\ncards (rows) with newline-or-dquote name:', len(multi))
for c in multi[:6]:
    print('   ', c['card_no'], repr(c['name']))
# also check name_key / faq / any string field for C0
print('\n--- longest string values per field (top 8) ---')
fields = collections.defaultdict(list)
for c in cards:
    if not isinstance(c, dict): continue
    for k, v in c.items():
        if isinstance(v, str):
            fields[k].append(len(v.encode('utf-8')))
for k, v in sorted(fields.items(), key=lambda kv: -max(kv[1])):
    print(f'   {k:<14} n={len(v):>5} max={max(v):>5} p99={sorted(v)[int(len(v)*.99)]:>5}')