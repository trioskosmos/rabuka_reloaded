"""Verify specific conditions and dest issues."""
import json, os, unicodedata

path = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'abilities.json')
with open(path, 'r', encoding='utf-8') as f:
    data = json.load(f)
ab = data['unique_abilities']

# Check Category 8 conditions with missing fields
print('=== CAT 8: Condition details ===')
for idx in [47, 95, 112, 123, 143, 150, 156, 168, 211, 316, 365, 377, 416, 443, 448, 487, 542]:
    if idx >= len(ab): continue
    eff = ab[idx].get('effect', {})
    cond = eff.get('condition', {})
    print(f'\n[{idx}] type={cond.get("type","?")}')
    print(f'  target={cond.get("target")} location={cond.get("location")}')
    print(f'  text={cond.get("text","")[:120]}')
    if cond.get('type') == 'compound':
        for i, c in enumerate(cond.get('conditions',[])):
            print(f'  sub[{i}] type={c.get("type")} target={c.get("target")} location={c.get("location")}')

# Check [154] dest=discard but text says 手札に加える
print()
print('=== [154] dest issue detailed ===')
idx = 154
if idx < len(ab):
    eff = ab[idx].get('effect', {})
    sa = eff.get('select_action', {})
    actions = sa.get('actions', [])
    for i, a in enumerate(actions):
        print(f'  sub[{i}] action={a.get("action")} src={a.get("source")} dst={a.get("destination")} text={a.get("text","")[:80]}')

# Check [455] dest=null
print()
print('=== [455] dest null detailed ===')
idx = 455
if idx < len(ab):
    eff = ab[idx].get('effect', {})
    print(f'  eff action={eff.get("action")} src={eff.get("source")} dst={eff.get("destination")}')
    print(f'  eff text={eff.get("text","")[:120]}')

# Check [574] dest=null
print()
print('=== [574] dest null detailed ===')
idx = 574
if idx < len(ab):
    eff = ab[idx].get('effect', {})
    print(f'  eff action={eff.get("action")} src={eff.get("source")} dst={eff.get("destination")}')
    print(f'  eff text={eff.get("text","")[:120]}')

# Check the カスタム actions more carefully for [431,432,433]
print()
print('=== [431] custom action detailed ===')
for idx in [431, 432, 433]:
    if idx >= len(ab): continue
    eff = ab[idx].get('effect', {})
    sa = eff.get('select_action', {})
    actions = sa.get('actions', [])
    print(f'\n[{idx}]')
    for i, a in enumerate(actions):
        print(f'  sub[{i}] action={a.get("action")} ct={a.get("card_type")} src={a.get("source")} dst={a.get("destination")} text={a.get("text","")[:80]}')

# Check [570] missing count
print()
print('=== [570] missing count detailed ===')
idx = 570
if idx < len(ab):
    eff = ab[idx].get('effect', {})
    def check(e, depth=0):
        if not e: return
        if e.get('action') in ('move_cards','draw_card'):
            dc = e.get('dynamic_count')
            print(f'  {"  "*depth}action={e["action"]} count={e.get("count")} dc={json.dumps(dc) if dc else "NONE"} text={e.get("text","")[:80]}')
        for s in e.get('actions', []):
            check(s, depth+1)
        if e.get('action') == 'look_and_select':
            check(e.get('look_action'), depth+1)
            check(e.get('select_action'), depth+1)
    check(eff)
