import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_ability, parse_effect, parse_action, normalize_fullwidth_digits

with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

a = data['unique_abilities'][150]
t = a['triggerless_text']
print('Triggerless:', t)
print()

# The issue: sub-text "スコアを-1する" doesn't get op/val
# Check what normalize_fullwidth_digits does to the minus sign
sub = 'ライブの合計スコアを－１する'
print('Original:', repr(sub))
norm = normalize_fullwidth_digits(sub)
print('Normalized:', repr(norm))
print()

# Now trace what parse_action produces
r = parse_action(norm)
print('parse_action on normalized:')
print('  action:', r.get('action'))
print('  op:', r.get('operation'))
print('  val:', r.get('value'))
print()

# What about without normalization (the actual sub-path)?
# The sub-text goes through parse_effect which normalizes first
r2 = parse_effect(sub)
print('parse_effect on original:')
print('  action:', r2.get('action'))
print('  op:', r2.get('operation'))
print('  val:', r2.get('value'))
