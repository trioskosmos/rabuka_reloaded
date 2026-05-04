import json, sys
sys.stdout.reconfigure(encoding='utf-8')
with open('../cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

# Find the La Bella Patria / A・ZU・NA ability
# Card PL!N-bp3-027-L | La Bella Patria (ab#0)
for entry in data['unique_abilities']:
    for c in entry.get('cards', []):
        if 'N-bp3-027' in c and 'ab#0' in c:
            print('=== ABILITY TEXT ===')
            print(entry['full_text'][:200])
            print()
            print('=== PARSED TRIGGER ===')
            print(entry.get('triggers'))
            print()
            print('=== COST ===')
            print(json.dumps(entry.get('cost'), ensure_ascii=False, indent=2))
            print()
            print('=== EFFECT ===')
            print(json.dumps(entry.get('effect'), ensure_ascii=False, indent=2)[:500])
            break
