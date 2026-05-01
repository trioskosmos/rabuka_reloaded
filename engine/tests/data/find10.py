import json
a = json.load(open('../../../cards/abilities.json', encoding='utf-8'))

# Find all entries sorted by card_count descending, get #10
entries = sorted(a['unique_abilities'], key=lambda x: len(x.get('cards', [])), reverse=True)
for i, ab in enumerate(entries[:15]):
    cards = ab.get('cards', [])
    trig = ab.get('triggers', '') or ''
    cost_type = (ab.get('cost') or {}).get('type', 'none') if ab.get('cost') else 'none'
    eff_action = (ab.get('effect') or {}).get('action', 'none') if ab.get('effect') else 'none'
    ft = ab.get('full_text', '')
    print(f'#{i+1}: [{len(cards)} cards] trig={trig} cost={cost_type} eff={eff_action}')
    print(f'   {ft[:100]}')
    if cards:
        print(f'   sample: {cards[0]}')
    print()
