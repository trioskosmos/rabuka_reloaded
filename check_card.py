import json
data = json.load(open('cards/cards.json', encoding='utf-8'))
for k in data:
    if 'bp4-004' in k and 'P' in k and 'SEC' not in k:
        card = data[k]
        print(f'Key: {repr(k)}')
        print(f'  card_no: {card.get("card_no")}')
        print(f'  name: {card.get("name")}')
        print()