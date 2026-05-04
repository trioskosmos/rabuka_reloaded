import json
d = json.load(open('cards/abilities.json', encoding='utf-8'))
e = d['unique_abilities'][523]['effect']
branches = e['actions']
for i, branch in enumerate(branches):
    print('Branch {}: action={}'.format(i, branch.get('action')))
    inner = branch.get('actions')
    if inner:
        for j, sub in enumerate(inner):
            print('  [{}] action={} dur={} group={} resource={} count={} card_type={}'.format(
                j, sub.get('action'), sub.get('duration'), sub.get('group'),
                sub.get('resource'), sub.get('count'), sub.get('card_type')))
    else:
        print('  direct: action={} dur={} value={}'.format(
            branch.get('action'), branch.get('duration'), branch.get('value')))
