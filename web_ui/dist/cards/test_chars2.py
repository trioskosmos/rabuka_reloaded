import json
d = json.load(open('cards/abilities.json', encoding='utf-8'))
entry = d['unique_abilities'][523]
ft = entry['full_text']
tt = entry['triggerless_text']

# Find the μ's reference
for label, text in [('full_text', ft), ('triggerless_text', tt)]:
    idx = text.find('μ')
    if idx >= 0:
        print(f'{label}: found μ at {idx}')
        print(f'  context: {repr(text[max(0,idx-20):idx+20])}')
    else:
        print(f'{label}: μ not found')
        # Check for single quotes near "ステージにいる"
        idx2 = text.find('ステージにいる')
        if idx2 >= 0:
            print(f'  context at stage: {repr(text[idx2:idx2+50])}')
