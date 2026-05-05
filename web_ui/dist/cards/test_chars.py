import json

# Read the actual card data
cards = json.load(open('cards/cards.json', encoding='utf-8'))
c = cards['PL!-bp5-021-L']
ability = c['ability']
print('ability bytes:', ability.encode('utf-8'))

# Find the mus group text
idx = ability.find("u's")
print('Context around mu:', repr(ability[idx-5:idx+10]))

# Read the abilities.json triggerless text
import json
d = json.load(open('cards/abilities.json', encoding='utf-8'))
e = d['unique_abilities'][523]
tt = e['triggerless_text']
idx2 = tt.find("u's")
print('Triggerless context:', repr(tt[idx2-5:idx2+10]))
if idx2 < 0:
    idx2 = tt.find("μ's")
    print('mu context:', repr(tt[idx2-5:idx2+10]))
