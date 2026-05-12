import json

with open('../cards/cards.json', encoding='utf-8') as f:
    cards_data = json.load(f)

card_nos = ['PL!-sd1-011-SD', 'PL!-sd1-012-SD', 'PL!-sd1-016-SD', 
            'PL!HS-sd1-013-SD', 'PL!SP-bp4-013-N',
            'PL!-pb1-012-R', 'PL!-pb1-012-P+']

for cn in card_nos:
    info = cards_data.get(cn)
    if info:
        print(f'{cn}: name={info.get("name","?")}, cost={info.get("cost","?")}, type={info.get("type","?")}')
    else:
        print(f'{cn}: NOT FOUND')

# Also check sd1-019-SD and sd1-020-SD (already used)
print()
for cn in ['PL!SP-sd1-019-SD', 'PL!SP-sd1-020-SD']:
    info = cards_data.get(cn)
    if info:
        print(f'{cn}: name={info.get("name","?")}, cost={info.get("cost","?")}, type={info.get("type","?")}')
    else:
        print(f'{cn}: NOT FOUND')
