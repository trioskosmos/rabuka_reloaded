"""Find Aqours-related cards."""
import json
cards = json.load(open('cards/cards.json', encoding='utf-8'))
for k, v in cards.items():
    unit = v.get('unit', '')
    ct = v.get('type', '')
    series = v.get('series', '')
    if 'Aqours' in unit or 'Aqours' in series:
        print(k, v.get('name',''), 'type:', ct, 'unit:', unit, 'cost:', v.get('cost'))
