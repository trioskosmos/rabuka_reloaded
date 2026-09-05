import json
cards = json.load(open('cards/cards.json', 'r', encoding='utf-8'))
types = set(c.get('card_type') for c in cards.values())
print('Card types:', types)
for card_no, card in list(cards.items())[:10]:
    print(f'  {card_no}: type={card.get("card_type")}')