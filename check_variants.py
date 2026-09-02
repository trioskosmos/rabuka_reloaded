import json
data = json.load(open('cards/cards.json', encoding='utf-8'))
fullwidth = [k for k in data if '＋' in data[k].get('card_no', '')]
print(f'Fullwidth plus cards: {len(fullwidth)}')
for k in fullwidth[:20]:
    print(f'  {repr(data[k].get("card_no"))} -> rare: {data[k].get("rare")}')
print()
for suffix in ['P2', 'R2', 'L2', 'N2', 'SEC2']:
    matches = [k for k in data if data[k].get('card_no', '').endswith(suffix)]
    if matches:
        print(f'{suffix}: {len(matches)} cards')
        for k in matches[:5]:
            print(f'  {repr(data[k].get("card_no"))} -> {data[k].get("rare")}')