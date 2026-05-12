import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_ability

# Card 1: Liella! group filter
t1 = '{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分のデッキの上からカードを7枚見る。その中から『Liella!』のカードを1枚公開して手札に加えてもよい。残りを控え室に置く。'
r1 = parse_ability(t1)
s1 = r1.get('effect', {}).get('select_action', {})
print('=== Card 1: Liella! filter ===')
print('group_names:', s1.get('group_names'))
print('cost_limit:', s1.get('cost_limit'))
print('destination:', s1.get('destination'))
print('count:', s1.get('count'))
print()

# Card 2: cost 11+ filter
t2 = '自分のデッキの上からカードを3枚見る。その中からコスト11以上のカードを1枚公開して手札に加えてもよい。残りを控え室に置く。'
r2 = parse_ability(t2)
s2 = r2.get('effect', {}).get('select_action', {})
print('=== Card 2: cost 11+ filter ===')
print('group_names:', s2.get('group_names'))
print('cost_limit:', s2.get('cost_limit'))
print('cost_limit_operator:', s2.get('cost_limit_operator'))
print('destination:', s2.get('destination'))
print('count:', s2.get('count'))
print()

# Card 3: the user's center card (previously broken)
t3 = '{{center.png|センター}}メンバー1人をウェイトにする：ライブ終了時まで、これによってウェイト状態になったメンバーは、「{{jyouji.png|常時}}ライブの合計スコアを+1する。」を得る。（この能力はセンターエリアに登場している場合のみ起動できる。）'
r3 = parse_ability(t3)
e3 = r3.get('effect', {})
print('=== Card 3: center gain_ability ===')
print('action:', e3.get('action'))
print('ability_gain:', e3.get('ability_gain','')[:40])
print('gained_effect:', e3.get('gained_effect',{}).get('action'))
print('activation_position:', e3.get('activation_position'))
