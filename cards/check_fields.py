"""Check what product/series fields exist for Aqours cards."""
import json
cards = json.load(open('cards/cards.json', encoding='utf-8'))

# Find the PL!S-sd1 (Sunshine start deck) cards — they're definitely Aqours
print("=== PL!S-sd1-001 (Chika) — all fields ===")
c = cards['PL!S-sd1-001-SD']
for k, v in c.items():
    print(f"  {k}: {repr(v)[:100]}")

print()
print("=== PL!S-pb1-021 (Strawberry Trapper) — all fields ===")
c = cards['PL!S-pb1-021-L']
for k, v in c.items():
    print(f"  {k}: {repr(v)[:100]}")

print()
print("=== LL-E-001-SD (Energy card) — all fields ===")
c = cards['LL-E-001-SD']
for k, v in c.items():
    print(f"  {k}: {repr(v)[:100]}")
