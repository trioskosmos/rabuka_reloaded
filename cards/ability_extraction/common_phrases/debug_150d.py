import json, sys
sys.path.insert(0, 'cards/ability_extraction')
import parser

with open('cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

a = data['unique_abilities'][150]
t = a['triggerless_text']
idx = t.find('-1')
if idx >= 0:
    print('found ASCII -1 at', idx)
else:
    print('ASCII -1 NOT found')
idx2 = t.find('\uff0d\uff11')
if idx2 >= 0:
    print('found fullwidth -\uff11 at', idx2)
# Show the full text around where the minus should be
# Look for スコアを and show context
idx3 = t.rfind('\u30b9\u30b3\u30a2\u3092')
if idx3 >= 0:
    print('found スコアを at', idx3)
    print('context:', repr(t[idx3:idx3+30]))

r = parser.parse_ability(t)
eff = r.get('effect', {})
def find_ms(d, depth=0):
    if isinstance(d, dict):
        for k, v in d.items():
            if k == 'action' and v == 'modify_score':
                print('MS at depth', depth, ':', d.get('operation'), d.get('value'))
            if isinstance(v, (dict, list)):
                find_ms(v, depth+1)
    elif isinstance(d, list):
        for item in d:
            find_ms(item, depth+1)
find_ms(eff)
