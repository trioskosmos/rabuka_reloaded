# -*- coding: utf-8 -*-
"""Deep analysis of specific ability text patterns"""
import json, re, sys

with open(r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']
output = []

def out(s=''):
    output.append(s)
    print(s)

#===============================================================
# PATTERN: 元々持つ (original/natural blade count, original cost)
#===============================================================
out("=" * 70)
out("A. 元々持つ (original/natural) patterns")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    
    if '元々持つ' in ft:
        # Check what the parser does with it
        cond = eff.get('condition', {})
        if isinstance(cond, dict):
            out(f"  TEXT: {ft[:120]}")
            out(f"  CONDITION TYPE: {cond.get('type')}")
            out(f"  CONDITION: {json.dumps(cond, ensure_ascii=False)[:150]}")
            out(f"  ACTION: {eff.get('action')}")
            out(f"  CARDS: {a['card_count']}")
            out('')
        
        # Check cost modification
        cost = a.get('cost', {})
        if isinstance(cost, dict) and '元々持つ' in cost.get('text',''):
            out(f"  COST has 元々持つ: {cost.get('text')[:80]}")
            out('')

out("=" * 70)
out("B. 能力を持たない (does not have ability) patterns")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    
    if '能力を持たない' in ft or '能力も持たない' in ft:
        cond = eff.get('condition', {})
        out(f"  TEXT: {ft[:120]}")
        out(f"  CONDITION: {json.dumps(cond, ensure_ascii=False)[:150]}")
        out(f"  CARDS: {a['card_count']}")
        out('')

out("=" * 70)
out("C. 名前が異なる (different names) patterns")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    if '名前が異なる' not in ft and '名前の異なる' not in ft:
        continue
    eff = a.get('effect', {})
    cond = eff.get('condition', {}) if isinstance(eff, dict) else {}
    out(f"  TEXT: {ft[:120]}")
    out(f"  CONDITION TYPE: {cond.get('type')}")
    if cond.get('type') == 'compound':
        for i, sub in enumerate(cond.get('conditions', [])):
            out(f"    SUB-COND {i}: type={sub.get('type')}, distinct={sub.get('distinct')}")
    else:
        out(f"  DISTINCT: {cond.get('distinct')}")
    out(f"  CARDS: {a['card_count']}")
    out('')

out("=" * 70)
out("D. コストがそれぞれ異なる (costs all different)")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    if 'コストがそれぞれ異なる' not in ft:
        continue
    eff = a.get('effect', {})
    cond = eff.get('condition', {}) if isinstance(eff, dict) else {}
    out(f"  TEXT: {ft[:120]}")
    out(f"  CONDITION: {json.dumps(cond, ensure_ascii=False)[:200]}")
    out(f"  CARDS: {a['card_count']}")
    out('')

out("=" * 70)
out("E. 登場か、エリアを移動 (OR conditions)")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    if '登場か' not in ft and ('か、' not in ft or '移動' not in ft):
        continue
    if '登場か、エリアを移動' not in ft:
        continue
    eff = a.get('effect', {})
    cond = eff.get('condition', {}) if isinstance(eff, dict) else {}
    out(f"  TEXT: {ft[:120]}")
    out(f"  CONDITION TYPE: {cond.get('type')}")
    out(f"  CARDS: {a['card_count']}")
    out('')

out("=" * 70)
out("F. ステージから控え室に置かれた (placed from stage to discard)")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    if 'ステージから控え室に置かれ' not in ft:
        continue
    eff = a.get('effect', {})
    cond = eff.get('condition', {}) if isinstance(eff, dict) else {}
    out(f"  TEXT: {ft[:120]}")
    out(f"  CONDITION: {json.dumps(cond, ensure_ascii=False)[:150]}")
    out(f"  CARDS: {a['card_count']}")
    out('')

out("=" * 70)
out("G. いずれかの領域 (any zone/region)")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    if 'いずれかの領域' not in ft and 'いずれかの' not in ft:
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    out(f"  TEXT: {ft[:120]}")
    out(f"  EFFECT ACTION: {eff.get('action')}")
    if 'set_card_identity' in str(eff.get('action','')):
        out(f"  ALL_REGIONS: {eff.get('all_regions')}")
    out(f"  CARDS: {a['card_count']}")
    out('')

out("=" * 70)
out("H. グループ名 (group name) patterns")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    if 'グループ名' not in ft:
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    out(f"  TEXT: {ft[:120]}")
    out(f"  HAS group_names: {'group_names' in eff}")
    out(f"  HAS group: {'group' in eff}")
    out(f"  ACTION: {eff.get('action')}")
    out(f"  CARDS: {a['card_count']}")
    out('')

out("=" * 70)
out("I. カード名がすべて含まれる (card name contains all)")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    if 'カード名がすべて含まれる' not in ft:
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    out(f"  TEXT: {ft[:120]}")
    out(f"  HAS name_constraint: {eff.get('name_constraint')}")
    out(f"  CARDS: {a['card_count']}")
    out('')

out("=" * 70)
out("J. エールで～枚数を増やす (yell count modification)")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    if 'エール' in ft and ('枚数' in ft or '増やす' in ft or '減らす' in ft):
        eff = a.get('effect', {})
        if not isinstance(eff, dict): continue
        if 'yell' in str(eff) or 'エール' in str(eff):
            out(f"  TEXT: {ft[:120]}")
            out(f"  PARSED: {json.dumps(eff, ensure_ascii=False)[:200]}")
            out(f"  CARDS: {a['card_count']}")
            out('')

out("=" * 70)
out("K. ハートを増やす/減らす (modify required hearts)")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    if '必要ハート' not in ft or ('増やす' not in ft and '減らす' not in ft and '多くなる' not in ft and '少なくなる' not in ft):
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    out(f"  TEXT: {ft[:120]}")
    out(f"  PARSED: {json.dumps(eff, ensure_ascii=False)[:200]}")
    out(f"  CARDS: {a['card_count']}")
    out('')

out("=" * 70)
out("L. すべての領域 (all zones) card identity setting")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    if 'すべての領域' not in ft:
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    out(f"  TEXT: {ft[:120]}")
    out(f"  PARSED: {json.dumps(eff, ensure_ascii=False)[:200]}")
    out(f"  CARDS: {a['card_count']}")
    out('')

out("=" * 70)
out("M. として扱う (treat as) patterns")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    if 'として扱う' not in ft:
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    out(f"  TEXT: {ft[:120]}")
    out(f"  PARSED: {json.dumps(eff, ensure_ascii=False)[:200]}")
    out(f"  CARDS: {a['card_count']}")
    out('')

out("=" * 70)
out("N. ～に等しい枚数 (number equal to X)")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    if '等しい枚数' not in ft and '等しい' not in ft:
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    if '等しい' in ft:
        out(f"  TEXT: {ft[:120]}")
        out(f"  PARSED: {json.dumps(eff, ensure_ascii=False)[:200]}")
        out(f"  CARDS: {a['card_count']}")
        out('')

#===============================================================
# O. Missing per_unit flag
#===============================================================
out("=" * 70)
out("O. Missing per_unit flag on につき patterns")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    if 'につき' in ft and eff.get('per_unit') != True and eff.get('action') not in ('sequential', 'modify_cost'):
        # Check if it's an exclusion case
        if 'グループ名' in ft and 'につき' in ft:
            continue
        if 'この能力を起動するためのコストは' in ft:
            continue
        out(f"  TEXT: {ft[:120]}")
        out(f"  PARSED: per_unit={eff.get('per_unit')}, action={eff.get('action')}")
        out(f"  CARDS: {a['card_count']}")
        out('')

#===============================================================
# P. Cost modification not parsed
#===============================================================
out("=" * 70)
out("P. Cost modification patterns (元々持つコスト)")
out("=" * 70)
for a in abilities:
    ft = a['triggerless_text']
    if '元々持つコスト' not in ft:
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict): continue
    if 'modify_cost' not in str(eff) and 'cost_modification' not in str(eff):
        out(f"  TEXT: {ft[:120]}")
        out(f"  PARSED: {json.dumps(eff, ensure_ascii=False)[:200]}")
        out(f"  CARDS: {a['card_count']}")
        out('')

with open(r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\parser_bugs_deep.txt', 'w', encoding='utf-8') as f:
    f.write('\n'.join(output))

print(f"\nDeep analysis saved to parser_bugs_deep.txt")
