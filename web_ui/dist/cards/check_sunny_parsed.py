"""Check current SUNNY DAY SONG parsed structure."""
import json
d = json.load(open('cards/abilities.json', encoding='utf-8'))
e = d['unique_abilities'][523]['effect']
actions = e['actions']
for i, a in enumerate(actions):
    print(f'Branch {i}:')
    cond = a.get('condition', {})
    if cond:
        print(f'  condition type={cond.get("type")}')
        if cond.get('type') == 'compound':
            for j, sub in enumerate(cond.get('conditions', [])):
                print(f'    sub[{j}]: type={sub.get("type")} count={sub.get("count")} unit={sub.get("unit")} ct={sub.get("card_type")} loc={sub.get("location")} dist={sub.get("distinct")}')
        else:
            print(f'  count={cond.get("count")} unit={cond.get("unit")} ct={cond.get("card_type")}')
    if a.get('action') == 'sequential':
        for j, sa in enumerate(a.get('actions', [])):
            print(f'  sub[{j}]: action={sa.get("action")} src={sa.get("source")} dst={sa.get("destination")} count={sa.get("count")} target={sa.get("target")}')
    else:
        print(f'  action={a.get("action")}')
