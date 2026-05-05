import json

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

ua = data.get('unique_abilities', [])
print(f"Total unique abilities: {len(ua)}")

# Find all entries with PL!-bp5-021
for i, entry in enumerate(ua):
    cards = entry.get('cards', [])
    for c in cards:
        if 'PL!-bp5-021' in c:
            print(f"\nEntry #{i}:")
            print(f"  full_text: {entry.get('full_text', '')[:80]}")
            effect = entry.get('effect', {})
            print(f"  effect action: {effect.get('action')}")
            if effect.get('actions'):
                for j, a in enumerate(effect['actions']):
                    a_text = a.get('text', '')[:40]
                    a_action = a.get('action')
                    print(f"  sub-action[{j}]: {a_action} text='{a_text}'")
                    if a.get('actions'):
                        for k, sa in enumerate(a.get('actions', [])):
                            print(f"    inner[{k}]: {sa.get('action')} text='{sa.get('text','')[:30]}' dur={sa.get('duration')}")
