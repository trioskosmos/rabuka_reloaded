import json

with open('../cards/cards.json', encoding='utf-8') as f:
    cards_data = json.load(f)

with open('../cards/abilities.json', encoding='utf-8') as f:
    abilities = json.load(f)

card_costs = {}
for card_no, card_info in cards_data.items():
    if isinstance(card_info, dict):
        card_costs[card_no] = card_info.get('cost', 999)

ab_list = abilities.get('unique_abilities', [])
for ab in ab_list:
    if not isinstance(ab, dict):
        continue
    if ab.get('triggers') != '登場':
        continue
    if ab.get('is_null', True):
        continue
    effect = ab.get('effect', {})
    if not isinstance(effect, dict):
        continue
    
    # Skip conditioned effects
    if effect.get('condition'):
        continue
    
    action = effect.get('action', 'none')
    cards = ab.get('cards', [])
    for c in cards:
        if isinstance(c, str) and '|' in c:
            card_no = c.split('|')[0].strip()
            cost = card_costs.get(card_no, 999)
            if cost <= 2:
                print(f'{card_no} | cost={cost} | action={action}')
