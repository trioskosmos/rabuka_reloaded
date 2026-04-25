import json

def extract_actions(obj, actions):
    if isinstance(obj, dict):
        if 'action' in obj:
            actions.add(obj['action'])
        for v in obj.values():
            extract_actions(v, actions)
    elif isinstance(obj, list):
        for item in obj:
            extract_actions(item, actions)

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

actions = set()
extract_actions(data, actions)

for action in sorted(actions):
    print(action)
