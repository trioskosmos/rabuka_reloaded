"""Find Aqours member cards for testing."""
import json
cards = json.load(open('cards/cards.json', encoding='utf-8'))
for k, v in cards.items():
    unit = v.get('unit', '')
    if unit and ('Aqours' in unit and 'メンバー' in v.get('type', '')):
        print(k, v.get('name',''), 'cost:', v.get('cost'), 'unit:', unit)
