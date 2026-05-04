import json
c = json.load(open('cards/cards.json', encoding='utf-8'))

# Check what units contain Nijigasaki-related chars
for k, v in c.items():
    u = v.get('unit', '')
    if '\u8679' in u or '\u54b2' in u:
        print(repr(k) + ': unit=' + repr(u) + ' name=' + repr(v.get('name','')) + ' type=' + repr(v.get('type','')))

print('---')
# All N-bp3 cards
for k, v in c.items():
    if 'N-bp3' in k:
        print(repr(k) + ': unit=' + repr(v.get('unit','')) + ' name=' + repr(v.get('name','')) + ' type=' + repr(v.get('type','')))

print('---')
# Any card with unit exactly "虹ヶ咲" or starting with it
for k, v in c.items():
    u = v.get('unit', '')
    if u and ('虹' in u or 'niji' in u.lower()):
        print(repr(k) + ': unit=' + repr(u))
