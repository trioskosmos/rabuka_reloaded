import json
data=json.load(open('C:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json','r',encoding='utf-8'))
abilities = data['unique_abilities']
matches = [(i,e) for i,e in enumerate(abilities) if e.get('triggers') and '\u5e38\u6642' in str(e['triggers'])]
print('Total: ' + str(len(matches)))
for i,e in matches:
    eff = e.get('effect',{})
    cond = eff.get('condition',{}) if eff else {}
    cost = e.get('cost')
    print('idx=' + str(i) + ' triggers=' + str(e['triggers']))
    print('  action=' + str(eff.get('action','N/A')))
    print('  cond_type=' + str(cond.get('type','none')))
    print('  cost=' + (str(cost.get('type','none')) if cost else 'none'))
    print('  use_limit=' + str(e.get('use_limit')))
    print('  full=' + str(e.get('full_text',''))[:120])
    print()
