import json, sys
sys.path.insert(0, 'cards/ability_extraction')
import parser

with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

data = parser.process_abilities(data)

count = 0
for a in data['unique_abilities']:
    e = a.get('effect')
    if not isinstance(e, dict):
        continue
    def has_dn(d, path=''):
        global count
        if not isinstance(d, dict):
            return []
        issues = []
        if d.get('action') == 'do_nothing':
            issues.append((path, d.get('text','')[:40]))
            count += 1
        for k in ('actions','options','primary_effect','alternative_effect'):
            sub = d.get(k)
            if isinstance(sub, list):
                for i, item in enumerate(sub):
                    issues.extend(has_dn(item, f'{path}.{k}[{i}]'))
            elif isinstance(sub, dict):
                issues.extend(has_dn(sub, f'{path}.{k}'))
        return issues
    issues = has_dn(e)
    for path, txt in issues:
        t = a.get('triggerless_text','') or a.get('full_text','')
        print(f'do_nothing at {path}: text={txt}')
        print(f'  ability: {t[:120]}')
        print()

print(f'Total remaining do_nothing: {count}')
