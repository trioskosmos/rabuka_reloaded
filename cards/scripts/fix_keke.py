import json

file_path = 'c:/Users/trios/OneDrive/Documents/rabuka_reloaded/cards/abilities.json'
with open(file_path, 'r', encoding='utf-8') as f:
    data = json.load(f)

for ability in data:
    if "これにより控え室に置いたカードの中にブレードハートを持たないメンバーカードが" in ability.get('full_text', ''):
        effect = ability.get('effect', {})
        if effect.get('action') == 'sequential':
            actions = effect.get('actions', [])
            if len(actions) >= 3:
                # First condition
                cond1 = actions[1].get('condition', {})
                cond1['source'] = 'preceding_moved'
                cond1['card_property'] = 'has_blade_heart'
                cond1['negation'] = True
                cond1['card_type'] = 'member_card'
                
                # Second condition
                cond2 = actions[2].get('condition', {})
                cond2['source'] = 'preceding_moved'
                cond2['card_property'] = 'has_blade_heart'
                cond2['negation'] = True
                cond2['card_type'] = 'member_card'

with open(file_path, 'w', encoding='utf-8') as f:
    json.dump(data, f, ensure_ascii=False, indent=2)

print("Updated abilities.json")
