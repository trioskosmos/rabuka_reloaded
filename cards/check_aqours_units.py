"""Check ALL unit values for Aqours references."""
import json
cards = json.load(open('cards/cards.json', encoding='utf-8'))

units_with_aqours = []
all_units = set()
for k, v in cards.items():
    u = v.get('unit', '')
    if u:
        all_units.add(u)
    if u and 'Aqours' in u:
        units_with_aqours.append((k, v.get('name',''), u, v.get('type','')))

print("ALL unit values containing Aqours:")
for k, name, unit, ct in units_with_aqours:
    print(f"  {k} {name} unit={unit} type={ct}")

# Also find ALL member cards in the PL!S series (Aqours franchise)
# that don't have Aqours in their unit
print()
print("PL!S member cards without Aqours in unit:")
series_wanted = ['ラブライブ！サンシャイン!!', 'Love Live! Sunshine!!']
for k, v in cards.items():
    if k.startswith('PL!S-') and v.get('type') == 'メンバー':
        u = v.get('unit', '')
        if 'Aqours' not in u:
            print(f"  {k} {v.get('name','')} unit={u}")
