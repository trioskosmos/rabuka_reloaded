import json

cards = json.load(open('cards/cards.json', encoding='utf-8'))

# Find cards whose series contains μ's (Love Live!)
for k, v in cards.items():
    name = v.get('name', '')
    unit = v.get('unit', '')
    series = v.get('series', '')
    card_type = v.get('type', '')
    rare = v.get('rare', '')
    # Check if it mentions μ's in name or series
    if 'μ' in name or 'μ' in unit or 'μ' in series:
        print(f'{k} name={name} unit={unit} type={card_type} rare={rare}')

print()
print('--- All series values ---')
for v in cards.values():
    s = v.get('series', '')
    if s:
        print(repr(s))
