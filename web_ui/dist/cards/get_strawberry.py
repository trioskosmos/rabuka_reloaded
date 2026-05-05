"""Get card PL!S-pb1-021-L data."""
import json

cards = json.load(open('cards/cards.json', encoding='utf-8'))
c = cards['PL!S-pb1-021-L']
print('name:', c.get('name'))
print('type:', c.get('type'))
print('group/unit:', c.get('unit'), c.get('group'))
print('ability:', c.get('ability','')[:400])
print()
for q in c.get('faq', []):
    print('QA:', q.get('title'))
    print('  Q:', q.get('question','')[:100])
    print('  A:', q.get('answer','')[:100])
    print()

# Also check abilities.json entry
d = json.load(open('cards/abilities.json', encoding='utf-8'))
for i, entry in enumerate(d['unique_abilities']):
    for card_ref in entry.get('cards', []):
        if 'PL!S-pb1-021' in card_ref:
            print(f"Abilities.json entry {i}:")
            print(f"  trigger: {entry.get('triggers')}")
            print(f"  triggerless: {entry.get('triggerless_text','')[:200]}")
            print(f"  effect: {json.dumps(entry.get('effect',{}), indent=2, ensure_ascii=False)[:800]}")
            print()
