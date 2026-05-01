import json
a = json.load(open('../../../cards/abilities.json', encoding='utf-8'))
cards_db = json.load(open('../../../cards/cards.json', encoding='utf-8'))

for ab in a['unique_abilities']:
    cards = ab.get('cards', [])
    if len(cards) != 6:
        continue
    trig = ab.get('triggers', '') or ''
    cost_type = (ab.get('cost') or {}).get('type', 'none') if ab.get('cost') else 'none'
    eff_action = (ab.get('effect') or {}).get('action', 'none') if ab.get('effect') else 'none'
    if cost_type == 'change_state' and eff_action == 'change_state':
        sample = cards[0]
        card_no = sample.split(' | ')[0]
        card = cards_db.get(card_no)
        cost = card.get('cost') if card else '?'
        print(f'card: {card_no} cost={cost}')
        print(f'full_text: {ab.get("full_text", "")[:150]}')
        print(f'cost: {json.dumps(ab.get("cost"), ensure_ascii=False)}')
        print(f'effect: {json.dumps(ab.get("effect"), ensure_ascii=False)[:500]}')
        break
