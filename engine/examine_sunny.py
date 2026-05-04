import json, sys
sys.stdout.reconfigure(encoding='utf-8')

# Card data
with open('../cards/cards.json', 'r', encoding='utf-8') as f:
    cards = json.load(f)

card = cards.get('PL!-bp5-021-L')
if card:
    print('=== CARD ABILITY TEXT ===')
    print(card.get('ability', 'MISSING'))
    print()

# Abilities.json
with open('../cards/abilities.json', 'r', encoding='utf-8') as f:
    abilities = json.load(f)

for entry in abilities['unique_abilities']:
    for c in entry.get('cards', []):
        if 'PL!-bp5-021-L' in c:
            print('=== ABILITIES.JSON ENTRY ===')
            print('Full text:', entry.get('full_text', '')[:300])
            print()
            print('Trigger:', entry.get('triggers'))
            print('Cost:', json.dumps(entry.get('cost'), ensure_ascii=False))
            print()
            print('Effect:', json.dumps(entry.get('effect'), ensure_ascii=False, indent=2)[:1500])
            print()
            break
