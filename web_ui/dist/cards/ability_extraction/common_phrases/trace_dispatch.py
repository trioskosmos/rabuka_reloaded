import sys, re
sys.path.insert(0, 'cards/ability_extraction')

text = 'このカードのコストを+4して{{heart_05.png|heart05}}を得る。この能力では下にあるメンバーカードは3枚までしか数えない。'

# Check positions in file
with open('cards/ability_extraction/parser.py', encoding='utf-8') as f:
    content = f.read()

idx_cost = content.find("コストを' in t or 'コストが' in t or 'コストは' in t")
idx_deru = content.find("得る', 'gain_ability'")
idx_score = content.find("'スコアを' in t", idx_cost)

print('コストを rule at:', idx_cost)
print('得る rule at:', idx_deru)
print('スコアを rule at:', idx_score)
print('コストを comes before 得る:', idx_cost < idx_deru)

# Now let's see what parse_action actually does
from parser import parse_action
r = parse_action(text)
print()
print('Result action:', r.get('action'))
print('Result keys:', list(r.keys()))
print('ability_gain:', repr(r.get('ability_gain','')))
