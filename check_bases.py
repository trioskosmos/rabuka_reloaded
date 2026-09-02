import json
data = json.load(open('cards/cards.json', encoding='utf-8'))
fullwidth = [data[k].get('card_no', '') for k in data if '＋' in data[k].get('card_no', '')]
for cn in fullwidth[:30]:
    base = cn.replace('＋', '')
    if base in data:
        print(f'FOUND: {cn} -> base {base} exists, rare: {data[base].get("rare")}')
    else:
        print(f'MISSING BASE: {cn} -> {base}')