import json

with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

print('=== UNPARSED CUSTOM ACTIONS ===\n')
custom_actions = [a for a in abilities if a.get('effect', {}).get('action') == 'custom']
for i, a in enumerate(custom_actions[:15], 1):
    text = a['triggerless_text']
    print(str(i) + '. ' + text)

print('\nTotal custom actions: ' + str(len(custom_actions)))
