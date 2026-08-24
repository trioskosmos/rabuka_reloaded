import json
ab = json.load(open('cards/abilities.json', encoding='utf-8'))
e = ab['unique_abilities'][916]
out = [json.dumps(e.get('cost'), ensure_ascii=False, indent=1)]
open('test_output/wwdcost.txt', 'w', encoding='utf-8').write(out[0])
print('ok')
