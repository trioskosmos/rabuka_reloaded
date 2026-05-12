import json, sys
sys.path.insert(0, 'cards/ability_extraction')
import parser

with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

data = parser.process_abilities(data)

# Check for ANY effect missing action/actions
untyped = 0
custom = 0
for a in data['unique_abilities']:
    e = a.get('effect')
    if not isinstance(e, dict):
        continue
    if 'action' not in e and 'actions' not in e:
        untyped += 1
        t = a.get('triggerless_text','') or a.get('full_text','')
        print(f'UNTYPED effect: {t[:120]}')
        print(f'  effect keys: {list(e.keys())}')
        print()
    elif e.get('action') == 'custom':
        custom += 1
        t = a.get('triggerless_text','') or a.get('full_text','')
        print(f'CUSTOM action: {t[:120]}')
        print(f'  effect: {json.dumps(e, ensure_ascii=False)[:300]}')
        print()

print(f'\nUntyped (no action/actions): {untyped}')
print(f'Custom action: {custom}')
print(f'Total unique abilities: {len(data["unique_abilities"])}')
