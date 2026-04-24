import json

with open('c:/Users/trios/OneDrive/Documents/rabuka_reloaded/cards/abilities.json', encoding='utf-8') as f:
    data = json.load(f)

actions = set()
conditions = set()

for item in data['unique_abilities']:
    effect = item.get('effect')
    if effect:
        actions.add(effect.get('action'))
        # Check nested actions
        if 'actions' in effect:
            for sub in effect['actions']:
                actions.add(sub.get('action'))
        if 'look_action' in effect:
            actions.add(effect['look_action'].get('action'))
        if 'select_action' in effect:
            actions.add(effect['select_action'].get('action'))
    
    cost = item.get('cost')
    if cost:
        actions.add(cost.get('action'))
    
    # Check conditions
    if effect and 'condition' in effect:
        conditions.add(effect['condition'].get('type'))
    if cost and 'condition' in cost:
        conditions.add(cost['condition'].get('type'))

print("Actions:", sorted([a for a in actions if a]))
print("\nConditions:", sorted([c for c in conditions if c]))
