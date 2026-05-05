import json

cards = json.load(open('cards/cards.json', encoding='utf-8'))

# Check unit field values
units = set()
for v in cards.values():
    u = v.get('unit')
    if u:
        units.add(u)
print('All unit values:', sorted(units))

# Find cards matching μ's 
for k, v in cards.items():
    unit = v.get('unit', '')
    group = v.get('group', '')
    if 'Muse' in unit or 'muse' in unit.lower() or unit == "μ's" or group == "μ's":
        print(f'{k} {v.get("name","")} unit={unit} group={group} type={v.get("type","")}')
