import json
data = json.load(open('C:/Users/trios/OneDrive/Documents/rabuka_reloaded/cards/abilities.json', encoding='utf-8'))
for ua in data['unique_abilities']:
    for c in ua.get('cards', []):
        if 'bp3-025' in c:
            eff = ua['effect']
            if eff.get('action') == 'sequential':
                for i,a in enumerate(eff.get('actions',[])):
                    print('Action %d: action=%s source=%s dest=%s card_type=%s' % (
                        i, a.get('action'), a.get('source'), a.get('destination'), a.get('card_type')))
            break
