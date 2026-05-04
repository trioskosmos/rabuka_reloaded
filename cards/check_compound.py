import json
import sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import _try_compound, _try_card_count, _try_distinct, COMPOUND_OPERATOR, COMPOUND_OPERATOR_ALT

d = json.load(open('cards/abilities.json', encoding='utf-8'))
entry = d['unique_abilities'][523]
tt = entry['triggerless_text']

# Find branch 3 text
parts = tt.split('。')
for i, p in enumerate(parts):
    p = p.strip()
    if 'さらに' in p:
        p2 = p.replace('さらに', '', 1).strip()
    else:
        p2 = p
    print(f'Part {i}: {repr(p2)}')

# Part 3 is branch 3
part3 = parts[2].strip().replace('さらに', '', 1).strip()
print()
print('Branch 3 text:', repr(part3))

# Check what handlers match
print()
print('COMPOUND_OPERATOR in text?', COMPOUND_OPERATOR in part3)
print('COMPOUND_OPERATOR_ALT in text?', COMPOUND_OPERATOR_ALT in part3)
print()
compound_result = _try_compound(part3)
print('_try_compound result:', compound_result)
print()
distinct_result = _try_distinct(part3)
print('_try_distinct result:', distinct_result)
