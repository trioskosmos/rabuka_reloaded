import json
data = json.load(open('engine/baked/deck_0_cards.json', encoding='utf-8'))
print(f'Deck 0: {len(data)} cards')
for c in data[:3]:
    print(f'  {c.get("card_no")} {c.get("name")[:20]}')