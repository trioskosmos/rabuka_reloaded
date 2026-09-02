import json
data = json.load(open('cards/cards.json', encoding='utf-8'))
for k in data:
    if 'bp1-011' in k:
        card = data[k]
        print(f'Key: {repr(k)}')
        print(f'  card_no: {card.get("card_no")}')
        print(f'  rare: {card.get("rare")}')
        print()