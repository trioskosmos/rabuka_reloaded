import json
with open('../cards/cards.json', 'r', encoding='utf-8') as f:
    cards = json.load(f)

targets = {'umi': '園田海未', 'yoshiko': '津島善子', 'rina': '天王寺璃奈'}
found = {k: [] for k in targets}
for card_no, card in cards.items():
    name = card.get('name', '')
    for k, t in targets.items():
        if t in name and card.get('type') == 'メンバー':
            found[k].append((card_no, card.get('cost', 0)))

for k, t in targets.items():
    print(f'{t}:')
    for cn, cost in found[k][:3]:
        print(f'  --test card {cn} cost={cost}')
