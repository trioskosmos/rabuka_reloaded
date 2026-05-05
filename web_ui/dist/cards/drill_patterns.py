"""
Drill into each of the 8 structural patterns to find the
REAL sub-patterns inside them.
"""
import json
import re
from collections import defaultdict

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']

def core_structure(text):
    if '：' in text and not '場合' in text:
        return 'cost_action'
    if any(m in text for m in ['場合、', 'とき、', 'なら、']):
        return 'conditional'
    if '以下から1つを選ぶ' in text:
        return 'choice'
    if 'その中から' in text:
        return 'look_select'
    if 'かぎり' in text:
        return 'duration'
    if 'につき' in text or 'ごとに' in text:
        return 'per_unit'
    if '代わりに' in text:
        return 'conditional_alt'
    if 'さらに' in text:
        return 'sequential'
    if 'その後' in text:
        return 'sequential'
    if '。' in text:
        parts = [p.strip() for p in text.split('。') if p.strip()]
        if len(parts) >= 2:
            return 'sequential'
    if '、' in text:
        first = text.split('、')[0].strip()
        if any(first.endswith(e) for e in ['き','ぎ','し','じ','ち','び','み','り','い','え']):
            return 'sequential'
    return 'simple'

def effect_shape(effect):
    if not isinstance(effect, dict):
        return '?'
    a = effect.get('action', '')
    if a == 'sequential':
        subs = effect.get('actions', [])
        tags = []
        for s in subs:
            if isinstance(s, dict):
                sa = s.get('action', '')
                if sa == 'move_cards':
                    tags.append(f"move_{s.get('source','?')}->{s.get('destination','?')}")
                elif sa == 'gain_resource':
                    r = s.get('resource', '?')
                    d = 't' if s.get('duration') else 'p'
                    tags.append(f"gain_{r}({d})")
                else:
                    tags.append(sa)
        return '+'.join(tags) if tags else 'seq'
    if a == 'move_cards':
        return f"move_{effect.get('source','?')}->{effect.get('destination','?')}"
    if a == 'gain_resource':
        r = effect.get('resource', '?')
        d = 't' if effect.get('duration') else 'p'
        g = '[group]' if effect.get('group') else ''
        return f"gain_{r}({d}){g}"
    return a

# Drill into each structure
for cs_name in ['conditional', 'cost_action', 'simple', 'sequential', 'duration', 'per_unit', 'look_select', 'choice']:
    entries = [ab for ab in abilities if core_structure(ab.get('triggerless_text', '')) == cs_name]
    print(f"\n{'='*60}")
    print(f"  {cs_name.upper()} — {len(entries)} abilities")
    print(f"{'='*60}")
    
    # Group by trigger+effect shape
    families = defaultdict(list)
    for ab in entries:
        t = (ab.get('triggers', '') or '', effect_shape(ab.get('effect', {})))
        families[t].append(ab.get('triggerless_text', '')[:80])
    
    sorted_f = sorted(families.items(), key=lambda x: -len(x[1]))
    for (trig, eff), texts in sorted_f[:10]:
        print(f"\n  trigger={trig or 'none':<15} effect={eff}")
        for t in texts[:2]:
            print(f"    {t}")
    
    # For conditional, also show condition types
    if cs_name == 'conditional':
        cond_types = defaultdict(int)
        for ab in entries:
            eff = ab.get('effect', {})
            cond = eff.get('condition', {})
            if isinstance(cond, dict):
                ct = cond.get('type', '?')
                cond_types[ct] += 1
        print(f"\n  Condition type distribution:")
        for ct, c in sorted(cond_types.items(), key=lambda x: -x[1]):
            print(f"    {ct:<30} x{c}")
    
    # For cost_action, show cost types
    if cs_name == 'cost_action':
        cost_types = defaultdict(int)
        for ab in entries:
            cost = ab.get('cost', {})
            if isinstance(cost, dict):
                ct = cost.get('type', '?')
                cost_types[ct] += 1
        print(f"\n  Cost type distribution:")
        for ct, c in sorted(cost_types.items(), key=lambda x: -x[1]):
            print(f"    {ct:<30} x{c}")
        
        # And for move_cards costs: which source→destination pairs?
        cost_move = defaultdict(int)
        for ab in entries:
            cost = ab.get('cost', {})
            if isinstance(cost, dict) and cost.get('type') == 'move_cards':
                key = f"{cost.get('source','?')}->{cost.get('destination','?')}"
                cost_move[key] += 1
        print(f"\n  Cost move_cards pairs:")
        for pair, c in sorted(cost_move.items(), key=lambda x: -x[1]):
            print(f"    {pair:<25} x{c}")
    
    if cs_name == 'simple':
        action_types = defaultdict(int)
        for ab in entries:
            eff = ab.get('effect', {})
            a = eff.get('action', '?') if isinstance(eff, dict) else '?'
            action_types[a] += 1
        print(f"\n  Action type distribution:")
        for at, c in sorted(action_types.items(), key=lambda x: -x[1]):
            print(f"    {at:<30} x{c}")
