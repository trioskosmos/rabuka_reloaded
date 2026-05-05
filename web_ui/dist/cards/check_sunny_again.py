import json
with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
ua = data.get('unique_abilities', [])
entry = ua[523]
effect = entry.get('effect', {})
actions2 = effect.get('actions', [])[1]
sub_actions = actions2.get('actions', [])
for a in sub_actions:
    print(f"Action: {a.get('action')}, text='{a.get('text','')[:30]}', duration={a.get('duration')}")
