import json
data = json.load(open('cards/cards.json', encoding='utf-8'))
for suffix in ['P2', 'R2', 'L2', 'N2', 'SEC2', 'SECE', 'SECL', 'SRL', 'LLE', 'RE', 'PE', 'PR', 'PP', 'AR', 'CL', 'DUO', 'RM', 'SD2', 'SECS', 'SRE']:
    matches = [k for k in data if data[k].get('card_no', '').endswith(suffix)]
    if matches:
        print(f'{suffix}: {len(matches)} cards')
        for k in matches[:5]:
            cn = data[k].get('card_no', '')
            print(f'  {cn} -> {data[k].get("rare")}')