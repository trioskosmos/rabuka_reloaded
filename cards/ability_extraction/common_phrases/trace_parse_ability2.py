import json, sys
sys.path.insert(0, 'cards/ability_extraction')
from parser import parse_ability, parse_effect, split_cost_effect, normalize

# The exact triggerless_text from the JSON (has fullwidth chars)
t_full = '{{center.png|センター}}メンバー1人をウェイトにする：ライブ終了時まで、これによってウェイト状態になったメンバーは、「{{jyouji.png|常時}}ライブの合計スコアを+1する。」を得る。（この能力はセンターエリアに登場している場合のみ起動できる。）'

print('=== parse_ability ===')
r = parse_ability(t_full)
eff = r.get('effect', {})
print('  action:', eff.get('action'))
if eff.get('action') == 'sequential':
    for i, a in enumerate(eff.get('actions', [])):
        print('  [%d] %s text=%s' % (i, a.get('action'), a.get('text','')[:50]))
print('  parenthetical:', eff.get('parenthetical'))
print('  activation_condition_parsed:', eff.get('activation_condition_parsed') is not None)
print('  gained_effect:', eff.get('gained_effect') is not None)

# Now test what parse_effect gives us on the effect part
print()
print('=== split_cost_effect + parse_effect ===')
t_norm = normalize(t_full.strip())
cost_text, effect_text = split_cost_effect(t_norm)
print('  effect_text:', effect_text[:80])
r2 = parse_effect(effect_text)
print('  action:', r2.get('action'))
if r2.get('action') == 'sequential':
    for i, a in enumerate(r2.get('actions', [])):
        print('  [%d] %s text=%s' % (i, a.get('action'), a.get('text','')[:50]))
print('  parenthetical:', r2.get('parenthetical'))
print('  activation_condition_parsed:', r2.get('activation_condition_parsed') is not None)
