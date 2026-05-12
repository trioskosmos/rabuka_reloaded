import json, sys
sys.path.insert(0, 'cards/ability_extraction/common_phrases')
from pathlib import Path
import parser

ABILITIES_FILE = Path('cards/abilities.json')
with open(ABILITIES_FILE, encoding='utf-8') as f:
    data = json.load(f)
data = parser.process_abilities(data)
abilities = data['unique_abilities']

from validate_parser import ability_context, find_in_tree

# Show all entries that would trigger multiple_targets_missing
# (contain ずつ or それぞれ but missing multiple_targets)
print('=== ALL multiple_targets_missing entries ===')
for idx, a in enumerate(abilities):
    t, combined = ability_context(a)
    has_zutsu = 'ずつ' in t
    has_sore = 'それぞれ' in t
    if not has_zutsu and not has_sore:
        continue
    mults = find_in_tree(combined, 'multiple_targets') or set()
    if 'True' not in mults:
        print('IDX=' + str(idx) + ' | mult=' + str(mults))
        print('  ' + t[:90])
        print()

print()
print('=== ずつ entries ===')
for idx, a in enumerate(abilities):
    t, combined = ability_context(a)
    if 'ずつ' not in t:
        continue
    mults = find_in_tree(combined, 'multiple_targets') or set()
    print('IDX=' + str(idx) + ' | mult=' + str(mults))
    print('  ' + t[:90])
    print()
