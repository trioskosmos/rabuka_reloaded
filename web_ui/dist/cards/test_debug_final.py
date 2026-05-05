"""Final debug: trace exactly what happens during parsing."""
import sys
import json
sys.path.insert(0, 'cards/ability_extraction')

# Load abilities.json and get the triggerless text
d = json.load(open('cards/abilities.json', encoding='utf-8'))
entry = d['unique_abilities'][523]
tt = entry['triggerless_text']

# Simulate _try_furthermore
parts = tt.split('。')
print('Parts:')
for i, p in enumerate(parts):
    p = p.strip()
    if 'さらに' in p:
        p2 = p.replace('さらに', '', 1).strip()
    else:
        p2 = p
    print(f'  [{i}]: {p2[:60]}')

# Part 2 is the one we care about
part2 = parts[1].strip().replace('さらに', '', 1).strip()
print('\nPart 2 (after strip):', part2[:80])

# Check extract_group on Part 2
from parser import extract_group, extract_duration, extract_target, extract_card_type
print('\nGroup check on part 2:', extract_group(part2))
print('Duration check on part 2:', extract_duration(part2))

# Now simulate split_condition_action
from parser import split_condition_action
ct, at = split_condition_action(part2)
print('\nCondition:', ct)
print('Action after cond:', at[:80])
print('Group in action after cond:', extract_group(at))

# Now split action text by 、
sub_parts = at.split('、')
for i, sp in enumerate(sub_parts):
    sp = sp.strip()
    print(f'\nSub-part [{i}]: {sp[:50]}')
    print(f'  group={extract_group(sp)}')
    print(f'  duration={extract_duration(sp)}')
    print(f'  target={extract_target(sp)}')
    print(f'  card_type={extract_card_type(sp)}')
