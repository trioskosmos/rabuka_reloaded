# -*- coding: utf-8 -*-
"""Comprehensive analysis of parser bugs in abilities.json"""
import json, re, sys

with open(r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']
output = []

def out(s):
    output.append(s)
    print(s)

#===============================================================
# 1. SOURCE MISMATCHES
#===============================================================
out("=" * 70)
out("1. ENERGY_ZONE source parsed as DECK (when text says エネルギー置き場から)")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    src = eff.get('source')
    if src == 'deck' and 'エネルギー置き場から' in ft:
        out(f"  TEXT: {ft[:100]}")
        out(f"  PARSED: source={src}, action={eff.get('action')}")
        out(f"  EXPECTED: source='energy_zone'")
        out(f"  CARDS: {a['card_count']}")
        out('')

out("=" * 70)
out("2. SUCCESS_LIVE_ZONE source parsed as DISCARD")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    src = eff.get('source')
    cost = a.get('cost', {})
    cost_src = cost.get('source') if isinstance(cost, dict) else None
    # Check effects
    if '成功ライブカード置き場から' in ft and src not in ('success_live_zone',) and src:
        out(f"  TEXT: {ft[:100]}")
        out(f"  PARSED EFFECT source={src}, action={eff.get('action')}")
        out(f"  EXPECTED: source='success_live_zone'")
        out(f"  CARDS: {a['card_count']}")
        out('')
    # Check costs
    if '成功ライブカード置き場から' in ft and cost_src and cost_src != 'success_live_zone':
        out(f"  COST source: {cost_src} != success_live_zone")
        out(f"  TEXT: {ft[:100]}")
        out('')

out("=" * 70)
out("3. UNDER_MEMBER source missing (actions referencing 下に置かれている)")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    if '下に置かれているエネルギーカード' in ft:
        src = eff.get('source')
        act = eff.get('action')
        if src != 'under_member':
            out(f"  TEXT: {ft[:100]}")
            out(f"  PARSED: source={src}, action={act}")
            out(f"  EXPECTED: source='under_member'")
            out(f"  CARDS: {a['card_count']}")
            out('')

#===============================================================
# 2. gain_resource with wrong resource type
#===============================================================
out("=" * 70)
out("4. gain_resource with WRONG resource type")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    if eff.get('action') != 'gain_resource': continue
    res = eff.get('resource', '')
    
    # Has blade icon but resource is not blade
    if '{{icon_blade.png|ブレード}}' in ft and res != 'blade':
        out(f"  TEXT: {ft[:100]}")
        out(f"  PARSED: resource={res}")
        out(f"  EXPECTED: resource='blade'")
        out(f"  CARDS: {a['card_count']}")
        out('')
    
    # Has specific heart icon but wrong resource
    heart_specific = re.findall(r'heart_(\d+)', ft)
    if heart_specific and res not in ('heart', 'blade', 'energy'):
        # Check if text is about modifying hearts (different from gaining)
        is_heart_gain = '得る' in ft.split('：')[-1] if '：' in ft else '得る' in ft
        if is_heart_gain:
            expected = f'heart{heart_specific[0].zfill(2)}'
            if res != expected:
                out(f"  TEXT: {ft[:100]}")
                out(f"  HEART ICONS: {heart_specific}")
                out(f"  PARSED: resource={res}")
                out(f"  EXPECTED: resource='{expected}' or 'heart'")
                out(f"  CARDS: {a['card_count']}")
                out('')

#===============================================================
# 3. change_state missing state_change
#===============================================================
out("=" * 70)
out("5. change_state actions MISSING state_change field")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    if eff.get('action') in ('change_state',) and 'state_change' not in eff:
        out(f"  TEXT: {ft[:100]}")
        out(f"  PARSED: action={eff.get('action')}, has state_change=False")
        out(f"  CARDS: {a['card_count']}")
        out('')

# Check cost state_change too
for a in abilities:
    cost = a.get('cost', {})
    if not isinstance(cost, dict): continue
    # Check if cost has ウェイトにする but parsed as something other than change_state
    if 'ウェイトにする' in cost.get('text','') and cost.get('type') != 'change_state':
        out(f"  COST with ウェイトにする but type={cost.get('type')}")
        out(f"  TEXT: {a['triggerless_text'][:100]}")
        out('')

out("=" * 70)
out("6. Missing state_change field in EFFECT change_state")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    if eff.get('action') == 'change_state' and eff.get('state_change') in (None, ''):
        out(f"  TEXT: {ft[:100]}")
        out(f"  state_change: {eff.get('state_change')}")
        out(f"  CARDS: {a['card_count']}")
        out('')

#===============================================================
# 4. Conditional sequential actions
#===============================================================
out("=" * 70)
out("7. Sequential actions missing conditional=true (contain 場合など)")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    if eff.get('action') == 'sequential' and eff.get('conditional') != True:
        if 'そうした場合' in ft:
            out(f"  TEXT: {ft[:100]}")
            out(f"  Contains そうした場合 but PARSED: conditional={eff.get('conditional')}")
            out(f"  CARDS: {a['card_count']}")
            out('')

#===============================================================
# 5. Conditions parsed with wrong type
#===============================================================
out("=" * 70)
out("8. Conditions with potentially wrong type")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    cond = eff.get('condition', {})
    if not isinstance(cond, dict) or cond.get('type') == 'custom': continue
    
    # Check for 名前が異なる conditions
    if '名前が異なる' in ft and cond.get('type') != 'location_condition':
        out(f"  TEXT: {ft[:100]}")
        out(f"  '名前が異なる' but type={cond.get('type')}")
        out(f"  COND: {json.dumps(cond, ensure_ascii=False)[:100]}")
        out(f"  CARDS: {a['card_count']}")
        out('')

    # Check for 能力を持たない
    if '能力を持たない' in ft:
        if cond.get('type') != 'ability_negation_condition':
            out(f"  TEXT: {ft[:100]}")
            out(f"  '能力を持たない' but type={cond.get('type')}")
            out(f"  CARDS: {a['card_count']}")
            out('')

out("=" * 70)
out("9. exclude_self missing on actions with このメンバー以外")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    if 'このメンバー以外' in ft:
        if eff.get('exclude_self') != True:
            out(f"  TEXT: {ft[:100]}")
            out(f"  PARSED: exclude_self={eff.get('exclude_self')}")
            out(f"  EXPECTED: exclude_self=True")
            out(f"  CARDS: {a['card_count']}")
            out('')

out("=" * 70)
out("10. Missing position fields (センター/左/右)")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    for kw in ['センターエリア', '左サイドエリア', '右サイドエリア']:
        if kw in ft:
            pos = eff.get('position')
            if pos is None or pos == '':
                out(f"  TEXT: {ft[:100]}")
                out(f"  Contains '{kw}' but position={pos}")
                out(f"  CARDS: {a['card_count']}")
                out('')

out("=" * 70)
out("11. 'all' flag missing for すべての patterns")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    if 'すべての' in ft and eff.get('all') != True:
        act = eff.get('action')
        if act in ('move_cards', 'change_state', 'gain_resource', 'modify_score'):
            out(f"  TEXT: {ft[:100]}")
            out(f"  Contains 'すべての' but PARSED: all={eff.get('all')}")
            out(f"  CARDS: {a['card_count']}")
            out('')

#===============================================================
# 12. Wrong card_type
#===============================================================
out("=" * 70)
out("12. Wrong card_type in effect")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    ct = eff.get('card_type', '')
    # If effect text says 'エネルギーカード' but card_type is not energy_card
    if 'エネルギーカード' in ft.split('：')[-1] if '：' in ft else 'エネルギーカード' in ft:
        eff_part = ft.split('：')[-1] if '：' in ft else ft
        if 'エネルギーカード' in eff_part and ct != 'energy_card' and ct != '':
            out(f"  TEXT: {ft[:100]}")
            out(f"  Contains 'エネルギーカード' but card_type={ct}")
            out(f"  CARDS: {a['card_count']}")
            out('')

#===============================================================
# 13. Missing count on move_cards actions
#===============================================================
out("=" * 70)
out("13. Missing count on move_cards with explicit 枚")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    if eff.get('action') == 'move_cards':
        cnt = eff.get('count')
        if cnt is None and '枚' in ft and 'すべて' not in ft and 'all' not in eff:
            # Check if dynamic_count explains it
            if 'dynamic_count' not in eff:
                out(f"  TEXT: {ft[:100]}")
                out(f"  PARSED: count={cnt}, no dynamic_count either")
                out(f"  CARDS: {a['card_count']}")
                out('')

#===============================================================
# 14. do_nothing between real actions (artifact of comma splitting)
#===============================================================
out("=" * 70)
out("14. do_nothing actions between real actions")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    if eff.get('action') == 'sequential':
        acts = eff.get('actions', [])
        for i, act in enumerate(acts):
            if act.get('action') == 'do_nothing':
                # Check if it's between real actions (artifact)
                if (i > 0 and acts[i-1].get('action') != 'do_nothing') or \
                   (i < len(acts)-1 and acts[i+1].get('action') != 'do_nothing'):
                    out(f"  TEXT: {ft[:100]}")
                    out(f"  do_nothing at index {i} between real actions")
                    out(f"  ALL ACTS: {[a.get('action') for a in acts]}")
                    out(f"  CARDS: {a['card_count']}")
                    out('')

with open(r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\parser_bugs_report.txt', 'w', encoding='utf-8') as f:
    f.write('\n'.join(output))

print(f"\n\nReport saved to parser_bugs_report.txt")
print(f"Total findings: {len([l for l in output if l.startswith('  TEXT:')])}")
