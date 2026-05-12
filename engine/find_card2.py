import json

with open('../cards/cards.json', encoding='utf-8') as f:
    cards_data = json.load(f)

with open('../cards/abilities.json', encoding='utf-8') as f:
    abilities = json.load(f)

# Get card costs
card_costs = {}
card_names = {}
for card_no, card_info in cards_data.items():
    if isinstance(card_info, dict):
        cost = card_info.get('cost', 999)
        card_costs[card_no] = cost
        card_names[card_no] = card_info.get('name', '')

# Find debut abilities for cards with cost <= 2
ab_list = abilities.get('unique_abilities', [])
found = []
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
    if effect.get('action') == 'move_cards':
        continue
    
    cards = ab.get('cards', [])
    for c in cards:
        if isinstance(c, str) and '|' in c:
            card_no = c.split('|')[0].strip()
            cost = card_costs.get(card_no, 999)
            if cost <= 2:
                name = card_names.get(card_no, '??')
                action = effect.get('action', 'none')
                found.append((card_no, name, cost, action, ab.get('full_text', '')[:60]))
                if len(found) >= 10:
                    break
    if len(found) >= 10:
        break

print(f'Found {len(found)} cards')
for card_no, name, cost, action, text in found:
    print(f'{card_no} | {name} | cost={cost} | action={action} | {text}')
