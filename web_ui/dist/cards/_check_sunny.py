import json
data = json.load(open('abilities.json','r',encoding='utf-8'))
for e in data['unique_abilities']:
    for card in e['cards']:
        if 'PL!-bp5-021-L' in card:
            # Print the full effect structure
            eff = e.get('effect', {})
            print('Action:', eff.get('action'))
            for i, act in enumerate(eff.get('actions', [])):
                print(f'\n--- Sub-action {i} ---')
                print(f'  Action: {act.get("action")}')
                if 'condition' in act:
                    c = act['condition']
                    print(f'  Condition type: {c.get("type")}')
                    print(f'  Condition count: {c.get("count")} operator: {c.get("operator")}')
                    print(f'  Condition unit: {c.get("unit")}')
                    print(f'  Condition distinct: {c.get("distinct")}')
                    print(f'  Condition locations: {c.get("locations")}')
                    print(f'  Condition text: {c.get("text","")[:80]}')
                for j, sub in enumerate(act.get('actions', [])):
                    print(f'  Sub-action[{j}]: {sub.get("action")} target={sub.get("target")} count={sub.get("count")}')
                    print(f'    source={sub.get("source")} dest={sub.get("destination")}')
                if act.get('action') == 'modify_score':
                    print(f'  operation={act.get("operation")} value={act.get("value")}')
                    print(f'  self_target={act.get("self_target")} target={act.get("target")}')
            break
