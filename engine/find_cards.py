import json
with open('../cards/cards.json', 'r', encoding='utf-8') as f:
    cards = json.load(f)

targets = ['園田海未', '津島善子', '天王寺璃奈']
for card_no, card in cards.items():
    name = card.get('name', '')
    for t in targets:
        if t in name and card.get('type') == 'メンバー' and card.get('rare') in ('SD', 'N', 'N-', 'R', 'R+'):
            print(f'{t}: {card_no}  cost={card.get("cost", "?")}')
            break
