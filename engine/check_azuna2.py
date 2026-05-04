import json, sys
sys.stdout.reconfigure(encoding='utf-8')
with open('../cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

for entry in data['unique_abilities']:
    for c in entry.get('cards', []):
        if 'N-bp3-027' in c and 'ab#0' in c:
            print('=== FULL EFFECT ===')
            print(json.dumps(entry.get('effect'), ensure_ascii=False, indent=2))
            break
