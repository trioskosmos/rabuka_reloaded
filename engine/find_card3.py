import json

with open('../cards/cards.json', encoding='utf-8') as f:
    cards_data = json.load(f)

with open('../cards/abilities.json', encoding='utf-8') as f:
    abilities = json.load(f)

card_costs = {}
for card_no, card_info in cards_data.items():
    if isinstance(card_info, dict):
        cost = card_info.get('cost', 999)
        typ = card_info.get('type', '')
        card_costs[card_no] = (cost, typ)

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
    action = effect.get('action', 'none')
    cards = ab.get('cards', [])
    for c in cards:
        if isinstance(c, str) and '|' in c:
            card_no = c.split('|')[0].strip()
            info = card_costs.get(card_no, (999, ''))
            cost, typ = info
            if cost <= 2:
                full_text = ab.get('full_text', '')
                condition = effect.get('condition')
                has_condition = 'yes' if condition else 'no'
                text_short = full_text[:100].replace('\n', ' | ')
                print(f'[{card_no}] cost={cost} type={typ} action={action} cond={has_condition}')
