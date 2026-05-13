import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_ability

t = '{{icon_energy.png|E}}支払ってもよい：以下から1つを選ぶ。\n・相手のステージにいるコスト4以下のメンバー1人をウェイトにする。\n・カードを1枚引く。'
print('Input:', repr(t[:100]))
print()

r = parse_ability(t)
eff = r.get('effect', {})
print('Effect action:', eff.get('action'))
if eff.get('action') == 'choice':
    print('Options:', len(eff.get('options', [])))
    for i, opt in enumerate(eff.get('options', [])):
        print('  [%d] %s' % (i, opt.get('action')))
else:
    print('Full:', json.dumps(eff, ensure_ascii=False)[:600])
