import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from pathlib import Path
import parser

ABILITIES_FILE = Path('cards/abilities.json')
with open(ABILITIES_FILE, encoding='utf-8') as f:
    data = json.load(f)
data = parser.process_abilities(data)
abilities = data['unique_abilities']

def find_in_tree(root, key):
    results = set()
    def walk(d, path=''):
        if not isinstance(d, dict):
            return
        if key in d:
            v = d[key]
            if isinstance(v, list):
                for x in v:
                    results.add(str(x))
            elif isinstance(v, bool):
                results.add(str(v))
            elif v is not None:
                results.add(str(v))
        for k, v in d.items():
            if k == 'text':
                continue
            if isinstance(v, dict):
                walk(v, path + '.' + k)
            elif isinstance(v, list):
                for i, item in enumerate(v):
                    walk(item, path + '.' + k + '[' + str(i) + ']')
    walk(root)
    return results if results else None

def ability_context(a):
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    eff = a.get('effect')
    cost = a.get('cost')
    combined = {}
    if isinstance(cost, dict):
        combined['cost'] = cost
    if isinstance(eff, dict):
        combined['effect'] = eff
    return t, combined

# Show all entries that would trigger multiple_targets_missing
print('=== ALL multiple_targets_missing entries ===')
for idx, a in enumerate(abilities):
    t, combined = ability_context(a)
    has_zutsu = chr(12378)+chr(12388) in t  # ずつ
    has_sore = chr(12381)+chr(12428)+chr(12380)+chr(12428) in t  # それぞれ
    if not has_zutsu and not has_sore:
        continue
    mults = find_in_tree(combined, 'multiple_targets') or set()
    if 'True' not in mults:
        print('IDX=' + str(idx) + ' | mult=' + str(mults))
        print('  ' + t[:100])
        print()

print()
print('=== ALL ずつ entries ===')
for idx, a in enumerate(abilities):
    t, combined = ability_context(a)
    if chr(12378)+chr(12388) not in t:  # ずつ
        continue
    mults = find_in_tree(combined, 'multiple_targets') or set()
    print('IDX=' + str(idx) + ' | mult=' + str(mults))
    print('  ' + t[:100])
    print()
