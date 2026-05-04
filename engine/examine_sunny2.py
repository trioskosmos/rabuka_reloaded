import json, sys
sys.stdout.reconfigure(encoding='utf-8')
with open('../cards/abilities.json', 'r', encoding='utf-8') as f:
    abilities = json.load(f)
for entry in abilities['unique_abilities']:
    for c in entry.get('cards', []):
        if 'PL!-bp5-021-L' in c:
            eff = entry.get('effect', {})
            print(json.dumps(eff, ensure_ascii=False, indent=2))
            break
