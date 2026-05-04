import json

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

ua = data.get('unique_abilities', [])
entry = ua[523]
effect = entry.get('effect', {})
actions = effect.get('actions', [])
print("Branches:", len(actions))

for i, a in enumerate(actions):
    print(f"\nBranch {i}: action={a.get('action')}")
    inner = a.get('actions', [])
    for j, ia in enumerate(inner):
        print(f"  [{j}] action={ia.get('action')} text='{ia.get('text','')[:30]}' dur={ia.get('duration')} resource={ia.get('resource')} group={ia.get('group')} card_type={ia.get('card_type')}")
